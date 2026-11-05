//! M8/D1: the deck area (no audio yet, D2).
//!
//! Top to bottom: a slim section bar (one chevron toggle per optional
//! section), ONE merged zoomed detail waveform (deck A above the seam growing
//! up, deck B below growing down, one time axis, one playhead), a thin
//! club-standard control strip (A left, B right, CUE/PLAY outermost), an
//! optional mixer placeholder (D4) and, at the bottom, two SEPARATE
//! full-track overviews (A first, B second). Every section is optional and
//! persisted in [`Settings`]; the whole area still toggles with Ctrl+D.
//!
//! Decks are told apart by temperature (A warm, B cool, or the Classic
//! cyan/purple preset), a tinted half wash, a coloured rail, a faint
//! watermark letter and position (A always above / left).
//!
//! The transport is silent: PLAY advances the playhead by wall clock.
//! Hot cues set on the pads live for the session only; imported Rekordbox
//! cues and loops are drawn as stored and never written back.

use crate::beat_grid::{self, BeatGrid, GridColours};
use crate::settings::{DeckPalette, DeckSections, Playhead, TempoRange};
use crate::theme::ColorTokens;
use crate::track_waveform::{GridSource, Loaded};
use crate::waveform_view::pick_level;
use crate::LibraryApp;
use eframe::egui::{self, pos2, vec2, Color32, FontId, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui};
use oxidj_application::{WaveformBand, WaveformPeaks};
use oxidj_library::{Catalog, LibraryCue, TrackDetails};
use std::sync::{Arc, Mutex};

/// Deck letters, A first.
pub const DECKS: [&str; 2] = ["A", "B"];
/// Height of the section bar holding the collapse toggles.
pub const SECTION_BAR_HEIGHT: f32 = 24.0;
/// One full-track overview strip.
pub const OVERVIEW_HEIGHT: f32 = 34.0;
/// Gap between stacked deck-area sections.
pub const GAP: f32 = 4.0;
/// One control-strip row (WCAG 24 px targets).
pub const ROW: f32 = 24.0;
/// The mixer placeholder row.
pub const MIXER_HEIGHT: f32 = 28.0;
/// Hot-cue pads per deck.
pub const PADS: usize = 8;
/// Detail window per "bar" when a deck has no beat grid (2/4/8/16 s).
pub const NO_GRID_BAR_MS: f64 = 2000.0;
/// Status-bar marker while the transport is silent.
pub const NO_AUDIO: &str = "NO AUDIO (D2)";
/// Quiet caption in a half/overview whose waveform is not analysed yet.
pub const NOT_READY: &str = "analysing · waveform not ready";
/// Mixer placeholder text.
pub const MIXER_NOTE: &str = "Mixer and crossfader arrive in D4 · hidden by default";
/// Rail width on each half's / overview's left edge.
pub const RAIL: f32 = 4.0;
/// Overview content starts after the rail and the deck chip.
pub const OVERVIEW_INSET: f32 = RAIL + 22.0;
/// Width of the anti-aliasing fringe along the waveform edge.
const FRINGE: f32 = 1.0;
/// Display auto-gain ceiling (a near-silent track is not blown up).
const MAX_GAIN: f32 = 4.0;
/// Auto-gain target: the loudest column reaches this fraction of the half.
const GAIN_TARGET: f32 = 0.95;

/// Rekordbox-like default hot-cue colours when the source stores none.
pub const PAD_COLOURS: [Color32; PADS] = [
    Color32::from_rgb(0xe5, 0x48, 0x4d),
    Color32::from_rgb(0xf7, 0x6b, 0x15),
    Color32::from_rgb(0xff, 0xc5, 0x3d),
    Color32::from_rgb(0x46, 0xa7, 0x58),
    Color32::from_rgb(0x12, 0xa5, 0x94),
    Color32::from_rgb(0x00, 0x90, 0xff),
    Color32::from_rgb(0x8e, 0x4e, 0xc6),
    Color32::from_rgb(0xd6, 0x40, 0x9f),
];

// ---------------------------------------------------------------- colours

/// Every colour one deck paints with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeckColours {
    /// Outer (loudest) waveform layer, rail, bar lines, chip fill.
    pub outer: Color32,
    pub mid: Color32,
    /// The "white-hot" core (dark core on light themes).
    pub core: Color32,
    pub wash: Color32,
    pub beat: Color32,
    pub watermark: Color32,
    /// Text on an `outer` chip (>= 4.5:1).
    pub chip_text: Color32,
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// WCAG relative luminance.
pub fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let s = f32::from(v) / 255.0;
        if s <= 0.040_45 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG contrast ratio of two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The deck palette for `deck` (0 = A) on the current theme.
pub fn deck_colours(palette: DeckPalette, deck: usize, tokens: ColorTokens) -> DeckColours {
    let light = luminance(tokens.background) > 0.4;
    let a = deck == 0;
    let (outer, mid, core) = match (palette, a, light) {
        // Warm A: amber → orange → white-hot core.
        (DeckPalette::WarmCool, true, false) => (rgb(0xff, 0x9e, 0x3d), rgb(0xff, 0x6a, 0x1a), rgb(0xff, 0xf3, 0xdc)),
        (DeckPalette::WarmCool, true, true) => (rgb(0xb0, 0x52, 0x00), rgb(0xc4, 0x3c, 0x00), rgb(0x4a, 0x1a, 0x00)),
        // Cool B: blue → cyan → icy core.
        (DeckPalette::WarmCool, false, false) => (rgb(0x4c, 0x8d, 0xff), rgb(0x38, 0xd6, 0xff), rgb(0xe6, 0xfa, 0xff)),
        (DeckPalette::WarmCool, false, true) => (rgb(0x1f, 0x5f, 0xd6), rgb(0x00, 0x78, 0x9e), rgb(0x00, 0x2c, 0x4a)),
        // Classic: cyan A, purple B.
        (DeckPalette::Classic, true, false) => (rgb(0x7d, 0xcf, 0xff), rgb(0x2a, 0xc3, 0xde), rgb(0xe6, 0xf7, 0xff)),
        (DeckPalette::Classic, true, true) => (rgb(0x00, 0x71, 0x97), rgb(0x00, 0x5f, 0x87), rgb(0x00, 0x30, 0x4a)),
        (DeckPalette::Classic, false, false) => (rgb(0xbb, 0x9a, 0xf7), rgb(0x9d, 0x7c, 0xd8), rgb(0xf0, 0xe8, 0xff)),
        (DeckPalette::Classic, false, true) => (rgb(0x7a, 0x3f, 0xd1), rgb(0x6a, 0x35, 0xb8), rgb(0x3b, 0x1d, 0x6e)),
    };
    let dark = rgb(0x10, 0x10, 0x18);
    let chip_text = if contrast(outer, dark) >= contrast(outer, Color32::WHITE) { dark } else { Color32::WHITE };
    DeckColours {
        outer,
        mid,
        core,
        wash: alpha(outer, if light { 26 } else { 18 }),
        beat: alpha(outer, 80),
        watermark: alpha(outer, if light { 36 } else { 22 }),
        chip_text,
    }
}

/// Colour of hot cue `n` (1-based) from the stored cue or the default pad colour.
fn pad_colour(n: usize, cue: Option<&LibraryCue>) -> Color32 {
    cue.and_then(|c| c.colour)
        .map(|[r, g, b]| rgb(r, g, b))
        .unwrap_or(PAD_COLOURS[(n.max(1) - 1) % PADS])
}

// ------------------------------------------------------------------ model

/// A track loaded on a deck (read once, on load).
pub(crate) struct DeckTrack {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) duration_ms: f64,
    /// `None` = not analysed yet (time grid + quiet caption instead).
    pub(crate) peaks: Option<WaveformPeaks>,
    /// Imported grid wins over a computed one (same rule as the details panel).
    pub(crate) grid: Option<(BeatGrid, GridSource)>,
    /// Imported Rekordbox cues and loops, as stored.
    pub(crate) cues: Vec<LibraryCue>,
    /// Loudest band extent over the whole track (auto-gain input).
    pub(crate) loudest: f32,
}

impl DeckTrack {
    pub(crate) fn read(catalog: Option<&Catalog>, item: &TrackDetails) -> Self {
        let loaded = Loaded::read(catalog, item);
        let peaks = loaded.computed.ok().flatten();
        let grid = loaded.grid.ok().flatten();
        let (uri, _) = oxidj_application::location_uri(&item.track.path);
        let cues = catalog
            .and_then(|db| db.cues_for_locations(&[&item.track.path, &uri]).ok())
            .unwrap_or_default();
        let duration_ms = match &peaks {
            Some(p) => p.sample_count() as f64 * 1000.0 / f64::from(p.sample_rate().max(1)),
            None => item.duration_seconds.unwrap_or(0.0) * 1000.0,
        };
        let loudest = peaks.as_ref().map_or(1.0, loudest_extent);
        Self {
            id: item.track.id.as_u64(),
            name: std::path::Path::new(&item.track.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            duration_ms,
            peaks,
            grid,
            cues,
            loudest,
        }
    }

    /// Display gain: auto-gain lifts quiet masters so the loudest column
    /// fills [`GAIN_TARGET`] of the half (display only, audio unchanged).
    pub(crate) fn gain(&self, auto: bool) -> f32 {
        if auto && self.loudest > f32::EPSILON {
            (GAIN_TARGET / self.loudest).clamp(1.0, MAX_GAIN)
        } else {
            1.0
        }
    }

    pub(crate) fn beat_grid(&self) -> Option<&BeatGrid> {
        self.grid.as_ref().map(|(g, _)| g)
    }

    /// Imported hot cue `n` (1..=8).
    fn hot_cue(&self, n: usize) -> Option<&LibraryCue> {
        self.cues.iter().find(|c| c.hot_cue == Some(n as u32))
    }
}

fn amplitude_ext(p: oxidj_application::WaveformPeak) -> f32 {
    let s = |v: u8| ((f32::from(v) - 128.0) / 127.0).clamp(-1.0, 1.0);
    s(p.max()).max(-s(p.min())).max(0.0)
}

fn loudest_extent(peaks: &WaveformPeaks) -> f32 {
    let level = peaks.level_count() - 1;
    WaveformBand::ALL
        .iter()
        .filter_map(|b| peaks.band(level, *b))
        .flat_map(|col| col.iter().map(|p| amplitude_ext(*p)))
        .fold(0.0, f32::max)
}

/// One deck's transport (silent until D2).
#[derive(Default)]
pub(crate) struct Deck {
    pub(crate) track: Option<DeckTrack>,
    pub(crate) position_ms: f64,
    pub(crate) playing: bool,
    pub(crate) cue_ms: f64,
    /// Tempo fader in percent (within the Settings range).
    pub(crate) tempo_pct: f32,
    pub(crate) loop_in: Option<f64>,
    /// Active manual loop `[start, end)`.
    pub(crate) loop_range: Option<(f64, f64)>,
    /// Hot-cue pads (ms), seeded from the imported hot cues.
    pub(crate) pads: [Option<f64>; PADS],
}

impl Deck {
    pub(crate) fn load(&mut self, track: DeckTrack) {
        let mut pads = [None; PADS];
        for (n, pad) in pads.iter_mut().enumerate() {
            *pad = track.hot_cue(n + 1).map(|c| f64::from(c.start_ms));
        }
        *self = Self { track: Some(track), pads, ..Self::default() };
    }

    fn duration(&self) -> f64 {
        self.track.as_ref().map_or(0.0, |t| t.duration_ms)
    }

    /// Effective speed multiplier from the tempo fader.
    pub(crate) fn speed(&self) -> f64 {
        1.0 + f64::from(self.tempo_pct) / 100.0
    }

    pub(crate) fn seek(&mut self, ms: f64) {
        self.position_ms = ms.clamp(0.0, self.duration());
    }

    pub(crate) fn toggle_play(&mut self) {
        if self.track.is_some() {
            self.playing = !self.playing;
        }
    }

    /// Club CUE: while playing, stop and return to the cue point; while
    /// paused, set the cue point here.
    pub(crate) fn cue(&mut self) {
        if self.track.is_none() {
            return;
        }
        if self.playing {
            self.playing = false;
            self.position_ms = self.cue_ms;
        } else {
            self.cue_ms = self.position_ms;
        }
    }

    /// A set pad jumps there; an empty pad stores the current position.
    pub(crate) fn pad(&mut self, index: usize) {
        if self.track.is_none() || index >= PADS {
            return;
        }
        match self.pads[index] {
            Some(ms) => self.seek(ms),
            None => self.pads[index] = Some(self.position_ms),
        }
    }

    pub(crate) fn loop_in(&mut self) {
        if self.track.is_some() {
            self.loop_in = Some(self.position_ms);
        }
    }

    /// OUT closes a loop after IN; on an active loop it exits the loop.
    pub(crate) fn loop_out(&mut self) {
        if self.loop_range.take().is_some() {
            return;
        }
        if let Some(start) = self.loop_in.filter(|s| *s < self.position_ms) {
            self.loop_range = Some((start, self.position_ms));
        }
    }

    pub(crate) fn loop_scale(&mut self, factor: f64) {
        if let Some((start, end)) = self.loop_range {
            let len = ((end - start) * factor).max(10.0);
            self.loop_range = Some((start, (start + len).min(self.duration().max(start + 10.0))));
        }
    }

    /// Advance the silent transport by `dt` seconds of wall clock.
    pub(crate) fn advance(&mut self, dt: f64) {
        if !self.playing {
            return;
        }
        self.position_ms += dt * 1000.0 * self.speed();
        if let Some((start, end)) = self.loop_range {
            if self.position_ms >= end && end > start {
                self.position_ms = start + (self.position_ms - end) % (end - start);
            }
        }
        if self.position_ms >= self.duration() {
            self.position_ms = self.duration();
            self.playing = false;
        }
    }
}

/// Rects of the last frame (tests drive real input with them).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeckLayout {
    pub(crate) area: Option<Rect>,
    pub(crate) toggles: Option<[Rect; 4]>,
    pub(crate) detail: Option<Rect>,
    /// Deck A (upper) and deck B (lower) halves of the merged detail view.
    pub(crate) halves: Option<[Rect; 2]>,
    pub(crate) playhead_x: Option<f32>,
    /// Track-time window `[start, end)` ms each half shows.
    pub(crate) windows: [Option<(f64, f64)>; 2],
    pub(crate) readout: Option<Rect>,
    pub(crate) strip: Option<Rect>,
    pub(crate) cue: [Option<Rect>; 2],
    pub(crate) play: [Option<Rect>; 2],
    pub(crate) sync: [Option<Rect>; 2],
    pub(crate) key_lock: [Option<Rect>; 2],
    pub(crate) load: [Option<Rect>; 2],
    pub(crate) tempo: [Option<Rect>; 2],
    pub(crate) pads: [Option<[Rect; PADS]>; 2],
    /// IN, OUT, ½, ×2.
    pub(crate) loops: [Option<[Rect; 4]>; 2],
    pub(crate) mixer: Option<Rect>,
    /// Deck A overview first, deck B second.
    pub(crate) overviews: Option<[Rect; 2]>,
}

/// The whole deck area's state.
#[derive(Default)]
pub(crate) struct DeckArea {
    pub(crate) decks: [Deck; 2],
    /// Deck the keyboard (Space, 1–8, Tab) acts on.
    pub(crate) active: usize,
    last_time: Option<f64>,
    /// A Tab press taken by [`LibraryApp::deck_input_hook`].
    pending_tab: bool,
    pub(crate) layout: DeckLayout,
}

impl DeckArea {
    /// Advance both silent transports to `now` (seconds, egui input time).
    pub(crate) fn tick(&mut self, now: f64) {
        let dt = self.last_time.map_or(0.0, |t| (now - t).clamp(0.0, 0.25));
        self.last_time = Some(now);
        for deck in &mut self.decks {
            deck.advance(dt);
        }
    }

    pub(crate) fn any_playing(&self) -> bool {
        self.decks.iter().any(|d| d.playing)
    }

    /// Track id loaded on each deck.
    pub(crate) fn loaded(&self) -> [Option<u64>; 2] {
        [0, 1].map(|d| self.decks[d].track.as_ref().map(|t| t.id))
    }

    /// Real-time window length (ms) of the merged view at `zoom_bars`:
    /// bars of the active deck's grid at its playhead (else the other
    /// deck's), or 2 s per bar without a grid.
    pub(crate) fn window_ms(&self, zoom_bars: usize) -> f64 {
        let order = [self.active, 1 - self.active];
        let bar = order.iter().find_map(|&d| {
            let deck = &self.decks[d];
            let grid = deck.track.as_ref()?.beat_grid()?;
            Some(beat_len_at(grid, deck.position_ms)? * 4.0 / deck.speed())
        });
        bar.unwrap_or(NO_GRID_BAR_MS) * zoom_bars.max(1) as f64
    }
}

/// Beat length (ms) of `grid` around `t`.
pub(crate) fn beat_len_at(grid: &BeatGrid, t: f64) -> Option<f64> {
    let beats = grid.beats_in((f64::NEG_INFINITY, f64::INFINITY));
    let i = beats.partition_point(|b| b.0 <= t).clamp(1, beats.len().saturating_sub(1).max(1));
    let len = beats.get(i)?.0 - beats.get(i - 1)?.0;
    (len > 0.0).then_some(len)
}

/// `(beat length, fraction 0..1 since the last beat)` at `t`.
pub(crate) fn phase_at(grid: &BeatGrid, t: f64) -> Option<(f64, f64)> {
    let beats = grid.beats_in((f64::NEG_INFINITY, f64::INFINITY));
    let len = beat_len_at(grid, t)?;
    let i = beats.partition_point(|b| b.0 <= t);
    let last = if i == 0 { beats[0].0 - len } else { beats[i - 1].0 };
    Some((len, ((t - last) / len).rem_euclid(1.0)))
}

/// Seam readout text: phase offset (ms, wrapped to ±½ beat of deck A) and
/// tempo-adjusted BPM delta; `None` unless both decks have a beat grid.
pub(crate) fn seam_readout(decks: &[Deck; 2]) -> Option<String> {
    let mut phase = [(0.0, 0.0); 2];
    let mut bpm = [0.0; 2];
    for (d, deck) in decks.iter().enumerate() {
        let grid = deck.track.as_ref()?.beat_grid()?;
        phase[d] = phase_at(grid, deck.position_ms)?;
        bpm[d] = 60_000.0 / phase[d].0 * deck.speed();
    }
    let real_beat = phase[0].0 / decks[0].speed();
    let offset = ((phase[0].1 - phase[1].1 + 0.5).rem_euclid(1.0) - 0.5) * real_beat;
    Some(format!("Δ {offset:+.0} ms · {:+.2} BPM", bpm[0] - bpm[1]))
}

pub(crate) fn format_clock(ms: f64) -> String {
    let s = (ms.max(0.0) / 1000.0).floor() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

// --------------------------------------------------------------- geometry

/// Fill `layers` (cleared first) with deck `colours` waveform strips that
/// grow from the seam: `up` = from `rect.bottom()` upwards (deck A), else
/// from `rect.top()` downwards (deck B). Points sit on pixel boundaries
/// over the track-time `window`; columns outside the track are skipped
/// (the strip breaks there). Every vertex is clamped into `rect`, so the
/// painted bounds never leave it. Layers: outer, mid, core.
#[allow(clippy::too_many_arguments)]
pub fn build_half(
    peaks: &WaveformPeaks,
    window: (f64, f64),
    gain: f32,
    rect: Rect,
    up: bool,
    colours: DeckColours,
    fringe: f32,
    layers: &mut [Mesh; 3],
) {
    for mesh in layers.iter_mut() {
        mesh.vertices.clear();
        mesh.indices.clear();
    }
    let (t0, t1) = window;
    if !(rect.is_finite() && rect.width() >= 1.0 && rect.height() >= 2.0 && t1 > t0) {
        return;
    }
    let px = rect.width().floor() as usize;
    let rate = f64::from(peaks.sample_rate().max(1));
    let total = peaks.sample_count() as f64;
    let span = (t1 - t0) * rate / 1000.0;
    let level = pick_level(peaks, span.max(1.0) as u64, px);
    let bands = WaveformBand::ALL.map(|b| peaks.band(level, b).unwrap_or(&[]));
    let len = bands[0].len();
    let col = f64::from(peaks.samples_per_peak()) * (1u64 << level) as f64;
    let base = if up { rect.bottom() } else { rect.top() };
    let dir = if up { -1.0 } else { 1.0 };
    let reach = (rect.height() - fringe - 0.5).max(1.0);
    let colors = [colours.outer, colours.mid, colours.core];
    let mut connected = false;
    for i in 0..=px {
        let x = rect.left() + rect.width() * i as f32 / px as f32;
        let centre = (t0 * rate / 1000.0) + span * i as f64 / px as f64;
        let half_px = span / px as f64 / 2.0;
        let (s0, s1) = (centre - half_px, centre + half_px);
        if s1 <= 0.0 || s0 >= total || len == 0 {
            connected = false;
            continue;
        }
        let c0 = ((s0.max(0.0) / col) as usize).min(len - 1);
        let c1 = ((s1 / col).ceil() as usize).clamp(c0 + 1, len);
        let e: [f32; 3] = core::array::from_fn(|b| {
            bands[b][c0..c1].iter().map(|p| amplitude_ext(*p)).fold(0.0, f32::max) * gain
        });
        let outer = e.iter().copied().fold(0.0, f32::max).min(1.0);
        let mid = e[1].max(e[2]).min(outer);
        let core = (0.45 * outer).max(e[2]).min(mid);
        for (layer, (mesh, ext)) in layers.iter_mut().zip([outer, mid, core]).enumerate() {
            let height = (ext * reach).max(if layer == 0 { 1.0 } else { 0.0 });
            let edge = base + dir * height;
            let outside = (edge + dir * fringe).clamp(rect.top(), rect.bottom());
            push(mesh, x, outside, edge, base, colors[layer], connected);
        }
        connected = true;
    }
}

/// Append one strip point `[fringe, edge, base]`; joins the previous point
/// when `connect`.
fn push(mesh: &mut Mesh, x: f32, fringe: f32, edge: f32, base: f32, color: Color32, connect: bool) {
    let v = |y: f32, color| egui::epaint::Vertex { pos: Pos2::new(x, y), uv: egui::epaint::WHITE_UV, color };
    let n = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[v(fringe, Color32::TRANSPARENT), v(edge, color), v(base, color)]);
    if connect && n >= 3 {
        let a = n - 3;
        for k in 0..2 {
            mesh.indices.extend_from_slice(&[a + k, a + k + 1, n + k, a + k + 1, n + k + 1, n + k]);
        }
    }
}

/// x of track time `t` in `rect` showing `window`.
fn x_at(t: f64, (t0, t1): (f64, f64), rect: Rect) -> f32 {
    rect.left() + ((t - t0) / (t1 - t0)) as f32 * rect.width()
}

#[derive(Clone, Default)]
struct Buffers(Arc<Mutex<[Arc<Mesh>; 4]>>);

/// Paint deck `colours` waveform (3 layers) and the grid `lines` mesh with
/// reused buffers keyed by `id`.
fn paint_meshes(ui: &Ui, painter: &egui::Painter, id: egui::Id, fill: impl FnOnce(&mut [Mesh; 3], &mut Mesh)) {
    let buffers = ui.data_mut(|d| d.get_temp_mut_or_default::<Buffers>(id).clone());
    let mut slots = buffers.0.lock().unwrap_or_else(|e| e.into_inner());
    let mut owned: [Mesh; 4] = slots.each_mut().map(|s| Arc::get_mut(s).map(std::mem::take).unwrap_or_default());
    let [a, b, c, lines] = &mut owned;
    let mut layers = [std::mem::take(a), std::mem::take(b), std::mem::take(c)];
    fill(&mut layers, lines);
    let [la, lb, lc] = layers;
    owned = [la, lb, lc, std::mem::take(lines)];
    for (slot, mesh) in slots.iter_mut().zip(owned) {
        match Arc::get_mut(slot) {
            Some(dst) => *dst = mesh,
            None => *slot = Arc::new(mesh),
        }
    }
    // Waveform layers first, then the grid lines over them.
    for slot in slots.iter() {
        if !slot.is_empty() {
            painter.add(Shape::Mesh(Arc::clone(slot)));
        }
    }
}

/// Time grid (mm:ss) for a half or overview without a beat grid: minor
/// lines every `step` ms, labelled majors every 4 steps (inside the track).
fn time_grid(painter: &egui::Painter, rect: Rect, window: (f64, f64), duration: f64, colours: DeckColours, label: Color32, step: f64, labels_at_top: bool) {
    let (t0, t1) = window;
    if !(t1 > t0 && step > 0.0) {
        return;
    }
    let first = (t0.max(0.0) / step).ceil() as i64;
    let last = (t1.min(duration) / step).floor() as i64;
    for k in first..=last {
        let t = k as f64 * step;
        let x = x_at(t, window, rect);
        let major = k % 4 == 0;
        let w = if major { 1.5 } else { 1.0 };
        let x0 = (x - w / 2.0).clamp(rect.left(), rect.right() - w);
        let color = if major { alpha(colours.outer, 150) } else { colours.beat };
        painter.rect_filled(Rect::from_min_max(pos2(x0, rect.top()), pos2(x0 + w, rect.bottom())), 0.0, color);
        if major {
            let galley = painter.layout_no_wrap(format_clock(t), FontId::monospace(10.0), label);
            let y = if labels_at_top { rect.top() + 2.0 } else { rect.bottom() - 2.0 - galley.size().y };
            let pos = pos2(x0 + w + 2.0, y);
            if Rect::from_min_size(pos, galley.size()).max.x <= rect.right() && y >= rect.top() {
                painter.galley(pos, galley, label);
            }
        }
    }
}

/// Imported loops (and the active manual loop) as shaded regions, hot cues
/// as numbered coloured markers with a flag at the `outer` edge, memory
/// cues as small triangles there. Everything is clamped into `rect`.
#[allow(clippy::too_many_arguments)]
fn paint_cues(painter: &egui::Painter, rect: Rect, window: (f64, f64), deck: &Deck, top_edge: bool, flag: f32, memory: Color32, label: Color32) {
    let Some(track) = &deck.track else { return };
    let span = |a: f64, b: f64| {
        let (x0, x1) = (x_at(a, window, rect), x_at(b, window, rect));
        Rect::from_x_y_ranges(x0.max(rect.left())..=x1.min(rect.right()), rect.y_range())
    };
    for cue in track.cues.iter().filter(|c| c.end_ms.is_some()) {
        let r = span(f64::from(cue.start_ms), f64::from(cue.end_ms.unwrap_or(cue.start_ms)));
        if r.width() > 0.0 {
            let colour = cue.hot_cue.map_or(memory, |n| pad_colour(n as usize, Some(cue)));
            painter.rect_filled(r, 0.0, alpha(colour, 46));
        }
    }
    if let Some((a, b)) = deck.loop_range {
        let r = span(a, b);
        if r.width() > 0.0 {
            painter.rect_filled(r, 0.0, alpha(label, 40));
        }
    }
    let edge_y = if top_edge { rect.top() } else { rect.bottom() };
    let dir = if top_edge { 1.0 } else { -1.0 };
    for cue in track.cues.iter().filter(|c| c.hot_cue.is_none()) {
        let x = x_at(f64::from(cue.start_ms), window, rect);
        if x < rect.left() || x > rect.right() {
            continue;
        }
        let s = (flag * 0.5).min(5.0);
        let x = x.clamp(rect.left() + s, rect.right() - s);
        painter.add(Shape::convex_polygon(
            vec![pos2(x - s, edge_y), pos2(x + s, edge_y), pos2(x, edge_y + dir * s * 1.4)],
            memory,
            Stroke::NONE,
        ));
    }
    for (index, pad) in deck.pads.iter().enumerate() {
        let Some(ms) = *pad else { continue };
        let x = x_at(ms, window, rect);
        if x < rect.left() || x > rect.right() {
            continue;
        }
        let n = index + 1;
        let colour = pad_colour(n, track.hot_cue(n));
        let x0 = (x - 1.0).clamp(rect.left(), rect.right() - 2.0);
        painter.rect_filled(Rect::from_min_max(pos2(x0, rect.top()), pos2(x0 + 2.0, rect.bottom())), 0.0, colour);
        let size = vec2(flag, flag.min(rect.height()));
        let fx = x0.min(rect.right() - size.x).max(rect.left());
        let fy = if top_edge { rect.top() } else { rect.bottom() - size.y };
        let f = Rect::from_min_size(pos2(fx, fy), size);
        painter.rect_filled(f, 2.0, colour);
        let dark = rgb(0x10, 0x10, 0x18);
        let text = if contrast(colour, dark) >= contrast(colour, Color32::WHITE) { dark } else { Color32::WHITE };
        let galley = painter.layout_no_wrap(n.to_string(), FontId::proportional((size.y - 2.0).clamp(6.0, 11.0)), text);
        let gp = f.center() - galley.size() / 2.0;
        if f.contains_rect(Rect::from_min_size(gp, galley.size())) {
            painter.galley(gp, galley, text);
        }
    }
}

/// Deck letter chip (fill + letter) inside `rect`.
fn chip(painter: &egui::Painter, rect: Rect, letter: &str, colours: DeckColours) {
    painter.rect_filled(rect, 3.0, colours.outer);
    let galley = painter.layout_no_wrap(letter.to_owned(), FontId::proportional(12.0), colours.chip_text);
    painter.galley(rect.center() - galley.size() / 2.0, galley, colours.chip_text);
}

// ------------------------------------------------------------------ input

impl LibraryApp {
    /// eframe raw-input hook: with the decks shown and nothing focused, Tab
    /// switches the active deck instead of starting focus traversal
    /// (Shift+Tab still enters the controls; Esc leaves them).
    pub(crate) fn deck_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if !self.show_decks || self.settings_ui.open || ctx.memory(|m| m.focused()).is_some() {
            return;
        }
        let is_tab = |e: &egui::Event| {
            matches!(e, egui::Event::Key { key: egui::Key::Tab, modifiers, .. } if modifiers.is_none())
        };
        if raw.events.iter().any(|e| is_tab(e) && matches!(e, egui::Event::Key { pressed: true, .. })) {
            self.decks.pending_tab = true;
        }
        raw.events.retain(|e| !is_tab(e));
    }

    /// Deck keys (only with the decks shown and Settings closed). Section
    /// toggles (Ctrl+Shift+1–4) always work; the DJ keys only when no
    /// widget has keyboard focus, so focused controls keep Space/Enter.
    pub(crate) fn handle_deck_keys(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.decks.pending_tab) && self.show_decks {
            self.decks.active = 1 - self.decks.active;
        }
        if !self.show_decks || self.settings_ui.open {
            return;
        }
        let chord = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
        let digits = [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
        ];
        ctx.input_mut(|i| {
            for (s, key) in digits.iter().take(4).enumerate() {
                if i.consume_key(chord, *key) {
                    let slot = self.settings.sections.get_mut(s);
                    *slot = !*slot;
                }
            }
        });
        if ctx.memory(|m| m.focused()).is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let none = egui::Modifiers::NONE;
        let mut load = None;
        ctx.input_mut(|i| {
            let active = self.decks.active;
            if i.consume_key(none, egui::Key::Space) {
                self.decks.decks[active].toggle_play();
            }
            if i.consume_key(none, egui::Key::Q) {
                self.decks.decks[0].cue();
            }
            if i.consume_key(none, egui::Key::P) {
                self.decks.decks[1].cue();
            }
            for (pad, key) in digits.iter().enumerate() {
                if i.consume_key(none, *key) {
                    self.decks.decks[active].pad(pad);
                }
            }
            if i.consume_key(none, egui::Key::A) {
                load = Some(0);
            }
            if i.consume_key(none, egui::Key::B) {
                load = Some(1);
            }
            if i.consume_key(none, egui::Key::Plus) || i.consume_key(none, egui::Key::Equals) {
                self.settings.zoom_bars = zoom_step(self.settings.zoom_bars, -1);
            }
            if i.consume_key(none, egui::Key::Minus) {
                self.settings.zoom_bars = zoom_step(self.settings.zoom_bars, 1);
            }
        });
        if let (Some(deck), Some(id)) = (load, self.selected) {
            self.load_deck(deck, id);
        }
    }

    /// Load catalog track `id` onto `deck` (0 = A).
    pub(crate) fn load_deck(&mut self, deck: usize, id: u64) {
        let Some(item) = self.tracks.iter().find(|t| t.track.id.as_u64() == id) else { return };
        let track = DeckTrack::read(self.catalog.as_ref().ok(), item);
        self.notice = format!("Loaded {} to deck {}", track.name, DECKS[deck]);
        self.decks.decks[deck].load(track);
    }

    /// Re-read loaded tracks after a catalog reload (a waveform analysed
    /// meanwhile replaces the time grid in place); position is kept.
    pub(crate) fn refresh_decks(&mut self) {
        for d in 0..2 {
            let Some(id) = self.decks.decks[d].track.as_ref().map(|t| t.id) else { continue };
            let Some(item) = self.tracks.iter().find(|t| t.track.id.as_u64() == id) else { continue };
            let track = DeckTrack::read(self.catalog.as_ref().ok(), item);
            self.decks.decks[d].track = Some(track);
        }
    }
}

/// Next zoom in [`crate::settings::ZOOM_BARS`] (`-1` = closer, `+1` = wider).
pub(crate) fn zoom_step(bars: usize, dir: i32) -> usize {
    let z = crate::settings::ZOOM_BARS;
    let i = z.iter().position(|b| *b == bars).unwrap_or(2) as i32;
    z[(i + dir).clamp(0, z.len() as i32 - 1) as usize]
}

fn tempo_range(range: TempoRange) -> f32 {
    match range {
        TempoRange::Six => 6.0,
        TempoRange::Ten => 10.0,
        TempoRange::Sixteen => 16.0,
    }
}

// ------------------------------------------------------------------- view

/// Height of the merged detail view for a window of `screen_h`.
pub(crate) fn detail_height(screen_h: f32) -> f32 {
    (screen_h * 0.28).clamp(120.0, 260.0)
}

/// Control-strip height for one deck column `width` (loops wrap to a third
/// row when the second row does not fit).
fn strip_height(width: f32) -> f32 {
    if width >= ROW2_WIDTH { 2.0 * ROW + GAP } else { 3.0 * ROW + 2.0 * GAP }
}

const CUE_W: f32 = 40.0;
const PLAY_W: f32 = 54.0;
const PAD_W: f32 = 24.0;
const PAD_GAP: f32 = 1.0;
const LOOP_W: f32 = 26.0;
const BTN_GAP: f32 = 2.0;
const GROUP_GAP: f32 = 6.0;
const PADS_W: f32 = PADS as f32 * PAD_W + (PADS as f32 - 1.0) * PAD_GAP;
const LOOPS_W: f32 = 4.0 * LOOP_W + 3.0 * BTN_GAP;
const ROW2_WIDTH: f32 = CUE_W + BTN_GAP + PLAY_W + GROUP_GAP + PADS_W + GROUP_GAP + LOOPS_W;

/// Total deck-area height for `sections` at window `screen` size.
pub(crate) fn area_height(sections: DeckSections, screen: Rect) -> f32 {
    let column = (screen.width() - 16.0 - 12.0) / 2.0;
    let mut h = SECTION_BAR_HEIGHT;
    if sections.detail {
        h += GAP + detail_height(screen.height());
    }
    if sections.strip {
        h += GAP + strip_height(column);
    }
    if sections.mixer {
        h += GAP + MIXER_HEIGHT;
    }
    if sections.overviews {
        h += GAP + 2.0 * OVERVIEW_HEIGHT + 2.0;
    }
    h
}

impl LibraryApp {
    /// The whole deck area inside its panel.
    pub(crate) fn deck_area(&mut self, ui: &mut Ui, tokens: ColorTokens) {
        let mut layout = DeckLayout::default();
        let width = ui.available_width();
        let screen = ui.ctx().screen_rect();
        let sections = self.settings.sections;
        let height = area_height(sections, screen);
        let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        layout.area = Some(area);
        let colours = [0, 1].map(|d| deck_colours(self.settings.deck_palette, d, tokens));
        let mut y = area.top();
        let take = |h: f32, y: &mut f32| {
            let r = Rect::from_min_size(pos2(area.left(), *y), vec2(width, h));
            *y += h + GAP;
            r
        };
        let bar = take(SECTION_BAR_HEIGHT, &mut y);
        self.section_bar(ui, bar, tokens, colours, &mut layout);
        if sections.detail {
            let r = take(detail_height(screen.height()), &mut y);
            self.detail_view(ui, r, tokens, colours, &mut layout);
        }
        if sections.strip {
            let column = (width - 12.0) / 2.0;
            let r = take(strip_height(column), &mut y);
            self.control_strip(ui, r, tokens, colours, &mut layout);
        }
        if sections.mixer {
            let r = take(MIXER_HEIGHT, &mut y);
            ui.painter().rect_filled(r, 3.0, tokens.surface);
            let galley = ui.painter().layout_no_wrap(MIXER_NOTE.into(), FontId::proportional(11.5), tokens.muted_text);
            ui.painter().galley(r.center() - galley.size() / 2.0, galley, tokens.muted_text);
            let response = ui.interact(r, ui.id().with("mixer-placeholder"), Sense::hover());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, MIXER_NOTE));
            layout.mixer = Some(r);
        }
        if sections.overviews {
            let r = take(2.0 * OVERVIEW_HEIGHT + 2.0, &mut y);
            let a = Rect::from_min_size(r.min, vec2(width, OVERVIEW_HEIGHT));
            let b = Rect::from_min_size(pos2(r.left(), a.bottom() + 2.0), vec2(width, OVERVIEW_HEIGHT));
            for (d, rect) in [a, b].into_iter().enumerate() {
                self.overview(ui, d, rect, tokens, colours[d], &layout);
            }
            layout.overviews = Some([a, b]);
        }
        if self.decks.any_playing() {
            ui.ctx().request_repaint();
        }
        self.decks.layout = layout;
    }

    /// Section bar: deck chips with the active deck marked, then the four
    /// chevron toggles at the right edge.
    fn section_bar(&mut self, ui: &mut Ui, bar: Rect, tokens: ColorTokens, colours: [DeckColours; 2], layout: &mut DeckLayout) {
        let painter = ui.painter_at(bar);
        let label = painter.layout_no_wrap("DECKS".into(), FontId::proportional(11.0), tokens.muted_text);
        painter.galley(pos2(bar.left() + 2.0, bar.center().y - label.size().y / 2.0), label, tokens.muted_text);
        let mut x = bar.left() + 48.0;
        for d in 0..2 {
            let r = Rect::from_min_size(pos2(x, bar.center().y - 9.0), vec2(18.0, 18.0));
            chip(&painter, r, DECKS[d], colours[d]);
            if self.decks.active == d {
                painter.rect_stroke(r.expand(1.5), 4.0, Stroke::new(1.5_f32, tokens.focus), egui::StrokeKind::Outside);
            }
            x += 24.0;
        }
        let active = painter.layout_no_wrap(
            format!("active deck {} · Tab switches", DECKS[self.decks.active]),
            FontId::proportional(11.0),
            tokens.muted_text,
        );
        painter.galley(pos2(x + 4.0, bar.center().y - active.size().y / 2.0), active, tokens.muted_text);
        let labels = ["Detail", "Overviews", "Controls", "Mixer"];
        let mut rects = [Rect::NOTHING; 4];
        let mut right = bar.right();
        for i in (0..4).rev() {
            let text = painter.layout_no_wrap(labels[i].into(), FontId::proportional(11.5), tokens.text);
            let w = text.size().x + 24.0;
            let r = Rect::from_min_max(pos2(right - w, bar.top()), pos2(right, bar.bottom()));
            right -= w + 2.0;
            rects[i] = r;
            let expanded = self.settings.sections.get(i);
            let name = format!("{} section", DeckSections::NAMES[i]);
            let response = ui.interact(r, ui.id().with(("deck-section", i)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, expanded, &name));
            if response.hovered() || response.has_focus() {
                painter.rect_filled(r, 3.0, tokens.surface);
            }
            if response.has_focus() {
                painter.rect_stroke(r, 3.0, Stroke::new(2.0_f32, tokens.focus), egui::StrokeKind::Inside);
            }
            let ink = if expanded { tokens.text } else { tokens.muted_text };
            let c = pos2(r.left() + 10.0, r.center().y);
            let pts = if expanded {
                vec![pos2(c.x - 4.0, c.y - 2.0), pos2(c.x + 4.0, c.y - 2.0), pos2(c.x, c.y + 3.0)]
            } else {
                vec![pos2(c.x - 2.0, c.y - 4.0), pos2(c.x - 2.0, c.y + 4.0), pos2(c.x + 3.0, c.y)]
            };
            painter.add(Shape::convex_polygon(pts, ink, Stroke::NONE));
            painter.galley(pos2(r.left() + 18.0, r.center().y - text.size().y / 2.0), text, ink);
            let verb = if expanded { "Hide" } else { "Show" };
            if response
                .on_hover_text(format!("{verb} the {} (Ctrl+Shift+{}; also in Settings)", DeckSections::NAMES[i].to_lowercase(), i + 1))
                .clicked()
            {
                let slot = self.settings.sections.get_mut(i);
                *slot = !*slot;
            }
        }
        layout.toggles = Some(rects);
    }

    /// The merged detail waveform.
    fn detail_view(&mut self, ui: &mut Ui, rect: Rect, tokens: ColorTokens, colours: [DeckColours; 2], layout: &mut DeckLayout) {
        let response = ui.interact(rect, ui.id().with("merged-detail"), Sense::click_and_drag());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Merged deck waveform: deck A above, deck B below"));
        let seam = (rect.center().y).round();
        let halves = [
            Rect::from_min_max(rect.min, pos2(rect.right(), seam)),
            Rect::from_min_max(pos2(rect.left(), seam), rect.max),
        ];
        let window_real = self.decks.window_ms(self.settings.zoom_bars);
        let frac = match self.settings.playhead {
            Playhead::Centre => 0.5,
            Playhead::LeftThird => 1.0 / 3.0,
        };
        let ph = (rect.left() + rect.width() * frac).round();
        let mpp = window_real / f64::from(rect.width().max(1.0));
        // Zoom: wheel over the view (one step per wheel frame).
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.settings.zoom_bars = zoom_step(self.settings.zoom_bars, if scroll > 0.0 { -1 } else { 1 });
            }
        }
        // Drag scrubs the deck under the pointer (content moves with it).
        if response.dragged() {
            if let Some(p) = response.interact_pointer_pos() {
                let d = usize::from(p.y >= seam);
                let deck = &mut self.decks.decks[d];
                let delta = f64::from(response.drag_delta().x) * mpp * deck.speed();
                deck.seek(deck.position_ms - delta);
            }
        }
        let smooth = crate::waveform_view::smooth(ui.ctx());
        let auto_gain = self.settings.auto_gain;
        let beat_lines = self.settings.beat_lines;
        for (d, half) in halves.into_iter().enumerate() {
            let deck = &self.decks.decks[d];
            let c = colours[d];
            let painter = ui.painter_at(half);
            painter.rect_filled(half, 0.0, tokens.background);
            painter.rect_filled(half, 0.0, c.wash);
            // Large faint watermark letter at the right end.
            let size = (half.height() * 0.85).min(110.0);
            let wm = painter.layout_no_wrap(DECKS[d].into(), FontId::proportional(size), c.watermark);
            let wpos = pos2(half.right() - wm.size().x - 16.0, half.center().y - wm.size().y / 2.0);
            if half.contains_rect(Rect::from_min_size(wpos, wm.size())) {
                painter.galley(wpos, wm, c.watermark);
            }
            let up = d == 0;
            let speed = deck.speed();
            let t0 = deck.position_ms - f64::from(ph - half.left()) * mpp * speed;
            let t1 = deck.position_ms + f64::from(half.right() - ph) * mpp * speed;
            let window = (t0, t1);
            if let Some(track) = &deck.track {
                layout.windows[d] = Some(window);
                paint_meshes(ui, &painter, ui.id().with(("deck-half", d)), |layers, lines| {
                    match &track.peaks {
                        Some(peaks) => build_half(peaks, window, track.gain(auto_gain), half, up, c, if smooth { FRINGE } else { 0.0 }, layers),
                        None => layers.iter_mut().for_each(|m| {
                            m.vertices.clear();
                            m.indices.clear();
                        }),
                    }
                    lines.vertices.clear();
                    lines.indices.clear();
                    if let (true, Some(grid)) = (beat_lines, track.beat_grid()) {
                        let gc = GridColours { beat: c.beat, bar: c.outer, label: tokens.text };
                        beat_grid::build_lines(grid, window, half, gc, lines);
                    }
                });
                if track.beat_grid().is_none() {
                    time_grid(&painter, half, window, track.duration_ms, c, tokens.muted_text, 1000.0 * speed, up);
                }
                paint_cues(&painter, half, window, deck, up, 14.0, tokens.error, tokens.text);
                if track.peaks.is_none() {
                    let g = painter.layout_no_wrap(NOT_READY.into(), FontId::proportional(11.0), tokens.muted_text);
                    let y = if up { half.top() + 16.0 } else { half.bottom() - 16.0 - g.size().y };
                    painter.galley(pos2(half.left() + 30.0, y), g, tokens.muted_text);
                }
            } else {
                let g = painter.layout_no_wrap(
                    format!("Deck {} empty · select a track and press {} (or Load)", DECKS[d], DECKS[d]),
                    FontId::proportional(12.0),
                    tokens.muted_text,
                );
                painter.galley(half.center() - g.size() / 2.0, g, tokens.muted_text);
            }
            // Coloured rail and deck chip at the outer-left corner.
            painter.rect_filled(Rect::from_min_size(half.min, vec2(RAIL, half.height())), 0.0, c.outer);
            let cy = if up { half.top() + 4.0 } else { half.bottom() - 22.0 };
            chip(&painter, Rect::from_min_size(pos2(half.left() + RAIL + 4.0, cy), vec2(18.0, 18.0)), DECKS[d], c);
        }
        let painter = ui.painter_at(rect);
        // The seam: a 1 px rect (a line's caps would poke past the clip).
        painter.rect_filled(Rect::from_x_y_ranges(rect.x_range(), seam - 0.5..=seam + 0.5), 0.0, tokens.border);
        // One playhead across both halves, with small caps inside the rect.
        painter.rect_filled(Rect::from_min_max(pos2(ph - 1.0, rect.top()), pos2(ph + 1.0, rect.bottom())), 0.0, tokens.error);
        painter.add(Shape::convex_polygon(
            vec![pos2(ph - 5.0, rect.top()), pos2(ph + 5.0, rect.top()), pos2(ph, rect.top() + 6.0)],
            tokens.error,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![pos2(ph - 5.0, rect.bottom()), pos2(ph, rect.bottom() - 6.0), pos2(ph + 5.0, rect.bottom())],
            tokens.error,
            Stroke::NONE,
        ));
        if self.settings.show_phase {
            if let Some(text) = seam_readout(&self.decks.decks) {
                let g = painter.layout_no_wrap(text, FontId::monospace(11.0), tokens.text);
                let box_ = Rect::from_min_size(pos2(ph + 8.0, seam - g.size().y / 2.0 - 3.0), g.size() + vec2(12.0, 6.0));
                let box_ = box_.translate(vec2((rect.right() - box_.right()).min(0.0), 0.0));
                painter.rect_filled(box_, 3.0, tokens.surface);
                painter.rect_stroke(box_, 3.0, Stroke::new(1.0_f32, tokens.border), egui::StrokeKind::Inside);
                painter.galley(box_.min + vec2(6.0, 3.0), g, tokens.text);
                layout.readout = Some(box_);
            }
        }
        layout.detail = Some(rect);
        layout.halves = Some(halves);
        layout.playhead_x = Some(ph);
    }

    /// Club-standard strip: deck A in the left column, deck B mirrored in
    /// the right column; CUE and PLAY sit at the outer edges.
    fn control_strip(&mut self, ui: &mut Ui, rect: Rect, tokens: ColorTokens, colours: [DeckColours; 2], layout: &mut DeckLayout) {
        layout.strip = Some(rect);
        let column = (rect.width() - 12.0) / 2.0;
        let cols = [
            Rect::from_min_size(rect.min, vec2(column, rect.height())),
            Rect::from_min_size(pos2(rect.right() - column, rect.top()), vec2(column, rect.height())),
        ];
        let range = tempo_range(self.settings.tempo_range);
        let mut load = None;
        for (d, col) in cols.into_iter().enumerate() {
            let c = colours[d];
            let letter = DECKS[d];
            // Mirror x for deck B so its CUE/PLAY end up at the right edge.
            let place = |x: f32, row: usize, w: f32| {
                let y = col.top() + row as f32 * (ROW + GAP);
                let left = if d == 0 { col.left() + x } else { col.right() - x - w };
                Rect::from_min_size(pos2(left, y), vec2(w, ROW))
            };
            ui.painter().rect_filled(Rect::from_min_size(col.min, vec2(RAIL, col.height())).translate(vec2(if d == 0 { 0.0 } else { col.width() - RAIL }, 0.0)), 0.0, c.outer);
            let inset = RAIL + 4.0;
            let deck = &mut self.decks.decks[d];
            let loaded = deck.track.is_some();
            // Row 1: chip, Load, title + clock + BPM, tempo, SYNC, KEY.
            let chip_r = place(inset, 0, 22.0);
            chip(ui.painter(), chip_r.shrink2(vec2(0.0, 2.0)), letter, c);
            let load_r = place(inset + 26.0, 0, 44.0);
            let r = ui.put(load_r, egui::Button::new(egui::RichText::new("Load").small()));
            let r = r.on_hover_text(format!("Load the selected library track to deck {letter} ({letter} key)"));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Load selected track to deck {letter}")));
            if r.clicked() {
                load = Some(d);
            }
            layout.load[d] = Some(load_r);
            let key_r = place(column - 36.0, 0, 36.0);
            let sync_r = place(column - 36.0 - BTN_GAP - 44.0, 0, 44.0);
            let tempo_w = 120.0;
            let tempo_r = place(column - 36.0 - 44.0 - 2.0 * BTN_GAP - GROUP_GAP - tempo_w, 0, tempo_w);
            for (r, text, name) in [(sync_r, "SYNC", "sync"), (key_r, "KEY", "key lock")] {
                let resp = ui.add_enabled_ui(false, |ui| ui.put(r, egui::Button::new(egui::RichText::new(text).small().strong()))).inner;
                resp.on_disabled_hover_text(format!("Deck {letter} {name}: arrives in D3"))
                    .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, format!("Deck {letter} {name} (D3)")));
            }
            layout.sync[d] = Some(sync_r);
            layout.key_lock[d] = Some(key_r);
            let slider_r = Rect::from_min_size(tempo_r.min, vec2(tempo_w - 46.0, ROW));
            let value_r = Rect::from_min_max(pos2(slider_r.right() + 2.0, tempo_r.top()), tempo_r.max);
            let mut tempo = deck.tempo_pct.clamp(-range, range);
            let slider = ui.put(slider_r, egui::Slider::new(&mut tempo, -range..=range).show_value(false).step_by(0.1));
            slider.widget_info(|| egui::WidgetInfo::slider(true, f64::from(tempo), format!("Deck {letter} tempo")));
            let slider = slider.on_hover_text(format!("Deck {letter} tempo (±{range:.0} %, Settings → Decks); double-click resets"));
            if slider.double_clicked() {
                tempo = 0.0;
            }
            deck.tempo_pct = tempo;
            let value = ui.painter().layout_no_wrap(format!("{tempo:+.1}%"), FontId::monospace(11.0), tokens.text);
            ui.painter().galley(pos2(value_r.right() - value.size().x, value_r.center().y - value.size().y / 2.0), value, tokens.text);
            layout.tempo[d] = Some(slider_r);
            // Title / clock / BPM between Load and the tempo group.
            let (a, b) = if d == 0 { (load_r.right() + 6.0, tempo_r.left() - 6.0) } else { (tempo_r.right() + 6.0, load_r.left() - 6.0) };
            if b > a + 20.0 {
                let info = Rect::from_min_max(pos2(a, load_r.top()), pos2(b, load_r.bottom()));
                let text = match &deck.track {
                    Some(t) => {
                        let bpm = t
                            .beat_grid()
                            .and_then(|g| beat_len_at(g, deck.position_ms))
                            .map(|len| format!(" · {:.2} BPM", 60_000.0 / len * deck.speed()))
                            .unwrap_or_default();
                        format!("{} · {} / {}{bpm}", t.name, format_clock(deck.position_ms), format_clock(t.duration_ms))
                    }
                    None => format!("Deck {letter} · empty"),
                };
                let g = egui::WidgetText::from(egui::RichText::new(text).small().color(if loaded { tokens.text } else { tokens.muted_text }))
                    .into_galley(ui, Some(egui::TextWrapMode::Truncate), info.width(), egui::TextStyle::Small);
                let x = if d == 0 { info.left() } else { info.right() - g.size().x };
                ui.painter_at(info).galley(pos2(x, info.center().y - g.size().y / 2.0), g, tokens.text);
            }
            // Row 2: CUE, PLAY (outermost), pads 1–8, loops.
            let cue_r = place(inset, 1, CUE_W);
            let play_r = place(inset + CUE_W + BTN_GAP, 1, PLAY_W);
            let at_cue = loaded && !deck.playing && (deck.position_ms - deck.cue_ms).abs() < 1.0;
            if deck_button(ui, cue_r, "CUE", &format!("Deck {letter} cue"), at_cue, loaded, tokens.warning, tokens).clicked() {
                deck.cue();
            }
            let play_label = if deck.playing { "PAUSE" } else { "PLAY" };
            if deck_button(ui, play_r, play_label, &format!("Deck {letter} play/pause"), deck.playing, loaded, tokens.success, tokens).clicked() {
                deck.toggle_play();
            }
            layout.cue[d] = Some(cue_r);
            layout.play[d] = Some(play_r);
            let pads_x = inset + CUE_W + BTN_GAP + PLAY_W + GROUP_GAP;
            // Pads always read 1..8 left to right, also on deck B.
            let pads_block = place(pads_x, 1, PADS_W);
            let mut pad_rects = [Rect::NOTHING; PADS];
            for (p, pad_rect) in pad_rects.iter_mut().enumerate() {
                let r = Rect::from_min_size(pos2(pads_block.left() + p as f32 * (PAD_W + PAD_GAP), pads_block.top()), vec2(PAD_W, ROW));
                *pad_rect = r;
                let n = p + 1;
                let set = deck.pads[p].is_some();
                let colour = pad_colour(n, deck.track.as_ref().and_then(|t| t.hot_cue(n)));
                let name = format!("Deck {letter} hot cue {n} ({})", if set { "set" } else { "empty" });
                if pad_button(ui, r, n, set, colour, loaded, &name, tokens).clicked() {
                    deck.pad(p);
                }
            }
            layout.pads[d] = Some(pad_rects);
            let (loop_row, loop_x) = if column >= ROW2_WIDTH { (1, pads_x + PADS_W + GROUP_GAP) } else { (2, inset + CUE_W + BTN_GAP + PLAY_W + GROUP_GAP) };
            let loops_block = place(loop_x, loop_row, LOOPS_W);
            let mut loop_rects = [Rect::NOTHING; 4];
            let has_loop = deck.loop_range.is_some();
            for (k, (text, name)) in [("IN", "loop in"), ("OUT", "loop out / exit"), ("½", "halve loop"), ("×2", "double loop")].into_iter().enumerate() {
                let r = Rect::from_min_size(pos2(loops_block.left() + k as f32 * (LOOP_W + BTN_GAP), loops_block.top()), vec2(LOOP_W, ROW));
                loop_rects[k] = r;
                let enabled = loaded && (k < 2 || has_loop);
                let active = (k == 0 && deck.loop_in.is_some()) || (k == 1 && has_loop);
                if deck_button(ui, r, text, &format!("Deck {letter} {name}"), active, enabled, c.outer, tokens).clicked() {
                    match k {
                        0 => deck.loop_in(),
                        1 => deck.loop_out(),
                        2 => deck.loop_scale(0.5),
                        _ => deck.loop_scale(2.0),
                    }
                }
            }
            layout.loops[d] = Some(loop_rects);
        }
        if let (Some(deck), Some(id)) = (load, self.selected) {
            self.load_deck(deck, id);
        }
    }

    /// Deck `d`'s full-track overview: seek by click/drag, played part
    /// dimmed, detail-window box, cues and loops.
    fn overview(&mut self, ui: &mut Ui, d: usize, rect: Rect, tokens: ColorTokens, c: DeckColours, layout: &DeckLayout) {
        let letter = DECKS[d];
        let response = ui.interact(rect, ui.id().with(("deck-overview", d)), Sense::click_and_drag());
        let deck = &mut self.decks.decks[d];
        let duration = deck.duration();
        let inner = Rect::from_min_max(pos2(rect.left() + OVERVIEW_INSET, rect.top()), rect.max);
        response.widget_info(|| {
            egui::WidgetInfo::slider(deck.track.is_some(), deck.position_ms / 1000.0, format!("Deck {letter} overview: click or drag to seek"))
        });
        if duration > 0.0 && (response.is_pointer_button_down_on() || response.clicked() || response.dragged()) {
            if let Some(p) = response.interact_pointer_pos() {
                let f = ((p.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
                deck.seek(f64::from(f) * duration);
            }
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, tokens.background);
        painter.rect_filled(rect, 0.0, c.wash);
        let window = (0.0, duration.max(1.0));
        if let Some(track) = &deck.track {
            let mid = inner.center().y.round();
            let up = Rect::from_min_max(inner.min, pos2(inner.right(), mid));
            let down = Rect::from_min_max(pos2(inner.left(), mid), inner.max);
            let gain = track.gain(self.settings.auto_gain);
            match &track.peaks {
                Some(peaks) => {
                    for (k, (half, is_up)) in [(up, true), (down, false)].into_iter().enumerate() {
                        paint_meshes(ui, &painter, ui.id().with(("deck-overview-mesh", d, k)), |layers, lines| {
                            build_half(peaks, window, gain, half, is_up, c, FRINGE, layers);
                            lines.vertices.clear();
                            lines.indices.clear();
                        });
                    }
                }
                None => {
                    time_grid(&painter, inner, window, duration, c, tokens.muted_text, 15_000.0, true);
                    let g = painter.layout_no_wrap(NOT_READY.into(), FontId::proportional(11.0), tokens.muted_text);
                    painter.galley(pos2(inner.center().x - g.size().x / 2.0, inner.bottom() - g.size().y - 1.0), g, tokens.muted_text);
                }
            }
            paint_cues(&painter, inner, window, deck, true, 10.0, tokens.error, tokens.text);
            let x = x_at(deck.position_ms, window, inner).clamp(inner.left(), inner.right());
            // Played part dimmed.
            painter.rect_filled(Rect::from_min_max(inner.min, pos2(x, inner.bottom())), 0.0, alpha(tokens.background, 150));
            if let Some((a, b)) = layout.windows[d] {
                let (x0, x1) = (x_at(a, window, inner).max(inner.left()), x_at(b, window, inner).min(inner.right()));
                if x1 > x0 {
                    let r = Rect::from_x_y_ranges(x0..=x1.max(x0 + 2.0).min(inner.right()), inner.y_range());
                    painter.rect_stroke(r, 1.0, Stroke::new(1.0_f32, tokens.focus), egui::StrokeKind::Inside);
                }
            }
            let px = x.clamp(inner.left(), inner.right() - 2.0);
            painter.rect_filled(Rect::from_min_max(pos2(px, inner.top()), pos2(px + 2.0, inner.bottom())), 0.0, tokens.error);
        } else {
            let g = painter.layout_no_wrap(format!("Deck {letter} · empty"), FontId::proportional(11.0), tokens.muted_text);
            painter.galley(inner.left_center() + vec2(8.0, -g.size().y / 2.0), g, tokens.muted_text);
        }
        painter.rect_filled(Rect::from_min_size(rect.min, vec2(RAIL, rect.height())), 0.0, c.outer);
        chip(&painter, Rect::from_center_size(pos2(rect.left() + RAIL + 11.0, rect.center().y), vec2(18.0, 18.0)), letter, c);
        if response.has_focus() {
            painter.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, tokens.focus), egui::StrokeKind::Inside);
        }
    }
}

/// A strip button placed at `rect`: `active` fills it with `accent`.
#[allow(clippy::too_many_arguments)]
fn deck_button(ui: &mut Ui, rect: Rect, text: &str, name: &str, active: bool, enabled: bool, accent: Color32, tokens: ColorTokens) -> egui::Response {
    let dark = rgb(0x10, 0x10, 0x18);
    let ink = if active {
        if contrast(accent, dark) >= contrast(accent, Color32::WHITE) { dark } else { Color32::WHITE }
    } else {
        tokens.text
    };
    let mut button = egui::Button::new(egui::RichText::new(text).small().strong().color(ink)).wrap_mode(egui::TextWrapMode::Truncate);
    if active {
        button = button.fill(accent);
    }
    let response = ui.add_enabled_ui(enabled, |ui| ui.put(rect, button)).inner;
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, active, name));
    response
}

/// A hot-cue pad: filled with its colour when set, outlined when empty.
#[allow(clippy::too_many_arguments)]
fn pad_button(ui: &mut Ui, rect: Rect, n: usize, set: bool, colour: Color32, enabled: bool, name: &str, tokens: ColorTokens) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(("pad", name)), if enabled { Sense::click() } else { Sense::hover() });
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, set, name));
    let painter = ui.painter_at(rect);
    let dark = rgb(0x10, 0x10, 0x18);
    let (fill, ink) = if set {
        (colour, if contrast(colour, dark) >= contrast(colour, Color32::WHITE) { dark } else { Color32::WHITE })
    } else {
        (tokens.surface, if enabled { tokens.text } else { tokens.muted_text })
    };
    painter.rect_filled(rect.shrink(0.5), 3.0, fill);
    painter.rect_stroke(rect.shrink(0.5), 3.0, Stroke::new(1.0_f32, alpha(colour, if set { 255 } else { 170 })), egui::StrokeKind::Inside);
    if response.hovered() || response.has_focus() {
        painter.rect_stroke(rect, 3.0, Stroke::new(if response.has_focus() { 2.0_f32 } else { 1.0 }, tokens.focus), egui::StrokeKind::Inside);
    }
    let g = painter.layout_no_wrap(n.to_string(), FontId::proportional(12.0), ink);
    painter.galley(rect.center() - g.size() / 2.0, g, ink);
    response.on_hover_text(name)
}

/// Deck letters loaded with track `id` (table badges).
pub(crate) fn badges(loaded: [Option<u64>; 2], id: u64) -> [bool; 2] {
    loaded.map(|l| l == Some(id))
}

