//! Reusable vector waveform renderer (W3).
//!
//! [`waveform_view`] draws a [`WaveformPeaks`] into an egui rect as one
//! anti-aliased triangle-strip [`Mesh`] per layer (slanted edges between
//! column centres, a 1 px transparent fringe along the outline). It picks the mipmap level whose density is closest to
//! (but not below) one column per pixel, so per-frame cost is O(visible px),
//! independent of track length. EQ gains scale drawn band amplitude live; the
//! peaks themselves are never re-analysed or mutated. Layer meshes live in
//! egui temp memory behind `Arc`s and are cleared and refilled in place each
//! frame, so a steady state performs no heap allocation once egui has dropped
//! the previous frame's shapes.

use crate::theme::{self, ColorTokens};
use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Response, Sense, Shape, Ui, Vec2};
use oxidj_application::{WaveformBand, WaveformPeak, WaveformPeaks};
use std::sync::{Arc, Mutex};

/// Default height requested for the waveform rect.
pub const HEIGHT: f32 = 96.0;
/// Vertical inset keeping every quad (and the minimum-thickness line) inside the rect.
const INSET: f32 = 1.0;
/// A silent or fully attenuated column still paints a 1 px line.
const MIN_THICKNESS: f32 = 1.0;
/// Width of the transparent anti-aliasing fringe around each layer outline.
pub const FRINGE: f32 = 1.0;
/// Widest viewport honoured (guards absurd widths).
const MAX_WIDTH_PX: f32 = 16_384.0;

/// Visual encoding of the bands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// One envelope: per column the max of the gained band amplitudes.
    #[default]
    Mono,
    /// Three opaque, mirrored layers: low outermost, mid inside it, high as
    /// the innermost core.
    ThreeBand,
    /// One envelope coloured per column: gained low/mid/high energy → R/G/B.
    Rgb,
}

impl Style {
    pub const ALL: [Self; 3] = [Self::Mono, Self::ThreeBand, Self::Rgb];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mono => "Mono",
            Self::ThreeBand => "3-band",
            Self::Rgb => "RGB",
        }
    }
}

/// The visible sample range `[start_sample, end_sample)` and the requested pixel width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub start_sample: u64,
    pub end_sample: u64,
    pub width_px: f32,
}

impl Viewport {
    /// The whole track at `width_px`.
    pub fn whole(peaks: &WaveformPeaks, width_px: f32) -> Self {
        Self {
            start_sample: 0,
            end_sample: peaks.sample_count(),
            width_px,
        }
    }

    fn width(self) -> f32 {
        if self.width_px.is_finite() && self.width_px > 0.0 {
            self.width_px.min(MAX_WIDTH_PX)
        } else {
            0.0
        }
    }
}

/// Per-band display gains in `0..=1`. Values are clamped; NaN reads as unity
/// (a broken control must not hide the waveform).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eq {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

impl Default for Eq {
    fn default() -> Self {
        Self {
            low: 1.0,
            mid: 1.0,
            high: 1.0,
        }
    }
}

impl Eq {
    /// Sanitised `[low, mid, high]` gains.
    pub fn gains(self) -> [f32; 3] {
        [self.low, self.mid, self.high].map(|g| if g.is_nan() { 1.0 } else { g.clamp(0.0, 1.0) })
    }
}

/// Colours for each layer, derived from semantic theme tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub mono: Color32,
    pub low: Color32,
    pub mid: Color32,
    pub high: Color32,
    /// Rgb colour of a column with no remaining energy.
    pub silent: Color32,
    /// Backing plate drawn behind the waveform.
    pub plate: Color32,
    /// Hairline through the vertical centre.
    pub centre: Color32,
}

impl Palette {
    pub fn from_tokens(t: ColorTokens) -> Self {
        Self {
            mono: t.secondary,
            low: t.secondary,
            mid: t.warning,
            high: t.text,
            silent: t.muted_text,
            plate: t.background,
            centre: Color32::from_rgba_unmultiplied(
                t.muted_text.r(),
                t.muted_text.g(),
                t.muted_text.b(),
                48,
            ),
        }
    }
}

/// Mipmap level for `samples` visible samples across `px` pixels: the
/// coarsest level that still has at least one column per pixel (so each pixel
/// unions at most three columns).
pub fn pick_level(peaks: &WaveformPeaks, samples: u64, px: usize) -> usize {
    if px == 0 || samples == 0 {
        return 0;
    }
    let cols_per_px = samples as f64 / f64::from(peaks.samples_per_peak()) / px as f64;
    if cols_per_px < 2.0 {
        0
    } else {
        (cols_per_px.log2().floor() as usize).min(peaks.level_count() - 1)
    }
}

/// Signed amplitude of a quantized peak: `(top, bottom)` in `[-1, 1]`.
fn amplitude(p: WaveformPeak) -> (f32, f32) {
    let s = |v: u8| ((f32::from(v) - 128.0) / 127.0).clamp(-1.0, 1.0);
    (s(p.max()), s(p.min()))
}

/// Vertices emitted per strip point: outer fringe, body edge, body edge, outer fringe.
pub const VERTS_PER_POINT: usize = 4;
/// Triangles per strip segment: top fringe, body, bottom fringe (two each).
const TRIS_PER_SEGMENT: usize = 6;

/// Fill `layers` (cleared first) with one anti-aliased triangle strip per
/// layer for one frame and return the mip level used. Pure: no egui context
/// needed. `layers[0]` is the mono/RGB envelope or the back (largest) 3-band
/// layer; `[1]`/`[2]` are only used by [`Style::ThreeBand`].
///
/// Each layer is one polygon: per drawn pixel column a point at the column
/// centre (plus one at each end so the shape spans the full columns), whose
/// top and bottom extents are joined to the next point by slanted segments.
/// Every point carries [`VERTS_PER_POINT`] vertices; the outer two form a
/// [`FRINGE`]-px fully transparent band offset along the outline normal, so
/// the GPU's colour interpolation fades the edge (anti-aliasing without
/// textures). Vertex colours give Rgb a smooth per-column gradient.
pub fn build_layers(
    peaks: &WaveformPeaks,
    style: Style,
    viewport: Viewport,
    eq: Eq,
    rect: Rect,
    palette: Palette,
    layers: &mut [Mesh; 3],
) -> usize {
    for mesh in layers.iter_mut() {
        // Not `Mesh::clear()`: in epaint 0.32 it drops the vertex buffer
        // (`vertices = Default::default()`), forcing a per-frame allocation.
        mesh.vertices.clear();
        mesh.indices.clear();
    }
    let px = if rect.is_finite() && rect.width() >= 1.0 {
        rect.width().floor() as usize
    } else {
        0
    };
    let start = viewport.start_sample;
    let span = viewport.end_sample.saturating_sub(start);
    let level = pick_level(peaks, span, px);
    if px == 0 || span == 0 {
        return level;
    }
    let bands = WaveformBand::ALL.map(|b| peaks.band(level, b).unwrap_or(&[]));
    let len = bands[0].len();
    let col_samples = u128::from(peaks.samples_per_peak()) << level;
    let gains = eq.gains();
    let w = rect.width() / px as f32;
    let cy = rect.center().y;
    let half = (rect.height() / 2.0 - INSET).max(0.0);
    let used = if style == Style::ThreeBand { 3 } else { 1 };
    for mesh in layers.iter_mut().take(used) {
        mesh.reserve_vertices((px + 2) * VERTS_PER_POINT);
        mesh.reserve_triangles((px + 1) * TRIS_PER_SEGMENT);
    }
    // Body edge y for a (top, bottom) amplitude pair, at least MIN_THICKNESS apart.
    let edges = |(top, bottom): (f32, f32)| {
        let (mut y0, mut y1) = (cy - top * half, cy - bottom * half);
        if y1 - y0 < MIN_THICKNESS {
            let mid = (y0 + y1) / 2.0;
            (y0, y1) = (mid - MIN_THICKNESS / 2.0, mid + MIN_THICKNESS / 2.0);
        }
        (y0, y1)
    };
    // Per layer: the last column's (top, bottom, colour), repeated at the right end.
    let mut last = [(0.0_f32, 0.0_f32, Color32::TRANSPARENT); 3];
    let mut drawn = 0usize;
    for i in 0..px {
        let s0 = u128::from(start) + u128::from(span) * i as u128 / px as u128;
        let s1 = u128::from(start) + u128::from(span) * (i as u128 + 1) / px as u128;
        let c0 = (s0 / col_samples) as usize;
        if c0 >= len {
            break; // beyond the end of the track
        }
        let c1 = (s1.div_ceil(col_samples) as usize).clamp(c0 + 1, len);
        // Gained (top, bottom) per band for this pixel.
        let band_amp: [(f32, f32); 3] = core::array::from_fn(|b| {
            let p = bands[b][c0 + 1..c1]
                .iter()
                .fold(bands[b][c0], |acc, p| acc.union(*p));
            let (t, bo) = amplitude(p);
            (t * gains[b], bo * gains[b])
        });
        let mut column = [(0.0_f32, 0.0_f32, Color32::TRANSPARENT); 3];
        match style {
            Style::ThreeBand => {
                let colors = [palette.low, palette.mid, palette.high];
                // Mirrored about the centre line: the louder side sets the extent.
                let ext: [f32; 3] = core::array::from_fn(|b| band_amp[b].0.max(-band_amp[b].1));
                // Nest by rank: the largest extent goes to the back layer, the
                // smallest to the front, so no band is ever covered by a larger
                // one. Each keeps its own colour at its own true extent. Linear
                // interpolation between columns preserves this order, so the
                // strips stay nested between points too.
                let mut order = [0usize, 1, 2];
                order.sort_unstable_by(|&a, &b| ext[b].total_cmp(&ext[a]).then(a.cmp(&b)));
                for (rank, &b) in order.iter().enumerate() {
                    let (y0, y1) = edges((ext[b], -ext[b]));
                    column[rank] = (y0, y1, colors[b]);
                }
            }
            Style::Mono | Style::Rgb => {
                let top = band_amp.iter().map(|a| a.0).fold(f32::MIN, f32::max);
                let bottom = band_amp.iter().map(|a| a.1).fold(f32::MAX, f32::min);
                let color = if style == Style::Mono {
                    palette.mono
                } else {
                    rgb_color(band_amp, palette.silent)
                };
                let (y0, y1) = edges((top, bottom));
                column[0] = (y0, y1, color);
            }
        }
        let xc = rect.left() + (i as f32 + 0.5) * w;
        for (mesh, &(y0, y1, color)) in layers.iter_mut().zip(&column).take(used) {
            if i == 0 {
                push_point(mesh, rect.left(), y0, y1, color);
            }
            push_point(mesh, xc, y0, y1, color);
        }
        last = column;
        drawn = i + 1;
    }
    if drawn > 0 {
        let x_end = (rect.left() + drawn as f32 * w).min(rect.right());
        for (mesh, &(y0, y1, color)) in layers.iter_mut().zip(&last).take(used) {
            push_point(mesh, x_end, y0, y1, color);
            finish_strip(mesh, rect);
        }
    }
    level
}

/// Append one strip point: `[fringe, top, bottom, fringe]`, fringe still at
/// the body position (moved outward by [`finish_strip`]) and transparent.
fn push_point(mesh: &mut Mesh, x: f32, top: f32, bottom: f32, color: Color32) {
    let v = |y: f32, color: Color32| egui::epaint::Vertex {
        pos: Pos2::new(x, y),
        uv: egui::epaint::WHITE_UV,
        color,
    };
    let n = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        v(top, Color32::TRANSPARENT),
        v(top, color),
        v(bottom, color),
        v(bottom, Color32::TRANSPARENT),
    ]);
    if n >= VERTS_PER_POINT as u32 {
        let a = n - VERTS_PER_POINT as u32;
        let b = n;
        // Top fringe, body, bottom fringe: two triangles each.
        for k in 0..3 {
            mesh.indices
                .extend_from_slice(&[a + k, a + k + 1, b + k, a + k + 1, b + k + 1, b + k]);
        }
    }
}

/// Move every fringe vertex [`FRINGE`] px outward along the outline normal
/// (averaged over the neighbouring segments), clamped into `rect`.
fn finish_strip(mesh: &mut Mesh, rect: Rect) {
    let points = mesh.vertices.len() / VERTS_PER_POINT;
    for j in 0..points {
        let prev = j.saturating_sub(1) * VERTS_PER_POINT;
        let next = (j + 1).min(points - 1) * VERTS_PER_POINT;
        let at = j * VERTS_PER_POINT;
        // Top edge (vertex 1) normal points up, bottom edge (vertex 2) down.
        for (body, fringe, sign) in [(1, 0, -1.0_f32), (2, 3, 1.0)] {
            let t = mesh.vertices[next + body].pos - mesh.vertices[prev + body].pos;
            let n = Vec2::new(-t.y, t.x).normalized() * sign;
            let n = if n.is_finite() && n != Vec2::ZERO {
                n
            } else {
                Vec2::new(0.0, sign)
            };
            let p = mesh.vertices[at + body].pos + n * FRINGE;
            mesh.vertices[at + fringe].pos = p.clamp(rect.min, rect.max);
        }
    }
}

/// Gained band energies (half peak-to-peak) mapped to R/G/B, normalised to the
/// loudest band so the hue reflects the spectral balance.
fn rgb_color(band_amp: [(f32, f32); 3], silent: Color32) -> Color32 {
    let e = band_amp.map(|(t, b)| ((t - b) / 2.0).max(0.0));
    let m = e.iter().copied().fold(0.0, f32::max);
    if m <= f32::EPSILON {
        return silent;
    }
    let c = e.map(|v| (v / m * 255.0).round() as u8);
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Reused layer buffers, keyed by the widget id in egui temp memory.
#[derive(Clone, Default)]
struct LayerBuffers(Arc<Mutex<[Arc<Mesh>; 3]>>);

/// Draw `peaks` (or nothing for `None`) at `viewport` with `style` and `eq`.
/// Geometry is derived from the allocated response rect and clipped to it.
pub fn waveform_view(
    ui: &mut Ui,
    peaks: Option<&WaveformPeaks>,
    style: Style,
    viewport: Viewport,
    eq: Eq,
) -> Response {
    waveform_view_sized(ui, peaks, style, viewport, eq, HEIGHT)
}

/// [`waveform_view`] with an explicit strip `height` (e.g. a slim overview).
pub fn waveform_view_sized(
    ui: &mut Ui,
    peaks: Option<&WaveformPeaks>,
    style: Style,
    viewport: Viewport,
    eq: Eq,
    height: f32,
) -> Response {
    let response = ui.allocate_response(Vec2::new(viewport.width(), height), Sense::hover());
    let Some(peaks) = peaks else {
        return response;
    };
    let rect = response.rect;
    let palette = Palette::from_tokens(theme::current_tokens(ui.ctx()));
    let buffers = ui.data_mut(|d| {
        d.get_temp_mut_or_default::<LayerBuffers>(response.id)
            .clone()
    });
    let mut slots = buffers.0.lock().unwrap_or_else(|e| e.into_inner());
    // Take unique ownership of each mesh; egui has normally dropped last
    // frame's shapes by now, in which case this is allocation-free.
    // `mem::take` on a `Mesh` moves its buffers out without allocating.
    let mut owned: [Mesh; 3] = slots
        .each_mut()
        .map(|slot| Arc::get_mut(slot).map(std::mem::take).unwrap_or_default());
    build_layers(peaks, style, viewport, eq, rect, palette, &mut owned);
    for (slot, mesh) in slots.iter_mut().zip(owned) {
        match Arc::get_mut(slot) {
            Some(dst) => *dst = mesh,
            None => *slot = Arc::new(mesh),
        }
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, palette.plate);
    painter.hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0_f32, palette.centre),
    );
    for slot in slots.iter() {
        if !slot.is_empty() {
            painter.add(Shape::Mesh(Arc::clone(slot)));
        }
    }
    response
}

/// Deterministic synthetic peaks for the demo and tests: `seconds` of audio
/// at 44.1 kHz / 150 peaks per second with a kick-like low band, a phrase-
/// modulated mid band and hats in the high band.
pub fn synthetic_peaks(seconds: u32) -> WaveformPeaks {
    const RATE: u32 = 44_100;
    const SPP: u32 = 294;
    let samples = u64::from(RATE) * u64::from(seconds.max(1));
    let cols = samples.div_ceil(u64::from(SPP)) as usize;
    let peak = |a: f32| {
        let a = a.clamp(0.0, 1.0);
        WaveformPeak::from_f32(-a, a).unwrap_or(WaveformPeak::ZERO)
    };
    let mut bands: [Vec<WaveformPeak>; 3] = Default::default();
    for c in 0..cols {
        let t = c as f32 / 150.0;
        let beat = (t * 2.0).fract(); // 120 bpm
        let phrase = 0.5 + 0.5 * (t * std::f32::consts::TAU / 16.0).sin();
        bands[0].push(peak(0.9 * (1.0 - beat).powi(3)));
        bands[1].push(peak(0.25 + 0.45 * phrase));
        bands[2].push(peak(if (t * 8.0).fract() < 0.15 { 0.55 } else { 0.12 }));
    }
    WaveformPeaks::from_base(RATE, SPP, samples, bands).expect("valid synthetic peaks")
}

/// View state for the provisional demo window.
pub(crate) struct DemoState {
    pub(crate) style: Style,
    pub(crate) eq: Eq,
    /// Visible fraction of the track, `0.01..=1`.
    pub(crate) zoom: f32,
    pub(crate) peaks: WaveformPeaks,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            style: Style::default(),
            eq: Eq::default(),
            zoom: 1.0,
            peaks: synthetic_peaks(180),
        }
    }
}

/// Demo body: style selector, three EQ sliders, zoom and the view itself.
pub(crate) fn show_demo(ui: &mut Ui, state: &mut DemoState) -> Response {
    ui.label("Synthetic peaks only — no catalog track or audio.");
    ui.horizontal(|ui| {
        ui.label("Style");
        for style in Style::ALL {
            ui.selectable_value(&mut state.style, style, style.label());
        }
    });
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut state.eq.low, 0.0..=1.0).text("Low"));
        ui.add(egui::Slider::new(&mut state.eq.mid, 0.0..=1.0).text("Mid"));
        ui.add(egui::Slider::new(&mut state.eq.high, 0.0..=1.0).text("High"));
    });
    ui.add(egui::Slider::new(&mut state.zoom, 0.01..=1.0).text("Visible fraction"));
    let span = (state.peaks.sample_count() as f64 * f64::from(state.zoom.clamp(0.01, 1.0))) as u64;
    let viewport = Viewport {
        end_sample: span.max(1),
        ..Viewport::whole(&state.peaks, ui.available_width())
    };
    waveform_view(ui, Some(&state.peaks), state.style, viewport, state.eq)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens() -> ColorTokens {
        theme::ThemeMode::TokyoNight.tokens()
    }

    /// Render `waveform_view` in a central panel and return its rect, the
    /// clipped shapes it produced, and the layer vertex buffer pointers.
    fn render(
        ctx: &egui::Context,
        peaks: Option<&WaveformPeaks>,
        style: Style,
        viewport: Viewport,
        eq: Eq,
    ) -> (Rect, Vec<egui::epaint::ClippedShape>) {
        let mut rect = Rect::NOTHING;
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            theme::publish_tokens(ctx, tokens());
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = waveform_view(ui, peaks, style, viewport, eq).rect;
            });
        });
        let shapes = output
            .shapes
            .into_iter()
            .filter(|c| matches!(c.shape, Shape::Mesh(_)))
            .collect();
        (rect, shapes)
    }

    fn meshes(shapes: &[egui::epaint::ClippedShape]) -> Vec<Arc<Mesh>> {
        shapes
            .iter()
            .filter_map(|c| match &c.shape {
                Shape::Mesh(m) => Some(Arc::clone(m)),
                _ => None,
            })
            .collect()
    }

    fn ten_minutes() -> WaveformPeaks {
        synthetic_peaks(600)
    }

    #[test]
    fn painted_bounds_stay_inside_clip_rect_at_min_mid_max_zoom_and_gain() {
        let peaks = ten_minutes();
        let n = peaks.sample_count();
        let ctx = egui::Context::default();
        // min zoom = whole track, mid = one minute, max = fewer samples than pixels.
        let zooms = [(0, n), (n / 3, n / 3 + n / 10), (n / 2, n / 2 + 300)];
        let gains = [0.0, 0.5, 1.0];
        for style in Style::ALL {
            for (start, end) in zooms {
                for g in gains {
                    let eq = Eq {
                        low: g,
                        mid: g,
                        high: g,
                    };
                    let vp = Viewport {
                        start_sample: start,
                        end_sample: end,
                        width_px: 600.0,
                    };
                    let (rect, shapes) = render(&ctx, Some(&peaks), style, vp, eq);
                    assert!(!shapes.is_empty(), "{style:?} {start}..{end} g={g}");
                    for clipped in &shapes {
                        // Meshes carry no stroke, so the visual bounds are the full extent.
                        let bounds = clipped.shape.visual_bounding_rect();
                        assert!(
                            clipped.clip_rect.contains_rect(bounds) && rect.contains_rect(bounds),
                            "{style:?} {start}..{end} g={g}: {bounds:?} outside {:?}/{rect:?}",
                            clipped.clip_rect
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn zero_gains_draw_a_flat_centre_line() {
        let peaks = synthetic_peaks(30);
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(400.0, HEIGHT));
        let mut layers: [Mesh; 3] = Default::default();
        for style in Style::ALL {
            let eq = Eq {
                low: 0.0,
                mid: -3.0,
                high: 0.0,
            };
            build_layers(
                &peaks,
                style,
                Viewport::whole(&peaks, 400.0),
                eq,
                rect,
                Palette::from_tokens(tokens()),
                &mut layers,
            );
            let verts: Vec<_> = layers.iter().flat_map(|m| m.vertices.iter()).collect();
            assert!(verts.iter().any(|v| v.color.a() == 255));
            for v in verts {
                // Opaque body: the 1 px line; transparent fringe: 1 px outside it.
                let limit = if v.color.a() == 0 {
                    MIN_THICKNESS / 2.0 + FRINGE
                } else {
                    MIN_THICKNESS / 2.0
                };
                assert!(
                    (v.pos.y - rect.center().y).abs() <= limit + 1e-4,
                    "{style:?} {v:?}"
                );
            }
        }
    }

    #[test]
    fn eq_gain_scales_band_amplitude_live_without_touching_peaks() {
        let peaks = synthetic_peaks(30);
        let before = peaks.clone();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, HEIGHT));
        let mut layers: [Mesh; 3] = Default::default();
        let pal = Palette::from_tokens(tokens());
        // Layers are ranked by extent, so measure a band by its colour: the
        // largest vertical span over every vertex painted in that band's colour.
        let height = |layers: &[Mesh; 3], i: usize| {
            let colour = [pal.low, pal.mid, pal.high][i];
            let ys = layers
                .iter()
                .flat_map(|m| m.vertices.iter())
                .filter(|v| v.color == colour)
                .map(|v| v.pos.y);
            let (lo, hi) = ys.fold((f32::MAX, f32::MIN), |(l, h), y| (l.min(y), h.max(y)));
            hi - lo
        };
        let vp = Viewport::whole(&peaks, 300.0);
        build_layers(
            &peaks,
            Style::ThreeBand,
            vp,
            Eq::default(),
            rect,
            pal,
            &mut layers,
        );
        let full = [0, 1, 2].map(|i| height(&layers, i));
        build_layers(
            &peaks,
            Style::ThreeBand,
            vp,
            Eq {
                low: 0.5,
                mid: 1.0,
                high: f32::NAN,
            },
            rect,
            pal,
            &mut layers,
        );
        let low_half = height(&layers, 0);
        assert!(
            (low_half - full[0] / 2.0).abs() < 2.0,
            "{low_half} vs {}",
            full[0]
        );
        assert_eq!(height(&layers, 1), full[1]);
        assert_eq!(height(&layers, 2), full[2], "NaN gain reads as unity");
        // Out-of-range gains clamp.
        build_layers(
            &peaks,
            Style::ThreeBand,
            vp,
            Eq {
                low: 7.0,
                mid: 1.0,
                high: 1.0,
            },
            rect,
            pal,
            &mut layers,
        );
        assert_eq!(height(&layers, 0), full[0]);
        assert_eq!(peaks, before);
        // Rgb: killing low removes the red channel everywhere.
        build_layers(
            &peaks,
            Style::Rgb,
            vp,
            Eq {
                low: 0.0,
                mid: 1.0,
                high: 1.0,
            },
            rect,
            pal,
            &mut layers,
        );
        assert!(layers[0].vertices.iter().all(|v| v.color.r() == 0));
        build_layers(
            &peaks,
            Style::Rgb,
            vp,
            Eq::default(),
            rect,
            pal,
            &mut layers,
        );
        assert!(layers[0].vertices.iter().any(|v| v.color.r() > 0));
    }

    /// Constant-amplitude peaks per band: (low, mid, high), `cols` columns.
    fn flat_peaks(amps: [f32; 3], cols: usize) -> WaveformPeaks {
        let band = |a: f32| vec![WaveformPeak::from_f32(-a, a).unwrap(); cols];
        WaveformPeaks::from_base(
            44_100,
            294,
            cols as u64 * 294,
            [band(amps[0]), band(amps[1]), band(amps[2])],
        )
        .expect("valid flat peaks")
    }

    /// Extents (half-height above the centre line) and colour of each layer's
    /// first column, back to front, after mirroring.
    fn layer_extents(amps: [f32; 3], eq: Eq) -> Vec<(f32, Color32)> {
        let peaks = flat_peaks(amps, 64);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(32.0, HEIGHT));
        let mut layers: [Mesh; 3] = Default::default();
        let pal = Palette::from_tokens(tokens());
        build_layers(
            &peaks,
            Style::ThreeBand,
            Viewport::whole(&peaks, 32.0),
            eq,
            rect,
            pal,
            &mut layers,
        );
        layers
            .iter()
            .map(|m| {
                let c = rect.center().y;
                let span = |opaque: bool| {
                    m.vertices
                        .iter()
                        .filter(|v| (v.color.a() == 255) == opaque)
                        .fold((f32::MAX, f32::MIN), |(l, h), v| {
                            (l.min(v.pos.y), h.max(v.pos.y))
                        })
                };
                // The strip is the body (opaque) plus a transparent fringe;
                // there is no third kind of vertex.
                assert!(
                    m.vertices
                        .iter()
                        .all(|v| v.color.a() == 255 || v.color == Color32::TRANSPARENT)
                );
                let (lo, hi) = span(true);
                let (flo, fhi) = span(false);
                // Mirror symmetry about the centre line, body and fringe.
                assert!(((c - lo) - (hi - c)).abs() < 1e-3, "{lo} {hi} vs {c}");
                assert!(((c - flo) - (fhi - c)).abs() < 1e-3, "{flo} {fhi} vs {c}");
                // Fringe hugs the body outline (clamped into the rect).
                assert!(lo - flo <= FRINGE + 1e-3 && flo <= lo, "{flo} {lo}");
                let body: Vec<_> = m.vertices.iter().filter(|v| v.color.a() == 255).collect();
                assert!(!body.is_empty());
                assert!(body.iter().all(|v| v.color == body[0].color));
                assert_eq!(body[0].color.a(), 255, "layers are opaque");
                (c - lo, body[0].color)
            })
            .collect()
    }

    #[test]
    fn three_band_layers_are_nested_by_extent_in_every_dominance_case() {
        let pal = Palette::from_tokens(tokens());
        let colours = [pal.low, pal.mid, pal.high];
        for amps in [
            [0.9, 0.5, 0.2], // low-heavy
            [0.1, 0.8, 0.3], // mid-heavy
            [0.1, 0.2, 0.8], // high-heavy: used to cover low and mid entirely
            [0.4, 0.4, 0.4], // ties keep band order, still nested (equal)
        ] {
            let l = layer_extents(amps, Eq::default());
            assert!(l[0].0 >= l[1].0 && l[1].0 >= l[2].0, "{amps:?}: {l:?}");
            // Every band is still drawn once, in its own colour, at its own extent.
            for (b, &colour) in colours.iter().enumerate() {
                let layer = l
                    .iter()
                    .find(|(_, c)| *c == colour)
                    .unwrap_or_else(|| panic!("{amps:?} band {b} missing"));
                let want = amps[b] * (HEIGHT / 2.0 - INSET);
                assert!(
                    (layer.0 - want).abs() < 1.5,
                    "{amps:?} band {b}: {} vs {want}",
                    layer.0
                );
            }
        }
        // High-heavy: the high band is the back (outermost) layer.
        let l = layer_extents([0.1, 0.2, 0.8], Eq::default());
        assert_eq!(l[0].1, pal.high);
        assert_eq!(l[2].1, pal.low);
        // EQ re-ranks live: cutting the dominant high band makes mid the outermost.
        let l = layer_extents(
            [0.1, 0.2, 0.8],
            Eq {
                low: 1.0,
                mid: 1.0,
                high: 0.0,
            },
        );
        assert_eq!(l[0].1, pal.mid);
        assert!(l[0].0 >= l[1].0 && l[1].0 >= l[2].0, "{l:?}");
    }

    #[test]
    fn style_switch_changes_shapes_not_peaks() {
        let peaks = synthetic_peaks(30);
        let before = peaks.clone();
        let ctx = egui::Context::default();
        let vp = Viewport::whole(&peaks, 500.0);
        let mut seen = vec![];
        for style in Style::ALL {
            let (_, shapes) = render(&ctx, Some(&peaks), style, vp, Eq::default());
            let m = meshes(&shapes);
            let expected = if style == Style::ThreeBand { 3 } else { 1 };
            assert_eq!(m.len(), expected, "{style:?}");
            seen.push(
                m.iter()
                    .map(|m| {
                        (
                            m.vertices.len(),
                            m.vertices.iter().map(|v| v.color).collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        assert_ne!(seen[0], seen[1]);
        assert_ne!(seen[0], seen[2]);
        assert_ne!(seen[1], seen[2]);
        assert_eq!(peaks, before);
    }

    #[test]
    fn empty_short_and_degenerate_viewports_are_safe() {
        let ctx = egui::Context::default();
        let short = WaveformPeaks::from_base(
            44_100,
            294,
            1,
            [
                vec![WaveformPeak::new(0, 255).unwrap()],
                vec![WaveformPeak::ZERO],
                vec![WaveformPeak::ZERO],
            ],
        )
        .unwrap();
        // No peaks: nothing painted.
        let (_, shapes) = render(
            &ctx,
            None,
            Style::Mono,
            Viewport {
                start_sample: 0,
                end_sample: 10,
                width_px: 300.0,
            },
            Eq::default(),
        );
        assert!(shapes.is_empty());
        for style in Style::ALL {
            // One-column track: one quad per layer, inside the rect.
            let (rect, shapes) = render(
                &ctx,
                Some(&short),
                style,
                Viewport::whole(&short, 300.0),
                Eq::default(),
            );
            assert!(!shapes.is_empty());
            for c in &shapes {
                assert!(rect.contains_rect(c.shape.visual_bounding_rect()));
            }
            for vp in [
                Viewport {
                    start_sample: 5,
                    end_sample: 5,
                    width_px: 300.0,
                },
                Viewport {
                    start_sample: 9,
                    end_sample: 2,
                    width_px: 300.0,
                },
                Viewport {
                    start_sample: 10_000,
                    end_sample: 20_000,
                    width_px: 300.0,
                },
                Viewport {
                    start_sample: 0,
                    end_sample: u64::MAX,
                    width_px: 300.0,
                },
                Viewport {
                    start_sample: 0,
                    end_sample: 1,
                    width_px: f32::NAN,
                },
                Viewport {
                    start_sample: 0,
                    end_sample: 1,
                    width_px: -4.0,
                },
                Viewport {
                    start_sample: 0,
                    end_sample: 1,
                    width_px: f32::INFINITY,
                },
            ] {
                let (rect, shapes) = render(&ctx, Some(&short), style, vp, Eq::default());
                for c in &shapes {
                    assert!(rect.contains_rect(c.shape.visual_bounding_rect()), "{vp:?}");
                }
            }
        }
    }

    #[test]
    fn mip_level_tracks_pixel_density_so_cost_is_per_pixel() {
        let peaks = ten_minutes();
        let n = peaks.sample_count();
        let cols0 = peaks.band(0, WaveformBand::Low).unwrap().len();
        assert_eq!(
            pick_level(&peaks, n, 1440),
            (cols0 as f64 / 1440.0).log2().floor() as usize
        );
        assert_eq!(pick_level(&peaks, 300, 1440), 0);
        // One pixel: the coarsest level that still has >= 1 column per pixel.
        let one_px = pick_level(&peaks, n, 1);
        assert!(!peaks.band(one_px, WaveformBand::Low).unwrap().is_empty());
        assert!(one_px >= peaks.level_count() - 2);
        assert_eq!(pick_level(&peaks, n, 0), 0);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, HEIGHT));
        let mut layers: [Mesh; 3] = Default::default();
        for (start, end) in [(0, n), (n / 4, n / 4 + n / 20), (n / 2, n / 2 + 200)] {
            let vp = Viewport {
                start_sample: start,
                end_sample: end,
                width_px: 1440.0,
            };
            let level = build_layers(
                &peaks,
                Style::ThreeBand,
                vp,
                Eq::default(),
                rect,
                Palette::from_tokens(tokens()),
                &mut layers,
            );
            let cols = peaks.band(level, WaveformBand::Low).unwrap().len() as u64;
            let visible_cols = (end - start) as f64 / f64::from(peaks.samples_per_peak() << level);
            assert!(
                visible_cols < 2.0 * 1440.0 || level == peaks.level_count() - 1,
                "level {level} cols {cols}"
            );
            for m in &layers {
                assert!(
                    m.vertices.len() <= (1440 + 2) * VERTS_PER_POINT,
                    "geometry is O(visible px)"
                );
            }
        }
    }

    #[test]
    fn mip_drawn_envelope_matches_level_zero_union() {
        // The mip shortcut must not change what is drawn: compare with a naive
        // per-pixel union over level 0.
        let peaks = synthetic_peaks(120);
        let n = peaks.sample_count();
        let px = 512;
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(px as f32, HEIGHT));
        let mut layers: [Mesh; 3] = Default::default();
        let level = build_layers(
            &peaks,
            Style::Mono,
            Viewport::whole(&peaks, px as f32),
            Eq::default(),
            rect,
            Palette::from_tokens(tokens()),
            &mut layers,
        );
        assert!(level > 0);
        let low0 = peaks.band(0, WaveformBand::Low).unwrap();
        let spp = u64::from(peaks.samples_per_peak());
        let scale = 1usize << level;
        let y_of = |q: u8| {
            rect.center().y
                - ((f32::from(q) - 128.0) / 127.0).clamp(-1.0, 1.0) * (HEIGHT / 2.0 - INSET)
        };
        let top_over = |a: usize, b: usize| {
            (a..b.min(low0.len()))
                .flat_map(|c| WaveformBand::ALL.map(|band| peaks.band(0, band).unwrap()[c].max()))
                .max()
                .unwrap()
        };
        for i in 0..px {
            let s0 = n * i as u64 / px as u64;
            let s1 = n * (i as u64 + 1) / px as u64;
            let c0 = (s0 / spp) as usize;
            let c1 = s1.div_ceil(spp) as usize;
            // Exact level-0 union, and the same range widened to mip-column alignment.
            let exact = y_of(top_over(c0, c1.max(c0 + 1)));
            let widened = y_of(top_over(c0 / scale * scale, c1.div_ceil(scale) * scale));
            // Point 0 is the left end; column i's top body vertex follows.
            let drawn = layers[0].vertices[(i + 1) * VERTS_PER_POINT + 1].pos.y;
            // y grows downward: drawn is at least the exact peak, at most the widened one.
            assert!(
                drawn <= exact + 1e-3 && drawn >= widened - 1e-3,
                "pixel {i}: {widened} <= {drawn} <= {exact}"
            );
        }
    }

    #[test]
    fn layers_are_slanted_strips_with_a_soft_fringe_and_rgb_gradient() {
        let peaks = synthetic_peaks(8);
        let rect = Rect::from_min_size(Pos2::new(5.0, 7.0), Vec2::new(200.0, HEIGHT));
        let vp = Viewport::whole(&peaks, 200.0);
        let pal = Palette::from_tokens(tokens());
        let mut layers: [Mesh; 3] = Default::default();
        for style in Style::ALL {
            build_layers(&peaks, style, vp, Eq::default(), rect, pal, &mut layers);
            let used = if style == Style::ThreeBand { 3 } else { 1 };
            for (k, m) in layers.iter().enumerate() {
                if k >= used {
                    assert!(m.is_empty(), "{style:?} unused layer {k}");
                    continue;
                }
                let points = m.vertices.len() / VERTS_PER_POINT;
                assert_eq!(points, 202, "{style:?}: one point per column + 2 ends");
                assert_eq!(m.vertices.len() % VERTS_PER_POINT, 0);
                // One connected strip: 6 triangles per segment, all indices valid.
                assert_eq!(m.indices.len(), (points - 1) * TRIS_PER_SEGMENT * 3);
                assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
                let pt = |j: usize, k: usize| m.vertices[j * VERTS_PER_POINT + k];
                let mut slanted = 0;
                for j in 0..points {
                    // Layout: transparent fringe, opaque top, opaque bottom, transparent fringe.
                    assert_eq!(pt(j, 0).color, Color32::TRANSPARENT);
                    assert_eq!(pt(j, 3).color, Color32::TRANSPARENT);
                    assert_eq!(pt(j, 1).color.a(), 255);
                    assert_eq!(pt(j, 1).color, pt(j, 2).color);
                    assert!(pt(j, 0).pos.y <= pt(j, 1).pos.y && pt(j, 1).pos.y < pt(j, 2).pos.y);
                    assert!(pt(j, 2).pos.y <= pt(j, 3).pos.y);
                    // The fringe sits ~1 px from the outline (closer only where clamped).
                    for (f, b) in [(0, 1), (3, 2)] {
                        let d = pt(j, f).pos.distance(pt(j, b).pos);
                        let clamped = [rect.top(), rect.bottom()].contains(&pt(j, f).pos.y)
                            || [rect.left(), rect.right()].contains(&pt(j, f).pos.x);
                        assert!(
                            (d - FRINGE).abs() < 1e-3 || (clamped && d <= FRINGE),
                            "{style:?} point {j}: fringe {d}"
                        );
                        assert!(rect.contains(pt(j, f).pos));
                    }
                    if j > 0 {
                        // No vertical stair-steps: x strictly increases point to point.
                        assert!(pt(j, 1).pos.x > pt(j - 1, 1).pos.x, "{style:?} {j}");
                        if pt(j, 1).pos.y != pt(j - 1, 1).pos.y {
                            slanted += 1;
                        }
                    }
                }
                assert!(
                    slanted > 20,
                    "{style:?} layer {k}: {slanted} slanted segments"
                );
                // The strip spans the whole drawn width.
                assert_eq!(pt(0, 1).pos.x, rect.left());
                assert!((pt(points - 1, 1).pos.x - rect.right()).abs() < 1e-3);
            }
            if style == Style::Rgb {
                // Per-column colour, interpolated between neighbouring points.
                let colours: std::collections::BTreeSet<_> = layers[0]
                    .vertices
                    .iter()
                    .filter(|v| v.color.a() == 255)
                    .map(|v| v.color.to_array())
                    .collect();
                assert!(colours.len() > 20, "{} distinct Rgb colours", colours.len());
            }
        }
    }

    /// Test-only global allocator that counts allocations made by the current
    /// thread (tests run in parallel, so a global counter would be noisy).
    mod alloc_count {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;

        thread_local! {
            static COUNT: Cell<u64> = const { Cell::new(0) };
        }

        struct Counting;

        // SAFETY: delegates every operation to `System`; only adds a counter.
        unsafe impl GlobalAlloc for Counting {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let _ = COUNT.try_with(|c| c.set(c.get() + 1));
                // SAFETY: caller upholds `GlobalAlloc::alloc` contract.
                unsafe { System.alloc(layout) }
            }
            unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
                let _ = COUNT.try_with(|c| c.set(c.get() + 1));
                // SAFETY: as above.
                unsafe { System.alloc_zeroed(layout) }
            }
            unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new: usize) -> *mut u8 {
                let _ = COUNT.try_with(|c| c.set(c.get() + 1));
                // SAFETY: as above.
                unsafe { System.realloc(ptr, layout, new) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                // SAFETY: as above.
                unsafe { System.dealloc(ptr, layout) }
            }
        }

        #[global_allocator]
        static GLOBAL: Counting = Counting;

        /// Allocations (incl. reallocs) by this thread so far.
        pub(super) fn count() -> u64 {
            COUNT.with(Cell::get)
        }
    }

    #[test]
    fn layer_buffers_are_reused_across_frames() {
        // Allocation-aware: after warm-up, rebuilding the geometry for every
        // style (with varying EQ) must not allocate at all on the renderer path.
        let peaks = synthetic_peaks(60);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(700.0, HEIGHT));
        let vp = Viewport::whole(&peaks, 700.0);
        let palette = Palette::from_tokens(tokens());
        let eqs = [
            Eq::default(),
            Eq {
                low: 0.3,
                mid: 0.6,
                high: 0.9,
            },
            Eq {
                low: 0.0,
                mid: 1.0,
                high: 0.5,
            },
        ];
        let n = peaks.sample_count();
        // Whole track and a zoomed window (different mip level, same strip size).
        let vps = [
            vp,
            Viewport {
                start_sample: n / 3,
                end_sample: n / 3 + 441_000,
                ..vp
            },
        ];
        for style in Style::ALL {
            let mut layers: [Mesh; 3] = Default::default();
            for vp in vps {
                for eq in eqs {
                    build_layers(&peaks, style, vp, eq, rect, palette, &mut layers);
                }
            }
            let before = alloc_count::count();
            for _ in 0..5 {
                for vp in vps {
                    for eq in eqs {
                        build_layers(&peaks, style, vp, eq, rect, palette, &mut layers);
                    }
                }
            }
            let allocs = alloc_count::count() - before;
            assert!(!layers[0].is_empty(), "{style:?} drew nothing");
            // The anti-aliased strip path ran: 700 columns + 2 end points.
            let used = if style == Style::ThreeBand { 3 } else { 1 };
            for m in layers.iter().take(used) {
                assert_eq!(m.vertices.len(), 702 * VERTS_PER_POINT, "{style:?}");
                assert_eq!(m.indices.len(), 701 * TRIS_PER_SEGMENT * 3, "{style:?}");
            }
            assert_eq!(allocs, 0, "{style:?}: steady-state frames allocated");
        }
    }

    #[test]
    fn toolbar_opens_demo_and_style_and_eq_controls_drive_the_view() {
        let mut app = crate::LibraryApp::new(
            std::path::PathBuf::from("/definitely/missing/oxidj-wave-test.sqlite"),
            theme::ThemeMode::TokyoNight,
            tokens(),
        );
        let ctx = egui::Context::default();
        let click = |pos: Pos2, pressed| egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        // Returns the toolbar button rect, the rect of the "RGB" selector and
        // the number of waveform meshes painted.
        let frame = |app: &mut crate::LibraryApp, input| {
            let mut button = Rect::NOTHING;
            let output = ctx.run(input, |ctx| {
                theme::publish_tokens(ctx, tokens());
                egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                    button = app.show_waveform_demo_button(ui).rect;
                });
                app.show_waveform_demo_window(ctx);
            });
            let rgb = output.shapes.iter().find_map(|c| match &c.shape {
                Shape::Text(t) if t.galley.job.text == "RGB" => Some(t.visual_bounding_rect()),
                _ => None,
            });
            // Tallest painted waveform mesh (0 if none).
            let meshes = output
                .shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    Shape::Mesh(m) => Some(m.calc_bounds().height()),
                    _ => None,
                })
                .fold(0.0_f32, f32::max);
            (button, rgb, meshes)
        };
        let (button, rgb, _) = frame(&mut app, egui::RawInput::default());
        assert!(rgb.is_none() && app.waveform_demo.is_none());
        frame(&mut app, click(button.center(), true));
        frame(&mut app, click(button.center(), false));
        assert!(app.show_waveform_demo);
        // Window appears (may need a frame to size itself).
        frame(&mut app, egui::RawInput::default());
        let (_, rgb, meshes) = frame(&mut app, egui::RawInput::default());
        let rgb = rgb.expect("style selector rendered");
        assert!(meshes > 10.0, "waveform painted: {meshes}");
        assert_eq!(app.waveform_demo.as_ref().unwrap().style, Style::Mono);
        let peaks_before = app.waveform_demo.as_ref().unwrap().peaks.clone();
        frame(&mut app, click(rgb.center(), true));
        frame(&mut app, click(rgb.center(), false));
        let state = app.waveform_demo.as_ref().unwrap();
        assert_eq!(state.style, Style::Rgb);
        assert_eq!(state.peaks, peaks_before);
        // EQ state behind the sliders feeds the view live: all gains 0 → flat line.
        app.waveform_demo.as_mut().unwrap().eq = Eq {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        };
        let (_, _, height) = frame(&mut app, egui::RawInput::default());
        assert!(
            height > 0.0 && height <= MIN_THICKNESS + 2.0 * FRINGE + 1e-3,
            "{height}"
        );
    }

    /// MEASURED, not asserted: `cargo test --release --manifest-path
    /// prototypes/oxidj-library-ui/Cargo.toml waveform_frame_time -- --ignored --nocapture`.
    #[test]
    #[ignore = "timing report; run in release"]
    fn waveform_frame_time() {
        use std::time::Instant;
        let peaks = ten_minutes();
        let n = peaks.sample_count();
        let ctx = egui::Context::default();
        let frames = 300;
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1500.0, 300.0))),
            ..Default::default()
        };
        for (label, start, end) in [
            ("whole 10 min", 0, n),
            ("10 s window", n / 2, n / 2 + 441_000),
        ] {
            for style in Style::ALL {
                let vp = Viewport {
                    start_sample: start,
                    end_sample: end,
                    width_px: 1440.0,
                };
                let mut build = std::time::Duration::ZERO;
                let mut total = std::time::Duration::ZERO;
                for f in 0..frames + 10 {
                    let eq = Eq {
                        low: (f % 10) as f32 / 10.0,
                        mid: 1.0,
                        high: 0.5,
                    };
                    let t0 = Instant::now();
                    let mut b = std::time::Duration::ZERO;
                    let output = ctx.run(input(), |ctx| {
                        theme::publish_tokens(ctx, tokens());
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let tb = Instant::now();
                            waveform_view(ui, Some(&peaks), style, vp, eq);
                            b = tb.elapsed();
                        });
                    });
                    let prims = ctx.tessellate(output.shapes, output.pixels_per_point);
                    std::hint::black_box(prims);
                    if f >= 10 {
                        build += b;
                        total += t0.elapsed();
                    }
                }
                println!(
                    "waveform_frame_time {label} @1440px {:<6} view={:.3} ms/frame  frame(run+tessellate)={:.3} ms/frame",
                    style.label(),
                    build.as_secs_f64() * 1e3 / f64::from(frames),
                    total.as_secs_f64() * 1e3 / f64::from(frames),
                );
            }
        }
    }
}
