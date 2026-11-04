//! Streaming PCM decode (W2): the catalog's supported WAV / FLAC / MP3 decoders
//! (`hound` / `claxon` / `nanomp3`) and the hand-rolled AIFF / AIFF-C parser
//! ([`crate::aiff`], A1) feeding **mono** `f32` chunks to a caller sink.
//!
//! Nothing is buffered beyond one chunk of [`MONO_CHUNK_FRAMES`] frames, source
//! files are only opened for reading, and the supported matrix is the importer's
//! (integer PCM 8/16/24/32 and float32 WAV; FLAC; MPEG layer III; AIFF and
//! AIFF-C `NONE`/`sowt`/`fl32`/`fl64` at 8/16/24/32 bits). Channels are
//! mixed down by their arithmetic mean; integer samples are scaled by
//! `1 / 2^(bits - 1)`. Reservoir-dependent leading MP3 frames, whose PCM cannot
//! be reconstructed, are emitted as silence so frame counts match the importer.

use crate::{ImportStatus, PaddingStrippingReader, wav_container_padding_offsets};
use std::path::Path;

/// Frames per chunk handed to [`MonoSink::push`].
pub const MONO_CHUNK_FRAMES: usize = 4096;

/// Stream format reported to [`MonoSink::begin`] before any samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmFormat {
    pub sample_rate: u32,
    pub channels: u16,
    /// Frames (samples per channel) the container declares; `None` when unknown.
    pub frames: Option<u64>,
}

/// Why decoding stopped. `Sink` carries the sink's own refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmError<E> {
    /// The file could not be opened or read.
    Unreadable,
    /// The file is outside the supported format matrix.
    Unsupported,
    /// The file is structurally invalid or truncated.
    Malformed,
    Sink(E),
}

impl<E> PcmError<E> {
    pub(crate) fn status(status: ImportStatus) -> Self {
        match status {
            ImportStatus::Unreadable => Self::Unreadable,
            ImportStatus::Unsupported => Self::Unsupported,
            _ => Self::Malformed,
        }
    }
}

/// Receiver of decoded mono samples.
pub trait MonoSink {
    type Error;
    /// Called once, before the first [`push`](Self::push).
    fn begin(&mut self, format: PcmFormat) -> Result<(), Self::Error>;
    /// Next chunk of mono samples (never empty).
    fn push(&mut self, mono: &[f32]) -> Result<(), Self::Error>;
}

/// Mixes interleaved frames into a bounded mono chunk and flushes it to the sink.
struct Mixer {
    channels: u16,
    chunk: Vec<f32>,
    frames: u64,
}

impl Mixer {
    fn new<S: MonoSink>(sink: &mut S, format: PcmFormat) -> Result<Self, PcmError<S::Error>> {
        sink.begin(format).map_err(PcmError::Sink)?;
        Ok(Self {
            channels: format.channels,
            chunk: Vec::with_capacity(MONO_CHUNK_FRAMES),
            frames: 0,
        })
    }

    fn frame<S: MonoSink>(&mut self, sink: &mut S, sum: f64) -> Result<(), PcmError<S::Error>> {
        self.chunk.push((sum / f64::from(self.channels)) as f32);
        self.frames += 1;
        if self.chunk.len() == MONO_CHUNK_FRAMES {
            self.flush(sink)?;
        }
        Ok(())
    }

    fn flush<S: MonoSink>(&mut self, sink: &mut S) -> Result<(), PcmError<S::Error>> {
        if !self.chunk.is_empty() {
            sink.push(&self.chunk).map_err(PcmError::Sink)?;
            self.chunk.clear();
        }
        Ok(())
    }
}

/// Decode `path` (a `FORM`/`AIFF`|`AIFC` header always decodes as AIFF; otherwise
/// by extension: `.wav`, `.flac`, `.mp3`, `.aiff`/`.aif`/`.aifc`) into mono
/// chunks. Returns the format on success; on error the sink may have received a
/// prefix of the stream and must discard it (AIFF refuses every structural defect
/// before [`MonoSink::begin`], so only a read failure can leave a prefix).
pub fn decode_mono<S: MonoSink>(
    path: &Path,
    sink: &mut S,
) -> Result<PcmFormat, PcmError<S::Error>> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Content wins for AIFF: `FORM`/`AIFF`|`AIFC` bytes decode as AIFF whatever
    // the extension says (owner libraries hold mislabelled files).
    if has_aiff_magic(path) {
        return decode_aiff(path, sink);
    }
    match extension.as_str() {
        "wav" => decode_wav(path, sink),
        "flac" => decode_flac(path, sink),
        "aiff" | "aif" | "aifc" => decode_aiff(path, sink),
        "mp3" => {
            let mut mixer: Option<Mixer> = None;
            let (rate, channels, frames) = crate::mp3_stream(path, &mut |rate, channels, pcm| {
                let mixer = match &mut mixer {
                    Some(mixer) => mixer,
                    slot @ None => slot.insert(Mixer::new(
                        sink,
                        PcmFormat {
                            sample_rate: rate,
                            channels,
                            frames: None,
                        },
                    )?),
                };
                match pcm {
                    crate::Mp3Pcm::Interleaved(samples) => {
                        for frame in samples.chunks_exact(usize::from(channels)) {
                            mixer.frame(sink, frame.iter().map(|&s| f64::from(s)).sum())?;
                        }
                    }
                    crate::Mp3Pcm::Silence(count) => {
                        for _ in 0..count {
                            mixer.frame(sink, 0.0)?;
                        }
                    }
                }
                Ok(())
            })?;
            if let Some(mut mixer) = mixer {
                mixer.flush(sink)?;
            }
            Ok(PcmFormat {
                sample_rate: rate,
                channels,
                frames: Some(frames),
            })
        }
        _ => Err(PcmError::Unsupported),
    }
}

fn has_aiff_magic(path: &Path) -> bool {
    use std::io::Read;
    let mut header = [0_u8; 12];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok()
        && crate::aiff::is_aiff_magic(&header)
}

fn decode_aiff<S: MonoSink>(path: &Path, sink: &mut S) -> Result<PcmFormat, PcmError<S::Error>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|_| PcmError::Unreadable)?;
    let len = file.metadata().map_err(|_| PcmError::Unreadable)?.len();
    let layout = crate::aiff::parse(&mut file, len).map_err(PcmError::status)?;
    file.seek(SeekFrom::Start(layout.data_start))
        .map_err(|_| PcmError::Unreadable)?;
    let format = PcmFormat {
        sample_rate: layout.sample_rate,
        channels: layout.channels,
        frames: Some(layout.frames),
    };
    let mut reader = std::io::BufReader::with_capacity(1 << 16, file);
    let width = layout.sample_width();
    let mut bytes = [0_u8; 8];
    let mut mixer = Mixer::new(sink, format)?;
    for _ in 0..layout.frames {
        let mut sum = 0.0;
        for _ in 0..layout.channels {
            // `parse` proved the data is present; a short read means the file
            // changed underneath us.
            reader
                .read_exact(&mut bytes[..width])
                .map_err(|_| PcmError::Unreadable)?;
            sum += layout.sample(&bytes[..width]);
        }
        mixer.frame(sink, sum)?;
    }
    mixer.flush(sink)?;
    Ok(format)
}

fn decode_wav<S: MonoSink>(path: &Path, sink: &mut S) -> Result<PcmFormat, PcmError<S::Error>> {
    let (padding, supported) = wav_container_padding_offsets(path).ok_or(PcmError::Malformed)?;
    if !supported {
        return Err(PcmError::Unsupported);
    }
    let source = PaddingStrippingReader::open(path, padding).map_err(|_| PcmError::Unreadable)?;
    let mut reader =
        hound::WavReader::new(std::io::BufReader::new(source)).map_err(|_| PcmError::Malformed)?;
    let spec = reader.spec();
    let total = u64::from(reader.len());
    if spec.channels == 0 || spec.sample_rate == 0 || total % u64::from(spec.channels) != 0 {
        return Err(PcmError::Malformed);
    }
    let format = PcmFormat {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames: Some(total / u64::from(spec.channels)),
    };
    let mut mixer = Mixer::new(sink, format)?;
    let (mut sum, mut channel, mut seen) = (0.0_f64, 0_u16, 0_u64);
    let mut add = |value: f64, mixer: &mut Mixer, sink: &mut S| -> Result<(), PcmError<S::Error>> {
        sum += value;
        channel += 1;
        seen += 1;
        if channel == spec.channels {
            mixer.frame(sink, sum)?;
            sum = 0.0;
            channel = 0;
        }
        Ok(())
    };
    if spec.sample_format == hound::SampleFormat::Float {
        for sample in reader.samples::<f32>() {
            add(
                f64::from(sample.map_err(|_| PcmError::Malformed)?),
                &mut mixer,
                sink,
            )?;
        }
    } else {
        let scale = f64::from(1_u32 << (spec.bits_per_sample - 1));
        for sample in reader.samples::<i32>() {
            add(
                f64::from(sample.map_err(|_| PcmError::Malformed)?) / scale,
                &mut mixer,
                sink,
            )?;
        }
    }
    if seen != total {
        return Err(PcmError::Malformed);
    }
    mixer.flush(sink)?;
    Ok(format)
}

fn decode_flac<S: MonoSink>(path: &Path, sink: &mut S) -> Result<PcmFormat, PcmError<S::Error>> {
    let file = std::fs::File::open(path).map_err(|_| PcmError::Unreadable)?;
    let mut reader = claxon::FlacReader::new_ext(
        file,
        claxon::FlacReaderOptions {
            metadata_only: false,
            read_vorbis_comment: false,
        },
    )
    .map_err(|_| PcmError::Malformed)?;
    let info = reader.streaminfo();
    if info.sample_rate == 0
        || info.channels == 0
        || info.bits_per_sample == 0
        || info.bits_per_sample > 32
    {
        return Err(PcmError::Malformed);
    }
    let channels = u16::try_from(info.channels).map_err(|_| PcmError::Malformed)?;
    let declared = info.samples.filter(|&n| n != 0);
    let format = PcmFormat {
        sample_rate: info.sample_rate,
        channels,
        frames: declared,
    };
    let scale = 2_f64.powi(info.bits_per_sample as i32 - 1);
    let mut mixer = Mixer::new(sink, format)?;
    let mut blocks = reader.blocks();
    let mut buffer = Vec::new();
    loop {
        let block = match blocks.read_next_or_eof(buffer) {
            Ok(Some(block)) => block,
            Ok(None) => break,
            Err(_) => return Err(PcmError::Malformed),
        };
        if block.channels() != info.channels {
            return Err(PcmError::Malformed);
        }
        for index in 0..block.duration() {
            let sum: f64 = (0..block.channels())
                .map(|ch| f64::from(block.sample(ch, index)) / scale)
                .sum();
            mixer.frame(sink, sum)?;
        }
        buffer = block.into_buffer();
    }
    if mixer.frames == 0 || declared.is_some_and(|n| n != mixer.frames) {
        return Err(PcmError::Malformed);
    }
    mixer.flush(sink)?;
    Ok(format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Collect {
        format: Option<PcmFormat>,
        samples: Vec<f32>,
        max_chunk: usize,
    }

    impl MonoSink for Collect {
        type Error = ();
        fn begin(&mut self, format: PcmFormat) -> Result<(), ()> {
            assert!(self.format.replace(format).is_none(), "begin called twice");
            Ok(())
        }
        fn push(&mut self, mono: &[f32]) -> Result<(), ()> {
            assert!(self.format.is_some() && !mono.is_empty());
            self.max_chunk = self.max_chunk.max(mono.len());
            self.samples.extend_from_slice(mono);
            Ok(())
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oxidj-pcm-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn wav_stereo_is_mixed_to_mono_in_bounded_chunks() {
        let path = scratch("stereo.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        let frames = MONO_CHUNK_FRAMES * 2 + 7;
        for i in 0..frames {
            writer.write_sample((i % 100) as i16 * 100).unwrap();
            writer.write_sample(-((i % 50) as i16) * 100).unwrap();
        }
        writer.finalize().unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut sink = Collect::default();
        let format = decode_mono(&path, &mut sink).unwrap();
        assert_eq!(
            format,
            PcmFormat {
                sample_rate: 48_000,
                channels: 2,
                frames: Some(frames as u64)
            }
        );
        assert_eq!(sink.samples.len(), frames);
        assert_eq!(sink.max_chunk, MONO_CHUNK_FRAMES);
        for (i, &s) in sink.samples.iter().enumerate() {
            let expected =
                (((i % 100) as f64 * 100.0 - (i % 50) as f64 * 100.0) / 2.0 / 32_768.0) as f32;
            assert_eq!(s, expected);
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn flac_and_mp3_fixtures_decode_with_importer_frame_counts() {
        let flac = scratch("tone.flac");
        std::fs::write(&flac, include_bytes!("../tests/fixtures/tone.flac")).unwrap();
        let mut sink = Collect::default();
        let format = decode_mono(&flac, &mut sink).unwrap();
        assert_eq!((format.sample_rate, format.channels), (44_100, 1));
        assert!(!sink.samples.is_empty());
        assert!(sink.samples.iter().any(|s| s.abs() > 0.01));
        let facts = crate::decode_flac_facts(&flac).ok().unwrap();
        let duration = sink.samples.len() as f64 / f64::from(format.sample_rate);
        assert!((duration - facts.duration).abs() < 1e-9);

        for name in ["tone.mp3", "tone-stereo.mp3"] {
            let mp3 = scratch(name);
            let bytes: &[u8] = if name == "tone.mp3" {
                include_bytes!("../tests/fixtures/tone.mp3")
            } else {
                include_bytes!("../tests/fixtures/tone-stereo.mp3")
            };
            std::fs::write(&mp3, bytes).unwrap();
            let mut sink = Collect::default();
            let format = decode_mono(&mp3, &mut sink).unwrap();
            let facts = crate::decode_mp3_facts(&mp3).ok().unwrap();
            assert_eq!(format.channels, facts.channels);
            assert_eq!(format.frames, Some(sink.samples.len() as u64));
            let duration = sink.samples.len() as f64 / f64::from(format.sample_rate);
            assert!((duration - facts.duration).abs() < 1e-9, "{name}");
        }
    }

    #[test]
    fn corrupt_truncated_and_unknown_inputs_are_typed_refusals() {
        let truncated = scratch("truncated.flac");
        std::fs::write(
            &truncated,
            include_bytes!("../tests/fixtures/tone-total-truncated.flac"),
        )
        .unwrap();
        let garbage = scratch("garbage.wav");
        std::fs::write(&garbage, b"RIFF\x04\0\0\0WAVE").unwrap();
        let other = scratch("tone.ogg");
        std::fs::write(&other, b"OggS").unwrap();
        let missing = scratch("missing.flac");
        let cases = [
            (truncated, PcmError::Malformed),
            (garbage, PcmError::Malformed),
            (other, PcmError::Unsupported),
            (missing, PcmError::Unreadable),
        ];
        for (path, expected) in cases {
            assert_eq!(
                decode_mono(&path, &mut Collect::default()),
                Err(expected),
                "{path:?}"
            );
        }
    }

    #[test]
    fn sink_refusal_stops_decoding() {
        struct Refuse;
        impl MonoSink for Refuse {
            type Error = &'static str;
            fn begin(&mut self, _: PcmFormat) -> Result<(), &'static str> {
                Err("no")
            }
            fn push(&mut self, _: &[f32]) -> Result<(), &'static str> {
                unreachable!()
            }
        }
        let flac = scratch("tone.flac");
        std::fs::write(&flac, include_bytes!("../tests/fixtures/tone.flac")).unwrap();
        assert_eq!(decode_mono(&flac, &mut Refuse), Err(PcmError::Sink("no")));
    }
}
