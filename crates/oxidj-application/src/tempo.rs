//! B2b: compute and store tempo/beat grids for local catalog tracks. Composes
//! the library's streaming decoders (`oxidj_library::pcm`) with the analysis
//! builder (`oxidj_analysis::TempoBuilder`) chunk by chunk — the audio is never
//! buffered — and stores the result by track ID in the computed-grid table,
//! never in the imported vendor tables.
//!
//! A stored grid records the SHA-256 of the exact bytes it was computed from
//! (hashed before and after decoding, which must agree). A run reports
//! `Unchanged` only when the file still hashes to that digest and the
//! algorithm and BPM range match; otherwise the track is re-analysed. When the
//! analyser finds no trustworthy beat the track is `NoGrid`: nothing is stored
//! and a stale grid from an earlier source or range is removed. Any failure
//! leaves the stored row untouched. Source files are only read.

use std::convert::Infallible;
use std::path::Path;

use oxidj_analysis::key::{Key, KeyBuilder};
use oxidj_analysis::tempo::{BpmRange, TEMPO_ALGORITHM, TempoBuilder, TempoError, TempoOptions};
use oxidj_core::TrackId;
use oxidj_core::tempo::TempoGrid;
use oxidj_library::pcm::{MonoSink, PcmError, PcmFormat, decode_mono};
use oxidj_library::{Catalog, TempoBasis};

/// Default BPM fold range of `catalog analyze-tempo` (Rekordbox's default).
pub const DEFAULT_TEMPO_RANGE: (u32, u32) = (70, 180);

/// Why a track's tempo was not analysed. Messages never contain paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TempoAnalysisError {
    /// The catalog does not exist (it is never created implicitly).
    MissingCatalog,
    /// No catalog track has the requested ID.
    UnknownTrack,
    /// The source file could not be read.
    SourceUnreadable,
    /// The source file changed while it was being analysed.
    SourceChanged,
    /// The audio format is outside the supported decode matrix.
    UnsupportedAudio,
    /// The audio is corrupt or truncated.
    MalformedAudio,
    /// The tempo analyser refused the request (invalid range, too long).
    Tempo(TempoError),
    /// A catalog storage error.
    Catalog(String),
}

impl std::fmt::Display for TempoAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCatalog => f.write_str("catalog does not exist"),
            Self::UnknownTrack => f.write_str("unknown track ID"),
            Self::SourceUnreadable => f.write_str("source audio could not be read"),
            Self::SourceChanged => f.write_str("source audio changed during analysis"),
            Self::UnsupportedAudio => f.write_str("unsupported audio format"),
            Self::MalformedAudio => f.write_str("corrupt or truncated audio"),
            Self::Tempo(error) => write!(f, "tempo analysis refused: {error}"),
            Self::Catalog(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for TempoAnalysisError {}

fn catalog_error(error: oxidj_library::CatalogError) -> TempoAnalysisError {
    TempoAnalysisError::Catalog(error.to_string())
}

/// Per-track result of [`analyze_catalog_tempo`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempoOutcome {
    /// A grid was computed and stored.
    Analyzed,
    /// A current grid for this source, algorithm and range was already stored.
    Unchanged,
    /// No trustworthy beat was found; nothing is stored (not a failure).
    NoGrid,
}

/// Outcomes in track-ID order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TempoReport {
    pub outcomes: Vec<(TrackId, Result<TempoOutcome, TempoAnalysisError>)>,
    /// Computed key suggestions; `None` means the audio provided no confident key.
    pub keys: Vec<(TrackId, Result<Option<Key>, TempoAnalysisError>)>,
    /// Imported-library entries whose locations could not be read as local audio.
    pub unavailable_library_audio: usize,
}

impl TempoReport {
    fn count(&self, wanted: Option<TempoOutcome>) -> usize {
        self.outcomes
            .iter()
            .filter(|(_, result)| result.as_ref().ok().copied() == wanted)
            .count()
    }
    pub fn analyzed(&self) -> usize {
        self.count(Some(TempoOutcome::Analyzed))
    }
    pub fn unchanged(&self) -> usize {
        self.count(Some(TempoOutcome::Unchanged))
    }
    pub fn no_grid(&self) -> usize {
        self.count(Some(TempoOutcome::NoGrid))
    }
    pub fn failed(&self) -> usize {
        self.count(None)
    }
}

/// A grid computed from one file, with its decode basis.
#[derive(Clone, Debug, PartialEq)]
pub struct FileTempo {
    pub grid: TempoGrid,
    pub sample_rate: u32,
    pub duration_ms: u64,
}

struct TempoSink {
    options: TempoOptions,
    builder: Option<TempoBuilder>,
    sample_rate: u32,
    samples: u64,
}

impl MonoSink for TempoSink {
    type Error = Infallible;
    fn begin(&mut self, format: PcmFormat) -> Result<(), Infallible> {
        self.builder = Some(TempoBuilder::new(format.sample_rate, self.options));
        self.sample_rate = format.sample_rate;
        Ok(())
    }
    fn push(&mut self, mono: &[f32]) -> Result<(), Infallible> {
        if let Some(builder) = self.builder.as_mut() {
            builder.push(mono);
            self.samples += mono.len() as u64;
        }
        Ok(())
    }
}

fn options(range: (u32, u32)) -> Result<TempoOptions, TempoAnalysisError> {
    Ok(TempoOptions {
        range: BpmRange::new(f64::from(range.0), f64::from(range.1))
            .map_err(TempoAnalysisError::Tempo)?,
        ..TempoOptions::default()
    })
}

/// Decode one audio file and compute its grid (no catalog access). `Ok(None)`
/// when the analyser found no trustworthy beat.
pub fn analyze_tempo_file(
    path: &Path,
    range: (u32, u32),
) -> Result<Option<FileTempo>, TempoAnalysisError> {
    let mut sink = TempoSink {
        options: options(range)?,
        builder: None,
        sample_rate: 0,
        samples: 0,
    };
    decode_mono(path, &mut sink).map_err(|error| match error {
        PcmError::Unreadable => TempoAnalysisError::SourceUnreadable,
        PcmError::Unsupported => TempoAnalysisError::UnsupportedAudio,
        PcmError::Malformed => TempoAnalysisError::MalformedAudio,
        PcmError::Sink(never) => match never {},
    })?;
    let builder = sink.builder.ok_or(TempoAnalysisError::MalformedAudio)?;
    let grid = builder.finish().map_err(TempoAnalysisError::Tempo)?;
    Ok(grid.map(|grid| FileTempo {
        grid,
        sample_rate: sink.sample_rate,
        duration_ms: sink.samples.saturating_mul(1000) / u64::from(sink.sample_rate.max(1)),
    }))
}

struct KeySink {
    builder: Option<KeyBuilder>,
}

impl MonoSink for KeySink {
    type Error = Infallible;
    fn begin(&mut self, format: PcmFormat) -> Result<(), Infallible> {
        self.builder = Some(KeyBuilder::new(format.sample_rate));
        Ok(())
    }
    fn push(&mut self, mono: &[f32]) -> Result<(), Infallible> {
        if let Some(builder) = self.builder.as_mut() {
            builder.push(mono);
        }
        Ok(())
    }
}

/// Compute a key suggestion from one local audio file. `None` is a deliberate
/// no-result for short, silent, unsupported-rate, or ambiguous material.
pub fn analyze_key_file(path: &Path) -> Result<Option<Key>, TempoAnalysisError> {
    let mut sink = KeySink { builder: None };
    decode_mono(path, &mut sink).map_err(|error| match error {
        PcmError::Unreadable => TempoAnalysisError::SourceUnreadable,
        PcmError::Unsupported => TempoAnalysisError::UnsupportedAudio,
        PcmError::Malformed => TempoAnalysisError::MalformedAudio,
        PcmError::Sink(never) => match never {},
    })?;
    Ok(sink
        .builder
        .ok_or(TempoAnalysisError::MalformedAudio)?
        .finish())
}

fn source_digest(path: &Path) -> Result<[u8; 32], TempoAnalysisError> {
    oxidj_library::file_sha256(path)
        .map(|(digest, _)| digest)
        .map_err(|_| TempoAnalysisError::SourceUnreadable)
}

/// Analyse and store the grid of one catalog track. See the module docs.
pub fn analyze_track_tempo(
    catalog: &mut Catalog,
    id: TrackId,
    range: (u32, u32),
) -> Result<TempoOutcome, TempoAnalysisError> {
    options(range)?;
    let track = catalog
        .read_track(id)
        .map_err(catalog_error)?
        .ok_or(TempoAnalysisError::UnknownTrack)?;
    let path = Path::new(&track.path);
    let digest = source_digest(path)?;
    if catalog
        .computed_beat_grid(id)
        .map_err(catalog_error)?
        .is_some_and(|stored| {
            stored.source_sha256 == digest
                && stored.grid.algorithm() == TEMPO_ALGORITHM
                && (stored.basis.range_min_bpm, stored.basis.range_max_bpm) == range
        })
    {
        return Ok(TempoOutcome::Unchanged);
    }
    let analysed = analyze_tempo_file(path, range)?;
    if source_digest(path)? != digest {
        return Err(TempoAnalysisError::SourceChanged);
    }
    match analysed {
        Some(file) => {
            let basis = TempoBasis {
                sample_rate: file.sample_rate,
                duration_ms: file.duration_ms,
                range_min_bpm: range.0,
                range_max_bpm: range.1,
            };
            catalog
                .store_computed_beat_grid(id, digest, basis, &file.grid)
                .map_err(catalog_error)?;
            Ok(TempoOutcome::Analyzed)
        }
        None => {
            catalog
                .remove_computed_beat_grid(id)
                .map_err(catalog_error)?;
            Ok(TempoOutcome::NoGrid)
        }
    }
}

/// `catalog analyze-tempo`: analyse one track (`Some(id)`, unknown → error) or
/// every catalog track, folding into `range` (whole BPM, at least one octave).
/// Per-track refusals are reported, not fatal.
pub fn analyze_catalog_tempo(
    db: impl AsRef<Path>,
    track: Option<TrackId>,
    range: (u32, u32),
) -> Result<TempoReport, TempoAnalysisError> {
    options(range)?;
    let db = db.as_ref();
    if db.symlink_metadata().is_err() {
        return Err(TempoAnalysisError::MissingCatalog);
    }
    let mut catalog = Catalog::open(db).map_err(catalog_error)?;
    let mut unavailable_library_audio = 0;
    let ids = match track {
        Some(id) => {
            catalog
                .read_track(id)
                .map_err(catalog_error)?
                .ok_or(TempoAnalysisError::UnknownTrack)?;
            vec![id]
        }
        None => {
            let mut ids: Vec<TrackId> = catalog
                .list_tracks()
                .map_err(catalog_error)?
                .into_iter()
                .map(|t| t.id)
                .collect();
            let mut seen: std::collections::HashSet<TrackId> = ids.iter().copied().collect();
            for imported in catalog.list_library_tracks().map_err(catalog_error)? {
                match catalog.reference_id_for_library_location(&imported.location) {
                    Ok(Some(id)) if seen.insert(id) => ids.push(id),
                    Ok(Some(_)) => {}
                    Ok(None) | Err(oxidj_library::CatalogError::UnavailableLibraryAudio) => {
                        unavailable_library_audio += 1
                    }
                    Err(error) => return Err(catalog_error(error)),
                }
            }
            ids
        }
    };
    let keys = ids
        .iter()
        .map(|id| {
            let result = catalog
                .read_track(*id)
                .map_err(catalog_error)
                .and_then(|track| {
                    let track = track.ok_or(TempoAnalysisError::UnknownTrack)?;
                    let path = Path::new(&track.path);
                    let before = source_digest(path)?;
                    let key = analyze_key_file(path)?;
                    if source_digest(path)? != before {
                        return Err(TempoAnalysisError::SourceChanged);
                    }
                    Ok(key)
                });
            (*id, result)
        })
        .collect();
    let outcomes = ids
        .into_iter()
        .map(|id| (id, analyze_track_tempo(&mut catalog, id, range)))
        .collect();
    Ok(TempoReport {
        outcomes,
        keys,
        unavailable_library_audio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 22_050;
    const SECONDS: f64 = 30.0;
    const T0: f64 = 0.3;

    fn scratch() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("oxidj-app-tempo-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 16-bit mono WAV of a kick+click track at `bpm` (silence when `bpm` is 0).
    fn click_wav(bpm: f64) -> Vec<u8> {
        let n = (SECONDS * f64::from(RATE)) as usize;
        let mut out = vec![0.0f64; n];
        if bpm > 0.0 {
            let len = (0.25 * f64::from(RATE)) as usize;
            let mut hit = T0;
            while hit < SECONDS {
                let start = (hit * f64::from(RATE)).round() as usize;
                for (i, slot) in out.iter_mut().skip(start).take(len).enumerate() {
                    let t = i as f64 / f64::from(RATE);
                    *slot += 0.6 * (std::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp()
                        + 0.3 * (std::f64::consts::TAU * 1_800.0 * t).sin() * (-t / 0.004).exp();
                }
                hit += 60.0 / bpm;
            }
        }
        let data: Vec<u8> = out
            .iter()
            .flat_map(|s| ((s * 32_000.0) as i16).to_le_bytes())
            .collect();
        let mut wav = b"RIFF".to_vec();
        wav.extend((36 + data.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(RATE.to_le_bytes());
        wav.extend((RATE * 2).to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((data.len() as u32).to_le_bytes());
        wav.extend(data);
        wav
    }

    /// Every stored beat within 30 ms of a generator beat; BPM within 0.5.
    fn assert_matches(grid: &TempoGrid, bpm: f64) {
        assert!((grid.bpm() - bpm).abs() <= 0.5, "bpm {}", grid.bpm());
        assert!(grid.beats_ms().len() > 50);
        let period = 60.0 / bpm;
        for &ms in grid.beats_ms() {
            let k = ((f64::from(ms) / 1000.0 - T0) / period).round();
            let off = (f64::from(ms) / 1000.0 - (T0 + k * period)).abs();
            assert!(off <= 0.030, "beat {ms} ms off by {off}");
        }
    }

    #[test]
    fn click_track_is_stored_then_unchanged_then_reanalysed_after_change() {
        let root = scratch();
        let audio = root.join("audio");
        std::fs::create_dir(&audio).unwrap();
        std::fs::write(audio.join("a.wav"), click_wav(128.0)).unwrap();
        std::fs::write(audio.join("b.wav"), click_wav(0.0)).unwrap();
        let before = std::fs::read(audio.join("a.wav")).unwrap();
        let db = root.join("catalog.sqlite");
        Catalog::open(&db)
            .unwrap()
            .import_audio_directory(&audio)
            .unwrap();
        let (a, b) = (TrackId::new(1).unwrap(), TrackId::new(2).unwrap());

        let report = analyze_catalog_tempo(&db, None, DEFAULT_TEMPO_RANGE).unwrap();
        assert_eq!(
            report.outcomes,
            vec![
                (a, Ok(TempoOutcome::Analyzed)),
                (b, Ok(TempoOutcome::NoGrid))
            ]
        );
        let catalog = Catalog::open(&db).unwrap();
        let stored = catalog.computed_beat_grid(a).unwrap().unwrap();
        assert_matches(&stored.grid, 128.0);
        assert_eq!(stored.grid.algorithm(), "taktus 0.1.0");
        assert_eq!(
            stored.source_sha256,
            catalog.read_track(a).unwrap().unwrap().sha256
        );
        assert_eq!(
            stored.basis,
            TempoBasis {
                sample_rate: RATE,
                duration_ms: 30_000,
                range_min_bpm: 70,
                range_max_bpm: 180
            }
        );
        assert_eq!(
            catalog.computed_beat_grid(b).unwrap(),
            None,
            "NoGrid stores nothing"
        );
        drop(catalog);

        let report = analyze_catalog_tempo(&db, None, DEFAULT_TEMPO_RANGE).unwrap();
        assert_eq!(
            report.outcomes,
            vec![
                (a, Ok(TempoOutcome::Unchanged)),
                (b, Ok(TempoOutcome::NoGrid))
            ]
        );
        assert_eq!(std::fs::read(audio.join("a.wav")).unwrap(), before);
        // A different range is a different analysis.
        let report = analyze_catalog_tempo(&db, Some(a), (60, 200)).unwrap();
        assert_eq!(report.outcomes, vec![(a, Ok(TempoOutcome::Analyzed))]);

        // Changed source: re-analysed and re-stored with the new digest.
        std::fs::write(audio.join("a.wav"), click_wav(140.0)).unwrap();
        let report = analyze_catalog_tempo(&db, Some(a), DEFAULT_TEMPO_RANGE).unwrap();
        assert_eq!(report.outcomes, vec![(a, Ok(TempoOutcome::Analyzed))]);
        let stored = Catalog::open(&db)
            .unwrap()
            .computed_beat_grid(a)
            .unwrap()
            .unwrap();
        assert_matches(&stored.grid, 140.0);
        assert_eq!(
            stored.source_sha256,
            oxidj_library::file_sha256(&audio.join("a.wav")).unwrap().0
        );
        // Source now silent: NoGrid removes the stale grid.
        std::fs::write(audio.join("a.wav"), click_wav(0.0)).unwrap();
        let report = analyze_catalog_tempo(&db, Some(a), DEFAULT_TEMPO_RANGE).unwrap();
        assert_eq!(report.outcomes, vec![(a, Ok(TempoOutcome::NoGrid))]);
        assert_eq!(
            Catalog::open(&db).unwrap().computed_beat_grid(a).unwrap(),
            None
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_missing_and_bad_range_are_typed_failures_and_others_continue() {
        let root = scratch();
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        let mut add = |name: &str, bytes: &[u8]| {
            let path = root.join(name);
            std::fs::write(&path, bytes).unwrap();
            let (sha, len) = oxidj_library::file_sha256(&path).unwrap();
            catalog
                .insert_track(name, path.to_str().unwrap(), sha, len)
                .unwrap()
                .id
        };
        let mut truncated = click_wav(128.0);
        truncated.truncate(truncated.len() - 1_001);
        let bad = add("truncated.wav", &truncated);
        let garbage = add("garbage.mp3", &[0; 64]);
        let good = add("good.wav", &click_wav(128.0));
        let gone = add("gone.wav", &click_wav(128.0));
        std::fs::remove_file(root.join("gone.wav")).unwrap();
        drop(catalog);
        let report = analyze_catalog_tempo(&db, None, DEFAULT_TEMPO_RANGE).unwrap();
        assert_eq!(
            report.outcomes,
            vec![
                (bad, Err(TempoAnalysisError::MalformedAudio)),
                (garbage, Err(TempoAnalysisError::MalformedAudio)),
                (good, Ok(TempoOutcome::Analyzed)),
                (gone, Err(TempoAnalysisError::SourceUnreadable)),
            ]
        );
        assert_eq!(
            (report.analyzed(), report.failed(), report.no_grid()),
            (1, 3, 0)
        );
        let catalog = Catalog::open(&db).unwrap();
        for id in [bad, garbage, gone] {
            assert_eq!(catalog.computed_beat_grid(id).unwrap(), None, "partial row");
        }
        drop(catalog);
        assert_eq!(
            analyze_catalog_tempo(&db, None, (100, 150)),
            Err(TempoAnalysisError::Tempo(TempoError::InvalidRange))
        );
        assert_eq!(
            analyze_catalog_tempo(&db, Some(TrackId::new(9).unwrap()), DEFAULT_TEMPO_RANGE),
            Err(TempoAnalysisError::UnknownTrack)
        );
        assert_eq!(
            analyze_catalog_tempo(root.join("absent.sqlite"), None, DEFAULT_TEMPO_RANGE),
            Err(TempoAnalysisError::MissingCatalog)
        );
        assert!(!root.join("absent.sqlite").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
