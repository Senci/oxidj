//! M8/D1 + D2: the deck area, playing through the realtime engine.
//!
//! Top to bottom: a slim section bar (one chevron toggle per optional
//! section), ONE merged zoomed detail waveform (deck A above the seam growing
//! up, deck B below growing down, one time axis, one playhead), a thin
//! club-standard control strip (A left, B right, CUE/PLAY outermost), an
//! optional D4 mixer (trim, 3-band EQ, filter, faders, crossfader, meters;
//! see `mixer_view`) and, at the bottom, two SEPARATE
//! full-track overviews (A first, B second). Every section is optional and
//! persisted in [`Settings`]; the whole area still toggles with Ctrl+D.
//!
//! Decks are told apart by temperature (A warm, B cool, or the Classic
//! cyan/purple preset), a tinted half wash, a coloured rail, a faint
//! watermark letter and position (A always above / left).
//!
//! D2: every control drives the `oxidj-audio` engine (CPAL output, both
//! decks mixed at unity; a silent paced output when no device opens). A
//! loaded track decodes to RAM off the UI thread with progress, then plays;
//! the playhead is the audio callback's sample position, never the frame
//! clock. CUE follows CDJ rules (set at stop, preview while held, return).
//! Hot cues set on the pads live for the session only; imported Rekordbox
//! cues and loops are drawn as stored and never written back.

use crate::LibraryApp;
use crate::beat_grid::{self, BeatGrid, GridColours};
use crate::deck_audio::Loading;
use crate::icons::Icon;
use crate::settings::{DeckPalette, DeckSections, Playhead, TempoRange};
use crate::theme::ColorTokens;
use crate::track_waveform::{GridSource, Loaded};
use crate::waveform_view::pick_level;
use eframe::egui::{self, Color32, FontId, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};
use oxidj_application::{WaveformBand, WaveformPeaks};
use oxidj_audio::{
    Assign, ChannelParams, Command, Curve, Engine, METERS, Meter, MixerParams, Renderer,
};
use oxidj_library::{Catalog, LibraryCue, TrackDetails};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Deck letters, A first.
pub const DECKS: [&str; 2] = ["A", "B"];
/// Height of the section bar holding the collapse toggles.
pub const SECTION_BAR_HEIGHT: f32 = 24.0;
/// One full-track overview strip.
pub const OVERVIEW_HEIGHT: f32 = 34.0;
/// Gap between stacked deck-area sections.
pub const GAP: f32 = 4.0;
/// Hot-cue pads per deck.
pub const PADS: usize = 8;
/// Detail window per "bar" when a deck has no beat grid (2/4/8/16 s).
pub const NO_GRID_BAR_MS: f64 = 2000.0;
/// Quiet caption in a half/overview whose waveform is not analysed yet.
pub const NOT_READY: &str = "analysing · waveform not ready";
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
    /// Deck accent: rail, bar lines, chip fill, active buttons.
    pub outer: Color32,
    /// W12 band colours (deep low, mid, pale high core on dark themes;
    /// on light themes the core is the darkest). Each >= 3:1 on the plate
    /// and adjacent bands >= 1.5:1 apart, so the three layers separate.
    pub low: Color32,
    pub mid: Color32,
    pub high: Color32,
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
        if s <= 0.040_45 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
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
    // Accent (rail/chip/buttons), unchanged from D1.
    let outer = match (palette, a, light) {
        (DeckPalette::WarmCool, true, false) => rgb(0xff, 0x9e, 0x3d),
        (DeckPalette::WarmCool, true, true) => rgb(0xb0, 0x52, 0x00),
        (DeckPalette::WarmCool, false, false) => rgb(0x4c, 0x8d, 0xff),
        (DeckPalette::WarmCool, false, true) => rgb(0x1f, 0x5f, 0xd6),
        (DeckPalette::Classic, true, false) => rgb(0x7d, 0xcf, 0xff),
        (DeckPalette::Classic, true, true) => rgb(0x00, 0x71, 0x97),
        (DeckPalette::Classic, false, false) => rgb(0xbb, 0x9a, 0xf7),
        (DeckPalette::Classic, false, true) => rgb(0x7a, 0x3f, 0xd1),
    };
    // W12 bands: [low, mid, high].
    let [low, mid, high] = match (palette, a, light) {
        // Warm A: deep red-orange low, amber mid, pale-yellow core.
        (DeckPalette::WarmCool, true, false) => [
            rgb(0xe8, 0x59, 0x0c),
            rgb(0xff, 0xb3, 0x40),
            rgb(0xff, 0xf4, 0xc2),
        ],
        (DeckPalette::WarmCool, true, true) => [
            rgb(0xdc, 0x4c, 0x0c),
            rgb(0x7a, 0x4a, 0x00),
            rgb(0x2e, 0x16, 0x00),
        ],
        // Cool B: deep blue low, cyan mid, ice-white core.
        (DeckPalette::WarmCool, false, false) => [
            rgb(0x3a, 0x6d, 0xf0),
            rgb(0x22, 0xd3, 0xee),
            rgb(0xe6, 0xfb, 0xff),
        ],
        (DeckPalette::WarmCool, false, true) => [
            rgb(0x3b, 0x7d, 0xdd),
            rgb(0x00, 0x68, 0x7e),
            rgb(0x00, 0x1f, 0x33),
        ],
        // Classic: cyan A, purple B.
        (DeckPalette::Classic, true, false) => [
            rgb(0x2a, 0x8f, 0xd6),
            rgb(0x7d, 0xcf, 0xff),
            rgb(0xe6, 0xf7, 0xff),
        ],
        (DeckPalette::Classic, true, true) => [
            rgb(0x2f, 0x7f, 0xbf),
            rgb(0x00, 0x5f, 0x87),
            rgb(0x00, 0x24, 0x3a),
        ],
        (DeckPalette::Classic, false, false) => [
            rgb(0x7a, 0x4f, 0xd8),
            rgb(0xbb, 0x9a, 0xf7),
            rgb(0xf3, 0xec, 0xff),
        ],
        (DeckPalette::Classic, false, true) => [
            rgb(0x8a, 0x5a, 0xd8),
            rgb(0x5a, 0x2f, 0xa8),
            rgb(0x24, 0x0f, 0x4a),
        ],
    };
    let dark = rgb(0x10, 0x10, 0x18);
    let chip_text = if contrast(outer, dark) >= contrast(outer, Color32::WHITE) {
        dark
    } else {
        Color32::WHITE
    };
    DeckColours {
        outer,
        low,
        mid,
        high,
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
    /// Audio file (decoded on load).
    pub(crate) path: PathBuf,
    pub(crate) duration_ms: f64,
    /// `None` = not analysed yet (time grid + quiet caption instead).
    pub(crate) peaks: Option<WaveformPeaks>,
    /// Imported grid wins over a computed one (same rule as the details panel).
    pub(crate) grid: Option<(BeatGrid, GridSource)>,
    /// Imported Rekordbox cues and loops, as stored.
    pub(crate) cues: Vec<LibraryCue>,
    /// Loudest band extent over the whole track (auto-gain input).
    pub(crate) loudest: f32,
    /// Constant-tempo fit of `grid` (SYNC, BPM display, seam readout).
    pub(crate) fit: Option<GridFit>,
    /// Analyser confidence of a computed grid (`None` for imported grids).
    pub(crate) confidence: Option<f32>,
}

impl DeckTrack {
    pub(crate) fn read(catalog: Option<&Catalog>, item: &TrackDetails) -> Self {
        let loaded = Loaded::read(catalog, item);
        let peaks = loaded.computed.ok().flatten();
        let grid = loaded.grid.ok().flatten();
        let confidence = match (&grid, &loaded.tempo) {
            (Some((_, GridSource::Computed)), Ok(Some(tempo))) => Some(tempo.confidence),
            _ => None,
        };
        let fit = grid.as_ref().and_then(|(g, _)| GridFit::of(g));
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
            path: PathBuf::from(&item.track.path),
            duration_ms,
            peaks,
            grid,
            cues,
            loudest,
            fit,
            confidence,
        }
    }

    /// Track BPM at `ms`: the constant fit when the grid is constant, else
    /// the local beat length.
    pub(crate) fn bpm_at(&self, ms: f64) -> Option<f64> {
        match self.fit {
            Some(fit) if fit.constant() => Some(fit.bpm()),
            _ => beat_len_at(self.beat_grid()?, ms).map(|len| 60_000.0 / len),
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

/// A deck's audio: decoding, playable from RAM, or refused.
#[derive(Default)]
pub(crate) enum DeckAudio {
    #[default]
    None,
    Loading(Loading),
    /// Handed to the engine; positions convert at the track's own rate.
    Ready {
        rate: u32,
    },
    Failed(String),
}

/// Constant-tempo model of a beat grid: `time = anchor + i × beat` fitted
/// to every beat by least squares (imported beat times are whole ms, so a
/// single beat pair is not precise enough for sub-millisecond SYNC).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridFit {
    pub(crate) beat_ms: f64,
    pub(crate) anchor_ms: f64,
    /// Largest distance of a beat from the fitted line.
    pub(crate) residual_ms: f64,
}

/// A grid whose beats stray further than this from one tempo is variable
/// (tempo changes); SYNC refuses it.
pub(crate) const MAX_GRID_RESIDUAL_MS: f64 = 3.0;

impl GridFit {
    pub(crate) fn of(grid: &BeatGrid) -> Option<Self> {
        let beats = grid.beats_in((f64::NEG_INFINITY, f64::INFINITY));
        if beats.len() < 2 {
            return None;
        }
        let n = beats.len() as f64;
        let (mut si, mut st, mut sii, mut sit) = (0.0, 0.0, 0.0, 0.0);
        for (i, (t, _)) in beats.iter().enumerate() {
            let i = i as f64;
            (si, st, sii, sit) = (si + i, st + t, sii + i * i, sit + i * t);
        }
        let beat_ms = (n * sit - si * st) / (n * sii - si * si);
        let anchor_ms = (st - beat_ms * si) / n;
        let residual_ms = beats
            .iter()
            .enumerate()
            .map(|(i, (t, _))| (t - (anchor_ms + i as f64 * beat_ms)).abs())
            .fold(0.0, f64::max);
        (beat_ms.is_finite() && beat_ms > 0.0 && anchor_ms.is_finite()).then_some(Self {
            beat_ms,
            anchor_ms,
            residual_ms,
        })
    }

    pub(crate) fn constant(&self) -> bool {
        self.residual_ms <= MAX_GRID_RESIDUAL_MS
    }

    pub(crate) fn bpm(&self) -> f64 {
        60_000.0 / self.beat_ms
    }

    /// Fraction `0..1` of the beat at track time `ms`.
    pub(crate) fn phase(&self, ms: f64) -> f64 {
        ((ms - self.anchor_ms) / self.beat_ms).rem_euclid(1.0)
    }
}

/// What SYNC sends: the follower's speed and both constant grids (ms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SyncPlan {
    pub(crate) speed: f64,
    pub(crate) fit: GridFit,
    pub(crate) master: usize,
    pub(crate) master_fit: GridFit,
}

/// SYNC deck `d` to the other deck: match its tempo (track BPM × speed)
/// and phase. `Err` is the visible refusal reason.
pub(crate) fn sync_plan(decks: &[Deck; 2], d: usize, range_pct: f32) -> Result<SyncPlan, String> {
    let master = 1 - d;
    let mut fits = [None; 2];
    for k in [d, master] {
        let letter = DECKS[k];
        let deck = &decks[k];
        let Some(track) = &deck.track else {
            return Err(format!("SYNC: deck {letter} is empty"));
        };
        if !deck.ready() {
            return Err(format!("SYNC: deck {letter} is not playable yet"));
        }
        if track.beat_grid().is_none() {
            return Err(format!("SYNC: deck {letter} has no beat grid"));
        }
        if let Some(c) = track
            .confidence
            .filter(|c| *c < crate::track_waveform::LOW_CONFIDENCE)
        {
            return Err(format!(
                "SYNC: deck {letter} grid confidence is low ({c:.2})"
            ));
        }
        match track.fit {
            Some(fit) if fit.constant() => fits[k] = Some(fit),
            _ => return Err(format!("SYNC: deck {letter} has a variable-tempo grid")),
        }
    }
    let (Some(fit), Some(master_fit)) = (fits[d], fits[master]) else {
        return Err("SYNC: no beat grid".into());
    };
    let speed = master_fit.bpm() * decks[master].speed() / fit.bpm();
    let pct = (speed - 1.0) * 100.0;
    if pct.abs() > f64::from(range_pct) + 1e-9 {
        return Err(format!(
            "SYNC: needs {pct:+.1} %, beyond the ±{range_pct:.0} % tempo range"
        ));
    }
    Ok(SyncPlan {
        speed,
        fit,
        master,
        master_fit,
    })
}

/// A deck-local engine request (converted to source frames on flush).
enum Op {
    Sync(SyncPlan),
    TogglePlay,
    CueDown,
    CueUp,
    Seek(f64),
    LoopIn,
    LoopOut,
    LoopScale(f64),
}

/// Linear deck volume `0..=1` (default unity).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Volume(pub(crate) f32);

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

/// D4 channel knobs of one deck (UI units; the channel fader is
/// [`Deck::volume`]). Kept across track loads, like a hardware mixer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChannelUi {
    /// Trim in dB (±[`oxidj_audio::TRIM_DB`]).
    pub(crate) trim_db: f32,
    /// Low, mid, high in dB; at or below [`oxidj_audio::EQ_KILL_DB`] = kill.
    pub(crate) eq_db: [f32; 3],
    /// −1 low-pass .. 0 off .. +1 high-pass.
    pub(crate) filter: f32,
    pub(crate) assign: Assign,
    /// Cue button: when active, this channel is routed to the headphone cue bus.
    pub(crate) cue: bool,
}

impl Default for ChannelUi {
    fn default() -> Self {
        Self {
            trim_db: 0.0,
            eq_db: [0.0; 3],
            filter: 0.0,
            assign: Assign::Thru,
            cue: false,
        }
    }
}

/// A level meter as drawn: decaying peak and clip-indicator hold, fed by
/// the engine's meter readings once per UI frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MeterLevel {
    /// Displayed peak (linear, 1.0 = 0 dBFS).
    pub(crate) level: f32,
    /// UI frames the clip indicator stays lit.
    pub(crate) clip_frames: u32,
}

/// Frames a clip stays lit (~1.5 s at 60 fps).
pub(crate) const CLIP_HOLD_FRAMES: u32 = 90;
/// Per-frame decay of the displayed peak.
const METER_DECAY: f32 = 0.88;

impl MeterLevel {
    pub(crate) fn update(&mut self, reading: Meter) {
        let peak = if reading.peak.is_finite() {
            reading.peak.max(0.0)
        } else {
            0.0
        };
        self.level = peak.max(self.level * METER_DECAY);
        if self.level < 1e-4 {
            self.level = 0.0;
        }
        self.clip_frames = if reading.clip {
            CLIP_HOLD_FRAMES
        } else {
            self.clip_frames.saturating_sub(1)
        };
    }

    pub(crate) fn clipping(&self) -> bool {
        self.clip_frames > 0
    }

    /// Still animating (needs repaints).
    pub(crate) fn active(&self) -> bool {
        self.level > 0.0 || self.clipping()
    }
}

/// One deck: the loaded track, its audio and a mirror of the engine's
/// state (position, play, cue, loop), refreshed every frame from the audio
/// callback's snapshot. Controls queue engine commands; only a seek also
/// moves the mirror at once (scrub feel) until the callback confirms it.
#[derive(Default)]
pub(crate) struct Deck {
    pub(crate) track: Option<DeckTrack>,
    pub(crate) audio: DeckAudio,
    pub(crate) position_ms: f64,
    pub(crate) playing: bool,
    pub(crate) previewing: bool,
    pub(crate) cue_ms: f64,
    /// Tempo fader in percent (within the Settings range).
    pub(crate) tempo_pct: f32,
    /// Key lock (master tempo): the fader changes tempo, not pitch.
    pub(crate) keylock: bool,
    /// SYNC is on: the speed follows the other deck's tempo.
    pub(crate) sync: bool,
    /// Speed set by SYNC (exact; the fader shows it while SYNC is on).
    sync_speed: Option<f64>,
    /// Why the last SYNC press was refused (shown until the next press or load).
    pub(crate) sync_note: Option<String>,
    /// Channel fader (mixer); out of the path whenever the mixer is hidden.
    pub(crate) volume: Volume,
    /// D4 channel knobs (trim, EQ, filter, crossfader assign).
    pub(crate) channel: ChannelUi,
    pub(crate) loop_in: Option<f64>,
    /// Active manual loop `[start, end)`.
    pub(crate) loop_range: Option<(f64, f64)>,
    /// Hot-cue pads (ms), seeded from the imported hot cues.
    pub(crate) pads: [Option<f64>; PADS],
    /// CUE key (Q / P) is held and owes a release.
    pub(crate) cue_held: bool,
    /// CUE button is held by the pointer and owes a release.
    pub(crate) cue_pointer: bool,
    /// Album artwork: async load, decoded texture, or the placeholder.
    pub(crate) art: crate::artwork::DeckArt,
    outbox: Vec<Op>,
    /// The engine still plays the previous track: unload it before anything
    /// else is sent for this deck (a failed replacement must not leave
    /// hidden, uncontrollable audio).
    unload: bool,
    sent_gain: Option<f32>,
    sent_speed: Option<f64>,
    sent_keylock: Option<bool>,
}

impl Deck {
    /// Show `track` and start decoding its audio off the UI thread.
    pub(crate) fn load(&mut self, track: DeckTrack) {
        let mut pads = [None; PADS];
        for (n, pad) in pads.iter_mut().enumerate() {
            *pad = track.hot_cue(n + 1).map(|c| f64::from(c.start_ms));
        }
        let audio = DeckAudio::Loading(Loading::spawn(track.path.clone()));
        *self = Self {
            track: Some(track),
            audio,
            pads,
            tempo_pct: self.tempo_pct,
            keylock: self.keylock,
            volume: self.volume,
            channel: self.channel,
            sent_gain: self.sent_gain,
            sent_speed: self.sent_speed,
            sent_keylock: self.sent_keylock,
            unload: true,
            ..Self::default()
        };
    }

    /// Audio decoded and handed to the engine: the controls are live.
    pub(crate) fn ready(&self) -> bool {
        self.track.is_some() && matches!(self.audio, DeckAudio::Ready { .. })
    }

    pub(crate) fn duration(&self) -> f64 {
        self.track.as_ref().map_or(0.0, |t| t.duration_ms)
    }

    /// Effective speed multiplier: SYNC's while it is on, else the fader's.
    pub(crate) fn speed(&self) -> f64 {
        self.sync_speed
            .unwrap_or(1.0 + f64::from(self.tempo_pct) / 100.0)
    }

    /// SYNC off; the fader keeps the synced tempo (clamped to `range`).
    pub(crate) fn sync_off(&mut self, range: f32) {
        if let Some(speed) = self.sync_speed.take() {
            self.tempo_pct = (((speed - 1.0) * 100.0) as f32).clamp(-range, range);
        }
        self.sync = false;
    }

    pub(crate) fn toggle_keylock(&mut self) {
        self.keylock = !self.keylock;
    }

    fn queue(&mut self, op: Op) {
        if self.ready() {
            self.outbox.push(op);
        }
    }

    /// Jump to `ms` (seek, scrub, hot cue); the mirror moves at once.
    pub(crate) fn seek(&mut self, ms: f64) {
        if self.ready() {
            self.position_ms = ms.clamp(0.0, self.duration());
            self.queue(Op::Seek(self.position_ms));
        }
    }

    pub(crate) fn toggle_play(&mut self) {
        self.queue(Op::TogglePlay);
    }

    /// CUE pressed (CDJ): playing → stop at the cue point; paused off the
    /// cue point → set it here; paused on it → preview while held.
    pub(crate) fn cue_down(&mut self) {
        self.queue(Op::CueDown);
    }

    /// CUE released: a preview returns to the cue point.
    pub(crate) fn cue_up(&mut self) {
        self.queue(Op::CueUp);
    }

    /// A set pad jumps there; an empty pad stores the current position.
    pub(crate) fn pad(&mut self, index: usize) {
        if !self.ready() || index >= PADS {
            return;
        }
        match self.pads[index] {
            Some(ms) => self.seek(ms),
            None => self.pads[index] = Some(self.position_ms),
        }
    }

    pub(crate) fn loop_in(&mut self) {
        self.queue(Op::LoopIn);
    }

    /// OUT closes a loop after IN; on an active loop it exits the loop.
    pub(crate) fn loop_out(&mut self) {
        self.queue(Op::LoopOut);
    }

    pub(crate) fn loop_scale(&mut self, factor: f64) {
        self.queue(Op::LoopScale(factor));
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
    /// Track metadata block, large BPM number, time remaining, range button.
    pub(crate) metadata: [Option<Rect>; 2],
    pub(crate) bpm: [Option<Rect>; 2],
    pub(crate) time: [Option<Rect>; 2],
    pub(crate) range: [Option<Rect>; 2],
    pub(crate) pads: [Option<[Rect; PADS]>; 2],
    /// IN, OUT, ½, ×2.
    pub(crate) loops: [Option<[Rect; 4]>; 2],
    pub(crate) mixer: Option<Rect>,
    /// Channel faders.
    pub(crate) volume: [Option<Rect>; 2],
    pub(crate) trim: [Option<Rect>; 2],
    /// Low, mid, high EQ knobs.
    pub(crate) eq: [Option<[Rect; 3]>; 2],
    pub(crate) filter: [Option<Rect>; 2],
    /// Crossfader assign A, THRU, B.
    pub(crate) assign: [Option<[Rect; 3]>; 2],
    /// Level meters A, B, master, cue.
    pub(crate) meters: [Option<Rect>; METERS],
    pub(crate) master: Option<Rect>,
    pub(crate) crossfader: Option<Rect>,
    /// The crossfader's own show/hide icon toggle.
    pub(crate) crossfader_toggle: Option<Rect>,
    /// Mixer CUE buttons for cue bus routing.
    pub(crate) mixer_cue: [Option<Rect>; 2],
    /// Cue mix rotary.
    pub(crate) cue_mix: Option<Rect>,
    /// Cue volume rotary.
    pub(crate) cue_volume: Option<Rect>,
    /// Deck A overview first, deck B second.
    pub(crate) overviews: Option<[Rect; 2]>,
    /// Layout B: the x position of the needle shared by both overviews.
    pub(crate) overview_needle: Option<f32>,
    /// Centered artwork/placeholder rect of each deck half.
    pub(crate) artwork: [Option<Rect>; 2],
}

/// Measured callback statistics, only for an active device with callbacks.
pub(crate) fn audio_metrics(output: &oxidj_audio::OutputKind, stats: oxidj_audio::StatsSnapshot) -> Option<String> {
    if !matches!(output, oxidj_audio::OutputKind::Device { .. }) || stats.callbacks == 0 {
        return None;
    }
    let latency = if stats.latency_ns == 0 {
        "latency unavailable".to_owned()
    } else {
        format!("latency {:.1} ms", stats.latency_ns as f64 / 1_000_000.0)
    };
    Some(format!("{latency} · peak DSP {:.1}%", f64::from(stats.max_load_permille) / 10.0))
}

/// The whole deck area's state.
pub(crate) struct DeckArea {
    pub(crate) decks: [Deck; 2],
    /// Deck the keyboard (Space, 1–8, Tab) acts on.
    pub(crate) active: usize,
    /// A Tab press taken by [`LibraryApp::deck_input_hook`].
    pending_tab: bool,
    pub(crate) layout: DeckLayout,
    pub(crate) engine: Engine,
    /// The fake sink: blocks rendered by hand (tests); `None` once a real
    /// output is attached.
    manual: Option<Renderer>,
    /// Tempo fader range ±% (Settings → Decks), set by the app every frame.
    pub(crate) range: f32,
    /// Crossfader 0 (A) .. 0.5 (centre) .. 1 (B).
    pub(crate) crossfader: f32,
    /// Master volume 0..1.
    pub(crate) master: f32,
    /// Crossfader shown (Settings), set by the app every frame.
    pub(crate) crossfader_on: bool,
    /// Crossfader curve (Settings), set by the app every frame.
    pub(crate) curve: Curve,
    /// Cue mix: 0.0 = only deck A in cue, 1.0 = only deck B in cue, 0.5 = both equal.
    pub(crate) cue_mix: f32,
    /// Cue volume: 0.0..1.0, controls the cue bus output level.
    pub(crate) cue_volume: f32,
    /// Last mixer state the engine accepted (`Some(None)` = bypass).
    sent_mixer: Option<Option<MixerParams>>,
    /// Meters A, B, master, cue as drawn.
    pub(crate) meters: [MeterLevel; METERS],
}

impl Default for DeckArea {
    /// A manual engine (nothing plays until [`DeckArea::render`]);
    /// `main` attaches the device output with [`DeckArea::attach`].
    fn default() -> Self {
        let (engine, renderer) = Engine::manual(48_000);
        let mut decks: [Deck; 2] = Default::default();
        decks[0].channel.assign = Assign::A;
        decks[1].channel.assign = Assign::B;
        Self {
            decks,
            active: 0,
            pending_tab: false,
            layout: DeckLayout::default(),
            engine,
            manual: Some(renderer),
            range: tempo_range(TempoRange::default()),
            crossfader: 0.5,
            master: 1.0,
            crossfader_on: true,
            curve: Curve::default(),
            cue_mix: 0.5,
            cue_volume: 1.0,
            sent_mixer: None,
            meters: Default::default(),
        }
    }
}

impl DeckArea {
    /// Use `engine` (the device output, or the silent fallback).
    pub(crate) fn attach(&mut self, engine: Engine) {
        self.engine = engine;
        self.manual = None;
        self.sent_mixer = None;
        for deck in &mut self.decks {
            (deck.sent_gain, deck.sent_speed, deck.sent_keylock) = (None, None, None);
        }
    }

    /// Render `frames` on the fake sink (tests; no-op with a real output).
    #[cfg(test)]
    pub(crate) fn render(&mut self, frames: usize) -> Vec<f32> {
        self.render_channels(frames, 2)
    }

    #[cfg(test)]
    pub(crate) fn render_channels(&mut self, frames: usize, channels: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * channels];
        if let Some(renderer) = &mut self.manual {
            renderer.render(&mut out, channels);
        }
        out
    }

    /// Start of a frame: engine housekeeping, finished decodes go to the
    /// engine, and every deck mirror takes the callback's latest state (a
    /// deck with unapplied commands keeps its optimistic position).
    pub(crate) fn pump(&mut self) {
        self.engine.poll();
        for (shown, reading) in self.meters.iter_mut().zip(self.engine.meters()) {
            shown.update(reading);
        }
        self.send_unloads();
        for (d, deck) in self.decks.iter_mut().enumerate() {
            if deck.unload {
                continue; // ring full: retry next frame, keep Unload before Load
            }
            let Some(id) = deck.track.as_ref().map(|t| t.id) else {
                continue;
            };
            if let DeckAudio::Loading(loading) = &deck.audio {
                match loading.poll() {
                    None => {}
                    Some(Err(reason)) => deck.audio = DeckAudio::Failed(reason),
                    Some(Ok(pcm)) => {
                        let rate = pcm.sample_rate();
                        if self.engine.send(Command::Load {
                            deck: d,
                            id,
                            track: Arc::new(pcm),
                        }) {
                            deck.audio = DeckAudio::Ready { rate };
                        } else {
                            deck.audio = DeckAudio::Failed("audio engine busy".into());
                        }
                    }
                }
            }
            let DeckAudio::Ready { rate } = deck.audio else {
                continue;
            };
            let snap = self.engine.deck(d);
            if snap.id != Some(id) || self.engine.pending(d) {
                continue;
            }
            let ms = |frame: f64| frame * 1000.0 / f64::from(rate);
            deck.position_ms = ms(snap.frame);
            deck.cue_ms = ms(snap.cue);
            deck.loop_in = snap.loop_in.map(ms);
            deck.loop_range = snap.loop_range.map(|(a, b)| (ms(a), ms(b)));
            deck.playing = snap.playing;
            deck.previewing = snap.previewing;
        }
    }

    /// SYNC pressed on deck `d`: on → off; off → match the other deck's
    /// tempo and phase, or refuse with a visible reason.
    pub(crate) fn press_sync(&mut self, d: usize) {
        if self.decks[d].sync {
            self.decks[d].sync_off(self.range);
            return;
        }
        match sync_plan(&self.decks, d, self.range) {
            Ok(plan) => {
                let deck = &mut self.decks[d];
                deck.sync = true;
                deck.sync_speed = Some(plan.speed);
                deck.sync_note = None;
                deck.queue(Op::Sync(plan));
            }
            Err(reason) => {
                self.decks[d].sync_off(self.range);
                self.decks[d].sync_note = Some(reason);
            }
        }
    }

    /// While SYNC is on, the follower's speed tracks the master's tempo
    /// (phase is aligned only on press); a master that can no longer be
    /// followed turns SYNC off with the reason.
    fn follow_sync(&mut self) {
        for d in 0..2 {
            if !self.decks[d].sync {
                continue;
            }
            match sync_plan(&self.decks, d, self.range) {
                Ok(plan) => self.decks[d].sync_speed = Some(plan.speed),
                Err(reason) => {
                    self.decks[d].sync_off(self.range);
                    self.decks[d].sync_note = Some(reason);
                }
            }
        }
    }

    /// What the mixer knobs ask of the engine.
    pub(crate) fn mixer_params(&self) -> MixerParams {
        MixerParams {
            channels: [0, 1].map(|d| {
                let deck = &self.decks[d];
                let c = deck.channel;
                ChannelParams {
                    trim: oxidj_audio::trim_gain(c.trim_db),
                    eq: c.eq_db.map(oxidj_audio::eq_gain),
                    filter: c.filter,
                    fader: deck.volume.0,
                    assign: c.assign,
                    cue: c.cue,
                }
            }),
            crossfader: self.crossfader_on.then_some(self.crossfader),
            curve: self.curve,
            master: self.master,
            cue_mix: self.cue_mix,
            cue_volume: self.cue_volume,
        }
    }

    /// Any meter still moving (repaint while the mixer is shown).
    pub(crate) fn meters_active(&self) -> bool {
        self.meters.iter().any(MeterLevel::active)
    }

    /// End of a frame: send every queued control, the mixer (D4: its
    /// knobs while `mixer` is shown, else `None` = exact unity bypass),
    /// tempo and key lock changes. Deck gain stays at unity: the channel
    /// fader lives in the mixer.
    pub(crate) fn flush(&mut self, mixer: bool) {
        self.follow_sync();
        self.send_unloads();
        let params = mixer.then(|| self.mixer_params());
        if self.sent_mixer != Some(params) && self.engine.send(Command::Mixer(params)) {
            self.sent_mixer = Some(params);
        }
        let rates = [0, 1].map(|d| match self.decks[d].audio {
            DeckAudio::Ready { rate } => f64::from(rate),
            _ => 0.0,
        });
        for (d, deck) in self.decks.iter_mut().enumerate() {
            let gain = 1.0;
            if deck.sent_gain != Some(gain) && self.engine.send(Command::Gain { deck: d, gain }) {
                deck.sent_gain = Some(gain);
            }
            let speed = deck.speed();
            if deck.sent_speed != Some(speed) && self.engine.send(Command::Speed { deck: d, speed })
            {
                deck.sent_speed = Some(speed);
            }
            if deck.sent_keylock != Some(deck.keylock)
                && self.engine.send(Command::KeyLock {
                    deck: d,
                    on: deck.keylock,
                })
            {
                deck.sent_keylock = Some(deck.keylock);
            }
            let DeckAudio::Ready { rate } = deck.audio else {
                deck.outbox.clear();
                continue;
            };
            let frame = |ms: f64| ms * f64::from(rate) / 1000.0;
            for op in deck.outbox.drain(..) {
                let command = match op {
                    Op::Sync(plan) => {
                        let to = |ms: f64, rate: f64| ms * rate / 1000.0;
                        let master_rate = rates[plan.master];
                        if master_rate <= 0.0 {
                            continue;
                        }
                        Command::Sync {
                            deck: d,
                            master: plan.master,
                            speed: plan.speed,
                            beat: to(plan.fit.beat_ms, f64::from(rate)),
                            anchor: to(plan.fit.anchor_ms, f64::from(rate)),
                            master_beat: to(plan.master_fit.beat_ms, master_rate),
                            master_anchor: to(plan.master_fit.anchor_ms, master_rate),
                        }
                    }
                    Op::TogglePlay => Command::TogglePlay { deck: d },
                    Op::CueDown => Command::CueDown { deck: d },
                    Op::CueUp => Command::CueUp { deck: d },
                    Op::Seek(ms) => Command::Seek {
                        deck: d,
                        frame: frame(ms),
                    },
                    Op::LoopIn => Command::LoopIn { deck: d },
                    Op::LoopOut => Command::LoopOut { deck: d },
                    Op::LoopScale(f) => Command::LoopScale { deck: d, factor: f },
                };
                self.engine.send(command);
            }
        }
    }

    /// Stop and empty the engine deck of every deck that was just replaced.
    fn send_unloads(&mut self) {
        for (d, deck) in self.decks.iter_mut().enumerate() {
            if deck.unload && self.engine.send(Command::Unload { deck: d }) {
                deck.unload = false;
            }
        }
    }

    /// Status-bar text: where audio goes, or why it is silent.
    pub(crate) fn status(&self) -> (String, bool) {
        match self.engine.error() {
            Some(reason) => (format!("NO AUDIO · {reason} · decks run silently"), false),
            None => (format!("AUDIO · {}", self.engine.output()), true),
        }
    }

    pub(crate) fn any_playing(&self) -> bool {
        self.decks.iter().any(|d| d.playing)
    }

    /// Any deck still decoding (repaint for progress).
    pub(crate) fn any_loading(&self) -> bool {
        self.decks
            .iter()
            .any(|d| matches!(d.audio, DeckAudio::Loading(_)))
    }

    /// Collect finished artwork jobs; textures are built on the UI thread.
    pub(crate) fn pump_artwork(&mut self, ctx: &egui::Context) {
        for (d, deck) in self.decks.iter_mut().enumerate() {
            deck.art.poll(ctx, d);
            if deck.art.is_loading() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
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
    let i = beats
        .partition_point(|b| b.0 <= t)
        .clamp(1, beats.len().saturating_sub(1).max(1));
    let len = beats.get(i)?.0 - beats.get(i - 1)?.0;
    (len > 0.0).then_some(len)
}

/// `(beat length, fraction 0..1 since the last beat)` at `t`.
pub(crate) fn phase_at(grid: &BeatGrid, t: f64) -> Option<(f64, f64)> {
    let beats = grid.beats_in((f64::NEG_INFINITY, f64::INFINITY));
    let len = beat_len_at(grid, t)?;
    let i = beats.partition_point(|b| b.0 <= t);
    let last = if i == 0 {
        beats[0].0 - len
    } else {
        beats[i - 1].0
    };
    Some((len, ((t - last) / len).rem_euclid(1.0)))
}

/// Seam readout text: phase offset (ms, wrapped to ±½ beat of deck A) and
/// tempo-adjusted BPM delta; `None` unless both decks have a beat grid.
pub(crate) fn seam_readout(decks: &[Deck; 2]) -> Option<String> {
    let mut phase = [(0.0, 0.0); 2];
    let mut bpm = [0.0; 2];
    for (d, deck) in decks.iter().enumerate() {
        let track = deck.track.as_ref()?;
        phase[d] = match track.fit {
            Some(fit) if fit.constant() => (fit.beat_ms, fit.phase(deck.position_ms)),
            _ => phase_at(track.beat_grid()?, deck.position_ms)?,
        };
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

/// How one pixel column summarises the stored peaks it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Envelope {
    /// Loudest peak per band (zoomed detail: every transient shows).
    Peak,
    /// Mean peak per band over finer stored columns (overview: a
    /// time-compressed envelope where breaks, drops and phrases stand out
    /// instead of one flat full-height bar).
    Mean,
}

/// Stored columns per pixel the [`Envelope::Mean`] overview averages.
const MEAN_COLUMNS_PER_PX: usize = 8;
/// Overview auto-gain ceiling (a silent track is not blown up to noise).
const MAX_OVERVIEW_GAIN: f32 = 20.0;

/// Per-pixel band extents of `peaks` over a track-time `window`.
struct Columns<'a> {
    bands: [&'a [oxidj_application::WaveformPeak]; 3],
    /// Samples per stored column at the chosen level.
    col: f64,
    total: f64,
    /// First visible sample and samples per pixel.
    start: f64,
    step: f64,
    envelope: Envelope,
}

impl<'a> Columns<'a> {
    fn new(peaks: &'a WaveformPeaks, (t0, t1): (f64, f64), px: usize, envelope: Envelope) -> Self {
        let rate = f64::from(peaks.sample_rate().max(1));
        let span = (t1 - t0) * rate / 1000.0;
        let want = match envelope {
            Envelope::Peak => px,
            Envelope::Mean => px * MEAN_COLUMNS_PER_PX,
        };
        let level = pick_level(peaks, span.max(1.0) as u64, want);
        Self {
            bands: WaveformBand::ALL.map(|b| peaks.band(level, b).unwrap_or(&[])),
            col: f64::from(peaks.samples_per_peak()) * (1u64 << level) as f64,
            total: peaks.sample_count() as f64,
            start: t0 * rate / 1000.0,
            step: span / px.max(1) as f64,
            envelope,
        }
    }

    /// `[low, mid, high]` extents (ungained, `0..=1`) of the pixel column
    /// centred on point `i`; `None` outside the track.
    fn at(&self, i: usize) -> Option<[f32; 3]> {
        let len = self.bands[0].len();
        let centre = self.start + self.step * i as f64;
        let (s0, s1) = (centre - self.step / 2.0, centre + self.step / 2.0);
        if s1 <= 0.0 || s0 >= self.total || len == 0 {
            return None;
        }
        let c0 = ((s0.max(0.0) / self.col) as usize).min(len - 1);
        let c1 = ((s1 / self.col).ceil() as usize).clamp(c0 + 1, len);
        Some(core::array::from_fn(|b| {
            let it = self.bands[b][c0..c1].iter().map(|p| amplitude_ext(*p));
            match self.envelope {
                Envelope::Peak => it.fold(0.0, f32::max),
                Envelope::Mean => it.sum::<f32>() / (c1 - c0) as f32,
            }
        }))
    }
}

/// Per-track overview auto-gain: the loudest [`Envelope::Mean`] column of
/// the whole track reaches [`GAIN_TARGET`] of the half, so quiet and loud
/// masters both use the height (display only; `auto` off = unity).
pub fn overview_gain(peaks: &WaveformPeaks, window: (f64, f64), width: f32, auto: bool) -> f32 {
    if !auto || width.is_nan() || width < 1.0 || window.1 <= window.0 {
        return 1.0;
    }
    let px = width.floor() as usize;
    let columns = Columns::new(peaks, window, px, Envelope::Mean);
    let loudest = (0..=px)
        .filter_map(|i| columns.at(i))
        .flatten()
        .fold(0.0, f32::max);
    if loudest > f32::EPSILON {
        (GAIN_TARGET / loudest).clamp(1.0, MAX_OVERVIEW_GAIN)
    } else {
        1.0
    }
}

/// Nested `[low, mid, high]` layer extents of one column (`raw` band
/// extents times `gain`): the outer edge is the true loudest band
/// (`max`, clamped to 1) and each band's ring is as thick as its share of
/// the column's ([`SHARE_WEIGHTS`]-weighted) band sum, so low ⊇ mid ⊇
/// high always nest and each colour covers an area proportional to its
/// energy. (Ranking bands by their own
/// extent, as in the details panel, made the smallest band the largest
/// visible area: real masters read as one pale core.)
pub fn stack(raw: [f32; 3], gain: f32) -> [f32; 3] {
    let e = raw.map(|v| (v * gain).max(0.0));
    let total = e.iter().copied().fold(0.0, f32::max).min(1.0);
    let w: [f32; 3] = core::array::from_fn(|b| e[b] * SHARE_WEIGHTS[b]);
    let sum = w[0] + w[1] + w[2];
    if sum <= f32::EPSILON {
        return [0.0; 3];
    }
    [total, total * (w[1] + w[2]) / sum, total * w[2] / sum]
}

/// Display weights of the `[low, mid, high]` ring shares (not the outer
/// edge). Mid-band peaks (200 Hz–2.5 kHz) dominate real masters; checked
/// on real library tracks, an unweighted low ring was a hairline, so the
/// low share is lifted and the high core trimmed a little.
pub const SHARE_WEIGHTS: [f32; 3] = [1.5, 1.0, 0.85];

/// Fill `layers` (cleared first) with deck `colours` waveform strips that
/// grow from the seam: `up` = from `rect.bottom()` upwards (deck A), else
/// from `rect.top()` downwards (deck B). Points sit on pixel boundaries
/// over the track-time `window`; columns outside the track are skipped
/// (the strip breaks there). Every vertex is clamped into `rect`, so the
/// painted bounds never leave it.
///
/// W12: three opaque nested band layers in fixed order — `layers[0]` low
/// (outermost), `[1]` mid, `[2]` high (core) — see [`stack`].
#[allow(clippy::too_many_arguments)]
pub fn build_half(
    peaks: &WaveformPeaks,
    window: (f64, f64),
    gain: f32,
    rect: Rect,
    up: bool,
    colours: DeckColours,
    fringe: f32,
    envelope: Envelope,
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
    let columns = Columns::new(peaks, window, px, envelope);
    let base = if up { rect.bottom() } else { rect.top() };
    let dir = if up { -1.0 } else { 1.0 };
    let reach = (rect.height() - fringe - 0.5).max(1.0);
    let colors = [colours.low, colours.mid, colours.high];
    let mut connected = false;
    for i in 0..=px {
        let x = rect.left() + rect.width() * i as f32 / px as f32;
        let Some(raw) = columns.at(i) else {
            connected = false;
            continue;
        };
        for (layer, (mesh, ext)) in layers.iter_mut().zip(stack(raw, gain)).enumerate() {
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
    let v = |y: f32, color| egui::epaint::Vertex {
        pos: Pos2::new(x, y),
        uv: egui::epaint::WHITE_UV,
        color,
    };
    let n = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        v(fringe, Color32::TRANSPARENT),
        v(edge, color),
        v(base, color),
    ]);
    if connect && n >= 3 {
        let a = n - 3;
        for k in 0..2 {
            mesh.indices
                .extend_from_slice(&[a + k, a + k + 1, n + k, a + k + 1, n + k + 1, n + k]);
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
fn paint_meshes(
    ui: &Ui,
    painter: &egui::Painter,
    id: egui::Id,
    fill: impl FnOnce(&mut [Mesh; 3], &mut Mesh),
) {
    let buffers = ui.data_mut(|d| d.get_temp_mut_or_default::<Buffers>(id).clone());
    let mut slots = buffers.0.lock().unwrap_or_else(|e| e.into_inner());
    let mut owned: [Mesh; 4] = slots
        .each_mut()
        .map(|s| Arc::get_mut(s).map(std::mem::take).unwrap_or_default());
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
fn time_grid(
    painter: &egui::Painter,
    rect: Rect,
    window: (f64, f64),
    duration: f64,
    colours: DeckColours,
    label: Color32,
    step: f64,
    labels_at_top: bool,
) {
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
        let color = if major {
            alpha(colours.outer, 150)
        } else {
            colours.beat
        };
        painter.rect_filled(
            Rect::from_min_max(pos2(x0, rect.top()), pos2(x0 + w, rect.bottom())),
            0.0,
            color,
        );
        if major {
            let galley = painter.layout_no_wrap(format_clock(t), FontId::monospace(10.0), label);
            let y = if labels_at_top {
                rect.top() + 2.0
            } else {
                rect.bottom() - 2.0 - galley.size().y
            };
            let pos = pos2(x0 + w + 2.0, y);
            if Rect::from_min_size(pos, galley.size()).max.x <= rect.right() && y >= rect.top() {
                painter.galley(pos, galley, label);
            }
        }
    }
}

/// Bars per phrase tick on the overview.
pub const PHRASE_BARS: usize = 16;
/// Hot-cue flag size on the overview (>= 6 px tall, numbered).
pub const OVERVIEW_FLAG: f32 = 12.0;
/// Pointer distance (px) at which an overview cue names itself on hover.
const CUE_HOVER_PX: f32 = 6.0;

/// One 1 px full-height phrase tick every [`PHRASE_BARS`] bars (bar 1, 17,
/// 33, …) of `grid` in `window`, clamped into `rect`, appended to `mesh`.
pub fn phrase_lines(
    grid: &BeatGrid,
    window: (f64, f64),
    rect: Rect,
    colour: Color32,
    mesh: &mut Mesh,
) {
    for &(t, bar) in grid.beats_in(window) {
        if bar.is_some_and(|n| (n - 1) % PHRASE_BARS == 0) {
            let x0 = (x_at(t, window, rect) - 0.5).clamp(rect.left(), rect.right() - 1.0);
            mesh.add_colored_rect(
                Rect::from_min_max(pos2(x0, rect.top()), pos2(x0 + 1.0, rect.bottom())),
                colour,
            );
        }
    }
}

/// Name of the cue, loop or hot cue nearest to `x` (within
/// [`CUE_HOVER_PX`]) on a `rect` showing `window`, for the hover tooltip.
pub(crate) fn cue_label_at(deck: &Deck, window: (f64, f64), rect: Rect, x: f32) -> Option<String> {
    let track = deck.track.as_ref()?;
    let mut best: Option<(f32, String)> = None;
    let mut consider = |ms: f64, label: String| {
        let dx = (x_at(ms, window, rect) - x).abs();
        if dx <= CUE_HOVER_PX && best.as_ref().is_none_or(|(b, _)| dx < *b) {
            best = Some((dx, label));
        }
    };
    for (i, pad) in deck.pads.iter().enumerate() {
        if let Some(ms) = *pad {
            consider(ms, format!("Hot cue {} · {}", i + 1, format_clock(ms)));
        }
    }
    for cue in &track.cues {
        let start = f64::from(cue.start_ms);
        match (cue.hot_cue, cue.end_ms) {
            (_, Some(end)) => {
                let range = format!("{}–{}", format_clock(start), format_clock(f64::from(end)));
                let name = cue
                    .hot_cue
                    .map_or("Loop".to_owned(), |n| format!("Loop (hot cue {n})"));
                consider(start, format!("{name} · {range}"));
                consider(f64::from(end), format!("{name} end · {range}"));
            }
            (None, None) => consider(start, format!("Memory cue · {}", format_clock(start))),
            (Some(_), None) => {}
        }
    }
    if let Some((a, b)) = deck.loop_range {
        let range = format!("{}–{}", format_clock(a), format_clock(b));
        consider(a, format!("Loop IN · {range}"));
        consider(b, format!("Loop OUT · {range}"));
    }
    best.map(|(_, label)| label)
}

/// Imported loops (and the active manual loop) as shaded regions, hot cues
/// as numbered coloured markers with a flag at the `outer` edge, memory
/// cues as small triangles there. Everything is clamped into `rect`.
#[allow(clippy::too_many_arguments)]
fn paint_cues(
    painter: &egui::Painter,
    rect: Rect,
    window: (f64, f64),
    deck: &Deck,
    top_edge: bool,
    flag: f32,
    memory: Color32,
    label: Color32,
) {
    let Some(track) = &deck.track else { return };
    let span = |a: f64, b: f64| {
        let (x0, x1) = (x_at(a, window, rect), x_at(b, window, rect));
        Rect::from_x_y_ranges(x0.max(rect.left())..=x1.min(rect.right()), rect.y_range())
    };
    // W12: a loop is a light fill plus an opaque bracket (edge bar + end
    // ticks >= 6 px) so it stays findable on the 34 px overview.
    let bracket = |a: f64, b: f64, colour: Color32, fill: u8| {
        let r = span(a, b);
        if r.width() <= 0.0 {
            return;
        }
        painter.rect_filled(r, 0.0, alpha(colour, fill));
        let tick = (flag * 0.6).max(6.0).min(r.height());
        let (y0, y1) = if top_edge {
            (r.top(), r.top() + tick)
        } else {
            (r.bottom() - tick, r.bottom())
        };
        let bar = if top_edge {
            r.top()..=r.top() + 2.0
        } else {
            r.bottom() - 2.0..=r.bottom()
        };
        painter.rect_filled(Rect::from_x_y_ranges(r.x_range(), bar), 0.0, colour);
        for x in [r.left(), r.right() - 2.0] {
            let x = x.clamp(rect.left(), rect.right() - 2.0);
            painter.rect_filled(
                Rect::from_min_max(pos2(x, y0), pos2(x + 2.0, y1)),
                0.0,
                colour,
            );
        }
    };
    for cue in track.cues.iter().filter(|c| c.end_ms.is_some()) {
        let colour = cue
            .hot_cue
            .map_or(memory, |n| pad_colour(n as usize, Some(cue)));
        bracket(
            f64::from(cue.start_ms),
            f64::from(cue.end_ms.unwrap_or(cue.start_ms)),
            colour,
            46,
        );
    }
    if let Some((a, b)) = deck.loop_range {
        bracket(a, b, label, 40);
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
            vec![
                pos2(x - s, edge_y),
                pos2(x + s, edge_y),
                pos2(x, edge_y + dir * s * 1.4),
            ],
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
        painter.rect_filled(
            Rect::from_min_max(pos2(x0, rect.top()), pos2(x0 + 2.0, rect.bottom())),
            0.0,
            colour,
        );
        let size = vec2(flag, flag.min(rect.height()));
        let fx = x0.min(rect.right() - size.x).max(rect.left());
        let fy = if top_edge {
            rect.top()
        } else {
            rect.bottom() - size.y
        };
        let f = Rect::from_min_size(pos2(fx, fy), size);
        painter.rect_filled(f, 2.0, colour);
        let dark = rgb(0x10, 0x10, 0x18);
        let text = if contrast(colour, dark) >= contrast(colour, Color32::WHITE) {
            dark
        } else {
            Color32::WHITE
        };
        let galley = painter.layout_no_wrap(
            n.to_string(),
            FontId::proportional((size.y - 2.0).clamp(6.0, 11.0)),
            text,
        );
        let gp = f.center() - galley.size() / 2.0;
        if f.contains_rect(Rect::from_min_size(gp, galley.size())) {
            painter.galley(gp, galley, text);
        }
    }
}

/// Deck letter chip (fill + letter) inside `rect`.
pub(crate) fn chip(painter: &egui::Painter, rect: Rect, letter: &str, colours: DeckColours) {
    painter.rect_filled(rect, 3.0, colours.outer);
    let galley = painter.layout_no_wrap(
        letter.to_owned(),
        FontId::proportional(12.0),
        colours.chip_text,
    );
    painter.galley(
        rect.center() - galley.size() / 2.0,
        galley,
        colours.chip_text,
    );
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
        let is_tab = |e: &egui::Event| matches!(e, egui::Event::Key { key: egui::Key::Tab, modifiers, .. } if modifiers.is_none());
        if raw
            .events
            .iter()
            .any(|e| is_tab(e) && matches!(e, egui::Event::Key { pressed: true, .. }))
        {
            self.decks.pending_tab = true;
        }
        raw.events.retain(|e| !is_tab(e));
    }

    /// Deck keys (only with the decks shown and Settings closed). Section
    /// toggles (Ctrl+Shift+1–4, 5 = crossfader) always work; the DJ keys only when no
    /// widget has keyboard focus, so focused controls keep Space/Enter.
    pub(crate) fn handle_deck_keys(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.decks.pending_tab) && self.show_decks {
            self.decks.active = 1 - self.decks.active;
        }
        // A cue preview always owes a release, even if focus/settings changes
        // while the key is held. Never leave preview audio stuck behind a guard.
        let cue_keys = if self.settings.simple_dj_keys {
            [egui::Key::Z, egui::Key::N]
        } else {
            [egui::Key::Q, egui::Key::P]
        };
        let guarded = !self.show_decks
            || self.settings_ui.open
            || ctx.memory(|m| m.focused()).is_some()
            || ctx.wants_keyboard_input();
        for (d, key) in cue_keys.into_iter().enumerate() {
            if self.decks.decks[d].cue_held
                && (guarded || ctx.input(|i| !i.key_down(key) || !i.focused))
            {
                self.decks.decks[d].cue_held = false;
                self.decks.decks[d].cue_up();
            }
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
            if i.consume_key(chord, egui::Key::Num5) {
                self.settings.crossfader = !self.settings.crossfader;
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
            // CUE keys: press = CUE down, release = CUE up (hold-preview).
            let cue_keys = if self.settings.simple_dj_keys {
                [egui::Key::Z, egui::Key::N]
            } else {
                [egui::Key::Q, egui::Key::P]
            };
            for (d, key) in cue_keys.into_iter().enumerate() {
                let deck = &mut self.decks.decks[d];
                if i.consume_key(none, key) && !deck.cue_held {
                    deck.cue_held = true;
                    deck.cue_down();
                }
                if deck.cue_held && i.key_released(key) {
                    deck.cue_held = false;
                    deck.cue_up();
                }
            }
            if self.settings.simple_dj_keys {
                for (d, key) in [egui::Key::X, egui::Key::M].into_iter().enumerate() {
                    if i.consume_key(none, key) {
                        self.decks.decks[d].toggle_play();
                    }
                }
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
        if let Some(deck) = load {
            if let Some(member) = self.selected_imported {
                self.load_imported_member(member, deck);
            } else if let Some(id) = self.selected {
                self.load_deck(deck, id);
            }
        }
    }

    /// Apply Settings without waiting on a decoder; stale jobs and textures are
    /// dropped, and both loaded decks restart under the new budget.
    pub(crate) fn sync_artwork_settings(&mut self) {
        let bytes = if self.settings.show_artwork {
            self.settings.artwork_cache_mib.clamp(1, 1024) as usize * 1024 * 1024
        } else {
            0
        };
        if self.artwork_cache.configure(bytes) {
            for d in 0..2 {
                self.decks.decks[d].art = Default::default();
                if bytes > 0
                    && let Some(id) = self.decks.decks[d].track.as_ref().map(|t| t.id)
                {
                    self.decks.decks[d].art.load(
                        id,
                        self.artwork_cache.clone(),
                        self.db_path.clone(),
                        None,
                    );
                }
            }
        }
    }

    /// Load catalog track `id` onto `deck` (0 = A).
    pub(crate) fn load_deck(&mut self, deck: usize, id: u64) {
        let Some(item) = self.tracks.iter().find(|t| t.track.id.as_u64() == id) else {
            return;
        };
        let track = DeckTrack::read(self.catalog.as_ref().ok(), item);
        self.sync_artwork_settings();
        self.notice = format!("Loaded {} to deck {}", track.name, DECKS[deck]);
        self.decks.decks[deck].load(track);
        if self.settings.show_artwork {
            #[cfg(test)]
            let gate = self.artwork_gate.take();
            #[cfg(not(test))]
            let gate = None;
            self.decks.decks[deck].art.load(
                id,
                self.artwork_cache.clone(),
                self.db_path.clone(),
                gate,
            );
        }
    }

    /// Re-read loaded tracks after a catalog reload (a waveform analysed
    /// meanwhile replaces the time grid in place); position is kept.
    pub(crate) fn refresh_decks(&mut self) {
        for d in 0..2 {
            let Some(id) = self.decks.decks[d].track.as_ref().map(|t| t.id) else {
                continue;
            };
            let Some(item) = self.tracks.iter().find(|t| t.track.id.as_u64() == id) else {
                continue;
            };
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

pub(crate) fn tempo_range(range: TempoRange) -> f32 {
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

/// Height of the deck panel's single content band (cover, controls, pitch
/// column share its top and bottom edges).
pub(crate) const BAND_H: f32 = 104.0;

const BTN_GAP: f32 = 2.0;
const LOOP_W: f32 = 20.0;
const LOOP_H: f32 = 20.0;
const REPEAT_W: f32 = 14.0;
const LEFT_W: f32 = 4.0 * LOOP_W + REPEAT_W + 4.0 * BTN_GAP;
const CP_W: f32 = 44.0;
const CP_H: f32 = 24.0;
const PAD_SIZE: f32 = 24.0;
const PAD_GAP: f32 = 2.0;
const PADS_W: f32 = 4.0 * PAD_SIZE + 3.0 * PAD_GAP;
const PITCH_W: f32 = 70.0;
const FADER_W: f32 = 16.0;
const META_H: f32 = 30.0;
/// Narrowest deck column that still fits the cover beside the controls.
const COVER_MIN_COLUMN: f32 =
    RAIL + 4.0 + BAND_H + 8.0 + LEFT_W + 8.0 + PADS_W + 8.0 + PITCH_W + 6.0 + FADER_W;

/// Total deck-area height for `sections` at window `screen` size, for a
/// deck area `width` wide.
pub(crate) fn area_height(sections: DeckSections, mixer_h: f32, screen: Rect, _width: f32) -> f32 {
    let mut h = SECTION_BAR_HEIGHT;
    if sections.detail {
        h += GAP + detail_height(screen.height());
    }
    if sections.overviews {
        h += GAP + 2.0 * OVERVIEW_HEIGHT + 2.0;
    }
    let row = bottom_row_height(sections, mixer_h);
    if row > 0.0 {
        h += GAP + row;
    }
    h
}

/// Height the library table keeps below the deck area (the deck area
/// scrolls instead of squeezing the library under this).
pub(crate) const LIBRARY_MIN_H: f32 = 120.0;

/// Layout B bottom row (deck A, mixer, deck B): as tall as its tallest part.
pub(crate) fn bottom_row_height(sections: DeckSections, mixer_h: f32) -> f32 {
    let strip = if sections.strip { BAND_H } else { 0.0 };
    let mixer = if sections.mixer { mixer_h } else { 0.0 };
    strip.max(mixer)
}

impl LibraryApp {
    /// The whole deck area inside its panel.
    pub(crate) fn deck_area(&mut self, ui: &mut Ui, tokens: ColorTokens) {
        let mut layout = DeckLayout::default();
        let width = ui.available_width();
        let screen = ui.ctx().screen_rect();
        let sections = self.settings.sections;
        let mixer_h = crate::mixer_view::mixer_height(self.settings.mixer);
        let height = area_height(sections, mixer_h, screen, width);
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
        if sections.overviews {
            let r = take(2.0 * OVERVIEW_HEIGHT + 2.0, &mut y);
            let a = Rect::from_min_size(r.min, vec2(width, OVERVIEW_HEIGHT));
            let b = Rect::from_min_size(
                pos2(r.left(), a.bottom() + 2.0),
                vec2(width, OVERVIEW_HEIGHT),
            );
            for (d, rect) in [a, b].into_iter().enumerate() {
                self.overview(ui, d, rect, tokens, colours[d], &layout);
            }
            layout.overviews = Some([a, b]);
            // Static layout: both overviews share one needle x position.
            let inner_left = a.left() + OVERVIEW_INSET;
            let nx = (inner_left + a.right()) / 2.0;
            ui.painter()
                .vline(nx, a.top()..=b.bottom(), Stroke::new(1.0_f32, tokens.focus));
            layout.overview_needle = Some(nx);
        }
        let row_h = bottom_row_height(sections, mixer_h);
        if row_h > 0.0 {
            // Layout B: deck A | mixer | deck B in one row.
            let row = take(row_h, &mut y);
            let centre_w = if sections.mixer {
                crate::mixer_view::mixer_total_width(self.settings.mixer, screen.width()).min(width)
            } else {
                0.0
            };
            if sections.strip {
                self.control_strip(ui, row, centre_w, tokens, colours, &mut layout);
            }
            if sections.mixer {
                let r = Rect::from_center_size(row.center(), vec2(centre_w, row_h));
                self.mixer(ui, r, tokens, colours, &mut layout);
            }
        }
        if self.decks.any_playing() || (sections.mixer && self.decks.meters_active()) {
            ui.ctx().request_repaint();
        } else if self.decks.any_loading() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.decks.layout = layout;
    }

    /// Section bar: deck chips with the active deck marked, then the four
    /// chevron toggles at the right edge.
    fn section_bar(
        &mut self,
        ui: &mut Ui,
        bar: Rect,
        tokens: ColorTokens,
        colours: [DeckColours; 2],
        layout: &mut DeckLayout,
    ) {
        let painter = ui.painter_at(bar);
        let label = painter.layout_no_wrap(
            "DECKS".into(),
            FontId::proportional(11.0),
            tokens.muted_text,
        );
        painter.galley(
            pos2(bar.left() + 2.0, bar.center().y - label.size().y / 2.0),
            label,
            tokens.muted_text,
        );
        let mut x = bar.left() + 48.0;
        for d in 0..2 {
            let r = Rect::from_min_size(pos2(x, bar.center().y - 9.0), vec2(18.0, 18.0));
            chip(&painter, r, DECKS[d], colours[d]);
            if self.decks.active == d {
                painter.rect_stroke(
                    r.expand(1.5),
                    4.0,
                    Stroke::new(1.5_f32, tokens.focus),
                    egui::StrokeKind::Outside,
                );
            }
            x += 24.0;
        }
        let active = painter.layout_no_wrap(
            format!("active deck {} · Tab switches", DECKS[self.decks.active]),
            FontId::proportional(11.0),
            tokens.muted_text,
        );
        painter.galley(
            pos2(x + 4.0, bar.center().y - active.size().y / 2.0),
            active,
            tokens.muted_text,
        );
        let labels = ["Detail", "Overviews", "Controls", "Mixer"];
        let mut rects = [Rect::NOTHING; 4];
        let mut right = bar.right();
        for i in (0..4).rev() {
            let text =
                painter.layout_no_wrap(labels[i].into(), FontId::proportional(11.5), tokens.text);
            let w = text.size().x + 24.0;
            let r = Rect::from_min_max(pos2(right - w, bar.top()), pos2(right, bar.bottom()));
            right -= w + 2.0;
            rects[i] = r;
            let expanded = self.settings.sections.get(i);
            let name = format!("{} section", DeckSections::NAMES[i]);
            let response = ui.interact(r, ui.id().with(("deck-section", i)), Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, expanded, &name)
            });
            if response.hovered() || response.has_focus() {
                painter.rect_filled(r, 3.0, tokens.surface);
            }
            if response.has_focus() {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(2.0_f32, tokens.focus),
                    egui::StrokeKind::Inside,
                );
            }
            let ink = if expanded {
                tokens.text
            } else {
                tokens.muted_text
            };
            let c = pos2(r.left() + 10.0, r.center().y);
            let pts = if expanded {
                vec![
                    pos2(c.x - 4.0, c.y - 2.0),
                    pos2(c.x + 4.0, c.y - 2.0),
                    pos2(c.x, c.y + 3.0),
                ]
            } else {
                vec![
                    pos2(c.x - 2.0, c.y - 4.0),
                    pos2(c.x - 2.0, c.y + 4.0),
                    pos2(c.x + 3.0, c.y),
                ]
            };
            painter.add(Shape::convex_polygon(pts, ink, Stroke::NONE));
            painter.galley(
                pos2(r.left() + 18.0, r.center().y - text.size().y / 2.0),
                text,
                ink,
            );
            let verb = if expanded { "Hide" } else { "Show" };
            if response
                .on_hover_text(format!(
                    "{verb} the {} (Ctrl+Shift+{}; also in Settings)",
                    DeckSections::NAMES[i].to_lowercase(),
                    i + 1
                ))
                .clicked()
            {
                let slot = self.settings.sections.get_mut(i);
                *slot = !*slot;
            }
        }
        layout.toggles = Some(rects);
    }

    /// The merged detail waveform.
    fn detail_view(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        tokens: ColorTokens,
        colours: [DeckColours; 2],
        layout: &mut DeckLayout,
    ) {
        let response = ui.interact(rect, ui.id().with("merged-detail"), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Other,
                true,
                "Merged deck waveform: deck A above, deck B below",
            )
        });
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
                self.settings.zoom_bars =
                    zoom_step(self.settings.zoom_bars, if scroll > 0.0 { -1 } else { 1 });
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
            let wm =
                painter.layout_no_wrap(DECKS[d].into(), FontId::proportional(size), c.watermark);
            let wpos = pos2(
                half.right() - wm.size().x - 16.0,
                half.center().y - wm.size().y / 2.0,
            );
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
                paint_meshes(
                    ui,
                    &painter,
                    ui.id().with(("deck-half", d)),
                    |layers, lines| {
                        match &track.peaks {
                            Some(peaks) => build_half(
                                peaks,
                                window,
                                track.gain(auto_gain),
                                half,
                                up,
                                c,
                                if smooth { FRINGE } else { 0.0 },
                                Envelope::Peak,
                                layers,
                            ),
                            None => layers.iter_mut().for_each(|m| {
                                m.vertices.clear();
                                m.indices.clear();
                            }),
                        }
                        lines.vertices.clear();
                        lines.indices.clear();
                        if let (true, Some(grid)) = (beat_lines, track.beat_grid()) {
                            let gc = GridColours {
                                beat: c.beat,
                                bar: c.outer,
                                label: tokens.text,
                            };
                            beat_grid::build_lines(grid, window, half, gc, lines);
                        }
                    },
                );
                if track.beat_grid().is_none() {
                    time_grid(
                        &painter,
                        half,
                        window,
                        track.duration_ms,
                        c,
                        tokens.muted_text,
                        1000.0 * speed,
                        up,
                    );
                }
                paint_cues(
                    &painter,
                    half,
                    window,
                    deck,
                    up,
                    14.0,
                    tokens.error,
                    tokens.text,
                );
                if track.peaks.is_none() {
                    let g = painter.layout_no_wrap(
                        NOT_READY.into(),
                        FontId::proportional(11.0),
                        tokens.muted_text,
                    );
                    let y = if up {
                        half.top() + 16.0
                    } else {
                        half.bottom() - 16.0 - g.size().y
                    };
                    painter.galley(pos2(half.left() + 30.0, y), g, tokens.muted_text);
                }
            } else {
                let g = painter.layout_no_wrap(
                    format!(
                        "Deck {} empty · select a track and press {} (or Load)",
                        DECKS[d], DECKS[d]
                    ),
                    FontId::proportional(12.0),
                    tokens.muted_text,
                );
                painter.galley(half.center() - g.size() / 2.0, g, tokens.muted_text);
            }
            // Coloured rail and deck chip at the outer-left corner.
            painter.rect_filled(
                Rect::from_min_size(half.min, vec2(RAIL, half.height())),
                0.0,
                c.outer,
            );
            let cy = if up {
                half.top() + 4.0
            } else {
                half.bottom() - 22.0
            };
            chip(
                &painter,
                Rect::from_min_size(pos2(half.left() + RAIL + 4.0, cy), vec2(18.0, 18.0)),
                DECKS[d],
                c,
            );
        }
        let painter = ui.painter_at(rect);
        // The seam: a 1 px rect (a line's caps would poke past the clip).
        painter.rect_filled(
            Rect::from_x_y_ranges(rect.x_range(), seam - 0.5..=seam + 0.5),
            0.0,
            tokens.border,
        );
        // One playhead across both halves, with small caps inside the rect.
        painter.rect_filled(
            Rect::from_min_max(pos2(ph - 1.0, rect.top()), pos2(ph + 1.0, rect.bottom())),
            0.0,
            tokens.error,
        );
        painter.add(Shape::convex_polygon(
            vec![
                pos2(ph - 5.0, rect.top()),
                pos2(ph + 5.0, rect.top()),
                pos2(ph, rect.top() + 6.0),
            ],
            tokens.error,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![
                pos2(ph - 5.0, rect.bottom()),
                pos2(ph, rect.bottom() - 6.0),
                pos2(ph + 5.0, rect.bottom()),
            ],
            tokens.error,
            Stroke::NONE,
        ));
        if self.settings.show_phase {
            if let Some(text) = seam_readout(&self.decks.decks) {
                let g = painter.layout_no_wrap(text, FontId::monospace(11.0), tokens.text);
                let box_ = Rect::from_min_size(
                    pos2(ph + 8.0, seam - g.size().y / 2.0 - 3.0),
                    g.size() + vec2(12.0, 6.0),
                );
                let box_ = box_.translate(vec2((rect.right() - box_.right()).min(0.0), 0.0));
                painter.rect_filled(box_, 3.0, tokens.surface);
                painter.rect_stroke(
                    box_,
                    3.0,
                    Stroke::new(1.0_f32, tokens.border),
                    egui::StrokeKind::Inside,
                );
                painter.galley(box_.min + vec2(6.0, 3.0), g, tokens.text);
                layout.readout = Some(box_);
            }
        }
        layout.detail = Some(rect);
        layout.halves = Some(halves);
        layout.playhead_x = Some(ph);
    }

    /// Deck panel: deck A in the left column, deck B in the right one, each
    /// one aligned content band (cover, metadata + controls, pitch column).
    fn control_strip(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        centre_w: f32,
        tokens: ColorTokens,
        colours: [DeckColours; 2],
        layout: &mut DeckLayout,
    ) {
        layout.strip = Some(rect);
        let gap = if centre_w > 0.0 {
            12.0 + centre_w
        } else {
            12.0
        };
        let column = (rect.width() - gap) / 2.0;
        let top = rect.top() + ((rect.height() - BAND_H) / 2.0).max(0.0);
        let cols = [
            Rect::from_min_size(pos2(rect.left(), top), vec2(column, BAND_H)),
            Rect::from_min_size(pos2(rect.right() - column, top), vec2(column, BAND_H)),
        ];
        let range = tempo_range(self.settings.tempo_range);
        self.decks.range = range;
        let mut load = None;
        let mut sync_press = None;
        let mut range_press = false;
        for (d, col) in cols.into_iter().enumerate() {
            let c = colours[d];
            let letter = DECKS[d];
            let top = col.top();
            let bottom = col.top() + BAND_H;
            let mirror = d == 1 && self.settings.mirror_b;
            let painter = ui.painter_at(col);
            painter.rect_filled(col, self.settings.deck_theme.radius(), c.wash);
            let rail_x = if mirror {
                col.right() - RAIL
            } else {
                col.left()
            };
            painter.rect_filled(
                Rect::from_min_size(pos2(rail_x, col.top()), vec2(RAIL, BAND_H)),
                0.0,
                c.outer,
            );
            let deck = &mut self.decks.decks[d];
            let loaded = deck.ready();
            let mut x = col.left() + RAIL + 4.0;
            // Pitch column geometry: right edge, or (mirrored deck B) the
            // left edge, nearest the mixer.
            let fader_r = if mirror {
                Rect::from_min_max(
                    pos2(col.left() + 4.0, top),
                    pos2(col.left() + 4.0 + FADER_W, bottom),
                )
            } else {
                Rect::from_min_max(pos2(col.right() - FADER_W, top), pos2(col.right(), bottom))
            };
            let (pitch_left, pitch_right, body_right) = if mirror {
                let l = fader_r.right() + 6.0;
                x = l + PITCH_W + 8.0;
                (l, l + PITCH_W, col.right() - RAIL - 4.0)
            } else {
                let r = fader_r.left() - 6.0;
                (r - PITCH_W, r, r - PITCH_W - 6.0)
            };
            // Cover: square, full band height; dropped when too narrow.
            let cover_fits = column >= COVER_MIN_COLUMN;
            let show_cover = self.settings.show_artwork && deck.track.is_some() && cover_fits;
            let mut body_right = body_right;
            if show_cover {
                let cx = if mirror { body_right - BAND_H } else { x };
                let slot = Rect::from_min_size(pos2(cx, top), vec2(BAND_H, BAND_H));
                crate::artwork::paint_cover(&painter, slot, &deck.art, c, &tokens);
                layout.artwork[d] = Some(slot);
                if mirror {
                    body_right = cx - 8.0;
                } else {
                    x += BAND_H + 8.0;
                }
            }
            // Controls column: loop row, CUE, PLAY, bottom aligned.
            let play_r = Rect::from_min_size(
                pos2(x + (LEFT_W - CP_W) / 2.0, bottom - CP_H),
                vec2(CP_W, CP_H),
            );
            let cue_r = play_r.translate(vec2(0.0, -(CP_H + 3.0)));
            let loops_y = cue_r.top() - 3.0 - LOOP_H;
            let at_cue = loaded
                && (deck.previewing
                    || (!deck.playing && (deck.position_ms - deck.cue_ms).abs() < 1.0));
            let cue = icon_button(
                ui,
                cue_r,
                Icon::CircleDot,
                &format!("Deck {letter} cue"),
                at_cue,
                loaded,
                tokens.warning,
                tokens.warning,
                tokens,
            )
            .on_hover_text(format!("Deck {letter} CUE ({}): stopped, sets the cue point; on it, hold to preview; playing, returns to it", ["Q", "P"][d]));
            // Pointer: press = CUE down, release = CUE up; keyboard
            // activation of the focused button is a press + release.
            let held = cue.is_pointer_button_down_on();
            if held && !deck.cue_pointer {
                deck.cue_pointer = true;
                deck.cue_down();
            } else if !held && deck.cue_pointer {
                deck.cue_pointer = false;
                deck.cue_up();
            } else if cue.clicked() {
                deck.cue_down();
                deck.cue_up();
            }
            let (play_icon, play_name) = if deck.playing {
                (Icon::Pause, "pause")
            } else {
                (Icon::Play, "play")
            };
            if icon_button(
                ui,
                play_r,
                play_icon,
                &format!("Deck {letter} play/pause"),
                deck.playing,
                loaded,
                tokens.success,
                tokens.success,
                tokens,
            )
            .on_hover_text(format!("Deck {letter} {play_name}"))
            .clicked()
            {
                deck.toggle_play();
            }
            layout.cue[d] = Some(cue_r);
            layout.play[d] = Some(play_r);
            // Loop row: IN, OUT, repeat glyph, then halve / double.
            let mut loop_rects = [Rect::NOTHING; 4];
            let has_loop = deck.loop_range.is_some();
            let mut lx = x;
            for k in 0..4 {
                if k == 2 {
                    let glyph = Rect::from_min_size(pos2(lx, loops_y), vec2(REPEAT_W, LOOP_H));
                    let ink = if has_loop { c.outer } else { tokens.muted_text };
                    crate::icons::paint(
                        &ui.painter_at(glyph),
                        Icon::Repeat,
                        glyph.shrink(1.0),
                        ink,
                    );
                    lx += REPEAT_W + BTN_GAP;
                }
                let r = Rect::from_min_size(pos2(lx, loops_y), vec2(LOOP_W, LOOP_H));
                lx += LOOP_W + BTN_GAP;
                loop_rects[k] = r;
                let enabled = loaded && (k < 2 || has_loop);
                let active = (k == 0 && deck.loop_in.is_some()) || (k == 1 && has_loop);
                let name = ["loop in", "loop out / exit", "halve loop", "double loop"][k];
                let name = format!("Deck {letter} {name}");
                let clicked = match k {
                    0 | 1 => icon_button(
                        ui,
                        r,
                        [Icon::LogIn, Icon::LogOut][k],
                        &name,
                        active,
                        enabled,
                        c.outer,
                        tokens.text,
                        tokens,
                    )
                    .on_hover_text(name.clone())
                    .clicked(),
                    _ => deck_button(
                        ui,
                        r,
                        ["½", "×2"][k - 2],
                        &name,
                        false,
                        enabled,
                        c.outer,
                        tokens,
                    )
                    .on_hover_text(name.clone())
                    .clicked(),
                };
                if clicked {
                    match k {
                        0 => deck.loop_in(),
                        1 => deck.loop_out(),
                        2 => deck.loop_scale(0.5),
                        _ => deck.loop_scale(2.0),
                    }
                }
            }
            layout.loops[d] = Some(loop_rects);
            // Hot-cue pads 4 x 2, bottom aligned with PLAY; always 1..8
            // left to right, also on deck B.
            let pads_x = x + LEFT_W + 8.0;
            let pads_top = bottom - (2.0 * PAD_SIZE + PAD_GAP);
            let mut pad_rects = [Rect::NOTHING; PADS];
            for (p, pad_rect) in pad_rects.iter_mut().enumerate() {
                let (col_i, row_i) = ((p % 4) as f32, (p / 4) as f32);
                let r = Rect::from_min_size(
                    pos2(
                        pads_x + col_i * (PAD_SIZE + PAD_GAP),
                        pads_top + row_i * (PAD_SIZE + PAD_GAP),
                    ),
                    vec2(PAD_SIZE, PAD_SIZE),
                );
                *pad_rect = r;
                let n = p + 1;
                let set = deck.pads[p].is_some();
                let colour = pad_colour(n, deck.track.as_ref().and_then(|t| t.hot_cue(n)));
                let name = format!(
                    "Deck {letter} hot cue {n} ({})",
                    if set { "set" } else { "empty" }
                );
                if pad_button(ui, r, n, set, colour, loaded, &name, tokens).clicked() {
                    deck.pad(p);
                }
            }
            layout.pads[d] = Some(pad_rects);
            // Metadata block (top of the band), Load at its right end.
            let load_r = Rect::from_min_size(pos2(body_right - 40.0, top), vec2(40.0, LOOP_H));
            let r = deck_button(
                ui,
                load_r,
                "Load",
                &format!("Load selected track to deck {letter}"),
                false,
                true,
                c.outer,
                tokens,
            );
            if r.on_hover_text(format!(
                "Load the selected library track to deck {letter} ({letter} key)"
            ))
            .clicked()
            {
                load = Some(d);
            }
            layout.load[d] = Some(load_r);
            let meta = Rect::from_min_max(pos2(x, top), pos2(load_r.left() - 4.0, top + META_H));
            layout.metadata[d] = Some(meta);
            if self.settings.show_meta && meta.width() > 20.0 {
                let (title, line2, ink2) = match (&deck.track, &deck.audio) {
                    (Some(t), DeckAudio::Failed(reason)) => (
                        t.name.clone(),
                        format!("cannot play: {reason}"),
                        tokens.error,
                    ),
                    (Some(t), DeckAudio::Loading(l)) => {
                        let pct = l
                            .fraction()
                            .map(|f| format!(" {:.0} %", f * 100.0))
                            .unwrap_or_default();
                        (t.name.clone(), format!("decoding{pct}…"), tokens.muted_text)
                    }
                    (Some(t), _) if deck.sync_note.is_some() => (
                        t.name.clone(),
                        deck.sync_note.clone().unwrap_or_default(),
                        tokens.error,
                    ),
                    (Some(t), _) => (
                        t.name.clone(),
                        format!(
                            "DECK {letter} · {} / {}",
                            format_clock(deck.position_ms),
                            format_clock(t.duration_ms)
                        ),
                        tokens.muted_text,
                    ),
                    (None, _) => (
                        format!("Deck {letter} · empty"),
                        String::new(),
                        tokens.muted_text,
                    ),
                };
                let ink1 = if title.starts_with("Deck ") && deck.track.is_none() {
                    tokens.muted_text
                } else {
                    tokens.text
                };
                let mp = ui.painter_at(meta);
                for (k, (text, ink, size)) in [(title, ink1, 13.0_f32), (line2, ink2, 10.5)]
                    .into_iter()
                    .enumerate()
                {
                    if text.is_empty() {
                        continue;
                    }
                    let g = egui::WidgetText::from(egui::RichText::new(text).size(size).color(ink))
                        .into_galley(
                            ui,
                            Some(egui::TextWrapMode::Truncate),
                            meta.width(),
                            egui::TextStyle::Small,
                        );
                    let y = meta.top() + if k == 0 { 0.0 } else { 15.5 };
                    mp.galley(pos2(meta.left(), y), g, ink);
                }
            }
            // Pitch column: BPM, SYNC + key lock, KEY, remaining, range.
            let right_aligned = |ui: &Ui, text: String, font: FontId, ink: Color32, y: f32| {
                let g = ui.painter().layout_no_wrap(text, font, ink);
                let px = if mirror {
                    pitch_left
                } else {
                    pitch_right - g.size().x
                };
                let r = Rect::from_min_size(pos2(px, y), g.size());
                ui.painter().galley(r.min, g, ink);
                r
            };
            let bpm_text = match &deck.track {
                Some(t) => t
                    .bpm_at(deck.position_ms)
                    .map(|b| format!("{:.1}", b * deck.speed()))
                    .unwrap_or_else(|| "—".into()),
                None => "—".into(),
            };
            let bpm_r = right_aligned(ui, bpm_text, FontId::proportional(24.0), tokens.text, top);
            layout.bpm[d] = Some(bpm_r);
            let row_y = top + 30.0;
            let (sync_r, lock_r) = if mirror {
                let s = Rect::from_min_size(pos2(pitch_left, row_y), vec2(40.0, LOOP_H));
                let l = Rect::from_min_size(pos2(s.right() + BTN_GAP, row_y), vec2(22.0, LOOP_H));
                (s, l)
            } else {
                let l = Rect::from_min_size(pos2(pitch_right - 22.0, row_y), vec2(22.0, LOOP_H));
                let s =
                    Rect::from_min_size(pos2(l.left() - BTN_GAP - 40.0, row_y), vec2(40.0, LOOP_H));
                (s, l)
            };
            let sync_hover = match &deck.sync_note {
                Some(note) => note.clone(),
                None if deck.sync => format!(
                    "Deck {letter} SYNC on: follows deck {}'s tempo; press to release",
                    DECKS[1 - d]
                ),
                None => format!(
                    "Deck {letter} SYNC: match deck {}'s tempo and align to the nearest beat",
                    DECKS[1 - d]
                ),
            };
            if deck_button(
                ui,
                sync_r,
                "SYNC",
                &format!("Deck {letter} sync"),
                deck.sync,
                loaded,
                c.outer,
                tokens,
            )
            .on_hover_text(sync_hover)
            .clicked()
            {
                sync_press = Some(d);
            }
            let key_hover = format!(
                "Deck {letter} key lock (master tempo): {}",
                if deck.keylock {
                    "on, tempo changes keep the pitch"
                } else {
                    "off, pitch follows tempo"
                }
            );
            if icon_button(
                ui,
                lock_r,
                Icon::Lock,
                &format!("Deck {letter} key lock"),
                deck.keylock,
                loaded,
                c.outer,
                tokens.text,
                tokens,
            )
            .on_hover_text(key_hover)
            .clicked()
            {
                deck.toggle_keylock();
            }
            layout.sync[d] = Some(sync_r);
            layout.key_lock[d] = Some(lock_r);
            // The library does not yet carry a musical key for loaded decks.
            right_aligned(
                ui,
                "KEY —".into(),
                FontId::monospace(11.0),
                tokens.muted_text,
                top + 54.0,
            );
            let remaining = deck
                .track
                .as_ref()
                .map(|t| {
                    format!(
                        "−{}",
                        format_clock((t.duration_ms - deck.position_ms).max(0.0))
                    )
                })
                .unwrap_or_else(|| "−–:––".into());
            let time_r = right_aligned(
                ui,
                remaining,
                FontId::monospace(11.0),
                tokens.text,
                top + 69.0,
            );
            crate::icons::paint(
                ui.painter(),
                Icon::Timer,
                Rect::from_center_size(
                    pos2(
                        if mirror {
                            time_r.right() + 9.0
                        } else {
                            time_r.left() - 9.0
                        },
                        time_r.center().y,
                    ),
                    vec2(12.0, 12.0),
                ),
                tokens.muted_text,
            );
            layout.time[d] = Some(time_r);
            // Range button and the live tempo value on the bottom row.
            let range_r = if mirror {
                Rect::from_min_max(
                    pos2(pitch_left, bottom - 18.0),
                    pos2(pitch_left + 38.0, bottom),
                )
            } else {
                Rect::from_min_max(
                    pos2(pitch_right - 38.0, bottom - 18.0),
                    pos2(pitch_right, bottom),
                )
            };
            if deck_button(
                ui,
                range_r,
                &format!("±{range:.0}%"),
                &format!("Deck {letter} tempo range"),
                false,
                true,
                c.outer,
                tokens,
            )
            .on_hover_text("Tempo range of both faders; click to cycle ±6 / ±10 / ±16 %")
            .clicked()
            {
                range_press = true;
            }
            layout.range[d] = Some(range_r);
            let mut tempo = (((deck.speed() - 1.0) * 100.0) as f32 * 10.0).round() / 10.0;
            tempo = tempo.clamp(-range, range);
            // Vertical pitch fader across the full band.
            let mut slider = ui
                .scope(|ui| {
                    crate::track_waveform::slider_look(ui, tokens);
                    ui.visuals_mut().selection.bg_fill = c.outer;
                    // A visible rail: the default rail fill vanishes on the deck tint.
                    let widgets = &mut ui.visuals_mut().widgets;
                    widgets.inactive.bg_fill = tokens.border;
                    widgets.hovered.bg_fill = tokens.border;
                    widgets.active.bg_fill = tokens.border;
                    ui.spacing_mut().slider_width = BAND_H;
                    ui.spacing_mut().interact_size.y = FADER_W;
                    ui.put(
                        fader_r,
                        egui::Slider::new(&mut tempo, -range..=range)
                            .vertical()
                            .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 0.5 })
                            .show_value(false)
                            .step_by(0.1),
                    )
                })
                .inner;
            // Hover-wheel takes the fader's 0.1 % step, in both directions,
            // clamped to the Settings range. The response must be marked
            // changed so the fader's normal SYNC release runs.
            let wheel_id = slider.id.with("tempo-wheel-remainder");
            let mut wheel = ui
                .ctx()
                .data(|data| data.get_temp::<f32>(wheel_id).unwrap_or(0.0));
            if !slider.enabled() || !slider.hovered() {
                wheel = 0.0;
            }
            crate::mixer_view::mixer_wheel(ui, &slider, &mut wheel, -1000.0..=1000.0, 0.1);
            let steps = (wheel * 10.0 + wheel.signum() * 0.00001).trunc();
            wheel -= steps * 0.1;
            let adjusted = (tempo + steps * 0.1).clamp(-range, range);
            if (adjusted >= range && wheel > 0.0) || (adjusted <= -range && wheel < 0.0) {
                wheel = 0.0;
            }
            ui.ctx().data_mut(|data| data.insert_temp(wheel_id, wheel));
            if adjusted != tempo {
                tempo = adjusted;
                slider.mark_changed();
            }
            slider.widget_info(|| {
                egui::WidgetInfo::slider(true, f64::from(tempo), format!("Deck {letter} tempo"))
            });
            let slider = slider.on_hover_text(format!(
                "Deck {letter} tempo (±{range:.0} %, Settings → Decks); double-click resets"
            ));
            let reset = reset_clicked(&slider);
            if reset {
                tempo = 0.0;
            }
            if slider.changed() || reset {
                // Moving the fader takes the tempo back from SYNC.
                deck.sync_off(range);
                deck.tempo_pct = tempo;
            } else if !deck.sync {
                // A narrower Settings range clamps the fader.
                deck.tempo_pct = deck.tempo_pct.clamp(-range, range);
            }
            let value = ui.painter().layout_no_wrap(
                format!("{:+.1}%", ((deck.speed() - 1.0) * 100.0) as f32),
                FontId::monospace(11.0),
                tokens.text,
            );
            ui.painter().galley(
                pos2(
                    if mirror {
                        range_r.right() + 4.0
                    } else {
                        range_r.left() - 4.0 - value.size().x
                    },
                    range_r.center().y - value.size().y / 2.0,
                ),
                value,
                tokens.text,
            );
            layout.tempo[d] = Some(fader_r);
        }
        if range_press {
            self.settings.tempo_range = match self.settings.tempo_range {
                TempoRange::Six => TempoRange::Ten,
                TempoRange::Ten => TempoRange::Sixteen,
                TempoRange::Sixteen => TempoRange::Six,
            };
        }
        if let Some(d) = sync_press {
            self.decks.press_sync(d);
        }
        if let Some(deck) = load {
            if let Some(member) = self.selected_imported {
                self.load_imported_member(member, deck);
            } else if let Some(id) = self.selected {
                self.load_deck(deck, id);
            }
        }
    }

    /// Deck `d`'s full-track overview: seek by click/drag, played part
    /// dimmed, detail-window box, cues and loops.
    fn overview(
        &mut self,
        ui: &mut Ui,
        d: usize,
        rect: Rect,
        tokens: ColorTokens,
        c: DeckColours,
        layout: &DeckLayout,
    ) {
        let letter = DECKS[d];
        let response = ui.interact(
            rect,
            ui.id().with(("deck-overview", d)),
            Sense::click_and_drag(),
        );
        let deck = &mut self.decks.decks[d];
        let duration = deck.duration();
        let inner = Rect::from_min_max(pos2(rect.left() + OVERVIEW_INSET, rect.top()), rect.max);
        response.widget_info(|| {
            egui::WidgetInfo::slider(
                deck.track.is_some(),
                deck.position_ms / 1000.0,
                format!("Deck {letter} overview: click or drag to seek"),
            )
        });
        if duration > 0.0
            && (response.is_pointer_button_down_on() || response.clicked() || response.dragged())
        {
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
            match &track.peaks {
                Some(peaks) => {
                    // W12: time-compressed mean envelope, per-track auto-gain.
                    let gain = overview_gain(peaks, window, up.width(), self.settings.auto_gain);
                    let phrase = alpha(tokens.text, 110);
                    for (k, (half, is_up)) in [(up, true), (down, false)].into_iter().enumerate() {
                        paint_meshes(
                            ui,
                            &painter,
                            ui.id().with(("deck-overview-mesh", d, k)),
                            |layers, lines| {
                                build_half(
                                    peaks,
                                    window,
                                    gain,
                                    half,
                                    is_up,
                                    c,
                                    FRINGE,
                                    Envelope::Mean,
                                    layers,
                                );
                                lines.vertices.clear();
                                lines.indices.clear();
                                if let (0, Some(grid)) = (k, track.beat_grid()) {
                                    phrase_lines(grid, window, inner, phrase, lines);
                                }
                            },
                        );
                    }
                }
                None => {
                    time_grid(
                        &painter,
                        inner,
                        window,
                        duration,
                        c,
                        tokens.muted_text,
                        15_000.0,
                        true,
                    );
                    let g = painter.layout_no_wrap(
                        NOT_READY.into(),
                        FontId::proportional(11.0),
                        tokens.muted_text,
                    );
                    painter.galley(
                        pos2(
                            inner.center().x - g.size().x / 2.0,
                            inner.bottom() - g.size().y - 1.0,
                        ),
                        g,
                        tokens.muted_text,
                    );
                }
            }
            let x = x_at(deck.position_ms, window, inner).clamp(inner.left(), inner.right());
            // Played part dimmed.
            painter.rect_filled(
                Rect::from_min_max(inner.min, pos2(x, inner.bottom())),
                0.0,
                alpha(tokens.background, 150),
            );
            // Cues over the dimming so played markers stay readable.
            paint_cues(
                &painter,
                inner,
                window,
                deck,
                true,
                OVERVIEW_FLAG,
                tokens.error,
                tokens.text,
            );
            if let Some(label) = response
                .hover_pos()
                .and_then(|p| cue_label_at(deck, window, inner, p.x))
            {
                response.clone().on_hover_text_at_pointer(label);
            }
            if let Some((a, b)) = layout.windows[d] {
                let (x0, x1) = (
                    x_at(a, window, inner).max(inner.left()),
                    x_at(b, window, inner).min(inner.right()),
                );
                if x1 > x0 {
                    let r = Rect::from_x_y_ranges(
                        x0..=x1.max(x0 + 2.0).min(inner.right()),
                        inner.y_range(),
                    );
                    painter.rect_stroke(
                        r,
                        1.0,
                        Stroke::new(1.0_f32, tokens.focus),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            let px = x.clamp(inner.left(), inner.right() - 2.0);
            painter.rect_filled(
                Rect::from_min_max(pos2(px, inner.top()), pos2(px + 2.0, inner.bottom())),
                0.0,
                tokens.error,
            );
        } else {
            let g = painter.layout_no_wrap(
                format!("Deck {letter} · empty"),
                FontId::proportional(11.0),
                tokens.muted_text,
            );
            painter.galley(
                inner.left_center() + vec2(8.0, -g.size().y / 2.0),
                g,
                tokens.muted_text,
            );
        }
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(RAIL, rect.height())),
            0.0,
            c.outer,
        );
        chip(
            &painter,
            Rect::from_center_size(
                pos2(rect.left() + RAIL + 11.0, rect.center().y),
                vec2(18.0, 18.0),
            ),
            letter,
            c,
        );
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0_f32, tokens.focus),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// A strip button painted at `rect` (exact size, centred label): `active`
/// fills it with `accent`; native click/keyboard response and AccessKit name.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deck_button(
    ui: &mut Ui,
    rect: Rect,
    text: &str,
    name: &str,
    active: bool,
    enabled: bool,
    accent: Color32,
    tokens: ColorTokens,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, ui.id().with(("deck-button", name)), sense);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, active, name)
    });
    let painter = ui.painter_at(rect);
    let dark = rgb(0x10, 0x10, 0x18);
    let (fill, ink) = if active {
        (
            accent,
            if contrast(accent, dark) >= contrast(accent, Color32::WHITE) {
                dark
            } else {
                Color32::WHITE
            },
        )
    } else if response.hovered() && enabled {
        (tokens.surface, tokens.text)
    } else {
        (
            tokens.background,
            if enabled {
                tokens.text
            } else {
                tokens.muted_text
            },
        )
    };
    painter.rect_filled(rect.shrink(0.5), 3.0, fill);
    let outline = if enabled {
        tokens.border
    } else {
        alpha(tokens.border, 110)
    };
    painter.rect_stroke(
        rect.shrink(0.5),
        3.0,
        Stroke::new(1.0_f32, outline),
        egui::StrokeKind::Inside,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            3.0,
            Stroke::new(2.0_f32, tokens.focus),
            egui::StrokeKind::Inside,
        );
    }
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), ink);
    painter.galley(rect.center() - g.size() / 2.0, g, ink);
    response
}

/// A strip button with a Lucide `icon` instead of a label. The icon is drawn
/// in `idle_ink` when idle and enabled, muted when disabled and in the
/// contrasting ink when `active` fills the button with `accent`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    icon: Icon,
    name: &str,
    active: bool,
    enabled: bool,
    accent: Color32,
    idle_ink: Color32,
    tokens: ColorTokens,
) -> egui::Response {
    let response = deck_button(ui, rect, "", name, active, enabled, accent, tokens);
    let dark = rgb(0x10, 0x10, 0x18);
    let ink = if active {
        if contrast(accent, dark) >= contrast(accent, Color32::WHITE) {
            dark
        } else {
            Color32::WHITE
        }
    } else if enabled {
        idle_ink
    } else {
        tokens.muted_text
    };
    let side = rect.width().min(rect.height()) - 8.0;
    crate::icons::paint(
        &ui.painter_at(rect),
        icon,
        Rect::from_center_size(rect.center(), vec2(side, side)),
        ink,
    );
    response
}

/// A double click on a fader (egui's `Slider` senses drags only, so its
/// own `double_clicked()` never fires).
pub(crate) fn reset_clicked(slider: &egui::Response) -> bool {
    slider.hovered()
        && slider.ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        })
}

/// A hot-cue pad: filled with its colour when set, outlined when empty.
#[allow(clippy::too_many_arguments)]
fn pad_button(
    ui: &mut Ui,
    rect: Rect,
    n: usize,
    set: bool,
    colour: Color32,
    enabled: bool,
    name: &str,
    tokens: ColorTokens,
) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.id().with(("pad", name)),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, set, name));
    let painter = ui.painter_at(rect);
    let dark = rgb(0x10, 0x10, 0x18);
    let (fill, ink) = if set {
        (
            colour,
            if contrast(colour, dark) >= contrast(colour, Color32::WHITE) {
                dark
            } else {
                Color32::WHITE
            },
        )
    } else {
        (
            tokens.surface,
            if enabled {
                tokens.text
            } else {
                tokens.muted_text
            },
        )
    };
    painter.rect_filled(rect.shrink(0.5), 3.0, fill);
    painter.rect_stroke(
        rect.shrink(0.5),
        3.0,
        Stroke::new(1.0_f32, alpha(colour, if set { 255 } else { 170 })),
        egui::StrokeKind::Inside,
    );
    if response.hovered() || response.has_focus() {
        painter.rect_stroke(
            rect,
            3.0,
            Stroke::new(
                if response.has_focus() { 2.0_f32 } else { 1.0 },
                tokens.focus,
            ),
            egui::StrokeKind::Inside,
        );
    }
    let g = painter.layout_no_wrap(n.to_string(), FontId::proportional(12.0), ink);
    painter.galley(rect.center() - g.size() / 2.0, g, ink);
    response.on_hover_text(name)
}

/// Deck letters loaded with track `id` (table badges).
pub(crate) fn badges(loaded: [Option<u64>; 2], id: u64) -> [bool; 2] {
    loaded.map(|l| l == Some(id))
}
