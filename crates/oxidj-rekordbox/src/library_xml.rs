//! Write a new Rekordbox XML document from a caller-supplied library (M7 slice).
//! The input is this adapter's own DTO so no catalog type enters the adapter.
use super::*;

/// One COLLECTION TRACK to write. `track_id` is the XML `TrackID`/playlist `Key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryXmlTrack {
    pub track_id: u32,
    pub name: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub average_bpm_hundredths: u32,
    pub total_time_seconds: u32,
    /// Already a URI; written verbatim.
    pub location: String,
}

/// One PLAYLISTS NODE to write, in sibling order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryXmlNode {
    Folder {
        name: String,
        children: Vec<LibraryXmlNode>,
    },
    Playlist {
        name: String,
        track_ids: Vec<u32>,
    },
}

/// TRACK attributes this writer never emits (the catalog and the DeviceSQL PDB
/// preview do not supply them); callers report them as "not carried".
pub const LIBRARY_XML_NOT_WRITTEN_TRACK_ATTRIBUTES: &[&str] = &[
    "Comments",
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
    "BitRate",
    "SampleRate",
    "DateAdded",
    "DateModified",
    "LastPlayed",
    "PlayCount",
    "Rating",
    "Colour",
];
/// TRACK child elements this writer never emits (beat grid, cues).
pub const LIBRARY_XML_NOT_WRITTEN_TRACK_ELEMENTS: &[&str] = &["TEMPO", "POSITION_MARK"];

fn checked(value: &str) -> Result<String, Error> {
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
    {
        return Err(Error(
            "library value contains a character XML 1.0 cannot carry".into(),
        ));
    }
    Ok(escape_attribute(value, true))
}

/// Serialize a complete DJ_PLAYLISTS document. Fails closed on values XML cannot carry
/// and on playlist keys that do not name a written track.
pub fn write_library_xml(
    tracks: &[LibraryXmlTrack],
    roots: &[LibraryXmlNode],
) -> Result<Vec<u8>, Error> {
    use std::fmt::Write as _;
    let ids: HashSet<u32> = tracks.iter().map(|t| t.track_id).collect();
    if ids.len() != tracks.len() {
        return Err(Error("duplicate library track identity".into()));
    }
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<DJ_PLAYLISTS Version=\"1.0.0\">\n  <PRODUCT Name=\"OxiDJ\" Version=\"0.1.0\" Company=\"OxiDJ\"/>\n",
    );
    let _ = writeln!(out, "  <COLLECTION Entries=\"{}\">", tracks.len());
    for t in tracks {
        let _ = write!(
            out,
            "    <TRACK TrackID=\"{}\" Name=\"{}\"",
            t.track_id,
            checked(&t.name)?
        );
        if let Some(artist) = &t.artist {
            let _ = write!(out, " Artist=\"{}\"", checked(artist)?);
        }
        if let Some(album) = &t.album {
            let _ = write!(out, " Album=\"{}\"", checked(album)?);
        }
        let _ = writeln!(
            out,
            " TotalTime=\"{}\" AverageBpm=\"{}.{:02}\" Location=\"{}\"/>",
            t.total_time_seconds,
            t.average_bpm_hundredths / 100,
            t.average_bpm_hundredths % 100,
            checked(&t.location)?
        );
    }
    out.push_str("  </COLLECTION>\n  <PLAYLISTS>\n");
    fn node(
        out: &mut String,
        n: &LibraryXmlNode,
        depth: usize,
        ids: &HashSet<u32>,
    ) -> Result<(), Error> {
        use std::fmt::Write as _;
        if depth > 256 {
            return Err(Error("library tree is too deep".into()));
        }
        let pad = "  ".repeat(depth + 2);
        match n {
            LibraryXmlNode::Folder { name, children } => {
                let _ = writeln!(
                    out,
                    "{pad}<NODE Type=\"0\" Name=\"{}\" Count=\"{}\">",
                    checked(name)?,
                    children.len()
                );
                for child in children {
                    node(out, child, depth + 1, ids)?;
                }
                let _ = writeln!(out, "{pad}</NODE>");
            }
            LibraryXmlNode::Playlist { name, track_ids } => {
                let _ = writeln!(
                    out,
                    "{pad}<NODE Type=\"1\" Name=\"{}\" KeyType=\"0\" Entries=\"{}\">",
                    checked(name)?,
                    track_ids.len()
                );
                for id in track_ids {
                    if !ids.contains(id) {
                        return Err(Error("playlist key names no written track".into()));
                    }
                    let _ = writeln!(out, "{pad}  <TRACK Key=\"{id}\"/>");
                }
                let _ = writeln!(out, "{pad}</NODE>");
            }
        }
        Ok(())
    }
    node(
        &mut out,
        &LibraryXmlNode::Folder {
            name: "ROOT".into(),
            children: roots.to_vec(),
        },
        0,
        &ids,
    )?;
    out.push_str("  </PLAYLISTS>\n</DJ_PLAYLISTS>\n");
    Ok(out.into_bytes())
}

#[cfg(test)]
mod library_xml_tests {
    use super::*;

    #[test]
    fn written_library_reparses_with_order_escaping_and_absent_optionals() {
        let tracks = vec![
            LibraryXmlTrack {
                track_id: 1,
                name: "A & \"B\"\tC".into(),
                artist: None,
                album: Some("<Album>".into()),
                average_bpm_hundredths: 12_805,
                total_time_seconds: 181,
                location: "file://localhost/a%20b.wav".into(),
            },
            LibraryXmlTrack {
                track_id: 2,
                name: "Two".into(),
                artist: Some("Art".into()),
                album: None,
                average_bpm_hundredths: 9_000,
                total_time_seconds: 7,
                location: "file://localhost/two.mp3".into(),
            },
        ];
        let roots = vec![
            LibraryXmlNode::Playlist {
                name: "Zeta".into(),
                track_ids: vec![2, 1, 2],
            },
            LibraryXmlNode::Folder {
                name: "F".into(),
                children: vec![
                    LibraryXmlNode::Folder {
                        name: "Empty".into(),
                        children: vec![],
                    },
                    LibraryXmlNode::Playlist {
                        name: "Inner".into(),
                        track_ids: vec![],
                    },
                ],
            },
        ];
        let bytes = write_library_xml(&tracks, &roots).unwrap();
        let preview = parse(&bytes).unwrap();
        assert!(preview.unsupported.is_empty());
        assert_eq!(preview.tracks.len(), 2);
        assert_eq!(preview.tracks[0].name, "A & \"B\"\tC");
        assert_eq!(preview.tracks[0].artist, None);
        assert_eq!(preview.tracks[0].album.as_deref(), Some("<Album>"));
        assert_eq!(preview.tracks[0].average_bpm.as_deref(), Some("128.05"));
        assert_eq!(preview.tracks[1].total_time.as_deref(), Some("7"));
        let paths: Vec<_> = preview.playlists.iter().map(|p| p.path.clone()).collect();
        assert_eq!(
            paths,
            vec![vec!["Zeta".to_string()], vec!["F".into(), "Inner".into()]]
        );
        assert_eq!(preview.playlists[0].tracks, vec!["2", "1", "2"]);
        let folders: Vec<_> = preview.folders.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            folders,
            vec![
                vec!["ROOT".to_string()],
                vec!["ROOT".into(), "F".into()],
                vec!["ROOT".into(), "F".into(), "Empty".into()]
            ]
        );
    }

    #[test]
    fn writer_fails_closed_on_unknown_key_duplicate_id_and_control_chars() {
        let track = LibraryXmlTrack {
            track_id: 1,
            name: "One".into(),
            artist: None,
            album: None,
            average_bpm_hundredths: 0,
            total_time_seconds: 0,
            location: "file://localhost/one.wav".into(),
        };
        let bad_key = [LibraryXmlNode::Playlist {
            name: "P".into(),
            track_ids: vec![9],
        }];
        assert!(write_library_xml(std::slice::from_ref(&track), &bad_key).is_err());
        assert!(write_library_xml(&[track.clone(), track.clone()], &[]).is_err());
        let mut control = track;
        control.name = "bad\u{1}".into();
        assert!(write_library_xml(&[control], &[]).is_err());
    }
}
