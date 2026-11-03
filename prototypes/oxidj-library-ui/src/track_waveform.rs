//! W4: the selected track's waveform in the track-details panel.
//!
//! Computed peaks come only from the catalog (`track_waveforms`, W2) and are
//! drawn with [`waveform_view`]. Vendor-derived data (Rekordbox ANLZ PWV3/PWV5
//! of an imported library track whose location is this file) is shown in its
//! own labelled strip and never merged with, or substituted for, computed
//! peaks. Catalog reads happen once per selection (and after a reload), not per
//! frame.

use crate::beat_grid::{self, BeatGrid};
use crate::components::{SectionHeading, StatusBadge, StatusKind};
use crate::theme::ColorTokens;
use crate::waveform_view::{self as wv, Style, Viewport, waveform_view, waveform_view_sized};
use eframe::egui::{self, Color32, Rect, Sense, Ui, Vec2};
use oxidj_application::WaveformPeaks;
use oxidj_library::{Catalog, TrackDetails, VendorWaveform};

/// Length of the always-visible zoomed detail window when the track has no
/// usable imported beat grid (a few bars at typical tempos).
pub const DETAIL_SECONDS: f64 = 8.0;
/// Caption shown when the detail window follows an imported beat grid.
pub const GRID_SOURCE_LABEL: &str = "beat grid: library import (vendor-derived)";
/// Height of the slim whole-track overview.
pub const OVERVIEW_HEIGHT: f32 = 36.0;
/// Where the detail window starts on a newly selected track (skips most intros).
const DEFAULT_POSITION: f32 = 0.25;

/// Height of the vendor-derived strip.
pub const VENDOR_HEIGHT: f32 = 40.0;

/// What the catalog holds for one selected track, read on selection.
struct Loaded {
    track: u64,
    /// `Ok(None)` = not analysed; `Err` = unreadable/stale row (reason shown).
    computed: Result<Option<WaveformPeaks>, String>,
    vendor: Result<Vec<VendorWaveform>, String>,
    /// Imported beat grid validated against the computed track length;
    /// `Ok(None)` = none or unusable (fixed-length window).
    grid: Result<Option<BeatGrid>, String>,
}

/// Rects of the last rendered frame (tests drive real pointer input with them).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PanelLayout {
    pub(crate) styles: Option<[Rect; 3]>,
    /// Low, mid, high sliders.
    pub(crate) eq: Option<[Rect; 3]>,
    pub(crate) view: Option<Rect>,
    pub(crate) vendor: Option<Rect>,
    /// Detail-window Position slider.
    pub(crate) position: Option<Rect>,
    /// Detail viewport actually drawn.
    pub(crate) detail: Option<Viewport>,
    /// `(start, end)` ms and starting bar index when the beat grid is used.
    pub(crate) grid_window: Option<((f64, f64), usize)>,
}

#[derive(Default)]
pub(crate) struct TrackWaveformPanel {
    pub(crate) style: Style,
    pub(crate) eq: wv::Eq,
    loaded: Option<Loaded>,
    pub(crate) layout: PanelLayout,
    /// Detail-window position `0..=1` through the track; `None` = default for a new selection.
    position: Option<f32>,
    /// Starting bar index when a beat grid drives the window; `None` = default.
    bar: Option<usize>,
}

impl TrackWaveformPanel {
    /// Forget cached catalog reads (after import/reload).
    pub(crate) fn invalidate(&mut self) {
        self.loaded = None;
    }

    fn load(&mut self, catalog: Option<&Catalog>, item: &TrackDetails) -> &Loaded {
        let id = item.track.id.as_u64();
        if self.loaded.as_ref().is_none_or(|l| l.track != id) {
            let unavailable = || "catalog unavailable".to_string();
            let computed = catalog.ok_or_else(unavailable).and_then(|db| {
                match db.read_waveform(item.track.id) {
                    Ok(Some(stored)) if stored.source_sha256 == item.track.sha256 => {
                        Ok(Some(stored.peaks))
                    }
                    Ok(Some(_)) => Err("stored waveform is out of date (source changed)".into()),
                    Ok(None) => Ok(None),
                    Err(e) => Err(e.to_string()),
                }
            });
            let (uri, _) = oxidj_application::location_uri(&item.track.path);
            let vendor = catalog.ok_or_else(unavailable).and_then(|db| {
                db.vendor_waveforms_for_locations(&[&item.track.path, &uri])
                    .map_err(|e| e.to_string())
            });
            let grid = match (&computed, catalog) {
                (Ok(Some(peaks)), Some(db)) => db
                    .beat_grid_for_locations(&[&item.track.path, &uri])
                    .map(|beats| {
                        let ms = peaks.sample_count() as f64 * 1000.0
                            / f64::from(peaks.sample_rate().max(1));
                        BeatGrid::new(&beats, ms)
                    })
                    .map_err(|e| e.to_string()),
                _ => Ok(None),
            };
            self.position = None;
            self.bar = None;
            self.loaded = Some(Loaded {
                track: id,
                computed,
                vendor,
                grid,
            });
        }
        self.loaded.as_ref().expect("just loaded")
    }

    /// Waveform section for the selected `item`.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        catalog: Option<&Catalog>,
        item: &TrackDetails,
        tokens: ColorTokens,
    ) {
        self.layout = PanelLayout::default();
        ui.separator();
        SectionHeading {
            title: "Waveform · computed by OxiDJ",
        }
        .show(ui, tokens);
        // Disjoint borrows: the loaded data is read while style/eq/layout change.
        self.load(catalog, item);
        let Self {
            style,
            eq,
            loaded,
            layout,
            position,
            bar,
        } = self;
        let loaded = loaded.as_ref().expect("loaded above");
        match &loaded.computed {
            Ok(Some(peaks)) => {
                ui.horizontal(|ui| {
                    ui.label("Style");
                    layout.styles =
                        Some(Style::ALL.map(|s| ui.selectable_value(style, s, s.label()).rect));
                });
                ui.horizontal_wrapped(|ui| {
                    let mut slider = |value: &mut f32, text| {
                        ui.add(
                            egui::Slider::new(value, 0.0..=1.0)
                                .text(text)
                                .max_decimals(2),
                        )
                        .rect
                    };
                    layout.eq = Some([
                        slider(&mut eq.low, "Low"),
                        slider(&mut eq.mid, "Mid"),
                        slider(&mut eq.high, "High"),
                    ]);
                });
                let width = ui.available_width();
                let total = peaks.sample_count();
                let rate = f64::from(peaks.sample_rate().max(1));
                let grid = loaded.grid.as_ref().ok().and_then(Option::as_ref);
                let (start, window) = match grid {
                    Some(grid) => {
                        // Position moves by whole bars; the window starts on a downbeat.
                        let max = grid.max_start_bar();
                        let default = (max as f32 * DEFAULT_POSITION).round() as usize;
                        let mut b = bar.unwrap_or(default).min(max);
                        layout.position = Some(
                            ui.add(
                                egui::Slider::new(&mut b, 0..=max)
                                    .text("Position")
                                    .show_value(false),
                            )
                            .rect,
                        );
                        *bar = Some(b);
                        let ms = grid.window_ms(b);
                        layout.grid_window = Some((ms, b));
                        let s = (ms.0 * rate / 1000.0).round() as u64;
                        let e = (ms.1 * rate / 1000.0).round() as u64;
                        (s, e.saturating_sub(s).max(1))
                    }
                    None => {
                        let window = ((rate * DETAIL_SECONDS) as u64).clamp(1, total.max(1));
                        let slack = total.saturating_sub(window);
                        let mut pos = position.unwrap_or(DEFAULT_POSITION).clamp(0.0, 1.0);
                        layout.position = Some(
                            ui.add(
                                egui::Slider::new(&mut pos, 0.0..=1.0)
                                    .text("Position")
                                    .show_value(false),
                            )
                            .rect,
                        );
                        *position = Some(pos);
                        ((slack as f64 * f64::from(pos)) as u64, window)
                    }
                };
                let detail = Viewport {
                    start_sample: start,
                    end_sample: start + window,
                    width_px: width,
                };
                layout.detail = Some(detail);
                let view = waveform_view(ui, Some(peaks), *style, detail, *eq);
                if let (Some(grid), Some((ms, _))) = (grid, layout.grid_window) {
                    beat_grid::paint(ui, view.id.with("beat-grid"), view.rect, grid, ms, tokens);
                }
                layout.view = Some(view.rect);
                let overview = waveform_view_sized(
                    ui,
                    Some(peaks),
                    *style,
                    Viewport::whole(peaks, width),
                    *eq,
                    OVERVIEW_HEIGHT,
                );
                // Window marker over the overview.
                let r = overview.rect;
                let frac = |s: u64| r.left() + r.width() * (s as f32 / total.max(1) as f32);
                let marker = Rect::from_min_max(
                    egui::pos2(frac(start), r.top()),
                    egui::pos2(frac(start + window).max(frac(start) + 2.0), r.bottom()),
                )
                .intersect(r);
                ui.painter_at(r).rect_filled(
                    marker,
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                );
                let track_s = total as f64 / rate;
                match (grid, layout.grid_window) {
                    (Some(grid), Some((_, b))) => {
                        ui.small(format!(
                            "Detail {} bars from Bar {} of {} · track {track_s:.1} s · {} Hz · display gain only (audio unchanged)",
                            beat_grid::BARS,
                            b + 1,
                            grid.bar_count(),
                            peaks.sample_rate()
                        ));
                        ui.small(GRID_SOURCE_LABEL);
                    }
                    _ => {
                        ui.small(format!(
                            "Detail {DETAIL_SECONDS:.0} s window · track {track_s:.1} s · {} Hz · display gain only (audio unchanged)",
                            peaks.sample_rate()
                        ));
                    }
                }
                if let Err(reason) = &loaded.grid {
                    StatusBadge {
                        label: &format!("Imported beat grid unreadable: {reason}"),
                        kind: StatusKind::Error,
                    }
                    .show(ui, tokens);
                }
            }
            Ok(None) => {
                StatusBadge {
                    label: "No computed waveform: track not analysed",
                    kind: StatusKind::Warning,
                }
                .show(ui, tokens);
                ui.small("Run `oxidj catalog analyze-waveform --db <catalog>` then Reload.");
            }
            Err(reason) => {
                StatusBadge {
                    label: &format!("Waveform unavailable: {reason}"),
                    kind: StatusKind::Error,
                }
                .show(ui, tokens);
            }
        }
        match &loaded.vendor {
            Ok(vendor) => {
                // PWV5 (colour) preferred when both kinds are present.
                if let Some(waveform) = vendor.last() {
                    ui.separator();
                    SectionHeading {
                        title: "Vendor-derived waveform (not computed by OxiDJ)",
                    }
                    .show(ui, tokens);
                    ui.small(format!("Source: {} · shown as stored", waveform.kind()));
                    layout.vendor = Some(vendor_strip(ui, waveform, tokens));
                }
            }
            Err(reason) => {
                StatusBadge {
                    label: &format!("Vendor-derived waveform unreadable: {reason}"),
                    kind: StatusKind::Error,
                }
                .show(ui, tokens);
            }
        }
    }
}

/// Paint vendor columns as stored (height 0..=31; PWV5 colour 0..=7 per channel)
/// into their own rect, one bar per pixel (nearest column).
fn vendor_strip(ui: &mut Ui, waveform: &VendorWaveform, tokens: ColorTokens) -> Rect {
    let width = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, VENDOR_HEIGHT), Sense::hover());
    let columns: Vec<(u8, Color32)> = match waveform {
        VendorWaveform::RekordboxDetail(c) => {
            c.iter().map(|&[h, _]| (h, tokens.secondary)).collect()
        }
        VendorWaveform::RekordboxColourDetail(c) => c
            .iter()
            .map(|&[r, g, b, h]| {
                let s = |v: u8| (u16::from(v) * 255 / 7) as u8;
                (h, Color32::from_rgb(s(r), s(g), s(b)))
            })
            .collect(),
    };
    let painter = ui.painter_at(rect);
    let px = rect.width().floor() as usize;
    if columns.is_empty() || px == 0 {
        return rect;
    }
    for i in 0..px {
        let (h, color) = columns[i * columns.len() / px];
        let half = (rect.height() / 2.0) * f32::from(h.min(31)) / 31.0;
        let x = rect.left() + i as f32;
        let bar = Rect::from_min_max(
            egui::pos2(x, rect.center().y - half),
            egui::pos2(x + 1.0, rect.center().y + half),
        );
        painter.rect_filled(bar.intersect(rect), 0.0, color);
    }
    rect
}

#[cfg(test)]
mod tests {
    use crate::LibraryApp;
    use crate::theme::ThemeMode;
    use crate::waveform_view::Style;
    use eframe::egui::{self, Pos2, Rect, Shape};
    use oxidj_library::{Catalog, LibrarySnapshot, LibraryTrack, VendorWaveform};
    use std::path::Path;

    /// 16-bit mono PCM WAV of `f(t)` for `seconds` at 44.1 kHz.
    fn write_wav(path: &Path, seconds: f32, f: impl Fn(f32) -> f32) {
        let rate = 44_100u32;
        let n = (rate as f32 * seconds) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + n * 2).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(n * 2).to_le_bytes());
        for i in 0..n {
            let v = (f(i as f32 / rate as f32).clamp(-1.0, 1.0) * 32_000.0) as i16;
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    fn sine(hz: f32, amp: f32) -> impl Fn(f32) -> f32 {
        move |t| amp * (t * hz * std::f32::consts::TAU).sin()
    }

    struct Harness {
        ctx: egui::Context,
        app: LibraryApp,
    }

    type Frame = egui::FullOutput;

    impl Harness {
        fn frame(&mut self, events: Vec<egui::Event>) -> Frame {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0))),
                events,
                ..Default::default()
            };
            let app = &mut self.app;
            self.ctx.run(input, |ctx| app.ui(ctx))
        }

        /// Real press and release on separate frames, then two settle frames
        /// (a newly shown panel section settles its layout on the next frame).
        fn click(&mut self, pos: Pos2) -> Frame {
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            };
            self.frame(vec![egui::Event::PointerMoved(pos), button(true)]);
            self.frame(vec![button(false)]);
            self.frame(vec![]);
            self.frame(vec![])
        }
    }

    fn texts(output: &Frame) -> Vec<(String, Rect)> {
        output
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                Shape::Text(t) => Some((t.galley.job.text.clone(), t.visual_bounding_rect())),
                _ => None,
            })
            .collect()
    }

    /// The only text exactly equal to `label`.
    fn unique_text(output: &Frame, label: &str) -> Rect {
        let hits: Vec<_> = texts(output)
            .into_iter()
            .filter(|(t, _)| t == label)
            .collect();
        assert_eq!(hits.len(), 1, "{label:?}: {hits:?}");
        hits[0].1
    }

    fn has_text(output: &Frame, needle: &str) -> bool {
        texts(output).iter().any(|(t, _)| t.contains(needle))
    }

    /// Painted waveform geometry inside `view` (positions and colours).
    fn waveform_vertices(output: &Frame, view: Rect) -> Vec<(Pos2, egui::Color32)> {
        let mut out = Vec::new();
        for clipped in &output.shapes {
            if let Shape::Mesh(mesh) = &clipped.shape {
                if view.expand(0.5).contains_rect(mesh.calc_bounds()) {
                    out.extend(mesh.vertices.iter().map(|v| (v.pos, v.color)));
                }
            }
        }
        out
    }

    fn vendor_bars(output: &Frame, strip: Rect) -> usize {
        output
            .shapes
            .iter()
            .filter(
                |c| matches!(&c.shape, Shape::Rect(r) if strip.expand(0.5).contains_rect(r.rect)),
            )
            .count()
    }

    #[test]
    fn selecting_tracks_shows_catalog_waveform_style_eq_and_vendor_strip_separately() {
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-w4-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        // Low-heavy, high-heavy and an unanalysed track (synthetic only).
        write_wav(&audio.join("a-low.wav"), 2.0, sine(60.0, 0.9));
        write_wav(&audio.join("b-high.wav"), 2.0, sine(6_000.0, 0.4));
        write_wav(&audio.join("c-none.wav"), 1.0, sine(440.0, 0.5));
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .import_audio_directory(audio.to_str().unwrap())
            .unwrap();
        let tracks = catalog.list_track_details().unwrap();
        let id_of = |name: &str| {
            tracks
                .iter()
                .find(|t| t.track.path.ends_with(name))
                .unwrap()
                .track
                .clone()
        };
        for name in ["a-low.wav", "b-high.wav"] {
            oxidj_application::analyze_track_waveform(&mut catalog, id_of(name).id).unwrap();
        }
        // Vendor-derived data for track A only, keyed by its imported location.
        let (location, _) = oxidj_application::location_uri(&id_of("a-low.wav").path);
        let vendor = VendorWaveform::RekordboxColourDetail(vec![[7, 0, 0, 31], [0, 7, 0, 10]]);
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "synthetic".into(),
                    location,
                    vendor_waveforms: vec![vendor],
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        let stored_a = catalog
            .read_waveform(id_of("a-low.wav").id)
            .unwrap()
            .unwrap();
        drop(catalog);

        let mut h = Harness {
            ctx: egui::Context::default(),
            app: LibraryApp::new(
                db.clone(),
                ThemeMode::TokyoNight,
                ThemeMode::TokyoNight.tokens(),
            ),
        };
        let first = h.frame(vec![]);
        assert!(
            h.app.track_waveform.layout.view.is_none(),
            "nothing selected yet"
        );
        assert!(has_text(&first, "Select a track to inspect its details."));

        // Select A through the production track table.
        let out = h.click(unique_text(&first, "a-low.wav").center());
        assert_eq!(h.app.selected, Some(id_of("a-low.wav").id.as_u64()));
        let view = h
            .app
            .track_waveform
            .layout
            .view
            .expect("waveform drawn for A");
        let a_mono = waveform_vertices(&out, view);
        assert!(!a_mono.is_empty(), "computed waveform painted");
        assert!(has_text(&out, "Waveform · computed by OxiDJ"));
        // Vendor strip: own labelled rect, never overlapping computed peaks.
        assert!(has_text(&out, "Vendor-derived waveform"));
        assert!(has_text(&out, "rekordbox-anlz-pwv5"));
        let strip = h
            .app
            .track_waveform
            .layout
            .vendor
            .expect("vendor strip for A");
        assert!(
            !strip.intersects(view),
            "vendor data drawn apart: {strip:?} vs {view:?}"
        );
        assert!(vendor_bars(&out, strip) > 0);

        // Style change via the real selector alters painted shapes.
        let styles = h.app.track_waveform.layout.styles.unwrap();
        let out = h.click(styles[2].center());
        assert_eq!(h.app.track_waveform.style, Style::Rgb);
        assert_eq!(
            h.app.track_waveform.layout.view,
            Some(view),
            "view rect stable"
        );
        let a_rgb = waveform_vertices(&out, view);
        assert!(!a_rgb.is_empty());
        assert_ne!(a_rgb, a_mono, "RGB differs from mono");
        let out = h.click(styles[1].center());
        assert_eq!(h.app.track_waveform.style, Style::ThreeBand);
        let a_three = waveform_vertices(&out, view);
        assert_ne!(a_three, a_rgb);

        // Low EQ cut via the real slider (press at its left end).
        let low = h.app.track_waveform.layout.eq.unwrap()[0];
        let out = h.click(Pos2::new(low.left() + 1.0, low.center().y));
        assert!(
            h.app.track_waveform.eq.low < 0.05,
            "{:?}",
            h.app.track_waveform.eq
        );
        let a_cut = waveform_vertices(&out, view);
        assert_ne!(a_cut, a_three, "low cut changes the painted low layer");
        // A is low-heavy: cutting low shrinks its painted vertical extent.
        let extent =
            |v: &[(Pos2, egui::Color32)]| v.iter().map(|(p, _)| p.y).fold(f32::MAX, f32::min);
        assert!(extent(&a_cut) > extent(&a_three), "top edge moved down");
        // Display gain never touches stored peaks.
        let reopened = Catalog::open(&db).unwrap();
        assert_eq!(
            reopened
                .read_waveform(id_of("a-low.wav").id)
                .unwrap()
                .unwrap(),
            stored_a
        );
        drop(reopened);

        // Unanalysed track: clear empty state, no computed geometry, no vendor strip.
        let out = h.click(unique_text(&out, "c-none.wav").center());
        assert_eq!(h.app.selected, Some(id_of("c-none.wav").id.as_u64()));
        assert!(has_text(&out, "No computed waveform: track not analysed"));
        assert!(h.app.track_waveform.layout.view.is_none());
        assert!(h.app.track_waveform.layout.vendor.is_none());
        assert!(waveform_vertices(&out, view).is_empty());

        // B: data swapped (same style/EQ as A had), no vendor data shown.
        h.app.track_waveform.eq = Default::default();
        let out = h.click(unique_text(&out, "b-high.wav").center());
        assert_eq!(h.app.selected, Some(id_of("b-high.wav").id.as_u64()));
        let view_b = h
            .app
            .track_waveform
            .layout
            .view
            .expect("waveform drawn for B");
        let b_three = waveform_vertices(&out, view_b);
        assert!(!b_three.is_empty());
        assert_ne!(b_three, a_three, "selection change swaps data");
        assert!(h.app.track_waveform.layout.vendor.is_none());
        assert!(!has_text(&out, "Vendor-derived waveform"));

        std::fs::remove_dir_all(root).unwrap();
    }

    /// Grid line quads painted inside `view`, as (centre x, is bar line).
    fn grid_lines(output: &Frame, view: Rect) -> Vec<(f32, bool)> {
        let c = crate::beat_grid::GridColours::from_tokens(ThemeMode::TokyoNight.tokens());
        let mut out = Vec::new();
        for clipped in &output.shapes {
            let Shape::Mesh(mesh) = &clipped.shape else {
                continue;
            };
            if mesh.vertices.is_empty()
                || !mesh
                    .vertices
                    .iter()
                    .all(|v| v.color == c.bar || v.color == c.beat)
            {
                continue;
            }
            // Painted bounds: every quad inside the view rect and its clip rect.
            let bounds = clipped.shape.visual_bounding_rect();
            assert!(
                view.contains_rect(bounds) && clipped.clip_rect.contains_rect(bounds),
                "{bounds:?} outside {view:?}/{:?}",
                clipped.clip_rect
            );
            for quad in mesh.vertices.chunks(4) {
                let x = quad.iter().map(|v| v.pos.x).sum::<f32>() / 4.0;
                out.push((x, quad[0].color == c.bar));
            }
        }
        out
    }

    #[test]
    fn imported_beat_grid_drives_a_four_bar_window_with_lines_and_bar_counter() {
        use oxidj_library::LibraryBeat;
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-w6-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        write_wav(&audio.join("g-grid.wav"), 20.0, sine(60.0, 0.8));
        write_wav(&audio.join("n-nogrid.wav"), 20.0, sine(60.0, 0.8));
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .import_audio_directory(audio.to_str().unwrap())
            .unwrap();
        let tracks = catalog.list_track_details().unwrap();
        let id_of = |name: &str| {
            tracks
                .iter()
                .find(|t| t.track.path.ends_with(name))
                .unwrap()
                .track
                .clone()
        };
        for name in ["g-grid.wav", "n-nogrid.wav"] {
            oxidj_application::analyze_track_waveform(&mut catalog, id_of(name).id).unwrap();
        }
        // 120 bpm, first beat at 250 ms with a two-beat pickup: downbeats at
        // 1250 + 2000 k ms (bar 1 = 1250 ms), 40 beats → 10 bars in 20 s.
        let beats: Vec<_> = (0..40u32)
            .map(|i| LibraryBeat {
                time_ms: 250 + 500 * i,
                tempo_hundredths_bpm: 12_000,
                beat_in_bar: ((i + 2) % 4 + 1) as u8,
            })
            .collect();
        let (location, _) = oxidj_application::location_uri(&id_of("g-grid.wav").path);
        let (other, _) = oxidj_application::location_uri(&id_of("n-nogrid.wav").path);
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![
                    LibraryTrack {
                        title: "synthetic grid".into(),
                        location,
                        beat_grid: beats,
                        ..LibraryTrack::default()
                    },
                    // Imported too, but without a grid.
                    LibraryTrack {
                        title: "synthetic no grid".into(),
                        location: other,
                        ..LibraryTrack::default()
                    },
                ],
                roots: vec![],
            })
            .unwrap();
        let stored = catalog
            .read_waveform(id_of("g-grid.wav").id)
            .unwrap()
            .unwrap();
        drop(catalog);

        let mut h = Harness {
            ctx: egui::Context::default(),
            app: LibraryApp::new(
                db.clone(),
                ThemeMode::TokyoNight,
                ThemeMode::TokyoNight.tokens(),
            ),
        };
        let first = h.frame(vec![]);
        let out = h.click(unique_text(&first, "g-grid.wav").center());
        let panel = &h.app.track_waveform;
        let view = panel.layout.view.expect("detail view");
        // Default: bar index round(0.25 * 6) = 2 → Bar 3 at 5250 ms, 4 bars.
        let (window, bar) = panel.layout.grid_window.expect("grid used");
        assert_eq!((window, bar), ((5_250.0, 13_250.0), 2));
        let detail = panel.layout.detail.unwrap();
        assert_eq!(
            (detail.start_sample, detail.end_sample),
            (5_250 * 441 / 10, 13_250 * 441 / 10)
        );
        assert!(has_text(&out, super::GRID_SOURCE_LABEL));
        assert!(has_text(&out, "Detail 4 bars from Bar 3 of 10"));
        // Lines at the expected x for the known beat times, bars stronger.
        let lines = grid_lines(&out, view);
        assert_eq!(lines.len(), 16, "{lines:?}");
        for (k, &(x, is_bar)) in lines.iter().enumerate() {
            let t = 5_250.0 + 500.0 * k as f32;
            let want = view.left() + (t - 5_250.0) / 8_000.0 * view.width();
            assert!((x - want).abs() <= 1.01, "beat {k}: {x} vs {want}");
            assert_eq!(is_bar, k % 4 == 0, "beat {k}");
        }
        // Bar counter: one "Bar N" label per visible bar, inside the view.
        for n in 3..=6 {
            let label = unique_text(&out, &format!("Bar {n}"));
            assert!(view.contains_rect(label), "Bar {n}: {label:?}");
        }
        assert!(!has_text(&out, "Detail 8 s window"));

        // Position slider moves by whole bars: press at its rail's right end
        // (the response rect also covers the "Position" text).
        let slider = h.app.track_waveform.layout.position.unwrap();
        let rail_right = slider.left() + h.ctx.style().spacing.slider_width;
        let out = h.click(Pos2::new(rail_right - 1.0, slider.center().y));
        let (window, bar) = h.app.track_waveform.layout.grid_window.unwrap();
        assert_eq!((window, bar), ((13_250.0, 21_250.0), 6));
        let lines = grid_lines(&out, view);
        // Bars 7..=10 within the 20 s track: beats up to 19 750 ms.
        assert_eq!(lines.len(), 14, "{lines:?}");
        assert!(lines[0].1 && lines[0].0 <= view.left() + 1.01);
        for n in 7..=10 {
            assert!(view.contains_rect(unique_text(&out, &format!("Bar {n}"))));
        }
        // Then to the left end: Bar 1 at the first downbeat.
        let out = h.click(Pos2::new(slider.left() + 1.0, slider.center().y));
        assert_eq!(
            h.app.track_waveform.layout.grid_window.unwrap(),
            ((1_250.0, 9_250.0), 0)
        );
        assert_eq!(grid_lines(&out, view).len(), 16);
        assert!(has_text(&out, "Bar 1"));

        // Imported track without a grid: unchanged 8 s window, no lines, no label.
        let out = h.click(unique_text(&out, "n-nogrid.wav").center());
        assert_eq!(h.app.selected, Some(id_of("n-nogrid.wav").id.as_u64()));
        let panel = &h.app.track_waveform;
        assert!(panel.layout.grid_window.is_none());
        let detail = panel.layout.detail.unwrap();
        assert_eq!(detail.end_sample - detail.start_sample, 8 * 44_100);
        assert!(has_text(&out, "Detail 8 s window"));
        assert!(!has_text(&out, super::GRID_SOURCE_LABEL));
        assert!(grid_lines(&out, panel.layout.view.unwrap()).is_empty());
        assert!(!texts(&out).iter().any(|(t, _)| t.starts_with("Bar ")));

        // Computed peaks untouched by the grid path.
        let reopened = Catalog::open(&db).unwrap();
        assert_eq!(
            reopened
                .read_waveform(id_of("g-grid.wav").id)
                .unwrap()
                .unwrap(),
            stored
        );
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}
