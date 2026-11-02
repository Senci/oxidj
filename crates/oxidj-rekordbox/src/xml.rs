//! Rekordbox XML parsing, export preflight, and shared attribute helpers.
use super::*;

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

pub(crate) fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Result<String, Error> {
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
pub(crate) fn optional(
    e: &quick_xml::events::BytesStart<'_>,
    key: &str,
) -> Result<Option<String>, Error> {
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
pub(crate) fn validate_tempo_beat(meter: &str, raw: &str) -> Result<i32, Error> {
    let beat = raw
        .parse::<i32>()
        .map_err(|_| Error(format!("invalid tempo beat {raw}")))?;
    if meter == "4/4" && !(1..=4).contains(&beat) {
        return Err(Error(format!(
            "tempo Battito {raw} outside 4/4 range 1..=4"
        )));
    }
    Ok(beat)
}
pub(crate) fn validate_decimal(field: &str, raw: &str, positive: bool) -> Result<(), Error> {
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
        folders: vec![],
        playlists: vec![],
        playlist_nodes: vec![],
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
                        out.playlist_nodes.push(PlaylistNodeMetadata {
                            path: playlist.as_ref().unwrap().path.clone(),
                            entries: optional(&e, "Entries")?,
                            key_type: optional(&e, "KeyType")?,
                        });
                    } else if ty == "0" {
                        out.folders.push(Folder {
                            ordinal: out.folders.len(),
                            path: names
                                .iter()
                                .cloned()
                                .chain(std::iter::once(n.clone()))
                                .collect(),
                            name: n.clone(),
                            count: optional(&e, "Count")?,
                        });
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
                            red: optional(&e, "Red")?,
                            green: optional(&e, "Green")?,
                            blue: optional(&e, "Blue")?,
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
                            red: optional(&e, "Red")?,
                            green: optional(&e, "Green")?,
                            blue: optional(&e, "Blue")?,
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
                        out.playlist_nodes.push(PlaylistNodeMetadata {
                            path: out.playlists.last().unwrap().path.clone(),
                            entries: optional(&e, "Entries")?,
                            key_type: optional(&e, "KeyType")?,
                        });
                    } else if ty == "0" {
                        out.folders.push(Folder {
                            ordinal: out.folders.len(),
                            path: names
                                .iter()
                                .cloned()
                                .chain(std::iter::once(n.clone()))
                                .collect(),
                            name: n.clone(),
                            count: optional(&e, "Count")?,
                        });
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
            validate_tempo_beat(&tempo.meter, &tempo.beat)?;
        }
        for mark in &track.marks {
            mark.kind
                .parse::<u8>()
                .map_err(|_| Error(format!("invalid position mark Type {}", mark.kind)))?;
            mark.number
                .parse::<i32>()
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

pub(crate) fn validate_export_element(
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
        "POSITION_MARK" => &[
            "Name", "Type", "Start", "Num", "End", "Red", "Green", "Blue",
        ],
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

pub(crate) fn valid_export_parent(stack: &[String], name: &str) -> bool {
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

/// Validate and return unchanged bytes for the frozen synthetic A1 no-edit subset.
pub fn export_xml(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    preflight_export(bytes)?;
    Ok(bytes.to_vec())
}

/// True when every character is allowed in XML 1.0 text.
pub(crate) fn valid_xml_text(value: &str) -> bool {
    value.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF),
    )
}

/// Escape an XML attribute value; `whitespace` also character-encodes TAB/LF/CR.
pub(crate) fn escape_attribute(value: &str, whitespace: bool) -> String {
    let escaped = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    if whitespace {
        escaped
            .replace('\t', "&#9;")
            .replace('\n', "&#10;")
            .replace('\r', "&#13;")
    } else {
        escaped
    }
}

pub(crate) fn attribute_value_range(
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
