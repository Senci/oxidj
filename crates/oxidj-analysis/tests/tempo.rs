//! B2a tempo/beat-grid acceptance tests on synthetic, exactly known signals.

use taktus as _;

use oxidj_analysis::tempo::{
    BpmRange, MIN_TEMPO_SECONDS, ONSETS_PER_SECOND, TEMPO_ALGORITHM, TempoBuilder, TempoError,
    TempoGate, TempoOptions,
};
use oxidj_core::tempo::TempoGrid;

/// First generator beat, seconds.
const T0: f64 = 0.3;

/// A decaying 55 Hz kick plus a short 1.8 kHz click at each time in `hits`.
fn render(rate: u32, seconds: f64, hits: &[f64]) -> Vec<f32> {
    let hits: Vec<(f64, f64, f64)> = hits.iter().map(|&t| (t, 0.6, 0.3)).collect();
    render_mix(rate, seconds, &hits)
}

/// `(time, kick level, click level)` hits.
fn render_mix(rate: u32, seconds: f64, hits: &[(f64, f64, f64)]) -> Vec<f32> {
    let n = (seconds * f64::from(rate)) as usize;
    let mut out = vec![0.0f32; n];
    let len = (0.25 * f64::from(rate)) as usize;
    for &(hit, kick_level, click_level) in hits {
        let start = (hit * f64::from(rate)).round() as usize;
        for i in 0..len {
            let Some(slot) = out.get_mut(start + i) else {
                break;
            };
            let t = i as f64 / f64::from(rate);
            let kick = (core::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp();
            let click = (core::f64::consts::TAU * 1_800.0 * t).sin() * (-t / 0.004).exp();
            *slot += (kick_level * kick + click_level * click) as f32;
        }
    }
    out
}

fn beat_times(bpm: f64, seconds: f64) -> Vec<f64> {
    let period = 60.0 / bpm;
    (0..)
        .map(|k| T0 + k as f64 * period)
        .take_while(|t| *t < seconds)
        .collect()
}

fn analyse(
    rate: u32,
    samples: &[f32],
    chunk: usize,
    options: TempoOptions,
) -> Result<Option<TempoGrid>, TempoError> {
    let mut builder = TempoBuilder::new(rate, options);
    for part in samples.chunks(chunk.max(1)) {
        builder.push(part);
    }
    builder.finish()
}

fn grid(rate: u32, samples: &[f32]) -> Option<TempoGrid> {
    analyse(rate, samples, 4096, TempoOptions::default()).unwrap()
}

/// Deterministic white noise in [-0.5, 0.5).
fn noise(n: usize, mut state: u64) -> Vec<f32> {
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u64 << 24) as f32 - 0.5
        })
        .collect()
}

/// BPM within 0.5, every grid beat within 30 ms of a generator beat (so the
/// first-beat phase too), even spacing, and the beat count of the span.
fn assert_grid_matches(grid: &TempoGrid, bpm: f64, seconds: f64) {
    assert!(
        (grid.bpm() - bpm).abs() <= 0.5,
        "bpm {} vs {bpm}",
        grid.bpm()
    );
    assert_eq!(grid.algorithm(), TEMPO_ALGORITHM);
    assert!((0.0..=1.0).contains(&grid.confidence()));
    let truth = beat_times(bpm, seconds);
    let beats = grid.beats_ms();
    let off = |ms: u32| {
        truth
            .iter()
            .map(|t| (f64::from(ms) / 1000.0 - t).abs())
            .fold(f64::INFINITY, f64::min)
    };
    for &ms in beats {
        assert!(
            off(ms) <= 0.030,
            "beat {ms} ms is {:.1} ms off the generator",
            off(ms) * 1000.0
        );
    }
    let period_ms = 60_000.0 / bpm;
    assert!(
        f64::from(beats[0]) / 1000.0 <= T0 + 60.0 / bpm + 0.030,
        "first beat {} ms is late",
        beats[0]
    );
    for pair in beats.windows(2) {
        let gap = f64::from(pair[1] - pair[0]);
        assert!(
            (gap - period_ms).abs() <= 30.0,
            "gap {gap} ms vs period {period_ms:.1} ms"
        );
    }
    // One grid beat per generator beat between the first and last grid beat.
    let (first, last) = (
        f64::from(beats[0]) / 1000.0,
        f64::from(*beats.last().unwrap()) / 1000.0,
    );
    let expected = truth
        .iter()
        .filter(|t| **t >= first - 0.03 && **t <= last + 0.03)
        .count();
    assert_eq!(beats.len(), expected);
    // The grid covers the track, give or take a beat at either end.
    assert!(
        last >= truth.last().unwrap() - 60.0 / bpm - 0.03,
        "grid ends early at {last:.2} s"
    );
}

#[test]
fn click_tracks_at_dance_tempi() {
    let (rate, seconds) = (44_100, 30.0);
    for bpm in [90.0, 128.0, 140.0, 174.0] {
        let samples = render(rate, seconds, &beat_times(bpm, seconds));
        let grid = grid(rate, &samples).unwrap_or_else(|| panic!("{bpm} BPM: no grid"));
        assert_grid_matches(&grid, bpm, seconds);
    }
}

#[test]
fn low_rate_8k_sixty_seconds() {
    let (rate, seconds, bpm) = (8_000, 60.0, 124.0);
    let samples = render(rate, seconds, &beat_times(bpm, seconds));
    let grid = grid(rate, &samples).expect("8 kHz grid");
    assert_grid_matches(&grid, bpm, seconds);
}

fn gated_noise_is_refused(gate: TempoGate) -> bool {
    let options = TempoOptions {
        gate,
        ..TempoOptions::default()
    };
    [
        0x9E37_79B9_7F4A_7C15,
        0x0123_4567_89AB_CDEF,
        0xDEAD_BEEF_CAFE_F00D,
    ]
    .iter()
    .all(|&seed| {
        analyse(44_100, &noise(44_100 * 30, seed), 4096, options)
            .unwrap()
            .is_none()
    })
}

#[test]
fn white_noise_fails_the_gate() {
    assert!(gated_noise_is_refused(TempoGate::DEFAULT));
}

/// Mutation control: with the gate thresholds corrupted to zero, the noise
/// test above would fail — taktus reads noise as a (weak) tempo.
#[test]
fn mutation_control_zero_gate_lets_noise_through() {
    assert!(!gated_noise_is_refused(TempoGate {
        min_sure: 0.0,
        min_catch: 0.0
    }));
}

#[test]
fn swung_eighths_keep_the_quarter_note_grid() {
    // A kick on each beat and a lighter hi-hat click two thirds through it
    // (triplet swing), 120 BPM: the grid stays on the kicks.
    let (rate, seconds, period) = (44_100, 30.0, 0.5);
    let hits: Vec<(f64, f64, f64)> = beat_times(120.0, seconds)
        .into_iter()
        .flat_map(|t| [(t, 0.6, 0.3), (t + 2.0 * period / 3.0, 0.0, 0.15)])
        .collect();
    let grid = grid(rate, &render_mix(rate, seconds, &hits)).expect("swung track has a beat");
    assert_grid_matches(&grid, 120.0, seconds);
}

#[test]
fn unaccented_swing_is_read_but_its_phase_is_ambiguous() {
    // Equal hits on the beat and two thirds through it: the exact outcome is a
    // 120 BPM grid (period right) whose beats sit on the swung hits — the
    // signal gives no accent to tell them apart. Pinned so a change is seen.
    let (rate, seconds, period) = (44_100, 30.0, 0.5);
    let hits: Vec<f64> = beat_times(120.0, seconds)
        .into_iter()
        .flat_map(|t| [t, t + 2.0 * period / 3.0])
        .collect();
    let grid = grid(rate, &render(rate, seconds, &hits)).expect("a reading");
    assert!((grid.bpm() - 120.0).abs() <= 0.5, "{}", grid.bpm());
    let swung_phase = (T0 + 2.0 * period / 3.0) * 1000.0;
    let phase = f64::from(grid.beats_ms()[0]) % (period * 1000.0);
    assert!(
        (phase - swung_phase % 500.0).abs() <= 30.0,
        "phase {phase} ms"
    );
}

#[test]
fn degenerate_inputs_return_none_without_panicking() {
    let rate = 44_100;
    let n = rate as usize * 30;
    let mut nan = render(rate, 30.0, &beat_times(128.0, 30.0));
    nan[n / 2] = f32::NAN;
    let mut inf = render(rate, 30.0, &beat_times(128.0, 30.0));
    inf[n / 3] = f32::INFINITY;
    let mut neg_inf = render(rate, 30.0, &beat_times(128.0, 30.0));
    neg_inf[0] = f32::NEG_INFINITY;
    let cases: Vec<(&str, u32, Vec<f32>)> = vec![
        ("empty", rate, vec![]),
        ("one sample", rate, vec![0.5]),
        ("0.1 s", rate, render(rate, 0.1, &[0.0])),
        (
            "just under the minimum",
            rate,
            render(
                rate,
                MIN_TEMPO_SECONDS as f64 - 0.2,
                &beat_times(128.0, 5.0),
            ),
        ),
        ("silence", rate, vec![0.0; n]),
        ("DC", rate, vec![0.4; n]),
        ("NaN", rate, nan),
        ("inf", rate, inf),
        ("-inf", rate, neg_inf),
    ];
    for (name, rate, samples) in cases {
        for chunk in [1, 4096, samples.len().max(1)] {
            if chunk == 1 && samples.len() > 100_000 {
                continue;
            }
            assert_eq!(
                analyse(rate, &samples, chunk, TempoOptions::default()),
                Ok(None),
                "{name}"
            );
        }
    }
    let good = render(rate, 30.0, &beat_times(128.0, 30.0));
    for bad_rate in [0, 1, 7_999] {
        assert_eq!(
            analyse(bad_rate, &good, 4096, TempoOptions::default()),
            Ok(None),
            "rate {bad_rate}"
        );
    }
}

#[test]
fn chunk_size_does_not_change_the_grid() {
    let rate = 44_100;
    let samples = render(rate, 25.0, &beat_times(128.0, 25.0));
    let whole = analyse(rate, &samples, samples.len(), TempoOptions::default()).unwrap();
    assert!(whole.is_some());
    for chunk in [1, 7, 4096] {
        assert_eq!(
            analyse(rate, &samples, chunk, TempoOptions::default()).unwrap(),
            whole,
            "chunk {chunk}"
        );
    }
}

#[test]
fn onset_buffer_is_bounded() {
    let rate = 44_100;
    let options = TempoOptions {
        max_seconds: 10,
        ..TempoOptions::default()
    };
    let mut builder = TempoBuilder::new(rate, options);
    assert_eq!(builder.max_onsets(), 10 * ONSETS_PER_SECOND);
    let one_second = render(rate, 1.0, &[0.1, 0.6]);
    for second in 1..=10 {
        builder.push(&one_second);
        // About 100 onset values a second, never the audio itself.
        assert!(builder.buffered_onsets() <= second * ONSETS_PER_SECOND);
        assert!(builder.buffered_onsets() + 10 >= second * ONSETS_PER_SECOND);
    }
    builder.push(&one_second);
    builder.push(&one_second);
    assert_eq!(
        builder.buffered_onsets(),
        0,
        "buffer released once over the cap"
    );
    assert_eq!(
        builder.finish(),
        Err(TempoError::TooLong { max_seconds: 10 })
    );
}

#[test]
fn octave_fold_into_the_range() {
    let (rate, seconds) = (44_100, 30.0);
    let samples = render(rate, seconds, &beat_times(190.0, seconds));
    // Default 70-180: 190 BPM is folded to 95 and tracked at that level.
    let folded = grid(rate, &samples).expect("folded grid");
    assert!((folded.bpm() - 95.0).abs() <= 0.5, "{}", folded.bpm());
    let options = TempoOptions {
        range: BpmRange::new(95.0, 200.0).unwrap(),
        ..TempoOptions::default()
    };
    let wide = analyse(rate, &samples, 4096, options)
        .unwrap()
        .expect("unfolded grid");
    assert_grid_matches(&wide, 190.0, seconds);
    // Every folded beat is a generator beat; the folded grid has about half as many.
    let truth = beat_times(190.0, seconds);
    for &ms in folded.beats_ms() {
        let off = truth
            .iter()
            .map(|t| (f64::from(ms) / 1000.0 - t).abs())
            .fold(f64::INFINITY, f64::min);
        assert!(off <= 0.030, "folded beat {ms} ms off by {off}");
    }
    let half = wide.beats_ms().len() / 2;
    assert!(folded.beats_ms().len().abs_diff(half) <= 1);
    // A slow reading is doubled into the default range.
    let slow = render(rate, seconds, &beat_times(120.0, seconds));
    let options = TempoOptions {
        range: BpmRange::new(30.0, 65.0).unwrap(),
        ..TempoOptions::default()
    };
    let g = analyse(rate, &slow, 4096, options)
        .unwrap()
        .expect("slow-range grid");
    assert!((g.bpm() - 60.0).abs() <= 0.5, "{}", g.bpm());
}
