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
//! reads at a fractional step through a windowed-sinc interpolator
//! (`resample.rs`: in-crate, dependency-free, band-limited so faster reads
//! do not alias; at equal rates and speed 1 the step is exactly 1.0 and
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
mod resample;
mod stretch;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub use mixer::{
    Assign, ChannelParams, Curve, EQ_KILL_DB, EQ_MAX_DB, FILTER_DEADZONE, LOW_MID_HZ, MID_HIGH_HZ,
    MixerParams, TRIM_DB, crossfader_gains, db_to_gain, eq_gain, trim_gain,
};
pub use output::{OutputConfig, OutputInfo, OutputKind, Routing, list_outputs};
use resample::Resampler;
pub use stretch::{Signalsmith, TimeStretch};

/// Decks the engine mixes.
pub const DECKS: usize = 2;
/// Command ring capacity (UI → callback).
pub const COMMAND_CAPACITY: usize = 256;
/// Fade applied by real outputs when a deck starts, stops or jumps (play,
/// pause, cue, seek, sync, loop seam, load): long enough to remove the
/// click, short enough to keep the attack of a cued beat.
pub const DECLICK_MS: f64 = 3.0;
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

    /// Band-limited read at fractional source frame `pos` through kernel
    /// set `table` ([`Resampler::table`]); silence before the start and
    /// after the end.
    fn at(&self, pos: f64, resampler: &Resampler, table: usize) -> (f32, f32) {
        resampler.at(&self.samples, pos, table)
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
    /// Master peak limiter on the main output (on by default): overs are
    /// turned down smoothly instead of being clipped. Off = the bare clamp
    /// at full scale.
    Limiter(bool),
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
            Self::Mixer(_) | Self::Limiter(_) => DECKS,
        }
    }
}

/// What a deck was playing just before a stop or jump, faded out over the
/// declick time so the old signal ends in a ramp, not a step.
#[derive(Clone, Debug)]
struct Tail {
    track: Arc<TrackPcm>,
    pos: f64,
    speed: f64,
    /// Output frames still to play.
    left: u32,
}

/// What a key-locked deck was playing just before a stop or jump: the read
/// head of its stretch engine. The renderer moves that engine aside (each
/// deck has a spare) and keeps feeding it for the declick time, so the old
/// stretched signal itself fades out while the other engine starts the new
/// one.
#[derive(Clone, Debug)]
struct KeyTail {
    track: Arc<TrackPcm>,
    read: f64,
    carry: f64,
    speed: f64,
    range: Option<(f64, f64)>,
    /// Output frames still to play.
    left: u32,
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
    /// Key lock: output frames the engine ran before the play point when
    /// it was primed (its burst phase is `clock + stagger`).
    stagger: usize,
    /// Commands applied to this deck (the UI compares with what it sent).
    applied: u64,
    /// Declick length in output frames (0 = off: starts, stops and jumps
    /// are sample-exact steps, as the fake-sink tests expect).
    declick: u32,
    /// Output frames of fade-in still to go after a start or jump.
    fade_in: u32,
    /// The pre-jump signal fading out.
    tail: Option<Tail>,
    /// The pre-jump key-locked signal fading out (on the spare engine).
    key_tail: Option<KeyTail>,
    /// Ask the renderer to exchange this deck's stretch engine and spare.
    swap: bool,
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
            stagger: 0,
            applied: 0,
            declick: 0,
            fade_in: 0,
            tail: None,
            key_tail: None,
            swap: false,
        }
    }
}

impl DeckState {
    /// Where the deck is audibly playing (`None` = silent).
    fn sounding(&self) -> Option<f64> {
        (self.playing && self.track.is_some()).then_some(self.pos)
    }

    /// After a command: when it stopped or moved a sounding deck, keep the
    /// old signal as a fading tail (`old` = the track it was playing, when
    /// the command replaced it); when the deck now sounds from a new place,
    /// fade it in. Clones and drops of `Arc<TrackPcm>` here are counter
    /// updates only: the UI holds every track until the callback lets go.
    ///
    /// `raw`: what sounded before was the plain track read. A key-locked
    /// deck sounded through the stretch engine, which a plain read does not
    /// match sample for sample, so it gets no tail (its old signal stops;
    /// only the new one fades in). `switched`: the signal path changed at
    /// the same position (key lock toggled), which is a jump for the ear.
    fn declick_after(
        &mut self,
        before: Option<f64>,
        old: Option<Arc<TrackPcm>>,
        raw: bool,
        key: Option<KeyTail>,
        switched: bool,
    ) {
        if self.declick == 0 {
            return;
        }
        let now = self.sounding();
        if now == before && old.is_none() && !switched {
            return;
        }
        // A jump while the previous fade is still running: one tail only,
        // so keep whichever signal is louder right now. Early in the fade
        // that is the running tail (the new signal has barely started and
        // is dropped: two jumps in one callback splice nothing); later it
        // is the faded-in signal, which becomes the tail at its own gain.
        let early = self.tail.is_some() && self.fade_in > self.declick / 2;
        if let Some(pos) = before.filter(|_| raw && !early) {
            let left = (self.declick + 1)
                .saturating_sub(self.fade_in)
                .min(self.declick);
            self.tail = old.or_else(|| self.track.clone()).map(|track| Tail {
                track,
                pos,
                speed: self.speed,
                left: left.max(1),
            });
        } else if !early {
            self.tail = None;
        }
        // The stretched signal: one tail too; a running one is kept (the
        // signal started since has had at most a few frames of fade-in).
        if let Some(key) = key
            && self.key_tail.is_none()
        {
            self.key_tail = Some(KeyTail {
                left: self.declick,
                ..key
            });
            self.swap = true;
        }
        self.fade_in = if now.is_some() { self.declick } else { 0 };
    }

    /// The stretch read head to continue as a tail, when the deck is
    /// audibly playing through its stretch engine right now.
    fn key_source(&self) -> Option<KeyTail> {
        let track = self.track.clone()?;
        (self.declick > 0 && self.keylock && self.primed && self.playing).then(|| KeyTail {
            track,
            read: self.read,
            carry: self.carry,
            speed: self.speed.min(MAX_KEYLOCK_SPEED),
            range: self.loop_range.filter(|(start, end)| end > start),
            left: 0,
        })
    }

    /// Add the fading stretched tail to `out` (interleaved stereo, at most
    /// [`CHUNK`] frames): keep feeding `spare` from the old read head.
    fn render_key_tail(
        &mut self,
        spare: &mut dyn TimeStretch,
        resampler: &Resampler,
        input: &mut [f32],
        scratch: &mut [f32],
        out: &mut [f32],
        device_rate: f64,
    ) {
        let Some(mut tail) = self.key_tail.take() else {
            return;
        };
        let frames = (tail.left as usize)
            .min(out.len() / 2)
            .min(scratch.len() / 2);
        let step = f64::from(tail.track.sample_rate) / device_rate;
        let table = Resampler::table(step);
        let want = frames as f64 * tail.speed + tail.carry;
        let n_in = (want as usize).min(input.len() / 2);
        tail.carry = (want - n_in as f64).min(1.0);
        let wrap = |x: f64| match tail.range {
            Some((start, end)) if x >= end => start + (x - end) % (end - start),
            _ => x,
        };
        let (Some(input), Some(scratch)) =
            (input.get_mut(..n_in * 2), scratch.get_mut(..frames * 2))
        else {
            return;
        };
        for f in input.as_chunks_mut::<2>().0 {
            let (l, r) = tail.track.at(tail.read, resampler, table);
            *f = [l, r];
            tail.read = wrap(tail.read + step);
        }
        spare.process(input, scratch);
        for (o, s) in out
            .as_chunks_mut::<2>()
            .0
            .iter_mut()
            .zip(scratch.as_chunks::<2>().0)
        {
            let g = tail.left as f32 / (self.declick as f32 + 1.0) * self.gain;
            o[0] += s[0] * g;
            o[1] += s[1] * g;
            tail.left = tail.left.saturating_sub(1);
        }
        if tail.left > 0 && frames > 0 {
            self.key_tail = Some(tail);
        }
    }

    /// Gain of the next fade-in frame (1.0 when no fade is running).
    fn fade_gain(&mut self) -> f32 {
        if self.fade_in == 0 {
            return 1.0;
        }
        let g = 1.0 - self.fade_in as f32 / (self.declick as f32 + 1.0);
        self.fade_in -= 1;
        g
    }

    /// Next frame of the fading tail (silence when there is none), at the
    /// complementary gain of [`DeckState::fade_gain`].
    fn tail_frame(&mut self, device_rate: f64, resampler: &Resampler) -> (f32, f32) {
        let Some(tail) = self.tail.as_mut() else {
            return (0.0, 0.0);
        };
        let g = tail.left as f32 / (self.declick as f32 + 1.0) * self.gain;
        let step = f64::from(tail.track.sample_rate) / device_rate * tail.speed;
        let (l, r) = tail.track.at(tail.pos, resampler, Resampler::table(step));
        tail.pos += step;
        tail.left = tail.left.saturating_sub(1);
        if tail.left == 0 {
            self.tail = None;
        }
        (l * g, r * g)
    }

    fn frames(&self) -> f64 {
        self.track.as_ref().map_or(0.0, |t| t.frames() as f64)
    }

    fn min_loop(&self) -> f64 {
        self.track
            .as_ref()
            .map_or(1.0, |t| MIN_LOOP_MS / t.ms_per_frame())
    }

    fn apply(&mut self, command: Command) {
        let (declick, before) = (self.declick, self.sounding());
        let old = match command {
            Command::Load { .. } | Command::Unload { .. } if before.is_some() => self.track.clone(),
            _ => None,
        };
        let switched = matches!(command, Command::KeyLock { on, .. } if on != self.keylock);
        let (raw, key) = (!self.keylock, self.key_source());
        self.apply_command(command);
        self.declick = declick;
        self.declick_after(before, old, raw, key, switched);
    }

    fn apply_command(&mut self, command: Command) {
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
                let key_tail = self.key_tail.take();
                let swap = self.swap;
                // Dropping the old Arc here only decrements: the UI holds a clone.
                *self = Self {
                    track: Some(track),
                    id,
                    gain,
                    speed,
                    keylock,
                    applied,
                    key_tail,
                    swap,
                    ..Self::default()
                };
            }
            Command::Unload { .. } => {
                let (gain, speed, keylock, applied) =
                    (self.gain, self.speed, self.keylock, self.applied);
                let key_tail = self.key_tail.take();
                let swap = self.swap;
                // As for Load: the UI's retired list still holds the track.
                *self = Self {
                    gain,
                    speed,
                    keylock,
                    applied,
                    key_tail,
                    swap,
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
            Command::Sync { .. } | Command::Mixer(_) | Command::Limiter(_) => {}
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
        resampler: &Resampler,
        input: &mut [f32],
        out: &mut [f32],
        device_rate: f64,
        stagger: usize,
    ) {
        let Some(track) = self.track.clone().filter(|_| self.playing) else {
            for f in out.as_chunks_mut::<2>().0 {
                let (l, r) = self.tail_frame(device_rate, resampler);
                *f = [l, r];
            }
            return;
        };
        let step = f64::from(track.sample_rate) / device_rate;
        let table = Resampler::table(step);
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
            // Stagger: run the engine `stagger` output frames before the
            // play point and throw them away. Its transform bursts then
            // fall half an interval away from the other deck's instead of
            // into the same callbacks. The read head starts that much
            // earlier, so it is at `pos + lag` again when real output
            // begins and nothing about the position changes.
            let skip_in = (stagger as f64 * speed) as usize;
            let head = self.pos + lag - skip_in as f64 * step;
            let pre = stretch.preroll_frames().min(capacity);
            let start = head - pre as f64 * step;
            for (i, f) in input[..pre * 2]
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .enumerate()
            {
                let (l, r) = track.at(wrap(start + i as f64 * step), resampler, table);
                *f = [l, r];
            }
            stretch.preroll(&input[..pre * 2], speed);
            let (mut left_out, mut left_in, mut at) = (stagger, skip_in, head);
            while left_out > 0 && !out.is_empty() {
                let frames = left_out.min(out.len() / 2).max(1);
                let n = if frames == left_out {
                    left_in
                } else {
                    (frames as f64 * speed) as usize
                }
                .min(left_in)
                .min(capacity);
                for f in input[..n * 2].as_chunks_mut::<2>().0 {
                    let (l, r) = track.at(wrap(at), resampler, table);
                    *f = [l, r];
                    at += step;
                }
                stretch.process(&input[..n * 2], &mut out[..frames * 2]);
                left_out -= frames;
                left_in -= n;
            }
            self.stagger = stagger;
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
            let (l, r) = track.at(self.read, resampler, table);
            *f = [l, r];
            self.read = wrap(self.read + step);
        }
        self.path += n_in as f64 * step;
        self.clock += out_frames;
        stretch.process(&input[..n_in * 2], out);
        for f in out.as_chunks_mut::<2>().0 {
            let g = self.gain * self.fade_gain();
            let (l, r) = self.tail_frame(device_rate, resampler);
            *f = [f[0] * g + l, f[1] * g + r];
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
    fn next(&mut self, device_rate: f64, resampler: &Resampler) -> (f32, f32) {
        let (tl, tr) = self.tail_frame(device_rate, resampler);
        let Some(track) = self.track.as_deref().filter(|_| self.playing) else {
            return (tl, tr);
        };
        let step = f64::from(track.sample_rate) / device_rate * self.speed;
        let (l, r) = track.at(self.pos, resampler, Resampler::table(step));
        let frames = track.frames() as f64;
        let before = self.pos;
        self.pos += step;
        let mut wrapped = false;
        if let Some((start, end)) = self.loop_range
            && self.pos >= end
            && end > start
        {
            self.pos = start + (self.pos - end) % (end - start);
            wrapped = true;
        }
        if self.pos >= frames {
            self.pos = frames;
            self.playing = false;
            self.previewing = false;
        }
        let g = self.gain * self.fade_gain();
        if wrapped && self.declick > 0 {
            // Crossfade the loop seam: what followed the loop end fades out
            // under the loop start fading in.
            self.declick_after(Some(before + step), None, true, None, false);
        }
        (l * g + tl, r * g + tr)
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
    /// Frames of the latest block.
    pub(crate) block: AtomicU32,
    /// Longest time one callback spent rendering.
    pub(crate) max_render_ns: AtomicU64,
    /// Total time spent rendering (mean load = this / audio time).
    pub(crate) render_ns: AtomicU64,
    /// Largest render time as a share of its block's duration (1000 = all).
    pub(crate) max_load_permille: AtomicU32,
    /// How far ahead of the DAC the latest block was written.
    pub(crate) latency_ns: AtomicU64,
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
    /// Frames of the latest callback block (the real buffer size).
    pub block: u32,
    /// Longest time one callback spent rendering.
    pub max_render_ns: u64,
    /// Total time spent rendering since the stream opened; divided by the
    /// audio time (`frames` / device rate) it is the mean DSP load.
    pub render_ns: u64,
    /// Worst DSP load: render time per 1000 of the block's own duration.
    /// Near 1000 means dropouts; aim below 300.
    pub max_load_permille: u32,
    /// Output latency the device reported for the latest block (time from
    /// the callback to the DAC); 0 when the host does not report it.
    pub latency_ns: u64,
}

/// Peak meters: channel A, channel B, master, cue.
pub const METERS: usize = DECKS + 2;
pub const CUE_METER: usize = DECKS + 1;

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
    /// A second engine per deck: it carries the fading tail of a key-locked
    /// deck across a stop or jump, then waits to be the next main engine.
    spare: [Box<dyn TimeStretch>; DECKS],
    /// Output of the spare engine for one chunk (interleaved stereo).
    tail_buf: Vec<f32>,
    /// Interpolation kernels (built off the callback).
    resampler: Resampler,
    /// Key-lock input scratch (interleaved stereo, preallocated).
    input: Vec<f32>,
    /// One deck's chunk and the mix of all decks (interleaved stereo).
    deck_buf: Vec<f32>,
    /// Pre-fader buffer for cue bus (interleaved stereo).
    pre_fader: Vec<f32>,
    mix: Vec<f32>,
    /// Cue bus output buffer (interleaved stereo).
    cue_buf: Vec<f32>,
    /// D4 mixer (bypassed while its params are `None`).
    mixer: mixer::Mixer,
    /// Master peak limiter on the main output.
    limiter: Limiter,
    /// Which device channels carry main and cue.
    routing: Routing,
}

/// Seconds for the limiter to recover about two thirds of its reduction.
const LIMITER_RELEASE_S: f64 = 0.15;

/// Look-ahead of the limiter on real outputs: the main output is delayed
/// by this much so the gain can come down before a peak instead of on it.
pub const LIMITER_LOOKAHEAD_MS: f64 = 1.0;

/// Main-output peak limiter: sustained overs (two hot tracks summed) are
/// turned down instead of being clipped on every wave. Below full scale
/// the gain is exactly 1.0 and the signal passes bit for bit.
///
/// Without look-ahead (hand-driven renderer) the gain drops on the peak
/// itself and nothing is delayed. With look-ahead the signal runs through
/// a delay line of that length and the gain ramps down ahead of a peak,
/// reaching the needed value on the frame the peak leaves the delay; it is
/// held while overs keep arriving and recovers slowly afterwards.
#[derive(Clone, Debug)]
struct Limiter {
    on: bool,
    gain: f32,
    release: f32,
    /// Delay line (interleaved stereo); empty = no look-ahead.
    delay: Vec<f32>,
    /// Next frame of `delay` to replace.
    at: usize,
    /// Lowest gain needed by a peak still in the delay line.
    target: f32,
    /// Gain decrease per frame while approaching `target`.
    slope: f32,
    /// Frames until the last over has left the delay line.
    hold: usize,
}

impl Limiter {
    fn new(rate: f64, on: bool, lookahead_ms: f64) -> Self {
        let frames = (rate * lookahead_ms.clamp(0.0, 20.0) / 1000.0) as usize;
        Self {
            on,
            gain: 1.0,
            release: (1.0 - (-1.0 / (LIMITER_RELEASE_S * rate.max(1.0))).exp()) as f32,
            delay: vec![0.0; frames * 2],
            at: 0,
            target: 1.0,
            slope: 0.0,
            hold: 0,
        }
    }

    fn recover(&mut self) {
        if self.gain < 1.0 {
            self.gain += (1.0 - self.gain) * self.release;
            if self.gain > 0.9999 {
                self.gain = 1.0;
            }
        }
    }

    /// Limit `mix` (interleaved stereo, both channels by the same gain).
    /// With look-ahead the output is the input delayed by its length, also
    /// while the limiter is switched off (the delay never jumps).
    fn process(&mut self, mix: &mut [f32]) {
        let frames = self.delay.len() / 2;
        for f in mix.as_chunks_mut::<2>().0 {
            let peak = f[0].abs().max(f[1].abs());
            let need = if self.on && peak > 1.0 {
                1.0 / peak
            } else {
                1.0
            };
            if frames == 0 {
                self.recover();
                self.gain = self.gain.min(need);
            } else {
                if need < 1.0 {
                    // This peak leaves the delay in `frames` frames: be at
                    // its gain by then, never slower than already planned.
                    self.target = self.target.min(need);
                    self.slope = self.slope.max((self.gain - need).max(0.0) / frames as f32);
                    // Include this input frame and the frame on which it
                    // emerges from the delay before starting recovery.
                    self.hold = frames + 1;
                }
                if self.hold > 0 {
                    self.hold -= 1;
                    self.gain = (self.gain - self.slope).max(self.target);
                    if self.hold == 0 {
                        self.target = 1.0;
                        self.slope = 0.0;
                    }
                } else {
                    self.recover();
                }
                if let Some(slot) = self.delay.get_mut(self.at * 2..self.at * 2 + 2) {
                    let delayed = [slot[0], slot[1]];
                    slot.copy_from_slice(f);
                    *f = delayed;
                }
                self.at = (self.at + 1) % frames;
            }
            if self.gain < 1.0 {
                f[0] *= self.gain;
                f[1] *= self.gain;
            }
        }
    }
}

impl Renderer {
    /// Render `out` (interleaved, `channels` per frame) at the device rate:
    /// drain queued commands, mix both decks at their gains (unity = 1.0)
    /// and publish the deck snapshots. Channels 1–2 are main L/R, 3–4 are
    /// headphone cue L/R when a complete pair exists (other pairs or split
    /// cue: [`Routing`]); any other channels are silent. Mono folds main
    /// only; cue is never folded into main.
    /// RT-safe: see the crate docs.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        while let Ok(command) = self.rx.pop() {
            match command {
                Command::Mixer(params) => {
                    self.mixer.set(params);
                    continue;
                }
                Command::Limiter(on) => {
                    self.limiter.on = on;
                    continue;
                }
                _ => {}
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
                    let state = &mut self.decks[deck];
                    let (before, raw) = (state.sounding(), !state.keylock);
                    let key = state.key_source();
                    state.sync(speed, beat, anchor, phase);
                    state.declick_after(before, None, raw, key, false);
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
            spare,
            tail_buf,
            resampler,
            input,
            deck_buf,
            pre_fader,
            mix,
            cue_buf,
            mixer,
            limiter,
            shared,
            routing,
            ..
        } = self;
        let mixing = mixer.params.is_some();
        for block in out.chunks_mut(CHUNK * channels) {
            let frames = block.len().div_ceil(channels);
            let mix = &mut mix[..frames * 2];
            mix.fill(0.0);
            let cue_buf = &mut cue_buf[..frames * 2];
            cue_buf.fill(0.0);
            // Burst phase of each key-locked deck at the end of this chunk
            // (decks not rendered yet are still `frames` behind).
            let mut bursts: [Option<f64>; DECKS] = std::array::from_fn(|d| {
                let deck = &decks[d];
                (deck.keylock && deck.primed && deck.playing)
                    .then_some(deck.clock + deck.stagger as f64 + frames as f64)
            });
            for (c, ((deck, stretch), spare)) in decks
                .iter_mut()
                .zip(stretch.iter_mut())
                .zip(spare.iter_mut())
                .enumerate()
            {
                if deck.swap {
                    // The engine that was sounding becomes the fading tail;
                    // the other one starts the new signal from scratch.
                    std::mem::swap(stretch, spare);
                    deck.swap = false;
                    deck.primed = false;
                }
                let buf = &mut deck_buf[..frames * 2];
                if deck.keylock && deck.track.is_some() {
                    // Used only if the deck primes now: half an interval
                    // away from the burst phase of a deck already running.
                    let interval = stretch.interval_frames();
                    let stagger = bursts
                        .iter()
                        .enumerate()
                        .find_map(|(o, phase)| phase.filter(|_| o != c && interval > 0))
                        // `phase` is the other deck's at the end of this chunk;
                        // this deck's will be `stagger + frames` by then.
                        .map_or(0, |phase| {
                            (phase as usize + interval / 2 + interval - frames % interval)
                                % interval
                        });
                    deck.render_keylock(stretch.as_mut(), resampler, input, buf, rate, stagger);
                } else {
                    deck.primed = false;
                    for f in buf.as_chunks_mut::<2>().0 {
                        let (l, r) = deck.next(rate, resampler);
                        *f = [l, r];
                    }
                }
                if deck.key_tail.is_some() {
                    deck.render_key_tail(spare.as_mut(), resampler, input, tail_buf, buf, rate);
                }
                // This deck's burst phase at the start of the next chunk.
                bursts[c] = (deck.keylock && deck.primed && deck.playing)
                    .then_some(deck.clock + deck.stagger as f64);
                // Capture pre-fader signal for cue bus.
                let pre = &mut pre_fader[..frames * 2];
                if mixing {
                    let peak = mixer.process_channel(c, buf, Some(pre));
                    meter(shared, c, peak);
                }
                for (m, v) in mix.iter_mut().zip(buf.iter()) {
                    *m += v;
                }
                // Process cue channel: apply cue gain to pre-fader signal and sum into cue_buf.
                if mixing {
                    mixer.process_cue_channel(c, pre);
                    for (cue, v) in cue_buf.iter_mut().zip(pre.iter()) {
                        *cue += v;
                    }
                }
            }
            if mixing {
                let peak = mixer.process_master(mix);
                meter(shared, DECKS, peak);
                // Process cue bus master volume and meter it.
                let cue_peak = mixer.process_cue(cue_buf);
                meter(shared, CUE_METER, cue_peak);
            }
            limiter.process(mix);
            let routing = *routing;
            for ((frame, [l, r]), [cue_l, cue_r]) in block
                .chunks_mut(channels)
                .zip(mix.as_chunks::<2>().0)
                .zip(cue_buf.as_chunks::<2>().0)
            {
                let (l, r) = (l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0));
                let (cue_l, cue_r) = (cue_l.clamp(-1.0, 1.0), cue_r.clamp(-1.0, 1.0));
                match frame {
                    [mono] => *mono = (l + r) * 0.5,
                    [first, second, rest @ ..] if routing.split => {
                        // Split cue: main in mono on the left, cue in mono
                        // on the right of the first pair.
                        *first = (l + r) * 0.5;
                        *second = (cue_l + cue_r) * 0.5;
                        rest.fill(0.0);
                    }
                    _ => {
                        // Complete, separate stereo pairs only: a device
                        // never receives half a bus, and cue is never
                        // folded into main.
                        frame.fill(0.0);
                        let (master, cue) = (usize::from(routing.master), usize::from(routing.cue));
                        if let Some(pair) = frame.get_mut(master..master + 2) {
                            pair[0] = l;
                            pair[1] = r;
                        }
                        if master.abs_diff(cue) >= 2
                            && let Some(pair) = frame.get_mut(cue..cue + 2)
                        {
                            pair[0] = cue_l;
                            pair[1] = cue_r;
                        }
                    }
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

    /// Fade time for starts, stops and jumps (0 = sample-exact steps). Real
    /// outputs use [`DECLICK_MS`]; a hand-driven renderer ([`Engine::manual`])
    /// starts at 0. Call before rendering, not from another thread.
    /// Which device channels carry main and cue (default: main on 1/2, cue
    /// on 3/4). Call before rendering, not from another thread.
    pub fn set_routing(&mut self, routing: Routing) {
        self.routing = routing;
    }

    /// Limiter look-ahead (0 = none, the main output is not delayed). Real
    /// outputs use [`LIMITER_LOOKAHEAD_MS`]; a hand-driven renderer starts
    /// at 0. Allocates: call before rendering, not from the callback.
    pub fn set_limiter_lookahead_ms(&mut self, ms: f64) {
        self.limiter = Limiter::new(self.device_rate, self.limiter.on, ms);
    }

    pub fn set_declick_ms(&mut self, ms: f64) {
        let frames = (self.device_rate * ms.clamp(0.0, 100.0) / 1000.0) as u32;
        for deck in &mut self.decks {
            deck.declick = frames;
            deck.fade_in = 0;
            deck.tail = None;
            deck.key_tail = None;
        }
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
    mut decks: [DeckState; DECKS],
    device_rate: u32,
    mixer: Option<MixerParams>,
    // A real output (declick and limiter look-ahead on) or the hand-driven
    // renderer of the tests (sample-exact, undelayed).
    real: bool,
    limiter: bool,
) -> (Renderer, rtrb::Producer<Command>, Arc<Shared>) {
    let declick_ms = if real { DECLICK_MS } else { 0.0 };
    let lookahead_ms = if real { LIMITER_LOOKAHEAD_MS } else { 0.0 };
    let declick = (f64::from(device_rate) * declick_ms / 1000.0) as u32;
    for deck in &mut decks {
        deck.declick = declick;
        // A deck carried over from another output resumes with a fade.
        deck.fade_in = if deck.playing { declick } else { 0 };
    }
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
    let engines = || -> [Box<dyn TimeStretch>; DECKS] {
        std::array::from_fn(|_| Box::new(Signalsmith::new(device_rate)) as Box<dyn TimeStretch>)
    };
    let (stretch, spare) = (engines(), engines());
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
        spare,
        tail_buf: vec![0.0; CHUNK * 2],
        resampler: Resampler::new(),
        input: vec![0.0; input_frames * 2],
        deck_buf: vec![0.0; CHUNK * 2],
        pre_fader: vec![0.0; CHUNK * 2],
        mix: vec![0.0; CHUNK * 2],
        cue_buf: vec![0.0; CHUNK * 2],
        mixer: mixer::Mixer::new(device_rate, mixer),
        limiter: Limiter::new(f64::from(device_rate), limiter, lookahead_ms),
        routing: Routing::default(),
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
    /// The limiter was switched off (it is on by default).
    limiter_off: bool,
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
    /// The output the user asked for (reopened after a device loss).
    config: OutputConfig,
    /// When to try the device again (`None` = not waiting for one).
    retry_at: Option<Instant>,
}

/// Pause between attempts to take a lost device back.
const RETRY: Duration = Duration::from_secs(2);

fn try_output<T>(
    config: &OutputConfig,
    selected_jack: bool,
    mut attempt: impl FnMut(bool) -> Result<T, String>,
) -> Result<T, String> {
    match attempt(true) {
        Err(first)
            if cfg!(all(target_os = "linux", feature = "jack"))
                && selected_jack
                && config.host.is_none()
                && config.device.is_none() =>
        {
            attempt(false)
                .map_err(|second| format!("{first}; default output also failed: {second}"))
        }
        result => result,
    }
}

impl Engine {
    /// Open the output chosen by the environment ([`OutputConfig::from_env`];
    /// nothing set = the default device); fall back to the silent output
    /// (with the reason in [`Engine::error`]) when it does not open. The
    /// device is then retried from [`Engine::poll`].
    pub fn start() -> Self {
        Self::start_with(OutputConfig::from_env())
    }

    /// As [`Engine::start`] with an explicit output choice.
    pub fn start_with(config: OutputConfig) -> Self {
        Self::launch(Default::default(), Memory::default(), config, None)
    }

    /// Silent paced output only (no device is touched, none is retried).
    pub fn silent(reason: &str) -> Self {
        Self::launch(
            Default::default(),
            Memory::default(),
            OutputConfig::default(),
            Some(reason.to_owned()),
        )
    }

    /// Fake sink for tests: the caller renders blocks by hand.
    pub fn manual(device_rate: u32) -> (Self, Renderer) {
        let (renderer, tx, shared) = channel(Default::default(), device_rate, None, false, true);
        let engine = Self {
            tx,
            shared,
            output: output::Output::Manual,
            device_rate,
            memory: Memory::default(),
            sent: [0; DECKS],
            retired: Vec::new(),
            error: None,
            config: OutputConfig::default(),
            retry_at: None,
        };
        (engine, renderer)
    }

    /// `silent`: `Some(reason)` renders into the void without touching a
    /// device; `None` opens `config` and, if that fails, renders into the
    /// void and schedules a retry.
    fn launch(
        decks: [DeckState; DECKS],
        memory: Memory,
        config: OutputConfig,
        silent: Option<String>,
    ) -> Self {
        let wanted = silent.is_none();
        // Positions are source frames, so the device rate only sets the pace
        // and a fallback renderer continues from the same deck state.
        let attempt = match silent {
            Some(reason) => Err(reason),
            None => output::select_host(&config).and_then(|(host, fallback)| {
                try_output(&config, fallback, |preferred| {
                    let default;
                    let host = if preferred {
                        &host
                    } else {
                        default = cpal::default_host();
                        &default
                    };
                    let device = output::Device::open_on(&config, host)?;
                    let rate = device.rate;
                    let (mut renderer, tx, shared) =
                        channel(decks.clone(), rate, memory.mixer, true, !memory.limiter_off);
                    renderer.set_routing(config.routing);
                    device
                        .play(renderer, shared.clone())
                        .map(|output| (output, tx, shared, rate))
                })
            }),
        };
        let (output, tx, shared, device_rate, error) = match attempt {
            Ok((output, tx, shared, rate)) => (output, tx, shared, rate, None),
            Err(reason) => {
                let (renderer, tx, shared) = channel(
                    decks,
                    output::SILENT_RATE,
                    memory.mixer,
                    true,
                    !memory.limiter_off,
                );
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
            retry_at: (wanted && error.is_some()).then(|| Instant::now() + RETRY),
            error,
            config,
        }
    }

    /// The output choice in use (what [`Engine::reopen`] last received).
    pub fn config(&self) -> &OutputConfig {
        &self.config
    }

    /// Switch to another output (device, rate, buffer size) without losing
    /// the decks: tracks, positions, cues, loops, play state, gains, tempo
    /// and the mixer carry over. Audio pauses for the time the device takes
    /// to open. When it does not open the engine continues silently with
    /// the reason in [`Engine::error`] and keeps retrying.
    pub fn reopen(&mut self, config: OutputConfig) {
        self.config = config;
        self.rebuild(None);
    }

    /// Replace the output, carrying every deck over. The old output is
    /// closed first: exclusive hosts (ASIO, ALSA `hw:`) refuse a second
    /// stream on the same device.
    fn rebuild(&mut self, silent: Option<String>) {
        let memory = self.memory.clone();
        let decks: [DeckState; DECKS] = std::array::from_fn(|d| {
            let snap = self.deck(d);
            let settings = DeckState {
                gain: memory.gains[d].unwrap_or(1.0),
                speed: memory.speeds[d].unwrap_or(1.0),
                keylock: memory.keylocks[d],
                ..DeckState::default()
            };
            match &memory.current[d] {
                // The callback was playing the track the UI last sent.
                Some((id, track)) if Some(*id) == snap.id => DeckState {
                    id: *id,
                    track: Some(track.clone()),
                    playing: snap.playing && !snap.previewing,
                    pos: snap.frame,
                    cue: snap.cue,
                    loop_in: snap.loop_in,
                    loop_range: snap.loop_range,
                    ..settings
                },
                // A Load still in the ring when the output went away: the
                // deck comes back with that track, at its start.
                Some((id, track)) => DeckState {
                    id: *id,
                    track: Some(track.clone()),
                    ..settings
                },
                None => settings,
            }
        });
        // Stops the stream or thread; its renderer's tracks are still held
        // by `memory`, so nothing is freed on an audio thread.
        self.output = output::Output::Manual;
        let retired = std::mem::take(&mut self.retired);
        *self = Self::launch(decks, memory, self.config.clone(), silent);
        self.retired = retired;
    }

    /// Queue `command`; `false` when the ring is full (command dropped).
    pub fn send(&mut self, command: Command) -> bool {
        // The single producer cannot lose capacity between this check and push;
        // only the consumer runs concurrently and it can only free slots.
        if self.tx.is_full() {
            return false;
        }
        match &command {
            Command::Mixer(params) => self.memory.mixer = *params,
            Command::Limiter(on) => self.memory.limiter_off = !on,
            _ => {}
        }
        let deck = command.deck();
        if deck >= DECKS && !matches!(command, Command::Mixer(_) | Command::Limiter(_)) {
            // Never reaches the callback: a track for a deck that does not
            // exist would otherwise be freed there.
            return false;
        }
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
                // Remember only what the callback accepts (same rules).
                Command::Gain { gain, .. } if gain.is_finite() => {
                    self.memory.gains[deck] = Some(gain.max(0.0));
                }
                Command::Speed { speed, .. } | Command::Sync { speed, .. }
                    if speed.is_finite() && *speed > 0.0 =>
                {
                    self.memory.speeds[deck] = Some(*speed);
                }
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
            block: s.block.load(Ordering::Relaxed),
            max_render_ns: s.max_render_ns.load(Ordering::Relaxed),
            render_ns: s.render_ns.load(Ordering::Relaxed),
            max_load_permille: s.max_load_permille.load(Ordering::Relaxed),
            latency_ns: s.latency_ns.load(Ordering::Relaxed),
        }
    }

    /// Forget the worst-case timings (gap, render time, load, block range)
    /// so a new measurement window starts; counters keep running.
    pub fn reset_stats(&self) {
        let s = &self.shared.stats;
        s.max_interval_ns.store(0, Ordering::Relaxed);
        s.max_render_ns.store(0, Ordering::Relaxed);
        s.max_load_permille.store(0, Ordering::Relaxed);
        s.min_block.store(u32::MAX, Ordering::Relaxed);
        s.max_block.store(0, Ordering::Relaxed);
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

    /// Housekeeping, once per UI frame: free tracks the callback released;
    /// when the device died, continue silently with every deck's state
    /// (track, position, cue, loop, play state, gain, speed) carried over;
    /// and while a wanted device is missing, try it again every two seconds
    /// and move the decks back onto it when it opens.
    pub fn poll(&mut self) {
        self.retired.retain(|t| Arc::strong_count(t) > 1);
        if self.shared.stats.dead.load(Ordering::Acquire) {
            let was_device = matches!(self.output, output::Output::Device { .. });
            let reason = format!("audio device lost ({}); continuing silently", self.output());
            self.rebuild(Some(reason));
            // `rebuild(Some)` never schedules a retry; a lost real device
            // does (an engine that never had one stays silent).
            self.retry_at = was_device.then(|| Instant::now() + RETRY);
        } else if self.retry_at.is_some_and(|at| Instant::now() >= at) {
            // Probe first so a still-missing device costs no deck handover.
            match output::select_host(&self.config).and_then(|(host, fallback)| {
                try_output(&self.config, fallback, |preferred| {
                    let default;
                    let host = if preferred {
                        &host
                    } else {
                        default = cpal::default_host();
                        &default
                    };
                    output::Device::open_on(&self.config, host)
                })
            }) {
                Ok(_) => self.rebuild(None),
                Err(_) => self.retry_at = Some(Instant::now() + RETRY),
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod keylock_tests;
