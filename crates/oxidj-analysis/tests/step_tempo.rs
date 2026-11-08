use oxidj_analysis::step_tempo::{ResourceLimit, StepTempoBuilder};
use oxidj_core::step_tempo::{ComputedStepTempoV1, StepRefusal};
use taktus as _;

fn render(rate: u32, seconds: f64, hits: &[f64]) -> Vec<f32> {
    let mut out = vec![0.0; (seconds * f64::from(rate)).round() as usize];
    for &hit in hits {
        let start = (hit * f64::from(rate)).round() as usize;
        for i in 0..(0.25 * f64::from(rate)) as usize {
            let Some(slot) = out.get_mut(start + i) else {
                break;
            };
            let t = i as f64 / f64::from(rate);
            *slot += (0.6 * (std::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp()
                + 0.3 * (std::f64::consts::TAU * 1800.0 * t).sin() * (-t / 0.004).exp())
                as f32;
        }
    }
    out
}
fn truth(sections: &[(f64, f64)], end: f64) -> Vec<f64> {
    let mut hits = vec![];
    for (i, &(start, bpm)) in sections.iter().enumerate() {
        let stop = sections.get(i + 1).map_or(end, |s| s.0);
        hits.extend(
            (0..)
                .map(|n| start + n as f64 * 60.0 / bpm)
                .take_while(|&t| t < stop - 1e-8),
        );
    }
    hits
}
fn analyse(
    rate: u32,
    pcm: &[f32],
    partition: &[usize],
) -> Result<ComputedStepTempoV1, ResourceLimit> {
    let mut b = StepTempoBuilder::new(rate);
    let mut pos = 0;
    let mut i = 0;
    while pos < pcm.len() {
        let end = (pos + partition[i % partition.len()]).min(pcm.len());
        b.push(&[]);
        b.push(&pcm[pos..end]);
        pos = end;
        i += 1;
    }
    b.push(&[]);
    b.finish()
}
fn invariant_result(rate: u32, pcm: &[f32]) -> Result<ComputedStepTempoV1, ResourceLimit> {
    let expected = analyse(rate, pcm, &[pcm.len().max(1)]);
    for p in [&[1][..], &[7][..], &[4096][..], &[13, 509, 1, 4096][..]] {
        assert_eq!(
            analyse(rate, pcm, p),
            expected,
            "rate {rate}, partition {p:?}"
        );
    }
    expected
}
fn invariant(rate: u32, pcm: &[f32]) -> ComputedStepTempoV1 {
    invariant_result(rate, pcm).unwrap()
}
fn assert_truth(result: &ComputedStepTempoV1, sections: &[(f64, f64)], end: f64) {
    let map = result
        .map()
        .unwrap_or_else(|| panic!("expected map: {result:?}"));
    assert_eq!(map.segments().len(), sections.len());
    assert_eq!(
        matches!(result, ComputedStepTempoV1::Constant(_)),
        sections.len() == 1
    );
    let mut predicted = vec![];
    for (i, s) in map.segments().iter().enumerate() {
        assert!((s.bpm - sections[i].1).abs() <= 0.5, "{s:?}");
        assert!(
            (f64::from(s.start_ms) / 1000.0 - sections[i].0).abs() <= 0.03,
            "{s:?}"
        );
        let stop = map
            .segments()
            .get(i + 1)
            .map_or(f64::from(map.end_ms()), |n| f64::from(n.start_ms));
        predicted.extend(
            (0..)
                .map(|n| f64::from(s.start_ms) + n as f64 * 60000.0 / s.bpm)
                .take_while(|&t| t < stop - 0.001),
        );
        if let Some(next) = map.segments().get(i + 1) {
            assert!(
                (f64::from(next.start_ms - s.start_ms)
                    - f64::from(next.ordinal - s.ordinal) * 60000.0 / s.bpm)
                    .abs()
                    <= 1.0
            );
        }
    }
    let truth = truth(sections, end);
    assert!(
        predicted.len() <= truth.len(),
        "no extra predicted beats at either edge: {map:?}; predicted={predicted:?}; truth={truth:?}"
    );
    assert!(truth.len() - predicted.len() <= 1);
    for (p, t) in predicted.iter().zip(truth) {
        assert!((p - t * 1000.0).abs() <= 30.0, "beat {p} vs {t}");
    }
}
#[test]
fn positive_steps_and_exact_chunk_invariance() {
    for rate in [8000, 44100, 48000] {
        for (sections, end) in [
            (vec![(0.3, 120.0)], 40.3),
            (vec![(0.3, 120.0), (20.3, 150.0)], 40.3),
            (vec![(0.3, 120.0), (20.3, 150.0), (40.3, 120.0)], 60.3),
        ] {
            let pcm = render(rate, end, &truth(&sections, end));
            let result = invariant(rate, &pcm);
            assert_truth(&result, &sections, end);
        }
    }
}
#[test]
fn refused_content() {
    for rate in [8000, 44100] {
        let sections = [(0.3, 120.0)];
        let mut hits = truth(&sections, 40.3);
        hits.extend(truth(&[(0.3 + 0.5 / 3.0, 120.0)], 40.3));
        assert_eq!(
            invariant(rate, &render(rate, 40.3, &hits)),
            ComputedStepTempoV1::NoMap(StepRefusal::Ambiguous)
        );
        for value in [0.0, 0.4] {
            assert!(matches!(
                invariant(rate, &vec![value; rate as usize * 30]),
                ComputedStepTempoV1::NoMap(_)
            ));
        }
        for n in [0, 1, (f64::from(rate) * 7.9) as usize] {
            assert_eq!(
                invariant(
                    rate,
                    &render(rate, n as f64 / f64::from(rate), &truth(&sections, 7.9))
                ),
                ComputedStepTempoV1::NoMap(StepRefusal::Insufficient)
            );
        }
        let short = [(0.3, 120.0), (20.3, 150.0), (23.5, 120.0)];
        assert_eq!(
            invariant(rate, &render(rate, 43.5, &truth(&short, 43.5))),
            ComputedStepTempoV1::NoMap(StepRefusal::ShortSegment)
        );
        let many = [(0.3, 120.0), (20.3, 150.0), (40.3, 120.0), (60.3, 150.0)];
        assert_eq!(
            invariant(rate, &render(rate, 80.3, &truth(&many, 80.3))),
            ComputedStepTempoV1::NoMap(StepRefusal::UnsupportedModel)
        );
        for bpm in [90.0, 174.0] {
            assert!(matches!(
                invariant(rate, &render(rate, 40.3, &truth(&[(0.3, bpm)], 40.3))),
                ComputedStepTempoV1::NoMap(_)
            ));
        }
        let guard = [(0.3, 120.0), (20.3, 123.0)];
        assert!(matches!(
            invariant(rate, &render(rate, 40.3, &truth(&guard, 40.3))),
            ComputedStepTempoV1::NoMap(_)
        ));
        // Integral n = 2*x + x*x/160: solve positive quadratic root.
        let ramp: Vec<_> = (0..)
            .map(|n| 0.3 + (-2.0 + (4.0 + 4.0 * n as f64 / 160.0).sqrt()) * 80.0)
            .take_while(|&t| t < 40.3)
            .collect();
        assert!(matches!(
            invariant(rate, &render(rate, 40.3, &ramp)),
            ComputedStepTempoV1::NoMap(_)
        ));
    }
}
#[test]
fn resource_invalid_rate_and_nonfinite_precedence() {
    for rate in [0, 7999, 48001] {
        let mut b = StepTempoBuilder::new(rate);
        b.push(&[f32::NAN]);
        assert_eq!(
            b.finish(),
            Ok(ComputedStepTempoV1::NoMap(StepRefusal::UnsupportedRate))
        );
    }
    for rate in [8000, 44100] {
        let mut b = StepTempoBuilder::new(rate);
        for _ in 0..180 {
            b.push(&vec![0.0; rate as usize]);
        }
        assert!(b.buffered_onsets() <= 18000);
        assert!(b.finish().is_ok());
        let mut pcm = vec![0.0; rate as usize * 180 + 1];
        pcm[0] = f32::NAN;
        assert_eq!(invariant_result(rate, &pcm), Err(ResourceLimit));
        let good = render(rate, 40.3, &truth(&[(0.3, 120.0)], 40.3));
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for index in [0, good.len() / 2, good.len()] {
                let mut pcm = good.clone();
                if index == pcm.len() {
                    pcm.push(bad)
                } else {
                    pcm[index] = bad;
                }
                assert_eq!(
                    invariant(rate, &pcm),
                    ComputedStepTempoV1::NoMap(StepRefusal::NonFiniteInput)
                );
            }
        }
    }
    assert!(StepTempoBuilder::heap_bound_bytes() <= 8 * 1024 * 1024);
}
#[test]
fn seeded_noise_never_maps() {
    for rate in [8000, 44100] {
        for mut state in [
            0x9E37_79B9_7F4A_7C15u64,
            0x0123_4567_89AB_CDEF,
            0xDEAD_BEEF_CAFE_F00D,
        ] {
            let pcm: Vec<_> = (0..rate * 30)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state >> 40) as f32 / (1u64 << 24) as f32 - 0.5
                })
                .collect();
            assert!(matches!(
                invariant(rate, &pcm),
                ComputedStepTempoV1::NoMap(_)
            ));
        }
    }
}
