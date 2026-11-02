//! Read-only DeviceSQL (export.pdb) preview.
use super::*;

/// Vendor track identity; deliberately not convertible to OxiDJ's `TrackId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PdbTrackId(pub u32);
/// Vendor playlist identity, scoped to one DeviceSQL source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdbNodeId(pub u32);
/// One DeviceSQL track row. Optional fields are `None` when the source holds no value
/// (id 0, numeric 0 or an empty string). A value the preview cannot interpret — a
/// lookup id with no (or more than one) matching row, an unrecognised date or an
/// out-of-range rating — is also `None` and its XML attribute name is listed in
/// `unresolved`; nothing is ever invented.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdbTrackPreview {
    pub source_id: PdbTrackId,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub tempo_hundredths_bpm: u32,
    pub duration_seconds: u16,
    pub file_path: String,
    /// Genre-table name of `genre_id`.
    pub genre: Option<String>,
    /// Label-table name of `label_id`.
    pub label: Option<String>,
    /// Key-table name of `key_id` (XML `Tonality`).
    pub tonality: Option<String>,
    /// Track string slot `comment`.
    pub comments: Option<String>,
    /// Track string slot `date_added`, only when it is exactly `YYYY-MM-DD`.
    pub date_added: Option<String>,
    pub year: Option<u16>,
    /// Kilobits per second.
    pub bit_rate: Option<u32>,
    /// Hertz.
    pub sample_rate: Option<u32>,
    /// File size in bytes.
    pub file_size: Option<u32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u16>,
    /// Star rating 1..=5 (0 = unrated is `None`).
    pub rating_stars: Option<u8>,
    /// Track string slot `analyze_path` (device-absolute ANLZ `.DAT` path), raw.
    pub analysis_path: String,
    /// Beat grid from the track's ANLZ analysis (empty = none); see [`attach_analysis`].
    pub beat_grid: Vec<AnlzBeat>,
    /// Cues and loops from the track's ANLZ analysis (empty = none).
    pub cues: Vec<AnlzCue>,
    /// XML attribute names (or `TEMPO` / `POSITION_MARK`) whose source value could
    /// not be interpreted.
    pub unresolved: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdbPlaylistPreview {
    pub source_id: PdbNodeId,
    pub parent_id: PdbNodeId,
    pub name: String,
    pub hierarchy: Vec<String>,
    pub ordered_track_ids: Vec<PdbTrackId>,
    /// Raw sibling sort key from the source playlist tree.
    pub sort_order: u32,
}
/// A playlist-tree folder node (parent `PdbNodeId(0)` is the implicit root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdbFolderPreview {
    pub source_id: PdbNodeId,
    pub parent_id: PdbNodeId,
    pub name: String,
    pub sort_order: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdbPreview {
    pub tracks: Vec<PdbTrackPreview>,
    pub playlists: Vec<PdbPlaylistPreview>,
    pub folders: Vec<PdbFolderPreview>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdbPreviewError(pub String);
impl std::fmt::Display for PdbPreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for PdbPreviewError {}

pub fn parse_pdb_file(path: impl AsRef<Path>) -> Result<PdbPreview, PdbPreviewError> {
    let bytes = fs::read(path).map_err(|error| PdbPreviewError(error.to_string()))?;
    parse_pdb(&bytes)
}

/// Read-only boundary: invoke only the pinned dependency's `from_bytes` API.
pub fn parse_pdb(bytes: &[u8]) -> Result<PdbPreview, PdbPreviewError> {
    if bytes.len() < 0x2c || !bytes.len().is_multiple_of(4096) {
        return Err(PdbPreviewError(
            "truncated or unaligned DeviceSQL source".into(),
        ));
    }
    let page_size = u32::from_le_bytes(bytes[4..8].try_into().expect("fixed header field"));
    let table_count = u32::from_le_bytes(bytes[8..12].try_into().expect("fixed header field"));
    let page_count = bytes.len() / 4096;
    if page_size != 4096 || table_count > 20 || 0x1c + table_count as usize * 16 > bytes.len() {
        return Err(PdbPreviewError("unsupported DeviceSQL header".into()));
    }
    for index in 0..table_count as usize {
        let offset = 0x1c + index * 16;
        let first = u32::from_le_bytes(
            bytes[offset + 8..offset + 12]
                .try_into()
                .expect("table field"),
        ) as usize;
        let last = u32::from_le_bytes(
            bytes[offset + 12..offset + 16]
                .try_into()
                .expect("table field"),
        ) as usize;
        if first >= page_count || last >= page_count {
            return Err(PdbPreviewError(
                "DeviceSQL table points outside source".into(),
            ));
        }
        let mut current = first;
        let mut visited = std::collections::HashSet::new();
        loop {
            if current >= page_count || !visited.insert(current) {
                return Err(PdbPreviewError("invalid DeviceSQL table page chain".into()));
            }
            if current == last {
                break;
            }
            let next_offset = current * 4096 + 12;
            current = u32::from_le_bytes(
                bytes[next_offset..next_offset + 4]
                    .try_into()
                    .expect("page field"),
            ) as usize;
        }
    }
    let parsed = std::panic::catch_unwind(|| rekordbox_pdb::Database::from_bytes(bytes.to_vec()))
        .map_err(|_| PdbPreviewError("malformed or unsupported DeviceSQL input".into()))?
        .map_err(|error| PdbPreviewError(error.to_string()))?;
    let artists = Lookup::new(parsed.artists.iter().map(|r| (r.id, r.name.as_str())));
    let albums = Lookup::new(parsed.albums.iter().map(|r| (r.id, r.name.as_str())));
    let genres = Lookup::new(parsed.genres.iter().map(|r| (r.id, r.name.as_str())));
    let labels = Lookup::new(parsed.labels.iter().map(|r| (r.id, r.name.as_str())));
    let keys = Lookup::new(parsed.keys.iter().map(|r| (r.id, r.name.as_str())));
    let tracks = parsed
        .tracks
        .iter()
        .map(|row| {
            let mut unresolved = Vec::new();
            let artist = artists.resolve(row.artist_id, "Artist", &mut unresolved);
            let album = albums.resolve(row.album_id, "Album", &mut unresolved);
            let genre = genres.resolve(row.genre_id, "Genre", &mut unresolved);
            let label = labels.resolve(row.label_id, "Label", &mut unresolved);
            let tonality = keys.resolve(row.key_id, "Tonality", &mut unresolved);
            let date_added = match row.date_added() {
                "" => None,
                value if is_iso_date(value) => Some(value.to_owned()),
                _ => {
                    unresolved.push("DateAdded");
                    None
                }
            };
            let rating_stars = match row.rating {
                0 => None,
                stars @ 1..=5 => Some(stars),
                _ => {
                    unresolved.push("Rating");
                    None
                }
            };
            PdbTrackPreview {
                source_id: PdbTrackId(row.id),
                title: row.title().to_owned(),
                artist,
                album,
                tempo_hundredths_bpm: row.tempo,
                duration_seconds: row.duration,
                file_path: row.file_path().to_owned(),
                genre,
                label,
                tonality,
                comments: non_empty(row.comment()),
                date_added,
                year: non_zero(row.year),
                bit_rate: non_zero(row.bitrate),
                sample_rate: non_zero(row.sample_rate),
                file_size: non_zero(row.file_size),
                track_number: non_zero(row.track_number),
                disc_number: non_zero(row.disc_number),
                rating_stars,
                analysis_path: row.analyze_path().to_owned(),
                beat_grid: Vec::new(),
                cues: Vec::new(),
                unresolved,
            }
        })
        .collect();
    let nodes: std::collections::HashMap<_, _> = parsed
        .playlist_tree
        .iter()
        .map(|node| (node.id, node))
        .collect();
    let mut playlists = Vec::new();
    for node in parsed.playlist_tree.iter().filter(|node| !node.is_folder) {
        let mut hierarchy = vec![node.name.clone()];
        let mut parent_id = node.parent_id;
        let mut visited = std::collections::HashSet::new();
        while parent_id != 0 {
            if !visited.insert(parent_id) {
                return Err(PdbPreviewError("cyclic playlist hierarchy".into()));
            }
            let parent = nodes
                .get(&parent_id)
                .ok_or_else(|| PdbPreviewError("playlist references missing parent".into()))?;
            if !parent.is_folder {
                return Err(PdbPreviewError("playlist parent is not a folder".into()));
            }
            hierarchy.push(parent.name.clone());
            parent_id = parent.parent_id;
        }
        hierarchy.reverse();
        let mut entries: Vec<_> = parsed
            .playlist_entries
            .iter()
            .filter(|entry| entry.playlist_id == node.id)
            .collect();
        entries.sort_by_key(|entry| entry.entry_index);
        playlists.push(PdbPlaylistPreview {
            source_id: PdbNodeId(node.id),
            parent_id: PdbNodeId(node.parent_id),
            name: node.name.clone(),
            hierarchy,
            ordered_track_ids: entries
                .into_iter()
                .map(|entry| PdbTrackId(entry.track_id))
                .collect(),
            sort_order: node.sort_order,
        });
    }
    let folders = parsed
        .playlist_tree
        .iter()
        .filter(|node| node.is_folder)
        .map(|node| PdbFolderPreview {
            source_id: PdbNodeId(node.id),
            parent_id: PdbNodeId(node.parent_id),
            name: node.name.clone(),
            sort_order: node.sort_order,
        })
        .collect();
    Ok(PdbPreview {
        tracks,
        playlists,
        folders,
    })
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// Outcome of reading one device-relative analysis file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisRead {
    Found(Vec<u8>),
    NotFound,
    Failed,
}

/// Device-relative path components of an `analyze_path`, or `None` when it is not a
/// plain `/PIONEER/...` path (no empty, `.`, `..`, backslash or NUL components).
fn device_relative(path: &str) -> Option<Vec<&str>> {
    let rest = path.strip_prefix('/')?;
    let parts: Vec<&str> = rest.split('/').collect();
    let safe = parts.first() == Some(&"PIONEER")
        && parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && *p != "." && *p != ".." && !p.contains(['\\', '\0']));
    safe.then_some(parts)
}

/// Attach ANLZ beat grids and cues read from the device whose root directory holds
/// `PIONEER/` (see [`attach_analysis_with`] for the rules). `None` = the root is
/// unknown, so every track that references analysis is unresolved.
pub fn attach_analysis(preview: &mut PdbPreview, device_root: Option<&Path>) {
    attach_analysis_with(preview, |parts| {
        let Some(root) = device_root else {
            return AnalysisRead::Failed;
        };
        let path = parts.iter().fold(root.to_path_buf(), |p, c| p.join(c));
        match path.symlink_metadata() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => AnalysisRead::NotFound,
            Ok(m) if m.is_file() && m.len() <= ANLZ_MAX_BYTES as u64 => fs::read(&path)
                .map(AnalysisRead::Found)
                .unwrap_or(AnalysisRead::Failed),
            _ => AnalysisRead::Failed,
        }
    });
}

/// Rules (nothing is invented; an unresolved value is listed in `unresolved` as
/// `TEMPO` and/or `POSITION_MARK` and left empty):
/// - empty `analyze_path` → no grid, no cues (absent);
/// - the `.DAT` must exist, decode, and its `PPTH` must equal the track `file_path`,
///   else both are unresolved;
/// - grid: absent or empty `PQTZ` → absent; beats outside 1..=4, a zero tempo or
///   non-increasing times → unresolved;
/// - cues: from the sibling `.EXT` `PCO2` lists when present (every `.DAT` `PCOB`
///   cue must also appear there, and the `.EXT` `PPTH` must match), else from the
///   `.DAT` `PCOB` lists; an unreadable/undecodable `.EXT` → unresolved.
pub fn attach_analysis_with(
    preview: &mut PdbPreview,
    mut read: impl FnMut(&[&str]) -> AnalysisRead,
) {
    for track in &mut preview.tracks {
        if track.analysis_path.is_empty() {
            continue;
        }
        let Some(dat_parts) = device_relative(&track.analysis_path)
            .filter(|parts| parts.last().is_some_and(|name| name.ends_with(".DAT")))
        else {
            track.unresolved.extend(["TEMPO", "POSITION_MARK"]);
            continue;
        };
        let file_path = track.file_path.clone();
        let mut owned = |parts: &[&str]| -> Result<Option<AnlzFile>, ()> {
            match read(parts) {
                AnalysisRead::Found(bytes) => match parse_anlz(&bytes) {
                    Ok(file) if file.path.as_deref() == Some(file_path.as_str()) => Ok(Some(file)),
                    _ => Err(()),
                },
                AnalysisRead::NotFound => Ok(None),
                AnalysisRead::Failed => Err(()),
            }
        };
        let dat = match owned(&dat_parts) {
            Ok(Some(file)) => file,
            _ => {
                track.unresolved.extend(["TEMPO", "POSITION_MARK"]);
                continue;
            }
        };
        let grid = dat.beat_grid.unwrap_or_default();
        let valid_grid = grid
            .iter()
            .all(|b| (1..=4).contains(&b.beat_in_bar) && b.tempo_hundredths_bpm > 0)
            && grid.windows(2).all(|w| w[0].time_ms < w[1].time_ms);
        if valid_grid {
            track.beat_grid = grid;
        } else {
            track.unresolved.push("TEMPO");
        }
        let mut ext_parts = dat_parts.clone();
        let ext_name = format!(
            "{}EXT",
            ext_parts.last().expect("non-empty").trim_end_matches("DAT")
        );
        *ext_parts.last_mut().expect("non-empty") = &ext_name;
        let dat_cues = dat.cues.unwrap_or_default();
        let key = |c: &AnlzCue| (c.hot_cue, c.start_ms, c.loop_end_ms);
        match owned(&ext_parts) {
            Ok(Some(AnlzFile {
                extended_cues: Some(extended),
                ..
            })) => {
                let known: HashSet<_> = extended.iter().map(key).collect();
                if dat_cues.iter().all(|c| known.contains(&key(c))) {
                    track.cues = extended;
                } else {
                    track.unresolved.push("POSITION_MARK");
                }
            }
            Ok(_) => track.cues = dat_cues,
            Err(()) => track.unresolved.push("POSITION_MARK"),
        }
    }
}

fn non_zero<T: PartialEq + Default>(value: T) -> Option<T> {
    (value != T::default()).then_some(value)
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        && matches!(value[5..7].parse::<u8>(), Ok(1..=12))
        && matches!(value[8..10].parse::<u8>(), Ok(1..=31))
}

/// An id → name lookup table. An id present on more than one row is ambiguous and
/// never resolved.
struct Lookup<'a>(std::collections::HashMap<u32, Option<&'a str>>);

impl<'a> Lookup<'a> {
    fn new(rows: impl Iterator<Item = (u32, &'a str)>) -> Self {
        let mut map = std::collections::HashMap::new();
        for (id, name) in rows {
            map.entry(id)
                .and_modify(|slot| *slot = None)
                .or_insert(Some(name));
        }
        Self(map)
    }

    /// `0` means "no value". An empty name is treated as no value.
    fn resolve(
        &self,
        id: u32,
        field: &'static str,
        unresolved: &mut Vec<&'static str>,
    ) -> Option<String> {
        if id == 0 {
            return None;
        }
        match self.0.get(&id) {
            Some(Some(name)) => non_empty(name),
            _ => {
                unresolved.push(field);
                None
            }
        }
    }
}

#[cfg(test)]
mod analysis_tests {
    use super::*;
    use crate::anlz::fixture::*;
    use std::collections::HashMap;

    fn track(id: u32, analysis_path: &str) -> PdbTrackPreview {
        PdbTrackPreview {
            source_id: PdbTrackId(id),
            file_path: format!("/Contents/{id}.mp3"),
            analysis_path: analysis_path.into(),
            ..PdbTrackPreview::default()
        }
    }

    fn attach(tracks: Vec<PdbTrackPreview>, files: &[(&str, Vec<u8>)]) -> Vec<PdbTrackPreview> {
        let files: HashMap<String, Vec<u8>> = files
            .iter()
            .map(|(p, b)| (p.to_string(), b.clone()))
            .collect();
        let mut preview = PdbPreview {
            tracks,
            playlists: vec![],
            folders: vec![],
        };
        attach_analysis_with(&mut preview, |parts| {
            match files.get(&format!("/{}", parts.join("/"))) {
                Some(b) if b == b"io-error" => AnalysisRead::Failed,
                Some(b) => AnalysisRead::Found(b.clone()),
                None => AnalysisRead::NotFound,
            }
        });
        preview.tracks
    }

    const DAT: &str = "/PIONEER/USBANLZ/P001/0001/ANLZ0000.DAT";
    const EXT: &str = "/PIONEER/USBANLZ/P001/0001/ANLZ0000.EXT";

    fn grid() -> Vec<u8> {
        pqtz(&[(4, 12_000, 0), (1, 12_000, 500)])
    }

    #[test]
    fn analysis_is_attached_absent_or_unresolved_and_never_invented() {
        let owner = ppth("/Contents/1.mp3");
        let dat_cues = pcob(1, &[pcpt(1, 1, 1_000, u32::MAX)]);
        let dat = file(&[owner.clone(), grid(), dat_cues.clone()]);
        let ext = file(&[
            owner.clone(),
            pco2(
                1,
                &[
                    pcp2(1, 1, 1_000, u32::MAX, "A", Some([1, 2, 3])),
                    pcp2(4, 2, 2_000, 3_000, "", Some([4, 5, 6])),
                ],
            ),
        ]);
        // 1: DAT + EXT superset → grid from DAT, cues from EXT.
        let t = &attach(
            vec![track(1, DAT)],
            &[(DAT, dat.clone()), (EXT, ext.clone())],
        )[0];
        assert!(t.unresolved.is_empty());
        assert_eq!(t.beat_grid.len(), 2);
        assert_eq!(t.cues.len(), 2);
        assert_eq!(t.cues[0].comment.as_deref(), Some("A"));
        // 2: no EXT → DAT cues (no name, no colour).
        let t = &attach(vec![track(1, DAT)], &[(DAT, dat.clone())])[0];
        assert!(t.unresolved.is_empty());
        assert_eq!(
            t.cues,
            vec![AnlzCue {
                hot_cue: 1,
                start_ms: 1_000,
                loop_end_ms: None,
                comment: None,
                rgb: None
            }]
        );
        // 3: empty analyze_path → absent, nothing read.
        let t = &attach(vec![track(1, "")], &[])[0];
        assert!(t.unresolved.is_empty() && t.beat_grid.is_empty() && t.cues.is_empty());
        // 4: empty grid and no cue sections → absent.
        let t = &attach(
            vec![track(1, DAT)],
            &[(DAT, file(&[owner.clone(), pqtz(&[])]))],
        )[0];
        assert!(t.unresolved.is_empty() && t.beat_grid.is_empty() && t.cues.is_empty());
        let both = vec!["TEMPO", "POSITION_MARK"];
        for (path, files, expected) in [
            // unsafe or non-DAT analyze paths
            ("/PIONEER/../x.DAT", vec![], both.clone()),
            ("PIONEER/USBANLZ/a.DAT", vec![], both.clone()),
            ("/OTHER/a.DAT", vec![], both.clone()),
            ("/PIONEER/USBANLZ/a.EXT", vec![], both.clone()),
            // missing, unreadable, foreign-owned, unknown-section DAT
            (DAT, vec![], both.clone()),
            (DAT, vec![(DAT, b"io-error".to_vec())], both.clone()),
            (
                DAT,
                vec![(DAT, file(&[ppth("/Contents/other.mp3"), grid()]))],
                both.clone(),
            ),
            (DAT, vec![(DAT, file(&[grid()]))], both.clone()),
            (
                DAT,
                vec![(DAT, file(&[owner.clone(), section(b"PZZZ", &[], &[])]))],
                both.clone(),
            ),
            // bad grids leave cues intact
            (
                DAT,
                vec![(DAT, file(&[owner.clone(), pqtz(&[(5, 12_000, 0)])]))],
                vec!["TEMPO"],
            ),
            (
                DAT,
                vec![(DAT, file(&[owner.clone(), pqtz(&[(1, 0, 0)])]))],
                vec!["TEMPO"],
            ),
            (
                DAT,
                vec![(
                    DAT,
                    file(&[owner.clone(), pqtz(&[(1, 100, 9), (2, 100, 9)])]),
                )],
                vec!["TEMPO"],
            ),
            // EXT unreadable, malformed, foreign-owned, or missing a DAT cue
            (
                DAT,
                vec![(DAT, dat.clone()), (EXT, b"io-error".to_vec())],
                vec!["POSITION_MARK"],
            ),
            (
                DAT,
                vec![(DAT, dat.clone()), (EXT, b"PMAI".to_vec())],
                vec!["POSITION_MARK"],
            ),
            (
                DAT,
                vec![(DAT, dat.clone()), (EXT, file(&[ppth("/x"), pco2(1, &[])]))],
                vec!["POSITION_MARK"],
            ),
            (
                DAT,
                vec![
                    (DAT, dat.clone()),
                    (EXT, file(&[owner.clone(), pco2(1, &[])])),
                ],
                vec!["POSITION_MARK"],
            ),
        ] {
            let t = &attach(vec![track(1, path)], &files)[0];
            assert_eq!(t.unresolved, expected, "{path}");
            if expected.contains(&"TEMPO") {
                assert!(t.beat_grid.is_empty());
            }
            if expected.contains(&"POSITION_MARK") {
                assert!(t.cues.is_empty());
            }
        }
        // EXT without PCO2 falls back to DAT cues.
        let t = &attach(
            vec![track(1, DAT)],
            &[(DAT, dat), (EXT, file(&[owner, dat_cues]))],
        )[0];
        assert!(t.unresolved.is_empty());
        assert_eq!(t.cues.len(), 1);
    }

    #[test]
    fn attach_analysis_reads_the_device_layout_and_refuses_symlinks() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("oxidj-anlz-{}-{nonce}", std::process::id()));
        let dir = root.join("PIONEER/USBANLZ/P001/0001");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("ANLZ0000.DAT"),
            file(&[ppth("/Contents/1.mp3"), grid()]),
        )
        .unwrap();
        let mut preview = PdbPreview {
            tracks: vec![track(1, DAT)],
            playlists: vec![],
            folders: vec![],
        };
        attach_analysis(&mut preview, Some(&root));
        assert!(preview.tracks[0].unresolved.is_empty());
        assert_eq!(preview.tracks[0].beat_grid.len(), 2);
        // Unknown device root → unresolved, nothing guessed.
        let mut preview = PdbPreview {
            tracks: vec![track(1, DAT)],
            playlists: vec![],
            folders: vec![],
        };
        attach_analysis(&mut preview, None);
        assert_eq!(preview.tracks[0].unresolved, vec!["TEMPO", "POSITION_MARK"]);
        #[cfg(unix)]
        {
            fs::rename(dir.join("ANLZ0000.DAT"), dir.join("real.DAT")).unwrap();
            std::os::unix::fs::symlink(dir.join("real.DAT"), dir.join("ANLZ0000.DAT")).unwrap();
            let mut preview = PdbPreview {
                tracks: vec![track(1, DAT)],
                playlists: vec![],
                folders: vec![],
            };
            attach_analysis(&mut preview, Some(&root));
            assert_eq!(preview.tracks[0].unresolved, vec!["TEMPO", "POSITION_MARK"]);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
