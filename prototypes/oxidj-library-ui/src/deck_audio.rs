//! M8/D2: decode a deck's track to RAM off the UI thread, with progress.
//!
//! The catalog's own decoders (`oxidj_library::pcm::decode_stereo`, the same
//! WAV/FLAC/MP3/AIFF matrix as import and analysis) fill an interleaved
//! stereo buffer on a worker thread; the deck polls [`Loading::poll`] once
//! per frame and hands the finished [`TrackPcm`] to the audio engine.

use oxidj_audio::TrackPcm;
use oxidj_library::pcm::{PcmError, PcmFormat, StereoSink, decode_stereo};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// A decode in progress.
pub(crate) struct Loading {
    /// `(frames decoded, frames declared or 0)`.
    progress: Arc<[AtomicU64; 2]>,
    rx: Receiver<Result<TrackPcm, String>>,
}

struct Collect {
    samples: Vec<f32>,
    rate: u32,
    progress: Arc<[AtomicU64; 2]>,
}

impl StereoSink for Collect {
    type Error = ();
    fn begin(&mut self, format: PcmFormat) -> Result<(), ()> {
        self.rate = format.sample_rate;
        if let Some(frames) = format.frames {
            self.progress[1].store(frames, Ordering::Relaxed);
            self.samples
                .reserve(usize::try_from(frames.saturating_mul(2)).unwrap_or(0));
        }
        Ok(())
    }
    fn push(&mut self, stereo: &[f32]) -> Result<(), ()> {
        self.samples.extend_from_slice(stereo);
        self.progress[0].store((self.samples.len() / 2) as u64, Ordering::Relaxed);
        Ok(())
    }
}

/// Why a track cannot play, in words for the deck.
pub(crate) fn reason(error: PcmError<()>) -> &'static str {
    match error {
        PcmError::Unreadable => "file missing or unreadable",
        PcmError::Unsupported => "audio format not supported",
        PcmError::Malformed | PcmError::Sink(()) => "audio file damaged",
    }
}

impl Loading {
    /// Start decoding `path` on a worker thread.
    pub(crate) fn spawn(path: PathBuf) -> Self {
        let progress: Arc<[AtomicU64; 2]> = Arc::new([AtomicU64::new(0), AtomicU64::new(0)]);
        let (tx, rx) = channel();
        let shared = progress.clone();
        let started = std::thread::Builder::new()
            .name("oxidj-deck-decode".into())
            .spawn(move || {
                let mut sink = Collect {
                    samples: Vec::new(),
                    rate: 0,
                    progress: shared,
                };
                // A vanished file reads as damaged to some decoders; say what it is.
                let result = match std::fs::File::open(&path)
                    .map_err(|_| PcmError::Unreadable)
                    .and_then(|_| decode_stereo(&path, &mut sink))
                {
                    Ok(_) => TrackPcm::new(sink.rate, sink.samples)
                        .ok_or_else(|| "no audio in file".to_owned()),
                    Err(error) => Err(reason(error).to_owned()),
                };
                let _ = tx.send(result);
            });
        if started.is_err() {
            let (tx, rx) = channel();
            let _ = tx.send(Err("could not start the decoder".to_owned()));
            return Self { progress, rx };
        }
        Self { progress, rx }
    }

    /// Fraction decoded (`None` while the length is unknown, e.g. MP3).
    pub(crate) fn fraction(&self) -> Option<f32> {
        let [done, total] = [0, 1].map(|i| self.progress[i].load(Ordering::Relaxed));
        (total > 0).then(|| (done as f64 / total as f64).clamp(0.0, 1.0) as f32)
    }

    /// The result once decoding finished.
    pub(crate) fn poll(&self) -> Option<Result<TrackPcm, String>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("decoder stopped".to_owned())),
        }
    }
}
