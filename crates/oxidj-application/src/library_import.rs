//! M7 slice: import a DeviceSQL PDB preview into a new local catalog, and export a
//! catalog's imported library as a new Rekordbox XML file. This layer composes the
//! adapter previews with library handlers; the library never sees adapter types.
//! Reports carry aggregate counts and booleans only — never names, paths or IDs.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use oxidj_library::{
    Catalog, LibraryBeat, LibraryCounts, LibraryCue, LibraryNode, LibrarySnapshot, LibraryTrack,
    VendorWaveform,
};
use oxidj_rekordbox::{
    LIBRARY_XML_NOT_WRITTEN_TRACK_ATTRIBUTES, LIBRARY_XML_NOT_WRITTEN_TRACK_ELEMENTS,
    LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES, LIBRARY_XML_OPTIONAL_TRACK_ELEMENTS, LibraryXmlMark,
    LibraryXmlNode, LibraryXmlTempo, LibraryXmlTrack, PdbNodeId, PdbPreview,
};

/// The track's ANLZ waveforms as vendor-derived catalog data (never computed peaks).
fn vendor_waveforms(track: &oxidj_rekordbox::PdbTrackPreview) -> Vec<VendorWaveform> {
    let detail = track.waveform_detail.as_ref().map(|columns| {
        VendorWaveform::RekordboxDetail(columns.iter().map(|c| [c.height, c.whiteness]).collect())
    });
    let colour = track.colour_waveform_detail.as_ref().map(|columns| {
        VendorWaveform::RekordboxColourDetail(
            columns
                .iter()
                .map(|c| [c.red, c.green, c.blue, c.height])
                .collect(),
        )
    });
    detail.into_iter().chain(colour).collect()
}

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
    /// (optional XML TRACK child element the library carries, number of tracks
    /// without any — so none was written). Absent, not dropped.
    pub elements_absent: Vec<(&'static str, usize)>,
}

impl NotCarried {
    /// Number of tracks that carry the named optional attribute.
    pub fn carried(&self, attribute: &str, tracks: usize) -> usize {
        tracks
            - self
                .absent
                .iter()
                .chain(&self.elements_absent)
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
            elements_absent: LIBRARY_XML_OPTIONAL_TRACK_ELEMENTS
                .iter()
                .map(|name| {
                    let present = |t: &LibraryTrack| match *name {
                        "TEMPO" => !t.beat_grid.is_empty(),
                        "POSITION_MARK" => !t.cues.is_empty(),
                        _ => unreachable!("every optional element is mapped"),
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
    /// With `computed_grids`: one decision per library track, in collection
    /// order; empty otherwise. Carries no names or paths.
    pub computed_grids: Vec<ComputedGridDecision>,
}

/// Below this analyser confidence a computed grid is not exported; the same
/// threshold as the library UI's "low" confidence badge.
pub const COMPUTED_GRID_MIN_CONFIDENCE: f32 = 0.5;

/// What `--computed-grids` did for one library track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputedGridDecision {
    /// The track has a vendor (Rekordbox) grid; it is exported unchanged.
    VendorKept,
    /// One constant `TEMPO` from the computed grid was written.
    Written,
    /// The computed grid's confidence is below [`COMPUTED_GRID_MIN_CONFIDENCE`].
    LowConfidence,
    /// The matching local track has no computed grid.
    NotAnalyzed,
    /// No local track matches the library location.
    NoMatch,
    /// More than one local track (or library location) matches.
    Ambiguous,
}

impl ComputedGridDecision {
    /// Every decision, in report order.
    pub const ALL: [Self; 6] = [
        Self::Written,
        Self::VendorKept,
        Self::LowConfidence,
        Self::NotAnalyzed,
        Self::NoMatch,
        Self::Ambiguous,
    ];

    /// Stable report label.
    pub fn label(self) -> &'static str {
        match self {
            Self::VendorKept => "VendorGridKept",
            Self::Written => "ComputedGridWritten",
            Self::LowConfidence => "SkippedLowConfidence",
            Self::NotAnalyzed => "SkippedNotAnalyzed",
            Self::NoMatch => "SkippedNoMatch",
            Self::Ambiguous => "SkippedAmbiguous",
        }
    }
}

/// Fill the empty beat grids of `snapshot` from computed grids (M7 slice 5):
/// one constant beat at the first computed beat, `beat_in_bar` 1, and the
/// computed BPM as the average BPM when the track has none. Vendor grids are
/// never replaced or merged.
fn add_computed_grids(
    catalog: &Catalog,
    snapshot: &mut LibrarySnapshot,
) -> Result<Vec<ComputedGridDecision>, LibraryWorkflowError> {
    use oxidj_library::LibraryComputedGrid as Lookup;
    snapshot
        .tracks
        .iter_mut()
        .map(|track| {
            if !track.beat_grid.is_empty() {
                return Ok(ComputedGridDecision::VendorKept);
            }
            let lookup = catalog
                .computed_beat_grid_for_library_location(&track.location)
                .map_err(|e| LibraryWorkflowError(e.to_string()))?;
            Ok(match lookup {
                Lookup::NoMatch => ComputedGridDecision::NoMatch,
                Lookup::Ambiguous => ComputedGridDecision::Ambiguous,
                Lookup::NotAnalyzed => ComputedGridDecision::NotAnalyzed,
                Lookup::Found(stored)
                    if stored.grid.confidence() < COMPUTED_GRID_MIN_CONFIDENCE =>
                {
                    ComputedGridDecision::LowConfidence
                }
                Lookup::Found(stored) => {
                    let bpm = (stored.grid.bpm() * 100.0).round();
                    let first = stored.grid.beats_ms().first().copied();
                    let (Some(time_ms), true) = (first, (1.0..=f64::from(u32::MAX)).contains(&bpm))
                    else {
                        return Err(fail("stored computed grid is invalid"));
                    };
                    let tempo_hundredths_bpm = bpm as u32;
                    track.beat_grid = vec![LibraryBeat {
                        time_ms,
                        tempo_hundredths_bpm,
                        beat_in_bar: 1,
                    }];
                    if track.tempo_hundredths_bpm == 0 {
                        track.tempo_hundredths_bpm = tempo_hundredths_bpm;
                    }
                    ComputedGridDecision::Written
                }
            })
        })
        .collect()
}

/// The `file://localhost/…` location an imported library track records for a
/// local `path` (already-URI paths pass through); `true` when it was encoded.
pub fn location_uri(path: &str) -> (String, bool) {
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

/// A Windows drive letter (`A`–`Z`, stored upper-case) for Rekordbox XML exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveLetter(char);

impl DriveLetter {
    /// Accept exactly one ASCII letter, case-insensitive; anything else is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        let mut chars = value.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphabetic() => Some(Self(c.to_ascii_uppercase())),
            _ => None,
        }
    }
}

const DEVICE_RELATIVE: &str = "file://localhost/Contents/";

/// Qualify a device-relative `file://localhost/Contents/…` location with `drive`
/// (`file://localhost/X:/Contents/…`) so Windows Rekordbox can resolve it. Every
/// other location (already drive-qualified, other paths, other hosts) is unchanged.
pub fn drive_qualified_location(location: &str, drive: Option<DriveLetter>) -> String {
    match (drive, location.strip_prefix(DEVICE_RELATIVE)) {
        (Some(DriveLetter(letter)), Some(rest)) => {
            format!("file://localhost/{letter}:/Contents/{rest}")
        }
        _ => location.to_owned(),
    }
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
            beat_grid: track
                .beat_grid
                .iter()
                .map(|b| LibraryBeat {
                    time_ms: b.time_ms,
                    tempo_hundredths_bpm: u32::from(b.tempo_hundredths_bpm),
                    beat_in_bar: u8::try_from(b.beat_in_bar).unwrap_or(0),
                })
                .collect(),
            cues: track
                .cues
                .iter()
                .map(|c| LibraryCue {
                    hot_cue: (c.hot_cue != 0).then_some(c.hot_cue),
                    start_ms: c.start_ms,
                    end_ms: c.loop_end_ms,
                    name: c.comment.clone().unwrap_or_default(),
                    // Colour is carried for hot cues only (memory cues have none in XML).
                    colour: c.rgb.filter(|_| c.hot_cue != 0),
                })
                .collect(),
            vendor_waveforms: vendor_waveforms(track),
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

/// Per-attribute (then per-element) counts of source values the PDB preview could
/// not interpret, in the order of [`LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES`] then
/// [`LIBRARY_XML_OPTIONAL_TRACK_ELEMENTS`] (zero counts included).
pub fn unresolved_counts(preview: &PdbPreview) -> Vec<(&'static str, usize)> {
    LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES
        .iter()
        .chain(LIBRARY_XML_OPTIONAL_TRACK_ELEMENTS)
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

/// The device root of a `<root>/PIONEER/rekordbox/export.pdb` layout, where ANLZ
/// `analyze_path` values (`/PIONEER/USBANLZ/...`) resolve; `None` for any other layout.
fn device_root(pdb: &Path) -> Option<std::path::PathBuf> {
    let rekordbox = pdb.parent()?;
    let pioneer = rekordbox.parent()?;
    (rekordbox.file_name()? == "rekordbox" && pioneer.file_name()? == "PIONEER")
        .then(|| pioneer.parent().map(Path::to_path_buf))
        .flatten()
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
    let mut preview = oxidj_rekordbox::parse_pdb(&before)
        .map_err(|_| fail("PDB source was rejected by the read-only preview"))?;
    oxidj_rekordbox::attach_analysis(&mut preview, device_root(pdb).as_deref());
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
    drive: Option<DriveLetter>,
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
                location: drive_qualified_location(&t.location, drive),
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
                tempos: t
                    .beat_grid
                    .iter()
                    .map(|b| LibraryXmlTempo {
                        start_ms: b.time_ms,
                        bpm_hundredths: b.tempo_hundredths_bpm,
                        beat_in_bar: b.beat_in_bar,
                    })
                    .collect(),
                marks: t
                    .cues
                    .iter()
                    .map(|c| LibraryXmlMark {
                        name: c.name.clone(),
                        start_ms: c.start_ms,
                        end_ms: c.end_ms,
                        hot_cue: c.hot_cue,
                        colour: c.colour,
                    })
                    .collect(),
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

fn seconds(ms: u32) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

fn tempos_match(actual: &[oxidj_rekordbox::Tempo], expected: &[LibraryBeat]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(x, b)| {
            x.start_seconds == seconds(b.time_ms)
                && x.bpm
                    == format!(
                        "{}.{:02}",
                        b.tempo_hundredths_bpm / 100,
                        b.tempo_hundredths_bpm % 100
                    )
                && x.meter == "4/4"
                && x.beat == b.beat_in_bar.to_string()
        })
}

fn marks_match(actual: &[oxidj_rekordbox::PositionMark], expected: &[LibraryCue]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(x, c)| {
            let colour = |i: usize| c.colour.map(|rgb| rgb[i].to_string());
            x.name == c.name
                && x.kind == if c.end_ms.is_some() { "4" } else { "0" }
                && x.start_seconds == seconds(c.start_ms)
                && x.end_seconds == c.end_ms.map(seconds)
                && x.number == c.hot_cue.map_or(-1, |n| i64::from(n) - 1).to_string()
                && x.red == colour(0)
                && x.green == colour(1)
                && x.blue == colour(2)
        })
}

/// Check re-parsed XML against the catalog snapshot: tracks, folder and playlist
/// paths in document order, and ordered member keys.
fn verify_export(
    snapshot: &LibrarySnapshot,
    preview: &crate::Preview,
    drive: Option<DriveLetter>,
) -> bool {
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
                    && x.location == drive_qualified_location(&t.location, drive)
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
                    && tempos_match(&x.tempos, &t.beat_grid)
                    && marks_match(&x.marks, &t.cues)
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

/// Append a root playlist to an existing imported catalog. `track_ids` is a
/// comma-separated list of this catalog's exported XML TrackIDs (position + 1),
/// not audio TrackIds or vendor IDs. Returns committed membership count only.
pub fn create_library_playlist(
    catalog: impl AsRef<Path>,
    name: &str,
    track_ids: &str,
) -> Result<usize, LibraryWorkflowError> {
    oxidj_library::validate_library_playlist_name(name)
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    if track_ids.is_empty() {
        return Err(fail("playlist membership must not be empty"));
    }
    let members: Vec<usize> = track_ids
        .split(',')
        .map(|id| {
            if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
                return Err(fail("track IDs must be comma-separated positive integers"));
            }
            id.parse::<usize>()
                .ok()
                .and_then(|id| id.checked_sub(1))
                .ok_or_else(|| fail("track IDs must be positive integers within range"))
        })
        .collect::<Result<_, _>>()?;
    if !catalog.as_ref().is_file() {
        return Err(fail("catalog does not exist"));
    }
    let mut opened = Catalog::open(catalog).map_err(|e| LibraryWorkflowError(e.to_string()))?;
    opened
        .create_library_playlist(name, &members)
        .map_err(|e| LibraryWorkflowError(e.to_string()))
}

/// Export a catalog's imported library as a NEW Rekordbox XML file, without edits.
/// The bytes are re-parsed and verified before publication; nothing is written on
/// failure and an existing output is never replaced. `drive` qualifies device-relative
/// locations for Windows Rekordbox (see [`drive_qualified_location`]).
/// `computed_grids` (opt-in) writes OxiDJ's computed grid for tracks without a
/// vendor grid (see [`ComputedGridDecision`]); when `false` the bytes are
/// exactly those of a plain export.
pub fn export_library_xml(
    catalog: impl AsRef<Path>,
    output: impl AsRef<Path>,
    drive: Option<DriveLetter>,
    computed_grids: bool,
) -> Result<LibraryExportReport, LibraryWorkflowError> {
    let catalog = catalog.as_ref();
    if !catalog.is_file() {
        return Err(fail("catalog does not exist"));
    }
    if output.as_ref().symlink_metadata().is_ok() {
        return Err(fail("refusing to overwrite existing output"));
    }
    let opened = Catalog::open(catalog).map_err(|e| LibraryWorkflowError(e.to_string()))?;
    let mut snapshot = opened
        .library_snapshot()
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    if snapshot.tracks.is_empty() && snapshot.roots.is_empty() {
        return Err(fail("catalog holds no imported library"));
    }
    let decisions = if computed_grids {
        add_computed_grids(&opened, &mut snapshot)?
    } else {
        Vec::new()
    };
    drop(opened);
    let (tracks, roots) = xml_library(&snapshot, drive)?;
    let bytes = oxidj_rekordbox::write_library_xml(&tracks, &roots)
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    let preview = oxidj_rekordbox::parse(&bytes)
        .map_err(|_| fail("written XML was rejected by the XML preview"))?;
    if !verify_export(&snapshot, &preview, drive) {
        return Err(fail("written XML did not match the catalog"));
    }
    crate::publish_new_file(output.as_ref(), &bytes)
        .map_err(|e| LibraryWorkflowError(e.to_string()))?;
    Ok(LibraryExportReport {
        counts: snapshot.counts(),
        not_carried: NotCarried::of(&snapshot),
        preview_verified: true,
        computed_grids: decisions,
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
    fn anlz_waveforms_map_to_labelled_vendor_waveforms_only_when_present() {
        let mut source = preview();
        source.tracks[0].waveform_detail = Some(vec![oxidj_rekordbox::AnlzDetailColumn {
            height: 31,
            whiteness: 2,
        }]);
        source.tracks[0].colour_waveform_detail = Some(vec![oxidj_rekordbox::AnlzColourColumn {
            red: 1,
            green: 2,
            blue: 3,
            height: 4,
        }]);
        source.tracks[1].colour_waveform_detail = Some(vec![]);
        let (snapshot, _) = library_from_pdb_preview(&source).unwrap();
        assert_eq!(
            snapshot.tracks[0].vendor_waveforms,
            vec![
                VendorWaveform::RekordboxDetail(vec![[31, 2]]),
                VendorWaveform::RekordboxColourDetail(vec![[1, 2, 3, 4]]),
            ]
        );
        assert_eq!(
            snapshot.tracks[1].vendor_waveforms,
            vec![VendorWaveform::RekordboxColourDetail(vec![])]
        );
        assert!(
            snapshot.tracks[2..]
                .iter()
                .all(|t| t.vendor_waveforms.is_empty())
        );
        // Persisted in the vendor table and read back verbatim.
        let path = std::env::temp_dir().join(format!(
            "oxidj-vendor-waveform-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut catalog = Catalog::open(&path).unwrap();
        catalog.import_library(&snapshot).unwrap();
        assert_eq!(catalog.library_snapshot().unwrap(), snapshot);
        drop(catalog);
        for file in catalog_files(&path) {
            let _ = std::fs::remove_file(file);
        }
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
    fn export_verification_checks_every_tempo_and_mark_field() {
        let mut t = track(1, None);
        t.beat_grid = vec![oxidj_rekordbox::AnlzBeat {
            beat_in_bar: 2,
            tempo_hundredths_bpm: 12_345,
            time_ms: 1_001,
        }];
        t.cues = vec![oxidj_rekordbox::AnlzCue {
            hot_cue: 2,
            start_ms: 5,
            loop_end_ms: Some(9),
            comment: Some("N".into()),
            rgb: Some([1, 2, 3]),
        }];
        let source = PdbPreview {
            tracks: vec![t],
            playlists: vec![],
            folders: vec![],
        };
        let (snapshot, _) = library_from_pdb_preview(&source).unwrap();
        let (tracks, roots) = xml_library(&snapshot, None).unwrap();
        let preview =
            oxidj_rekordbox::parse(&oxidj_rekordbox::write_library_xml(&tracks, &roots).unwrap())
                .unwrap();
        assert!(verify_export(&snapshot, &preview, None));
        let mutations: [fn(&mut LibraryTrack); 10] = [
            (|t| t.beat_grid[0].time_ms += 1),
            (|t| t.beat_grid[0].tempo_hundredths_bpm += 1),
            (|t| t.beat_grid[0].beat_in_bar = 3),
            (|t| t.beat_grid.clear()),
            (|t| t.cues[0].start_ms += 1),
            (|t| t.cues[0].end_ms = None),
            (|t| t.cues[0].hot_cue = None),
            (|t| t.cues[0].name.clear()),
            (|t| t.cues[0].colour = Some([1, 2, 4])),
            (|t| t.cues[0].colour = None),
        ];
        for mutate in mutations {
            let mut changed = snapshot.clone();
            mutate(&mut changed.tracks[0]);
            assert!(!verify_export(&changed, &preview, None));
        }
    }

    #[test]
    fn drive_letter_parses_one_ascii_letter_and_qualifies_device_relative_locations_only() {
        for ok in ["d", "D", "a", "Z"] {
            assert!(DriveLetter::parse(ok).is_some(), "{ok}");
        }
        for bad in ["", "DD", "C:", "1", "-", "\u{e9}", " D"] {
            assert_eq!(DriveLetter::parse(bad), None, "{bad:?}");
        }
        let d = DriveLetter::parse("d");
        for (input, want) in [
            (
                "file://localhost/Contents/a%20b.mp3",
                "file://localhost/D:/Contents/a%20b.mp3",
            ),
            // Already drive-qualified, other paths/hosts: unchanged.
            (
                "file://localhost/E:/Contents/x.mp3",
                "file://localhost/E:/Contents/x.mp3",
            ),
            (
                "file://localhost/D:/Music/x.mp3",
                "file://localhost/D:/Music/x.mp3",
            ),
            ("file://localhost/b.wav", "file://localhost/b.wav"),
            (
                "file://server/Contents/x.mp3",
                "file://server/Contents/x.mp3",
            ),
        ] {
            assert_eq!(drive_qualified_location(input, d), want);
            assert_eq!(drive_qualified_location(input, None), input);
        }
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

    /// M7 slice 5: `computed_grids` adds exactly one constant TEMPO to a no-grid
    /// track with a confident unique computed grid; vendor grids, low
    /// confidence, not analysed, no match and ambiguous matches all leave their
    /// track's bytes as in the plain export, with the exact decision reported.
    #[test]
    fn computed_grids_fill_only_confident_unique_no_grid_tracks() {
        use oxidj_core::tempo::TempoGrid;
        use oxidj_library::TempoBasis;
        let dir = std::env::temp_dir().join(format!(
            "oxidj-computed-grid-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("catalog.sqlite");
        let mut catalog = Catalog::open(&path).unwrap();
        let basis = TempoBasis {
            sample_rate: 44_100,
            duration_ms: 30_000,
            range_min_bpm: 70,
            range_max_bpm: 180,
        };
        let mut analysed = |name: &str, bpm: f64, confidence: f32| {
            let op = format!("op-{name}");
            let id = catalog
                .insert_track(&op, &format!("/m/Contents/{name}"), [1; 32], 1)
                .unwrap()
                .id;
            if bpm > 0.0 {
                let grid =
                    TempoGrid::new(bpm, vec![300, 769, 1_238], confidence, "taktus 0.1.0").unwrap();
                catalog
                    .store_computed_beat_grid(id, [1; 32], basis, &grid)
                    .unwrap();
            }
        };
        analysed("V.wav", 128.0, 0.9);
        analysed("W.wav", 128.0, 0.5);
        analysed("L.wav", 128.0, 0.49);
        analysed("N.wav", 0.0, 0.0);
        analysed("A.wav", 126.5, 0.95);
        catalog
            .insert_track("twin-2", "/n/Contents/Twin/t.wav", [1; 32], 1)
            .unwrap();
        catalog
            .insert_track("twin-1", "/m/Contents/Twin/t.wav", [1; 32], 1)
            .unwrap();
        let track = |name: &str, tempo: u32, grid: bool| LibraryTrack {
            title: name.into(),
            tempo_hundredths_bpm: tempo,
            duration_seconds: 30,
            location: format!("file://localhost/Contents/{name}"),
            beat_grid: if grid {
                vec![LibraryBeat {
                    time_ms: 11,
                    tempo_hundredths_bpm: 12_345,
                    beat_in_bar: 3,
                }]
            } else {
                vec![]
            },
            ..LibraryTrack::default()
        };
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![
                    track("V.wav", 12_345, true),
                    track("W.wav", 0, false),
                    track("L.wav", 0, false),
                    track("N.wav", 0, false),
                    track("Twin/t.wav", 0, false),
                    track("missing.wav", 0, false),
                    track("A.wav", 12_000, false),
                ],
                roots: vec![LibraryNode::Playlist {
                    name: "P".into(),
                    members: vec![1, 0],
                }],
            })
            .unwrap();
        drop(catalog);
        let export = |name: &str, computed: bool| {
            let out = dir.join(name);
            let report = export_library_xml(&path, &out, None, computed).unwrap();
            (report, std::fs::read_to_string(out).unwrap())
        };
        let (plain_report, plain) = export("plain.xml", false);
        assert!(plain_report.computed_grids.is_empty());
        let (report, computed) = export("computed.xml", true);
        use ComputedGridDecision as D;
        assert_eq!(
            report.computed_grids,
            [
                D::VendorKept,
                D::Written,
                D::LowConfidence,
                D::NotAnalyzed,
                D::Ambiguous,
                D::NoMatch,
                D::Written
            ]
        );
        // Expected bytes: the plain export with exactly two tracks gaining one
        // TEMPO each (W: AverageBpm filled; A: its own AverageBpm kept).
        let tempo = |bpm: &str| {
            format!(
                "      <TEMPO Inizio=\"0.300\" Bpm=\"{bpm}\" Metro=\"4/4\" Battito=\"1\"/>\n    </TRACK>"
            )
        };
        let w_plain =
            "TotalTime=\"30\" AverageBpm=\"0.00\" Location=\"file://localhost/Contents/W.wav\"/>";
        let a_plain =
            "TotalTime=\"30\" AverageBpm=\"120.00\" Location=\"file://localhost/Contents/A.wav\"/>";
        assert_eq!(plain.matches(w_plain).count(), 1, "{plain}");
        assert_eq!(plain.matches(a_plain).count(), 1, "{plain}");
        let expected = plain
            .replace(
                w_plain,
                &format!(
                    "TotalTime=\"30\" AverageBpm=\"128.00\" Location=\"file://localhost/Contents/W.wav\">\n{}",
                    tempo("128.00")
                ),
            )
            .replace(
                a_plain,
                &format!(
                    "TotalTime=\"30\" AverageBpm=\"120.00\" Location=\"file://localhost/Contents/A.wav\">\n{}",
                    tempo("126.50")
                ),
            );
        assert_eq!(computed, expected);
        // Vendor track: unchanged even though a confident computed grid exists.
        assert!(
            computed
                .contains("<TEMPO Inizio=\"0.011\" Bpm=\"123.45\" Metro=\"4/4\" Battito=\"3\"/>")
        );
        assert_eq!(computed.matches("<TEMPO ").count(), 3);
        // The catalog is only read: a later plain export is byte-identical.
        let (_, again) = export("again.xml", false);
        assert_eq!(again, plain);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
