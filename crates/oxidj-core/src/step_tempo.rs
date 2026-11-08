//! Neutral, inspection-only step-tempo result; never a persisted `TempoGrid`.

/// Diagnostic values for one segment. Scores are not probabilities.
#[derive(Clone, Debug, PartialEq)]
pub struct StepEvidence {
    pub sure: f64,
    pub catch: f64,
    pub max_residual_ms: f64,
    pub supported_fraction: f64,
    pub unexplained_fraction: f64,
}

/// A segment's first beat, relative to decoded frame zero.
#[derive(Clone, Debug, PartialEq)]
pub struct StepSegment {
    pub start_ms: u32,
    pub ordinal: u32,
    pub bpm: f64,
    pub evidence: StepEvidence,
}

/// A validated step map, whose end is exclusive.
#[derive(Clone, Debug, PartialEq)]
pub struct StepMap {
    sample_rate: u32,
    frames: u64,
    end_ms: u32,
    segments: Vec<StepSegment>,
}

/// A failed map invariant; the estimator must not expose a partial map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidStepMap;

impl StepMap {
    /// Validate bounds, evidence and beat-continuous joins.
    pub fn new(
        sample_rate: u32,
        frames: u64,
        segments: Vec<StepSegment>,
    ) -> Result<Self, InvalidStepMap> {
        if !(8_000..=48_000).contains(&sample_rate)
            || frames < 8 * u64::from(sample_rate)
            || frames > 180 * u64::from(sample_rate)
            || !(1..=3).contains(&segments.len())
        {
            return Err(InvalidStepMap);
        }
        let end_ms = (frames * 1000 / u64::from(sample_rate)) as u32;
        if segments[0].ordinal != 0 {
            return Err(InvalidStepMap);
        }
        for (i, s) in segments.iter().enumerate() {
            let e = &s.evidence;
            if !(100.0..=160.0).contains(&s.bpm)
                || !e.sure.is_finite()
                || e.sure < 0.05
                || !e.catch.is_finite()
                || !e.max_residual_ms.is_finite()
                || !(0.0..=30.0).contains(&e.max_residual_ms)
                || !e.supported_fraction.is_finite()
                || !(0.9..=1.0).contains(&e.supported_fraction)
                || !e.unexplained_fraction.is_finite()
                || !(0.0..=0.1).contains(&e.unexplained_fraction)
            {
                return Err(InvalidStepMap);
            }
            let end = segments.get(i + 1).map_or(end_ms, |next| next.start_ms);
            let Some(span) = end.checked_sub(s.start_ms) else {
                return Err(InvalidStepMap);
            };
            if span < 8_000 || f64::from(span) * s.bpm / 60_000.0 < 12.0 {
                return Err(InvalidStepMap);
            }
            if let Some(next) = segments.get(i + 1) {
                let Some(count) = next.ordinal.checked_sub(s.ordinal) else {
                    return Err(InvalidStepMap);
                };
                if count == 0
                    || (f64::from(span) - f64::from(count) * 60_000.0 / s.bpm).abs() > 1.0
                    || !next.bpm.is_finite()
                    || (next.bpm - s.bpm).abs() / next.bpm.min(s.bpm) < 0.05
                {
                    return Err(InvalidStepMap);
                }
            }
        }
        Ok(Self {
            sample_rate,
            frames,
            end_ms,
            segments,
        })
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn frames(&self) -> u64 {
        self.frames
    }
    pub fn end_ms(&self) -> u32 {
        self.end_ms
    }
    pub fn segments(&self) -> &[StepSegment] {
        &self.segments
    }
}

/// Why analysis declined to offer any map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRefusal {
    Insufficient,
    Silent,
    Ambiguous,
    ShortSegment,
    UnsupportedModel,
    UnsupportedRate,
    NonFiniteInput,
}

/// Separate result; it has no conversion to the existing constant grid.
#[derive(Clone, Debug, PartialEq)]
pub enum ComputedStepTempoV1 {
    Constant(StepMap),
    Variable(StepMap),
    NoMap(StepRefusal),
}
