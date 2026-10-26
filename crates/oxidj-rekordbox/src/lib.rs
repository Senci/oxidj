//! Read-only inspection of the documented synthetic Rekordbox XML subset.
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

#[cfg(test)]
mod tests {
    use super::*;
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
