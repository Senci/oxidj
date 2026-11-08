use oxidj_core::step_tempo::{ComputedStepTempoV1, StepEvidence, StepMap, StepSegment};

fn segment(start_ms: u32, ordinal: u32, bpm: f64) -> StepSegment {
    StepSegment {
        start_ms,
        ordinal,
        bpm,
        evidence: StepEvidence {
            sure: 0.1,
            catch: 1.0,
            max_residual_ms: 10.0,
            supported_fraction: 1.0,
            unexplained_fraction: 0.0,
        },
    }
}

#[test]
fn classification_is_derived_from_validated_segment_count() {
    for parts in [
        vec![segment(300, 0, 120.0)],
        vec![segment(300, 0, 120.0), segment(20300, 40, 150.0)],
    ] {
        let constant = parts.len() == 1;
        let map = StepMap::new(8000, 322400, parts).unwrap();
        assert_eq!(map.representation(), "computed-step-tempo-v1");
        assert_eq!(map.bpm_band(), (100, 160));
        let result = ComputedStepTempoV1::from_map(map);
        assert_eq!(matches!(result, ComputedStepTempoV1::Constant(_)), constant);
        assert_eq!(
            matches!(result, ComputedStepTempoV1::Variable(_)),
            !constant
        );
    }
    assert!(StepMap::new(8000, 322400, vec![]).is_err());
    assert!(
        StepMap::new(
            8000,
            8000 * 180,
            vec![
                segment(300, 0, 120.0),
                segment(20300, 40, 150.0),
                segment(40300, 90, 120.0),
                segment(60300, 130, 150.0)
            ]
        )
        .is_err()
    );
}
#[test]
fn validates_step_boundaries_and_evidence() {
    let parts = vec![segment(300, 0, 120.0), segment(20_300, 40, 150.0)];
    assert!(StepMap::new(8_000, 322_400, parts.clone()).is_ok());
    let mut bad = parts.clone();
    bad[1].start_ms += 2;
    assert!(StepMap::new(8_000, 322_400, bad).is_err());
    let mut bad = parts;
    bad[0].evidence.sure = f64::NAN;
    assert!(StepMap::new(8_000, 322_400, bad).is_err());
}
