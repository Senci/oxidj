//! Inspection and bounded byte-preserving edits for a Rekordbox XML subset.
use quick_xml::{Reader, events::Event};
use std::{collections::HashSet, fs, path::Path};

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: String,
    pub name: String,
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
        "TRACK" if parent == Some("COLLECTION") => &["TrackID", "Name", "Location"],
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

fn track_name_range(
    bytes: &[u8],
    id: &str,
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
                    result = Some(attribute_value_range(&e, start, "Name", expected)?);
                }
                stack.push("TRACK".into());
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|p| p == "COLLECTION") =>
            {
                if attr(&e, "TrackID")? == id {
                    result = Some(attribute_value_range(&e, start, "Name", expected)?);
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
    result.ok_or_else(|| Error("Track Name location missing".into()))
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
    let range = track_name_range(bytes, &request.track_id, &request.expected_old_name)?;
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

fn tempo_start_range(
    bytes: &[u8],
    request: &TempoStartEditRequest,
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
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Inizio",
                            &request.expected_old_start_seconds,
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
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Inizio",
                            &request.expected_old_start_seconds,
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
                    "position mark {key} changed: expected {expected}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error(format!("POSITION_MARK missing {key} attribute")))
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

#[cfg(test)]
mod tests {
    use super::*;

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
