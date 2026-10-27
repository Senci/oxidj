//! Inspection and bounded byte-preserving edits for a Rekordbox XML subset.
use quick_xml::{Reader, events::Event};
use std::{collections::HashSet, fs, path::Path};

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub comments: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub composer: Option<String>,
    pub grouping: Option<String>,
    pub remixer: Option<String>,
    pub mix: Option<String>,
    pub tonality: Option<String>,
    pub kind: Option<String>,
    pub year: Option<String>,
    pub size: Option<String>,
    pub disc_number: Option<String>,
    pub track_number: Option<String>,
    pub total_time: Option<String>,
    pub bit_rate: Option<String>,
    pub sample_rate: Option<String>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub last_played: Option<String>,
    pub play_count: Option<String>,
    pub rating: Option<String>,
    pub colour: Option<String>,
    pub average_bpm: Option<String>,
    pub location: String,
    pub tempos: Vec<Tempo>,
    pub marks: Vec<PositionMark>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tempo {
    pub start_seconds: String,
    pub bpm: String,
    pub meter: String,
    pub beat: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PositionMark {
    pub name: String,
    pub kind: String,
    pub start_seconds: String,
    pub number: String,
    pub end_seconds: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub path: Vec<String>,
    pub name: String,
    pub tracks: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub unsupported: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportDecision {
    AllowExport,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPreflight {
    pub decision: ExportDecision,
    pub selected_tracks: Vec<Track>,
    pub selected_playlists: Vec<Playlist>,
    pub withheld_units: Vec<String>,
    pub loss_report: Vec<String>,
}
/// One optimistic, ordinal-scoped playlist rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistRenameRequest {
    pub playlist_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistRenameChange {
    pub playlist_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistRename {
    pub bytes: Vec<u8>,
    pub change: PlaylistRenameChange,
}
/// Delete one collection record and all playlist memberships under an exact guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackDeleteRequest {
    pub track_id: String,
    pub expected_memberships: Vec<(usize, Vec<String>)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackDelete {
    pub bytes: Vec<u8>,
}
/// Remove one playlist reference while retaining its collection track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberDeleteRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberDelete {
    pub bytes: Vec<u8>,
}
/// Clear playlist references without deleting collection tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistClearRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistClear {
    pub bytes: Vec<u8>,
}
/// Append one existing track to a playlist after an exact ordinal/order guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberAppendRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberAppend {
    pub bytes: Vec<u8>,
}
/// Insert one absent collection track at a zero-based playlist position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberInsertRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub track_id: String,
    pub position: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberInsert {
    pub bytes: Vec<u8>,
}
/// Reorder an existing playlist after an exact ordinal/order optimistic guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistReorderRequest {
    pub playlist_index: usize,
    pub expected_order: Vec<String>,
    pub new_order: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistReorder {
    pub bytes: Vec<u8>,
}
/// One optimistic, ordinal-scoped rename of a non-root Type=0 folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRenameRequest {
    pub folder_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRenameChange {
    pub folder_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderRename {
    pub bytes: Vec<u8>,
    pub change: FolderRenameChange,
}
/// One optimistic, ordinal-scoped tempo edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_bpm: String,
    pub new_bpm: String,
}
/// One optimistic, ordinal-scoped tempo-point start edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoStartEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_start_seconds: String,
    pub new_start_seconds: String,
}
/// One optimistic, ordinal-scoped tempo-point Battito edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoBeatEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_beat: String,
    pub new_beat: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoBeatEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_beat: String,
    pub new_beat: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoBeatEdit {
    pub bytes: Vec<u8>,
    pub change: TempoBeatEditChange,
}
/// One optimistic, ordinal-scoped raw tempo-point Metro edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMeterEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_meter: String,
    pub new_meter: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMeterEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_meter: String,
    pub new_meter: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoMeterEdit {
    pub bytes: Vec<u8>,
    pub change: TempoMeterEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoStartEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_start_seconds: String,
    pub new_start_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoStartEdit {
    pub bytes: Vec<u8>,
    pub change: TempoStartEditChange,
}
/// The exact change prepared for a source track's tempo point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_bpm: String,
    pub new_bpm: String,
}
/// Validated, transformed XML bytes plus the proposed change summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoEdit {
    pub bytes: Vec<u8>,
    pub change: TempoEditChange,
}
/// One optimistic, ordinal-scoped position mark start edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_start_seconds: String,
    pub new_start_seconds: String,
}
/// The exact change prepared for a source track's position mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_start_seconds: String,
    pub new_start_seconds: String,
}
/// Validated, transformed XML bytes plus the proposed change summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkNameEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkNameEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkNameEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkNameEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackNameEditRequest {
    pub track_id: String,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackNameEditChange {
    pub track_id: String,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackArtistEditRequest {
    pub track_id: String,
    pub expected_old_artist: String,
    pub new_artist: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackArtistEditChange {
    pub track_id: String,
    pub old_artist: String,
    pub new_artist: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackArtistEdit {
    pub bytes: Vec<u8>,
    pub change: TrackArtistEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackCommentsEditRequest {
    pub track_id: String,
    pub expected_old_comments: String,
    pub new_comments: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackCommentsEditChange {
    pub track_id: String,
    pub old_comments: String,
    pub new_comments: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackCommentsEdit {
    pub bytes: Vec<u8>,
    pub change: TrackCommentsEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackAlbumEditRequest {
    pub track_id: String,
    pub expected_old_album: String,
    pub new_album: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackAlbumEditChange {
    pub track_id: String,
    pub old_album: String,
    pub new_album: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackGenreEditRequest {
    pub track_id: String,
    pub expected_old_genre: String,
    pub new_genre: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackGenreEditChange {
    pub track_id: String,
    pub old_genre: String,
    pub new_genre: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackGenreEdit {
    pub bytes: Vec<u8>,
    pub change: TrackGenreEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackLabelEditRequest {
    pub track_id: String,
    pub expected_old_label: String,
    pub new_label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackLabelEditChange {
    pub track_id: String,
    pub old_label: String,
    pub new_label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackLabelEdit {
    pub bytes: Vec<u8>,
    pub change: TrackLabelEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackComposerEditRequest {
    pub track_id: String,
    pub expected_old_composer: String,
    pub new_composer: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackComposerEditChange {
    pub track_id: String,
    pub old_composer: String,
    pub new_composer: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackComposerEdit {
    pub bytes: Vec<u8>,
    pub change: TrackComposerEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackGroupingEditRequest {
    pub track_id: String,
    pub expected_old_grouping: String,
    pub new_grouping: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackGroupingEditChange {
    pub track_id: String,
    pub old_grouping: String,
    pub new_grouping: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackGroupingEdit {
    pub bytes: Vec<u8>,
    pub change: TrackGroupingEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRemixerEditRequest {
    pub track_id: String,
    pub expected_old_remixer: String,
    pub new_remixer: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRemixerEditChange {
    pub track_id: String,
    pub old_remixer: String,
    pub new_remixer: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackRemixerEdit {
    pub bytes: Vec<u8>,
    pub change: TrackRemixerEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMixEditRequest {
    pub track_id: String,
    pub expected_old_mix: String,
    pub new_mix: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMixEditChange {
    pub track_id: String,
    pub old_mix: String,
    pub new_mix: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackMixEdit {
    pub bytes: Vec<u8>,
    pub change: TrackMixEditChange,
}
/// Raw Tonality is preserved as supplied; no key notation is inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTonalityEditRequest {
    pub track_id: String,
    pub expected_old_tonality: String,
    pub new_tonality: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTonalityEditChange {
    pub track_id: String,
    pub old_tonality: String,
    pub new_tonality: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackTonalityEdit {
    pub bytes: Vec<u8>,
    pub change: TrackTonalityEditChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackAlbumEdit {
    pub bytes: Vec<u8>,
    pub change: TrackAlbumEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackNameEdit {
    pub bytes: Vec<u8>,
    pub change: TrackNameEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEndEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_end_seconds: String,
    pub new_end_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEndEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_end_seconds: String,
    pub new_end_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkEndEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkEndEditChange,
}
#[derive(Debug)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Result<String, Error> {
    e.attributes()
        .with_checks(true)
        .find_map(|a| match a {
            Ok(a) if a.key.as_ref() == key.as_bytes() => Some(
                a.decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, e.decoder())
                    .map(|v| v.into_owned())
                    .map_err(|x| Error(x.to_string())),
            ),
            _ => None,
        })
        .ok_or_else(|| Error(format!("missing required attribute {key}")))?
}
fn optional(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Result<Option<String>, Error> {
    for a in e.attributes().with_checks(true) {
        let a = a.map_err(|x| Error(x.to_string()))?;
        if a.key.as_ref() == key.as_bytes() {
            return a
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, e.decoder())
                .map(|v| Some(v.into_owned()))
                .map_err(|x| Error(x.to_string()));
        }
    }
    Ok(None)
}
fn validate_decimal(field: &str, raw: &str, positive: bool) -> Result<(), Error> {
    let parsed = raw.parse::<f64>().ok().filter(|n| n.is_finite());
    if parsed.is_none_or(|n| n < 0.0 || (positive && n == 0.0)) {
        return Err(Error(format!("invalid {field} value {raw}")));
    }
    Ok(())
}

pub fn parse_file(path: impl AsRef<Path>) -> Result<Preview, Error> {
    let bytes = fs::read(path).map_err(|e| Error(e.to_string()))?;
    parse(&bytes)
}
pub fn parse(bytes: &[u8]) -> Result<Preview, Error> {
    let mut r = Reader::from_reader(bytes);
    r.config_mut().trim_text(true);
    r.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut out = Preview {
        tracks: vec![],
        playlists: vec![],
        unsupported: vec![],
    };
    let mut ids = HashSet::new();
    let mut names: Vec<String> = vec![];
    let mut current: Option<Track> = None;
    let mut playlist: Option<Playlist> = None;
    loop {
        match r
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if stack.is_empty() && tag == "DJ_PLAYLISTS" {
                    let v = attr(&e, "Version")?;
                    if v != "1.0.0" {
                        return Err(Error(format!("unsupported XML version {v}")));
                    }
                }
                if tag == "TRACK" && stack.last().is_some_and(|x| x == "COLLECTION") {
                    let id = attr(&e, "TrackID")?;
                    if !ids.insert(id.clone()) {
                        return Err(Error(format!("duplicate track identity {id}")));
                    }
                    current = Some(Track {
                        id,
                        name: attr(&e, "Name")?,
                        artist: optional(&e, "Artist")?,
                        comments: optional(&e, "Comments")?,
                        album: optional(&e, "Album")?,
                        genre: optional(&e, "Genre")?,
                        label: optional(&e, "Label")?,
                        composer: optional(&e, "Composer")?,
                        grouping: optional(&e, "Grouping")?,
                        remixer: optional(&e, "Remixer")?,
                        mix: optional(&e, "Mix")?,
                        tonality: optional(&e, "Tonality")?,
                        kind: optional(&e, "Kind")?,
                        year: optional(&e, "Year")?,
                        size: optional(&e, "Size")?,
                        disc_number: optional(&e, "DiscNumber")?,
                        track_number: optional(&e, "TrackNumber")?,
                        total_time: optional(&e, "TotalTime")?,
                        bit_rate: optional(&e, "BitRate")?,
                        sample_rate: optional(&e, "SampleRate")?,
                        date_added: optional(&e, "DateAdded")?,
                        date_modified: optional(&e, "DateModified")?,
                        last_played: optional(&e, "LastPlayed")?,
                        play_count: optional(&e, "PlayCount")?,
                        rating: optional(&e, "Rating")?,
                        colour: optional(&e, "Colour")?,
                        average_bpm: optional(&e, "AverageBpm")?,
                        location: attr(&e, "Location")?,
                        tempos: vec![],
                        marks: vec![],
                    });
                } else if tag == "NODE" {
                    let ty = attr(&e, "Type")?;
                    let n = attr(&e, "Name")?;
                    if ty == "1" {
                        playlist = Some(Playlist {
                            path: names
                                .iter()
                                .skip(1)
                                .cloned()
                                .chain(std::iter::once(n.clone()))
                                .collect(),
                            name: n,
                            tracks: vec![],
                        });
                    } else if ty == "0" {
                        names.push(n);
                    } else {
                        out.unsupported.push(format!("NODE Type={ty}"));
                    }
                } else if tag == "TEMPO" {
                    if let Some(t) = current.as_mut() {
                        t.tempos.push(Tempo {
                            start_seconds: attr(&e, "Inizio")?,
                            bpm: attr(&e, "Bpm")?,
                            meter: attr(&e, "Metro")?,
                            beat: attr(&e, "Battito")?,
                        });
                    }
                } else if tag == "POSITION_MARK" {
                    if let Some(t) = current.as_mut() {
                        t.marks.push(PositionMark {
                            name: attr(&e, "Name")?,
                            kind: attr(&e, "Type")?,
                            start_seconds: attr(&e, "Start")?,
                            number: attr(&e, "Num")?,
                            end_seconds: optional(&e, "End")?,
                        });
                    }
                } else if tag == "TRACK"
                    && let Some(p) = playlist.as_mut()
                {
                    p.tracks.push(attr(&e, "Key")?);
                } else if ![
                    "DJ_PLAYLISTS",
                    "PRODUCT",
                    "COLLECTION",
                    "TRACK",
                    "TEMPO",
                    "POSITION_MARK",
                    "PLAYLISTS",
                    "NODE",
                ]
                .contains(&tag.as_str())
                {
                    out.unsupported.push(tag.clone());
                }
                stack.push(tag);
            }
            Event::Empty(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if tag == "DJ_PLAYLISTS" {
                    let v = attr(&e, "Version")?;
                    if v != "1.0.0" {
                        return Err(Error(format!("unsupported XML version {v}")));
                    }
                }
                if tag == "TRACK" && stack.last().is_some_and(|x| x == "COLLECTION") {
                    let id = attr(&e, "TrackID")?;
                    if !ids.insert(id.clone()) {
                        return Err(Error(format!("duplicate track identity {id}")));
                    }
                    out.tracks.push(Track {
                        id,
                        name: attr(&e, "Name")?,
                        artist: optional(&e, "Artist")?,
                        comments: optional(&e, "Comments")?,
                        album: optional(&e, "Album")?,
                        genre: optional(&e, "Genre")?,
                        label: optional(&e, "Label")?,
                        composer: optional(&e, "Composer")?,
                        grouping: optional(&e, "Grouping")?,
                        remixer: optional(&e, "Remixer")?,
                        mix: optional(&e, "Mix")?,
                        tonality: optional(&e, "Tonality")?,
                        kind: optional(&e, "Kind")?,
                        year: optional(&e, "Year")?,
                        size: optional(&e, "Size")?,
                        disc_number: optional(&e, "DiscNumber")?,
                        track_number: optional(&e, "TrackNumber")?,
                        total_time: optional(&e, "TotalTime")?,
                        bit_rate: optional(&e, "BitRate")?,
                        sample_rate: optional(&e, "SampleRate")?,
                        date_added: optional(&e, "DateAdded")?,
                        date_modified: optional(&e, "DateModified")?,
                        last_played: optional(&e, "LastPlayed")?,
                        play_count: optional(&e, "PlayCount")?,
                        rating: optional(&e, "Rating")?,
                        colour: optional(&e, "Colour")?,
                        average_bpm: optional(&e, "AverageBpm")?,
                        location: attr(&e, "Location")?,
                        tempos: vec![],
                        marks: vec![],
                    });
                } else if tag == "TRACK" && playlist.is_some() {
                    playlist.as_mut().unwrap().tracks.push(attr(&e, "Key")?);
                } else if tag == "TEMPO" {
                    if let Some(t) = current.as_mut() {
                        t.tempos.push(Tempo {
                            start_seconds: attr(&e, "Inizio")?,
                            bpm: attr(&e, "Bpm")?,
                            meter: attr(&e, "Metro")?,
                            beat: attr(&e, "Battito")?,
                        });
                    }
                } else if tag == "POSITION_MARK" {
                    if let Some(t) = current.as_mut() {
                        t.marks.push(PositionMark {
                            name: attr(&e, "Name")?,
                            kind: attr(&e, "Type")?,
                            start_seconds: attr(&e, "Start")?,
                            number: attr(&e, "Num")?,
                            end_seconds: optional(&e, "End")?,
                        });
                    }
                } else if tag == "NODE" {
                    let ty = attr(&e, "Type")?;
                    let n = attr(&e, "Name")?;
                    if ty == "1" {
                        out.playlists.push(Playlist {
                            path: names
                                .iter()
                                .skip(1)
                                .cloned()
                                .chain(std::iter::once(n.clone()))
                                .collect(),
                            name: n,
                            tracks: vec![],
                        });
                    } else if ty == "0" {
                        if names.is_empty() {
                            names.push(n);
                        }
                    } else {
                        out.unsupported.push(format!("NODE Type={ty}"));
                    }
                } else if tag == "PRODUCT" {
                } else {
                    out.unsupported.push(tag);
                }
            }
            Event::End(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if tag == "TRACK"
                    && let Some(t) = current.take()
                {
                    out.tracks.push(t);
                }
                if tag == "NODE" {
                    if let Some(p) = playlist.take() {
                        out.playlists.push(p);
                    } else if names.len() > 1 {
                        names.pop();
                    }
                }
                stack.pop();
            }
            Event::Eof => {
                if !stack.is_empty() {
                    return Err(Error("malformed XML: unexpected end of document".into()));
                }
                break;
            }
            Event::Decl(_)
            | Event::Comment(_)
            | Event::Text(_)
            | Event::CData(_)
            | Event::DocType(_) => {}
            _ => {}
        }
        buf.clear();
    }
    let known: HashSet<_> = out.tracks.iter().map(|t| t.id.as_str()).collect();
    for track in &out.tracks {
        if track.id.parse::<u64>().ok().is_none_or(|id| id == 0) {
            return Err(Error(format!("invalid TrackID {}", track.id)));
        }
        for tempo in &track.tempos {
            validate_decimal("tempo start", &tempo.start_seconds, false)?;
            validate_decimal("BPM", &tempo.bpm, true)?;
            tempo
                .beat
                .parse::<u32>()
                .map_err(|_| Error(format!("invalid tempo beat {}", tempo.beat)))?;
        }
        for mark in &track.marks {
            mark.kind
                .parse::<u8>()
                .map_err(|_| Error(format!("invalid position mark Type {}", mark.kind)))?;
            mark.number
                .parse::<u32>()
                .map_err(|_| Error(format!("invalid position mark Num {}", mark.number)))?;
            validate_decimal("position mark Start", &mark.start_seconds, false)?;
            if let Some(end) = &mark.end_seconds {
                validate_decimal("position mark End", end, false)?;
            }
        }
    }
    for p in &out.playlists {
        for id in &p.tracks {
            if !known.contains(id.as_str()) {
                return Err(Error(format!("dangling playlist track reference {id}")));
            }
        }
    }
    Ok(out)
}

fn validate_export_element(
    e: &quick_xml::events::BytesStart<'_>,
    parent: Option<&str>,
) -> Result<String, Error> {
    let name = std::str::from_utf8(e.name().as_ref())
        .map_err(|_| Error("non-UTF-8 element name".into()))?
        .to_owned();
    let allowed: &[&str] = match name.as_str() {
        "DJ_PLAYLISTS" => &["Version"],
        "PRODUCT" => &["Name", "Version", "Company"],
        "COLLECTION" => &["Entries"],
        "TRACK" if parent == Some("COLLECTION") => &[
            "TrackID",
            "Name",
            "Artist",
            "Comments",
            "Album",
            "Genre",
            "Label",
            "Composer",
            "Grouping",
            "Remixer",
            "Mix",
            "Tonality",
            "Kind",
            "Year",
            "Size",
            "DiscNumber",
            "TrackNumber",
            "TotalTime",
            "BitRate",
            "SampleRate",
            "DateAdded",
            "DateModified",
            "LastPlayed",
            "PlayCount",
            "Rating",
            "Colour",
            "AverageBpm",
            "Location",
        ],
        "TRACK" => &["Key"],
        "TEMPO" => &["Inizio", "Bpm", "Metro", "Battito"],
        "POSITION_MARK" => &["Name", "Type", "Start", "Num", "End"],
        "PLAYLISTS" => &[],
        "NODE" => &["Type", "Name", "Count", "Entries", "KeyType"],
        _ => return Err(Error(format!("unsupported export element {name}"))),
    };
    let mut seen = HashSet::new();
    for a in e.attributes().with_checks(true) {
        let a = a.map_err(|e| Error(e.to_string()))?;
        let key = std::str::from_utf8(a.key.as_ref())
            .map_err(|_| Error("non-UTF-8 attribute name".into()))?;
        if !allowed.contains(&key) {
            return Err(Error(format!("unsupported {name} attribute {key}")));
        }
        seen.insert(key.to_owned());
    }
    let required: &[&str] = match name.as_str() {
        "DJ_PLAYLISTS" => &["Version"],
        "PRODUCT" => &["Name", "Version", "Company"],
        "COLLECTION" => &["Entries"],
        "TRACK" if parent == Some("COLLECTION") => &["TrackID", "Name", "Location"],
        "TRACK" => &["Key"],
        "TEMPO" => &["Inizio", "Bpm", "Metro", "Battito"],
        "POSITION_MARK" => &["Name", "Type", "Start", "Num"],
        "NODE" => &["Type", "Name"],
        _ => &[],
    };
    for key in required {
        if !seen.contains(*key) {
            return Err(Error(format!("{name} missing required attribute {key}")));
        }
    }
    if name == "NODE" {
        let node_type = optional(e, "Type")?.ok_or_else(|| Error("NODE missing Type".into()))?;
        match node_type.as_str() {
            "0" if seen.contains("Count") => {}
            "1" if seen.contains("Entries") && seen.contains("KeyType") => {
                if optional(e, "KeyType")?.as_deref() != Some("0") {
                    return Err(Error("unsupported playlist KeyType".into()));
                }
            }
            "0" => return Err(Error("folder NODE missing Count".into())),
            "1" => return Err(Error("playlist NODE missing Entries or KeyType".into())),
            _ => return Err(Error(format!("unsupported NODE Type={node_type}"))),
        }
    }
    Ok(name)
}

fn valid_export_parent(stack: &[String], name: &str) -> bool {
    match stack.last().map(String::as_str) {
        None => name == "DJ_PLAYLISTS",
        Some("DJ_PLAYLISTS") => ["PRODUCT", "COLLECTION", "PLAYLISTS"].contains(&name),
        Some("COLLECTION") => name == "TRACK",
        Some("PLAYLISTS") => name == "NODE",
        Some("NODE") => ["NODE", "TRACK"].contains(&name),
        Some("TRACK") if stack.iter().any(|p| p == "COLLECTION") => {
            ["TEMPO", "POSITION_MARK"].contains(&name)
        }
        _ => false,
    }
}

/// Validate the frozen no-edit XML subset without writing or modifying data.
pub fn preflight_export(bytes: &[u8]) -> Result<ExportPreflight, Error> {
    if std::str::from_utf8(bytes).is_err() {
        return Err(Error("export requires UTF-8 XML".into()));
    }
    let preview = parse(bytes)?;
    if !preview.unsupported.is_empty() {
        return Err(Error(format!(
            "unsupported export content: {}",
            preview.unsupported.join(", ")
        )));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut roots = 0usize;
    let mut top_level = Vec::new();
    // Each playlist declares direct track references; each folder declares direct child NODES.
    let mut node_counts: Vec<(String, usize, usize)> = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e) => {
                let name = validate_export_element(&e, stack.last().map(String::as_str))?;
                if name == "NODE" {
                    let ty = attr(&e, "Type")?;
                    let field = if ty == "0" { "Count" } else { "Entries" };
                    let declared = attr(&e, field)?
                        .parse::<usize>()
                        .map_err(|_| Error(format!("invalid NODE {field}")))?;
                    node_counts.push((ty.clone(), declared, 0));
                    if let Some((parent_ty, _, count)) = node_counts.iter_mut().rev().nth(1)
                        && parent_ty == "0"
                    {
                        *count += 1;
                    }
                }
                if stack.last().is_some_and(|p| p == "NODE")
                    && name == "TRACK"
                    && let Some((ty, _, count)) = node_counts.last_mut()
                    && ty == "1"
                {
                    *count += 1;
                }
                if name == "COLLECTION" {
                    let declared = attr(&e, "Entries")?
                        .parse::<usize>()
                        .map_err(|_| Error("invalid COLLECTION Entries".into()))?;
                    if declared != preview.tracks.len() {
                        return Err(Error("COLLECTION Entries count mismatch".into()));
                    }
                }
                if !valid_export_parent(&stack, &name) {
                    return Err(Error(format!("invalid XML structure: {name}")));
                }
                if stack.is_empty() {
                    roots += 1;
                }
                if stack.last().is_some_and(|p| p == "DJ_PLAYLISTS") {
                    top_level.push(name.clone());
                }
                stack.push(name);
            }
            Event::Empty(e) => {
                let name = validate_export_element(&e, stack.last().map(String::as_str))?;
                if name == "NODE" {
                    let ty = attr(&e, "Type")?;
                    let field = if ty == "0" { "Count" } else { "Entries" };
                    let declared = attr(&e, field)?
                        .parse::<usize>()
                        .map_err(|_| Error(format!("invalid NODE {field}")))?;
                    if declared != 0 {
                        return Err(Error("NODE Count/Entries mismatch".into()));
                    }
                    if let Some((parent_ty, _, count)) = node_counts.last_mut()
                        && parent_ty == "0"
                    {
                        *count += 1;
                    }
                }
                if stack.last().is_some_and(|p| p == "NODE")
                    && name == "TRACK"
                    && let Some((ty, _, count)) = node_counts.last_mut()
                    && ty == "1"
                {
                    *count += 1;
                }
                if !valid_export_parent(&stack, &name) {
                    return Err(Error(format!("invalid XML structure: {name}")));
                }
                if stack.is_empty() {
                    roots += 1;
                }
                if stack.last().is_some_and(|p| p == "DJ_PLAYLISTS") {
                    top_level.push(name);
                }
            }
            Event::End(e) => {
                let end_name = e.name();
                let tag = std::str::from_utf8(end_name.as_ref())
                    .map_err(|_| Error("non-UTF-8 closing element".into()))?;
                if tag == "NODE"
                    && let Some((_, declared, actual)) = node_counts.pop()
                    && declared != actual
                {
                    return Err(Error("NODE Count/Entries mismatch".into()));
                }
                stack
                    .pop()
                    .ok_or_else(|| Error("unexpected closing element".into()))?;
            }
            Event::Text(t) => {
                if !t
                    .decode()
                    .map_err(|e| Error(e.to_string()))?
                    .trim()
                    .is_empty()
                {
                    return Err(Error("text content unsupported by export subset".into()));
                }
            }
            Event::Decl(d) => {
                let declaration = std::str::from_utf8(d.as_ref())
                    .map_err(|_| Error("invalid XML declaration".into()))?;
                if !declaration.contains("encoding=\"UTF-8\"")
                    && !declaration.contains("encoding='UTF-8'")
                {
                    return Err(Error(
                        "export requires an explicit UTF-8 XML declaration".into(),
                    ));
                }
            }
            Event::Eof => break,
            _ => return Err(Error("unsupported XML construct in export subset".into())),
        }
        buf.clear();
    }
    if roots != 1 || !stack.is_empty() || top_level != ["PRODUCT", "COLLECTION", "PLAYLISTS"] {
        return Err(Error(
            "invalid XML document structure or required top-level ordering".into(),
        ));
    }
    if preview.tracks.is_empty() {
        return Err(Error(
            "export requires at least one collection track".into(),
        ));
    }
    Ok(ExportPreflight {
        decision: ExportDecision::AllowExport,
        selected_tracks: preview.tracks,
        selected_playlists: preview.playlists,
        withheld_units: Vec::new(),
        loss_report: Vec::new(),
    })
}

/// Prepare one guarded playlist rename, preserving all source bytes outside Name.
pub fn prepare_playlist_rename(
    bytes: &[u8],
    request: &PlaylistRenameRequest,
) -> Result<PreparedPlaylistRename, Error> {
    let before = preflight_export(bytes)?;
    let old = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error(format!("no playlist at ordinal {}", request.playlist_index)))?;
    if old.name != request.expected_old_name {
        return Err(Error("playlist Name changed".into()));
    }
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| {
                let n = c as u32;
                n == 9
                    || n == 10
                    || n == 13
                    || (32..=0xD7FF).contains(&n)
                    || (0xE000..=0xFFFD).contains(&n)
                    || (0x10000..=0x10FFFF).contains(&n)
            })
    };
    if !valid(&request.new_name) {
        return Err(Error("invalid playlist Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut siblings: Vec<(Vec<String>, String)> = Vec::new();
    let mut playlist_no = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect();
                siblings.push((parent, name.clone()));
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    playlist_no += 1;
                }
                node_path.push(name);
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect();
                siblings.push((parent, name));
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    playlist_no += 1;
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let range = range.ok_or_else(|| Error("playlist Name location missing".into()))?;
    let parent = old.path[..old.path.len().saturating_sub(1)].to_vec();
    let matching_siblings = siblings
        .iter()
        .filter(|(sibling_parent, name)| sibling_parent == &parent && name == &request.new_name)
        .count();
    if matching_siblings > usize::from(request.expected_old_name == request.new_name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let escaped = request
        .new_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index].name = request.new_name.clone();
    expected[request.playlist_index].path.pop();
    expected[request.playlist_index]
        .path
        .push(request.new_name.clone());
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("playlist rename verification failed".into()));
    }
    Ok(PreparedPlaylistRename {
        bytes: output,
        change: PlaylistRenameChange {
            playlist_index: request.playlist_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

/// Prepare deletion of one collection track and all of its guarded playlist references.
pub fn prepare_track_delete(
    bytes: &[u8],
    request: &TrackDeleteRequest,
) -> Result<PreparedTrackDelete, Error> {
    let before = preflight_export(bytes)?;
    let track = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .count();
    if track != 1 {
        return Err(Error(
            "target collection track missing or duplicated".into(),
        ));
    }
    let actual_memberships: Vec<_> = before
        .selected_playlists
        .iter()
        .enumerate()
        .filter(|(_, p)| p.tracks.iter().any(|id| id == &request.track_id))
        .map(|(i, p)| (i, p.tracks.clone()))
        .collect();
    if request.expected_memberships != actual_memberships
        || actual_memberships.is_empty()
        || actual_memberships
            .iter()
            .any(|(_, ids)| ids.iter().filter(|id| *id == &request.track_id).count() != 1)
    {
        return Err(Error(
            "playlist membership guard is stale, incomplete, or duplicated".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut playlist_index = None;
    let mut next_playlist = 0usize;
    let mut collection_spans = Vec::new();
    let mut membership_spans: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
    let mut playlist_tag_spans = Vec::new();
    let mut collection_tag_span = None;
    let mut target_open: Option<usize> = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let eof = matches!(&event, Event::Eof);
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"COLLECTION" => {
                collection_tag_span = Some(start..end);
                stack.push("COLLECTION".into());
            }
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "1" {
                    playlist_index = Some(next_playlist);
                    playlist_tag_spans.push((next_playlist, start..end));
                    next_playlist += 1;
                }
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    playlist_tag_spans.push((next_playlist, start..end));
                    next_playlist += 1;
                }
            }
            Event::Start(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.iter().any(|p| p == "COLLECTION")
                    && attr(&e, "TrackID")? == request.track_id =>
            {
                target_open = Some(start);
                stack.push("TRACK".into());
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.iter().any(|p| p == "COLLECTION")
                    && attr(&e, "TrackID")? == request.track_id =>
            {
                collection_spans.push(start..end)
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && playlist_index.is_some()
                    && attr(&e, "Key")? == request.track_id =>
            {
                membership_spans.push((playlist_index.unwrap(), start..end))
            }
            Event::End(e) => {
                if e.name().as_ref() == b"TRACK"
                    && let Some(open) = target_open.take()
                {
                    collection_spans.push(open..end);
                }
                if e.name().as_ref() == b"NODE" {
                    playlist_index = None;
                }
                stack.pop();
            }
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|_| Error("invalid element name".into()))?
                    .to_owned();
                stack.push(name);
            }
            Event::Empty(_) | Event::Eof => {}
            _ => {}
        }
        buf.clear();
        if eof {
            break;
        }
    }
    if collection_spans.len() != 1
        || membership_spans.len()
            != actual_memberships
                .iter()
                .map(|(_, ids)| ids.iter().filter(|id| *id == &request.track_id).count())
                .sum::<usize>()
    {
        return Err(Error(
            "target references are missing, duplicated, or structurally ambiguous".into(),
        ));
    }
    let collection_tag = collection_tag_span.ok_or_else(|| Error("COLLECTION missing".into()))?;
    let mut edits: Vec<(std::ops::Range<usize>, Vec<u8>)> = Vec::new();
    edits.extend(collection_spans.into_iter().map(|r| (r, Vec::new())));
    edits.extend(membership_spans.into_iter().map(|(_, r)| (r, Vec::new())));
    for (span, key) in std::iter::once((collection_tag, "Entries".to_owned())).chain(
        playlist_tag_spans
            .into_iter()
            .filter(|(i, _)| actual_memberships.iter().any(|(j, _)| i == j))
            .map(|(_, span)| (span, "Entries".to_owned())),
    ) {
        let current = bytes[span.clone()].to_vec();
        let count = count_attribute_value_range(&current, &key)?;
        let old = std::str::from_utf8(&current[count.clone()])
            .map_err(|_| Error("invalid count".into()))?
            .parse::<usize>()
            .map_err(|_| Error("malformed entry count".into()))?;
        let replacement = old
            .checked_sub(1)
            .ok_or_else(|| Error("entry count underflow".into()))?
            .to_string()
            .into_bytes();
        edits.push((
            span.start + count.start..span.start + count.end,
            replacement,
        ));
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_tracks = before.selected_tracks.clone();
    expected_tracks.retain(|t| t.id != request.track_id);
    let mut expected_playlists = before.selected_playlists.clone();
    for (i, playlist) in expected_playlists.iter_mut().enumerate() {
        if actual_memberships.iter().any(|(j, _)| *j == i) {
            playlist.tracks.retain(|id| id != &request.track_id);
        }
    }
    if after.selected_tracks != expected_tracks || after.selected_playlists != expected_playlists {
        return Err(Error("track deletion semantic verification failed".into()));
    }
    Ok(PreparedTrackDelete { bytes: output })
}

fn count_attribute_value_range(tag: &[u8], key: &str) -> Result<std::ops::Range<usize>, Error> {
    let needle = key.as_bytes();
    let mut found = tag
        .windows(needle.len())
        .enumerate()
        .filter(|(_, w)| *w == needle);
    let (at, _) = found
        .next()
        .ok_or_else(|| Error(format!("{key} attribute missing")))?;
    if found.next().is_some() {
        return Err(Error(format!("duplicate {key} attribute")));
    }
    let mut i = at + needle.len();
    while tag.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    if tag.get(i) != Some(&b'=') {
        return Err(Error(format!("malformed {key} attribute")));
    }
    i += 1;
    while tag.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let quote = *tag
        .get(i)
        .ok_or_else(|| Error(format!("malformed {key} attribute")))?;
    if quote != b'\'' && quote != b'"' {
        return Err(Error(format!("malformed {key} attribute")));
    }
    let begin = i + 1;
    let end = tag[begin..]
        .iter()
        .position(|b| *b == quote)
        .map(|n| begin + n)
        .ok_or_else(|| Error(format!("malformed {key} attribute")))?;
    Ok(begin..end)
}

/// Prepare removal of one guarded playlist reference without deleting its collection track.
pub fn prepare_playlist_member_delete(
    bytes: &[u8],
    request: &PlaylistMemberDeleteRequest,
) -> Result<PreparedPlaylistMemberDelete, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if playlist
        .tracks
        .iter()
        .filter(|id| *id == &request.track_id)
        .count()
        != 1
    {
        return Err(Error(
            "selected playlist membership missing or ambiguous".into(),
        ));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut node_stack = Vec::<Option<usize>>::new();
    let mut playlist_no = 0usize;
    let mut target = false;
    let mut member = None;
    let mut node_tag = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                if ty == "1" {
                    target = playlist_no == request.playlist_index;
                    playlist_no += 1;
                }
                node_stack.push(target.then_some(start));
                if target && ty == "1" {
                    node_tag = Some(start..end);
                }
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    if playlist_no == request.playlist_index {
                        target = true;
                        node_tag = Some(start..end);
                    }
                    playlist_no += 1;
                }
                if target && attr(&e, "Type")? == "1" {
                    node_stack.push(None);
                }
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && target
                    && attr(&e, "Key")? == request.track_id =>
            {
                if member.replace(start..end).is_some() {
                    return Err(Error("membership is ambiguous".into()));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                node_stack.pop();
                if node_stack.is_empty() {
                    target = false;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let member = member.ok_or_else(|| Error("membership location missing".into()))?;
    let tag = node_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    let new = old
        .checked_sub(1)
        .ok_or_else(|| Error("Entries underflow".into()))?
        .to_string();
    let mut edits = vec![
        (
            tag.start + count.start..tag.start + count.end,
            new.into_bytes(),
        ),
        (member, Vec::new()),
    ];
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index]
        .tracks
        .retain(|id| id != &request.track_id);
    if after.selected_tracks != before.selected_tracks || after.selected_playlists != expected {
        return Err(Error("playlist member removal verification failed".into()));
    }
    Ok(PreparedPlaylistMemberDelete { bytes: output })
}

/// Prepare clearing every member from one guarded playlist.
pub fn prepare_playlist_clear(
    bytes: &[u8],
    request: &PlaylistClearRequest,
) -> Result<PreparedPlaylistClear, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut next_playlist = 0usize;
    let mut selected_depth = None;
    let mut selected_tag = None;
    let mut member_spans = Vec::new();
    let mut member_start = None;
    let mut member_ids = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(element) => {
                if element.name().as_ref() == b"NODE" && attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        selected_tag = Some(start..end);
                        selected_depth = Some(stack.len() + 1);
                    }
                    next_playlist += 1;
                }
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|tag| tag == "NODE")
                    && selected_depth == Some(stack.len())
                {
                    member_ids.push(attr(&element, "Key")?);
                    member_start = Some(start);
                }
                stack.push(String::from_utf8_lossy(element.name().as_ref()).into_owned());
            }
            Event::Empty(element) => {
                if element.name().as_ref() == b"NODE" && attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        selected_tag = Some(start..end);
                    }
                    next_playlist += 1;
                } else if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|tag| tag == "NODE")
                    && selected_depth == Some(stack.len())
                {
                    member_ids.push(attr(&element, "Key")?);
                    member_spans.push(start..end);
                }
            }
            Event::End(element) => {
                if element.name().as_ref() == b"TRACK"
                    && let Some(member_start) = member_start.take()
                {
                    member_spans.push(member_start..end);
                }
                if element.name().as_ref() == b"NODE" && selected_depth == Some(stack.len()) {
                    selected_depth = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    let tag = selected_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    if member_ids != request.expected_order || member_spans.len() != request.expected_order.len() {
        return Err(Error("playlist membership structure changed".into()));
    }
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old_count = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    if old_count != request.expected_order.len() {
        return Err(Error("playlist Entries count mismatch".into()));
    }
    let mut edits = vec![(
        tag.start + count.start..tag.start + count.end,
        b"0".to_vec(),
    )];
    edits.extend(member_spans.into_iter().map(|span| (span, Vec::new())));
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index].tracks.clear();
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error("playlist clear verification failed".into()));
    }
    Ok(PreparedPlaylistClear { bytes: output })
}

/// Prepare append of one existing collection track to a guarded playlist.
pub fn prepare_playlist_member_append(
    bytes: &[u8],
    request: &PlaylistMemberAppendRequest,
) -> Result<PreparedPlaylistMemberAppend, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if playlist.tracks.iter().any(|id| id == &request.track_id) {
        return Err(Error("playlist already contains TrackID".into()));
    }
    if before
        .selected_tracks
        .iter()
        .filter(|track| track.id == request.track_id)
        .count()
        != 1
    {
        return Err(Error(
            "target collection track missing or duplicated".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut track_key_value = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|parent| parent == "COLLECTION")
                    && attr(&element, "TrackID")? == request.track_id
                {
                    if track_key_value.is_some() {
                        return Err(Error("target collection track is ambiguous".into()));
                    }
                    let range =
                        attribute_value_range(&element, start, "TrackID", &request.track_id)?;
                    track_key_value = Some(bytes[range].to_vec());
                }
                stack.push(String::from_utf8_lossy(element.name().as_ref()).into_owned());
            }
            Event::Empty(element) => {
                if element.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|parent| parent == "COLLECTION")
                    && attr(&element, "TrackID")? == request.track_id
                {
                    if track_key_value.is_some() {
                        return Err(Error("target collection track is ambiguous".into()));
                    }
                    let range =
                        attribute_value_range(&element, start, "TrackID", &request.track_id)?;
                    track_key_value = Some(bytes[range].to_vec());
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let track_key_value = track_key_value
        .ok_or_else(|| Error("target collection TrackID location missing".into()))?;

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut node_stack = Vec::<Option<usize>>::new();
    let mut next_playlist = 0usize;
    let mut target_tag = None;
    let mut target_close = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(element) if element.name().as_ref() == b"NODE" => {
                let playlist = if attr(&element, "Type")? == "1" {
                    let current = next_playlist;
                    next_playlist += 1;
                    Some(current)
                } else {
                    None
                };
                if playlist == Some(request.playlist_index) {
                    target_tag = Some((start..end, false));
                }
                node_stack.push(playlist);
            }
            Event::Empty(element) if element.name().as_ref() == b"NODE" => {
                if attr(&element, "Type")? == "1" {
                    if next_playlist == request.playlist_index {
                        target_tag = Some((start..end, true));
                    }
                    next_playlist += 1;
                }
            }
            Event::End(element) if element.name().as_ref() == b"NODE" => {
                if node_stack.pop().flatten() == Some(request.playlist_index) {
                    target_close = Some(start);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let (tag, self_closing) = target_tag.ok_or_else(|| Error("playlist node missing".into()))?;
    let count = count_attribute_value_range(&bytes[tag.clone()], "Entries")?;
    let old_count = std::str::from_utf8(&bytes[tag.start + count.start..tag.start + count.end])
        .map_err(|_| Error("invalid Entries".into()))?
        .parse::<usize>()
        .map_err(|_| Error("malformed Entries".into()))?;
    if old_count != request.expected_order.len() {
        return Err(Error("playlist Entries count mismatch".into()));
    }
    let new_count = old_count
        .checked_add(1)
        .ok_or_else(|| Error("Entries overflow".into()))?
        .to_string();
    let mut member = b"<TRACK Key=\"".to_vec();
    member.extend_from_slice(&track_key_value);
    member.extend_from_slice(b"\"/>");

    let mut edits = Vec::new();
    if self_closing {
        let mut replacement = bytes[tag.clone()].to_vec();
        replacement.splice(count, new_count.bytes());
        if !replacement.ends_with(b"/>") {
            return Err(Error("empty playlist NODE is not self-closing".into()));
        }
        replacement.splice(replacement.len() - 2.., b">".iter().copied());
        replacement.extend_from_slice(&member);
        replacement.extend_from_slice(b"</NODE>");
        edits.push((tag, replacement));
    } else {
        let close = target_close.ok_or_else(|| Error("playlist closing NODE missing".into()))?;
        if close < tag.end {
            return Err(Error("playlist NODE span is invalid".into()));
        }
        edits.push((
            tag.start + count.start..tag.start + count.end,
            new_count.into_bytes(),
        ));
        edits.push((close..close, member));
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = bytes.to_vec();
    for (range, replacement) in edits {
        output.splice(range, replacement);
    }
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index]
        .tracks
        .push(request.track_id.clone());
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error("playlist member append verification failed".into()));
    }
    Ok(PreparedPlaylistMemberAppend { bytes: output })
}

/// Prepare insertion at one zero-based position under an exact playlist guard.
pub fn prepare_playlist_member_insert(
    bytes: &[u8],
    request: &PlaylistMemberInsertRequest,
) -> Result<PreparedPlaylistMemberInsert, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error("playlist ordinal is stale".into()))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    if request.position > request.expected_order.len() {
        return Err(Error("playlist insertion position is out of range".into()));
    }
    if request.expected_order.iter().collect::<HashSet<_>>().len() != request.expected_order.len() {
        return Err(Error(
            "playlist insertion does not support duplicate existing memberships".into(),
        ));
    }

    let appended = prepare_playlist_member_append(
        bytes,
        &PlaylistMemberAppendRequest {
            playlist_index: request.playlist_index,
            expected_order: request.expected_order.clone(),
            track_id: request.track_id.clone(),
        },
    )?;
    let mut new_order = request.expected_order.clone();
    new_order.insert(request.position, request.track_id.clone());
    let mut after_append = request.expected_order.clone();
    after_append.push(request.track_id.clone());
    let output = if request.position == request.expected_order.len() {
        appended.bytes
    } else {
        prepare_playlist_reorder(
            &appended.bytes,
            &PlaylistReorderRequest {
                playlist_index: request.playlist_index,
                expected_order: after_append,
                new_order,
            },
        )?
        .bytes
    };
    let after = preflight_export(&output)?;
    let mut expected_playlists = before.selected_playlists.clone();
    expected_playlists[request.playlist_index].tracks = request.expected_order.clone();
    expected_playlists[request.playlist_index]
        .tracks
        .insert(request.position, request.track_id.clone());
    if after.selected_tracks != before.selected_tracks
        || after.selected_playlists != expected_playlists
    {
        return Err(Error(
            "playlist member insertion verification failed".into(),
        ));
    }
    Ok(PreparedPlaylistMemberInsert { bytes: output })
}

/// Prepare a byte-preserving reorder of playlist membership elements.
pub fn prepare_playlist_reorder(
    bytes: &[u8],
    request: &PlaylistReorderRequest,
) -> Result<PreparedPlaylistReorder, Error> {
    let before = preflight_export(bytes)?;
    let playlist = before
        .selected_playlists
        .get(request.playlist_index)
        .ok_or_else(|| Error(format!("no playlist at ordinal {}", request.playlist_index)))?;
    if playlist.tracks != request.expected_order {
        return Err(Error("playlist order changed".into()));
    }
    let existing: HashSet<_> = playlist.tracks.iter().collect();
    let desired: HashSet<_> = request.new_order.iter().collect();
    if request.new_order.len() != playlist.tracks.len()
        || desired.len() != request.new_order.len()
        || desired != existing
    {
        return Err(Error(
            "new order must be a unique permutation of existing TrackIDs".into(),
        ));
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut playlist_no = 0usize;
    let mut node_ordinal = 0usize;
    let mut node_ids: Vec<usize> = Vec::new();
    let mut target_node: Option<usize> = None;
    let mut spans: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let this_node = node_ordinal;
                node_ordinal += 1;
                node_path.push(name);
                node_ids.push(this_node);
                if ty == "1" {
                    if playlist_no == request.playlist_index {
                        target_node = Some(this_node);
                    }
                    playlist_no += 1;
                }
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                if attr(&e, "Type")? == "1" {
                    let this_node = node_ordinal;
                    node_ordinal += 1;
                    if playlist_no == request.playlist_index {
                        target_node = Some(this_node);
                    }
                    playlist_no += 1;
                } else {
                    node_ordinal += 1;
                }
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && target_node.is_some_and(|target| node_ids.last() == Some(&target)) =>
            {
                spans.push((attr(&e, "Key")?, start..end));
            }
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                    node_ids.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let mut ordered = spans.clone();
    ordered.sort_by_key(|(id, _)| {
        request
            .new_order
            .iter()
            .position(|v| v == id)
            .unwrap_or(usize::MAX)
    });
    if spans.iter().map(|(id, _)| id).collect::<Vec<_>>()
        != playlist.tracks.iter().collect::<Vec<_>>()
    {
        return Err(Error(
            "playlist membership locations could not be verified".into(),
        ));
    }
    let mut output = bytes.to_vec();
    // Replace each membership slot with the requested element while retaining all inter-element bytes.
    for (i, (_, slot)) in spans.iter().enumerate().rev() {
        let element = &bytes[ordered[i].1.clone()];
        output.splice(slot.clone(), element.iter().copied());
    }
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    expected[request.playlist_index].tracks = request.new_order.clone();
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("playlist reorder verification failed".into()));
    }
    Ok(PreparedPlaylistReorder { bytes: output })
}

/// Prepare one guarded non-root folder rename, preserving every other source byte.
pub fn prepare_folder_rename(
    bytes: &[u8],
    request: &FolderRenameRequest,
) -> Result<PreparedFolderRename, Error> {
    let before = preflight_export(bytes)?;
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| {
                let n = c as u32;
                n == 9
                    || n == 10
                    || n == 13
                    || (32..=0xD7FF).contains(&n)
                    || (0xE000..=0xFFFD).contains(&n)
                    || (0x10000..=0x10FFFF).contains(&n)
            })
    };
    if !valid(&request.new_name) {
        return Err(Error("invalid folder Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut folders: Vec<(String, Vec<String>)> = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut node_path: Vec<String> = Vec::new();
    let mut siblings: Vec<(Vec<String>, String)> = Vec::new();
    let mut folder_no = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match ev {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect::<Vec<_>>();
                siblings.push((parent.clone(), name.clone()));
                if ty == "0" && !node_path.is_empty() {
                    if folder_no == request.folder_index {
                        if name != request.expected_old_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    folders.push((name.clone(), node_path.iter().skip(1).cloned().collect()));
                    folder_no += 1;
                }
                node_path.push(name);
                stack.push("NODE".into());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let ty = attr(&e, "Type")?;
                let name = attr(&e, "Name")?;
                let parent = node_path.iter().skip(1).cloned().collect::<Vec<_>>();
                siblings.push((parent, name.clone()));
                if ty == "0" && !node_path.is_empty() {
                    if folder_no == request.folder_index {
                        if name != request.expected_old_name {
                            return Err(Error("folder Name changed".into()));
                        }
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    folders.push((name, node_path.iter().skip(1).cloned().collect()));
                    folder_no += 1;
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(e) => {
                if e.name().as_ref() == b"NODE" {
                    node_path.pop();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let old = folders.get(request.folder_index).ok_or_else(|| {
        Error(format!(
            "no non-root folder at ordinal {}",
            request.folder_index
        ))
    })?;
    if old.0 != request.expected_old_name {
        return Err(Error("folder Name changed".into()));
    }
    let range = range.ok_or_else(|| Error("folder Name location missing".into()))?;
    let parent = old.1.clone();
    let matches = siblings
        .iter()
        .filter(|(p, n)| p == &parent && n == &request.new_name)
        .count();
    if matches > usize::from(request.expected_old_name == request.new_name) {
        return Err(Error("duplicate sibling playlist/folder Name".into()));
    }
    let escaped = request
        .new_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_playlists.clone();
    for playlist in &mut expected {
        if playlist.path.len() > parent.len()
            && playlist.path[..parent.len()] == parent[..]
            && playlist.path[parent.len()] == request.expected_old_name
        {
            playlist.path[parent.len()] = request.new_name.clone();
        }
    }
    if after.selected_playlists != expected || after.selected_tracks != before.selected_tracks {
        return Err(Error("folder rename verification failed".into()));
    }
    Ok(PreparedFolderRename {
        bytes: output,
        change: FolderRenameChange {
            folder_index: request.folder_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

/// Validate and return unchanged bytes for the frozen synthetic A1 no-edit subset.
pub fn export_xml(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    preflight_export(bytes)?;
    Ok(bytes.to_vec())
}

fn track_attribute_range(
    bytes: &[u8],
    id: &str,
    attribute: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut result = None;
    loop {
        let start = reader.buffer_position() as usize;
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|p| p == "COLLECTION") =>
            {
                if attr(&e, "TrackID")? == id {
                    result = Some(attribute_value_range(&e, start, attribute, expected)?);
                }
                stack.push("TRACK".into());
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|p| p == "COLLECTION") =>
            {
                if attr(&e, "TrackID")? == id {
                    result = Some(attribute_value_range(&e, start, attribute, expected)?);
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    result.ok_or_else(|| Error(format!("Track {attribute} location missing")))
}

/// Prepare one exact-old-name guarded TrackID edit with byte-preserving XML output.
pub fn prepare_track_name_edit(
    bytes: &[u8],
    request: &TrackNameEditRequest,
) -> Result<PreparedTrackNameEdit, Error> {
    let before = preflight_export(bytes)?;
    let track = before
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    if track.name != request.expected_old_name {
        return Err(Error("track Name changed".into()));
    }
    if !request.new_name.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Name contains invalid XML 1.0 text".into()));
    }
    let range =
        track_attribute_range(bytes, &request.track_id, "Name", &request.expected_old_name)?;
    let escaped = request
        .new_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .name = request.new_name.clone();
    if after.selected_tracks != expected {
        return Err(Error("track Name edit verification failed".into()));
    }
    Ok(PreparedTrackNameEdit {
        bytes: output,
        change: TrackNameEditChange {
            track_id: request.track_id.clone(),
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Comments edit.
pub fn prepare_track_comments_edit(
    bytes: &[u8],
    request: &TrackCommentsEditRequest,
) -> Result<PreparedTrackCommentsEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_comments = track
        .comments
        .as_deref()
        .ok_or_else(|| Error("track Comments missing".into()))?;
    if old_comments != request.expected_old_comments {
        return Err(Error("track Comments changed".into()));
    }
    if !request.new_comments.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Comments contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Comments",
        &request.expected_old_comments,
    )?;
    let escaped = request
        .new_comments
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .comments = Some(request.new_comments.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Comments edit verification failed".into()));
    }
    Ok(PreparedTrackCommentsEdit {
        bytes: output,
        change: TrackCommentsEditChange {
            track_id: request.track_id.clone(),
            old_comments: old_comments.to_owned(),
            new_comments: request.new_comments.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Album edit.
pub fn prepare_track_album_edit(
    bytes: &[u8],
    request: &TrackAlbumEditRequest,
) -> Result<PreparedTrackAlbumEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_album = track
        .album
        .as_deref()
        .ok_or_else(|| Error("track Album missing".into()))?;
    if old_album != request.expected_old_album {
        return Err(Error("track Album changed".into()));
    }
    if !request.new_album.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Album contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Album",
        &request.expected_old_album,
    )?;
    let escaped = request
        .new_album
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .album = Some(request.new_album.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Album edit verification failed".into()));
    }
    Ok(PreparedTrackAlbumEdit {
        bytes: output,
        change: TrackAlbumEditChange {
            track_id: request.track_id.clone(),
            old_album: old_album.to_owned(),
            new_album: request.new_album.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Genre edit.
pub fn prepare_track_genre_edit(
    bytes: &[u8],
    request: &TrackGenreEditRequest,
) -> Result<PreparedTrackGenreEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_genre = track
        .genre
        .as_deref()
        .ok_or_else(|| Error("track Genre missing".into()))?;
    if old_genre != request.expected_old_genre {
        return Err(Error("track Genre changed".into()));
    }
    if !request.new_genre.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Genre contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Genre",
        &request.expected_old_genre,
    )?;
    let escaped = request
        .new_genre
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .genre = Some(request.new_genre.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Genre edit verification failed".into()));
    }
    Ok(PreparedTrackGenreEdit {
        bytes: output,
        change: TrackGenreEditChange {
            track_id: request.track_id.clone(),
            old_genre: old_genre.to_owned(),
            new_genre: request.new_genre.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Label edit.
pub fn prepare_track_label_edit(
    bytes: &[u8],
    request: &TrackLabelEditRequest,
) -> Result<PreparedTrackLabelEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_label = track
        .label
        .as_deref()
        .ok_or_else(|| Error("track Label missing".into()))?;
    if old_label != request.expected_old_label {
        return Err(Error("track Label changed".into()));
    }
    if !request.new_label.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Label contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Label",
        &request.expected_old_label,
    )?;
    let escaped = request
        .new_label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .label = Some(request.new_label.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Label edit verification failed".into()));
    }
    Ok(PreparedTrackLabelEdit {
        bytes: output,
        change: TrackLabelEditChange {
            track_id: request.track_id.clone(),
            old_label: old_label.to_owned(),
            new_label: request.new_label.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Composer edit.
pub fn prepare_track_composer_edit(
    bytes: &[u8],
    request: &TrackComposerEditRequest,
) -> Result<PreparedTrackComposerEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_composer = track
        .composer
        .as_deref()
        .ok_or_else(|| Error("track Composer missing".into()))?;
    if old_composer != request.expected_old_composer {
        return Err(Error("track Composer changed".into()));
    }
    if !request.new_composer.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Composer contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Composer",
        &request.expected_old_composer,
    )?;
    let escaped = request
        .new_composer
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .composer = Some(request.new_composer.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Composer edit verification failed".into()));
    }
    Ok(PreparedTrackComposerEdit {
        bytes: output,
        change: TrackComposerEditChange {
            track_id: request.track_id.clone(),
            old_composer: old_composer.to_owned(),
            new_composer: request.new_composer.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Grouping edit.
pub fn prepare_track_grouping_edit(
    bytes: &[u8],
    request: &TrackGroupingEditRequest,
) -> Result<PreparedTrackGroupingEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_grouping = track
        .grouping
        .as_deref()
        .ok_or_else(|| Error("track Grouping missing".into()))?;
    if old_grouping != request.expected_old_grouping {
        return Err(Error("track Grouping changed".into()));
    }
    if !request.new_grouping.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Grouping contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Grouping",
        &request.expected_old_grouping,
    )?;
    let escaped = request
        .new_grouping
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .grouping = Some(request.new_grouping.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Grouping edit verification failed".into()));
    }
    Ok(PreparedTrackGroupingEdit {
        bytes: output,
        change: TrackGroupingEditChange {
            track_id: request.track_id.clone(),
            old_grouping: old_grouping.to_owned(),
            new_grouping: request.new_grouping.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Remixer edit.
pub fn prepare_track_remixer_edit(
    bytes: &[u8],
    request: &TrackRemixerEditRequest,
) -> Result<PreparedTrackRemixerEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_remixer = track
        .remixer
        .as_deref()
        .ok_or_else(|| Error("track Remixer missing".into()))?;
    if old_remixer != request.expected_old_remixer {
        return Err(Error("track Remixer changed".into()));
    }
    if !request.new_remixer.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Remixer contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Remixer",
        &request.expected_old_remixer,
    )?;
    let escaped = request
        .new_remixer
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .remixer = Some(request.new_remixer.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Remixer edit verification failed".into()));
    }
    Ok(PreparedTrackRemixerEdit {
        bytes: output,
        change: TrackRemixerEditChange {
            track_id: request.track_id.clone(),
            old_remixer: old_remixer.to_owned(),
            new_remixer: request.new_remixer.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Mix edit.
pub fn prepare_track_mix_edit(
    bytes: &[u8],
    request: &TrackMixEditRequest,
) -> Result<PreparedTrackMixEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_mix = track
        .mix
        .as_deref()
        .ok_or_else(|| Error("track Mix missing".into()))?;
    if old_mix != request.expected_old_mix {
        return Err(Error("track Mix changed".into()));
    }
    if !request.new_mix.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Mix contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(bytes, &request.track_id, "Mix", &request.expected_old_mix)?;
    let escaped = request
        .new_mix
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .mix = Some(request.new_mix.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Mix edit verification failed".into()));
    }
    Ok(PreparedTrackMixEdit {
        bytes: output,
        change: TrackMixEditChange {
            track_id: request.track_id.clone(),
            old_mix: old_mix.to_owned(),
            new_mix: request.new_mix.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Tonality edit.
pub fn prepare_track_tonality_edit(
    bytes: &[u8],
    request: &TrackTonalityEditRequest,
) -> Result<PreparedTrackTonalityEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_tonality = track
        .tonality
        .as_deref()
        .ok_or_else(|| Error("track Tonality missing".into()))?;
    if old_tonality != request.expected_old_tonality {
        return Err(Error("track Tonality changed".into()));
    }
    if !request.new_tonality.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Tonality contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Tonality",
        &request.expected_old_tonality,
    )?;
    let escaped = request
        .new_tonality
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .tonality = Some(request.new_tonality.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Tonality edit verification failed".into()));
    }
    Ok(PreparedTrackTonalityEdit {
        bytes: output,
        change: TrackTonalityEditChange {
            track_id: request.track_id.clone(),
            old_tonality: old_tonality.to_owned(),
            new_tonality: request.new_tonality.clone(),
        },
    })
}

/// Prepare one exact-old-value guarded Track Artist edit.
pub fn prepare_track_artist_edit(
    bytes: &[u8],
    request: &TrackArtistEditRequest,
) -> Result<PreparedTrackArtistEdit, Error> {
    let before = preflight_export(bytes)?;
    let matches: Vec<_> = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id)
        .collect();
    let track = match matches.as_slice() {
        [] => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        [track] => *track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old_artist = track
        .artist
        .as_deref()
        .ok_or_else(|| Error("track Artist missing".into()))?;
    if old_artist != request.expected_old_artist {
        return Err(Error("track Artist changed".into()));
    }
    if !request.new_artist.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    ) {
        return Err(Error("new Artist contains invalid XML 1.0 text".into()));
    }
    let range = track_attribute_range(
        bytes,
        &request.track_id,
        "Artist",
        &request.expected_old_artist,
    )?;
    let escaped = request
        .new_artist
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap()
        .artist = Some(request.new_artist.clone());
    if after.selected_tracks != expected {
        return Err(Error("track Artist edit verification failed".into()));
    }
    Ok(PreparedTrackArtistEdit {
        bytes: output,
        change: TrackArtistEditChange {
            track_id: request.track_id.clone(),
            old_artist: old_artist.to_owned(),
            new_artist: request.new_artist.clone(),
        },
    })
}

fn tempo_bpm_range(
    bytes: &[u8],
    request: &TempoEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut tempo_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    tempo_index = 0;
                } else if name == "TEMPO" && track_id.as_deref() == Some(&request.track_id) {
                    if tempo_index == request.tempo_index {
                        value_range = Some(bpm_attribute_range(
                            &element,
                            event_start,
                            &request.expected_old_bpm,
                        )?);
                    }
                    tempo_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TEMPO" && track_id.as_deref() == Some(&request.track_id) {
                    if tempo_index == request.tempo_index {
                        value_range = Some(bpm_attribute_range(
                            &element,
                            event_start,
                            &request.expected_old_bpm,
                        )?);
                    }
                    tempo_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })
}

fn tempo_attribute_range(
    bytes: &[u8],
    expected_track_id: &str,
    expected_tempo_index: usize,
    key: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_track_id: Option<String> = None;
    let mut current_tempo_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    current_track_id = Some(attr(&element, "TrackID")?);
                    current_tempo_index = 0;
                } else if name == "TEMPO" && current_track_id.as_deref() == Some(expected_track_id)
                {
                    if current_tempo_index == expected_tempo_index {
                        value_range = Some(tempo_attribute_value_range(
                            &element,
                            event_start,
                            key,
                            expected,
                        )?);
                    }
                    current_tempo_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TEMPO" && current_track_id.as_deref() == Some(expected_track_id) {
                    if current_tempo_index == expected_tempo_index {
                        value_range = Some(tempo_attribute_value_range(
                            &element,
                            event_start,
                            key,
                            expected,
                        )?);
                    }
                    current_tempo_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "TRACK") {
                    current_track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no tempo point {expected_tempo_index} for TrackID {expected_track_id}"
        ))
    })
}

fn tempo_attribute_value_range(
    element: &quick_xml::events::BytesStart<'_>,
    event_start: usize,
    key: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error(error.to_string()))?;
        if attribute.key.as_ref() == key.as_bytes() {
            let actual = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, element.decoder())
                .map_err(|error| Error(error.to_string()))?;
            if actual != expected {
                return Err(Error(format!(
                    "tempo {key} changed: expected {expected}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error(format!("TEMPO missing {key} attribute")))
}

fn tempo_start_range(
    bytes: &[u8],
    request: &TempoStartEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Inizio",
        &request.expected_old_start_seconds,
    )
}

fn tempo_beat_range(
    bytes: &[u8],
    request: &TempoBeatEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Battito",
        &request.expected_old_beat,
    )
}

fn position_mark_start_range(
    bytes: &[u8],
    request: &PositionMarkEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut mark_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    mark_index = 0;
                } else if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id)
                {
                    if mark_index == request.mark_index {
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Start",
                            &request.expected_old_start_seconds,
                        )?);
                    }
                    mark_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id) {
                    if mark_index == request.mark_index {
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Start",
                            &request.expected_old_start_seconds,
                        )?);
                    }
                    mark_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|open| open == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })
}

fn position_mark_end_range(
    bytes: &[u8],
    request: &PositionMarkEndEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut index = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    index = 0;
                } else if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id)
                {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &element,
                            start,
                            "End",
                            &request.expected_old_end_seconds,
                        )?);
                    }
                    index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id) {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &element,
                            start,
                            "End",
                            &request.expected_old_end_seconds,
                        )?);
                    }
                    index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|open| open == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    range.ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })
}

fn attribute_value_range(
    element: &quick_xml::events::BytesStart<'_>,
    event_start: usize,
    key: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error(error.to_string()))?;
        if attribute.key.as_ref() == key.as_bytes() {
            let actual = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, element.decoder())
                .map_err(|error| Error(error.to_string()))?;
            if actual != expected {
                return Err(Error(format!(
                    "attribute {key} changed: expected {expected}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error(format!("element missing {key} attribute")))
}

/// Prepare one position mark Start edit while preserving every other source byte.
pub fn prepare_position_mark_edit(
    bytes: &[u8],
    request: &PositionMarkEditRequest,
) -> Result<PreparedPositionMarkEdit, Error> {
    let source = preflight_export(bytes)?;
    let source_track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = source_track.marks.get(request.mark_index).ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })?;
    if mark.start_seconds != request.expected_old_start_seconds {
        return Err(Error(format!(
            "position mark Start changed: expected {}, found {}",
            request.expected_old_start_seconds, mark.start_seconds
        )));
    }
    validate_decimal("new position mark Start", &request.new_start_seconds, false)?;
    if let Some(end) = &mark.end_seconds {
        let new_start = request
            .new_start_seconds
            .parse::<f64>()
            .map_err(|_| Error("invalid new position mark Start".into()))?;
        let end = end
            .parse::<f64>()
            .map_err(|_| Error("invalid position mark End".into()))?;
        if new_start >= end {
            return Err(Error(
                "edited position mark Start must be before End".into(),
            ));
        }
    }

    let range = position_mark_start_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_start_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = source_track.clone();
    expected_track.marks[request.mark_index].start_seconds = request.new_start_seconds.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited position mark verification failed".into()));
    }
    Ok(PreparedPositionMarkEdit {
        bytes: output,
        change: PositionMarkEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_start_seconds: request.expected_old_start_seconds.clone(),
            new_start_seconds: request.new_start_seconds.clone(),
        },
    })
}

pub fn prepare_position_mark_end_edit(
    bytes: &[u8],
    request: &PositionMarkEndEditRequest,
) -> Result<PreparedPositionMarkEndEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = track
        .marks
        .get(request.mark_index)
        .ok_or_else(|| Error("position mark missing".into()))?;
    let old = mark
        .end_seconds
        .as_ref()
        .ok_or_else(|| Error("POSITION_MARK missing End attribute".into()))?;
    if old != &request.expected_old_end_seconds {
        return Err(Error(format!(
            "position mark End changed: expected {}, found {old}",
            request.expected_old_end_seconds
        )));
    }
    validate_decimal("new position mark End", &request.new_end_seconds, false)?;
    let new_end = request
        .new_end_seconds
        .parse::<f64>()
        .map_err(|_| Error("invalid new position mark End".into()))?;
    let start = mark
        .start_seconds
        .parse::<f64>()
        .map_err(|_| Error("invalid position mark Start".into()))?;
    if new_end <= start {
        return Err(Error("edited position mark End must be after Start".into()));
    }
    let range = position_mark_end_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_end_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected = track.clone();
    expected.marks[request.mark_index].end_seconds = Some(request.new_end_seconds.clone());
    let actual = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual != &expected {
        return Err(Error("edited position mark verification failed".into()));
    }
    Ok(PreparedPositionMarkEndEdit {
        bytes: output,
        change: PositionMarkEndEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_end_seconds: request.expected_old_end_seconds.clone(),
            new_end_seconds: request.new_end_seconds.clone(),
        },
    })
}

pub fn prepare_position_mark_name_edit(
    bytes: &[u8],
    request: &PositionMarkNameEditRequest,
) -> Result<PreparedPositionMarkNameEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = track
        .marks
        .get(request.mark_index)
        .ok_or_else(|| Error("position mark missing".into()))?;
    if mark.name != request.expected_old_name {
        return Err(Error("position mark Name changed".into()));
    }
    if request.new_name.is_empty()
        || request.new_name.chars().any(|character| {
            let code = character as u32;
            !(code == 0x9
                || code == 0xA
                || code == 0xD
                || (0x20..=0xD7FF).contains(&code)
                || (0xE000..=0xFFFD).contains(&code)
                || (0x10000..=0x10FFFF).contains(&code))
        })
    {
        return Err(Error("invalid position mark Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut tid: Option<String> = None;
    let mut index = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(e.to_string()))?
        {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if tag == "TRACK" && stack.last().is_some_and(|x| x == "COLLECTION") {
                    tid = Some(attr(&e, "TrackID")?);
                    index = 0;
                } else if tag == "POSITION_MARK" && tid.as_deref() == Some(&request.track_id) {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    index += 1;
                }
                stack.push(tag);
            }
            Event::Empty(e) => {
                if e.name().as_ref() == b"POSITION_MARK"
                    && tid.as_deref() == Some(&request.track_id)
                {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    index += 1;
                }
            }
            Event::End(e) => {
                if e.name().as_ref() == b"TRACK" && stack.last().is_some_and(|x| x == "TRACK") {
                    tid = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let range = range.ok_or_else(|| Error("position mark missing".into()))?;
    let escaped = request
        .new_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let verified = preflight_export(&output)?;
    let actual = verified
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| Error("edited track missing".into()))?;
    let mut expected = track.clone();
    expected.marks[request.mark_index].name = request.new_name.clone();
    if actual != &expected {
        return Err(Error("edited Name verification failed".into()));
    }
    Ok(PreparedPositionMarkNameEdit {
        bytes: output,
        change: PositionMarkNameEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

fn bpm_attribute_range(
    element: &quick_xml::events::BytesStart<'_>,
    event_start: usize,
    expected_old_bpm: &str,
) -> Result<std::ops::Range<usize>, Error> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error(error.to_string()))?;
        if attribute.key.as_ref() == b"Bpm" {
            let actual = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, element.decoder())
                .map_err(|error| Error(error.to_string()))?;
            if actual != expected_old_bpm {
                return Err(Error(format!(
                    "tempo BPM changed: expected {expected_old_bpm}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error("TEMPO missing Bpm attribute".into()))
}

/// Prepare a single BPM edit while preserving every other source byte.
pub fn prepare_tempo_edit(
    bytes: &[u8],
    request: &TempoEditRequest,
) -> Result<PreparedTempoEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.bpm != request.expected_old_bpm {
        return Err(Error(format!(
            "tempo BPM changed: expected {}, found {}",
            request.expected_old_bpm, tempo.bpm
        )));
    }
    validate_decimal("new BPM", &request.new_bpm, true)?;
    let range = tempo_bpm_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_bpm.bytes());
    let verified = preflight_export(&output)?;
    let actual = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .and_then(|track| track.tempos.get(request.tempo_index))
        .ok_or_else(|| Error("edited tempo point missing after validation".into()))?;
    if actual.bpm != request.new_bpm {
        return Err(Error(format!(
            "edited BPM verification failed: expected {}, found {}",
            request.new_bpm, actual.bpm
        )));
    }
    Ok(PreparedTempoEdit {
        bytes: output,
        change: TempoEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_bpm: request.expected_old_bpm.clone(),
            new_bpm: request.new_bpm.clone(),
        },
    })
}

/// Prepare a single tempo-point Inizio edit while preserving every other source byte.
pub fn prepare_tempo_start_edit(
    bytes: &[u8],
    request: &TempoStartEditRequest,
) -> Result<PreparedTempoStartEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.start_seconds != request.expected_old_start_seconds {
        return Err(Error(format!(
            "tempo Inizio changed: expected {}, found {}",
            request.expected_old_start_seconds, tempo.start_seconds
        )));
    }
    validate_decimal("new tempo Inizio", &request.new_start_seconds, false)?;
    let range = tempo_start_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_start_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].start_seconds = request.new_start_seconds.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Inizio verification failed".into()));
    }
    Ok(PreparedTempoStartEdit {
        bytes: output,
        change: TempoStartEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_start_seconds: request.expected_old_start_seconds.clone(),
            new_start_seconds: request.new_start_seconds.clone(),
        },
    })
}

/// Prepare a single tempo-point Battito edit while preserving every other source byte.
pub fn prepare_tempo_beat_edit(
    bytes: &[u8],
    request: &TempoBeatEditRequest,
) -> Result<PreparedTempoBeatEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.beat != request.expected_old_beat {
        return Err(Error(format!(
            "tempo Battito changed: expected {}, found {}",
            request.expected_old_beat, tempo.beat
        )));
    }
    request
        .new_beat
        .parse::<u32>()
        .map_err(|_| Error(format!("invalid tempo Battito {}", request.new_beat)))?;
    let range = tempo_beat_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_beat.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].beat = request.new_beat.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Battito verification failed".into()));
    }
    Ok(PreparedTempoBeatEdit {
        bytes: output,
        change: TempoBeatEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_beat: request.expected_old_beat.clone(),
            new_beat: request.new_beat.clone(),
        },
    })
}

/// Prepare a single raw tempo-point Metro edit while preserving every other source byte.
pub fn prepare_tempo_meter_edit(
    bytes: &[u8],
    request: &TempoMeterEditRequest,
) -> Result<PreparedTempoMeterEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.meter != request.expected_old_meter {
        return Err(Error(format!(
            "tempo Metro changed: expected {}, found {}",
            request.expected_old_meter, tempo.meter
        )));
    }
    let valid_xml_text = |value: &str| {
        !value.is_empty()
            && value.chars().all(|c| {
                matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
            })
    };
    if !valid_xml_text(&request.new_meter) {
        return Err(Error("invalid tempo Metro XML text".into()));
    }
    let range = tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Metro",
        &request.expected_old_meter,
    )?;
    let escaped = request
        .new_meter
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].meter = request.new_meter.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Metro verification failed".into()));
    }
    Ok(PreparedTempoMeterEdit {
        bytes: output,
        change: TempoMeterEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_meter: request.expected_old_meter.clone(),
            new_meter: request.new_meter.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_position_mark_end_optionality_survives_both_track_parser_paths_and_export() {
        let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace(
                "<TRACK TrackID=\"100\" Name=\"A\" Location=\"file://localhost/music/a.flac\"/>",
                "<TRACK TrackID=\"100\" Name=\"A\" Location=\"file://localhost/music/a.flac\">\n      <POSITION_MARK Name=\"zero\" Type=\"0\" Start=\"0\" Num=\"0\" End=\"0\"/>\n    </TRACK>",
            )
            .replace(
                "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\"/>",
                "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\">\n      <POSITION_MARK Name=\"absent\" Type=\"0\" Start=\"0\" Num=\"0\"/>\n    </TRACK>",
            );
        let bytes = source.as_bytes();
        let parsed = parse(bytes).unwrap();
        assert_eq!(parsed.tracks[0].marks[0].end_seconds.as_deref(), Some("0"));
        assert_eq!(parsed.tracks[1].marks[0].end_seconds, None);
        let plan = preflight_export(bytes).unwrap();
        assert_eq!(
            plan.selected_tracks[0].marks[0].end_seconds.as_deref(),
            Some("0")
        );
        assert_eq!(plan.selected_tracks[1].marks[0].end_seconds, None);
        assert_eq!(export_xml(bytes).unwrap(), bytes);
    }

    #[test]
    fn raw_optional_dates_and_play_count_preserve_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" DateAdded=\"2024-02-03\" DateModified=\"2024-02-04\" LastPlayed=\"2024-02-05\" PlayCount=\"not-a-number\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" DateAdded=\"\" DateModified=\"\" LastPlayed=\"\" PlayCount=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(preview.tracks[0].date_added.as_deref(), Some("2024-02-03"));
        assert_eq!(preview.tracks[1].date_added.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].date_added, None);
        assert_eq!(
            preview.tracks[0].date_modified.as_deref(),
            Some("2024-02-04")
        );
        assert_eq!(preview.tracks[1].date_modified.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].date_modified, None);
        assert_eq!(preview.tracks[0].last_played.as_deref(), Some("2024-02-05"));
        assert_eq!(preview.tracks[1].last_played.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].last_played, None);
        assert_eq!(
            preview.tracks[0].play_count.as_deref(),
            Some("not-a-number")
        );
        assert_eq!(preview.tracks[1].play_count.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].play_count, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(
            plan.selected_tracks[0].date_added.as_deref(),
            Some("2024-02-03")
        );
        assert_eq!(plan.selected_tracks[1].date_added.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].date_added, None);
        assert_eq!(
            plan.selected_tracks[0].date_modified.as_deref(),
            Some("2024-02-04")
        );
        assert_eq!(plan.selected_tracks[1].date_modified.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].date_modified, None);
        assert_eq!(
            plan.selected_tracks[0].last_played.as_deref(),
            Some("2024-02-05")
        );
        assert_eq!(plan.selected_tracks[1].last_played.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].last_played, None);
        assert_eq!(
            plan.selected_tracks[0].play_count.as_deref(),
            Some("not-a-number")
        );
        assert_eq!(plan.selected_tracks[1].play_count.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].play_count, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn raw_optional_rating_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Rating=\"-1\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Rating=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(preview.tracks[0].rating.as_deref(), Some("-1"));
        assert_eq!(preview.tracks[1].rating.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].rating, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(plan.selected_tracks[0].rating.as_deref(), Some("-1"));
        assert_eq!(plan.selected_tracks[1].rating.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].rating, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn colour_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Colour=\"#12aBcF\" AverageBpm=\"126.12500000000001\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Colour=\"\" AverageBpm=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(preview.tracks[0].colour.as_deref(), Some("#12aBcF"));
        assert_eq!(preview.tracks[1].colour.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].colour, None);
        assert_eq!(
            preview.tracks[0].average_bpm.as_deref(),
            Some("126.12500000000001")
        );
        assert_eq!(preview.tracks[1].average_bpm.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].average_bpm, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(plan.selected_tracks[0].colour.as_deref(), Some("#12aBcF"));
        assert_eq!(plan.selected_tracks[1].colour.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].colour, None);
        assert_eq!(
            plan.selected_tracks[0].average_bpm.as_deref(),
            Some("126.12500000000001")
        );
        assert_eq!(plan.selected_tracks[1].average_bpm.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].average_bpm, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn kind_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Kind=\"AIFF\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Kind=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(preview.tracks[0].kind.as_deref(), Some("AIFF"));
        assert_eq!(preview.tracks[1].kind.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].kind, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(plan.selected_tracks[0].kind.as_deref(), Some("AIFF"));
        assert_eq!(plan.selected_tracks[1].kind.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].kind, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn year_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Year=\"2024\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Year=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(preview.tracks[0].year.as_deref(), Some("2024"));
        assert_eq!(preview.tracks[1].year.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].year, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(plan.selected_tracks[0].year.as_deref(), Some("2024"));
        assert_eq!(plan.selected_tracks[1].year.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].year, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn size_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Size=\"9223372036854775807\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Size=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(
            preview.tracks[0].size.as_deref(),
            Some("9223372036854775807")
        );
        assert_eq!(preview.tracks[1].size.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].size, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(
            plan.selected_tracks[0].size.as_deref(),
            Some("9223372036854775807")
        );
        assert_eq!(plan.selected_tracks[1].size.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].size, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn track_number_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" TrackNumber=\"-2147483648\" TotalTime=\"3.141592653589793\" BitRate=\"-2147483648\" SampleRate=\"48000.0\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" TrackNumber=\"\" TotalTime=\"\" BitRate=\"\" SampleRate=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(
            preview.tracks[0].track_number.as_deref(),
            Some("-2147483648")
        );
        assert_eq!(preview.tracks[1].track_number.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].track_number, None);
        assert_eq!(
            preview.tracks[0].total_time.as_deref(),
            Some("3.141592653589793")
        );
        assert_eq!(preview.tracks[1].total_time.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].total_time, None);
        assert_eq!(preview.tracks[0].bit_rate.as_deref(), Some("-2147483648"));
        assert_eq!(preview.tracks[1].bit_rate.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].bit_rate, None);
        assert_eq!(preview.tracks[0].sample_rate.as_deref(), Some("48000.0"));
        assert_eq!(preview.tracks[1].sample_rate.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].sample_rate, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(
            plan.selected_tracks[0].track_number.as_deref(),
            Some("-2147483648")
        );
        assert_eq!(plan.selected_tracks[1].track_number.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].track_number, None);
        assert_eq!(
            plan.selected_tracks[0].total_time.as_deref(),
            Some("3.141592653589793")
        );
        assert_eq!(plan.selected_tracks[1].total_time.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].total_time, None);
        assert_eq!(
            plan.selected_tracks[0].bit_rate.as_deref(),
            Some("-2147483648")
        );
        assert_eq!(plan.selected_tracks[1].bit_rate.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].bit_rate, None);
        assert_eq!(
            plan.selected_tracks[0].sample_rate.as_deref(),
            Some("48000.0")
        );
        assert_eq!(plan.selected_tracks[1].sample_rate.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].sample_rate, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn disc_number_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" DiscNumber=\"-2147483648\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" DiscNumber=\"\"",
            );
        let source = source.as_bytes();
        let preview = parse(source).unwrap();
        assert_eq!(
            preview.tracks[0].disc_number.as_deref(),
            Some("-2147483648")
        );
        assert_eq!(preview.tracks[1].disc_number.as_deref(), Some(""));
        assert_eq!(preview.tracks[2].disc_number, None);
        let plan = preflight_export(source).unwrap();
        assert_eq!(
            plan.selected_tracks[0].disc_number.as_deref(),
            Some("-2147483648")
        );
        assert_eq!(plan.selected_tracks[1].disc_number.as_deref(), Some(""));
        assert_eq!(plan.selected_tracks[2].disc_number, None);
        assert_eq!(export_xml(source).unwrap(), source);
    }

    #[test]
    fn guarded_playlist_member_delete_keeps_collection_and_other_playlist() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = PlaylistMemberDeleteRequest {
            playlist_index: 0,
            expected_order: vec!["100".into(), "200".into(), "300".into()],
            track_id: "200".into(),
        };
        let result = prepare_playlist_member_delete(source, &request).unwrap();
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(
            after
                .selected_tracks
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["100", "200", "300", "400"]
        );
        assert_eq!(after.selected_playlists[0].tracks, ["100", "300"]);
        assert_eq!(after.selected_playlists[1].tracks, ["200", "400"]);
        assert!(
            result
                .bytes
                .windows(b"Name=\"House\" Entries=\"2\"".len())
                .any(|w| w == b"Name=\"House\" Entries=\"2\"")
        );
        assert!(
            result
                .bytes
                .windows(b"Name=\"Techno\" Entries=\"2\"".len())
                .any(|w| w == b"Name=\"Techno\" Entries=\"2\"")
        );
        assert!(
            prepare_playlist_member_delete(
                source,
                &PlaylistMemberDeleteRequest {
                    expected_order: vec!["100".into(), "300".into()],
                    ..request
                }
            )
            .is_err()
        );
    }

    #[test]
    fn guarded_playlist_member_append_preserves_collection_and_other_playlist() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = PlaylistMemberAppendRequest {
            playlist_index: 1,
            expected_order: vec!["200".into(), "400".into()],
            track_id: "100".into(),
        };
        let result = prepare_playlist_member_append(source, &request).unwrap();
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(
            after.selected_tracks,
            preflight_export(source).unwrap().selected_tracks
        );
        assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(after.selected_playlists[1].tracks, ["200", "400", "100"]);
        assert!(
            result
                .bytes
                .windows(b"Name=\"Techno\" Entries=\"3\"".len())
                .any(|w| w == b"Name=\"Techno\" Entries=\"3\"")
        );
        assert_eq!(
            source,
            include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
        );
        for invalid in [
            PlaylistMemberAppendRequest {
                expected_order: vec!["200".into()],
                ..request.clone()
            },
            PlaylistMemberAppendRequest {
                track_id: "200".into(),
                ..request.clone()
            },
            PlaylistMemberAppendRequest {
                track_id: "999".into(),
                ..request.clone()
            },
        ] {
            assert!(prepare_playlist_member_append(source, &invalid).is_err());
        }
    }

    #[test]
    fn guarded_playlist_member_append_expands_an_empty_self_closing_playlist() {
        let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace(
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
            )
            .into_bytes();
        let request = PlaylistMemberAppendRequest {
            playlist_index: 1,
            expected_order: vec![],
            track_id: "100".into(),
        };
        let result = prepare_playlist_member_append(&source, &request).unwrap();
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(
            after.selected_tracks,
            preflight_export(&source).unwrap().selected_tracks
        );
        assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(after.selected_playlists[1].tracks, ["100"]);
        assert!(
            result
                .bytes
                .windows(b"Name=\"Techno\" Entries=\"1\"".len())
                .any(|w| w == b"Name=\"Techno\" Entries=\"1\"")
        );
        assert!(
            result
                .bytes
                .windows(b"<TRACK Key=\"100\"/></NODE>".len())
                .any(|w| w == b"<TRACK Key=\"100\"/></NODE>")
        );
        assert_eq!(
            source,
            String::from_utf8(fixture.to_vec())
                .unwrap()
                .replace(
                    "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                    "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
                )
                .as_bytes()
        );
    }

    #[test]
    fn guarded_playlist_clear_removes_memberships_but_retains_collection_and_other_playlist() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = PlaylistClearRequest {
            playlist_index: 0,
            expected_order: vec!["100".into(), "200".into(), "300".into()],
        };
        let result = prepare_playlist_clear(source, &request).unwrap();
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(
            after.selected_tracks,
            preflight_export(source).unwrap().selected_tracks
        );
        assert!(after.selected_playlists[0].tracks.is_empty());
        assert_eq!(after.selected_playlists[1].tracks, ["200", "400"]);
        assert!(
            result
                .bytes
                .windows(b"Name=\"House\" Entries=\"0\"".len())
                .any(|w| w == b"Name=\"House\" Entries=\"0\"")
        );
        assert_eq!(
            source,
            include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
        );
        let stale = PlaylistClearRequest {
            expected_order: vec!["100".into()],
            ..request
        };
        assert!(prepare_playlist_clear(source, &stale).is_err());
        let malformed = String::from_utf8(source.to_vec()).unwrap().replace(
            "Name=\"House\" Entries=\"3\"",
            "Name=\"House\" Entries=\"2\"",
        );
        assert!(
            prepare_playlist_clear(
                malformed.as_bytes(),
                &PlaylistClearRequest {
                    playlist_index: 0,
                    expected_order: vec!["100".into(), "200".into(), "300".into()],
                }
            )
            .is_err()
        );
    }

    #[test]
    fn guarded_playlist_member_insert_targets_one_position_and_rejects_invalid_position() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = PlaylistMemberInsertRequest {
            playlist_index: 1,
            expected_order: vec!["200".into(), "400".into()],
            track_id: "100".into(),
            position: 1,
        };
        let result = prepare_playlist_member_insert(source, &request).unwrap();
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(after.selected_playlists[1].tracks, ["200", "100", "400"]);
        assert_eq!(
            after.selected_tracks,
            preflight_export(source).unwrap().selected_tracks
        );
        assert_eq!(
            source,
            include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
        );
        let invalid_position = PlaylistMemberInsertRequest {
            position: 3,
            ..request.clone()
        };
        assert!(prepare_playlist_member_insert(source, &invalid_position).is_err());
        let stale = PlaylistMemberInsertRequest {
            expected_order: vec!["200".into()],
            ..request.clone()
        };
        assert!(prepare_playlist_member_insert(source, &stale).is_err());
        let present = PlaylistMemberInsertRequest {
            track_id: "200".into(),
            ..request.clone()
        };
        assert!(prepare_playlist_member_insert(source, &present).is_err());
        let unknown = PlaylistMemberInsertRequest {
            track_id: "999".into(),
            ..request
        };
        assert!(prepare_playlist_member_insert(source, &unknown).is_err());
    }

    #[test]
    fn guarded_tempo_beat_edit_changes_only_selected_battito_and_rejects_invalid_requests() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
        let request = TempoBeatEditRequest {
            track_id: "700".into(),
            tempo_index: 0,
            expected_old_beat: "1".into(),
            new_beat: "2".into(),
        };
        let result = prepare_tempo_beat_edit(source, &request).unwrap();
        let expected = String::from_utf8(source.to_vec()).unwrap().replacen(
            "Battito=\"1\"",
            "Battito=\"2\"",
            1,
        );
        assert_eq!(result.bytes, expected.as_bytes());
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(after.selected_tracks[0].tempos[0].beat, "2");
        assert_eq!(after.selected_tracks[0].tempos[1].beat, "1");

        for invalid in [
            TempoBeatEditRequest {
                expected_old_beat: "9".into(),
                ..request.clone()
            },
            TempoBeatEditRequest {
                tempo_index: 9,
                ..request.clone()
            },
            TempoBeatEditRequest {
                new_beat: "1.5".into(),
                ..request.clone()
            },
            TempoBeatEditRequest {
                track_id: "999".into(),
                ..request
            },
        ] {
            assert!(prepare_tempo_beat_edit(source, &invalid).is_err());
        }
    }

    #[test]
    fn guarded_tempo_meter_edit_changes_only_selected_raw_value_and_rejects_invalid_text() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
        let request = TempoMeterEditRequest {
            track_id: "700".into(),
            tempo_index: 0,
            expected_old_meter: "4/4".into(),
            new_meter: "3/4".into(),
        };
        let result = prepare_tempo_meter_edit(source, &request).unwrap();
        let expected = String::from_utf8(source.to_vec()).unwrap().replacen(
            "Metro=\"4/4\"",
            "Metro=\"3/4\"",
            1,
        );
        assert_eq!(result.bytes, expected.as_bytes());
        let after = preflight_export(&result.bytes).unwrap();
        assert_eq!(after.selected_tracks[0].tempos[0].meter, "3/4");
        assert_eq!(after.selected_tracks[0].tempos[1].meter, "4/4");

        for invalid in [
            TempoMeterEditRequest {
                expected_old_meter: "stale".into(),
                ..request.clone()
            },
            TempoMeterEditRequest {
                tempo_index: 9,
                ..request.clone()
            },
            TempoMeterEditRequest {
                new_meter: String::new(),
                ..request.clone()
            },
            TempoMeterEditRequest {
                new_meter: "bad\u{0001}".into(),
                ..request.clone()
            },
            TempoMeterEditRequest {
                track_id: "999".into(),
                ..request
            },
        ] {
            assert!(prepare_tempo_meter_edit(source, &invalid).is_err());
        }
    }

    #[test]
    fn guarded_track_delete_removes_collection_and_exact_shared_memberships() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = TrackDeleteRequest {
            track_id: "200".into(),
            expected_memberships: vec![
                (0, vec!["100".into(), "200".into(), "300".into()]),
                (1, vec!["200".into(), "400".into()]),
            ],
        };
        let prepared = prepare_track_delete(source, &request).unwrap();
        let after = preflight_export(&prepared.bytes).unwrap();
        assert_eq!(after.selected_tracks.len(), 3);
        assert_eq!(after.selected_playlists[0].tracks, ["100", "300"]);
        assert_eq!(after.selected_playlists[1].tracks, ["400"]);
        assert_eq!(
            after
                .selected_tracks
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["100", "300", "400"]
        );
        assert_eq!(
            after
                .selected_playlists
                .iter()
                .map(|p| p.tracks.len())
                .collect::<Vec<_>>(),
            [2, 1]
        );
        let expected_bytes = String::from_utf8(source.to_vec())
            .unwrap()
            .replace("<COLLECTION Entries=\"4\">", "<COLLECTION Entries=\"3\">")
            .replace(
                "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\"/>",
                "",
            )
            .replace(
                "Name=\"House\" Entries=\"3\"",
                "Name=\"House\" Entries=\"2\"",
            )
            .replace(
                "Name=\"Techno\" Entries=\"2\"",
                "Name=\"Techno\" Entries=\"1\"",
            )
            .replace("<TRACK Key=\"200\"/>", "");
        assert_eq!(prepared.bytes, expected_bytes.as_bytes());
        assert_eq!(
            prepared.bytes.iter().filter(|b| **b == b'\n').count(),
            source.iter().filter(|b| **b == b'\n').count()
        );
        for stale in [
            TrackDeleteRequest {
                expected_memberships: vec![(0, vec!["100".into(), "200".into(), "300".into()])],
                ..request.clone()
            },
            TrackDeleteRequest {
                expected_memberships: vec![
                    (0, vec!["100".into(), "300".into()]),
                    (1, vec!["200".into(), "400".into()]),
                ],
                ..request.clone()
            },
        ] {
            assert!(prepare_track_delete(source, &stale).is_err());
        }
    }

    #[test]
    fn guarded_track_delete_rejects_bad_target_references_and_counts() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = TrackDeleteRequest {
            track_id: "200".into(),
            expected_memberships: vec![
                (0, vec!["100".into(), "200".into(), "300".into()]),
                (1, vec!["200".into(), "400".into()]),
            ],
        };
        let duplicate_membership = String::from_utf8(source.to_vec()).unwrap().replacen(
            "<TRACK Key=\"200\"/>",
            "<TRACK Key=\"200\"/><TRACK Key=\"200\"/>",
            1,
        );
        let duplicate_guard = TrackDeleteRequest {
            expected_memberships: vec![
                (
                    0,
                    vec!["100".into(), "200".into(), "200".into(), "300".into()],
                ),
                (1, vec!["200".into(), "400".into()]),
            ],
            ..request.clone()
        };
        assert!(prepare_track_delete(duplicate_membership.as_bytes(), &duplicate_guard).is_err());
        for bad in [
            String::from_utf8(source.to_vec())
                .unwrap()
                .replace("Key=\"200\"", "Key=\"999\""),
            String::from_utf8(source.to_vec())
                .unwrap()
                .replace("Key=\"200\"", "Key=\"200\" Key=\"200\""),
            String::from_utf8(source.to_vec())
                .unwrap()
                .replace("Entries=\"4\"", "Entries=\"bad\""),
        ] {
            assert!(prepare_track_delete(bad.as_bytes(), &request).is_err());
        }
    }

    #[test]
    fn guarded_track_name_edit_changes_only_selected_attribute_and_checks_xml_text() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = TrackNameEditRequest {
            track_id: "100".into(),
            expected_old_name: "A".into(),
            new_name: "New < title".into(),
        };
        let prepared = prepare_track_name_edit(source, &request).unwrap();
        let expected = String::from_utf8(source.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"New &lt; title\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        let stale = TrackNameEditRequest {
            expected_old_name: "stale".into(),
            ..request.clone()
        };
        assert!(prepare_track_name_edit(source, &stale).is_err());
        let invalid = TrackNameEditRequest {
            new_name: "bad\u{1}".into(),
            ..request
        };
        assert!(prepare_track_name_edit(source, &invalid).is_err());
    }

    #[test]
    fn guarded_track_genre_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Genre=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].genre.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].genre, None);
        let request = TrackGenreEditRequest {
            track_id: "100".into(),
            expected_old_genre: "old & note".into(),
            new_genre: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_genre_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Genre=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].genre.as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Genre=\"old &amp; note\"",
            "Genre=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        for request in [
            TrackGenreEditRequest {
                track_id: "200".into(),
                expected_old_genre: "".into(),
                new_genre: "x".into(),
            },
            TrackGenreEditRequest {
                track_id: "999".into(),
                expected_old_genre: "x".into(),
                new_genre: "y".into(),
            },
            TrackGenreEditRequest {
                track_id: "100".into(),
                expected_old_genre: "stale".into(),
                new_genre: "y".into(),
            },
            TrackGenreEditRequest {
                track_id: "100".into(),
                expected_old_genre: "old & note".into(),
                new_genre: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_genre_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Genre=\"old &amp; note\"", "Genre=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0].genre.as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_label_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Label=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].label.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].label, None);
        let request = TrackLabelEditRequest {
            track_id: "100".into(),
            expected_old_label: "old & note".into(),
            new_label: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_label_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Label=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].label.as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Label=\"old &amp; note\"",
            "Label=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        for request in [
            TrackLabelEditRequest {
                track_id: "200".into(),
                expected_old_label: "".into(),
                new_label: "x".into(),
            },
            TrackLabelEditRequest {
                track_id: "999".into(),
                expected_old_label: "x".into(),
                new_label: "y".into(),
            },
            TrackLabelEditRequest {
                track_id: "100".into(),
                expected_old_label: "stale".into(),
                new_label: "y".into(),
            },
            TrackLabelEditRequest {
                track_id: "100".into(),
                expected_old_label: "old & note".into(),
                new_label: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_label_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Label=\"old &amp; note\"", "Label=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0].label.as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_composer_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Composer=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].composer.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].composer, None);
        let request = TrackComposerEditRequest {
            track_id: "100".into(),
            expected_old_composer: "old & note".into(),
            new_composer: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_composer_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Composer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0]
                .composer
                .as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Composer=\"old &amp; note\"",
            "Composer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
        let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Composer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Composer') is None",
                &xml,
            ])
            .status()
            .unwrap();
        assert!(decoded.success(), "independent ElementTree decode failed");
        for request in [
            TrackComposerEditRequest {
                track_id: "200".into(),
                expected_old_composer: "".into(),
                new_composer: "x".into(),
            },
            TrackComposerEditRequest {
                track_id: "999".into(),
                expected_old_composer: "x".into(),
                new_composer: "y".into(),
            },
            TrackComposerEditRequest {
                track_id: "100".into(),
                expected_old_composer: "stale".into(),
                new_composer: "y".into(),
            },
            TrackComposerEditRequest {
                track_id: "100".into(),
                expected_old_composer: "old & note".into(),
                new_composer: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_composer_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Composer=\"old &amp; note\"", "Composer=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0]
                .composer
                .as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_grouping_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Grouping=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].grouping.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].grouping, None);
        let request = TrackGroupingEditRequest {
            track_id: "100".into(),
            expected_old_grouping: "old & note".into(),
            new_grouping: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_grouping_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Grouping=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0]
                .grouping
                .as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Grouping=\"old &amp; note\"",
            "Grouping=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
        let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Grouping')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Grouping') is None",
                &xml,
            ])
            .status()
            .unwrap();
        assert!(decoded.success(), "independent ElementTree decode failed");
        for request in [
            TrackGroupingEditRequest {
                track_id: "200".into(),
                expected_old_grouping: "".into(),
                new_grouping: "x".into(),
            },
            TrackGroupingEditRequest {
                track_id: "999".into(),
                expected_old_grouping: "x".into(),
                new_grouping: "y".into(),
            },
            TrackGroupingEditRequest {
                track_id: "100".into(),
                expected_old_grouping: "stale".into(),
                new_grouping: "y".into(),
            },
            TrackGroupingEditRequest {
                track_id: "100".into(),
                expected_old_grouping: "old & note".into(),
                new_grouping: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_grouping_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Grouping=\"old &amp; note\"", "Grouping=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0]
                .grouping
                .as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_remixer_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Remixer=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].remixer.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].remixer, None);
        let request = TrackRemixerEditRequest {
            track_id: "100".into(),
            expected_old_remixer: "old & note".into(),
            new_remixer: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_remixer_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Remixer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].remixer.as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Remixer=\"old &amp; note\"",
            "Remixer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
        let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Remixer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Remixer') is None",
                &xml,
            ])
            .status()
            .unwrap();
        assert!(decoded.success(), "independent ElementTree decode failed");
        for request in [
            TrackRemixerEditRequest {
                track_id: "200".into(),
                expected_old_remixer: "".into(),
                new_remixer: "x".into(),
            },
            TrackRemixerEditRequest {
                track_id: "999".into(),
                expected_old_remixer: "x".into(),
                new_remixer: "y".into(),
            },
            TrackRemixerEditRequest {
                track_id: "100".into(),
                expected_old_remixer: "stale".into(),
                new_remixer: "y".into(),
            },
            TrackRemixerEditRequest {
                track_id: "100".into(),
                expected_old_remixer: "old & note".into(),
                new_remixer: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_remixer_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Remixer=\"old &amp; note\"", "Remixer=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0]
                .remixer
                .as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_mix_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec())
            .unwrap()
            .replacen(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" Mix=\"old &amp; note\"",
                1,
            )
            .replacen(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" Mix=\"\"",
                1,
            );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].mix.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].mix.as_deref(), Some(""));
        assert_eq!(parsed.tracks[2].mix, None);

        let request = TrackMixEditRequest {
            track_id: "100".into(),
            expected_old_mix: "old & note".into(),
            new_mix: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_mix_edit(source.as_bytes(), &request).unwrap();
        let output = String::from_utf8(prepared.bytes.clone()).unwrap();
        assert!(output.contains("Mix=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\""));
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].mix.as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Mix=\"old &amp; note\"",
            "Mix=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); ts={x.get('TrackID'):x for x in r.find('COLLECTION')}; assert ts['100'].get('Mix')=='A\\tB\\nC\\rD & <x>'; assert ts['200'].get('Mix')==''; assert ts['300'].get('Mix') is None",
                &output,
            ])
            .status()
            .unwrap();
        assert!(decoded.success(), "independent ElementTree decode failed");

        for request in [
            TrackMixEditRequest {
                track_id: "300".into(),
                expected_old_mix: "".into(),
                new_mix: "x".into(),
            },
            TrackMixEditRequest {
                track_id: "100".into(),
                expected_old_mix: "stale".into(),
                new_mix: "x".into(),
            },
            TrackMixEditRequest {
                track_id: "100".into(),
                expected_old_mix: "old & note".into(),
                new_mix: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_mix_edit(source.as_bytes(), &request).is_err());
        }
    }

    #[test]
    fn guarded_track_tonality_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Tonality=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].tonality.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].tonality, None);
        let request = TrackTonalityEditRequest {
            track_id: "100".into(),
            expected_old_tonality: "old & note".into(),
            new_tonality: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_tonality_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Tonality=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0]
                .tonality
                .as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Tonality=\"old &amp; note\"",
            "Tonality=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        for request in [
            TrackTonalityEditRequest {
                track_id: "200".into(),
                expected_old_tonality: "".into(),
                new_tonality: "x".into(),
            },
            TrackTonalityEditRequest {
                track_id: "999".into(),
                expected_old_tonality: "x".into(),
                new_tonality: "y".into(),
            },
            TrackTonalityEditRequest {
                track_id: "100".into(),
                expected_old_tonality: "stale".into(),
                new_tonality: "y".into(),
            },
            TrackTonalityEditRequest {
                track_id: "100".into(),
                expected_old_tonality: "old & note".into(),
                new_tonality: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_tonality_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Tonality=\"old &amp; note\"", "Tonality=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0]
                .tonality
                .as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_comments_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Comments=\"old &amp; note\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].comments.as_deref(), Some("old & note"));
        assert_eq!(parsed.tracks[1].comments, None);
        let request = TrackCommentsEditRequest {
            track_id: "100".into(),
            expected_old_comments: "old & note".into(),
            new_comments: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_comments_edit(source.as_bytes(), &request).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Comments=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0]
                .comments
                .as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen(
            "Comments=\"old &amp; note\"",
            "Comments=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
            1,
        );
        assert_eq!(prepared.bytes, expected.as_bytes());
        for request in [
            TrackCommentsEditRequest {
                track_id: "200".into(),
                expected_old_comments: "".into(),
                new_comments: "x".into(),
            },
            TrackCommentsEditRequest {
                track_id: "999".into(),
                expected_old_comments: "x".into(),
                new_comments: "y".into(),
            },
            TrackCommentsEditRequest {
                track_id: "100".into(),
                expected_old_comments: "stale".into(),
                new_comments: "y".into(),
            },
            TrackCommentsEditRequest {
                track_id: "100".into(),
                expected_old_comments: "old & note".into(),
                new_comments: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_comments_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replacen("Comments=\"old &amp; note\"", "Comments=\"\"", 1);
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0]
                .comments
                .as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_album_edit_preserves_optional_state_and_xml_whitespace() {
        let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(base.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Album=\"old &amp; record\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].album.as_deref(), Some("old & record"));
        assert_eq!(parsed.tracks[1].album, None);
        let request = TrackAlbumEditRequest {
            track_id: "100".into(),
            expected_old_album: "old & record".into(),
            new_album: "A\tB\nC\rD & <x>".into(),
        };
        let prepared = prepare_track_album_edit(source.as_bytes(), &request).unwrap();
        let encoded = "Album=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains(encoded)
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].album.as_deref(),
            Some("A\tB\nC\rD & <x>")
        );
        let expected = source.replacen("Album=\"old &amp; record\"", encoded, 1);
        assert_eq!(prepared.bytes, expected.as_bytes());
        for request in [
            TrackAlbumEditRequest {
                track_id: "200".into(),
                expected_old_album: "".into(),
                new_album: "x".into(),
            },
            TrackAlbumEditRequest {
                track_id: "999".into(),
                expected_old_album: "x".into(),
                new_album: "y".into(),
            },
            TrackAlbumEditRequest {
                track_id: "100".into(),
                expected_old_album: "stale".into(),
                new_album: "y".into(),
            },
            TrackAlbumEditRequest {
                track_id: "100".into(),
                expected_old_album: "old & record".into(),
                new_album: "bad\u{1}".into(),
            },
        ] {
            assert!(prepare_track_album_edit(source.as_bytes(), &request).is_err());
        }
        let empty = source.replace("Album=\"old &amp; record\"", "Album=\"\"");
        assert_eq!(
            parse(empty.as_bytes()).unwrap().tracks[0].album.as_deref(),
            Some("")
        );
    }

    #[test]
    fn guarded_track_artist_edit_preserves_optional_value_and_changes_only_target() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let source = String::from_utf8(source.to_vec()).unwrap().replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Artist=\"DJ &amp; Co\"",
            1,
        );
        let parsed = parse(source.as_bytes()).unwrap();
        assert_eq!(parsed.tracks[0].artist.as_deref(), Some("DJ & Co"));
        assert_eq!(parsed.tracks[1].artist, None);

        let request = TrackArtistEditRequest {
            track_id: "100".into(),
            expected_old_artist: "DJ & Co".into(),
            new_artist: "New & Artist".into(),
        };
        let prepared = prepare_track_artist_edit(source.as_bytes(), &request).unwrap();
        let expected = source.replacen("Artist=\"DJ &amp; Co\"", "Artist=\"New &amp; Artist\"", 1);
        assert_eq!(prepared.bytes, expected.as_bytes());
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].artist.as_deref(),
            Some("New & Artist")
        );

        let whitespace = TrackArtistEditRequest {
            expected_old_artist: "New & Artist".into(),
            new_artist: "A\tB\nC\rD".into(),
            ..request.clone()
        };
        let prepared = prepare_track_artist_edit(&prepared.bytes, &whitespace).unwrap();
        assert!(
            String::from_utf8(prepared.bytes.clone())
                .unwrap()
                .contains("Artist=\"A&#9;B&#10;C&#13;D\"")
        );
        assert_eq!(
            parse(&prepared.bytes).unwrap().tracks[0].artist.as_deref(),
            Some("A\tB\nC\rD")
        );

        let absent = TrackArtistEditRequest {
            track_id: "200".into(),
            expected_old_artist: "".into(),
            new_artist: "New Artist".into(),
        };
        assert!(prepare_track_artist_edit(source.as_bytes(), &absent).is_err());
        let stale = TrackArtistEditRequest {
            expected_old_artist: "stale".into(),
            ..request.clone()
        };
        assert!(prepare_track_artist_edit(source.as_bytes(), &stale).is_err());
        let invalid = TrackArtistEditRequest {
            new_artist: "bad\u{1}".into(),
            ..request
        };
        assert!(prepare_track_artist_edit(source.as_bytes(), &invalid).is_err());
    }

    #[test]
    fn export_accepts_frozen_fixtures_byte_exact_and_rejects_unknown_attributes() {
        let f02 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let f07 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
        let f10 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin");
        for source in [f02.as_slice(), f07.as_slice(), f10.as_slice()] {
            assert_eq!(export_xml(source).unwrap(), source);
            let preflight = preflight_export(source).unwrap();
            assert_eq!(preflight.decision, ExportDecision::AllowExport);
            assert!(preflight.withheld_units.is_empty());
            assert!(preflight.loss_report.is_empty());
        }
        let f10_plan = preflight_export(f10).unwrap();
        assert_eq!(f10_plan.selected_tracks.len(), 1);
        assert_eq!(
            f10_plan.selected_tracks[0]
                .tempos
                .iter()
                .map(|tempo| (
                    tempo.start_seconds.as_str(),
                    tempo.bpm.as_str(),
                    tempo.meter.as_str(),
                    tempo.beat.as_str()
                ))
                .collect::<Vec<_>>(),
            [
                ("0", "120", "4/4", "1"),
                ("8", "128", "4/4", "1"),
                ("16", "132", "7/8", "3")
            ]
        );
        let f02_text = String::from_utf8(f02.to_vec()).unwrap();
        let bad_folder_count = f02_text.replacen("Count=\"2\"", "Count=\"3\"", 1);
        assert_ne!(
            bad_folder_count, f02_text,
            "fixture must exercise folder Count"
        );
        assert!(export_xml(bad_folder_count.as_bytes()).is_err());
        let bad_playlist_entries = f02_text.replacen("Entries=\"3\"", "Entries=\"4\"", 1);
        assert_ne!(
            bad_playlist_entries, f02_text,
            "fixture must exercise playlist Entries"
        );
        assert!(export_xml(bad_playlist_entries.as_bytes()).is_err());
        let mutated = f02_text.replace("Name=\"A\"", "Name=\"A\" Extra=\"x\"");
        assert!(export_xml(mutated.as_bytes()).is_err());
        assert!(export_xml(b"<DJ_PLAYLISTS Version=\"1.0.0\"/>").is_err());
    }

    #[test]
    fn rejects_invalid_required_numeric_values() {
        let invalid = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Name=\"T\" Location=\"x\"><TEMPO Inizio=\"0\" Bpm=\"fast\" Metro=\"4/4\" Battito=\"1\"/></TRACK></COLLECTION></DJ_PLAYLISTS>";
        assert!(
            parse(invalid)
                .unwrap_err()
                .to_string()
                .contains("invalid BPM")
        );
    }

    #[test]
    fn rejects_dangling_playlist_references_and_missing_required_attributes() {
        let dangling = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION></COLLECTION><PLAYLISTS><NODE Type=\"0\" Name=\"ROOT\"><NODE Type=\"1\" Name=\"X\"><TRACK Key=\"9\"/></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>";
        assert!(
            parse(dangling)
                .unwrap_err()
                .to_string()
                .contains("dangling playlist track reference 9")
        );
        let missing = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Location=\"x\"/></COLLECTION></DJ_PLAYLISTS>";
        assert!(
            parse(missing)
                .unwrap_err()
                .to_string()
                .contains("missing required attribute Name")
        );
    }

    #[test]
    fn parses_a1_tracks_playlists_and_preserves_ordered_points() {
        let x = parse(include_bytes!(
            "../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin"
        ))
        .unwrap();
        assert_eq!(
            x.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["100", "200", "300", "400"]
        );
        assert_eq!(
            x.playlists
                .iter()
                .map(|p| p.tracks.iter().map(String::as_str).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [vec!["100", "200", "300"], vec!["200", "400"]]
        );
        let t = parse(include_bytes!(
            "../../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin"
        ))
        .unwrap();
        assert_eq!(
            t.tracks[0]
                .tempos
                .iter()
                .map(|p| p.bpm.as_str())
                .collect::<Vec<_>>(),
            ["120", "128", "132"]
        );
    }
    #[test]
    fn rejects_malformed_and_unsupported_version() {
        assert!(
            parse(b"<oops>")
                .unwrap_err()
                .to_string()
                .contains("malformed XML")
        );
        let e = parse(b"<DJ_PLAYLISTS Version=\"2.0.0\"/>").unwrap_err();
        assert!(e.to_string().contains("unsupported XML version"));
    }
    #[test]
    fn rejects_mismatched_closing_tags() {
        let malformed = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION></PLAYLISTS>";
        assert!(
            parse(malformed)
                .unwrap_err()
                .to_string()
                .contains("malformed XML")
        );
    }

    #[test]
    fn guarded_playlist_reorder_preserves_other_xml_and_rejects_non_permutations() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let request = PlaylistReorderRequest {
            playlist_index: 0,
            expected_order: vec!["100".into(), "200".into(), "300".into()],
            new_order: vec!["300".into(), "100".into(), "200".into()],
        };
        let changed = prepare_playlist_reorder(source, &request).unwrap().bytes;
        let parsed = parse(&changed).unwrap();
        assert_eq!(parsed.playlists[0].tracks, request.new_order);
        assert_eq!(parsed.playlists[1].tracks, vec!["200", "400"]);
        assert_eq!(parsed.tracks, parse(source).unwrap().tracks);
        let stale = PlaylistReorderRequest {
            expected_order: vec!["200".into(), "100".into(), "300".into()],
            ..request.clone()
        };
        assert!(prepare_playlist_reorder(source, &stale).is_err());
        for bad in [
            vec!["300".into(), "100".into(), "100".into()],
            vec!["300".into(), "100".into(), "999".into()],
            vec!["300".into(), "100".into()],
        ] {
            let invalid = PlaylistReorderRequest {
                new_order: bad,
                ..request.clone()
            };
            assert!(prepare_playlist_reorder(source, &invalid).is_err());
        }
    }

    #[test]
    fn guarded_reorder_distinguishes_duplicate_playlist_names_by_node_identity() {
        let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        let mut text = String::from_utf8(source.to_vec()).unwrap();
        let duplicate_branch = "      <NODE Type=\"0\" Name=\"Sets\" Count=\"1\">\n        <NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>\n      </NODE>\n";
        text = text.replace(
            "<NODE Type=\"0\" Name=\"ROOT\" Count=\"1\">",
            "<NODE Type=\"0\" Name=\"ROOT\" Count=\"2\">",
        );
        text = text.replace(
            "    </NODE>\n  </PLAYLISTS>",
            &format!("{duplicate_branch}    </NODE>\n  </PLAYLISTS>"),
        );
        let request = PlaylistReorderRequest {
            playlist_index: 0,
            expected_order: vec!["100".into(), "200".into(), "300".into()],
            new_order: vec!["300".into(), "100".into(), "200".into()],
        };
        let changed = prepare_playlist_reorder(text.as_bytes(), &request)
            .unwrap()
            .bytes;
        let parsed = parse(&changed).unwrap();
        assert_eq!(parsed.playlists[0].tracks, request.new_order);
        assert_eq!(parsed.playlists[1].tracks, vec!["200", "400"]);
        assert_eq!(parsed.playlists[2].tracks, vec!["100", "200", "300"]);
        assert!(
            String::from_utf8(changed)
                .unwrap()
                .contains(duplicate_branch)
        );
    }

    #[test]
    fn rejects_duplicate_track_ids() {
        let x=b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Name=\"a\" Location=\"x\"/><TRACK TrackID=\"1\" Name=\"b\" Location=\"y\"/></COLLECTION></DJ_PLAYLISTS>";
        assert!(
            parse(x)
                .unwrap_err()
                .to_string()
                .contains("duplicate track identity")
        );
    }
}
