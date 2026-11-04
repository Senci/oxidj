//! W4: the selected track's waveform in the track-details panel.
//!
//! Computed peaks come only from the catalog (`track_waveforms`, W2) and are
//! drawn with [`waveform_view`]. Vendor-derived data (Rekordbox ANLZ PWV3/PWV5
//! of an imported library track whose location is this file) is shown in its
//! own labelled strip and never merged with, or substituted for, computed
//! peaks. Catalog reads happen once per selection (and after a reload), not per
//! frame.
//!
//! B2c: when no usable imported grid exists, an OxiDJ-computed grid
//! (`track_tempo_grids`, current source digest only) drives the same W6
//! window and lines. Its downbeat is unknown, so bar lines fall on every 4th
//! beat counted from the first detected beat, and it is captioned as computed.
//! An imported grid always wins.

use crate::beat_grid::{self, BeatGrid};
use crate::components::{SectionHeading, StatusBadge, StatusKind};
use crate::theme::ColorTokens;
use crate::waveform_view::{self as wv, Style, Viewport, waveform_view_sized};
use eframe::egui::{self, Color32, Rect, Sense, Ui, Vec2};
use oxidj_application::WaveformPeaks;
use oxidj_library::{Catalog, TrackDetails, VendorWaveform};

/// Length of the always-visible zoomed detail window when the track has no
/// usable imported beat grid (a few bars at typical tempos).
pub const DETAIL_SECONDS: f64 = 8.0;
/// Caption shown when the detail window follows an imported beat grid.
pub const GRID_SOURCE_LABEL: &str = "beat grid: library import (vendor-derived)";
/// B2c: a computed grid has no downbeat, so bars are counted in fours from
/// the first detected beat (bar lines every 4th beat); this caption says so.
pub const COMPUTED_GRID_NOTE: &str = "beats counted from first detected beat";
/// Below this analyser confidence the computed BPM carries a muted "low"
/// badge. Confidence is taktus `sure` clamped to 0..=1, not a calibrated
/// probability, so the badge is only a hint.
pub const LOW_CONFIDENCE: f32 = 0.5;
/// Height of the slim whole-track overview.
pub const OVERVIEW_HEIGHT: f32 = 36.0;
/// Where the detail window starts on a newly selected track (skips most intros).
const DEFAULT_POSITION: f32 = 0.25;

/// Height of the vendor-derived strip.
pub const VENDOR_HEIGHT: f32 = 40.0;
/// Caption: the gain sliders scale the drawing only.
pub const GAIN_NOTE: &str = "display gain only (audio unchanged)";
/// Detail strip height range (W9): grows into spare details height.
pub const DETAIL_MIN_HEIGHT: f32 = wv::HEIGHT;
pub const DETAIL_MAX_HEIGHT: f32 = 140.0;
/// Fixed label column of the slider rows (fits "Position").
pub const SLIDER_LABEL_WIDTH: f32 = 60.0;

/// What the catalog holds for one selected track, read on selection.
struct Loaded {
    track: u64,
    /// `Ok(None)` = not analysed; `Err` = unreadable/stale row (reason shown).
    computed: Result<Option<WaveformPeaks>, String>,
    vendor: Result<Vec<VendorWaveform>, String>,
    /// Beat grid validated against the computed track length and where it
    /// came from (an imported grid always wins over a computed one);
    /// `Ok(None)` = none or unusable (fixed-length window).
    grid: Result<Option<(BeatGrid, GridSource)>, String>,
    /// B2b computed tempo of this exact file (`Err` = stale/unreadable row).
    tempo: Result<Option<ComputedTempo>, String>,
}

/// Where the detail window's beat grid came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridSource {
    /// Imported library (vendor-derived) grid with real downbeats.
    Library,
    /// OxiDJ-computed grid (`track_tempo_grids`): bars counted from the first beat.
    Computed,
}

/// A computed tempo grid as read from the catalog.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedTempo {
    pub(crate) bpm: f64,
    pub(crate) confidence: f32,
    pub(crate) algorithm: String,
    pub(crate) beats_ms: Vec<u32>,
}

impl ComputedTempo {
    /// The computed beats as a 4/4 grid counted from the first detected beat
    /// (its downbeat is unknown), so [`BeatGrid`] draws them like W6.
    fn beats(&self) -> Vec<oxidj_library::LibraryBeat> {
        let tempo = (self.bpm * 100.0).round() as u32;
        self.beats_ms
            .iter()
            .enumerate()
            .map(|(i, &time_ms)| oxidj_library::LibraryBeat {
                time_ms,
                tempo_hundredths_bpm: tempo,
                beat_in_bar: (i % 4 + 1) as u8,
            })
            .collect()
    }
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
    /// Whole-track overview strip.
    pub(crate) overview: Option<Rect>,
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
            let tempo = catalog.ok_or_else(unavailable).and_then(|db| {
                match db.computed_beat_grid(item.track.id) {
                    Ok(Some(stored)) if stored.source_sha256 == item.track.sha256 => {
                        Ok(Some(ComputedTempo {
                            bpm: stored.grid.bpm(),
                            confidence: stored.grid.confidence(),
                            algorithm: stored.grid.algorithm().to_owned(),
                            beats_ms: stored.grid.beats_ms().to_vec(),
                        }))
                    }
                    Ok(Some(_)) => Err("stored grid is out of date (source changed)".into()),
                    Ok(None) => Ok(None),
                    Err(e) => Err(e.to_string()),
                }
            });
            let grid = match (&computed, catalog) {
                (Ok(Some(peaks)), Some(db)) => db
                    .beat_grid_for_locations(&[&item.track.path, &uri])
                    .map(|beats| {
                        let ms = peaks.sample_count() as f64 * 1000.0
                            / f64::from(peaks.sample_rate().max(1));
                        // The imported grid always wins; the computed one
                        // is only the fallback when no usable import exists.
                        BeatGrid::new(&beats, ms)
                            .map(|g| (g, GridSource::Library))
                            .or_else(|| {
                                let tempo = tempo.as_ref().ok()?.as_ref()?;
                                Some((BeatGrid::new(&tempo.beats(), ms)?, GridSource::Computed))
                            })
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
                tempo,
            });
        }
        self.loaded.as_ref().expect("just loaded")
    }

    /// Computed `(bpm, confidence)` of the selected `item` for the facts
    /// row; `None` when no current computed grid is stored.
    pub(crate) fn computed_tempo(
        &mut self,
        catalog: Option<&Catalog>,
        item: &TrackDetails,
    ) -> Option<(f64, f32)> {
        let tempo = self.load(catalog, item).tempo.as_ref().ok()?.as_ref()?;
        Some((tempo.bpm, tempo.confidence))
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
                // W11: the style picker is the shared segmented toggle group,
                // in the slider rows' label column.
                labelled_row(ui, "Style", |ui| {
                    let cells = Style::ALL.map(|s| (s.label(), *style == s));
                    let responses = crate::components::segmented_toggles(ui, tokens, &cells);
                    let mut rects = [Rect::NOTHING; 3];
                    for (i, response) in responses.iter().enumerate() {
                        if response.clicked() {
                            *style = Style::ALL[i];
                        }
                        rects[i] = response.rect;
                    }
                    layout.styles = Some(rects);
                    responses[0].union(responses[2].clone())
                });
                // W9: one aligned row per control: fixed label column, rail,
                // then the value ("Low ━━━━○ 1.00"); every rail starts and ends
                // at the same x, Position included (it shows no value).
                let rail = slider_rail_width(ui);
                ui.vertical(|ui| {
                    slider_look(ui, tokens);
                    ui.spacing_mut().slider_width = rail;
                    let mut slider = |value: &mut f32, text| {
                        labelled_row(ui, text, |ui| {
                            ui.add(
                                egui::Slider::new(value, 0.0..=1.0)
                                    .trailing_fill(true)
                                    .max_decimals(2),
                            )
                        })
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
                let source = loaded.grid.as_ref().ok().and_then(Option::as_ref);
                let grid = source.map(|(grid, _)| grid);
                let (start, window) = match grid {
                    Some(grid) => {
                        // Position moves by whole bars; the window starts on a downbeat.
                        let max = grid.max_start_bar();
                        let default = (max as f32 * DEFAULT_POSITION).round() as usize;
                        let mut b = bar.unwrap_or(default).min(max);
                        layout.position = Some(position_row(ui, rail, tokens, |ui| {
                            ui.add(egui::Slider::new(&mut b, 0..=max).trailing_fill(true).show_value(false))
                        }));
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
                        layout.position = Some(position_row(ui, rail, tokens, |ui| {
                            ui.add(egui::Slider::new(&mut pos, 0.0..=1.0).trailing_fill(true).show_value(false))
                        }));
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
                // W9: the detail strip takes the details viewport's spare
                // height (96..=140 px) but never pushes content below it.
                let grid_lines = match source {
                    Some((_, GridSource::Library)) => 1,
                    Some((_, GridSource::Computed)) => 2,
                    None => 0,
                };
                let below = below_detail_height(ui, loaded.vendor.as_ref().is_ok_and(|v| !v.is_empty()), grid_lines);
                let height = detail_height(ui, below);
                let view = waveform_view_sized(ui, Some(peaks), *style, detail, *eq, height);
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
                // W11: a visible window marker: light tint plus a 1 px
                // focus-colour outline (the 16 % tint alone was faint).
                let painter = ui.painter_at(r);
                painter.rect_filled(marker, 2.0, tokens.text.gamma_multiply(0.12));
                painter.rect_stroke(marker, 2.0, egui::Stroke::new(1.0_f32, tokens.focus), egui::StrokeKind::Inside);
                layout.overview = Some(r);
                let track_s = total as f64 / rate;
                // W9: short lines that wrap inside the panel (never clip).
                let detail_line = match (grid, layout.grid_window) {
                    (Some(grid), Some((_, b))) => format!(
                        "Detail {} bars from Bar {} of {} · track {track_s:.1} s · {} Hz",
                        beat_grid::BARS,
                        b + 1,
                        grid.bar_count(),
                        peaks.sample_rate()
                    ),
                    _ => format!(
                        "Detail {DETAIL_SECONDS:.0} s window · track {track_s:.1} s · {} Hz",
                        peaks.sample_rate()
                    ),
                };
                caption(ui, &detail_line, tokens);
                match (source, &loaded.tempo) {
                    (Some((_, GridSource::Library)), _) => caption(ui, GRID_SOURCE_LABEL, tokens),
                    (Some((_, GridSource::Computed)), Ok(Some(tempo))) => {
                        caption(ui, &computed_grid_label(tempo), tokens);
                        caption(ui, COMPUTED_GRID_NOTE, tokens);
                    }
                    _ => {}
                }
                caption(ui, GAIN_NOTE, tokens);
                if let Err(reason) = &loaded.grid {
                    StatusBadge {
                        label: &format!("Imported beat grid unreadable: {reason}"),
                        kind: StatusKind::Error,
                    }
                    .show(ui, tokens);
                }
                if let (None, Err(reason)) = (source, &loaded.tempo) {
                    StatusBadge {
                        label: &format!("Computed beat grid unavailable: {reason}"),
                        kind: StatusKind::Warning,
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
                        title: "Vendor-derived waveform",
                    }
                    .show(ui, tokens);
                    // W9: provenance on its own wrapping line, so the heading
                    // never widens the details panel past its minimum.
                    caption(
                        ui,
                        &format!("Not computed by OxiDJ · source: {} · shown as stored", waveform.kind()),
                        tokens,
                    );
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

/// Rail width shared by every slider row: what is left of the row after the
/// label column and the value box, so the rails align and never overflow.
fn slider_rail_width(ui: &Ui) -> f32 {
    let spacing = ui.spacing();
    // DragValue "1.00" plus button padding is wider than interact_size.x.
    let value = spacing.interact_size.x.max(52.0);
    (ui.available_width() - SLIDER_LABEL_WIDTH - value - 2.0 * spacing.item_spacing.x)
        .clamp(80.0, 320.0)
}

/// `label` in the fixed left column, then `add` (the slider); returns the
/// slider's rect (rail first, so its left edge is the rail's left end).
fn labelled_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> egui::Response) -> Rect {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(SLIDER_LABEL_WIDTH, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(SLIDER_LABEL_WIDTH);
                ui.add(egui::Label::new(label).truncate());
            },
        );
        add(ui).rect
    })
    .inner
}

/// The shared slider look: full-strength primary trailing fill (the 20 %
/// selection tint left the filled rail barely visible in Tokyo Day).
fn slider_look(ui: &mut Ui, tokens: ColorTokens) {
    ui.visuals_mut().selection.bg_fill = tokens.primary;
}

/// The Position row: same label column, rail width and look as the gains
/// (W11: it was drawn without the fill, so its rail looked nearly invisible).
fn position_row(ui: &mut Ui, rail: f32, tokens: ColorTokens, add: impl FnOnce(&mut Ui) -> egui::Response) -> Rect {
    ui.scope(|ui| {
        slider_look(ui, tokens);
        ui.spacing_mut().slider_width = rail;
        labelled_row(ui, "Position", add)
    })
    .inner
}

/// "beat grid: computed (taktus 0.1.0), 128.00 BPM".
pub(crate) fn computed_grid_label(tempo: &ComputedTempo) -> String {
    format!("beat grid: computed ({}), {:.2} BPM", tempo.algorithm, tempo.bpm)
}

/// One small muted caption line, wrapped to the panel width.
fn caption(ui: &mut Ui, text: &str, tokens: ColorTokens) {
    ui.add(egui::Label::new(egui::RichText::new(text).small().color(tokens.muted_text)).wrap());
}

/// Height reserved below the detail strip: overview, caption lines (one
/// spare for wrapping) and, when present, the vendor-derived section.
fn below_detail_height(ui: &Ui, vendor: bool, grid_lines: u8) -> f32 {
    let gap = ui.spacing().item_spacing.y;
    let small = ui.text_style_height(&egui::TextStyle::Small);
    let body = ui.text_style_height(&egui::TextStyle::Body);
    let lines = 3.0 + f32::from(grid_lines);
    let mut h = gap + OVERVIEW_HEIGHT + lines * (small + gap);
    if vendor {
        // separator + heading + source line + strip
        h += 2.0 * gap + 6.0 + body + gap + small + gap + VENDOR_HEIGHT + gap;
    }
    h
}

/// Detail strip height: the viewport's spare height after the content so
/// far and `below`, clamped to `DETAIL_MIN_HEIGHT..=DETAIL_MAX_HEIGHT`.
/// Measured from the content top, so it does not change with scrolling.
pub(crate) fn detail_height(ui: &Ui, below: f32) -> f32 {
    let viewport = ui.clip_rect().height();
    let used = ui.cursor().top() - ui.min_rect().top();
    (viewport - used - below).clamp(DETAIL_MIN_HEIGHT, DETAIL_MAX_HEIGHT)
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
        // W8: details is a toggled panel (off by default); open it as a user would.
        h.app.show_details = true;
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
        // W11: the overview sits 4 px under the detail strip with the same
        // left/right inset, and the Position rail carries the same primary
        // trailing fill as the gain rails (it was drawn without one).
        let overview = h.app.track_waveform.layout.overview.expect("overview drawn");
        assert!(
            (overview.top() - view.bottom() - 4.0).abs() < 0.5,
            "4 px gap: {view:?} {overview:?}"
        );
        assert_eq!(overview.x_range(), view.x_range());
        let primary = ThemeMode::TokyoNight.tokens().primary;
        let filled = |rect: Rect| {
            out.shapes.iter().any(|c| {
                matches!(&c.shape, egui::Shape::Rect(r)
                    if r.fill == primary && rect.contains_rect(r.rect) && r.rect.width() > 1.0)
            })
        };
        let position = h.app.track_waveform.layout.position.expect("position slider");
        assert!(filled(h.app.track_waveform.layout.eq.unwrap()[0]), "gain rail filled");
        assert!(filled(position), "position rail filled like the gains");
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
            "view rect stable {:?}", h.app.track_waveform.layout.view
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
    fn device_relative_library_grid_matches_a_track_under_a_mounted_root() {
        // W10: the library stores `/Contents/…` (device-relative); the local file
        // sits under a fake mount root. The suffix fallback finds its grid.
        use oxidj_library::LibraryBeat;
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-w10-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("media/STICK/Contents/Art ist");
        std::fs::create_dir_all(&audio).unwrap();
        write_wav(&audio.join("g grid.wav"), 20.0, sine(60.0, 0.8));
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .import_audio_directory(root.join("media").to_str().unwrap())
            .unwrap();
        let track = catalog.list_track_details().unwrap()[0].track.clone();
        oxidj_application::analyze_track_waveform(&mut catalog, track.id).unwrap();
        let beats: Vec<_> = (0..40u32)
            .map(|i| LibraryBeat {
                time_ms: 250 + 500 * i,
                tempo_hundredths_bpm: 12_000,
                beat_in_bar: ((i + 2) % 4 + 1) as u8,
            })
            .collect();
        // Stored exactly as the PDB importer encodes a device path.
        let (location, _) = oxidj_application::location_uri("/Contents/Art ist/g grid.wav");
        assert_eq!(location, "file://localhost/Contents/Art%20ist/g%20grid.wav");
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "synthetic grid".into(),
                    location,
                    beat_grid: beats,
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        drop(catalog);

        let mut h = Harness {
            ctx: egui::Context::default(),
            app: LibraryApp::new(db, ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens()),
        };
        h.app.show_details = true;
        let first = h.frame(vec![]);
        let out = h.click(unique_text(&first, "g grid.wav").center());
        assert_eq!(
            h.app.track_waveform.layout.grid_window,
            Some(((5_250.0, 13_250.0), 2))
        );
        assert!(has_text(&out, super::GRID_SOURCE_LABEL));
        assert!(has_text(&out, "Detail 4 bars from Bar 3 of 10"));
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
        // W8: details is a toggled panel (off by default); open it as a user would.
        h.app.show_details = true;
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
        // (W9: the label sits in its own column, so the rect is the rail).
        let slider = h.app.track_waveform.layout.position.unwrap();
        let rail_right = slider.right();
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

    /// Kick+click at `bpm` from 250 ms (synthetic click track).
    fn click(bpm: f32) -> impl Fn(f32) -> f32 {
        move |t| {
            if t < 0.25 {
                return 0.0;
            }
            let tau = (t - 0.25) % (60.0 / bpm);
            0.6 * (std::f32::consts::TAU * 55.0 * tau).sin() * (-tau / 0.06).exp()
                + 0.3 * (std::f32::consts::TAU * 1_800.0 * tau).sin() * (-tau / 0.004).exp()
        }
    }

    /// Layout top (galley origin, not glyph bounds) of the painted text
    /// `label` inside `panel`.
    fn top_in(output: &Frame, label: &str, panel: Rect) -> f32 {
        output
            .shapes
            .iter()
            .find_map(|c| match &c.shape {
                Shape::Text(t) if t.galley.job.text == label && panel.contains(t.pos) => Some(t.pos.y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{label:?} not painted in {panel:?}"))
    }

    /// B2c: a track with only an OxiDJ-computed grid gets the W6 four-bar
    /// window and lines (bars counted from the first detected beat) with the
    /// computed caption and a "BPM (computed)" fact; an imported grid always
    /// wins over a computed one; no grid at all keeps the 8 s window.
    #[test]
    fn computed_grid_is_the_fallback_with_its_own_caption_and_bpm_fact() {
        use oxidj_library::LibraryBeat;
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-b2c-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        write_wav(&audio.join("c-computed.wav"), 20.0, click(120.0));
        write_wav(&audio.join("v-both.wav"), 20.0, click(120.0));
        write_wav(&audio.join("n-none.wav"), 20.0, sine(60.0, 0.8));
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
        for name in ["c-computed.wav", "v-both.wav", "n-none.wav"] {
            oxidj_application::analyze_track_waveform(&mut catalog, id_of(name).id).unwrap();
        }
        for name in ["c-computed.wav", "v-both.wav"] {
            oxidj_application::analyze_track_tempo(
                &mut catalog,
                id_of(name).id,
                oxidj_application::DEFAULT_TEMPO_RANGE,
            )
            .unwrap();
        }
        let stored = catalog
            .computed_beat_grid(id_of("c-computed.wav").id)
            .unwrap()
            .expect("click track has a computed grid")
            .grid;
        assert!((stored.bpm() - 120.0).abs() < 1.0, "{}", stored.bpm());
        assert!(catalog.computed_beat_grid(id_of("v-both.wav").id).unwrap().is_some());
        // v-both also has an imported grid (deliberately offset from the
        // computed beats so the two are distinguishable).
        let (location, _) = oxidj_application::location_uri(&id_of("v-both.wav").path);
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "synthetic grid".into(),
                    location,
                    beat_grid: (0..40u32)
                        .map(|i| LibraryBeat {
                            time_ms: 250 + 500 * i,
                            tempo_hundredths_bpm: 12_000,
                            beat_in_bar: ((i + 2) % 4 + 1) as u8,
                        })
                        .collect(),
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        drop(catalog);

        let mut h = Harness {
            ctx: egui::Context::default(),
            app: LibraryApp::new(db.clone(), ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens()),
        };
        h.app.show_details = true;
        let first = h.frame(vec![]);

        // 1. Computed only: W6 window from the computed beats.
        let out = h.click(unique_text(&first, "c-computed.wav").center());
        let panel = h.app.shell.details.expect("details panel");
        let view = h.app.track_waveform.layout.view.expect("detail view");
        let beats: Vec<f64> = stored.beats_ms().iter().map(|&b| f64::from(b)).filter(|&b| b < 20_000.0).collect();
        let bars = beats.len().div_ceil(4);
        let default = ((bars - super::beat_grid::BARS) as f32 * 0.25).round() as usize;
        let (window, bar) = h.app.track_waveform.layout.grid_window.expect("computed grid used");
        assert_eq!(bar, default);
        assert_eq!(window, (beats[4 * bar], beats[4 * (bar + 4)]), "four bars from a counted bar line");
        let caption = format!("beat grid: computed (taktus 0.1.0), {:.2} BPM", stored.bpm());
        for label in [caption.as_str(), super::COMPUTED_GRID_NOTE] {
            let rect = unique_text(&out, label);
            assert!(panel.contains_rect(rect), "{label:?} inside the panel");
        }
        assert!(!has_text(&out, super::GRID_SOURCE_LABEL), "never labelled as an import");
        assert!(has_text(&out, &format!("Detail 4 bars from Bar {} of {bars}", bar + 1)));
        assert!(!has_text(&out, "Detail 8 s window"));
        // Lines at the stored beat times; every 4th beat from the first is a bar line.
        let lines = grid_lines(&out, view);
        let first_beat = beats.partition_point(|&b| b < window.0);
        let expected: Vec<_> = beats.iter().enumerate().filter(|(_, b)| (window.0..window.1).contains(*b)).collect();
        assert_eq!(lines.len(), expected.len(), "{lines:?}");
        assert_eq!(expected.len(), 16);
        for (&(x, is_bar), &(i, &t)) in lines.iter().zip(&expected) {
            let want = view.left() + ((t - window.0) / (window.1 - window.0)) as f32 * view.width();
            assert!((x - want).abs() <= 1.01, "beat {i}: {x} vs {want}");
            assert_eq!(is_bar, i % 4 == 0, "beat {i}");
        }
        assert_eq!(first_beat, 4 * bar);
        for n in bar + 1..=bar + 4 {
            assert!(view.contains_rect(unique_text(&out, &format!("Bar {n}"))), "Bar {n}");
        }
        // Facts row: value with two decimals, in the facts rhythm, no badge
        // while confident, and a muted "low" badge below the threshold.
        let value = format!("{:.2}", stored.bpm());
        let key = unique_text(&out, crate::view::COMPUTED_BPM_LABEL);
        let value_rect = unique_text(&out, &value);
        assert!(panel.contains_rect(key) && panel.contains_rect(value_rect));
        assert!((key.center().y - value_rect.center().y).abs() <= 2.0, "one row");
        let pitch = top_in(&out, "Size", panel) - top_in(&out, "Bit depth", panel);
        let bpm_top = top_in(&out, crate::view::COMPUTED_BPM_LABEL, panel);
        assert!((bpm_top - top_in(&out, "Size", panel) - pitch).abs() <= 1.0, "Size -> BPM rhythm");
        assert!((top_in(&out, "Availability", panel) - bpm_top - pitch).abs() <= 1.0, "BPM -> Availability rhythm");
        let confident = stored.confidence() >= super::LOW_CONFIDENCE;
        assert_eq!(texts(&out).iter().any(|(t, _)| t == "low"), !confident);
        h.app.track_waveform.loaded.as_mut().unwrap().tempo.as_mut().unwrap().as_mut().unwrap().confidence = 0.1;
        let out = h.frame(vec![]);
        let low = unique_text(&out, "low");
        assert!(panel.contains_rect(low) && low.left() > value_rect.right(), "badge after the value {low:?}");
        assert!((low.center().y - value_rect.center().y).abs() <= 2.0);
        let tokens = ThemeMode::TokyoNight.tokens();
        let badge = out.shapes.iter().find_map(|c| match &c.shape {
            Shape::Rect(r) if r.rect.contains_rect(low) && r.stroke.color == tokens.muted_text => Some(r.rect),
            _ => None,
        });
        assert!(badge.is_some(), "muted-token badge outline around \"low\"");
        h.app.track_waveform.loaded.as_mut().unwrap().tempo.as_mut().unwrap().as_mut().unwrap().confidence = 0.9;
        assert!(!texts(&h.frame(vec![])).iter().any(|(t, _)| t == "low"));

        // 2. Imported and computed grids: the import wins (W6 window).
        let out = h.click(unique_text(&out, "v-both.wav").center());
        assert_eq!(h.app.selected, Some(id_of("v-both.wav").id.as_u64()));
        assert_eq!(h.app.track_waveform.layout.grid_window, Some(((5_250.0, 13_250.0), 2)));
        assert!(has_text(&out, super::GRID_SOURCE_LABEL));
        assert!(!has_text(&out, "beat grid: computed"));
        assert!(!has_text(&out, super::COMPUTED_GRID_NOTE));
        let view = h.app.track_waveform.layout.view.unwrap();
        let lines = grid_lines(&out, view);
        assert_eq!(lines.len(), 16);
        for (k, &(x, is_bar)) in lines.iter().enumerate() {
            let want = view.left() + (500.0 * k as f32) / 8_000.0 * view.width();
            assert!((x - want).abs() <= 1.01 && is_bar == (k % 4 == 0), "imported beat {k}");
        }
        // Its computed BPM is still a fact of the file.
        assert!(has_text(&out, crate::view::COMPUTED_BPM_LABEL));

        // 3. No grid at all: unchanged 8 s window, no BPM fact, no grid captions.
        let out = h.click(unique_text(&out, "n-none.wav").center());
        assert_eq!(h.app.selected, Some(id_of("n-none.wav").id.as_u64()));
        let layout = h.app.track_waveform.layout;
        assert!(layout.grid_window.is_none());
        let detail = layout.detail.unwrap();
        assert_eq!(detail.end_sample - detail.start_sample, 8 * 44_100);
        assert!(has_text(&out, "Detail 8 s window"));
        assert!(!has_text(&out, crate::view::COMPUTED_BPM_LABEL));
        assert!(!has_text(&out, "beat grid:"));
        assert!(!has_text(&out, super::COMPUTED_GRID_NOTE));
        assert!(grid_lines(&out, layout.view.unwrap()).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// W7 round 3: selection + details + every waveform style at a ~900 px
    /// window, rendered through the production `LibraryApp::ui`. Every gain
    /// and Position slider must be fully inside the viewport with a usable
    /// rail, and a real click on the Low rail must change the Low gain.
    #[test]
    fn narrow_window_keeps_details_waveform_styles_and_gain_sliders_reachable() {
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-w7-narrow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        write_wav(&audio.join("synthetic-tone.wav"), 2.0, sine(220.0, 0.8));
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .import_audio_directory(audio.to_str().unwrap())
            .unwrap();
        let id = catalog.list_track_details().unwrap()[0].track.id;
        oxidj_application::analyze_track_waveform(&mut catalog, id).unwrap();
        drop(catalog);

        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 900.0));
        let ctx = egui::Context::default();
        let mut app = LibraryApp::new(db, ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens());
        // W8: details is a toggled panel (off by default); open it as a user would.
        app.show_details = true;
        let frame = |app: &mut LibraryApp, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            ctx.run(input, |ctx| app.ui(ctx))
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let click = |app: &mut LibraryApp, pos: Pos2| {
            frame(app, vec![egui::Event::PointerMoved(pos), button(pos, true)]);
            frame(app, vec![button(pos, false)]);
            frame(app, vec![]);
            frame(app, vec![])
        };
        let first = click(&mut app, Pos2::new(1.0, 1.0));
        // Select through the production track table.
        click(&mut app, unique_text(&first, "synthetic-tone.wav").center());
        assert_eq!(app.selected, Some(id.as_u64()));

        for (index, style) in Style::ALL.into_iter().enumerate() {
            let styles = app.track_waveform.layout.styles.expect("style buttons");
            click(&mut app, styles[index].center());
            assert_eq!(app.track_waveform.style, style);
            let layout = &app.track_waveform.layout;
            let view = layout.view.expect("waveform drawn");
            let position = layout.position.expect("position slider");
            let eq = layout.eq.expect("gain sliders");
            for rect in styles.iter().chain(eq.iter()).chain([&position, &view]) {
                assert!(screen.contains_rect(*rect), "{style:?}: {rect:?} clipped at 900 px");
            }
            for rail in eq.iter().chain([&position]) {
                assert!(rail.width() >= 120.0 && rail.height() >= 18.0, "{style:?}: slider too small {rail:?}");
            }
            // The details panel leaves the track table >= 60% of the window.
            assert!(view.left() >= 900.0 * 0.6 - 24.0, "{style:?}: details panel too wide {view:?}");
        }

        // A real click near the left end of the Low rail lowers the gain.
        let low = app.track_waveform.layout.eq.unwrap()[0];
        assert_eq!(app.track_waveform.eq.low, 1.0);
        click(&mut app, Pos2::new(low.left() + 4.0, low.center().y));
        assert!(app.track_waveform.eq.low < 0.2, "Low rail reachable: {}", app.track_waveform.eq.low);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// W9: at the details panel's minimum width (750 px window → 320 px) and
    /// at 1440 px, in Tokyo Night and Day, for a grid + vendor track (the
    /// longest captions and the most content below the strip):
    /// captions wrap inside the panel; slider rows read label · rail · value
    /// with one aligned label column and aligned rails; the detail strip
    /// grows into spare height (96..=140 px) without pushing content out of
    /// the details panel, and drops to 96 px when the window is short.
    #[test]
    fn w9_captions_fit_slider_rows_align_and_strip_height_follows_details_height() {
        use oxidj_library::LibraryBeat;
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-w9-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let audio = root.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        write_wav(&audio.join("w9-grid-vendor.wav"), 20.0, sine(60.0, 0.8));
        // B2c: a computed-grid track (two grid captions, no vendor section).
        write_wav(&audio.join("w9-computed.wav"), 20.0, click(120.0));
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .import_audio_directory(audio.to_str().unwrap())
            .unwrap();
        let details = catalog.list_track_details().unwrap();
        let by_name = |name: &str| details.iter().find(|t| t.track.path.ends_with(name)).unwrap().track.clone();
        let track = by_name("w9-grid-vendor.wav");
        let computed = by_name("w9-computed.wav");
        oxidj_application::analyze_track_waveform(&mut catalog, track.id).unwrap();
        oxidj_application::analyze_track_waveform(&mut catalog, computed.id).unwrap();
        oxidj_application::analyze_track_tempo(&mut catalog, computed.id, oxidj_application::DEFAULT_TEMPO_RANGE).unwrap();
        let tempo = catalog.computed_beat_grid(computed.id).unwrap().expect("computed grid").grid;
        let computed_label = format!("beat grid: computed ({}), {:.2} BPM", tempo.algorithm(), tempo.bpm());
        let (location, _) = oxidj_application::location_uri(&track.path);
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "synthetic".into(),
                    location,
                    beat_grid: (0..40u32)
                        .map(|i| LibraryBeat {
                            time_ms: 250 + 500 * i,
                            tempo_hundredths_bpm: 12_000,
                            beat_in_bar: ((i + 2) % 4 + 1) as u8,
                        })
                        .collect(),
                    vendor_waveforms: vec![VendorWaveform::RekordboxColourDetail(vec![
                        [7, 0, 0, 31],
                        [0, 7, 0, 10],
                    ])],
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        drop(catalog);

        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        for (name, vendor) in [("w9-grid-vendor.wav", true), ("w9-computed.wav", false)] {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            for (w, h) in [(750.0, 900.0), (1440.0, 900.0), (1440.0, 600.0)] {
                let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h));
                let ctx = egui::Context::default();
                let mut app = LibraryApp::new(db.clone(), mode, mode.tokens());
                app.show_details = true;
                let frame = |app: &mut LibraryApp, events: Vec<egui::Event>| {
                    let input = egui::RawInput {
                        screen_rect: Some(screen),
                        events,
                        ..Default::default()
                    };
                    ctx.run(input, |ctx| app.ui(ctx))
                };
                let click = |app: &mut LibraryApp, pos: Pos2| {
                    frame(app, vec![egui::Event::PointerMoved(pos), button(pos, true)]);
                    frame(app, vec![button(pos, false)]);
                    frame(app, vec![]);
                    frame(app, vec![])
                };
                let first = frame(&mut app, vec![]);
                let out = click(&mut app, unique_text(&first, name).center());
                let tag = format!("{name} {mode:?} {w}x{h}");
                let panel = app.shell.details.expect("details panel");
                if w == 750.0 {
                    assert_eq!(panel.width(), crate::view::DETAILS_MIN_WIDTH, "{tag}: at the details minimum width");
                }
                let layout = app.track_waveform.layout;

                // 1. Captions: every caption glyph inside the panel and its clip
                // (at 600 px they sit below the scroll viewport and are culled).
                let detail_line = if vendor {
                    "Detail 4 bars from Bar 3 of 10 · track 20.0 s · 44100 Hz".to_owned()
                } else {
                    texts(&out)
                        .into_iter()
                        .map(|(t, _)| t)
                        .find(|t| t.starts_with("Detail 4 bars from Bar "))
                        // Culled below the viewport at 600 px (no captions checked there).
                        .unwrap_or_default()
                };
                let captions: Vec<&str> = match (h == 600.0, vendor) {
                    (true, _) => vec![],
                    (false, true) => vec![&detail_line, super::GRID_SOURCE_LABEL, super::GAIN_NOTE],
                    (false, false) => {
                        vec![&detail_line, &computed_label, super::COMPUTED_GRID_NOTE, super::GAIN_NOTE]
                    }
                };
                for &label in &captions {
                    let hits: Vec<_> = out
                        .shapes
                        .iter()
                        .filter_map(|c| match &c.shape {
                            Shape::Text(t) if t.galley.job.text == label => {
                                Some((t.visual_bounding_rect(), c.clip_rect))
                            }
                            _ => None,
                        })
                        .collect();
                    assert_eq!(hits.len(), 1, "{tag}: {label:?}");
                    let (rect, clip) = hits[0];
                    assert!(
                        panel.contains_rect(rect) && clip.contains_rect(rect),
                        "{tag}: {label:?} clipped: {rect:?} panel {panel:?} clip {clip:?}"
                    );
                }

                // 2. Slider rows: label column, then rail, then value.
                let eq = layout.eq.expect("gain sliders");
                let position = layout.position.expect("position slider");
                let labels = ["Low", "Mid", "High", "Position"].map(|l| unique_text(&out, l));
                let rows = [eq[0], eq[1], eq[2], position];
                for (label, row) in labels.iter().zip(rows) {
                    // Glyph bounds: allow ~1 px side bearing between letters.
                    assert!((label.left() - labels[0].left()).abs() <= 1.5, "{tag}: label column {labels:?}");
                    assert!(label.right() < row.left(), "{tag}: label before rail {label:?} {row:?}");
                    assert!(row.top() <= label.center().y && label.center().y <= row.bottom(), "{tag}: one row");
                    assert!((row.left() - rows[0].left()).abs() < 0.5, "{tag}: rails aligned {rows:?}");
                    assert!(row.height() >= 24.0 && panel.contains_rect(row), "{tag}: {row:?}");
                }
                let values: Vec<Rect> = texts(&out)
                    .into_iter()
                    .filter(|(t, _)| t == "1.00")
                    .map(|(_, r)| r)
                    .collect();
                assert_eq!(values.len(), 3, "{tag}: one value per gain");
                for (value, row) in values.iter().zip(&eq) {
                    let rail_end = position.right();
                    assert!(value.left() > rail_end, "{tag}: value after the rail {value:?}");
                    assert!(row.contains_rect(*value) && panel.contains_rect(*value), "{tag}: {value:?}");
                }
                for row in &eq[1..] {
                    assert!((row.right() - eq[0].right()).abs() < 0.5, "{tag}: values aligned");
                }

                // 3. Strip height follows the details height, never overflows.
                let view = layout.view.expect("detail strip");
                // Last content: the vendor strip, else the last caption
                // (culled below the viewport at 600 px).
                assert_eq!(layout.vendor.is_some(), vendor, "{tag}");
                let last = match vendor {
                    true => layout.vendor,
                    false => texts(&out).into_iter().find(|(t, _)| t == super::GAIN_NOTE).map(|t| t.1),
                };
                if h == 600.0 {
                    assert_eq!(view.height(), super::DETAIL_MIN_HEIGHT, "{tag}");
                } else {
                    assert!(view.height() > super::DETAIL_MIN_HEIGHT, "{tag}: spare height used {view:?}");
                    assert!(view.height() <= super::DETAIL_MAX_HEIGHT, "{tag}: {view:?}");
                    let last = last.expect("last content painted");
                    assert!(last.bottom() <= panel.bottom(), "{tag}: content stays inside {last:?} {panel:?}");
                }
                // Stable on later frames (no feedback with its own height).
                let again = frame(&mut app, vec![]);
                drop(again);
                assert_eq!(app.track_waveform.layout.view, Some(view), "{tag}: stable");
                assert_eq!(app.shell.details, Some(panel), "{tag}: panel unchanged");
            }
        }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
