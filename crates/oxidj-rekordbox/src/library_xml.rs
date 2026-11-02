//! Write a new Rekordbox XML document from a caller-supplied library (M7 slice).
//! The input is this adapter's own DTO so no catalog type enters the adapter.
use super::*;

/// One COLLECTION TRACK to write. `track_id` is the XML `TrackID`/playlist `Key`.
/// Optional attributes are written only when `Some`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryXmlTrack {
    pub track_id: u32,
    pub name: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub average_bpm_hundredths: u32,
    pub total_time_seconds: u32,
    /// Already a URI; written verbatim.
    pub location: String,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub tonality: Option<String>,
    pub comments: Option<String>,
    /// `YYYY-MM-DD`.
    pub date_added: Option<String>,
    pub year: Option<u32>,
    pub size: Option<u64>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub bit_rate: Option<u32>,
    pub sample_rate: Option<u32>,
    /// 1..=5 stars, written as the XML 0–255 scale (`stars * 51`).
    pub rating_stars: Option<u8>,
}

/// Optional TRACK attributes this writer emits when the caller supplies a value.
pub const LIBRARY_XML_OPTIONAL_TRACK_ATTRIBUTES: &[&str] = &[
    "Artist",
    "Album",
    "Genre",
    "Size",
    "DiscNumber",
    "TrackNumber",
    "Year",
    "DateAdded",
    "BitRate",
    "SampleRate",
    "Comments",
    "Rating",
    "Tonality",
    "Label",
];

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
    "Composer",
    "Grouping",
    "Remixer",
    "Mix",
    "Kind",
    "DateModified",
    "LastPlayed",
    "PlayCount",
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
        let text = |out: &mut String, key: &str, value: &Option<String>| -> Result<(), Error> {
            if let Some(value) = value {
                let _ = write!(out, " {key}=\"{}\"", checked(value)?);
            }
            Ok(())
        };
        let number = |out: &mut String, key: &str, value: Option<u64>| {
            if let Some(value) = value {
                let _ = write!(out, " {key}=\"{value}\"");
            }
        };
        if t.date_added.as_deref().is_some_and(|d| {
            d.len() != 10
                || !d.bytes().enumerate().all(|(i, b)| {
                    if i == 4 || i == 7 {
                        b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }
                })
        }) {
            return Err(Error("library DateAdded is not YYYY-MM-DD".into()));
        }
        if t.rating_stars
            .is_some_and(|stars| !(1..=5).contains(&stars))
        {
            return Err(Error("library rating is outside 1..=5 stars".into()));
        }
        text(&mut out, "Artist", &t.artist)?;
        text(&mut out, "Album", &t.album)?;
        text(&mut out, "Genre", &t.genre)?;
        number(&mut out, "Size", t.size);
        let _ = write!(out, " TotalTime=\"{}\"", t.total_time_seconds);
        number(&mut out, "DiscNumber", t.disc_number.map(u64::from));
        number(&mut out, "TrackNumber", t.track_number.map(u64::from));
        number(&mut out, "Year", t.year.map(u64::from));
        let _ = write!(
            out,
            " AverageBpm=\"{}.{:02}\"",
            t.average_bpm_hundredths / 100,
            t.average_bpm_hundredths % 100
        );
        text(&mut out, "DateAdded", &t.date_added)?;
        number(&mut out, "BitRate", t.bit_rate.map(u64::from));
        number(&mut out, "SampleRate", t.sample_rate.map(u64::from));
        text(&mut out, "Comments", &t.comments)?;
        number(
            &mut out,
            "Rating",
            t.rating_stars.map(|stars| u64::from(stars) * 51),
        );
        let _ = write!(out, " Location=\"{}\"", checked(&t.location)?);
        text(&mut out, "Tonality", &t.tonality)?;
        text(&mut out, "Label", &t.label)?;
        out.push_str("/>\n");
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
                ..LibraryXmlTrack::default()
            },
            LibraryXmlTrack {
                track_id: 2,
                name: "Two".into(),
                artist: Some("Art".into()),
                album: None,
                average_bpm_hundredths: 9_000,
                total_time_seconds: 7,
                location: "file://localhost/two.mp3".into(),
                ..LibraryXmlTrack::default()
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
            ..LibraryXmlTrack::default()
        };
        let bad_key = [LibraryXmlNode::Playlist {
            name: "P".into(),
            track_ids: vec![9],
        }];
        assert!(write_library_xml(std::slice::from_ref(&track), &bad_key).is_err());
        assert!(write_library_xml(&[track.clone(), track.clone()], &[]).is_err());
        let mut control = track.clone();
        control.name = "bad\u{1}".into();
        assert!(write_library_xml(&[control], &[]).is_err());
        for bad in [
            LibraryXmlTrack {
                date_added: Some("2026-1-02".into()),
                ..track.clone()
            },
            LibraryXmlTrack {
                rating_stars: Some(6),
                ..track.clone()
            },
            LibraryXmlTrack {
                genre: Some("bad\u{1}".into()),
                ..track.clone()
            },
        ] {
            assert!(write_library_xml(&[bad], &[]).is_err());
        }
    }

    #[test]
    fn optional_metadata_is_written_only_when_present_and_reparses() {
        let full = LibraryXmlTrack {
            track_id: 1,
            name: "Full".into(),
            location: "file://localhost/full.wav".into(),
            genre: Some("G & <H>".into()),
            label: Some("L".into()),
            tonality: Some("8A".into()),
            comments: Some("C \"q\"".into()),
            date_added: Some("2026-03-04".into()),
            year: Some(1999),
            size: Some(5_000_000_000),
            disc_number: Some(2),
            track_number: Some(11),
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            rating_stars: Some(3),
            ..LibraryXmlTrack::default()
        };
        let bare = LibraryXmlTrack {
            track_id: 2,
            name: "Bare".into(),
            location: "file://localhost/bare.wav".into(),
            ..LibraryXmlTrack::default()
        };
        let bytes = write_library_xml(&[full, bare], &[]).unwrap();
        let preview = parse(&bytes).unwrap();
        assert!(preview.unsupported.is_empty());
        let t = &preview.tracks[0];
        assert_eq!(t.genre.as_deref(), Some("G & <H>"));
        assert_eq!(t.label.as_deref(), Some("L"));
        assert_eq!(t.tonality.as_deref(), Some("8A"));
        assert_eq!(t.comments.as_deref(), Some("C \"q\""));
        assert_eq!(t.date_added.as_deref(), Some("2026-03-04"));
        assert_eq!(t.year.as_deref(), Some("1999"));
        assert_eq!(t.size.as_deref(), Some("5000000000"));
        assert_eq!(t.disc_number.as_deref(), Some("2"));
        assert_eq!(t.track_number.as_deref(), Some("11"));
        assert_eq!(t.bit_rate.as_deref(), Some("320"));
        assert_eq!(t.sample_rate.as_deref(), Some("44100"));
        assert_eq!(t.rating.as_deref(), Some("153"));
        let b = &preview.tracks[1];
        for value in [
            &b.artist,
            &b.album,
            &b.genre,
            &b.label,
            &b.tonality,
            &b.comments,
            &b.date_added,
            &b.year,
            &b.size,
            &b.disc_number,
            &b.track_number,
            &b.bit_rate,
            &b.sample_rate,
            &b.rating,
        ] {
            assert_eq!(value, &None);
        }
        let text = String::from_utf8(bytes).unwrap();
        let bare_line = text.lines().find(|l| l.contains("Name=\"Bare\"")).unwrap();
        assert_eq!(
            bare_line.trim(),
            "<TRACK TrackID=\"2\" Name=\"Bare\" TotalTime=\"0\" AverageBpm=\"0.00\" Location=\"file://localhost/bare.wav\"/>"
        );
    }
}
