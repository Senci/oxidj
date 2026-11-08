//! Explicit-file step-tempo inspection: decode only, never opens a catalog.
use oxidj_analysis::step_tempo::StepTempoBuilder;
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
#[derive(Debug)]
enum Stop {
    UnsupportedRate,
    ResourceLimit,
}
struct Sink(Option<StepTempoBuilder>);
impl MonoSink for Sink {
    type Error = Stop;
    fn begin(&mut self, format: PcmFormat) -> Result<(), Stop> {
        if !(8000..=48000).contains(&format.sample_rate) {
            return Err(Stop::UnsupportedRate);
        }
        self.0 = Some(StepTempoBuilder::new(format.sample_rate));
        Ok(())
    }
    fn push(&mut self, mono: &[f32]) -> Result<(), Stop> {
        if let Some(b) = &mut self.0 {
            b.push(mono);
            if b.resource_limited() {
                return Err(Stop::ResourceLimit);
            }
        }
        Ok(())
    }
}
/// No catalog path, storage handle, conversion to `TempoGrid`, or write operation.
pub fn inspect_step_tempo_file(path: &Path) -> Result<ComputedStepTempoV1, StepInspectionError> {
    let mut sink = Sink(None);
    match decode_mono(path, &mut sink) {
        Ok(_) => {}
        Err(PcmError::Sink(Stop::UnsupportedRate)) => {
            return Ok(ComputedStepTempoV1::NoMap(StepRefusal::UnsupportedRate));
        }
        Err(error) => {
            return Err(match error {
                PcmError::Unreadable => StepInspectionError::Unreadable,
                PcmError::Unsupported => StepInspectionError::UnsupportedAudio,
                PcmError::Malformed => StepInspectionError::MalformedAudio,
                PcmError::Sink(_) => StepInspectionError::ResourceLimit,
            });
        }
    }
    sink.0
        .ok_or(StepInspectionError::MalformedAudio)?
        .finish()
        .map_err(|_| StepInspectionError::ResourceLimit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_rate_stops_decoder_before_any_pcm() {
        for rate in [0, 7999, 48001, u32::MAX] {
            let mut sink = Sink(None);
            assert!(
                matches!(
                    sink.begin(PcmFormat {
                        sample_rate: rate,
                        channels: 1,
                        frames: None,
                    }),
                    Err(Stop::UnsupportedRate)
                ),
                "unsupported rate {rate} must stop even an unknown-length stream"
            );
            assert!(sink.0.is_none());
        }
    }
}
