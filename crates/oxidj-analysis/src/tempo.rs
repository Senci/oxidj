//! Streaming tempo and beat-grid analysis (B2a), built on the admitted
//! `taktus` engine (ADR-004 B2 amendment).
//!
//! Feed already-decoded **mono** `f32` samples to [`TempoBuilder::push`] in
//! chunks of any size, then call [`TempoBuilder::finish`]. Samples are turned
//! into an onset envelope as they arrive (100 values a second, see
//! [`ONSETS_PER_SECOND`]); the audio itself is never buffered, and the
//! envelope is capped at [`TempoOptions::max_seconds`]. The result is
//! identical however the input is split into chunks.
//!
//! The output is a *computed suggestion*: accuracy on real music is what the
//! B1 bake-off measured (BPM within 2% of Rekordbox on 91% of 90 tracks).
//!
//! # Failing closed
//!
//! `finish` returns `Ok(None)` — never a guess and never a panic — when the
//! sample rate is below [`MIN_TEMPO_SAMPLE_RATE`] (including 0), any sample was
//! NaN or infinite, the input is empty or shorter than [`MIN_TEMPO_SECONDS`],
//! or the reading fails the [`TempoGate`] (silence, DC and noise do). Only a
//! stream longer than `max_seconds` is an error ([`TempoError::TooLong`]).
//!
//! # BPM range fold
//!
//! A tempo and its half or double are the same beats counted at another
//! level, so engines make octave errors. The reading is folded by factors of
//! two into [`BpmRange`] (default [`BpmRange::DEFAULT`] = 70–180 BPM, the same
//! as Rekordbox's default analysis range) and the beats are then tracked at
//! the folded tempo. The range is a parameter of [`TempoOptions`].

use oxidj_core::tempo::TempoGrid;
use taktus::{Ear, beats_of, tempo_of};

/// Algorithm label stored in every [`TempoGrid`] this module produces.
pub const TEMPO_ALGORITHM: &str = "taktus 0.1.0";
/// Onset envelope values per second of audio (taktus' fixed rate).
pub const ONSETS_PER_SECOND: usize = taktus::RATE;
/// Lowest sample rate the tempo engine accepts.
pub const MIN_TEMPO_SAMPLE_RATE: u32 = 8_000;
/// Shortest input analysed, seconds (taktus needs 4 s of onsets).
pub const MIN_TEMPO_SECONDS: usize = 5;
/// Default cap on analysed audio, seconds (2 hours: 2.9 MB of onsets).
pub const DEFAULT_MAX_TEMPO_SECONDS: u32 = 2 * 60 * 60;

/// Why tempo analysis was refused (everything else fails closed to `None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempoError {
    /// A [`BpmRange`] that is not finite, not positive, or narrower than one
    /// octave (`max < 2 * min`), so octave folding could not land in it.
    InvalidRange,
    /// The stream is longer than [`TempoOptions::max_seconds`].
    TooLong {
        /// The configured cap, seconds.
        max_seconds: u32,
    },
}

impl core::fmt::Display for TempoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRange => f.write_str("BPM range must be positive and at least one octave"),
            Self::TooLong { max_seconds } => {
                write!(
                    f,
                    "input is longer than the {max_seconds} s tempo-analysis cap"
                )
            }
        }
    }
}

impl std::error::Error for TempoError {}

/// The tempo range readings are folded into, BPM, inclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BpmRange {
    min: f64,
    max: f64,
}

impl BpmRange {
    /// 70–180 BPM: Rekordbox's default analysis range.
    pub const DEFAULT: Self = Self {
        min: 70.0,
        max: 180.0,
    };

    /// A range `min..=max`; it must span at least one octave.
    pub fn new(min: f64, max: f64) -> Result<Self, TempoError> {
        if min.is_finite() && max.is_finite() && min > 0.0 && max >= 2.0 * min {
            Ok(Self { min, max })
        } else {
            Err(TempoError::InvalidRange)
        }
    }

    /// Lowest tempo, BPM.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Highest tempo, BPM.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// `bpm` doubled or halved until it lies in the range (an octave fold).
    /// `None` for a non-finite or non-positive `bpm`.
    pub fn fold(&self, bpm: f64) -> Option<f64> {
        if !(bpm.is_finite() && bpm > 0.0) {
            return None;
        }
        let mut bpm = bpm;
        while bpm < self.min {
            bpm *= 2.0;
        }
        while bpm > self.max {
            bpm /= 2.0;
        }
        // max >= 2 * min, so halving from above max cannot fall below min.
        (self.min..=self.max).contains(&bpm).then_some(bpm)
    }
}

impl Default for BpmRange {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The confidence gate a taktus reading must pass to become a grid.
///
/// `sure` is how well the onsets keep the period (0–1); `catch` is how much of
/// what starts falls on the beats (1 = no more than anywhere, sound with no
/// beat ≈ 0.7). taktus documents `sure ≥ 0.05` and `catch ≥ 1.0` as right 97–99
/// times in 100; the default is slightly stricter on `catch`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempoGate {
    /// Minimum `Beat::sure`.
    pub min_sure: f64,
    /// Minimum `Beat::catch`.
    pub min_catch: f64,
}

impl TempoGate {
    /// The default gate.
    pub const DEFAULT: Self = Self {
        min_sure: 0.05,
        min_catch: 1.1,
    };

    fn passes(&self, sure: f64, catch: f64) -> bool {
        sure.is_finite() && catch.is_finite() && sure >= self.min_sure && catch >= self.min_catch
    }
}

impl Default for TempoGate {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Tempo analysis parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempoOptions {
    /// Range readings are folded into.
    pub range: BpmRange,
    /// Confidence gate.
    pub gate: TempoGate,
    /// Longest stream analysed, seconds; bounds the onset buffer.
    pub max_seconds: u32,
}

impl Default for TempoOptions {
    fn default() -> Self {
        Self {
            range: BpmRange::DEFAULT,
            gate: TempoGate::DEFAULT,
            max_seconds: DEFAULT_MAX_TEMPO_SECONDS,
        }
    }
}

/// Streaming tempo/beat-grid builder. See the module docs for the contract.
#[derive(Clone, Debug)]
pub struct TempoBuilder {
    /// `None` when the sample rate is unsupported: input is ignored.
    ear: Option<Ear>,
    onsets: Vec<f32>,
    max_onsets: usize,
    options: TempoOptions,
    non_finite: bool,
    too_long: bool,
}

impl TempoBuilder {
    /// Start a builder for mono input at `sample_rate`. An unsupported rate is
    /// not an error here: [`finish`](Self::finish) returns `Ok(None)`.
    pub fn new(sample_rate: u32, options: TempoOptions) -> Self {
        Self {
            ear: (sample_rate >= MIN_TEMPO_SAMPLE_RATE).then(|| Ear::new(sample_rate)),
            onsets: Vec::new(),
            max_onsets: options.max_seconds as usize * ONSETS_PER_SECOND,
            options,
            non_finite: false,
            too_long: false,
        }
    }

    /// Feed the next chunk of mono samples.
    pub fn push(&mut self, samples: &[f32]) {
        if self.non_finite || self.too_long {
            return;
        }
        let Some(ear) = self.ear.as_mut() else {
            return;
        };
        if samples.iter().any(|s| !s.is_finite()) {
            self.non_finite = true;
            self.onsets = Vec::new();
            return;
        }
        for &sample in samples {
            if let Some(onset) = ear.hear(sample) {
                if self.onsets.len() == self.max_onsets {
                    self.too_long = true;
                    self.onsets = Vec::new();
                    return;
                }
                self.onsets.push(onset);
            }
        }
    }

    /// Onset values currently buffered (never more than
    /// `max_seconds * ONSETS_PER_SECOND`).
    pub fn buffered_onsets(&self) -> usize {
        self.onsets.len()
    }

    /// The onset-buffer cap, values.
    pub fn max_onsets(&self) -> usize {
        self.max_onsets
    }

    /// Finish the stream: the grid, or `None` when no trustworthy beat was found.
    pub fn finish(self) -> Result<Option<TempoGrid>, TempoError> {
        if self.too_long {
            return Err(TempoError::TooLong {
                max_seconds: self.options.max_seconds,
            });
        }
        if self.ear.is_none()
            || self.non_finite
            || self.onsets.len() < MIN_TEMPO_SECONDS * ONSETS_PER_SECOND
        {
            return Ok(None);
        }
        let Some(beat) = tempo_of(&self.onsets) else {
            return Ok(None);
        };
        if !self.options.gate.passes(beat.sure, beat.catch) {
            return Ok(None);
        }
        let Some(bpm) = self.options.range.fold(beat.bpm()) else {
            return Ok(None);
        };
        let beats_ms = beats_to_ms(&beats_of(&self.onsets, 60.0 / bpm));
        if beats_ms.len() < 2 {
            return Ok(None);
        }
        let confidence = beat.sure.clamp(0.0, 1.0) as f32;
        Ok(TempoGrid::new(bpm, beats_ms, confidence, TEMPO_ALGORITHM).ok())
    }
}

/// Seconds to whole milliseconds: negatives dropped, strictly ascending.
fn beats_to_ms(beats: &[f64]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::with_capacity(beats.len());
    for &seconds in beats {
        if !(seconds.is_finite() && seconds >= 0.0) {
            continue;
        }
        let ms = (seconds * 1000.0).round();
        if ms > f64::from(u32::MAX) {
            break;
        }
        let ms = ms as u32;
        if out.last().is_none_or(|&last| ms > last) {
            out.push(ms);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_lands_in_range() {
        let r = BpmRange::DEFAULT;
        assert_eq!(r.fold(64.0), Some(128.0));
        assert_eq!(r.fold(256.0), Some(128.0));
        assert_eq!(r.fold(87.0), Some(87.0));
        assert_eq!(r.fold(348.0), Some(174.0));
        assert_eq!(r.fold(35.0), Some(70.0));
        assert_eq!(r.fold(f64::NAN), None);
        assert_eq!(r.fold(0.0), None);
        assert_eq!(BpmRange::new(80.0, 150.0), Err(TempoError::InvalidRange));
        assert_eq!(BpmRange::new(0.0, 150.0), Err(TempoError::InvalidRange));
        let wide = BpmRange::new(60.0, 200.0).unwrap();
        assert_eq!(wide.fold(190.0), Some(190.0));
    }

    #[test]
    fn beats_to_ms_drops_negatives_and_duplicates() {
        assert_eq!(
            beats_to_ms(&[-0.01, 0.0, 0.0004, 0.5, f64::NAN, 1.0]),
            vec![0, 500, 1000]
        );
    }
}
