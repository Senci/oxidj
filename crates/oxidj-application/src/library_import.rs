//! M7 slice: import a DeviceSQL PDB preview into a new local catalog, and export a
//! catalog's imported library as a new Rekordbox XML file. This layer composes the
//! adapter previews with library handlers; the library never sees adapter types.
//! Reports carry aggregate counts and booleans only — never names, paths or IDs.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use oxidj_library::{Catalog, LibraryCounts, LibraryNode, LibrarySnapshot, LibraryTrack};
use oxidj_rekordbox::{
    LIBRARY_XML_NOT_WRITTEN_TRACK_ATTRIBUTES, LIBRARY_XML_NOT_WRITTEN_TRACK_ELEMENTS,
    LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES, LibraryXmlNode, LibraryXmlTrack, PdbNodeId, PdbPreview,
};

/// A fail-closed import/export refusal. Messages never contain source values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryWorkflowError(pub String);
impl std::fmt::Display for LibraryWorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LibraryWorkflowError {}

fn fail(message: &str) -> LibraryWorkflowError {
    LibraryWorkflowError(message.into())
}

/// Fields a written library cannot carry, as explicit per-field track counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotCarried {
    /// (XML TRACK attribute, number of tracks written without it).
    pub track_attributes: Vec<(&'static str, usize)>,
    /// (XML TRACK child element, number of tracks written without it).
    pub track_elements: Vec<(&'static str, usize)>,
    /// (optional XML TRACK attribute the library carries, number of tracks whose
    /// value was absent — so the attribute was not written). Absent, not dropped.
    pub absent: Vec<(&'static str, usize)>,
}

impl NotCarried {
    /// Number of tracks that carry the named optional attribute.
    pub fn carried(&self, attribute: &str, tracks: usize) -> usize {
        tracks
            - self
                .absent
                .iter()
                .find(|(name, _)| *name == attribute)
                .map_or(0, |(_, count)| *count)
    }

    fn of(snapshot: &LibrarySnapshot) -> Self {
        let tracks = snapshot.tracks.len();
        Self {
            track_attributes: LIBRARY_XML_NOT_WRITTEN_TRACK_ATTRIBUTES
                .iter()
                .map(|name| (*name, tracks))
                .collect(),
            track_elements: LIBRARY_XML_NOT_WRITTEN_TRACK_ELEMENTS
                .iter()
                .map(|name| (*name, tracks))
                .collect(),
            absent: LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES
                .iter()
                .map(|name| {
                    let present = |t: &LibraryTrack| match *name {
                        "Artist" => t.artist.is_some(),
                        "Album" => t.album.is_some(),
                        "Genre" => t.genre.is_some(),
                        "Size" => t.file_size.is_some(),
                        "DiscNumber" => t.disc_number.is_some(),
                        "TrackNumber" => t.track_number.is_some(),
                        "Year" => t.year.is_some(),
                        "DateAdded" => t.date_added.is_some(),
                        "BitRate" => t.bit_rate.is_some(),
                        "SampleRate" => t.sample_rate.is_some(),
                        "Comments" => t.comments.is_some(),
                        "Rating" => t.rating_stars.is_some(),
                        "Tonality" => t.key.is_some(),
                        "Label" => t.label.is_some(),
                        _ => unreachable!("every optional attribute is mapped"),
                    };
                    (
                        *name,
                        snapshot.tracks.iter().filter(|t| !present(t)).count(),
                    )
                })
                .collect(),
        }
    }
}

/// Read-only OneLibrary aggregate cross-check (counts and booleans only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneLibraryCrossCheck {
    pub tracks: usize,
    pub playlists: usize,
    pub memberships: usize,
    pub tracks_equal: bool,
    pub playlists_equal: bool,
    pub memberships_equal: bool,
    pub source_unchanged: bool,
}

/// Compare OneLibrary aggregates with the imported PDB counts.
pub fn onelibrary_cross_check(
    counts: &LibraryCounts,
    preview: &crate::OneLibraryPreview,
    source_unchanged: bool,
) -> OneLibraryCrossCheck {
    let memberships = preview.playlists.iter().map(|p| p.track_ids.len()).sum();
    OneLibraryCrossCheck {
        tracks: preview.tracks.len(),
        playlists: preview.playlists.len(),
        memberships,
        tracks_equal: preview.tracks.len() == counts.tracks,
        playlists_equal: preview.playlists.len() == counts.playlists,
        memberships_equal: memberships == counts.memberships,
        source_unchanged,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryImportReport {
    pub counts: LibraryCounts,
    pub not_carried: NotCarried,
    /// (XML TRACK attribute, number of tracks whose source value could not be
    /// interpreted — unknown/ambiguous lookup id or unrecognised encoding). Those
    /// values are not carried and never invented.
    pub unresolved: Vec<(&'static str, usize)>,
    /// Source paths that were not URIs and were percent-encoded as `file://localhost` URIs.
    pub locations_uri_encoded: usize,
    /// Source bytes identical before and after the import.
    pub source_unchanged: bool,
    /// The reopened catalog equals the mapped source library exactly.
    pub catalog_matches_source: bool,
    pub onelibrary: Option<OneLibraryCrossCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryExportReport {
    pub counts: LibraryCounts,
    pub not_carried: NotCarried,
    /// The written bytes re-parsed with the XML preview and matched the catalog.
    pub preview_verified: bool,
}

fn location_uri(path: &str) -> (String, bool) {
    if path.starts_with("file://") {
        return (path.to_owned(), false);
    }
    let mut out = String::from("file://localhost");
    if !path.starts_with('/') {
        out.push('/');
    }
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    (out, true)
}

/// Map a PDB preview to a vendor-neutral library snapshot. Fails closed on duplicate
/// identities, missing member tracks, and nodes unreachable from the root.
pub fn library_from_pdb_preview(
    preview: &PdbPreview,
) -> Result<(LibrarySnapshot, usize), LibraryWorkflowError> {
    let mut index = HashMap::new();
    let mut encoded = 0;
    let mut tracks = Vec::with_capacity(preview.tracks.len());
    for (position, track) in preview.tracks.iter().enumerate() {
        if index.insert(track.source_id, position).is_some() {
            return Err(fail("duplicate source track identity"));
        }
        let (location, was_encoded) = location_uri(&track.file_path);
        encoded += usize::from(was_encoded);
        tracks.push(LibraryTrack {
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            tempo_hundredths_bpm: track.tempo_hundredths_bpm,
            duration_seconds: u32::from(track.duration_seconds),
            location,
            genre: track.genre.clone(),
            label: track.label.clone(),
            key: track.tonality.clone(),
            comments: track.comments.clone(),
            date_added: track.date_added.clone(),
            year: track.year.map(u32::from),
            bit_rate: track.bit_rate,
            sample_rate: track.sample_rate,
            file_size: track.file_size.map(u64::from),
            track_number: track.track_number,
            disc_number: track.disc_number.map(u32::from),
            rating_stars: track.rating_stars,
        });
    }
    // (sort_order, id, node) per parent.
    enum Source<'a> {
        Folder(&'a str),
        Playlist(&'a oxidj_rekordbox::PdbPlaylistPreview),
    }
    let mut ids = HashSet::new();
    let mut children: HashMap<PdbNodeId, Vec<(u32, u32, Source<'_>)>> = HashMap::new();
    for folder in &preview.folders {
        if !ids.insert(folder.source_id) || folder.source_id.0 == 0 {
            return Err(fail("duplicate or reserved playlist-tree identity"));
        }
        children.entry(folder.parent_id).or_default().push((
            folder.sort_order,
            folder.source_id.0,
            Source::Folder(&folder.name),
        ));
    }
    for playlist in &preview.playlists {
        if !ids.insert(playlist.source_id) || playlist.source_id.0 == 0 {
            return Err(fail("duplicate or reserved playlist-tree identity"));
        }
        children.entry(playlist.parent_id).or_default().push((
            playlist.sort_order,
            playlist.source_id.0,
            Source::Playlist(playlist),
        ));
    }
    for siblings in children.values_mut() {
        siblings.sort_by_key(|(order, id, _)| (*order, *id));
    }
    fn build(
        parent: PdbNodeId,
        children: &HashMap<PdbNodeId, Vec<(u32, u32, Source<'_>)>>,
        index: &HashMap<oxidj_rekordbox::PdbTrackId, usize>,
        visited: &mut usize,
        depth: usize,
    ) -> Result<Vec<LibraryNode>, LibraryWorkflowError> {
        if depth > 256 {
            return Err(fail("playlist tree is too deep"));
        }
        let mut out = Vec::new();
        for (_, id, source) in children.get(&parent).map(Vec::as_slice).unwrap_or(&[]) {
            *visited += 1;
            out.push(match source {
                Source::Folder(name) => LibraryNode::Folder {
                    name: (*name).to_owned(),
                    children: build(PdbNodeId(*id), children, index, visited, depth + 1)?,
                },
                Source::Playlist(playlist) => LibraryNode::Playlist {
                    name: playlist.name.clone(),
                    members: playlist
                        .ordered_track_ids
                        .iter()
                        .map(|track| {
                            index
                                .get(track)
                                .copied()
                                .ok_or_else(|| fail("playlist member references a missing track"))
                        })
                        .collect::<Result<_, _>>()?,
                },
            });
        }
        Ok(out)
    }
    let mut visited = 0;
    let roots = build(PdbNodeId(0), &children, &index, &mut visited, 0)?;
    if visited != ids.len() {
        return Err(fail("playlist-tree node is unreachable from the root"));
    }
    Ok((LibrarySnapshot { tracks, roots }, encoded))
}

/// Per-attribute counts of source values the PDB preview could not interpret, in
/// the order of [`LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES`] (zero counts included).
pub fn unresolved_counts(preview: &PdbPreview) -> Vec<(&'static str, usize)> {
    LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES
        .iter()
        .map(|name| {
            (
                *name,
                preview
                    .tracks
                    .iter()
                    .filter(|t| t.unresolved.contains(name))
                    .count(),
            )
        })
        .collect()
}

fn catalog_files(catalog: &Path) -> [std::path::PathBuf; 3] {
    let mut wal = catalog.as_os_str().to_owned();
    wal.push("-wal");
    let mut shm = catalog.as_os_str().to_owned();
    shm.push("-shm");
    [catalog.to_path_buf(), wal.into(), shm.into()]
}

fn read_source(path: &Path) -> Result<Vec<u8>, LibraryWorkflowError> {
    std::fs::read(path).map_err(|_| fail("source could not be read"))
}

/// Import a PDB source into a NEW catalog (the path must not exist). On any failure
/// no catalog is left behind. `onelibrary` optionally supplies a read-only cross-check
/// source whose key is read from non-terminal stdin.
pub fn import_library_from_pdb(
    pdb: impl AsRef<Path>,
    catalog: impl AsRef<Path>,
    onelibrary: Option<&Path>,
) -> Result<LibraryImportReport, LibraryWorkflowError> {
    import_library_with(pdb.as_ref(), catalog.as_ref(), |counts| {
        onelibrary
            .map(|path| {
                let before = read_source(path)?;
                let preview = crate::preview_onelibrary_stdin(path)
                    .map_err(|_| fail("OneLibrary cross-check source was rejected"))?;
                let unchanged = read_source(path)? == before;
                Ok(onelibrary_cross_check(counts, &preview, unchanged))
            })
            .transpose()
    })
}

fn import_library_with(
    pdb: &Path,
    catalog: &Path,
    cross_check: impl FnOnce(
        &LibraryCounts,
    ) -> Result<Option<OneLibraryCrossCheck>, LibraryWorkflowError>,
) -> Result<LibraryImportReport, LibraryWorkflowError> {
    let files = catalog_files(catalog);
    if files.iter().any(|f| f.symlink_metadata().is_ok()) {
        return Err(fail("refusing to reuse an existing catalog path"));
    }
    let before = read_source(pdb)?;
    let preview = oxidj_rekordbox::parse_pdb(&before)
        .map_err(|_| fail("PDB source was rejected by the read-only preview"))?;
    let (snapshot, locations_uri_encoded) = library_from_pdb_preview(&preview)?;
    let unresolved = unresolved_counts(&preview);
    let counts = snapshot.counts();
    let onelibrary = cross_check(&counts)?;
    let result = (|| {
        let mut created =
            Catalog::open(catalog).map_err(|e| LibraryWorkflowError(e.to_string()))?;
        created
            .import_library(&snapshot)
            .map_err(|e| LibraryWorkflowError(e.to_string()))?;
        drop(created);
        let reopened = Catalog::open(catalog).map_err(|e| LibraryWorkflowError(e.to_string()))?;
        let catalog_matches_source = reopened
            .library_snapshot()
            .map_err(|e| LibraryWorkflowError(e.to_string()))?
            == snapshot;
        let source_unchanged = read_source(pdb)? == before;
        if !catalog_matches_source || !source_unchanged {
            return Err(fail("import verification failed"));
        }
        Ok(LibraryImportReport {
            counts,
            not_carried: NotCarried::of(&snapshot),
            unresolved,
            locations_uri_encoded,
            source_unchanged,
            catalog_matches_source,
            onelibrary,
        })
    })();
    if result.is_err() {
        for file in &files {
            let _ = std::fs::remove_file(file);
        }
    }
    result
}

fn xml_library(
    snapshot: &LibrarySnapshot,
) -> Result<(Vec<LibraryXmlTrack>, Vec<LibraryXmlNode>), LibraryWorkflowError> {
    let key = |index: usize| {
        u32::try_from(index + 1).map_err(|_| fail("library track index exceeds XML range"))
    };
    let tracks = snapshot
        .tracks
        .iter()
        .enumerate()
        .map(|(index, t)| {
            Ok(LibraryXmlTrack {
                track_id: key(index)?,
                name: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                average_bpm_hundredths: t.tempo_hundredths_bpm,
                total_time_seconds: t.duration_seconds,
                location: t.location.clone(),
                genre: t.genre.clone(),
                label: t.label.clone(),
                tonality: t.key.clone(),
                comments: t.comments.clone(),
                date_added: t.date_added.clone(),
                year: t.year,
                size: t.file_size,
                disc_number: t.disc_number,
                track_number: t.track_number,
                bit_rate: t.bit_rate,
                sample_rate: t.sample_rate,
                rating_stars: t.rating_stars,
            })
        })
        .collect::<Result<_, LibraryWorkflowError>>()?;
    fn nodes(
        input: &[LibraryNode],
        key: &impl Fn(usize) -> Result<u32, LibraryWorkflowError>,
    ) -> Result<Vec<LibraryXmlNode>, LibraryWorkflowError> {
        input
            .iter()
            .map(|node| match node {
                LibraryNode::Folder { name, children } => Ok(LibraryXmlNode::Folder {
                    name: name.clone(),
                    children: nodes(children, key)?,
                }),
                LibraryNode::Playlist { name, members } => Ok(LibraryXmlNode::Playlist {
                    name: name.clone(),
                    track_ids: members.iter().map(|&m| key(m)).collect::<Result<_, _>>()?,
                }),
            })
            .collect()
    }
    Ok((tracks, nodes(&snapshot.roots, &key)?))
}

/// Check re-parsed XML against the catalog snapshot: tracks, folder and playlist
/// paths in document order, and ordered member keys.
fn verify_export(snapshot: &LibrarySnapshot, preview: &crate::Preview) -> bool {
    let tracks_match = preview.tracks.len() == snapshot.tracks.len()
        && preview
            .tracks
            .iter()
            .zip(&snapshot.tracks)
            .enumerate()
            .all(|(i, (x, t))| {
                let num = |v: Option<u64>| v.map(|v| v.to_string());
                x.id == (i + 1).to_string()
                    && x.name == t.title
                    && x.artist == t.artist
                    && x.album == t.album
                    && x.location == t.location
                    && x.total_time == Some(t.duration_seconds.to_string())
                    && x.average_bpm
                        == Some(format!(
                            "{}.{:02}",
                            t.tempo_hundredths_bpm / 100,
                            t.tempo_hundredths_bpm % 100
                        ))
                    && x.genre == t.genre
                    && x.label == t.label
                    && x.tonality == t.key
                    && x.comments == t.comments
                    && x.date_added == t.date_added
                    && x.year == num(t.year.map(u64::from))
                    && x.size == num(t.file_size)
                    && x.disc_number == num(t.disc_number.map(u64::from))
                    && x.track_number == num(t.track_number.map(u64::from))
                    && x.bit_rate == num(t.bit_rate.map(u64::from))
                    && x.sample_rate == num(t.sample_rate.map(u64::from))
                    && x.rating == num(t.rating_stars.map(|s| u64::from(s) * 51))
                    && [
                        &x.composer,
                        &x.grouping,
                        &x.remixer,
                        &x.mix,
                        &x.kind,
                        &x.date_modified,
                        &x.last_played,
                        &x.play_count,
                        &x.colour,
                    ]
                    .iter()
                    .all(|v| v.is_none())
                    && x.tempos.is_empty()
                    && x.marks.is_empty()
            });
    let mut folders = vec![vec!["ROOT".to_string()]];
    let mut playlists = Vec::new();
    fn walk(
        nodes: &[LibraryNode],
        path: &mut Vec<String>,
        folders: &mut Vec<Vec<String>>,
        playlists: &mut Vec<(Vec<String>, Vec<String>)>,
    ) {
        for node in nodes {
            match node {
                LibraryNode::Folder { name, children } => {
                    path.push(name.clone());
                    folders.push(
                        std::iter::once("ROOT".to_string())
                            .chain(path.iter().cloned())
                            .collect(),
                    );
                    walk(children, path, folders, playlists);
                    path.pop();
                }
                LibraryNode::Playlist { name, members } => playlists.push((
                    path.iter()
                        .cloned()
                        .chain(std::iter::once(name.clone()))
                        .collect(),
                    members.iter().map(|m| (m + 1).to_string()).collect(),
                )),
            }
        }
    }
    walk(
        &snapshot.roots,
        &mut Vec::new(),
        &mut folders,
        &mut playlists,
    );
    let actual_folders: Vec<_> = preview.folders.iter().map(|f| f.path.clone()).collect();
    let actual_playlists: Vec<_> = preview
        .playlists
        .iter()
        .map(|p| (p.path.clone(), p.tracks.clone()))
        .collect();
    tracks_match
        && preview.unsupported.is_empty()
        && actual_folders == folders
        && actual_playlists == playlists
}

/// Export a catalog's imported library as a NEW Rekordbox XML file, without edits.
/// The bytes are re-parsed and verified before publication; nothing is written on
/// failure and an existing output is never replaced.
pub fn export_library_xml(
    catalog: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<LibraryExportReport, LibraryWorkflowError> {
    let catalog = catalog.as_ref();
    if !catalog.is_file() {
        return Err(fail("catalog does not exist"));
    }
    if output.as_ref().symlink_metadata().is_ok() {
        return Err(fail("refusing to overwrite existing output"));
    }
    let snapshot = Catalog::open(catalog)
        .and_then(|c| c.library_snapshot())
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    if snapshot.tracks.is_empty() && snapshot.roots.is_empty() {
        return Err(fail("catalog holds no imported library"));
    }
    let (tracks, roots) = xml_library(&snapshot)?;
    let bytes = oxidj_rekordbox::write_library_xml(&tracks, &roots)
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    let preview = oxidj_rekordbox::parse(&bytes)
        .map_err(|_| fail("written XML was rejected by the XML preview"))?;
    if !verify_export(&snapshot, &preview) {
        return Err(fail("written XML did not match the catalog"));
    }
    crate::publish_new_file(output.as_ref(), &bytes)
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    Ok(LibraryExportReport {
        counts: snapshot.counts(),
        not_carried: NotCarried::of(&snapshot),
        preview_verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidj_rekordbox::{PdbFolderPreview, PdbPlaylistPreview, PdbTrackId, PdbTrackPreview};

    fn track(id: u32, artist: Option<&str>) -> PdbTrackPreview {
        PdbTrackPreview {
            source_id: PdbTrackId(id),
            title: format!("T{id}"),
            artist: artist.map(Into::into),
            album: None,
            tempo_hundredths_bpm: 12_000,
            duration_seconds: 60,
            file_path: format!("/Contents/t {id}.mp3"),
            ..PdbTrackPreview::default()
        }
    }

    fn playlist(id: u32, parent: u32, order: u32, members: &[u32]) -> PdbPlaylistPreview {
        PdbPlaylistPreview {
            source_id: PdbNodeId(id),
            parent_id: PdbNodeId(parent),
            name: format!("P{id}"),
            hierarchy: vec![],
            ordered_track_ids: members.iter().map(|&m| PdbTrackId(m)).collect(),
            sort_order: order,
        }
    }

    fn preview() -> PdbPreview {
        PdbPreview {
            tracks: vec![track(7, Some("A")), track(3, None)],
            // Sibling order follows sort_order, not source id or listing order.
            playlists: vec![playlist(20, 10, 1, &[3, 7, 3]), playlist(21, 0, 0, &[7])],
            folders: vec![PdbFolderPreview {
                source_id: PdbNodeId(10),
                parent_id: PdbNodeId(0),
                name: "F".into(),
                sort_order: 1,
            }],
        }
    }

    #[test]
    fn pdb_preview_maps_to_ordered_library_with_encoded_locations() {
        let (snapshot, encoded) = library_from_pdb_preview(&preview()).unwrap();
        assert_eq!(encoded, 2);
        assert_eq!(
            snapshot.tracks[0].location,
            "file://localhost/Contents/t%207.mp3"
        );
        assert_eq!(
            snapshot.roots,
            vec![
                LibraryNode::Playlist {
                    name: "P21".into(),
                    members: vec![0]
                },
                LibraryNode::Folder {
                    name: "F".into(),
                    children: vec![LibraryNode::Playlist {
                        name: "P20".into(),
                        members: vec![1, 0, 1]
                    }]
                }
            ]
        );
    }

    #[test]
    fn pdb_mapping_fails_closed_on_missing_member_orphan_and_duplicate_ids() {
        let mut missing = preview();
        missing.playlists[0].ordered_track_ids.push(PdbTrackId(99));
        assert_eq!(
            library_from_pdb_preview(&missing).unwrap_err(),
            fail("playlist member references a missing track")
        );
        let mut orphan = preview();
        orphan.folders[0].parent_id = PdbNodeId(55);
        assert_eq!(
            library_from_pdb_preview(&orphan).unwrap_err(),
            fail("playlist-tree node is unreachable from the root")
        );
        let mut duplicate = preview();
        duplicate.tracks[1].source_id = PdbTrackId(7);
        assert_eq!(
            library_from_pdb_preview(&duplicate).unwrap_err(),
            fail("duplicate source track identity")
        );
    }

    #[test]
    fn onelibrary_cross_check_reports_counts_and_equality_only() {
        let (snapshot, _) = library_from_pdb_preview(&preview()).unwrap();
        let id = |v: &str| crate::OneLibrarySourceId::new(v.into()).unwrap();
        let one = crate::OneLibraryPreview {
            protocol_version: 1,
            schema_profile: 2,
            tracks: vec![
                crate::OneLibraryTrack {
                    id: id("a"),
                    title: None,
                    artist: None,
                    album: None,
                },
                crate::OneLibraryTrack {
                    id: id("b"),
                    title: None,
                    artist: None,
                    album: None,
                },
            ],
            playlists: vec![crate::OneLibraryPlaylist {
                id: id("p"),
                parent_id: None,
                sibling_order: 0,
                name: "x".into(),
                track_ids: vec![id("a")],
            }],
        };
        let check = onelibrary_cross_check(&snapshot.counts(), &one, true);
        assert_eq!(
            check,
            OneLibraryCrossCheck {
                tracks: 2,
                playlists: 1,
                memberships: 1,
                tracks_equal: true,
                playlists_equal: false,
                memberships_equal: false,
                source_unchanged: true,
            }
        );
    }

    #[test]
    fn malformed_source_leaves_no_catalog() {
        let dir = std::env::temp_dir().join(format!(
            "oxidj-app-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let catalog = dir.join("new.sqlite");
        // A malformed source is rejected before any catalog is created.
        std::fs::write(dir.join("bad.pdb"), b"not a pdb").unwrap();
        assert!(import_library_with(&dir.join("bad.pdb"), &catalog, |_| Ok(None)).is_err());
        assert!(!catalog.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
