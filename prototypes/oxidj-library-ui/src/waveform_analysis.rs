//! Background catalog waveform jobs for the Import references workflow.
use oxidj_application::{analyze_track_waveform, WaveformOutcome};
use oxidj_library::{Catalog, ImportReport};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

pub(crate) fn imported_paths(report: &ImportReport) -> HashSet<String> {
    report
        .outcomes
        .iter()
        .filter(|outcome| outcome.status == oxidj_library::ImportStatus::Imported)
        .map(|outcome| outcome.path.clone())
        .collect()
}

pub(crate) struct Job {
    total: usize,
    done: Arc<AtomicUsize>,
    rx: Receiver<Vec<(u64, Result<WaveformOutcome, String>)>>,
}

fn spawn_worker<F>(spawn: F) -> Result<std::thread::JoinHandle<()>, String>
where
    F: FnOnce() -> std::io::Result<std::thread::JoinHandle<()>>,
{
    spawn().map_err(|error| format!("could not start waveform worker: {error}"))
}

impl Job {
    #[cfg(test)]
    pub(crate) fn finished_for_test(results: Vec<(u64, Result<WaveformOutcome, String>)>) -> Self {
        let total = results.len();
        let (tx, rx) = channel();
        tx.send(results).expect("test receiver is alive");
        Self { total, done: Arc::new(AtomicUsize::new(total)), rx }
    }

    pub(crate) fn start(db_path: PathBuf, ids: Vec<u64>) -> Result<Self, String> {
        Self::start_after_gate(db_path, ids, None)
    }

    pub(crate) fn start_after_gate(
        db_path: PathBuf,
        ids: Vec<u64>,
        gate: Option<Arc<std::sync::Barrier>>,
    ) -> Result<Self, String> {
        let total = ids.len();
        let done = Arc::new(AtomicUsize::new(0));
        let worker_done = done.clone();
        let (tx, rx) = channel();
        let _worker = spawn_worker(|| {
            std::thread::Builder::new()
                .name("oxidj-waveform-analysis".into())
                .spawn(move || {
                    if let Some(gate) = gate { gate.wait(); }
                    let mut results = Vec::with_capacity(ids.len());
                    match Catalog::open(&db_path) {
                        Ok(mut catalog) => {
                            for raw_id in ids {
                                let result = catalog
                                    .list_tracks()
                                    .map_err(|e| e.to_string())
                                    .and_then(|tracks| {
                                        tracks
                                            .into_iter()
                                            .find(|track| track.id.as_u64() == raw_id)
                                            .ok_or_else(|| "track no longer exists".to_owned())
                                    })
                                    .and_then(|track| {
                                        analyze_track_waveform(&mut catalog, track.id, false)
                                            .map_err(|e| e.to_string())
                                    });
                                results.push((raw_id, result));
                                worker_done.fetch_add(1, Ordering::Release);
                            }
                        }
                        Err(error) => {
                            for id in ids {
                                results.push((id, Err(format!("catalog unavailable: {error}"))));
                                worker_done.fetch_add(1, Ordering::Release);
                            }
                        }
                    }
                    let _ = tx.send(results);
                })
        })?;
        Ok(Self { total, done, rx })
    }

    pub(crate) fn total(&self) -> usize {
        self.total
    }
    pub(crate) fn completed(&self) -> usize {
        self.done.load(Ordering::Acquire)
    }
    pub(crate) fn poll(&self) -> Option<Result<Vec<(u64, Result<WaveformOutcome, String>)>, ()>> {
        match self.rx.try_recv() {
            Ok(v) => Some(Ok(v)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidj_library::ImportStatus;
    use std::time::{Duration, Instant};

    fn scratch() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "oxidj-ui-waveform-job-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(path.join("audio")).unwrap();
        path
    }

    #[test]
    fn only_imported_report_rows_are_selected_for_waveform_analysis() {
        let report = ImportReport {
            complete: true,
            cancelled: false,
            outcomes: vec![
                oxidj_library::FileOutcome { path: "new.wav".into(), status: ImportStatus::Imported },
                oxidj_library::FileOutcome { path: "conflict.wav".into(), status: ImportStatus::Conflict },
                oxidj_library::FileOutcome { path: "unreadable.wav".into(), status: ImportStatus::Unreadable },
                oxidj_library::FileOutcome { path: "unchanged.wav".into(), status: ImportStatus::Unchanged },
                oxidj_library::FileOutcome { path: "unsupported.wav".into(), status: ImportStatus::Unsupported },
            ],
        };
        assert_eq!(imported_paths(&report), ["new.wav".to_owned()].into());
    }

    #[test]
    fn worker_start_failure_is_returned_not_reported_as_analysis() {
        let error =
            spawn_worker(|| Err(std::io::Error::other("synthetic spawn refusal"))).unwrap_err();
        assert!(error.contains("could not start waveform worker"), "{error}");
        assert!(error.contains("synthetic spawn refusal"), "{error}");
    }

    #[test]
    fn source_change_is_reported_as_a_per_track_failure() {
        let root = scratch();
        let audio = root.join("audio");
        let source = audio.join("tone.wav");
        let mut wav = b"RIFF".to_vec();
        wav.extend((36u32 + 22_050u32 * 2).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(22_050u32.to_le_bytes());
        wav.extend(44_100u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((22_050u32 * 2).to_le_bytes());
        for i in 0..22_050 {
            wav.extend(
                (((i as f64 * 440.0 * std::f64::consts::TAU / 22_050.0).sin() * 16_000.0) as i16)
                    .to_le_bytes(),
            );
        }
        std::fs::write(&source, wav).unwrap();
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog
            .import_audio_directory(audio.to_str().unwrap())
            .unwrap();
        assert_eq!(report.outcomes[0].status, ImportStatus::Imported);
        let id = catalog.list_tracks().unwrap()[0].id.as_u64();
        std::fs::write(&source, b"source changed").unwrap();
        drop(catalog);
        let job = Job::start(db, vec![id]).unwrap();
        let start = Instant::now();
        loop {
            if let Some(result) = job.poll() {
                let results = result.unwrap();
                assert_eq!(results.len(), 1);
                assert!(
                    results[0]
                        .1
                        .as_ref()
                        .unwrap_err()
                        .contains("no longer matches"),
                    "{results:?}"
                );
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker returns per-track result"
            );
            std::thread::yield_now();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
