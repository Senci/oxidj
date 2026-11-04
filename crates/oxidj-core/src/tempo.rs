//! Tempo and beat grid of one track (analysis B2, layer 0).
//!
//! A [`TempoGrid`] is a *computed suggestion*: one constant-ish tempo plus the
//! beat positions an analyser found. It is produced by `oxidj-analysis` and is
//! neutral data only; persistence and UI live in higher crates.

use core::fmt;

/// Most beats a grid may hold: 4 hours at 400 BPM. Analysers stay far below.
pub const MAX_BEATS: usize = 4 * 60 * 400;

/// A track's tempo and beat grid.
#[derive(Clone, Debug, PartialEq)]
pub struct TempoGrid {
    bpm: f64,
    beats_ms: Vec<u32>,
    confidence: f32,
    algorithm: String,
}

/// Why a [`TempoGrid`] was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempoGridError {
    /// BPM is not finite or not positive.
    InvalidBpm,
    /// Beat positions are not strictly ascending.
    BeatsNotAscending,
    /// More than [`MAX_BEATS`] beats.
    TooManyBeats,
    /// Confidence is not finite or outside `0.0..=1.0`.
    InvalidConfidence,
    /// The algorithm label is empty.
    EmptyAlgorithm,
}

impl fmt::Display for TempoGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBpm => "bpm must be finite and positive",
            Self::BeatsNotAscending => "beat positions must be strictly ascending",
            Self::TooManyBeats => "too many beats",
            Self::InvalidConfidence => "confidence must be within 0..=1",
            Self::EmptyAlgorithm => "algorithm label is empty",
        })
    }
}

impl std::error::Error for TempoGridError {}

impl TempoGrid {
    /// A validated grid. `bpm` is rounded to 0.01.
    pub fn new(
        bpm: f64,
        beats_ms: Vec<u32>,
        confidence: f32,
        algorithm: impl Into<String>,
    ) -> Result<Self, TempoGridError> {
        if !(bpm.is_finite() && bpm > 0.0) {
            return Err(TempoGridError::InvalidBpm);
        }
        if beats_ms.len() > MAX_BEATS {
            return Err(TempoGridError::TooManyBeats);
        }
        if beats_ms.windows(2).any(|w| w[0] >= w[1]) {
            return Err(TempoGridError::BeatsNotAscending);
        }
        if !(confidence.is_finite() && (0.0..=1.0).contains(&confidence)) {
            return Err(TempoGridError::InvalidConfidence);
        }
        let algorithm = algorithm.into();
        if algorithm.is_empty() {
            return Err(TempoGridError::EmptyAlgorithm);
        }
        Ok(Self {
            bpm: (bpm * 100.0).round() / 100.0,
            beats_ms,
            confidence,
            algorithm,
        })
    }

    /// Tempo in beats per minute, rounded to 0.01.
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Beat positions, milliseconds from the start of the audio, strictly ascending.
    pub fn beats_ms(&self) -> &[u32] {
        &self.beats_ms
    }

    /// Analyser confidence, `0.0..=1.0`.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// Algorithm and version that produced the grid, e.g. `"taktus 0.1.0"`.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_bpm_and_validates() {
        let grid = TempoGrid::new(127.996, vec![10, 479], 0.5, "x").unwrap();
        assert_eq!(grid.bpm(), 128.0);
        assert_eq!(grid.beats_ms(), &[10, 479]);
        assert_eq!(
            TempoGrid::new(f64::NAN, vec![], 0.5, "x"),
            Err(TempoGridError::InvalidBpm)
        );
        assert_eq!(
            TempoGrid::new(0.0, vec![], 0.5, "x"),
            Err(TempoGridError::InvalidBpm)
        );
        assert_eq!(
            TempoGrid::new(120.0, vec![5, 5], 0.5, "x"),
            Err(TempoGridError::BeatsNotAscending)
        );
        assert_eq!(
            TempoGrid::new(120.0, vec![], 1.5, "x"),
            Err(TempoGridError::InvalidConfidence)
        );
        assert_eq!(
            TempoGrid::new(120.0, vec![], 0.5, ""),
            Err(TempoGridError::EmptyAlgorithm)
        );
        assert_eq!(
            TempoGrid::new(120.0, vec![0; MAX_BEATS + 1], 0.5, "x"),
            Err(TempoGridError::TooManyBeats)
        );
    }
}
