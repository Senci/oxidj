//! Explicit-file step-tempo inspection: decode only, never opens a catalog.
use oxidj_analysis::step_tempo::{ResourceLimit, StepTempoBuilder};
pub use oxidj_analysis::step_tempo::{STEP_ALGORITHM, STEP_POLICY};
pub use oxidj_core::step_tempo::{ComputedStepTempoV1, StepRefusal};
use oxidj_library::pcm::{MonoSink, PcmError, PcmFormat, decode_mono};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepInspectionError {
    Unreadable,
    UnsupportedAudio,
    MalformedAudio,
    ResourceLimit,
}
impl std::fmt::Display for StepInspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "source audio could not be read",
            Self::UnsupportedAudio => "unsupported audio format",
            Self::MalformedAudio => "corrupt or truncated audio",
            Self::ResourceLimit => "step-tempo ResourceLimit",
        })
    }
}
impl std::error::Error for StepInspectionError {}
struct Sink(Option<StepTempoBuilder>);
impl MonoSink for Sink {
    type Error = ResourceLimit;
    fn begin(&mut self, format: PcmFormat) -> Result<(), ResourceLimit> {
        self.0 = Some(StepTempoBuilder::new(format.sample_rate));
        Ok(())
    }
    fn push(&mut self, mono: &[f32]) -> Result<(), ResourceLimit> {
        if let Some(b) = &mut self.0 {
            b.push(mono);
            if b.resource_limited() {
                return Err(ResourceLimit);
            }
        }
        Ok(())
    }
}
/// No catalog path, storage handle, conversion to `TempoGrid`, or write operation.
pub fn inspect_step_tempo_file(path: &Path) -> Result<ComputedStepTempoV1, StepInspectionError> {
    let mut sink = Sink(None);
    decode_mono(path, &mut sink).map_err(|e| match e {
        PcmError::Unreadable => StepInspectionError::Unreadable,
        PcmError::Unsupported => StepInspectionError::UnsupportedAudio,
        PcmError::Malformed => StepInspectionError::MalformedAudio,
        PcmError::Sink(_) => StepInspectionError::ResourceLimit,
    })?;
    sink.0
        .ok_or(StepInspectionError::MalformedAudio)?
        .finish()
        .map_err(|_| StepInspectionError::ResourceLimit)
}
