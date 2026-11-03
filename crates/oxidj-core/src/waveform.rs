//! Neutral 3-band waveform peak model (W1).
//!
//! A [`WaveformPeaks`] holds fixed-rate min/max peaks for three frequency
//! bands (low / mid / high) of a mono signal, quantized to `u8`, plus a
//! mipmap of coarser levels built by 2x decimation. It is pure data: no I/O,
//! no DSP, no dependencies. Band splitting lives in `oxidj-analysis`.
//!
//! The mono overview is *derived* ([`WaveformPeaks::mono`]) rather than
//! stored: it is the per-column envelope union of the three bands.

/// One frequency band of a [`WaveformPeaks`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Band {
    /// Below the low/mid crossover (~200 Hz).
    Low,
    /// Between the crossovers (~200 Hz to ~2.5 kHz).
    Mid,
    /// Above the mid/high crossover (~2.5 kHz).
    High,
}

impl Band {
    /// All bands in storage order.
    pub const ALL: [Self; 3] = [Self::Low, Self::Mid, Self::High];

    const fn index(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Mid => 1,
            Self::High => 2,
        }
    }
}

/// A quantized min/max pair. `min <= max` always holds.
///
/// Quantization maps a sample in `[-1.0, 1.0]` linearly onto `0..=255`
/// (128 is approximately zero); out-of-range samples are clamped. The mapping
/// is monotonic, so min/max of quantized values equals quantized min/max.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Peak {
    min: u8,
    max: u8,
}

impl Peak {
    /// The silent (zero-amplitude) peak.
    pub const ZERO: Self = Self { min: 128, max: 128 };

    /// Build a peak from quantized bounds, rejecting `min > max`.
    pub const fn new(min: u8, max: u8) -> Option<Self> {
        if min > max {
            None
        } else {
            Some(Self { min, max })
        }
    }

    /// Quantize a float min/max pair. Returns `None` for non-finite input or
    /// `min > max`.
    pub fn from_f32(min: f32, max: f32) -> Option<Self> {
        if !min.is_finite() || !max.is_finite() || min > max {
            return None;
        }
        Self::new(quantize(min), quantize(max))
    }

    /// Quantized lower bound.
    pub const fn min(self) -> u8 {
        self.min
    }

    /// Quantized upper bound.
    pub const fn max(self) -> u8 {
        self.max
    }

    /// Smallest peak enclosing both.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

/// Quantize one finite sample to `u8` (see [`Peak`]). Non-finite input maps to
/// the zero level (128); callers should reject it before quantizing.
pub fn quantize(sample: f32) -> u8 {
    if !sample.is_finite() {
        return 128;
    }
    let scaled = (sample.clamp(-1.0, 1.0) + 1.0) * 127.5;
    // In range 0.0..=255.0 after clamping, so the cast cannot saturate.
    scaled.round() as u8
}

/// Why a [`WaveformPeaks`] could not be constructed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaveformError {
    /// Sample rate was zero.
    ZeroSampleRate,
    /// Samples-per-peak was zero.
    ZeroSamplesPerPeak,
    /// No samples / no columns.
    Empty,
    /// The three bands have different column counts.
    BandLengthMismatch,
    /// Column count does not equal `ceil(sample_count / samples_per_peak)`.
    ColumnCountMismatch,
}

impl core::fmt::Display for WaveformError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ZeroSampleRate => "sample rate is zero",
            Self::ZeroSamplesPerPeak => "samples per peak is zero",
            Self::Empty => "waveform has no samples",
            Self::BandLengthMismatch => "band peak counts differ",
            Self::ColumnCountMismatch => "peak count does not match sample count",
        })
    }
}

impl std::error::Error for WaveformError {}

/// Fixed-rate 3-band min/max peaks with 2x-decimation mipmap levels.
///
/// Level 0 has one column per `samples_per_peak` input samples (the last
/// column may cover fewer). Level `n + 1` has `ceil(len_n / 2)` columns, each
/// the [`Peak::union`] of its two (or, at an odd tail, one) children. The last
/// level has exactly one column.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WaveformPeaks {
    sample_rate: u32,
    samples_per_peak: u32,
    sample_count: u64,
    /// `levels[level][band][column]`.
    levels: Vec<[Vec<Peak>; 3]>,
}

impl WaveformPeaks {
    /// Validate level-0 peaks and build the mipmap.
    pub fn from_base(
        sample_rate: u32,
        samples_per_peak: u32,
        sample_count: u64,
        bands: [Vec<Peak>; 3],
    ) -> Result<Self, WaveformError> {
        if sample_rate == 0 {
            return Err(WaveformError::ZeroSampleRate);
        }
        if samples_per_peak == 0 {
            return Err(WaveformError::ZeroSamplesPerPeak);
        }
        if sample_count == 0 {
            return Err(WaveformError::Empty);
        }
        let len = bands[0].len();
        if bands.iter().any(|b| b.len() != len) {
            return Err(WaveformError::BandLengthMismatch);
        }
        let expected = sample_count.div_ceil(u64::from(samples_per_peak));
        if u64::try_from(len).ok() != Some(expected) {
            return Err(WaveformError::ColumnCountMismatch);
        }
        let mut levels = vec![bands];
        while let Some(last) = levels.last()
            && last[0].len() > 1
        {
            let next: [Vec<Peak>; 3] = core::array::from_fn(|band| {
                last[band]
                    .chunks(2)
                    // `chunks(2)` never yields an empty slice.
                    .map(|pair| pair.iter().fold(pair[0], |acc, peak| acc.union(*peak)))
                    .collect()
            });
            levels.push(next);
        }
        Ok(Self {
            sample_rate,
            samples_per_peak,
            sample_count,
            levels,
        })
    }

    /// Input sample rate in Hz.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Input samples covered by one level-0 column.
    pub const fn samples_per_peak(&self) -> u32 {
        self.samples_per_peak
    }

    /// Total mono input samples.
    pub const fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Number of mipmap levels (level 0 included); always at least 1.
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Peaks for `band` at mipmap `level`, or `None` if the level is absent.
    pub fn band(&self, level: usize, band: Band) -> Option<&[Peak]> {
        self.levels.get(level).map(|l| l[band.index()].as_slice())
    }

    /// Derived mono envelope at `level`: per column, the union of the three
    /// band peaks. Not stored.
    pub fn mono(&self, level: usize) -> Option<Vec<Peak>> {
        let [low, mid, high] = self.levels.get(level)?;
        Some(
            low.iter()
                .zip(mid)
                .zip(high)
                .map(|((l, m), h)| l.union(*m).union(*h))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(min: u8, max: u8) -> Peak {
        Peak::new(min, max).unwrap()
    }

    #[test]
    fn quantize_is_monotonic_and_clamped() {
        assert_eq!(quantize(-1.0), 0);
        assert_eq!(quantize(1.0), 255);
        assert_eq!(quantize(-5.0), 0);
        assert_eq!(quantize(5.0), 255);
        assert_eq!(quantize(f32::NAN), 128);
        let mut prev = 0;
        for i in -1000..=1000 {
            let q = quantize(i as f32 / 1000.0);
            assert!(q >= prev);
            prev = q;
        }
        assert!(Peak::from_f32(f32::NAN, 0.0).is_none());
        assert!(Peak::from_f32(0.5, 0.0).is_none());
        assert!(Peak::new(3, 2).is_none());
    }

    #[test]
    fn mip_levels_are_union_of_children_down_to_one_column() {
        let base: Vec<Peak> = (0..5u8).map(|i| p(100 - i * 10, 130 + i * 20)).collect();
        let peaks =
            WaveformPeaks::from_base(44_100, 10, 45, [base.clone(), base.clone(), base]).unwrap();
        assert_eq!(peaks.level_count(), 4); // 5 -> 3 -> 2 -> 1
        for level in 1..peaks.level_count() {
            for band in Band::ALL {
                let parent = peaks.band(level, band).unwrap();
                let child = peaks.band(level - 1, band).unwrap();
                assert_eq!(parent.len(), child.len().div_ceil(2));
                for (i, peak) in parent.iter().enumerate() {
                    let kids = &child[i * 2..(i * 2 + 2).min(child.len())];
                    assert_eq!(peak.min(), kids.iter().map(|k| k.min()).min().unwrap());
                    assert_eq!(peak.max(), kids.iter().map(|k| k.max()).max().unwrap());
                }
            }
        }
        assert_eq!(peaks.band(3, Band::Low).unwrap(), &[p(60, 210)]);
        assert!(peaks.band(4, Band::Low).is_none());
    }

    #[test]
    fn mono_is_union_of_bands() {
        let peaks = WaveformPeaks::from_base(
            48_000,
            4,
            4,
            [vec![p(120, 130)], vec![p(100, 140)], vec![p(125, 200)]],
        )
        .unwrap();
        assert_eq!(peaks.mono(0).unwrap(), vec![p(100, 200)]);
    }

    #[test]
    fn invalid_shapes_are_refused() {
        let one = || vec![p(128, 128)];
        let ok = |sr, spp, n, b| WaveformPeaks::from_base(sr, spp, n, b);
        assert_eq!(
            ok(0, 1, 1, [one(), one(), one()]),
            Err(WaveformError::ZeroSampleRate)
        );
        assert_eq!(
            ok(1, 0, 1, [one(), one(), one()]),
            Err(WaveformError::ZeroSamplesPerPeak)
        );
        assert_eq!(
            ok(1, 1, 0, [vec![], vec![], vec![]]),
            Err(WaveformError::Empty)
        );
        assert_eq!(
            ok(1, 1, 1, [one(), vec![], one()]),
            Err(WaveformError::BandLengthMismatch)
        );
        assert_eq!(
            ok(1, 1, 2, [one(), one(), one()]),
            Err(WaveformError::ColumnCountMismatch)
        );
        assert_eq!(ok(1, 4, 4, [one(), one(), one()]).unwrap().level_count(), 1);
    }
}
