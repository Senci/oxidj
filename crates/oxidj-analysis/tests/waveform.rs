//! W1 acceptance tests for the streaming 3-band waveform builder.

use oxidj_analysis::{
    AnalysisError, Crossover, PEAKS_PER_SECOND, WaveformBuilder, samples_per_peak,
};
use oxidj_core::waveform::{Band, Peak, WaveformPeaks};
use taktus as _;

const RATE: u32 = 44_100;

fn sine(freq: f64, amplitude: f64, seconds: f64) -> Vec<f32> {
    let n = (f64::from(RATE) * seconds) as usize;
    (0..n)
        .map(|i| {
            (amplitude * (core::f64::consts::TAU * freq * i as f64 / f64::from(RATE)).sin()) as f32
        })
        .collect()
}

fn build(samples: &[f32]) -> WaveformPeaks {
    let mut builder = WaveformBuilder::new(RATE, Some(samples.len() as u64)).unwrap();
    builder.push(samples).unwrap();
    builder.finish().unwrap()
}

/// Largest half peak-to-peak (quantized units) in `band`, skipping the first
/// `skip` columns so the filters have settled.
fn amplitude(peaks: &WaveformPeaks, band: Band, skip: usize) -> u32 {
    peaks.band(3, band).unwrap()[skip..]
        .iter()
        .map(|p| u32::from(p.max() - p.min()))
        .max()
        .unwrap()
}

#[test]
fn sine_lands_in_its_band() {
    let skip = PEAKS_PER_SECOND as usize / 16; // 0.5 s at mip level 3
    for (freq, dominant) in [
        (60.0, Band::Low),
        (900.0, Band::Mid),
        (10_000.0, Band::High),
    ] {
        let peaks = build(&sine(freq, 0.8, 2.0));
        let main = amplitude(&peaks, dominant, skip);
        assert!(
            main > 150,
            "{freq} Hz: {dominant:?} amplitude {main} too small"
        );
        for other in Band::ALL.into_iter().filter(|b| *b != dominant) {
            let leak = amplitude(&peaks, other, skip);
            assert!(
                main > 3 * leak,
                "{freq} Hz: {dominant:?}={main} not dominant over {other:?}={leak}"
            );
        }
    }
}

#[test]
fn level0_matches_naive_full_buffer_reference() {
    // A mix of all three bands plus a non-multiple length (partial last column).
    let a = sine(55.0, 0.4, 1.31);
    let b = sine(1_000.0, 0.3, 1.31);
    let c = sine(7_000.0, 0.25, 1.31);
    let samples: Vec<f32> = (0..a.len()).map(|i| a[i] + b[i] + c[i]).collect();
    let spp = samples_per_peak(RATE) as usize;
    assert_eq!(spp, 37);
    assert_ne!(samples.len() % spp, 0);

    let mut crossover = Crossover::new(RATE).unwrap();
    let split: Vec<[f32; 3]> = samples.iter().map(|&s| crossover.split(s)).collect();
    let reference: Vec<Vec<Peak>> = (0..3)
        .map(|band| {
            split
                .chunks(spp)
                .map(|column| {
                    let values = column.iter().map(|v| v[band]);
                    let min = values.clone().fold(f32::INFINITY, f32::min);
                    let max = values.fold(f32::NEG_INFINITY, f32::max);
                    Peak::from_f32(min, max).unwrap()
                })
                .collect()
        })
        .collect();

    let peaks = build(&samples);
    assert_eq!(peaks.sample_rate(), RATE);
    assert_eq!(peaks.sample_count(), samples.len() as u64);
    for (i, band) in Band::ALL.into_iter().enumerate() {
        assert_eq!(
            peaks.band(0, band).unwrap(),
            reference[i].as_slice(),
            "{band:?}"
        );
    }
}

#[test]
fn mip_levels_are_min_max_of_children() {
    let peaks = build(&sine(440.0, 0.7, 3.1));
    assert!(peaks.level_count() > 5);
    let last = peaks.level_count() - 1;
    for band in Band::ALL {
        assert_eq!(peaks.band(last, band).unwrap().len(), 1);
        for level in 1..peaks.level_count() {
            let parent = peaks.band(level, band).unwrap();
            let child = peaks.band(level - 1, band).unwrap();
            assert_eq!(parent.len(), child.len().div_ceil(2));
            for (i, p) in parent.iter().enumerate() {
                let kids = &child[2 * i..(2 * i + 2).min(child.len())];
                assert_eq!(p.min(), kids.iter().map(|k| k.min()).min().unwrap());
                assert_eq!(p.max(), kids.iter().map(|k| k.max()).max().unwrap());
            }
        }
    }
}

#[test]
fn any_push_split_gives_identical_output() {
    let a = sine(80.0, 0.5, 1.0);
    let b = sine(3_000.0, 0.4, 1.0);
    let samples: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + y).collect();
    let whole = build(&samples);

    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for max_chunk in [1usize, 2, 7, 293, 294, 295, 4_096] {
        let mut builder = WaveformBuilder::new(RATE, Some(samples.len() as u64)).unwrap();
        let mut rest = samples.as_slice();
        while !rest.is_empty() {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let len = ((state >> 33) as usize % (max_chunk + 1)).min(rest.len()); // may be 0
            let (head, tail) = rest.split_at(len);
            builder.push(head).unwrap();
            rest = tail;
        }
        assert_eq!(builder.finish().unwrap(), whole, "max_chunk {max_chunk}");
    }
}

#[test]
fn bad_input_is_refused() {
    assert_eq!(
        WaveformBuilder::new(0, None).err(),
        Some(AnalysisError::ZeroSampleRate)
    );
    assert_eq!(
        WaveformBuilder::new(4_000, None).err(),
        Some(AnalysisError::UnsupportedSampleRate(4_000))
    );

    // Empty: no push, or only empty pushes.
    assert_eq!(
        WaveformBuilder::new(RATE, None).unwrap().finish(),
        Err(AnalysisError::Empty)
    );
    let mut b = WaveformBuilder::new(RATE, None).unwrap();
    b.push(&[]).unwrap();
    assert_eq!(b.finish(), Err(AnalysisError::Empty));

    // Truncated / overrun against the declared length.
    let mut b = WaveformBuilder::new(RATE, Some(1_000)).unwrap();
    b.push(&[0.1; 999]).unwrap();
    assert_eq!(
        b.finish(),
        Err(AnalysisError::Truncated {
            expected: 1_000,
            received: 999
        })
    );
    let mut b = WaveformBuilder::new(RATE, Some(1_000)).unwrap();
    b.push(&[0.1; 600]).unwrap();
    assert_eq!(
        b.push(&[0.1; 401]),
        Err(AnalysisError::TooManySamples { expected: 1_000 })
    );
    assert_eq!(
        b.finish(),
        Err(AnalysisError::TooManySamples { expected: 1_000 })
    );

    // NaN / infinities fail closed and poison the builder.
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut b = WaveformBuilder::new(RATE, None).unwrap();
        b.push(&[0.0; 10]).unwrap();
        assert_eq!(
            b.push(&[0.1, bad, 0.2]),
            Err(AnalysisError::NonFiniteSample)
        );
        assert_eq!(b.push(&[0.0; 10]), Err(AnalysisError::NonFiniteSample));
        assert_eq!(b.finish(), Err(AnalysisError::NonFiniteSample));
    }
}

#[test]
fn extreme_finite_input_does_not_panic() {
    let mut samples = vec![f32::MAX, f32::MIN, f32::MIN_POSITIVE, -0.0];
    samples.extend(std::iter::repeat_n(f32::MAX, 1_000));
    let peaks = build(&samples);
    assert_eq!(
        peaks.band(0, Band::Low).unwrap().len(),
        1_004usize.div_ceil(samples_per_peak(RATE) as usize)
    );
    // Exactly one sample: one column, one level.
    let one = build(&[0.5]);
    assert_eq!(one.level_count(), 1);
    assert_eq!(one.mono(0).unwrap().len(), 1);
}
