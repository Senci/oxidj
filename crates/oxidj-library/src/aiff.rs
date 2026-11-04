//! Hand-rolled AIFF / AIFF-C container parser (A1). No dependency: the format is
//! two chunks (`COMM`, `SSND`) inside an IFF `FORM`, so a parser is smaller than
//! an admitted crate.
//!
//! [`parse`] validates the whole container from chunk headers alone before any
//! sample is produced: every length is checked against the file and the `FORM`,
//! nothing is allocated from an untrusted length (all reads go into fixed stack
//! buffers), and the declared frames must fit inside the `SSND` data. A file that
//! passes can therefore be streamed without further structural checks.
//!
//! Supported: integer PCM of 8/16/24/32 bits, big-endian (`AIFF`, AIFF-C `NONE`)
//! or little-endian (AIFF-C `sowt`), and AIFF-C `fl32`/`fl64` big-endian floats.
//! Any other compression type, sample size or a non-integer / above-768 kHz rate
//! is [`ImportStatus::Unsupported`]; structural defects are `Malformed`.

use crate::ImportStatus;
use std::io::{Read, Seek, SeekFrom};

/// Highest sample rate accepted (Hz).
pub(crate) const MAX_SAMPLE_RATE: u32 = 768_000;
/// Bound on chunks walked in one `FORM`.
const MAX_CHUNKS: usize = 4096;

/// How one sample is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Encoding {
    /// Signed two's-complement integer, big-endian (`AIFF`, AIFF-C `NONE`).
    BigEndian,
    /// Signed two's-complement integer, little-endian (AIFF-C `sowt`).
    LittleEndian,
    /// IEEE 754 binary32, big-endian (AIFF-C `fl32`).
    Float32,
    /// IEEE 754 binary64, big-endian (AIFF-C `fl64`).
    Float64,
}

/// Validated stream layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub channels: u16,
    pub frames: u64,
    pub bits: u16,
    pub sample_rate: u32,
    pub encoding: Encoding,
    /// Absolute file offset of the first sample frame (`SSND` offset applied).
    pub data_start: u64,
}

impl Layout {
    /// Bytes per stored sample (1..=8).
    pub fn sample_width(&self) -> usize {
        match self.encoding {
            Encoding::Float32 => 4,
            Encoding::Float64 => 8,
            _ => usize::from(self.bits / 8),
        }
    }

    /// Decode one stored sample (`bytes.len() == sample_width()`) to `[-1, 1)`.
    pub fn sample(&self, bytes: &[u8]) -> f64 {
        let mut raw = [0_u8; 8];
        raw[..bytes.len()].copy_from_slice(bytes);
        match self.encoding {
            Encoding::Float32 => f64::from(f32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]])),
            Encoding::Float64 => f64::from_be_bytes(raw),
            Encoding::BigEndian | Encoding::LittleEndian => {
                let width = bytes.len();
                if self.encoding == Encoding::LittleEndian {
                    raw[..width].reverse();
                }
                // Left-justify, then arithmetic shift to sign-extend.
                let value = i64::from_be_bytes(raw) >> (64 - 8 * width);
                value as f64 / f64::from(1_u32 << (8 * width - 1))
            }
        }
    }
}

/// `FORM` header naming `AIFF` or `AIFC`.
pub(crate) fn is_aiff_magic(header: &[u8]) -> bool {
    header.len() >= 12
        && &header[0..4] == b"FORM"
        && (&header[8..12] == b"AIFF" || &header[8..12] == b"AIFC")
}

fn read_at<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    buffer: &mut [u8],
) -> Result<(), ImportStatus> {
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|_| ImportStatus::Unreadable)?;
    reader.read_exact(buffer).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            ImportStatus::Malformed
        } else {
            ImportStatus::Unreadable
        }
    })
}

fn be_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The `COMM` fields this decoder uses.
struct Comm {
    channels: u16,
    frames: u32,
    bits: u16,
    rate: [u8; 10],
    compression: [u8; 4],
}

/// 80-bit IEEE 754 extended (sign, 15-bit exponent, 64-bit mantissa with an
/// explicit integer bit) to an integer sample rate.
pub(crate) fn sample_rate(bytes: [u8; 10]) -> Result<u32, ImportStatus> {
    let sign_exponent = be_u16(&bytes[0..2]);
    let mut mantissa = [0_u8; 8];
    mantissa.copy_from_slice(&bytes[2..10]);
    let mantissa = u64::from_be_bytes(mantissa);
    let exponent = sign_exponent & 0x7fff;
    // Negative, zero, denormal/unnormal, infinity and NaN are not rates.
    if sign_exponent & 0x8000 != 0 || exponent == 0 || exponent == 0x7fff || mantissa >> 63 == 0 {
        return Err(ImportStatus::Malformed);
    }
    let power = i32::from(exponent) - 16_383;
    if power < 0 {
        return Err(ImportStatus::Malformed); // below 1 Hz
    }
    if power > 31 {
        return Err(ImportStatus::Unsupported); // above u32, so above the cap
    }
    let shift = 63 - power as u32;
    if mantissa & ((1_u64 << shift) - 1) != 0 {
        return Err(ImportStatus::Unsupported); // fractional rate
    }
    let rate = u32::try_from(mantissa >> shift).map_err(|_| ImportStatus::Unsupported)?;
    if rate > MAX_SAMPLE_RATE {
        return Err(ImportStatus::Unsupported);
    }
    Ok(rate)
}

fn read_comm<R: Read + Seek>(
    reader: &mut R,
    body: u64,
    size: u64,
    aifc: bool,
) -> Result<Comm, ImportStatus> {
    let needed = if aifc { 22 } else { 18 };
    if size < needed {
        return Err(ImportStatus::Malformed);
    }
    let mut fields = [0_u8; 22];
    read_at(reader, body, &mut fields[..needed as usize])?;
    let mut rate = [0_u8; 10];
    rate.copy_from_slice(&fields[8..18]);
    let mut compression = *b"NONE";
    if aifc {
        compression.copy_from_slice(&fields[18..22]);
    }
    Ok(Comm {
        channels: be_u16(&fields[0..2]),
        frames: be_u32(&fields[2..6]),
        bits: be_u16(&fields[6..8]),
        rate,
        compression,
    })
}

/// Walk the container and validate it; see the module docs.
pub(crate) fn parse<R: Read + Seek>(reader: &mut R, file_len: u64) -> Result<Layout, ImportStatus> {
    if file_len < 12 {
        return Err(ImportStatus::Malformed);
    }
    let mut header = [0_u8; 12];
    read_at(reader, 0, &mut header)?;
    if !is_aiff_magic(&header) {
        return Err(ImportStatus::Malformed);
    }
    let aifc = &header[8..12] == b"AIFC";
    let form_end = 8 + u64::from(be_u32(&header[4..8]));
    if form_end < 12 || form_end > file_len {
        return Err(ImportStatus::Malformed);
    }
    let mut comm = None;
    let mut ssnd = None;
    let mut offset = 12_u64;
    let mut chunks = 0;
    while offset < form_end {
        chunks += 1;
        if chunks > MAX_CHUNKS || form_end - offset < 8 {
            return Err(ImportStatus::Malformed);
        }
        let mut head = [0_u8; 8];
        read_at(reader, offset, &mut head)?;
        let size = u64::from(be_u32(&head[4..8]));
        let body = offset + 8;
        let end = body + size;
        if end > form_end {
            return Err(ImportStatus::Malformed);
        }
        match &head[0..4] {
            b"COMM" => {
                if comm.is_some() {
                    return Err(ImportStatus::Malformed);
                }
                comm = Some(read_comm(reader, body, size, aifc)?);
            }
            b"SSND" => {
                if ssnd.is_some() || size < 8 {
                    return Err(ImportStatus::Malformed);
                }
                let mut fields = [0_u8; 8];
                read_at(reader, body, &mut fields)?;
                // blockSize (fields[4..8]) is an alignment hint only; the first
                // frame starts `offset` bytes into the sound data.
                let data_offset = u64::from(be_u32(&fields[0..4]));
                let available = (size - 8)
                    .checked_sub(data_offset)
                    .ok_or(ImportStatus::Malformed)?;
                ssnd = Some((body + 8 + data_offset, available));
            }
            _ => {}
        }
        // Odd-length chunks are followed by one pad byte (which a last chunk's
        // FORM size may omit; the loop then simply ends).
        offset = end + (size & 1);
    }
    let comm = comm.ok_or(ImportStatus::Malformed)?;
    let (data_start, available) = ssnd.ok_or(ImportStatus::Malformed)?;
    if comm.channels == 0 || comm.frames == 0 {
        return Err(ImportStatus::Malformed);
    }
    let sample_rate = sample_rate(comm.rate)?;
    let encoding = match &comm.compression {
        b"NONE" => Encoding::BigEndian,
        b"sowt" => Encoding::LittleEndian,
        b"fl32" | b"FL32" => Encoding::Float32,
        b"fl64" | b"FL64" => Encoding::Float64,
        _ => return Err(ImportStatus::Unsupported),
    };
    match (encoding, comm.bits) {
        (Encoding::Float32, 32) | (Encoding::Float64, 64) => {}
        (Encoding::Float32 | Encoding::Float64, _) => return Err(ImportStatus::Malformed),
        (_, 8 | 16 | 24 | 32) => {}
        (_, 1..=32) => return Err(ImportStatus::Unsupported),
        _ => return Err(ImportStatus::Malformed),
    }
    let layout = Layout {
        channels: comm.channels,
        frames: u64::from(comm.frames),
        bits: comm.bits,
        sample_rate,
        encoding,
        data_start,
    };
    let needed = layout
        .frames
        .checked_mul(u64::from(layout.channels) * layout.sample_width() as u64)
        .ok_or(ImportStatus::Malformed)?;
    if needed > available {
        return Err(ImportStatus::Malformed);
    }
    Ok(layout)
}

/// Parse the AIFF at `path` (importer facts; no samples read).
pub(crate) fn parse_file(path: &std::path::Path) -> Result<Layout, ImportStatus> {
    let mut file = std::fs::File::open(path).map_err(|_| ImportStatus::Unreadable)?;
    let len = file.metadata().map_err(|_| ImportStatus::Unreadable)?.len();
    parse(&mut file, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(sign_exponent: u16, mantissa: u64) -> [u8; 10] {
        let mut out = [0_u8; 10];
        out[0..2].copy_from_slice(&sign_exponent.to_be_bytes());
        out[2..10].copy_from_slice(&mantissa.to_be_bytes());
        out
    }

    #[test]
    fn extended_rates_decode_and_invalid_ones_are_typed() {
        // Known encodings: 44100 = 0x400E AC44..., 48000 = 0x400E BB80..., 8000.
        assert_eq!(sample_rate(ext(0x400E, 0xAC44 << 48)), Ok(44_100));
        assert_eq!(sample_rate(ext(0x400E, 0xBB80 << 48)), Ok(48_000));
        assert_eq!(sample_rate(ext(0x400B, 0xFA00 << 48)), Ok(8_000));
        assert_eq!(sample_rate(ext(0x3FFF, 1 << 63)), Ok(1));
        assert_eq!(sample_rate(ext(0x4012, 0xBB80 << 48)), Ok(MAX_SAMPLE_RATE));
        let malformed = [
            ext(0, 0),                          // zero
            ext(0x8000 | 0x400E, 0xAC44 << 48), // negative
            ext(0x7FFF, 1 << 63),               // infinity
            ext(0x7FFF, (1 << 63) | 1),         // NaN
            ext(0x400E, 0x2C44 << 48),          // unnormal (no integer bit)
            ext(0x3FFE, 1 << 63),               // 0.5 Hz
            ext(0x0000, 1),                     // denormal
        ];
        for bytes in malformed {
            assert_eq!(
                sample_rate(bytes),
                Err(ImportStatus::Malformed),
                "{bytes:?}"
            );
        }
        let unsupported = [
            ext(0x7FFE, 1 << 63),           // huge
            ext(0x401F, 1 << 63),           // 2^32
            ext(0x4013, 0xBB80 << 48),      // 48000 * 32 = 1.536 MHz
            ext(0x400E, 0xAC44_8000 << 32), // 44100.5
        ];
        for bytes in unsupported {
            assert_eq!(
                sample_rate(bytes),
                Err(ImportStatus::Unsupported),
                "{bytes:?}"
            );
        }
    }
}
