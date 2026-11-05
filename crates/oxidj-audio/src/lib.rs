//! OxiDJ realtime deck engine (M8/D2, ADR-008 foundation).
//!
//! Two decks play decoded stereo PCM from RAM. The UI owns an [`Engine`]; the
//! audio callback owns a [`Renderer`]. They share nothing but:
//!
//! * a bounded lock-free SPSC ring ([`rtrb`]) carrying [`Command`]s
//!   UI → callback, drained at the start of every block, and
//! * per-deck atomics the callback publishes after every block
//!   ([`DeckSnapshot`]), read by the UI to draw the playhead.
//!
//! **RT-safety rules for [`Renderer::render`]** (the code that runs in the
//! device callback): no allocation, no deallocation, no lock, no I/O, no
//! panicking operation. Tracks arrive as `Arc<TrackPcm>`; the UI keeps its
//! own clone of every track it ever sent until the callback has released it
//! ([`Engine::poll`] drops retired tracks only when the UI holds the last
//! reference), so dropping an `Arc` in the callback is only a counter
//! decrement, never a free. Samples are read with `get`, never indexed.
//!
//! Positions are **source frames** (`f64`) of the loaded track, so seeking,
//! cue points and loops are sample accurate in the track's own clock. When
//! the track rate differs from the device rate (44.1 vs 48 kHz) the callback
//! reads at a fractional step with linear interpolation (ADR-008: simple,
//! dependency-free; at equal rates and speed 1 the step is exactly 1.0 and
//! samples are copied unchanged). The same step carries the deck's tempo
//! fader as plain varispeed (pitch follows tempo) unless key lock is on:
//! then the deck is read at the rate-conversion step only and a
//! [`TimeStretch`] engine (Signalsmith, ADR-008 D3 addendum) changes its
//! tempo while keeping its pitch. The published position is the *audible*
//! frame: the stretch latency is compensated (`read head - latency`), so
//! SYNC and the phase readout see what is heard.
//!
//! The callback renders in chunks of [`CHUNK`] frames into buffers
//! preallocated with the [`Renderer`]; key lock never allocates either
//! (`tests/rt_alloc.rs` counts Rust and C++ heap use around `render`).
//!
//! Output is a CPAL stream ([`Engine::start`]) when a device opens, else a
//! silent paced thread running the same renderer so decks stay usable; a
//! device that dies at runtime is replaced by the silent output with every
//! deck's state carried over ([`Engine::poll`]). Tests drive the renderer
//! by hand ([`Engine::manual`], the fake sink).

mod mixer;
mod output;
mod stretch;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub use mixer::{
    Assign, ChannelParams, Curve, EQ_KILL_DB, EQ_MAX_DB, FILTER_DEADZONE, LOW_MID_HZ, MID_HIGH_HZ,
    MixerParams, TRIM_DB, crossfader_gains, db_to_gain, eq_gain, trim_gain,
};
pub use output::OutputKind;
pub use stretch::{Signalsmith, TimeStretch};

/// Decks the engine mixes.
pub const DECKS: usize = 2;
/// Command ring capacity (UI → callback).
pub const COMMAND_CAPACITY: usize = 256;
/// Loops shorter than this many milliseconds are refused.
pub const MIN_LOOP_MS: f64 = 10.0;
/// No track loaded on a deck.
pub const NO_TRACK: u64 = u64::MAX;
/// Frames the callback renders per inner step (buffers are this size).
pub const CHUNK: usize = 256;

/// Key lock: tempo segments remembered across one stretch output latency.
const SEGMENTS: usize = 32;
/// Fastest key-locked speed (scratch input is sized for it).
pub const MAX_KEYLOCK_SPEED: f64 = 4.0;

/// Decoded interleaved stereo (`L R L R …`) PCM held in RAM.
#[derive(Debug)]
pub struct TrackPcm {
    sample_rate: u32,
    samples: Vec<f32>,
}

impl TrackPcm {
    /// `None` when `sample_rate` is 0, the sample count is odd or empty.
    pub fn new(sample_rate: u32, samples: Vec<f32>) -> Option<Self> {
        (sample_rate > 0 && !samples.is_empty() && samples.len().is_multiple_of(2)).then_some(
            Self {
                sample_rate,
                samples,
            },
        )
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Frames (samples per channel).
    pub fn frames(&self) -> u64 {
        (self.samples.len() / 2) as u64
    }

    /// Milliseconds per source frame.
    pub fn ms_per_frame(&self) -> f64 {
        1000.0 / f64::from(self.sample_rate)
    }

    /// `(left, right)` at source frame `i`; silence outside the track.
    fn frame(&self, i: u64) -> (f32, f32) {
        let i = usize::try_from(i).unwrap_or(usize::MAX).saturating_mul(2);
        match (self.samples.get(i), self.samples.get(i.saturating_add(1))) {
            (Some(l), Some(r)) => (*l, *r),
            _ => (0.0, 0.0),
        }
    }

    /// Linear interpolation at fractional source frame `pos`; silence
    /// before the start and after the end.
    fn at(&self, pos: f64) -> (f32, f32) {
        if pos < 0.0 {
            return (0.0, 0.0);
        }
        let i = pos as u64;
        let frac = (pos - i as f64) as f32;
        let (l0, r0) = self.frame(i);
        if frac == 0.0 {
            return (l0, r0);
        }
        let (l1, r1) = self.frame(i.saturating_add(1));
        (l0 + (l1 - l0) * frac, r0 + (r1 - r0) * frac)
    }
}

/// UI → callback message. Positions are source frames of the deck's track.
#[derive(Debug)]
pub enum Command {
    /// Replace the deck's track; resets position, cue and loops to 0/none.
    Load {
        deck: usize,
        id: u64,
        track: Arc<TrackPcm>,
    },
    /// Stop and empty the deck (a replacement is being decoded); gain and
    /// speed are kept.
    Unload {
        deck: usize,
    },
    TogglePlay {
        deck: usize,
    },
    /// CUE pressed: playing → stop and return to the cue point; paused off
    /// the cue point → set the cue point here; paused on it → preview
    /// while held.
    CueDown {
        deck: usize,
    },
    /// CUE released: a preview stops and returns to the cue point.
    CueUp {
        deck: usize,
    },
    /// Jump to `frame` (seek, scrub, hot cue); play state is kept.
    Seek {
        deck: usize,
        frame: f64,
    },
    LoopIn {
        deck: usize,
    },
    /// After IN: close the loop here. On an active loop: exit it.
    LoopOut {
        deck: usize,
    },
    /// Halve (0.5) or double (2.0) the active loop from its start.
    LoopScale {
        deck: usize,
        factor: f64,
    },
    /// Linear gain (1.0 = unity).
    Gain {
        deck: usize,
        gain: f32,
    },
    /// Playback speed multiplier (1.0 = original tempo).
    Speed {
        deck: usize,
        speed: f64,
    },
    /// Key lock (master tempo): speed changes tempo, not pitch.
    KeyLock {
        deck: usize,
        on: bool,
    },
    /// SYNC pressed: set `deck`'s speed and move it to the nearest position
    /// whose beat phase equals `master`'s at the moment the callback applies
    /// it. Grids are constant-tempo: beat length and one beat's position, in
    /// each deck's own source frames.
    Sync {
        deck: usize,
        master: usize,
        speed: f64,
        beat: f64,
        anchor: f64,
        master_beat: f64,
        master_anchor: f64,
    },
    /// D4 mixer controls for both channels; `None` = mixer hidden: both
    /// decks sum at unity with no EQ, filter or crossfader in the path
    /// (bit-identical to the engine without a mixer).
    Mixer(Option<MixerParams>),
}

impl Command {
    fn deck(&self) -> usize {
        match self {
            Self::Load { deck, .. }
            | Self::Unload { deck }
            | Self::TogglePlay { deck }
            | Self::CueDown { deck }
            | Self::CueUp { deck }
            | Self::Seek { deck, .. }
            | Self::LoopIn { deck }
            | Self::LoopOut { deck }
            | Self::LoopScale { deck, .. }
            | Self::Gain { deck, .. }
            | Self::Speed { deck, .. }
            | Self::KeyLock { deck, .. }
            | Self::Sync { deck, .. } => *deck,
            // Not a deck command (never counted per deck).
            Self::Mixer(_) => DECKS,
        }
    }
}

/// Callback-side state of one deck.
#[derive(Clone, Debug)]
pub(crate) struct DeckState {
    track: Option<Arc<TrackPcm>>,
    id: u64,
    pos: f64,
    playing: bool,
    previewing: bool,
    cue: f64,
    loop_in: Option<f64>,
    loop_range: Option<(f64, f64)>,
    gain: f32,
    speed: f64,
    keylock: bool,
    /// Key lock: source frame the stretch engine reads next; valid while
    /// `primed`.
    read: f64,
    /// Key lock: output frames rendered since priming.
    clock: f64,
    /// Key lock: playback-path distance (source frames, loop wraps
    /// unrolled) the read head has travelled since priming.
    path: f64,
    /// Key lock: playback-path distance heard so far (same origin).
    heard: f64,
    /// Key lock: tempo segments `(clock, path, source frames per output
    /// frame)` of the read head, newest last, so the audible position can
    /// be read off where the read head was one output latency ago.
    segments: [(f64, f64, f64); SEGMENTS],
    /// Number of valid entries in `segments` (at least 1 while primed).
    seg_len: usize,
    /// Key lock: the stretch engine holds history continuous with `read`.
    primed: bool,
    /// Key lock: fractional input frame carried to the next chunk.
    carry: f64,
    /// Commands applied to this deck (the UI compares with what it sent).
    applied: u64,
}

impl Default for DeckState {
    fn default() -> Self {
        Self {
            track: None,
            id: NO_TRACK,
            pos: 0.0,
            playing: false,
            previewing: false,
            cue: 0.0,
            loop_in: None,
            loop_range: None,
            gain: 1.0,
            speed: 1.0,
            keylock: false,
            read: 0.0,
            clock: 0.0,
            path: 0.0,
            heard: 0.0,
            segments: [(0.0, 0.0, 0.0); SEGMENTS],
            seg_len: 0,
            primed: false,
            carry: 0.0,
            applied: 0,
        }
    }
}

impl DeckState {
    fn frames(&self) -> f64 {
        self.track.as_ref().map_or(0.0, |t| t.frames() as f64)
    }

    fn min_loop(&self) -> f64 {
        self.track
            .as_ref()
            .map_or(1.0, |t| MIN_LOOP_MS / t.ms_per_frame())
    }

    fn apply(&mut self, command: Command) {
        // Every jump or start restarts the stretch history; tempo, gain and
        // loop edges at the current position do not.
        if !matches!(
            command,
            Command::Gain { .. }
                | Command::Speed { .. }
                | Command::LoopIn { .. }
                | Command::LoopOut { .. }
                | Command::KeyLock { .. }
        ) {
            self.primed = false;
        }
        match command {
            Command::Load { id, track, .. } => {
                let (gain, speed, keylock, applied) =
                    (self.gain, self.speed, self.keylock, self.applied);
                // Dropping the old Arc here only decrements: the UI holds a clone.
                *self = Self {
                    track: Some(track),
                    id,
                    gain,
                    speed,
                    keylock,
                    applied,
                    ..Self::default()
                };
            }
            Command::Unload { .. } => {
                let (gain, speed, keylock, applied) =
                    (self.gain, self.speed, self.keylock, self.applied);
                // As for Load: the UI's retired list still holds the track.
                *self = Self {
                    gain,
                    speed,
                    keylock,
                    applied,
                    ..Self::default()
                };
            }
            Command::KeyLock { on, .. } => {
                if on != self.keylock {
                    self.keylock = on;
                    self.primed = false;
                }
            }
            // Applied by the renderer (it needs the master deck / owns the mixer).
            Command::Sync { .. } | Command::Mixer(_) => {}
            Command::Gain { gain, .. } => {
                if gain.is_finite() {
                    self.gain = gain.max(0.0);
                }
            }
            Command::Speed { speed, .. } => {
                if speed.is_finite() && speed > 0.0 {
                    self.speed = speed;
                }
            }
            _ if self.track.is_none() => {}
            Command::TogglePlay { .. } => {
                if self.previewing {
                    // PLAY during a CUE preview keeps playing after release.
                    self.previewing = false;
                    self.playing = true;
                } else {
                    self.playing = !self.playing && self.pos < self.frames();
                }
            }
            Command::CueDown { .. } => {
                if self.playing {
                    self.playing = false;
                    self.previewing = false;
                    self.pos = self.cue;
                } else if (self.pos - self.cue).abs() < 0.5 {
                    self.playing = self.pos < self.frames();
                    self.previewing = self.playing;
                } else {
                    self.cue = self.pos;
                }
            }
            Command::CueUp { .. } => {
                if self.previewing {
                    self.previewing = false;
                    self.playing = false;
                    self.pos = self.cue;
                }
            }
            Command::Seek { frame, .. } => {
                if frame.is_finite() {
                    self.pos = frame.clamp(0.0, self.frames());
                    if self.pos >= self.frames() {
                        self.playing = false;
                        self.previewing = false;
                    }
                }
            }
            Command::LoopIn { .. } => self.loop_in = Some(self.pos),
            Command::LoopOut { .. } => {
                if self.loop_range.take().is_some() {
                    // Leaving a loop: the key-lock read head is inside the
                    // loop but playback continues past its end.
                    self.primed = false;
                } else if let Some(start) = self.loop_in
                    && self.pos - start >= self.min_loop()
                {
                    self.loop_range = Some((start, self.pos));
                }
            }
            Command::LoopScale { factor, .. } => {
                if let Some((start, end)) = self.loop_range
                    && factor.is_finite()
                    && factor > 0.0
                {
                    let len = ((end - start) * factor).max(self.min_loop());
                    let end = (start + len).min(self.frames());
                    if end > start {
                        self.loop_range = Some((start, end));
                        if self.pos >= end {
                            self.pos = start + (self.pos - start) % (end - start);
                        }
                    }
                }
            }
        }
    }

    /// Key-locked output for `out` (interleaved stereo, at most [`CHUNK`]
    /// frames): read `speed × frames` device-rate input frames at the plain
    /// rate-conversion step into `input`, stretch them to `out`, and
    /// publish the audible position (read head minus the stretch latency).
    fn render_keylock(
        &mut self,
        stretch: &mut dyn TimeStretch,
        input: &mut [f32],
        out: &mut [f32],
        device_rate: f64,
    ) {
        let Some(track) = self.track.as_deref().filter(|_| self.playing) else {
            out.fill(0.0);
            return;
        };
        let step = f64::from(track.sample_rate) / device_rate;
        let speed = self.speed.min(MAX_KEYLOCK_SPEED);
        let (in_lat, out_lat) = stretch.latency();
        let lag = (in_lat as f64 + out_lat as f64 * speed) * step;
        let capacity = input.len() / 2;
        let wrap = self.loop_wrap();
        if !self.primed {
            // Start the engine `lag` ahead (along the loop, if any) so what
            // is heard starts at `pos`.
            stretch.reset();
            self.read = wrap(self.pos + lag);
            self.clock = 0.0;
            self.path = lag;
            self.heard = 0.0;
            // Preroll fed the history at this tempo: extrapolates backwards.
            self.segments[0] = (0.0, lag, speed * step);
            self.seg_len = 1;
            let pre = stretch.preroll_frames().min(capacity);
            let start = self.pos + lag - pre as f64 * step;
            for (i, f) in input[..pre * 2]
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .enumerate()
            {
                let (l, r) = track.at(wrap(start + i as f64 * step));
                *f = [l, r];
            }
            stretch.preroll(&input[..pre * 2], speed);
            self.carry = 0.0;
            self.primed = true;
        }
        let rate = speed * step;
        if self.segments[self.seg_len - 1].2 != rate {
            // Tempo change: start a new read-head segment. When full (fader
            // moved every block), the newest segment is replaced.
            if self.seg_len == SEGMENTS {
                self.seg_len -= 1;
            }
            self.segments[self.seg_len] = (self.clock, self.path, rate);
            self.seg_len += 1;
        }
        let out_frames = (out.len() / 2) as f64;
        let want = out_frames * speed + self.carry;
        let n_in = (want as usize).min(capacity);
        self.carry = (want - n_in as f64).min(1.0);
        for f in input[..n_in * 2].as_chunks_mut::<2>().0 {
            let (l, r) = track.at(self.read);
            *f = [l, r];
            self.read = wrap(self.read + step);
        }
        self.path += n_in as f64 * step;
        self.clock += out_frames;
        stretch.process(&input[..n_in * 2], out);
        for v in out.iter_mut() {
            *v *= self.gain;
        }
        // What emerges now was synthesised one output latency ago from
        // input one input latency behind the read head of that moment, so
        // the ear follows the read head's own history (its tempo segments),
        // not the current tempo. Never backwards.
        let then = self.clock - out_lat as f64;
        let mut i = self.seg_len - 1;
        while i > 0 && self.segments[i].0 > then {
            i -= 1;
        }
        // Segments wholly before `then` are never needed again.
        if i > 0 {
            self.segments.copy_within(i..self.seg_len, 0);
            self.seg_len -= i;
        }
        let (t0, p0, r0) = self.segments[0];
        let heard = p0 + (then - t0) * r0 - in_lat as f64 * step;
        let advance = (heard - self.heard).max(0.0);
        self.heard += advance;
        self.pos = wrap(self.pos + advance).max(0.0);
        let frames = track.frames() as f64;
        if self.pos >= frames {
            self.pos = frames;
            self.playing = false;
            self.previewing = false;
            self.primed = false;
        }
    }

    /// Maps a playback-path frame into the active loop (any number of loop
    /// lengths past its end); identity without a loop.
    fn loop_wrap(&self) -> impl Fn(f64) -> f64 + use<> {
        let range = self.loop_range.filter(|(start, end)| end > start);
        move |x| match range {
            Some((start, end)) if x >= end => start + (x - end) % (end - start),
            _ => x,
        }
    }

    /// SYNC: take `speed` and jump to the nearest frame whose beat phase
    /// equals the master's (`master_phase`, 0..1).
    fn sync(&mut self, speed: f64, beat: f64, anchor: f64, master_phase: f64) {
        if !(speed.is_finite() && speed > 0.0 && beat.is_finite() && beat > 0.0) {
            return;
        }
        self.speed = speed;
        if self.track.is_none() || !(anchor.is_finite() && master_phase.is_finite()) {
            return;
        }
        let phase = ((self.pos - anchor) / beat).rem_euclid(1.0);
        let delta = (master_phase - phase + 0.5).rem_euclid(1.0) - 0.5;
        self.pos = (self.pos + delta * beat).clamp(0.0, self.frames());
        self.primed = false;
    }

    /// Next output frame at `device_rate`, advancing the position.
    fn next(&mut self, device_rate: f64) -> (f32, f32) {
        let Some(track) = self.track.as_deref().filter(|_| self.playing) else {
            return (0.0, 0.0);
        };
        let (l, r) = track.at(self.pos);
        self.pos += f64::from(track.sample_rate) / device_rate * self.speed;
        if let Some((start, end)) = self.loop_range
            && self.pos >= end
            && end > start
        {
            self.pos = start + (self.pos - end) % (end - start);
        }
        let frames = track.frames() as f64;
        if self.pos >= frames {
            self.pos = frames;
            self.playing = false;
            self.previewing = false;
        }
        (l * self.gain, r * self.gain)
    }
}

const PLAYING: u32 = 1;
const PREVIEWING: u32 = 2;
const KEYLOCK: u32 = 4;

/// Atomics one deck publishes after every block (`f64` stored as bits; NaN
/// = none). Fields are individually consistent; a reader may see one block
/// newer than another, which is harmless for display.
struct DeckShared {
    id: AtomicU64,
    pos: AtomicU64,
    cue: AtomicU64,
    loop_in: AtomicU64,
    loop_start: AtomicU64,
    loop_end: AtomicU64,
    flags: AtomicU32,
    applied: AtomicU64,
}

impl Default for DeckShared {
    fn default() -> Self {
        let nan = || AtomicU64::new(f64::NAN.to_bits());
        Self {
            id: AtomicU64::new(NO_TRACK),
            pos: AtomicU64::new(0),
            cue: AtomicU64::new(0),
            loop_in: nan(),
            loop_start: nan(),
            loop_end: nan(),
            flags: AtomicU32::new(0),
            applied: AtomicU64::new(0),
        }
    }
}

fn store(a: &AtomicU64, v: Option<f64>) {
    a.store(v.unwrap_or(f64::NAN).to_bits(), Ordering::Relaxed);
}

fn load(a: &AtomicU64) -> Option<f64> {
    let v = f64::from_bits(a.load(Ordering::Relaxed));
    (!v.is_nan()).then_some(v)
}

impl DeckShared {
    fn publish(&self, d: &DeckState) {
        self.id.store(d.id, Ordering::Relaxed);
        self.applied.store(d.applied, Ordering::Relaxed);
        store(&self.pos, Some(d.pos));
        store(&self.cue, Some(d.cue));
        store(&self.loop_in, d.loop_in);
        store(&self.loop_start, d.loop_range.map(|r| r.0));
        store(&self.loop_end, d.loop_range.map(|r| r.1));
        let flags = (u32::from(d.playing) * PLAYING)
            | (u32::from(d.previewing) * PREVIEWING)
            | (u32::from(d.keylock) * KEYLOCK);
        self.flags.store(flags, Ordering::Release);
    }

    fn snapshot(&self) -> DeckSnapshot {
        let flags = self.flags.load(Ordering::Acquire);
        let id = self.id.load(Ordering::Relaxed);
        DeckSnapshot {
            id: (id != NO_TRACK).then_some(id),
            frame: load(&self.pos).unwrap_or(0.0),
            cue: load(&self.cue).unwrap_or(0.0),
            loop_in: load(&self.loop_in),
            loop_range: load(&self.loop_start).zip(load(&self.loop_end)),
            playing: flags & PLAYING != 0,
            previewing: flags & PREVIEWING != 0,
            keylock: flags & KEYLOCK != 0,
            applied: self.applied.load(Ordering::Relaxed),
        }
    }
}

/// What the UI reads of one deck (positions in source frames).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeckSnapshot {
    /// Track id the callback is playing (`None` = empty deck).
    pub id: Option<u64>,
    pub frame: f64,
    pub cue: f64,
    pub loop_in: Option<f64>,
    pub loop_range: Option<(f64, f64)>,
    pub playing: bool,
    pub previewing: bool,
    /// Key lock is on (speed changes tempo, not pitch).
    pub keylock: bool,
    /// Commands the callback has applied to this deck so far.
    pub applied: u64,
}

/// Callback statistics (qualification evidence, ADR-008 addendum).
#[derive(Default)]
pub(crate) struct Stats {
    pub(crate) callbacks: AtomicU64,
    pub(crate) frames: AtomicU64,
    pub(crate) min_block: AtomicU32,
    pub(crate) max_block: AtomicU32,
    pub(crate) last_ns: AtomicU64,
    pub(crate) max_interval_ns: AtomicU64,
    pub(crate) xruns: AtomicU64,
    pub(crate) errors: AtomicU64,
    /// Output died (device lost / stream invalidated); UI falls back.
    pub(crate) dead: AtomicBool,
}

/// Snapshot of the callback statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsSnapshot {
    pub callbacks: u64,
    pub frames: u64,
    pub min_block: u32,
    pub max_block: u32,
    pub max_interval_ns: u64,
    pub xruns: u64,
    pub errors: u64,
}

/// Peak meters: channel A, channel B, master.
pub const METERS: usize = DECKS + 1;

pub(crate) struct Shared {
    decks: [DeckShared; DECKS],
    /// Largest |sample| since the UI last read it (`f32` bits; positive
    /// floats order like their bits, so `fetch_max` keeps the peak).
    peaks: [AtomicU32; METERS],
    /// A sample reached full scale since the UI last read it.
    clips: [AtomicBool; METERS],
    pub(crate) stats: Stats,
}

/// One level meter reading: the peak (linear, 1.0 = 0 dBFS) since the
/// previous read, and whether it clipped.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Meter {
    pub peak: f32,
    pub clip: bool,
}

/// The callback side: owns deck state and renders blocks.
pub struct Renderer {
    rx: rtrb::Consumer<Command>,
    decks: [DeckState; DECKS],
    shared: Arc<Shared>,
    device_rate: f64,
    /// One stretch engine per deck (key lock), built off the callback.
    stretch: [Box<dyn TimeStretch>; DECKS],
    /// Key-lock input scratch (interleaved stereo, preallocated).
    input: Vec<f32>,
    /// One deck's chunk and the mix of all decks (interleaved stereo).
    deck_buf: Vec<f32>,
    mix: Vec<f32>,
    /// D4 mixer (bypassed while its params are `None`).
    mixer: mixer::Mixer,
}

impl Renderer {
    /// Render `out` (interleaved, `channels` per frame) at the device rate:
    /// drain queued commands, mix both decks at their gains (unity = 1.0)
    /// and publish the deck snapshots. RT-safe: see the crate docs.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        while let Ok(command) = self.rx.pop() {
            if let Command::Mixer(params) = command {
                self.mixer.set(params);
                continue;
            }
            let deck = command.deck();
            if deck >= DECKS {
                continue;
            }
            if let Command::Sync {
                master,
                speed,
                beat,
                anchor,
                master_beat,
                master_anchor,
                ..
            } = command
            {
                let phase = self
                    .decks
                    .get(master)
                    .filter(|m| master != deck && m.track.is_some() && master_beat > 0.0)
                    .map(|m| ((m.pos - master_anchor) / master_beat).rem_euclid(1.0));
                if let Some(phase) = phase {
                    self.decks[deck].sync(speed, beat, anchor, phase);
                }
            }
            let state = &mut self.decks[deck];
            state.applied = state.applied.wrapping_add(1);
            state.apply(command);
        }
        let channels = channels.max(1);
        let rate = self.device_rate;
        let Self {
            decks,
            stretch,
            input,
            deck_buf,
            mix,
            mixer,
            shared,
            ..
        } = self;
        let mixing = mixer.params.is_some();
        for block in out.chunks_mut(CHUNK * channels) {
            let frames = block.len().div_ceil(channels);
            let mix = &mut mix[..frames * 2];
            mix.fill(0.0);
            for (c, (deck, stretch)) in decks.iter_mut().zip(stretch.iter_mut()).enumerate() {
                let buf = &mut deck_buf[..frames * 2];
                if deck.keylock && deck.track.is_some() {
                    deck.render_keylock(stretch.as_mut(), input, buf, rate);
                } else {
                    deck.primed = false;
                    for f in buf.as_chunks_mut::<2>().0 {
                        let (l, r) = deck.next(rate);
                        *f = [l, r];
                    }
                }
                if mixing {
                    let peak = mixer.process_channel(c, buf);
                    meter(shared, c, peak);
                }
                for (m, v) in mix.iter_mut().zip(buf.iter()) {
                    *m += v;
                }
            }
            if mixing {
                let peak = mixer.process_master(mix);
                meter(shared, DECKS, peak);
            }
            for (frame, [l, r]) in block.chunks_mut(channels).zip(mix.as_chunks::<2>().0) {
                let (l, r) = (l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0));
                match frame {
                    [mono] => *mono = (l + r) * 0.5,
                    [left, right, rest @ ..] => {
                        *left = l;
                        *right = r;
                        rest.fill(0.0);
                    }
                    [] => {}
                }
            }
        }
        for (shared, deck) in self.shared.decks.iter().zip(&self.decks) {
            shared.publish(deck);
        }
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate as u32
    }
}

/// Publish meter `m`'s chunk peak (lock-free; RT-safe).
fn meter(shared: &Shared, m: usize, peak: f32) {
    if let (Some(p), Some(c)) = (shared.peaks.get(m), shared.clips.get(m)) {
        p.fetch_max(peak.max(0.0).to_bits(), Ordering::Relaxed);
        if peak >= 1.0 {
            c.store(true, Ordering::Relaxed);
        }
    }
}

fn channel(
    decks: [DeckState; DECKS],
    device_rate: u32,
    mixer: Option<MixerParams>,
) -> (Renderer, rtrb::Producer<Command>, Arc<Shared>) {
    let (tx, rx) = rtrb::RingBuffer::new(COMMAND_CAPACITY);
    let shared = Arc::new(Shared {
        decks: Default::default(),
        peaks: Default::default(),
        clips: Default::default(),
        stats: Stats {
            min_block: AtomicU32::new(u32::MAX),
            ..Stats::default()
        },
    });
    for (s, d) in shared.decks.iter().zip(&decks) {
        s.publish(d);
    }
    let stretch: [Box<dyn TimeStretch>; DECKS] =
        std::array::from_fn(|_| Box::new(Signalsmith::new(device_rate)) as Box<dyn TimeStretch>);
    let input_frames = stretch
        .iter()
        .map(|s| s.preroll_frames())
        .max()
        .unwrap_or(0)
        .max(CHUNK * MAX_KEYLOCK_SPEED as usize + 1);
    let renderer = Renderer {
        rx,
        decks,
        shared: shared.clone(),
        device_rate: f64::from(device_rate.max(1)),
        stretch,
        input: vec![0.0; input_frames * 2],
        deck_buf: vec![0.0; CHUNK * 2],
        mix: vec![0.0; CHUNK * 2],
        mixer: mixer::Mixer::new(device_rate, mixer),
    };
    (renderer, tx, shared)
}

/// Engine state the UI keeps so it can rebuild the output (fallback).
#[derive(Clone, Default)]
struct Memory {
    /// The track each deck was last told to load.
    current: [Option<(u64, Arc<TrackPcm>)>; DECKS],
    gains: [Option<f32>; DECKS],
    speeds: [Option<f64>; DECKS],
    keylocks: [bool; DECKS],
    mixer: Option<MixerParams>,
}

/// The UI side of the engine.
pub struct Engine {
    tx: rtrb::Producer<Command>,
    shared: Arc<Shared>,
    output: output::Output,
    device_rate: u32,
    memory: Memory,
    /// Commands pushed per deck (compare [`DeckSnapshot::applied`]).
    sent: [u64; DECKS],
    /// Tracks the callback may still reference; dropped once only we hold them.
    retired: Vec<Arc<TrackPcm>>,
    /// Why no device is playing (shown to the user).
    error: Option<String>,
}

impl Engine {
    /// Open the default output device; fall back to the silent output (with
    /// the reason in [`Engine::error`]) when none opens.
    pub fn start() -> Self {
        Self::launch(Default::default(), Memory::default(), None)
    }

    /// Silent paced output only (no device is touched).
    pub fn silent(reason: &str) -> Self {
        Self::launch(
            Default::default(),
            Memory::default(),
            Some(reason.to_owned()),
        )
    }

    /// Fake sink for tests: the caller renders blocks by hand.
    pub fn manual(device_rate: u32) -> (Self, Renderer) {
        let (renderer, tx, shared) = channel(Default::default(), device_rate, None);
        let engine = Self {
            tx,
            shared,
            output: output::Output::Manual,
            device_rate,
            memory: Memory::default(),
            sent: [0; DECKS],
            retired: Vec::new(),
            error: None,
        };
        (engine, renderer)
    }

    fn launch(decks: [DeckState; DECKS], memory: Memory, silent: Option<String>) -> Self {
        let opened = match silent {
            Some(reason) => Err(reason),
            None => output::Device::open(),
        };
        // Positions are source frames, so the device rate only sets the pace
        // and a fallback renderer continues from the same deck state.
        let attempt = opened.and_then(|device| {
            let rate = device.rate;
            let (renderer, tx, shared) = channel(decks.clone(), rate, memory.mixer);
            device
                .play(renderer, shared.clone())
                .map(|output| (output, tx, shared, rate))
        });
        let (output, tx, shared, device_rate, error) = match attempt {
            Ok((output, tx, shared, rate)) => (output, tx, shared, rate, None),
            Err(reason) => {
                let (renderer, tx, shared) = channel(decks, output::SILENT_RATE, memory.mixer);
                (
                    output::Output::silent(renderer),
                    tx,
                    shared,
                    output::SILENT_RATE,
                    Some(reason),
                )
            }
        };
        Self {
            tx,
            shared,
            output,
            device_rate,
            memory,
            sent: [0; DECKS],
            retired: Vec::new(),
            error,
        }
    }

    /// Queue `command`; `false` when the ring is full (command dropped).
    pub fn send(&mut self, command: Command) -> bool {
        if let Command::Mixer(params) = &command {
            self.memory.mixer = *params;
        }
        let deck = command.deck();
        if deck < DECKS {
            match &command {
                Command::Load { id, track, .. } => {
                    if let Some((_, old)) = self.memory.current[deck].replace((*id, track.clone()))
                    {
                        self.retired.push(old);
                    }
                }
                Command::Unload { .. } => {
                    if let Some((_, old)) = self.memory.current[deck].take() {
                        self.retired.push(old);
                    }
                }
                Command::Gain { gain, .. } => self.memory.gains[deck] = Some(*gain),
                Command::Speed { speed, .. } => self.memory.speeds[deck] = Some(*speed),
                Command::Sync { speed, .. } => self.memory.speeds[deck] = Some(*speed),
                Command::KeyLock { on, .. } => self.memory.keylocks[deck] = *on,
                _ => {}
            }
        }
        let pushed = self.tx.push(command).is_ok();
        if pushed && deck < DECKS {
            self.sent[deck] = self.sent[deck].wrapping_add(1);
        }
        pushed
    }

    /// Whether `deck` has commands the callback has not applied yet (the UI
    /// keeps its optimistic view, e.g. a scrub position, until then).
    pub fn pending(&self, deck: usize) -> bool {
        deck < DECKS && self.deck(deck).applied != self.sent[deck]
    }

    /// What the callback last published for `deck`.
    pub fn deck(&self, deck: usize) -> DeckSnapshot {
        self.shared
            .decks
            .get(deck)
            .map(DeckShared::snapshot)
            .unwrap_or_default()
    }

    /// Meters A, B and master since the previous call (each read resets
    /// the peak and the clip flag). All zero while the mixer is hidden.
    pub fn meters(&self) -> [Meter; METERS] {
        std::array::from_fn(|m| Meter {
            peak: f32::from_bits(self.shared.peaks[m].swap(0, Ordering::Relaxed)),
            clip: self.shared.clips[m].swap(false, Ordering::Relaxed),
        })
    }

    pub fn stats(&self) -> StatsSnapshot {
        let s = &self.shared.stats;
        let min = s.min_block.load(Ordering::Relaxed);
        StatsSnapshot {
            callbacks: s.callbacks.load(Ordering::Relaxed),
            frames: s.frames.load(Ordering::Relaxed),
            min_block: if min == u32::MAX { 0 } else { min },
            max_block: s.max_block.load(Ordering::Relaxed),
            max_interval_ns: s.max_interval_ns.load(Ordering::Relaxed),
            xruns: s.xruns.load(Ordering::Relaxed),
            errors: s.errors.load(Ordering::Relaxed),
        }
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    /// Where the audio goes.
    pub fn output(&self) -> OutputKind {
        self.output.kind()
    }

    /// Why no device is playing (no device, open failure, device lost).
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Mark the output dead, as a lost device does (tests, fallback drill).
    pub fn simulate_device_loss(&self) {
        self.shared.stats.dead.store(true, Ordering::Release);
    }

    /// Housekeeping, once per UI frame: free tracks the callback released
    /// and, when the device died, continue silently with every deck's state
    /// (track, position, cue, loop, play state, gain, speed) carried over.
    pub fn poll(&mut self) {
        self.retired.retain(|t| Arc::strong_count(t) > 1);
        if !self.shared.stats.dead.load(Ordering::Acquire) {
            return;
        }
        let memory = self.memory.clone();
        let decks: [DeckState; DECKS] = std::array::from_fn(|d| {
            let snap = self.deck(d);
            let track = memory.current[d]
                .as_ref()
                .filter(|(id, _)| Some(*id) == snap.id)
                .map(|(_, t)| t.clone());
            DeckState {
                id: snap.id.filter(|_| track.is_some()).unwrap_or(NO_TRACK),
                playing: snap.playing && !snap.previewing && track.is_some(),
                track,
                pos: snap.frame,
                previewing: false,
                cue: snap.cue,
                loop_in: snap.loop_in,
                loop_range: snap.loop_range,
                gain: memory.gains[d].unwrap_or(1.0),
                speed: memory.speeds[d].unwrap_or(1.0),
                keylock: memory.keylocks[d],
                read: 0.0,
                clock: 0.0,
                path: 0.0,
                heard: 0.0,
                segments: [(0.0, 0.0, 0.0); SEGMENTS],
                seg_len: 0,
                primed: false,
                carry: 0.0,
                applied: 0,
            }
        });
        let reason = format!("audio device lost ({}); continuing silently", self.output());
        let retired = std::mem::take(&mut self.retired);
        let mut next = Self::launch(decks, memory, Some(reason));
        next.retired = retired;
        // The old output (stream or thread) stops when `old` drops; its
        // renderer's tracks are still held by `next.memory`.
        let old = std::mem::replace(self, next);
        drop(old);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod keylock_tests;
