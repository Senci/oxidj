//! W2: compute waveform peaks for local catalog tracks. Composes the library's
//! streaming decoders (`oxidj_library::pcm`) with the analysis builder
//! (`oxidj_analysis::WaveformBuilder`) chunk by chunk — the full signal is never
//! buffered — and stores the result by track ID. Source files are only read; each
//! is checked against the catalog's recorded digest before and after decoding.
//! Any refusal stores nothing for that track.

use std::path::Path;

use oxidj_analysis::{AnalysisError, WaveformBuilder};
use oxidj_core::TrackId;
use oxidj_core::waveform::WaveformPeaks;
use oxidj_library::Catalog;
use oxidj_library::pcm::{MonoSink, PcmError, PcmFormat, decode_mono};

/// Why a track's waveform was not computed. Messages never contain paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaveformAnalysisError {
    /// The catalog does not exist (it is never created implicitly).
    MissingCatalog,
    /// No catalog track has the requested ID.
    UnknownTrack,
    /// The source file could not be read.
    SourceUnreadable,
    /// The source file no longer matches the catalog's recorded digest/length.
    SourceChanged,
    /// The audio format is outside the supported decode matrix.
    UnsupportedAudio,
    /// The audio is corrupt or truncated.
    MalformedAudio,
    /// The decoded signal was refused by the analysis builder.
    Analysis(AnalysisError),
    /// A catalog storage error.
    Catalog(String),
}

impl std::fmt::Display for WaveformAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCatalog => f.write_str("catalog does not exist"),
            Self::UnknownTrack => f.write_str("unknown track ID"),
            Self::SourceUnreadable => f.write_str("source audio could not be read"),
            Self::SourceChanged => f.write_str("source audio no longer matches the catalog"),
            Self::UnsupportedAudio => f.write_str("unsupported audio format"),
            Self::MalformedAudio => f.write_str("corrupt or truncated audio"),
            Self::Analysis(error) => write!(f, "waveform analysis refused the audio: {error}"),
            Self::Catalog(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for WaveformAnalysisError {}

fn catalog_error(error: oxidj_library::CatalogError) -> WaveformAnalysisError {
    WaveformAnalysisError::Catalog(error.to_string())
}

/// Per-track result of [`analyze_catalog_waveforms`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformOutcome {
    /// Peaks were computed and stored.
    Analyzed,
    /// Current peaks for the track's recorded source were already stored.
    Unchanged,
}

/// Outcomes in track-ID order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WaveformReport {
    pub outcomes: Vec<(TrackId, Result<WaveformOutcome, WaveformAnalysisError>)>,
}

impl WaveformReport {
    fn count(&self, wanted: Option<WaveformOutcome>) -> usize {
        self.outcomes
            .iter()
            .filter(|(_, result)| result.as_ref().ok().copied() == wanted)
            .count()
    }
    pub fn analyzed(&self) -> usize {
        self.count(Some(WaveformOutcome::Analyzed))
    }
    pub fn unchanged(&self) -> usize {
        self.count(Some(WaveformOutcome::Unchanged))
    }
    pub fn failed(&self) -> usize {
        self.count(None)
    }
}

/// Feeds decoded chunks straight into the builder.
struct BuilderSink(Option<WaveformBuilder>);

impl MonoSink for BuilderSink {
    type Error = AnalysisError;
    fn begin(&mut self, format: PcmFormat) -> Result<(), AnalysisError> {
        self.0 = Some(WaveformBuilder::new(format.sample_rate, format.frames)?);
        Ok(())
    }
    fn push(&mut self, mono: &[f32]) -> Result<(), AnalysisError> {
        self.0.as_mut().ok_or(AnalysisError::Empty)?.push(mono)
    }
}

/// Decode one audio file and compute its peaks (no catalog access).
pub fn analyze_waveform_file(path: &Path) -> Result<WaveformPeaks, WaveformAnalysisError> {
    let mut sink = BuilderSink(None);
    decode_mono(path, &mut sink).map_err(|error| match error {
        PcmError::Unreadable => WaveformAnalysisError::SourceUnreadable,
        PcmError::Unsupported => WaveformAnalysisError::UnsupportedAudio,
        PcmError::Malformed => WaveformAnalysisError::MalformedAudio,
        PcmError::Sink(error) => WaveformAnalysisError::Analysis(error),
    })?;
    sink.0
        .ok_or(WaveformAnalysisError::MalformedAudio)?
        .finish()
        .map_err(WaveformAnalysisError::Analysis)
}

fn check_source(path: &Path, sha256: [u8; 32], length: u64) -> Result<(), WaveformAnalysisError> {
    match oxidj_library::file_sha256(path) {
        Ok(current) if current == (sha256, length) => Ok(()),
        Ok(_) => Err(WaveformAnalysisError::SourceChanged),
        Err(_) => Err(WaveformAnalysisError::SourceUnreadable),
    }
}

/// Compute and store peaks for one catalog track. Idempotent: when peaks for the
/// track's recorded source digest are already stored, nothing is read or written.
pub fn analyze_track_waveform(
    catalog: &mut Catalog,
    id: TrackId,
) -> Result<WaveformOutcome, WaveformAnalysisError> {
    let track = catalog
        .read_track(id)
        .map_err(catalog_error)?
        .ok_or(WaveformAnalysisError::UnknownTrack)?;
    if catalog
        .read_waveform(id)
        .map_err(catalog_error)?
        .is_some_and(|stored| stored.source_sha256 == track.sha256)
    {
        return Ok(WaveformOutcome::Unchanged);
    }
    let path = Path::new(&track.path);
    check_source(path, track.sha256, track.byte_length)?;
    let peaks = analyze_waveform_file(path)?;
    check_source(path, track.sha256, track.byte_length)?;
    catalog
        .store_waveform(id, track.sha256, &peaks)
        .map_err(catalog_error)?;
    Ok(WaveformOutcome::Analyzed)
}

/// `catalog analyze-waveform`: analyze one track (`Some(id)`, unknown → error) or
/// every catalog track. Per-track refusals are reported, not fatal.
pub fn analyze_catalog_waveforms(
    db: impl AsRef<Path>,
    track: Option<TrackId>,
) -> Result<WaveformReport, WaveformAnalysisError> {
    let db = db.as_ref();
    if db.symlink_metadata().is_err() {
        return Err(WaveformAnalysisError::MissingCatalog);
    }
    let mut catalog = Catalog::open(db).map_err(catalog_error)?;
    let ids = match track {
        Some(id) => {
            catalog
                .read_track(id)
                .map_err(catalog_error)?
                .ok_or(WaveformAnalysisError::UnknownTrack)?;
            vec![id]
        }
        None => catalog
            .list_tracks()
            .map_err(catalog_error)?
            .into_iter()
            .map(|t| t.id)
            .collect(),
    };
    let outcomes = ids
        .into_iter()
        .map(|id| (id, analyze_track_waveform(&mut catalog, id)))
        .collect();
    Ok(WaveformReport { outcomes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("oxidj-app-waveform-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Synthetic 16-bit PCM mono WAV.
    fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(rate.to_le_bytes());
        out.extend((rate * 2).to_le_bytes());
        out.extend(2u16.to_le_bytes());
        out.extend(16u16.to_le_bytes());
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    fn tone() -> Vec<i16> {
        (0..30_000)
            .map(|i| {
                let t = f64::from(i) / 22_050.0;
                ((t * 440.0 * std::f64::consts::TAU).sin() * 0.6 * 32_767.0
                    + (t * 90.0 * std::f64::consts::TAU).sin() * 0.3 * 32_767.0)
                    as i16
            })
            .collect()
    }

    const FLAC: &[u8] = include_bytes!("../../oxidj-library/tests/fixtures/tone.flac");

    /// Reference: the builder fed the whole signal in one push.
    fn direct(rate: u32, samples: &[f32]) -> WaveformPeaks {
        let mut builder = WaveformBuilder::new(rate, Some(samples.len() as u64)).unwrap();
        builder.push(samples).unwrap();
        builder.finish().unwrap()
    }

    struct Collect(u32, Vec<f32>);
    impl MonoSink for Collect {
        type Error = ();
        fn begin(&mut self, format: PcmFormat) -> Result<(), ()> {
            self.0 = format.sample_rate;
            Ok(())
        }
        fn push(&mut self, mono: &[f32]) -> Result<(), ()> {
            self.1.extend_from_slice(mono);
            Ok(())
        }
    }

    #[test]
    fn wav_and_flac_round_trip_to_stored_peaks_equal_to_direct_builder_output() {
        let root = scratch();
        let audio = root.join("audio");
        std::fs::create_dir(&audio).unwrap();
        let samples = tone();
        std::fs::write(audio.join("a.wav"), wav(&samples, 22_050)).unwrap();
        std::fs::write(audio.join("b.flac"), FLAC).unwrap();
        let before: Vec<Vec<u8>> = ["a.wav", "b.flac"]
            .iter()
            .map(|n| std::fs::read(audio.join(n)).unwrap())
            .collect();
        let db = root.join("catalog.sqlite");
        Catalog::open(&db)
            .unwrap()
            .import_audio_directory(&audio)
            .unwrap();

        let report = analyze_catalog_waveforms(&db, None).unwrap();
        assert_eq!(
            (report.analyzed(), report.unchanged(), report.failed()),
            (2, 0, 0)
        );
        let catalog = Catalog::open(&db).unwrap();
        let tracks = catalog.list_tracks().unwrap();

        // WAV reference built from the generated samples, independent of any decoder.
        let wav_ref: Vec<f32> = samples.iter().map(|&s| f32::from(s) / 32_768.0).collect();
        let stored = catalog.read_waveform(tracks[0].id).unwrap().unwrap();
        assert_eq!(stored.peaks, direct(22_050, &wav_ref));
        assert_eq!(stored.source_sha256, tracks[0].sha256);
        assert!(stored.peaks.level_count() > 1);

        // FLAC reference: the whole decoded signal pushed into the builder at once.
        let mut all = Collect(0, Vec::new());
        decode_mono(&audio.join("b.flac"), &mut all).unwrap();
        let stored = catalog.read_waveform(tracks[1].id).unwrap().unwrap();
        assert_eq!(stored.peaks, direct(all.0, &all.1));

        let after: Vec<Vec<u8>> = ["a.wav", "b.flac"]
            .iter()
            .map(|n| std::fs::read(audio.join(n)).unwrap())
            .collect();
        assert_eq!(after, before, "source files are never modified");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reanalysis_is_idempotent_and_single_track_selection_works() {
        let root = scratch();
        let audio = root.join("audio");
        std::fs::create_dir(&audio).unwrap();
        std::fs::write(audio.join("a.wav"), wav(&tone(), 22_050)).unwrap();
        std::fs::write(audio.join("b.flac"), FLAC).unwrap();
        let db = root.join("catalog.sqlite");
        Catalog::open(&db)
            .unwrap()
            .import_audio_directory(&audio)
            .unwrap();
        let second = TrackId::new(2).unwrap();
        let report = analyze_catalog_waveforms(&db, Some(second)).unwrap();
        assert_eq!(
            report.outcomes,
            vec![(second, Ok(WaveformOutcome::Analyzed))]
        );
        let first = Catalog::open(&db).unwrap();
        assert_eq!(first.read_waveform(TrackId::new(1).unwrap()).unwrap(), None);
        let stored_second = first.read_waveform(second).unwrap();
        drop(first);

        let report = analyze_catalog_waveforms(&db, None).unwrap();
        assert_eq!((report.analyzed(), report.unchanged()), (1, 1));
        let snapshot = |db: &Path| {
            let c = Catalog::open(db).unwrap();
            [1, 2].map(|n| c.read_waveform(TrackId::new(n).unwrap()).unwrap())
        };
        let once = snapshot(&db);
        assert_eq!(once[1], stored_second);
        let report = analyze_catalog_waveforms(&db, None).unwrap();
        assert_eq!((report.analyzed(), report.unchanged()), (0, 2));
        assert_eq!(snapshot(&db), once);
        // Forced recomputation (row removed) reproduces identical peaks.
        let mut catalog = Catalog::open(&db).unwrap();
        let peaks = analyze_waveform_file(&audio.join("a.wav")).unwrap();
        assert_eq!(once[0].as_ref().unwrap().peaks, peaks);
        catalog
            .store_waveform(
                TrackId::new(1).unwrap(),
                once[0].as_ref().unwrap().source_sha256,
                &peaks,
            )
            .unwrap();
        drop(catalog);
        assert_eq!(snapshot(&db), once);
        assert_eq!(
            analyze_catalog_waveforms(&db, Some(TrackId::new(9).unwrap())),
            Err(WaveformAnalysisError::UnknownTrack)
        );
        assert_eq!(
            analyze_catalog_waveforms(root.join("absent.sqlite"), None),
            Err(WaveformAnalysisError::MissingCatalog)
        );
        assert!(!root.join("absent.sqlite").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_unsupported_and_changed_sources_leave_the_catalog_unchanged() {
        let root = scratch();
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        let mut add = |name: &str, bytes: &[u8], digest: Option<[u8; 32]>| {
            let path = root.join(name);
            std::fs::write(&path, bytes).unwrap();
            let (sha, len) = oxidj_library::file_sha256(&path).unwrap();
            catalog
                .insert_track(name, path.to_str().unwrap(), digest.unwrap_or(sha), len)
                .unwrap()
                .id
        };
        let mut truncated_wav = wav(&tone(), 22_050);
        truncated_wav.truncate(truncated_wav.len() - 1_001);
        let cases = [
            (
                add("truncated.wav", &truncated_wav, None),
                WaveformAnalysisError::MalformedAudio,
            ),
            (
                add(
                    "truncated.flac",
                    include_bytes!("../../oxidj-library/tests/fixtures/tone-total-truncated.flac"),
                    None,
                ),
                WaveformAnalysisError::MalformedAudio,
            ),
            (
                add("garbage.mp3", &[0; 64], None),
                WaveformAnalysisError::MalformedAudio,
            ),
            (
                add("tone.ogg", b"OggS....", None),
                WaveformAnalysisError::UnsupportedAudio,
            ),
            (
                add("changed.wav", &wav(&tone(), 22_050), Some([0; 32])),
                WaveformAnalysisError::SourceChanged,
            ),
            (
                add("low-rate.wav", &wav(&[1, 2, 3, 4], 4_000), None),
                WaveformAnalysisError::Analysis(AnalysisError::UnsupportedSampleRate(4_000)),
            ),
        ];
        drop(catalog);
        let report = analyze_catalog_waveforms(&db, None).unwrap();
        let expected: Vec<_> = cases.iter().map(|(id, e)| (*id, Err(e.clone()))).collect();
        assert_eq!(report.outcomes, expected);
        let catalog = Catalog::open(&db).unwrap();
        for (id, _) in &cases {
            assert_eq!(
                catalog.read_waveform(*id).unwrap(),
                None,
                "partial row stored"
            );
        }
        // A missing source is a typed refusal too.
        std::fs::remove_file(root.join("truncated.wav")).unwrap();
        drop(catalog);
        assert_eq!(
            analyze_catalog_waveforms(&db, Some(cases[0].0))
                .unwrap()
                .outcomes,
            vec![(cases[0].0, Err(WaveformAnalysisError::SourceUnreadable))]
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
