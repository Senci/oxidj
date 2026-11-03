//! Read-only Rekordbox ANLZ analysis-file decoder (M7 slice 3): the beat grid
//! (`PQTZ`), cue lists (`PCOB`) and extended cue lists (`PCO2`). Big-endian tagged
//! sections per the public Deep Symmetry `rekordbox_anlz` reference. Every length is
//! checked before use; unknown, duplicated or malformed sections reject the whole
//! file. The `PWV3` waveform detail and `PWV5` colour waveform detail (W2) are
//! decoded as vendor-derived data, never as OxiDJ-computed peaks. Other
//! recognised-but-undecoded sections (other waveforms, VBR index, song structure,
//! extended grid) are length-checked and skipped. Nothing is ever written.

/// Analysis files larger than this are refused.
pub const ANLZ_MAX_BYTES: usize = 64 << 20;

/// One beat-grid entry exactly as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnlzBeat {
    /// Position in the bar as stored (Rekordbox grids number beats 1..=4).
    pub beat_in_bar: u16,
    pub tempo_hundredths_bpm: u16,
    pub time_ms: u32,
}

/// One cue or loop exactly as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnlzCue {
    /// `0` = memory cue; `n >= 1` = hot cue `n` (A = 1).
    pub hot_cue: u32,
    pub start_ms: u32,
    /// `Some` only for a loop entry.
    pub loop_end_ms: Option<u32>,
    /// Extended-entry comment; `None` when not stored or empty.
    pub comment: Option<String>,
    /// Extended-entry display colour bytes, when the entry stores them.
    pub rgb: Option<[u8; 3]>,
}

/// One `PWV3` waveform-detail column (vendor-derived): low 5 bits height, high 3
/// bits whiteness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnlzDetailColumn {
    /// 0..=31.
    pub height: u8,
    /// 0..=7.
    pub whiteness: u8,
}

/// One `PWV5` colour-waveform-detail column (vendor-derived): a big-endian `u16`
/// of red (bits 15-13), green (12-10), blue (9-7), height (6-2); bits 1-0 unused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnlzColourColumn {
    /// 0..=7.
    pub red: u8,
    /// 0..=7.
    pub green: u8,
    /// 0..=7.
    pub blue: u8,
    /// 0..=31.
    pub height: u8,
}

/// The decoded sections of one ANLZ file. `None` = the section is not present.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnlzFile {
    /// `PPTH`: the audio path the analysis belongs to.
    pub path: Option<String>,
    pub beat_grid: Option<Vec<AnlzBeat>>,
    /// `PCOB` lists (memory then hot cues in file order).
    pub cues: Option<Vec<AnlzCue>>,
    /// `PCO2` lists in file order.
    pub extended_cues: Option<Vec<AnlzCue>>,
    /// `PWV3` waveform detail (vendor-derived).
    pub waveform_detail: Option<Vec<AnlzDetailColumn>>,
    /// `PWV5` colour waveform detail (vendor-derived).
    pub colour_waveform_detail: Option<Vec<AnlzColourColumn>>,
}

/// A refusal; messages never contain source values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnlzError(pub &'static str);
impl std::fmt::Display for AnlzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for AnlzError {}

const SKIPPED: &[&[u8; 4]] = &[
    b"PVBR", b"PWAV", b"PWV2", b"PWV4", b"PWV6", b"PWV7", b"PWVC", b"PQT2", b"PSSI",
];

fn slice(bytes: &[u8], at: usize, len: usize) -> Result<&[u8], AnlzError> {
    at.checked_add(len)
        .and_then(|end| bytes.get(at..end))
        .ok_or(AnlzError("truncated ANLZ field"))
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, AnlzError> {
    Ok(u16::from_be_bytes(
        slice(bytes, at, 2)?.try_into().expect("2 bytes"),
    ))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, AnlzError> {
    Ok(u32::from_be_bytes(
        slice(bytes, at, 4)?.try_into().expect("4 bytes"),
    ))
}
fn size(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn utf16be(bytes: &[u8]) -> Result<String, AnlzError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(AnlzError("odd-length ANLZ UTF-16 string"));
    }
    let mut units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_be_bytes(*c))
        .collect();
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16(&units).map_err(|_| AnlzError("invalid ANLZ UTF-16 string"))
}

/// Split `section` (whose header is `header` bytes) into `count` tagged entries.
fn entries<'a>(
    section: &'a [u8],
    header: usize,
    count: usize,
    tag: &[u8; 4],
    min_len: usize,
) -> Result<Vec<&'a [u8]>, AnlzError> {
    let mut out = Vec::with_capacity(count.min(64));
    let mut at = header;
    for _ in 0..count {
        if slice(section, at, 4)? != tag {
            return Err(AnlzError("unexpected ANLZ cue entry tag"));
        }
        let entry_header = size(u32_at(section, at + 4)?);
        let entry_len = size(u32_at(section, at + 8)?);
        if entry_header < 12 || entry_len < entry_header || entry_len < min_len {
            return Err(AnlzError("malformed ANLZ cue entry length"));
        }
        out.push(slice(section, at, entry_len)?);
        at += entry_len;
    }
    if at != section.len() {
        return Err(AnlzError("ANLZ cue list length mismatch"));
    }
    Ok(out)
}

fn cue(hot_cue: u32, kind: u8, start_ms: u32, loop_ms: u32) -> Result<AnlzCue, AnlzError> {
    let loop_end_ms = match kind {
        1 => None,
        2 if loop_ms != u32::MAX && loop_ms > start_ms => Some(loop_ms),
        2 => return Err(AnlzError("ANLZ loop end is not after its start")),
        _ => return Err(AnlzError("unknown ANLZ cue type")),
    };
    Ok(AnlzCue {
        hot_cue,
        start_ms,
        loop_end_ms,
        comment: None,
        rgb: None,
    })
}

/// Validate one cue list's type against its entries and reject repeated hot cues.
fn check_list(list_type: u32, cues: &[AnlzCue]) -> Result<(), AnlzError> {
    let mut hot = std::collections::HashSet::new();
    for cue in cues {
        match (list_type, cue.hot_cue) {
            (0, 0) => {}
            (1, n) if n >= 1 => {
                if !hot.insert(n) {
                    return Err(AnlzError("duplicate ANLZ hot cue"));
                }
            }
            _ => return Err(AnlzError("ANLZ cue does not match its list type")),
        }
    }
    Ok(())
}

/// Entry bytes of a `PWV3`/`PWV5` section: header at least 24 bytes, the stored
/// entry size must equal `entry_size` and `count * entry_size` must fill the body.
fn waveform_entries(section: &[u8], header: usize, entry_size: u32) -> Result<&[u8], AnlzError> {
    if header < 24 {
        return Err(AnlzError("malformed ANLZ waveform section"));
    }
    if u32_at(section, 12)? != entry_size {
        return Err(AnlzError("unknown ANLZ waveform entry size"));
    }
    let count = size(u32_at(section, 16)?);
    if count
        .checked_mul(size(entry_size))
        .and_then(|n| n.checked_add(header))
        != Some(section.len())
    {
        return Err(AnlzError("ANLZ waveform length mismatch"));
    }
    slice(section, header, section.len() - header)
}

/// Decode one ANLZ file (`.DAT` or `.EXT`).
pub fn parse_anlz(bytes: &[u8]) -> Result<AnlzFile, AnlzError> {
    if bytes.len() > ANLZ_MAX_BYTES {
        return Err(AnlzError("ANLZ file is too large"));
    }
    if slice(bytes, 0, 4)? != b"PMAI" {
        return Err(AnlzError("not an ANLZ file"));
    }
    let header = size(u32_at(bytes, 4)?);
    if size(u32_at(bytes, 8)?) != bytes.len() || header < 12 || header > bytes.len() {
        return Err(AnlzError("malformed ANLZ file header"));
    }
    let mut file = AnlzFile::default();
    let mut cue_lists = std::collections::HashSet::new();
    let mut extended_lists = std::collections::HashSet::new();
    let mut at = header;
    while at < bytes.len() {
        let tag: [u8; 4] = slice(bytes, at, 4)?.try_into().expect("4 bytes");
        let section_header = size(u32_at(bytes, at + 4)?);
        let section_len = size(u32_at(bytes, at + 8)?);
        if section_header < 12 || section_len < section_header {
            return Err(AnlzError("malformed ANLZ section length"));
        }
        let section = slice(bytes, at, section_len)?;
        match &tag {
            b"PPTH" => {
                if file.path.is_some() || section_header < 16 {
                    return Err(AnlzError("duplicate or malformed ANLZ path section"));
                }
                let len = size(u32_at(section, 12)?);
                if section_header.checked_add(len) != Some(section_len) {
                    return Err(AnlzError("ANLZ path length mismatch"));
                }
                file.path = Some(utf16be(slice(section, section_header, len)?)?);
            }
            b"PQTZ" => {
                if file.beat_grid.is_some() || section_header < 24 {
                    return Err(AnlzError("duplicate or malformed ANLZ beat grid"));
                }
                let count = size(u32_at(section, 20)?);
                if count
                    .checked_mul(8)
                    .and_then(|n| n.checked_add(section_header))
                    != Some(section_len)
                {
                    return Err(AnlzError("ANLZ beat grid length mismatch"));
                }
                file.beat_grid = Some(
                    (0..count)
                        .map(|i| {
                            let at = section_header + 8 * i;
                            Ok(AnlzBeat {
                                beat_in_bar: u16_at(section, at)?,
                                tempo_hundredths_bpm: u16_at(section, at + 2)?,
                                time_ms: u32_at(section, at + 4)?,
                            })
                        })
                        .collect::<Result<_, AnlzError>>()?,
                );
            }
            b"PCOB" => {
                if section_header < 24 {
                    return Err(AnlzError("malformed ANLZ cue list"));
                }
                let list_type = u32_at(section, 12)?;
                if !cue_lists.insert(list_type) {
                    return Err(AnlzError("duplicate ANLZ cue list"));
                }
                let count = usize::from(u16_at(section, 18)?);
                let cues = entries(section, section_header, count, b"PCPT", 40)?
                    .into_iter()
                    .map(|e| cue(u32_at(e, 12)?, e[28], u32_at(e, 32)?, u32_at(e, 36)?))
                    .collect::<Result<Vec<_>, _>>()?;
                check_list(list_type, &cues)?;
                file.cues.get_or_insert_with(Vec::new).extend(cues);
            }
            b"PCO2" => {
                if section_header < 20 {
                    return Err(AnlzError("malformed ANLZ extended cue list"));
                }
                let list_type = u32_at(section, 12)?;
                if !extended_lists.insert(list_type) {
                    return Err(AnlzError("duplicate ANLZ extended cue list"));
                }
                let count = usize::from(u16_at(section, 16)?);
                let cues = entries(section, section_header, count, b"PCP2", 28)?
                    .into_iter()
                    .map(|e| {
                        let mut c = cue(u32_at(e, 12)?, e[16], u32_at(e, 20)?, u32_at(e, 24)?)?;
                        if e.len() >= 44 {
                            let len = size(u32_at(e, 40)?);
                            let text = utf16be(slice(e, 44, len)?)?;
                            c.comment = (!text.is_empty()).then_some(text);
                            if let Ok(colour) = slice(e, 44 + len, 4) {
                                c.rgb = Some([colour[1], colour[2], colour[3]]);
                            }
                        }
                        Ok(c)
                    })
                    .collect::<Result<Vec<_>, AnlzError>>()?;
                check_list(list_type, &cues)?;
                file.extended_cues.get_or_insert_with(Vec::new).extend(cues);
            }
            b"PWV3" => {
                if file.waveform_detail.is_some() {
                    return Err(AnlzError("duplicate ANLZ waveform detail"));
                }
                let body = waveform_entries(section, section_header, 1)?;
                file.waveform_detail = Some(
                    body.iter()
                        .map(|&b| AnlzDetailColumn {
                            height: b & 0x1f,
                            whiteness: b >> 5,
                        })
                        .collect(),
                );
            }
            b"PWV5" => {
                if file.colour_waveform_detail.is_some() {
                    return Err(AnlzError("duplicate ANLZ colour waveform detail"));
                }
                let body = waveform_entries(section, section_header, 2)?;
                file.colour_waveform_detail = Some(
                    body.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|&pair| {
                            let v = u16::from_be_bytes(pair);
                            AnlzColourColumn {
                                red: (v >> 13) as u8 & 7,
                                green: (v >> 10) as u8 & 7,
                                blue: (v >> 7) as u8 & 7,
                                height: (v >> 2) as u8 & 0x1f,
                            }
                        })
                        .collect(),
                );
            }
            other if SKIPPED.contains(&other) => {}
            _ => return Err(AnlzError("unknown ANLZ section")),
        }
        at += section_len;
    }
    Ok(file)
}

/// Synthetic ANLZ builder (test-only; layouts per the public format reference).
#[cfg(test)]
pub(crate) mod fixture {
    pub fn section(tag: &[u8; 4], header_rest: &[u8], body: &[u8]) -> Vec<u8> {
        let header = 12 + header_rest.len();
        let mut out = tag.to_vec();
        out.extend((header as u32).to_be_bytes());
        out.extend(((header + body.len()) as u32).to_be_bytes());
        out.extend(header_rest);
        out.extend(body);
        out
    }
    pub fn file(sections: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = sections.concat();
        let mut out = b"PMAI".to_vec();
        out.extend(28u32.to_be_bytes());
        out.extend(((28 + body.len()) as u32).to_be_bytes());
        out.extend([0u8; 16]);
        out.extend(body);
        out
    }
    pub fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .chain([0])
            .flat_map(|u| u.to_be_bytes())
            .collect()
    }
    pub fn ppth(path: &str) -> Vec<u8> {
        let body = utf16(path);
        section(b"PPTH", &(body.len() as u32).to_be_bytes(), &body)
    }
    pub fn pqtz(beats: &[(u16, u16, u32)]) -> Vec<u8> {
        let mut rest = vec![0u8; 4];
        rest.extend(0x0008_0000u32.to_be_bytes());
        rest.extend((beats.len() as u32).to_be_bytes());
        let body: Vec<u8> = beats
            .iter()
            .flat_map(|(beat, tempo, time)| {
                [
                    &beat.to_be_bytes()[..],
                    &tempo.to_be_bytes(),
                    &time.to_be_bytes(),
                ]
                .concat()
            })
            .collect();
        section(b"PQTZ", &rest, &body)
    }
    /// `PCPT`: hot cue, type (1 point / 2 loop), time ms, loop end ms (or u32::MAX).
    pub fn pcpt(hot: u32, kind: u8, time: u32, loop_end: u32) -> Vec<u8> {
        let mut e = b"PCPT".to_vec();
        e.extend(28u32.to_be_bytes());
        e.extend(56u32.to_be_bytes());
        e.extend(hot.to_be_bytes());
        e.extend(0u32.to_be_bytes()); // status
        e.extend(0x0001_0000u32.to_be_bytes());
        e.extend([0xff, 0xff, 0xff, 0xff]); // order first/last
        e.extend([kind, 0]);
        e.extend(1000u16.to_be_bytes());
        e.extend(time.to_be_bytes());
        e.extend(loop_end.to_be_bytes());
        e.resize(56, 0);
        e
    }
    pub fn pcob(list_type: u32, entries: &[Vec<u8>]) -> Vec<u8> {
        let mut rest = list_type.to_be_bytes().to_vec();
        rest.extend(0u16.to_be_bytes());
        rest.extend((entries.len() as u16).to_be_bytes());
        rest.extend(u32::MAX.to_be_bytes());
        section(b"PCOB", &rest, &entries.concat())
    }
    /// `PCP2` with an optional comment and optional colour bytes.
    pub fn pcp2(
        hot: u32,
        kind: u8,
        time: u32,
        loop_end: u32,
        comment: &str,
        rgb: Option<[u8; 3]>,
    ) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend(hot.to_be_bytes());
        e.extend([kind, 0, 0, 0]);
        e.extend(time.to_be_bytes());
        e.extend(loop_end.to_be_bytes());
        e.extend([0u8; 8]); // colour id + padding
        e.extend([0u8; 4]); // loop numerator/denominator
        let text = if comment.is_empty() {
            vec![]
        } else {
            utf16(comment)
        };
        e.extend((text.len() as u32).to_be_bytes());
        e.extend(&text);
        if let Some([r, g, b]) = rgb {
            e.extend([1, r, g, b]);
        }
        let mut out = b"PCP2".to_vec();
        out.extend(16u32.to_be_bytes());
        out.extend(((12 + e.len()) as u32).to_be_bytes());
        out.extend(e);
        out
    }
    /// `PWV3` (entry size 1) or `PWV5` (entry size 2) with raw entry bytes.
    pub fn pwv(tag: &[u8; 4], entry_size: u32, entries: &[u8]) -> Vec<u8> {
        let mut rest = entry_size.to_be_bytes().to_vec();
        rest.extend((entries.len() as u32 / entry_size).to_be_bytes());
        rest.extend(0x0096_0000u32.to_be_bytes());
        section(tag, &rest, entries)
    }
    pub fn pco2(list_type: u32, entries: &[Vec<u8>]) -> Vec<u8> {
        let mut rest = list_type.to_be_bytes().to_vec();
        rest.extend((entries.len() as u16).to_be_bytes());
        rest.extend(0u16.to_be_bytes());
        section(b"PCO2", &rest, &entries.concat())
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;

    fn full() -> Vec<u8> {
        file(&[
            ppth("/Contents/a.mp3"),
            section(b"PVBR", &[0; 4], &[7; 16]),
            pqtz(&[(1, 12_800, 25), (2, 12_800, 494), (3, 12_850, 962)]),
            pcob(1, &[pcpt(1, 1, 1_000, u32::MAX), pcpt(2, 2, 2_000, 4_000)]),
            pcob(0, &[pcpt(0, 1, 500, 0)]),
            section(b"PWAV", &[0; 8], &[1; 10]),
            pco2(
                1,
                &[
                    pcp2(1, 1, 1_000, u32::MAX, "Drop", Some([40, 226, 20])),
                    pcp2(2, 2, 2_000, 4_000, "", None),
                ],
            ),
        ])
    }

    #[test]
    fn present_sections_decode_exactly_as_stored() {
        let f = parse_anlz(&full()).unwrap();
        assert_eq!(f.path.as_deref(), Some("/Contents/a.mp3"));
        assert_eq!(
            f.beat_grid.unwrap(),
            vec![
                AnlzBeat {
                    beat_in_bar: 1,
                    tempo_hundredths_bpm: 12_800,
                    time_ms: 25
                },
                AnlzBeat {
                    beat_in_bar: 2,
                    tempo_hundredths_bpm: 12_800,
                    time_ms: 494
                },
                AnlzBeat {
                    beat_in_bar: 3,
                    tempo_hundredths_bpm: 12_850,
                    time_ms: 962
                },
            ]
        );
        let point = |hot, start| AnlzCue {
            hot_cue: hot,
            start_ms: start,
            loop_end_ms: None,
            comment: None,
            rgb: None,
        };
        let looped = AnlzCue {
            loop_end_ms: Some(4_000),
            ..point(2, 2_000)
        };
        // A point cue's stored loop field (0 here) is ignored, not turned into an end.
        assert_eq!(
            f.cues.unwrap(),
            vec![point(1, 1_000), looped.clone(), point(0, 500)]
        );
        assert_eq!(
            f.extended_cues.unwrap(),
            vec![
                AnlzCue {
                    comment: Some("Drop".into()),
                    rgb: Some([40, 226, 20]),
                    ..point(1, 1_000)
                },
                looped
            ]
        );
    }

    #[test]
    fn absent_and_empty_sections_are_distinguished() {
        let absent = parse_anlz(&file(&[ppth("/a")])).unwrap();
        assert_eq!(
            (absent.beat_grid, absent.cues, absent.extended_cues),
            (None, None, None)
        );
        let empty =
            parse_anlz(&file(&[ppth("/a"), pqtz(&[]), pcob(1, &[]), pco2(0, &[])])).unwrap();
        assert_eq!(empty.beat_grid, Some(vec![]));
        assert_eq!(empty.cues, Some(vec![]));
        assert_eq!(empty.extended_cues, Some(vec![]));
        assert_eq!(parse_anlz(&file(&[])).unwrap(), AnlzFile::default());
    }

    #[test]
    fn unknown_duplicate_and_malformed_sections_fail_closed() {
        let cases: Vec<(Vec<u8>, &str)> = vec![
            (file(&[section(b"PXYZ", &[], &[])]), "unknown ANLZ section"),
            (
                file(&[pqtz(&[(1, 1, 1)]), pqtz(&[(1, 1, 1)])]),
                "duplicate or malformed ANLZ beat grid",
            ),
            (
                file(&[ppth("/a"), ppth("/a")]),
                "duplicate or malformed ANLZ path section",
            ),
            (
                file(&[pcob(1, &[]), pcob(1, &[])]),
                "duplicate ANLZ cue list",
            ),
            (
                file(&[pco2(0, &[]), pco2(0, &[])]),
                "duplicate ANLZ extended cue list",
            ),
            (
                file(&[pcob(1, &[pcpt(1, 1, 5, 0), pcpt(1, 1, 9, 0)])]),
                "duplicate ANLZ hot cue",
            ),
            (
                file(&[pcob(0, &[pcpt(3, 1, 5, 0)])]),
                "ANLZ cue does not match its list type",
            ),
            (
                file(&[pcob(1, &[pcpt(1, 2, 5, 5)])]),
                "ANLZ loop end is not after its start",
            ),
            (
                file(&[pco2(1, &[pcp2(1, 2, 5, u32::MAX, "", None)])]),
                "ANLZ loop end is not after its start",
            ),
            (
                file(&[pcob(1, &[pcpt(1, 3, 5, 0)])]),
                "unknown ANLZ cue type",
            ),
            (
                file(&[pcob(1, &[pcp2(1, 1, 5, 0, "", None)])]),
                "unexpected ANLZ cue entry tag",
            ),
            (b"PMA".to_vec(), "truncated ANLZ field"),
            (b"XMAI\0\0\0\x0c\0\0\0\x0c".to_vec(), "not an ANLZ file"),
        ];
        for (bytes, reason) in cases {
            assert_eq!(parse_anlz(&bytes), Err(AnlzError(reason)));
        }
        // A beat-grid count that disagrees with the section length.
        let mut bad = file(&[pqtz(&[(1, 1, 1)])]);
        bad[28 + 23] = 2;
        assert_eq!(
            parse_anlz(&bad),
            Err(AnlzError("ANLZ beat grid length mismatch"))
        );
        // A cue count larger than the stored entries.
        let mut bad = file(&[pcob(1, &[pcpt(1, 1, 5, 0)])]);
        bad[28 + 19] = 2;
        assert!(parse_anlz(&bad).is_err());
        // Odd-length path text.
        let mut bad = file(&[section(b"PPTH", &3u32.to_be_bytes(), &[0, 0x41, 0])]);
        assert_eq!(
            parse_anlz(&bad),
            Err(AnlzError("odd-length ANLZ UTF-16 string"))
        );
        // File length field disagrees with the actual length.
        bad = full();
        bad.push(0);
        assert_eq!(
            parse_anlz(&bad),
            Err(AnlzError("malformed ANLZ file header"))
        );
    }

    #[test]
    fn pwv3_and_pwv5_decode_present_absent_and_fail_closed() {
        // PWV3 bytes: height 31 / whiteness 7, height 0 / whiteness 0, height 10 / whiteness 2.
        // PWV5 words: r7 g0 b3 h31 (+ unused bits set), r1 g2 b4 h5.
        let colour_a: u16 = (7 << 13) | (3 << 7) | (31 << 2) | 0b11;
        let colour_b: u16 = (1 << 13) | (2 << 10) | (4 << 7) | (5 << 2);
        let pwv5_bytes = [colour_a.to_be_bytes(), colour_b.to_be_bytes()].concat();
        let bytes = file(&[
            ppth("/a"),
            pwv(b"PWV3", 1, &[0xff, 0x00, 0x4a]),
            pwv(b"PWV5", 2, &pwv5_bytes),
        ]);
        let f = parse_anlz(&bytes).unwrap();
        let d = |height, whiteness| AnlzDetailColumn { height, whiteness };
        assert_eq!(f.waveform_detail, Some(vec![d(31, 7), d(0, 0), d(10, 2)]));
        let c = |red, green, blue, height| AnlzColourColumn {
            red,
            green,
            blue,
            height,
        };
        assert_eq!(
            f.colour_waveform_detail,
            Some(vec![c(7, 0, 3, 31), c(1, 2, 4, 5)])
        );
        let absent = parse_anlz(&file(&[ppth("/a")])).unwrap();
        assert_eq!(
            (absent.waveform_detail, absent.colour_waveform_detail),
            (None, None)
        );
        let empty = parse_anlz(&file(&[pwv(b"PWV3", 1, &[]), pwv(b"PWV5", 2, &[])])).unwrap();
        assert_eq!(empty.waveform_detail, Some(vec![]));
        assert_eq!(empty.colour_waveform_detail, Some(vec![]));

        let cases: Vec<(Vec<u8>, &str)> = vec![
            (
                file(&[pwv(b"PWV3", 1, &[1]), pwv(b"PWV3", 1, &[1])]),
                "duplicate ANLZ waveform detail",
            ),
            (
                file(&[pwv(b"PWV5", 2, &[0, 0]), pwv(b"PWV5", 2, &[0, 0])]),
                "duplicate ANLZ colour waveform detail",
            ),
            (
                file(&[pwv(b"PWV3", 2, &[1, 1])]),
                "unknown ANLZ waveform entry size",
            ),
            (
                file(&[pwv(b"PWV5", 1, &[1, 1])]),
                "unknown ANLZ waveform entry size",
            ),
            (
                file(&[section(b"PWV3", &[0, 0, 0, 1], &[1])]),
                "malformed ANLZ waveform section",
            ),
        ];
        for (bytes, reason) in cases {
            assert_eq!(parse_anlz(&bytes), Err(AnlzError(reason)));
        }
        // Declared entry count disagrees with the stored body (one entry short).
        let mut short = file(&[pwv(b"PWV5", 2, &pwv5_bytes)]);
        short[28 + 19] = 3;
        assert_eq!(
            parse_anlz(&short),
            Err(AnlzError("ANLZ waveform length mismatch"))
        );
        // Every truncation of a waveform-bearing file is refused.
        for len in 0..bytes.len() {
            assert!(parse_anlz(&bytes[..len]).is_err(), "prefix {len} accepted");
        }
    }

    #[test]
    fn every_truncation_and_byte_corruption_is_refused_without_panicking() {
        let bytes = full();
        for len in 0..bytes.len() {
            assert!(parse_anlz(&bytes[..len]).is_err(), "prefix {len} accepted");
        }
        for at in 0..bytes.len() {
            for value in [0x00, 0xff, 0x7f] {
                let mut corrupt = bytes.clone();
                corrupt[at] = value;
                let _ = parse_anlz(&corrupt);
            }
        }
    }
}
