//! Band-limited interpolation for reading a track at a fractional position
//! (rate conversion 44.1 ↔ 48 kHz, varispeed, the key-lock input).
//!
//! A Kaiser-windowed sinc of [`TAPS`] taps, tabulated at [`PHASES`]
//! fractional offsets (linear blend between neighbours), once per cutoff in
//! [`SCALES`]: reading faster than one source frame per output frame lowers
//! the cutoff so the content above the output's Nyquist is removed instead
//! of folding back. The kernel is symmetric (zero phase): positions keep
//! their meaning, a sample read at an integer position at full bandwidth is
//! the stored sample itself.
//!
//! Tables are built in [`Resampler::new`] (allocates; off the callback).
//! [`Resampler::at`] only reads: no allocation, lock or panic.

use std::f64::consts::PI;

/// Kernel length in source frames.
pub(crate) const TAPS: usize = 32;
/// Fractional offsets tabulated per source frame.
const PHASES: usize = 128;
/// Taps before the frame at `floor(pos)`.
const BEFORE: usize = TAPS / 2 - 1;
/// Cutoffs as a share of the source band, widest first.
const SCALES: [f64; 8] = [1.0, 0.95, 0.9, 0.85, 0.7, 0.5, 0.35, 0.25];
/// −6 dB point at full bandwidth, in cycles per source frame (20.7 kHz at
/// 44.1 kHz): flat to about 18 kHz, stop band from about 23 kHz.
const CUTOFF: f64 = 0.47;
/// Kaiser β (about 60 dB stop band).
const BETA: f64 = 6.0;

/// Modified Bessel function of the first kind, order 0 (power series).
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 1.0);
    while term > sum * 1e-12 {
        term *= (x / (2.0 * k)) * (x / (2.0 * k));
        sum += term;
        k += 1.0;
    }
    sum
}

pub(crate) struct Resampler {
    /// `SCALES.len() × (PHASES + 1)` kernels, each summing to 1.
    kernels: Vec<[f32; TAPS]>,
}

impl Resampler {
    pub(crate) fn new() -> Self {
        let half = TAPS as f64 / 2.0;
        let i0_beta = bessel_i0(BETA);
        let mut kernels = Vec::with_capacity(SCALES.len() * (PHASES + 1));
        for scale in SCALES {
            let fc = CUTOFF * scale;
            for phase in 0..=PHASES {
                let frac = phase as f64 / PHASES as f64;
                let mut taps = [0.0_f64; TAPS];
                for (k, tap) in taps.iter_mut().enumerate() {
                    // Distance (source frames) from the read position.
                    let x = k as f64 - BEFORE as f64 - frac;
                    let t = x / half;
                    if t.abs() >= 1.0 {
                        continue;
                    }
                    let window = bessel_i0(BETA * (1.0 - t * t).sqrt()) / i0_beta;
                    let a = 2.0 * PI * fc * x;
                    let sinc = if a.abs() < 1e-9 { 1.0 } else { a.sin() / a };
                    *tap = 2.0 * fc * sinc * window;
                }
                let sum: f64 = taps.iter().sum();
                kernels.push(taps.map(|v| (v / sum) as f32));
            }
        }
        Self { kernels }
    }

    /// The kernel set for reading `step` source frames per output frame.
    pub(crate) fn table(step: f64) -> usize {
        // The widest cutoff that still fits under the output's Nyquist
        // (2 % slack so 1.0 ± rounding stays at full bandwidth).
        let fit = if step.is_finite() && step > 1.0 {
            1.02 / step
        } else {
            1.0
        };
        SCALES
            .iter()
            .position(|&s| s <= fit)
            .unwrap_or(SCALES.len() - 1)
    }

    /// `(left, right)` of interleaved stereo `samples` at fractional source
    /// frame `pos`, through kernel set `table`; silence outside the track.
    pub(crate) fn at(&self, samples: &[f32], pos: f64, table: usize) -> (f32, f32) {
        self.at_with_end_hold(samples, pos, table, false)
    }

    /// Read with the final frame extended beyond EOF when requested by a loop.
    pub(crate) fn at_with_end_hold(
        &self,
        samples: &[f32],
        pos: f64,
        table: usize,
        hold: bool,
    ) -> (f32, f32) {
        if pos.is_nan() || pos < 0.0 {
            return (0.0, 0.0);
        }
        let i = pos as u64;
        let frac = pos - i as f64;
        let i = usize::try_from(i).unwrap_or(usize::MAX);
        if frac == 0.0 && table == 0 {
            // Full bandwidth on a stored frame: the frame itself, bit for bit.
            let at = i.saturating_mul(2);
            return match (samples.get(at), samples.get(at.saturating_add(1))) {
                (Some(l), Some(r)) => (*l, *r),
                _ => (0.0, 0.0),
            };
        }
        let p = frac * PHASES as f64;
        let p0 = (p as usize).min(PHASES - 1);
        let w = (p - p0 as f64) as f32;
        let base = table.min(SCALES.len() - 1) * (PHASES + 1) + p0;
        let (Some(a), Some(b)) = (self.kernels.get(base), self.kernels.get(base + 1)) else {
            return (0.0, 0.0);
        };
        let (mut l, mut r) = (0.0_f32, 0.0_f32);
        let window = i
            .checked_sub(BEFORE)
            .and_then(|start| samples.get(start.saturating_mul(2)..))
            .and_then(|s| s.get(..TAPS * 2));
        if let Some(window) = window {
            for ((frame, ka), kb) in window.as_chunks::<2>().0.iter().zip(a).zip(b) {
                let c = ka + (kb - ka) * w;
                l += c * frame[0];
                r += c * frame[1];
            }
        } else {
            // Near either end: frames outside the track are silence.
            for (k, (ka, kb)) in a.iter().zip(b).enumerate() {
                let Some(at) = i
                    .saturating_add(k)
                    .checked_sub(BEFORE)
                    .map(|f| f.saturating_mul(2))
                else {
                    continue;
                };
                let at = if hold && !samples.is_empty() {
                    at.min(samples.len().saturating_sub(2))
                } else {
                    at
                };
                if let (Some(sl), Some(sr)) = (samples.get(at), samples.get(at.saturating_add(1))) {
                    let c = ka + (kb - ka) * w;
                    l += c * sl;
                    r += c * sr;
                }
            }
        }
        (l, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stereo sine at `hz` (right inverted), `frames` long at `rate`.
    fn sine(hz: f64, rate: f64, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (2.0 * PI * hz * i as f64 / rate).sin() as f32;
                [v, -v]
            })
            .collect()
    }

    /// RMS of the left channel read at `step` from `samples`, skipping the
    /// edges, and the RMS error against the ideal sine at `hz`.
    fn read(rs: &Resampler, samples: &[f32], step: f64, hz: f64, rate: f64) -> (f64, f64) {
        let table = Resampler::table(step);
        let frames = samples.len() / 2;
        let (mut power, mut error, mut n) = (0.0, 0.0, 0.0);
        let mut pos = 64.0;
        while pos < (frames - 64) as f64 {
            let (l, r) = rs.at(samples, pos, table);
            assert_eq!(l, -r, "channels are filtered alike");
            let ideal = (2.0 * PI * hz * pos / rate).sin();
            power += f64::from(l) * f64::from(l);
            error += (f64::from(l) - ideal) * (f64::from(l) - ideal);
            n += 1.0;
            pos += step;
        }
        ((power / n).sqrt(), (error / n).sqrt())
    }

    #[test]
    fn integer_positions_at_full_bandwidth_return_the_stored_frame() {
        let rs = Resampler::new();
        let samples = sine(1_000.0, 44_100.0, 400);
        for i in [0_usize, 1, 17, 200, 399] {
            let got = rs.at(&samples, i as f64, 0);
            assert_eq!(got, (samples[i * 2], samples[i * 2 + 1]));
        }
        assert_eq!(rs.at(&samples, 400.0, 0), (0.0, 0.0));
        assert_eq!(rs.at(&samples, -1.0, 0), (0.0, 0.0));
        assert_eq!(rs.at(&samples, f64::NAN, 0), (0.0, 0.0));
        assert_eq!(rs.at(&samples, 1e300, 0), (0.0, 0.0));
    }

    #[test]
    fn pass_band_is_flat_and_accurate_where_linear_interpolation_droops() {
        let rs = Resampler::new();
        let step = 44_100.0 / 48_000.0;
        // (Hz, most error allowed as a share of the sine's RMS).
        for (hz, tolerance) in [
            (100.0, 0.002),
            (1_000.0, 0.002),
            (10_000.0, 0.01),
            (16_000.0, 0.02),
        ] {
            let samples = sine(hz, 44_100.0, 8_000);
            let (rms, error) = read(&rs, &samples, step, hz, 44_100.0);
            let ideal = std::f64::consts::FRAC_1_SQRT_2;
            assert!((rms / ideal - 1.0).abs() < 0.01, "{hz} Hz level {rms}");
            assert!(error / ideal < tolerance, "{hz} Hz error {error}");
        }
        // Linear interpolation at 16 kHz is off by far more than that.
        let samples = sine(16_000.0, 44_100.0, 8_000);
        let (mut error, mut n) = (0.0, 0.0);
        let mut pos = 64.0;
        while pos < 7_900.0 {
            let i = pos as usize;
            let f = pos - i as f64;
            let v = f64::from(samples[i * 2]) * (1.0 - f) + f64::from(samples[i * 2 + 2]) * f;
            let ideal = (2.0 * PI * 16_000.0 * pos / 44_100.0).sin();
            error += (v - ideal) * (v - ideal);
            n += 1.0;
            pos += step;
        }
        assert!(
            (error / n).sqrt() > 0.2,
            "linear error {}",
            (error / n).sqrt()
        );
    }

    #[test]
    fn reading_faster_removes_what_would_alias() {
        let rs = Resampler::new();
        // 20 kHz in a 48 kHz track read at double speed: the output's
        // Nyquist is 12 kHz of source band, so the tone must be gone...
        let samples = sine(20_000.0, 48_000.0, 16_000);
        let (rms, _) = read(&rs, &samples, 2.0, 20_000.0, 48_000.0);
        assert!(rms < 0.003, "aliased level {rms}");
        // ...while 5 kHz passes at full level.
        let samples = sine(5_000.0, 48_000.0, 16_000);
        let (rms, error) = read(&rs, &samples, 2.0, 5_000.0, 48_000.0);
        assert!(
            (rms / std::f64::consts::FRAC_1_SQRT_2 - 1.0).abs() < 0.02,
            "{rms}"
        );
        assert!(error < 0.02, "{error}");
        assert_eq!(Resampler::table(1.0), 0);
        assert_eq!(Resampler::table(44_100.0 / 48_000.0), 0);
        assert_eq!(Resampler::table(0.5), 0);
        assert!(Resampler::table(48_000.0 / 44_100.0) > 0);
        assert_eq!(Resampler::table(100.0), SCALES.len() - 1);
        assert_eq!(Resampler::table(f64::NAN), 0);
        // A few percent of tempo costs a few percent of bandwidth, no more.
        assert_eq!(SCALES[Resampler::table(1.03)], 0.95);
        assert_eq!(SCALES[Resampler::table(1.08)], 0.9);
        assert_eq!(SCALES[Resampler::table(1.16)], 0.85);
        let samples = sine(12_000.0, 44_100.0, 16_000);
        let (rms, error) = read(&rs, &samples, 1.08, 12_000.0, 44_100.0);
        assert!(
            (rms / std::f64::consts::FRAC_1_SQRT_2 - 1.0).abs() < 0.01,
            "{rms}"
        );
        assert!(error < 0.01, "{error}");
    }

    #[test]
    fn edges_fade_into_silence_without_reading_outside() {
        let rs = Resampler::new();
        let samples = vec![1.0_f32; 64 * 2];
        for pos in [0.25, 3.5, 30.75, 62.5, 63.99, 70.0, 79.0] {
            let (l, r) = rs.at(&samples, pos, 0);
            assert!(l.is_finite() && (l - r).abs() < 1e-6);
            assert!((-0.2..=1.2).contains(&l), "{pos}: {l}");
        }
        // Constant input well inside: unity gain at every phase and cutoff.
        let samples = vec![0.5_f32; 200 * 2];
        for table in 0..SCALES.len() {
            for phase in 0..40 {
                let (l, _) = rs.at(&samples, 100.0 + f64::from(phase) / 40.0, table);
                assert!((l - 0.5).abs() < 1e-5, "table {table} phase {phase}: {l}");
            }
        }
    }
}
