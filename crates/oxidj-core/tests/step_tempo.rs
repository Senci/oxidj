use oxidj_core::step_tempo::{StepEvidence, StepMap, StepSegment};

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
