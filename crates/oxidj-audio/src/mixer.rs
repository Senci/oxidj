//! M8/D4: the optional two-channel DJ mixer (ADR-008 D4 addendum).
//!
//! Per channel: trim → 3-band isolator EQ → colour filter → (meter) →
//! channel fader × crossfader; then master volume (meter) and the output
//! clamp. Everything is in-house and dependency-free:
//!
//! * **EQ**: a DJ isolator built from 4th-order Linkwitz-Riley crossovers
//!   (each two cascaded 2nd-order Butterworth biquads, RBJ cookbook) at
//!   [`LOW_MID_HZ`] and [`MID_HIGH_HZ`]. The low band passes an all-pass at
//!   the upper crossover so the three bands sum to a flat-magnitude
//!   all-pass at 0 dB; a band at its kill stop is removed (24 dB/octave
//!   skirts). Range: kill (−∞) to +6 dB.
//! * **Filter**: one knob, centre off; left sweeps a low-pass down, right a
//!   high-pass up (2nd order, slight resonance).
//! * **Crossfader**: Smooth / Sharp / Cut curves, both channels at unity in
//!   the centre ([`crossfader_gains`]).
//!
//! RT-safety: [`Mixer::process_channel`] and [`Mixer::process_master`] run
//! in the callback: no allocation, lock or I/O; filter coefficients are
//! plain arithmetic; gains ramp linearly over each chunk (no zipper).
//! Filter state is flushed to zero below 1e-25 (no denormals) and reset if
//! it ever becomes non-finite (a NaN sample cannot latch the filter).
//!
//! The mixer is optional: with no [`MixerParams`] the renderer skips this
//! module entirely and the decks sum at unity exactly as before (D2/D3).
//!
//! **Headphone cue**: every channel's post-filter, pre-fader signal is tapped
//! for the independent cue bus ([`Strip::apply_cue_gain`]). The per-deck cue
//! gain (`cue_mix`), the cue volume and the master volume all ramp linearly
//! across each chunk, so CUE switches and knob moves fade instead of clicking.

use crate::DECKS;
use std::f64::consts::PI;

/// Low / mid crossover (Hz).
pub const LOW_MID_HZ: f64 = 250.0;
/// Mid / high crossover (Hz).
pub const MID_HIGH_HZ: f64 = 2_500.0;
/// EQ knob bottom: at or below this many dB the band is killed (−∞).
pub const EQ_KILL_DB: f32 = -26.0;
/// EQ boost ceiling (dB).
pub const EQ_MAX_DB: f32 = 6.0;
/// Trim range ± dB.
pub const TRIM_DB: f32 = 12.0;
/// Filter knob dead zone around the centre (filter fully out of the path).
pub const FILTER_DEADZONE: f32 = 0.02;
/// Low-pass cutoff at the left end of the filter knob (Hz).
const LP_MIN_HZ: f64 = 120.0;
/// High-pass cutoff at the right end of the filter knob (Hz).
const HP_MAX_HZ: f64 = 7_000.0;
/// Filter sweep start (cutoff just off the knob centre).
const LP_MAX_HZ: f64 = 20_000.0;
const HP_MIN_HZ: f64 = 20.0;
/// Colour-filter resonance.
const FILTER_Q: f64 = 1.0;
const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// Filter state below this is flushed to zero.
const DENORMAL: f64 = 1e-25;

/// `10^(dB/20)`, exactly 1.0 at 0 dB.
pub fn db_to_gain(db: f32) -> f32 {
    if db == 0.0 {
        1.0
    } else {
        10f32.powf(db / 20.0)
    }
}

/// Linear EQ band gain for knob `db`: 0 (kill) at or below [`EQ_KILL_DB`],
/// at most +[`EQ_MAX_DB`].
pub fn eq_gain(db: f32) -> f32 {
    if db.is_nan() || db <= EQ_KILL_DB {
        0.0
    } else {
        db_to_gain(db.min(EQ_MAX_DB))
    }
}

/// Linear trim gain for knob `db` (±[`TRIM_DB`]).
pub fn trim_gain(db: f32) -> f32 {
    if db.is_nan() {
        1.0
    } else {
        db_to_gain(db.clamp(-TRIM_DB, TRIM_DB))
    }
}

/// Crossfader side a channel follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Assign {
    A,
    /// Not affected by the crossfader.
    #[default]
    Thru,
    B,
}

/// Crossfader curve (Settings).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Curve {
    /// Gradual fade over each half of the travel.
    #[default]
    Smooth,
    /// Full level until the last eighth of the travel, then a quick fade.
    Sharp,
    /// Scratch cut: off only within the last 3 % of the travel.
    Cut,
}

/// `[side A gain, side B gain]` at crossfader position `x` (0 = full A,
/// 0.5 = centre, 1 = full B). Both are exactly 1.0 from the centre towards
/// their own side; each falls monotonically to exactly 0.0 at the far end;
/// `gains(x)[0] == gains(1 - x)[1]`.
pub fn crossfader_gains(x: f32, curve: Curve) -> [f32; 2] {
    let x = if x.is_nan() { 0.5 } else { x.clamp(0.0, 1.0) };
    // Distance of the knob from the far end, 1 = centre or nearer.
    let side = |t: f32| -> f32 {
        let t = (t * 2.0).min(1.0);
        let t = match curve {
            Curve::Smooth => t,
            Curve::Sharp => (t * 8.0).min(1.0),
            Curve::Cut => (t * 32.0).min(1.0),
        };
        if t >= 1.0 {
            1.0
        } else if t <= 0.0 {
            0.0
        } else {
            match curve {
                // Equal-power shape within the fading half.
                Curve::Smooth | Curve::Sharp => (t * std::f32::consts::FRAC_PI_2).sin(),
                Curve::Cut => t,
            }
        }
    };
    [side(1.0 - x), side(x)]
}

/// One channel's controls (linear gains; see [`eq_gain`], [`trim_gain`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelParams {
    pub trim: f32,
    /// Low, mid, high band gains (0 = kill, 1 = flat, 2 ≈ +6 dB).
    pub eq: [f32; 3],
    /// −1 (low-pass closed) .. 0 (off) .. +1 (high-pass closed).
    pub filter: f32,
    /// Channel fader 0..1.
    pub fader: f32,
    pub assign: Assign,
    /// Cue button: when active, this channel is routed to the headphone cue bus.
    pub cue: bool,
}

impl Default for ChannelParams {
    fn default() -> Self {
        Self {
            trim: 1.0,
            eq: [1.0; 3],
            filter: 0.0,
            fader: 1.0,
            assign: Assign::Thru,
            cue: false,
        }
    }
}

/// The whole mixer (sent as one [`crate::Command::Mixer`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerParams {
    pub channels: [ChannelParams; DECKS],
    /// Crossfader position (0 = A, 1 = B); `None` = crossfader hidden and
    /// out of the path.
    pub crossfader: Option<f32>,
    pub curve: Curve,
    /// Master volume 0..1.
    pub master: f32,
    /// Cue mix: 0.0 = only deck A in cue, 1.0 = only deck B in cue, 0.5 = both equal.
    pub cue_mix: f32,
    /// Cue volume: 0.0..1.0, controls the cue bus output level.
    pub cue_volume: f32,
}

impl Default for MixerParams {
    /// Deck A on side A, deck B on side B, everything centred / at unity.
    fn default() -> Self {
        let mut channels = [ChannelParams::default(); DECKS];
        channels[0].assign = Assign::A;
        channels[1].assign = Assign::B;
        Self {
            channels,
            crossfader: Some(0.5),
            curve: Curve::Smooth,
            master: 1.0,
            cue_mix: 0.5,
            cue_volume: 1.0,
        }
    }
}

impl MixerParams {
    /// Post-EQ gain of channel `c`: its fader times its crossfader side.
    pub fn channel_out(&self, c: usize) -> f32 {
        let Some(ch) = self.channels.get(c) else {
            return 0.0;
        };
        let side = match (self.crossfader, ch.assign) {
            (Some(x), Assign::A) => crossfader_gains(x, self.curve)[0],
            (Some(x), Assign::B) => crossfader_gains(x, self.curve)[1],
            _ => 1.0,
        };
        finite01(ch.fader) * side
    }

    /// Cue bus gain for deck `c`: cue_mix blends between A (0) and B (1).
    /// Deck A gets (1 - cue_mix), deck B gets cue_mix.
    pub fn cue_gain(&self, c: usize) -> f32 {
        let gain = match c {
            0 => 1.0 - self.cue_mix,
            1 => self.cue_mix,
            _ => 0.0,
        };
        finite01(gain)
    }
}

fn finite01(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

fn finite_gain(v: f32, max: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, max) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Low,
    High,
    All,
}

/// 2nd-order IIR, transposed direct form II, `f64` state per stereo side.
#[derive(Clone, Copy, Debug, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [[f64; 2]; 2],
}

impl Biquad {
    fn new(kind: Kind, hz: f64, q: f64, rate: f64) -> Self {
        let mut f = Self::default();
        f.set(kind, hz, q, rate);
        f
    }

    /// RBJ cookbook coefficients (state is kept).
    fn set(&mut self, kind: Kind, hz: f64, q: f64, rate: f64) {
        let w0 = 2.0 * PI * (hz / rate).clamp(1e-6, 0.49);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b = match kind {
            Kind::Low => [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0],
            Kind::High => [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            Kind::All => [1.0 - alpha, -2.0 * cos, 1.0 + alpha],
        };
        self.b = b.map(|v| v / a0);
        self.a = [-2.0 * cos / a0, (1.0 - alpha) / a0];
    }

    #[inline]
    fn tick(&mut self, side: usize, x: f64) -> f64 {
        let [b0, b1, b2] = self.b;
        let [a1, a2] = self.a;
        let z = &mut self.z[side & 1];
        let y = b0 * x + z[0];
        z[0] = b1 * x - a1 * y + z[1];
        z[1] = b2 * x - a2 * y;
        y
    }

    fn reset(&mut self) {
        self.z = [[0.0; 2]; 2];
    }

    /// Flush tiny state to zero; `false` when the state is not finite.
    fn flush(&mut self) -> bool {
        let mut finite = true;
        for v in self.z.iter_mut().flatten() {
            finite &= v.is_finite();
            if v.abs() < DENORMAL {
                *v = 0.0;
            }
        }
        finite
    }
}

/// One Linkwitz-Riley 4th-order section: two Butterworth biquads in series.
#[inline]
fn lr4(pair: &mut [Biquad; 2], side: usize, x: f64) -> f64 {
    let [first, second] = pair;
    second.tick(side, first.tick(side, x))
}

/// Sanitised gains a channel ramps to: trim, low, mid, high, out.
fn targets(p: &ChannelParams, out: f32) -> [f32; 5] {
    let eq = |v| finite_gain(v, db_to_gain(EQ_MAX_DB));
    [
        finite_gain(p.trim, db_to_gain(TRIM_DB)),
        eq(p.eq[0]),
        eq(p.eq[1]),
        eq(p.eq[2]),
        finite01(out),
    ]
}

/// One channel's DSP state.
#[derive(Clone, Copy, Debug)]
struct Strip {
    /// LR4 crossovers: two cascaded Butterworth sections each.
    lp1: [Biquad; 2],
    hp1: [Biquad; 2],
    lp2: [Biquad; 2],
    hp2: [Biquad; 2],
    /// Aligns the low band with the upper crossover's phase.
    ap2: Biquad,
    filter: Biquad,
    /// Filter knob value the coefficients were computed for (`0` = off).
    filter_at: f32,
    /// Current (ramped) gains: trim, low, mid, high, out.
    gains: [f32; 5],
    /// Current (ramped) cue gain applied to the post-filter cue tap.
    cue_gain: f32,
}

impl Strip {
    fn new(rate: f64) -> Self {
        let lp = |hz| Biquad::new(Kind::Low, hz, BUTTERWORTH_Q, rate);
        let hp = |hz| Biquad::new(Kind::High, hz, BUTTERWORTH_Q, rate);
        Self {
            lp1: [lp(LOW_MID_HZ); 2],
            hp1: [hp(LOW_MID_HZ); 2],
            lp2: [lp(MID_HIGH_HZ); 2],
            hp2: [hp(MID_HIGH_HZ); 2],
            ap2: Biquad::new(Kind::All, MID_HIGH_HZ, BUTTERWORTH_Q, rate),
            filter: Biquad::default(),
            filter_at: 0.0,
            gains: [1.0; 5],
            cue_gain: 0.0,
        }
    }

    fn biquads(&mut self) -> impl Iterator<Item = &mut Biquad> {
        self.lp1
            .iter_mut()
            .chain(self.hp1.iter_mut())
            .chain(self.lp2.iter_mut())
            .chain(self.hp2.iter_mut())
            .chain([&mut self.ap2, &mut self.filter])
    }

    fn reset(&mut self) {
        self.biquads().for_each(Biquad::reset);
    }

    /// Point the colour filter at knob value `v` (state kept while it stays
    /// on the same side; cleared when it enters or changes side).
    fn set_filter(&mut self, v: f32, rate: f64) {
        let v = if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) };
        let v = if v.abs() < FILTER_DEADZONE { 0.0 } else { v };
        if v == self.filter_at {
            return;
        }
        if v == 0.0 || self.filter_at == 0.0 || v.signum() != self.filter_at.signum() {
            self.filter.reset();
        }
        self.filter_at = v;
        let t = f64::from(v.abs());
        if v < 0.0 {
            let hz = LP_MAX_HZ * (LP_MIN_HZ / LP_MAX_HZ).powf(t);
            self.filter.set(Kind::Low, hz, FILTER_Q, rate);
        } else if v > 0.0 {
            let hz = HP_MIN_HZ * (HP_MAX_HZ / HP_MIN_HZ).powf(t);
            self.filter.set(Kind::High, hz, FILTER_Q, rate);
        }
    }

    /// Process `buf` (interleaved stereo) in place through trim, EQ and the
    /// colour filter. If `pre_fader_out` is provided, the post-filter signal
    /// (trim, EQ and colour filter applied; before the channel fader and
    /// crossfader) is written there — that is the headphone cue tap. Then
    /// applies the fader×crossfader gain `out_gain` to `buf` in place. Returns
    /// the post-filter peak.
    fn process(
        &mut self,
        buf: &mut [f32],
        p: &ChannelParams,
        out_gain: f32,
        rate: f64,
        mut pre_fader_out: Option<&mut [f32]>,
    ) -> f32 {
        self.set_filter(p.filter, rate);
        let target = targets(p, out_gain);
        let frames = buf.len() / 2;
        let n = frames.max(1) as f32;
        let from = self.gains;
        let filter_on = self.filter_at != 0.0;
        let mut peak = 0.0f32;
        for (i, frame) in buf.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let k = (i + 1) as f32 / n;
            let g: [f32; 5] = std::array::from_fn(|j| from[j] + (target[j] - from[j]) * k);
            for (side, s) in frame.iter_mut().enumerate() {
                let x = f64::from(*s * g[0]);
                let low = lr4(&mut self.lp1, side, x);
                let rest = lr4(&mut self.hp1, side, x);
                let mid = lr4(&mut self.lp2, side, rest);
                let high = lr4(&mut self.hp2, side, rest);
                let low = self.ap2.tick(side, low);
                let mut y = low * f64::from(g[1]) + mid * f64::from(g[2]) + high * f64::from(g[3]);
                if filter_on {
                    y = self.filter.tick(side, y);
                }
                let y = y as f32;
                peak = peak.max(y.abs());
                // Write the post-filter cue tap if requested.
                if let Some(pre) = pre_fader_out.as_deref_mut()
                    && let Some(sample) = pre.get_mut(i * 2 + side)
                {
                    *sample = y;
                }
                *s = y * g[4];
            }
        }
        self.gains = target;
        let mut finite = true;
        for f in self.biquads() {
            finite &= f.flush();
        }
        if !finite {
            self.reset();
            buf.fill(0.0);
            if let Some(pre) = pre_fader_out {
                pre.fill(0.0);
            }
        }
        if peak.is_nan() { 0.0 } else { peak }
    }

    /// Ramp the cue tap from its current gain to `target` across `buf` (one
    /// chunk), so a CUE switch or a MIX move fades instead of clicking. `buf`
    /// holds the post-filter, pre-fader signal captured by [`Strip::process`];
    /// the filter state is not touched here.
    fn apply_cue_gain(&mut self, buf: &mut [f32], target: f32) -> f32 {
        let (from, to) = (self.cue_gain, finite01(target));
        let n = (buf.len() / 2).max(1) as f32;
        let mut peak = 0.0f32;
        for (i, frame) in buf.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let g = from + (to - from) * (i + 1) as f32 / n;
            for s in frame.iter_mut() {
                *s *= g;
                peak = peak.max(s.abs());
            }
        }
        self.cue_gain = to;
        if peak.is_nan() { 0.0 } else { peak }
    }
}

/// Callback-side mixer state.
#[derive(Clone, Debug)]
pub(crate) struct Mixer {
    pub(crate) params: Option<MixerParams>,
    strips: [Strip; DECKS],
    master: f32,
    cue_master: f32,
    rate: f64,
}

impl Mixer {
    pub(crate) fn new(rate: u32, params: Option<MixerParams>) -> Self {
        let rate = f64::from(rate.max(1));
        let mut mixer = Self {
            params: None,
            strips: [Strip::new(rate); DECKS],
            master: 1.0,
            cue_master: 1.0,
            rate,
        };
        mixer.set(params);
        mixer
    }

    /// New controls. Leaving bypass starts from clean filters and the new
    /// gains (no ramp from stale values).
    pub(crate) fn set(&mut self, params: Option<MixerParams>) {
        if let (None, Some(p)) = (self.params, params) {
            for (c, strip) in self.strips.iter_mut().enumerate() {
                strip.reset();
                strip.filter_at = 0.0;
                strip.gains = targets(&p.channels[c], p.channel_out(c));
                strip.cue_gain = if p.channels[c].cue {
                    p.cue_gain(c)
                } else {
                    0.0
                };
            }
            self.master = finite01(p.master);
            self.cue_master = finite01(p.cue_volume);
        }
        self.params = params;
    }

    /// Channel `c` of one chunk, in place; returns its pre-fader peak.
    /// If `pre_fader_out` is provided, writes the pre-fader signal there.
    pub(crate) fn process_channel(
        &mut self,
        c: usize,
        buf: &mut [f32],
        pre_fader_out: Option<&mut [f32]>,
    ) -> f32 {
        let (Some(p), Some(strip)) = (self.params.as_ref(), self.strips.get_mut(c)) else {
            if let Some(pre) = pre_fader_out {
                pre.fill(0.0);
            }
            return 0.0;
        };
        let Some(ch) = p.channels.get(c) else {
            if let Some(pre) = pre_fader_out {
                pre.fill(0.0);
            }
            return 0.0;
        };
        strip.process(buf, ch, p.channel_out(c), self.rate, pre_fader_out)
    }

    /// Process the cue tap: ramp the captured post-filter, pre-fader buffer to
    /// this channel's cue gain (silence when its CUE switch is off).
    pub(crate) fn process_cue_channel(&mut self, c: usize, buf: &mut [f32]) -> f32 {
        let (Some(p), Some(strip)) = (self.params.as_ref(), self.strips.get_mut(c)) else {
            return 0.0;
        };
        let Some(ch) = p.channels.get(c) else {
            return 0.0;
        };
        let target = if ch.cue { p.cue_gain(c) } else { 0.0 };
        strip.apply_cue_gain(buf, target)
    }

    /// Master volume over the summed chunk; returns the peak before the
    /// output clamp (above 1.0 = clipping).
    pub(crate) fn process_master(&mut self, mix: &mut [f32]) -> f32 {
        let Some(p) = self.params.as_ref() else {
            return 0.0;
        };
        let (from, to) = (self.master, finite01(p.master));
        let n = (mix.len() / 2).max(1) as f32;
        let mut peak = 0.0f32;
        for (i, frame) in mix.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let g = from + (to - from) * (i + 1) as f32 / n;
            for s in frame.iter_mut() {
                *s *= g;
                peak = peak.max(s.abs());
            }
        }
        self.master = to;
        if peak.is_nan() { 0.0 } else { peak }
    }

    /// Process the cue bus: mix both decks' pre-fader signals with cue_mix
    /// and apply cue_volume. Returns the peak before the output clamp.
    /// The input `buf` should contain the sum of both decks' pre-fader audio
    /// (after cue gains have been applied per channel).
    pub(crate) fn process_cue(&mut self, buf: &mut [f32]) -> f32 {
        let Some(p) = self.params.as_ref() else {
            buf.fill(0.0);
            return 0.0;
        };
        let (from, to) = (self.cue_master, finite01(p.cue_volume));
        let n = (buf.len() / 2).max(1) as f32;
        let mut peak = 0.0f32;
        for (i, frame) in buf.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let g = from + (to - from) * (i + 1) as f32 / n;
            for s in frame.iter_mut() {
                *s *= g;
                peak = peak.max(s.abs());
            }
        }
        self.cue_master = to;
        if peak.is_nan() { 0.0 } else { peak }
    }
}

#[cfg(test)]
#[path = "mixer_tests.rs"]
mod tests;
