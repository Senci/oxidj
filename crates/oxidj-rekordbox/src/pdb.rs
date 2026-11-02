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
    /// XML attribute names whose source value could not be interpreted.
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
