//! OxiDJ offline analysis (layer 1; depends only on `oxidj-core` and the
//! admitted `taktus` tempo engine, ADR-004 B2 amendment).
//!
//! B2a adds streaming tempo/beat-grid analysis: see [`tempo`] and
//! [`TempoBuilder`].
//!
//! W1 scope: a streaming 3-band waveform builder. Callers feed already-decoded
//! **mono** `f32` samples (mix stereo down before pushing, e.g. `(l + r) / 2`)
//! through [`WaveformBuilder::push`] in chunks of any size and call
//! [`WaveformBuilder::finish`] to obtain a [`WaveformPeaks`]. Only the peaks are
//! retained (one column per [`PEAKS_PER_SECOND`]th of a second), never the full
//! signal. Output is identical however the input is split into chunks.
//!
//! The band split is a fixed 2-pole (12 dB/oct) Butterworth crossover:
//! low = low-pass at [`LOW_MID_CROSSOVER_HZ`], high = high-pass at
//! [`MID_HIGH_CROSSOVER_HZ`], mid = high-pass at the first followed by
//! low-pass at the second.
//!
//! Input fails closed: a zero or unsupported sample rate, any NaN/infinite
//! sample, an empty stream, or a sample count different from the declared
//! expected count is refused, and a builder that refused input stays failed.

use oxidj_core::waveform::{Peak, WaveformError, WaveformPeaks};

pub mod key;
pub mod tempo;
pub use key::{Key, KeyBuilder};
pub use tempo::{BpmRange, TempoBuilder, TempoError, TempoGate, TempoOptions};

/// Level-0 column rate (columns per second of audio).
pub const PEAKS_PER_SECOND: u32 = 150;
/// Low/mid crossover frequency.
pub const LOW_MID_CROSSOVER_HZ: f64 = 200.0;
/// Mid/high crossover frequency.
pub const MID_HIGH_CROSSOVER_HZ: f64 = 2_500.0;
/// Lowest accepted sample rate (keeps the 2.5 kHz crossover well below Nyquist).
pub const MIN_SAMPLE_RATE: u32 = 8_000;

/// Why waveform analysis refused its input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalysisError {
    /// Sample rate was zero.
    ZeroSampleRate,
    /// Sample rate below [`MIN_SAMPLE_RATE`].
    UnsupportedSampleRate(u32),
    /// A sample was NaN or infinite.
    NonFiniteSample,
    /// No samples were pushed.
    Empty,
    /// Fewer samples than declared were pushed.
    Truncated {
        /// Declared sample count.
        expected: u64,
        /// Samples actually pushed.
        received: u64,
    },
    /// More samples than declared were pushed.
    TooManySamples {
        /// Declared sample count.
        expected: u64,
    },
    /// The core model rejected the peaks (not expected in practice).
    Peaks(WaveformError),
}

impl core::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroSampleRate => f.write_str("sample rate is zero"),
            Self::UnsupportedSampleRate(rate) => {
                write!(f, "sample rate {rate} Hz is below {MIN_SAMPLE_RATE} Hz")
            }
            Self::NonFiniteSample => f.write_str("input contains a NaN or infinite sample"),
            Self::Empty => f.write_str("no samples were provided"),
            Self::Truncated { expected, received } => {
                write!(
                    f,
                    "input truncated: expected {expected} samples, received {received}"
                )
            }
            Self::TooManySamples { expected } => {
                write!(f, "input exceeds the expected {expected} samples")
            }
            Self::Peaks(error) => write!(f, "invalid waveform peaks: {error}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// 2-pole biquad, transposed direct form II, f64 state.
#[derive(Clone, Copy, Debug)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    /// RBJ-cookbook Butterworth (Q = 1/sqrt 2) low- or high-pass.
    fn butterworth(sample_rate: u32, cutoff_hz: f64, high_pass: bool) -> Self {
        let w0 = core::f64::consts::TAU * cutoff_hz / f64::from(sample_rate);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin * core::f64::consts::FRAC_1_SQRT_2;
        let a0 = 1.0 + alpha;
        let (b0, b1) = if high_pass {
            ((1.0 + cos) / 2.0, -(1.0 + cos))
        } else {
            ((1.0 - cos) / 2.0, 1.0 - cos)
        };
        Self {
            b: [b0 / a0, b1 / a0, b0 / a0],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            z: [0.0; 2],
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// The fixed 3-band crossover, one sample at a time.
///
/// Public so tests and callers can compute a full-buffer reference; outputs are
/// clamped to `[-1.0, 1.0]` (the quantization range).
#[derive(Clone, Copy, Debug)]
pub struct Crossover {
    low: Biquad,
    mid_hp: Biquad,
    mid_lp: Biquad,
    high: Biquad,
}

impl Crossover {
    /// Crossover for `sample_rate`, which must satisfy [`WaveformBuilder::new`].
    pub fn new(sample_rate: u32) -> Result<Self, AnalysisError> {
        check_rate(sample_rate)?;
        Ok(Self {
            low: Biquad::butterworth(sample_rate, LOW_MID_CROSSOVER_HZ, false),
            mid_hp: Biquad::butterworth(sample_rate, LOW_MID_CROSSOVER_HZ, true),
            mid_lp: Biquad::butterworth(sample_rate, MID_HIGH_CROSSOVER_HZ, false),
            high: Biquad::butterworth(sample_rate, MID_HIGH_CROSSOVER_HZ, true),
        })
    }

    /// Split one finite sample into `[low, mid, high]`.
    pub fn split(&mut self, sample: f32) -> [f32; 3] {
        let x = f64::from(sample);
        let low = self.low.process(x);
        let mid = self.mid_lp.process(self.mid_hp.process(x));
        let high = self.high.process(x);
        // Finite f32 input through stable filters stays finite in f64; the
        // clamp keeps the cast in range.
        [low, mid, high].map(|v| v.clamp(-1.0, 1.0) as f32)
    }
}

fn check_rate(sample_rate: u32) -> Result<(), AnalysisError> {
    if sample_rate == 0 {
        Err(AnalysisError::ZeroSampleRate)
    } else if sample_rate < MIN_SAMPLE_RATE {
        Err(AnalysisError::UnsupportedSampleRate(sample_rate))
    } else {
        Ok(())
    }
}

/// Input samples per level-0 column for `sample_rate` (rounded, at least 1).
pub fn samples_per_peak(sample_rate: u32) -> u32 {
    (sample_rate.saturating_add(PEAKS_PER_SECOND / 2) / PEAKS_PER_SECOND).max(1)
}

/// Streaming 3-band waveform builder. See the crate docs for the contract.
#[derive(Clone, Debug)]
pub struct WaveformBuilder {
    sample_rate: u32,
    samples_per_peak: u32,
    expected: Option<u64>,
    crossover: Crossover,
    column_min: [f32; 3],
    column_max: [f32; 3],
    in_column: u32,
    bands: [Vec<Peak>; 3],
    sample_count: u64,
    failed: Option<AnalysisError>,
}

impl WaveformBuilder {
    /// Start a builder for mono input at `sample_rate`. If `expected_samples`
    /// is `Some`, [`finish`](Self::finish) refuses a shorter stream and
    /// [`push`](Self::push) refuses a longer one.
    pub fn new(sample_rate: u32, expected_samples: Option<u64>) -> Result<Self, AnalysisError> {
        let crossover = Crossover::new(sample_rate)?;
        Ok(Self {
            sample_rate,
            samples_per_peak: samples_per_peak(sample_rate),
            expected: expected_samples,
            crossover,
            column_min: [f32::INFINITY; 3],
            column_max: [f32::NEG_INFINITY; 3],
            in_column: 0,
            bands: Default::default(),
            sample_count: 0,
            failed: None,
        })
    }

    /// Feed the next chunk of mono samples. On error nothing from this chunk is
    /// consumed and the builder stays failed.
    pub fn push(&mut self, samples: &[f32]) -> Result<(), AnalysisError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if samples.iter().any(|s| !s.is_finite()) {
            return self.fail(AnalysisError::NonFiniteSample);
        }
        let total = u64::try_from(samples.len())
            .ok()
            .and_then(|len| self.sample_count.checked_add(len));
        match (total, self.expected) {
            (None, _) => return self.fail(AnalysisError::TooManySamples { expected: u64::MAX }),
            (Some(total), Some(expected)) if total > expected => {
                return self.fail(AnalysisError::TooManySamples { expected });
            }
            (Some(total), _) => self.sample_count = total,
        }
        for &sample in samples {
            let split = self.crossover.split(sample);
            for (band, value) in split.into_iter().enumerate() {
                self.column_min[band] = self.column_min[band].min(value);
                self.column_max[band] = self.column_max[band].max(value);
            }
            self.in_column += 1;
            if self.in_column == self.samples_per_peak {
                self.flush_column();
            }
        }
        Ok(())
    }

    /// Finish the stream and build the peaks with their mipmap.
    pub fn finish(mut self) -> Result<WaveformPeaks, AnalysisError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if self.sample_count == 0 {
            return Err(AnalysisError::Empty);
        }
        if let Some(expected) = self.expected
            && self.sample_count < expected
        {
            return Err(AnalysisError::Truncated {
                expected,
                received: self.sample_count,
            });
        }
        if self.in_column > 0 {
            self.flush_column();
        }
        WaveformPeaks::from_base(
            self.sample_rate,
            self.samples_per_peak,
            self.sample_count,
            self.bands,
        )
        .map_err(AnalysisError::Peaks)
    }

    fn flush_column(&mut self) {
        for band in 0..3 {
            // Values are finite and clamped, and at least one sample was seen,
            // so `from_f32` succeeds; the fallback is unreachable silence.
            let peak =
                Peak::from_f32(self.column_min[band], self.column_max[band]).unwrap_or(Peak::ZERO);
            self.bands[band].push(peak);
        }
        self.column_min = [f32::INFINITY; 3];
        self.column_max = [f32::NEG_INFINITY; 3];
        self.in_column = 0;
    }

    fn fail(&mut self, error: AnalysisError) -> Result<(), AnalysisError> {
        self.failed = Some(error);
        Err(error)
    }
}
