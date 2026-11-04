//! A1: AIFF / AIFF-C decode through the public `pcm::decode_mono` entry point.
//!
//! Every fixture is built byte by byte here. Expected samples are computed from
//! the generator's integer values (never by re-decoding), and every refusal is a
//! named test asserting the exact error kind and that the sink saw nothing.

use crate::pcm::{MONO_CHUNK_FRAMES, MonoSink, PcmError, PcmFormat, decode_mono};
use std::path::{Path, PathBuf};

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

fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "oxidj-aiff-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn decode(path: &Path) -> (Result<PcmFormat, PcmError<()>>, Collect) {
    let mut sink = Collect::default();
    let result = decode_mono(path, &mut sink);
    (result, sink)
}

// ---- byte-level fixture builder -------------------------------------------

/// 80-bit IEEE extended encoding of an integer rate (normalised mantissa).
fn ext80(rate: u32) -> [u8; 10] {
    let msb = 31 - rate.leading_zeros();
    let mut out = [0_u8; 10];
    out[0..2].copy_from_slice(&(16_383 + msb as u16).to_be_bytes());
    out[2..10].copy_from_slice(&((u64::from(rate)) << (63 - msb)).to_be_bytes());
    out
}

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn comm(
    channels: u16,
    frames: u32,
    bits: u16,
    rate: [u8; 10],
    compression: Option<&[u8; 4]>,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&channels.to_be_bytes());
    body.extend_from_slice(&frames.to_be_bytes());
    body.extend_from_slice(&bits.to_be_bytes());
    body.extend_from_slice(&rate);
    if let Some(kind) = compression {
        body.extend_from_slice(kind);
        body.extend_from_slice(&[4, b'n', b'o', b'n', b'e', 0]); // pstring + pad
    }
    chunk(b"COMM", &body)
}

fn ssnd(offset: u32, block: u32, data: &[u8]) -> Vec<u8> {
    let mut body = offset.to_be_bytes().to_vec();
    body.extend_from_slice(&block.to_be_bytes());
    body.extend(std::iter::repeat_n(0xA5, offset as usize)); // junk before frame 0
    body.extend_from_slice(data);
    chunk(b"SSND", &body)
}

fn form(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = kind.iter().copied().chain(chunks.concat()).collect();
    let mut out = b"FORM".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    out
}

/// Deterministic integer sample covering the full range including both
/// extremes (frame 0 = min, frame 1 = max).
fn generated(frame: usize, channel: usize, bits: u32) -> i64 {
    let half = 1_i64 << (bits - 1);
    match frame {
        0 => -half,
        1 => half - 1,
        _ => ((frame as i64 * 7_919 + channel as i64 * 104_729) % (2 * half)) - half,
    }
}

fn encode(value: i64, bits: u32, little: bool) -> Vec<u8> {
    let width = (bits / 8) as usize;
    let be = value.to_be_bytes();
    let mut out = be[8 - width..].to_vec();
    if little {
        out.reverse();
    }
    out
}

fn pcm_data(frames: usize, channels: usize, bits: u32, little: bool) -> Vec<u8> {
    (0..frames)
        .flat_map(|f| (0..channels).map(move |c| (f, c)))
        .flat_map(|(f, c)| encode(generated(f, c, bits), bits, little))
        .collect()
}

/// Oracle: mean of the generator values scaled by `2^(bits-1)`.
fn expected(frames: usize, channels: usize, bits: u32) -> Vec<f32> {
    let scale = (1_i64 << (bits - 1)) as f64;
    (0..frames)
        .map(|f| {
            let sum: f64 = (0..channels)
                .map(|c| generated(f, c, bits) as f64 / scale)
                .sum();
            (sum / channels as f64) as f32
        })
        .collect()
}

fn aiff(channels: u16, frames: u32, bits: u16, rate: u32) -> Vec<u8> {
    let data = pcm_data(frames as usize, channels as usize, u32::from(bits), false);
    form(
        b"AIFF",
        &[
            comm(channels, frames, bits, ext80(rate), None),
            ssnd(0, 0, &data),
        ],
    )
}

fn assert_decodes(path: &Path, rate: u32, channels: u16, want: &[f32]) {
    let (result, sink) = decode(path);
    let format = PcmFormat {
        sample_rate: rate,
        channels,
        frames: Some(want.len() as u64),
    };
    assert_eq!(result, Ok(format), "{path:?}");
    assert_eq!(sink.format, Some(format));
    assert_eq!(sink.samples, want, "{path:?}");
    assert!(sink.max_chunk <= MONO_CHUNK_FRAMES);
}

// ---- golden decodes ---------------------------------------------------------

#[test]
fn aiff_8_16_24_32_bit_mono_and_stereo_decode_exactly() {
    for bits in [8_u16, 16, 24, 32] {
        for channels in [1_u16, 2] {
            let frames = 300;
            let path = scratch("ramp.aiff", &aiff(channels, frames, bits, 44_100));
            let want = expected(frames as usize, channels as usize, u32::from(bits));
            assert_decodes(&path, 44_100, channels, &want);
        }
    }
}

#[test]
fn aiff_longer_than_one_chunk_streams_in_bounded_chunks() {
    let frames = (MONO_CHUNK_FRAMES * 2 + 7) as u32;
    let path = scratch("long.aif", &aiff(2, frames, 16, 48_000));
    let (result, sink) = decode(&path);
    assert!(result.is_ok());
    assert_eq!(sink.max_chunk, MONO_CHUNK_FRAMES);
    assert_eq!(sink.samples, expected(frames as usize, 2, 16));
}

#[test]
fn sine_16_bit_matches_generator() {
    let frames = 1_000;
    let values: Vec<i64> = (0..frames)
        .map(|i| {
            (f64::sin(i as f64 * 2.0 * std::f64::consts::PI * 440.0 / 22_050.0) * 32_767.0).round()
                as i64
        })
        .collect();
    let data: Vec<u8> = values.iter().flat_map(|&v| encode(v, 16, false)).collect();
    let bytes = form(
        b"AIFF",
        &[
            comm(1, frames as u32, 16, ext80(22_050), None),
            ssnd(0, 0, &data),
        ],
    );
    let want: Vec<f32> = values
        .iter()
        .map(|&v| (v as f64 / 32_768.0) as f32)
        .collect();
    assert_decodes(&scratch("sine.aiff", &bytes), 22_050, 1, &want);
}

#[test]
fn aifc_none_and_sowt_decode_exactly() {
    for (kind, little) in [(b"NONE", false), (b"sowt", true)] {
        for bits in [8_u32, 16, 24, 32] {
            let data = pcm_data(64, 2, bits, little);
            let bytes = form(
                b"AIFC",
                &[
                    comm(2, 64, bits as u16, ext80(96_000), Some(kind)),
                    ssnd(0, 0, &data),
                ],
            );
            assert_decodes(
                &scratch("c.aifc", &bytes),
                96_000,
                2,
                &expected(64, 2, bits),
            );
        }
    }
}

#[test]
fn aifc_fl32_and_fl64_decode_exactly() {
    let values = [0.0_f64, 0.5, -0.25, 1.0, -1.0, 0.125];
    let f32_data: Vec<u8> = values
        .iter()
        .flat_map(|&v| (v as f32).to_be_bytes())
        .collect();
    let f64_data: Vec<u8> = values.iter().flat_map(|&v| v.to_be_bytes()).collect();
    let mono: Vec<f32> = values.iter().map(|&v| v as f32).collect();
    let a = form(
        b"AIFC",
        &[
            comm(1, 6, 32, ext80(44_100), Some(b"fl32")),
            ssnd(0, 0, &f32_data),
        ],
    );
    assert_decodes(&scratch("f.aifc", &a), 44_100, 1, &mono);
    let b = form(
        b"AIFC",
        &[
            comm(1, 6, 64, ext80(44_100), Some(b"fl64")),
            ssnd(0, 0, &f64_data),
        ],
    );
    assert_decodes(&scratch("d.aifc", &b), 44_100, 1, &mono);
}

#[test]
fn odd_length_chunk_padding_is_skipped() {
    let data = pcm_data(10, 1, 16, false);
    let bytes = form(
        b"AIFF",
        &[
            chunk(b"NAME", b"abc"),
            comm(1, 10, 16, ext80(8_000), None),
            chunk(b"ANNO", b"x"),
            ssnd(0, 0, &data),
        ],
    );
    assert_decodes(&scratch("pad.aiff", &bytes), 8_000, 1, &expected(10, 1, 16));
}

#[test]
fn odd_8_bit_ssnd_as_last_chunk_without_trailing_pad_decodes() {
    // 3 frames of mono 8-bit: SSND body is 11 bytes; FORM size omits the pad.
    let data = pcm_data(3, 1, 8, false);
    let mut bytes = form(
        b"AIFF",
        &[comm(1, 3, 8, ext80(8_000), None), ssnd(0, 0, &data)],
    );
    bytes.pop();
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_be_bytes());
    assert_decodes(&scratch("nopad.aiff", &bytes), 8_000, 1, &expected(3, 1, 8));
}

#[test]
fn ssnd_offset_and_block_size_are_honoured() {
    let data = pcm_data(20, 2, 24, false);
    let bytes = form(
        b"AIFF",
        &[comm(2, 20, 24, ext80(44_100), None), ssnd(5, 8, &data)],
    );
    assert_decodes(
        &scratch("offset.aiff", &bytes),
        44_100,
        2,
        &expected(20, 2, 24),
    );
}

#[test]
fn ssnd_before_comm_decodes() {
    let data = pcm_data(10, 1, 16, false);
    let bytes = form(
        b"AIFF",
        &[ssnd(0, 0, &data), comm(1, 10, 16, ext80(44_100), None)],
    );
    assert_decodes(
        &scratch("order.aiff", &bytes),
        44_100,
        1,
        &expected(10, 1, 16),
    );
}

#[test]
fn aiff_is_recognised_by_magic_under_any_extension() {
    let bytes = aiff(1, 10, 16, 44_100);
    for name in ["x.AIFF", "x.aif", "x.aifc", "x.bin", "noext"] {
        assert_decodes(&scratch(name, &bytes), 44_100, 1, &expected(10, 1, 16));
    }
    let mut aifc = form(
        b"AIFC",
        &[
            comm(1, 10, 16, ext80(44_100), Some(b"NONE")),
            ssnd(0, 0, &pcm_data(10, 1, 16, false)),
        ],
    );
    assert_decodes(&scratch("y.dat", &aifc), 44_100, 1, &expected(10, 1, 16));
    // Not AIFF magic and not a known extension: still Unsupported.
    aifc[8..12].copy_from_slice(b"8SVX");
    assert_eq!(
        decode(&scratch("y.dat", &aifc)).0,
        Err(PcmError::Unsupported)
    );
}

// ---- mutation controls (the golden comparison must notice) ------------------

#[test]
fn mutation_control_flipped_endianness_breaks_golden() {
    let data = pcm_data(64, 2, 16, false); // big-endian bytes...
    let bytes = form(
        b"AIFC",
        &[
            comm(2, 64, 16, ext80(44_100), Some(b"sowt")),
            ssnd(0, 0, &data),
        ],
    );
    let (result, sink) = decode(&scratch("flip.aifc", &bytes)); // ...labelled little-endian
    assert!(result.is_ok());
    assert_ne!(sink.samples, expected(64, 2, 16));
}

#[test]
fn mutation_control_dropped_padding_breaks_golden() {
    let data = pcm_data(10, 1, 16, false);
    let mut body = b"AIFF".to_vec();
    let mut name = chunk(b"NAME", b"abc");
    name.pop(); // drop the pad byte after the odd chunk
    body.extend(name);
    body.extend(comm(1, 10, 16, ext80(8_000), None));
    body.extend(ssnd(0, 0, &data));
    let mut bytes = b"FORM".to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend(body);
    let (result, sink) = decode(&scratch("nopad.aiff", &bytes));
    assert!(result.is_err() || sink.samples != expected(10, 1, 16));
    assert!(
        sink.samples.is_empty(),
        "misaligned chunks must refuse, not emit"
    );
}

// ---- refusals: exact kind, no output -----------------------------------------

fn refuses(name: &str, bytes: &[u8], kind: PcmError<()>) {
    let path = scratch(name, bytes);
    let (result, sink) = decode(&path);
    assert_eq!(result, Err(kind), "{name}");
    assert_eq!(sink.format, None, "{name}: begin must not run");
    assert!(sink.samples.is_empty(), "{name}: no samples");
}

fn good() -> Vec<u8> {
    aiff(2, 10, 16, 44_100)
}

fn with_comm(c: Vec<u8>) -> Vec<u8> {
    form(b"AIFF", &[c, ssnd(0, 0, &pcm_data(10, 2, 16, false))])
}

fn with_rate(rate: [u8; 10]) -> Vec<u8> {
    with_comm(comm(2, 10, 16, rate, None))
}

fn ext(sign_exponent: u16, mantissa: u64) -> [u8; 10] {
    let mut out = [0_u8; 10];
    out[0..2].copy_from_slice(&sign_exponent.to_be_bytes());
    out[2..10].copy_from_slice(&mantissa.to_be_bytes());
    out
}

macro_rules! refusal {
    ($name:ident, $kind:ident, $bytes:expr) => {
        #[test]
        fn $name() {
            let bytes: Vec<u8> = $bytes;
            refuses(concat!(stringify!($name), ".aiff"), &bytes, PcmError::$kind);
        }
    };
}

refusal!(refuses_empty_file, Malformed, Vec::<u8>::new());
refusal!(
    refuses_truncated_form_header,
    Malformed,
    good()[..10].to_vec()
);
refusal!(refuses_non_aiff_magic_with_aiff_extension, Malformed, {
    let mut b = good();
    b[8..12].copy_from_slice(b"WAVE");
    b
});
refusal!(refuses_form_size_beyond_file, Malformed, {
    let mut b = good();
    b.truncate(b.len() - 1);
    b
});
refusal!(refuses_truncated_chunk_header, Malformed, {
    let mut b = form(b"AIFF", &[comm(2, 10, 16, ext80(44_100), None)]);
    b.extend_from_slice(b"SSN");
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
});
refusal!(refuses_chunk_longer_than_form, Malformed, {
    // Oversized SSND length (0xFFFFFFFF): must refuse without allocating.
    let mut b = good();
    let at = b.windows(4).position(|w| w == b"SSND").unwrap();
    b[at + 4..at + 8].copy_from_slice(&u32::MAX.to_be_bytes());
    b
});
refusal!(refuses_oversized_comm_length, Malformed, {
    let mut b = good();
    b[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
    b
});
refusal!(
    refuses_short_comm,
    Malformed,
    form(
        b"AIFF",
        &[chunk(b"COMM", &[0, 1, 0, 0]), ssnd(0, 0, &[0; 4])]
    )
);
refusal!(refuses_aifc_comm_without_compression, Malformed, {
    let c = comm(2, 10, 16, ext80(44_100), None);
    form(b"AIFC", &[c, ssnd(0, 0, &pcm_data(10, 2, 16, false))])
});
refusal!(
    refuses_zero_frames,
    Malformed,
    form(
        b"AIFF",
        &[comm(2, 0, 16, ext80(44_100), None), ssnd(0, 0, &[])]
    )
);
refusal!(
    refuses_frames_beyond_data,
    Malformed,
    with_comm(comm(2, 11, 16, ext80(44_100), None))
);
refusal!(
    refuses_huge_frame_count_without_allocating,
    Malformed,
    with_comm(comm(u16::MAX, u32::MAX, 32, ext80(44_100), None))
);
refusal!(
    refuses_zero_channels,
    Malformed,
    with_comm(comm(0, 10, 16, ext80(44_100), None))
);
refusal!(
    refuses_zero_sample_size,
    Malformed,
    with_comm(comm(2, 10, 0, ext80(44_100), None))
);
refusal!(
    refuses_sample_size_above_32,
    Malformed,
    with_comm(comm(2, 10, 40, ext80(44_100), None))
);
refusal!(
    refuses_12_bit_samples_as_unsupported,
    Unsupported,
    with_comm(comm(2, 10, 12, ext80(44_100), None))
);
refusal!(
    refuses_fl32_with_wrong_sample_size,
    Malformed,
    form(
        b"AIFC",
        &[
            comm(1, 2, 16, ext80(44_100), Some(b"fl32")),
            ssnd(0, 0, &[0; 8])
        ]
    )
);
refusal!(
    refuses_unknown_compression,
    Unsupported,
    form(
        b"AIFC",
        &[
            comm(2, 10, 16, ext80(44_100), Some(b"ima4")),
            ssnd(0, 0, &pcm_data(10, 2, 16, false))
        ]
    )
);
refusal!(refuses_zero_sample_rate, Malformed, with_rate(ext(0, 0)));
refusal!(
    refuses_nan_sample_rate,
    Malformed,
    with_rate(ext(0x7FFF, (1 << 63) | 1))
);
refusal!(
    refuses_infinite_sample_rate,
    Malformed,
    with_rate(ext(0x7FFF, 1 << 63))
);
refusal!(
    refuses_negative_sample_rate,
    Malformed,
    with_rate(ext(0xC00E, 0xAC44 << 48))
);
refusal!(
    refuses_huge_sample_rate,
    Unsupported,
    with_rate(ext(0x7FFE, 1 << 63))
);
refusal!(
    refuses_fractional_sample_rate,
    Unsupported,
    with_rate(ext(0x400E, 0xAC44_8000 << 32))
);
refusal!(
    refuses_missing_comm,
    Malformed,
    form(b"AIFF", &[ssnd(0, 0, &pcm_data(10, 2, 16, false))])
);
refusal!(
    refuses_missing_ssnd,
    Malformed,
    form(b"AIFF", &[comm(2, 10, 16, ext80(44_100), None)])
);
refusal!(
    refuses_duplicate_comm,
    Malformed,
    form(
        b"AIFF",
        &[
            comm(2, 10, 16, ext80(44_100), None),
            comm(2, 10, 16, ext80(44_100), None),
            ssnd(0, 0, &pcm_data(10, 2, 16, false))
        ]
    )
);
refusal!(
    refuses_duplicate_ssnd,
    Malformed,
    form(
        b"AIFF",
        &[
            comm(2, 10, 16, ext80(44_100), None),
            ssnd(0, 0, &pcm_data(10, 2, 16, false)),
            ssnd(0, 0, &pcm_data(10, 2, 16, false))
        ]
    )
);
refusal!(
    refuses_short_ssnd,
    Malformed,
    form(
        b"AIFF",
        &[
            comm(2, 10, 16, ext80(44_100), None),
            chunk(b"SSND", &[0; 4])
        ]
    )
);
refusal!(refuses_ssnd_offset_beyond_chunk, Malformed, {
    let mut b = good();
    let at = b.windows(4).position(|w| w == b"SSND").unwrap();
    b[at + 8..at + 12].copy_from_slice(&u32::MAX.to_be_bytes());
    b
});
refusal!(
    refuses_trailing_bytes_shorter_than_chunk_header,
    Malformed,
    {
        let mut b = good();
        b.extend_from_slice(&[0, 0, 0]);
        let size = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&size.to_be_bytes());
        b
    }
);

#[test]
fn missing_aiff_file_is_unreadable() {
    let path = std::env::temp_dir().join("oxidj-aiff-definitely-missing.aiff");
    assert_eq!(decode(&path).0, Err(PcmError::Unreadable));
}

#[test]
fn every_truncation_and_byte_corruption_returns_without_panicking() {
    let bytes = form(
        b"AIFC",
        &[
            chunk(b"FVER", &[0xA2, 0x80, 0x51, 0x40]),
            comm(2, 6, 24, ext80(44_100), Some(b"sowt")),
            chunk(b"NAME", b"abc"),
            ssnd(3, 0, &pcm_data(6, 2, 24, true)),
        ],
    );
    let path = scratch("fuzz.aifc", &bytes);
    for len in 0..bytes.len() {
        std::fs::write(&path, &bytes[..len]).unwrap();
        let (result, sink) = decode(&path);
        assert!(result.is_err(), "truncated to {len}");
        assert!(sink.samples.is_empty());
    }
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    for _ in 0..4_000 {
        let mut mutated = bytes.clone();
        for _ in 0..3 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let at = (state % mutated.len() as u64) as usize;
            mutated[at] = (state >> 32) as u8;
        }
        std::fs::write(&path, &mutated).unwrap();
        let _ = decode(&path);
    }
}
