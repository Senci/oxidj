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
//! fader as plain varispeed (pitch follows tempo; key lock is D3).
//!
//! Output is a CPAL stream ([`Engine::start`]) when a device opens, else a
//! silent paced thread running the same renderer so decks stay usable; a
//! device that dies at runtime is replaced by the silent output with every
//! deck's state carried over ([`Engine::poll`]). Tests drive the renderer
//! by hand ([`Engine::manual`], the fake sink).

mod output;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub use output::OutputKind;

/// Decks the engine mixes.
pub const DECKS: usize = 2;
/// Command ring capacity (UI → callback).
pub const COMMAND_CAPACITY: usize = 256;
/// Loops shorter than this many milliseconds are refused.
pub const MIN_LOOP_MS: f64 = 10.0;
/// No track loaded on a deck.
pub const NO_TRACK: u64 = u64::MAX;

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

    /// Linear interpolation at fractional source frame `pos` (`pos >= 0`).
    fn at(&self, pos: f64) -> (f32, f32) {
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
    /// Playback speed multiplier (1.0 = original tempo; varispeed).
    Speed {
        deck: usize,
        speed: f64,
    },
}

impl Command {
    fn deck(&self) -> usize {
        match self {
            Self::Load { deck, .. }
            | Self::TogglePlay { deck }
            | Self::CueDown { deck }
            | Self::CueUp { deck }
            | Self::Seek { deck, .. }
            | Self::LoopIn { deck }
            | Self::LoopOut { deck }
            | Self::LoopScale { deck, .. }
            | Self::Gain { deck, .. }
            | Self::Speed { deck, .. } => *deck,
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
        match command {
            Command::Load { id, track, .. } => {
                let (gain, speed, applied) = (self.gain, self.speed, self.applied);
                // Dropping the old Arc here only decrements: the UI holds a clone.
                *self = Self {
                    track: Some(track),
                    id,
                    gain,
                    speed,
                    applied,
                    ..Self::default()
                };
            }
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
                if self.loop_range.take().is_none()
                    && let Some(start) = self.loop_in
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
        let flags = (u32::from(d.playing) * PLAYING) | (u32::from(d.previewing) * PREVIEWING);
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

pub(crate) struct Shared {
    decks: [DeckShared; DECKS],
    pub(crate) stats: Stats,
}

/// The callback side: owns deck state and renders blocks.
pub struct Renderer {
    rx: rtrb::Consumer<Command>,
    decks: [DeckState; DECKS],
    shared: Arc<Shared>,
    device_rate: f64,
}

impl Renderer {
    /// Render `out` (interleaved, `channels` per frame) at the device rate:
    /// drain queued commands, mix both decks at their gains (unity = 1.0)
    /// and publish the deck snapshots. RT-safe: see the crate docs.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        while let Ok(command) = self.rx.pop() {
            if let Some(state) = self.decks.get_mut(command.deck()) {
                state.applied = state.applied.wrapping_add(1);
                state.apply(command);
            }
        }
        let channels = channels.max(1);
        for frame in out.chunks_mut(channels) {
            let (mut l, mut r) = (0.0, 0.0);
            for deck in &mut self.decks {
                let (dl, dr) = deck.next(self.device_rate);
                l += dl;
                r += dr;
            }
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
        for (shared, deck) in self.shared.decks.iter().zip(&self.decks) {
            shared.publish(deck);
        }
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate as u32
    }
}

fn channel(
    decks: [DeckState; DECKS],
    device_rate: u32,
) -> (Renderer, rtrb::Producer<Command>, Arc<Shared>) {
    let (tx, rx) = rtrb::RingBuffer::new(COMMAND_CAPACITY);
    let shared = Arc::new(Shared {
        decks: Default::default(),
        stats: Stats {
            min_block: AtomicU32::new(u32::MAX),
            ..Stats::default()
        },
    });
    for (s, d) in shared.decks.iter().zip(&decks) {
        s.publish(d);
    }
    let renderer = Renderer {
        rx,
        decks,
        shared: shared.clone(),
        device_rate: f64::from(device_rate.max(1)),
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
        let (renderer, tx, shared) = channel(Default::default(), device_rate);
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
            let (renderer, tx, shared) = channel(decks.clone(), rate);
            device
                .play(renderer, shared.clone())
                .map(|output| (output, tx, shared, rate))
        });
        let (output, tx, shared, device_rate, error) = match attempt {
            Ok((output, tx, shared, rate)) => (output, tx, shared, rate, None),
            Err(reason) => {
                let (renderer, tx, shared) = channel(decks, output::SILENT_RATE);
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
        let deck = command.deck();
        if deck < DECKS {
            match &command {
                Command::Load { id, track, .. } => {
                    if let Some((_, old)) = self.memory.current[deck].replace((*id, track.clone()))
                    {
                        self.retired.push(old);
                    }
                }
                Command::Gain { gain, .. } => self.memory.gains[deck] = Some(*gain),
                Command::Speed { speed, .. } => self.memory.speeds[deck] = Some(*speed),
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
