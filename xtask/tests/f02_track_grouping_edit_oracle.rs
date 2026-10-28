//! F-09 independent post-edit oracle for guarded F-02 TRACK.Grouping editing.
use oxidj_rekordbox::{TrackGroupingEditRequest, prepare_track_grouping_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-track-grouping-edit/source.xml");
#[derive(Debug, PartialEq, Eq)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<N>,
}
fn node(e: quick_xml::events::BytesStart<'_>, r: &Reader<&[u8]>) -> N {
    N {
        name: String::from_utf8(e.local_name().as_ref().to_vec()).unwrap(),
        attrs: e
            .attributes()
            .map(|a| {
                let a = a.unwrap();
                (
                    String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
                    a.decoded_and_normalized_value(XmlVersion::Explicit1_0, r.decoder())
                        .unwrap()
                        .into_owned(),
                )
            })
            .collect(),
        children: vec![],
    }
}
fn decode(bytes: &[u8]) -> N {
    let mut r = Reader::from_reader(bytes);
    let mut stack: Vec<N> = vec![];
    let mut root = None;
    loop {
        match r.read_event().unwrap() {
            Event::Start(e) => stack.push(node(e, &r)),
            Event::Empty(e) => {
                let n = node(e, &r);
                if let Some(p) = stack.last_mut() {
                    p.children.push(n)
                } else {
                    root = Some(n)
                }
            }
            Event::End(_) => {
                let n = stack.pop().unwrap();
                if let Some(p) = stack.last_mut() {
                    p.children.push(n)
                } else {
                    root = Some(n)
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}
fn change_expected_grouping(n: &mut N) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let value = n.attrs.iter_mut().find(|(k, _)| k == "Grouping").unwrap();
        assert_eq!(value.1, "old & crate");
        value.1 = "new < crate".into();
        return true;
    }
    let mut found = false;
    for child in &mut n.children {
        found |= change_expected_grouping(child)
    }
    found
}
fn has_grouping(n: &N, id: &str) -> bool {
    (n.name == "TRACK"
        && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == id)
        && n.attrs.iter().any(|(k, _)| k == "Grouping"))
        || n.children.iter().any(|c| has_grouping(c, id))
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(change_expected_grouping(&mut expected));
    if actual != expected {
        return Err("output differs beyond TrackID 100 Grouping".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_track_grouping_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackGroupingEditRequest {
        track_id: "100".into(),
        expected_old_grouping: "old & crate".into(),
        new_grouping: "new < crate".into(),
    };
    let output = prepare_track_grouping_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    let before = decode(SOURCE);
    assert!(has_grouping(&before, "100"));
    assert!(!has_grouping(&before, "200"));
    check(&output).unwrap();
    for (track_id, expected_old_grouping) in [
        ("100", "stale"),
        ("200", "old & crate"),
        ("999", "old & crate"),
    ] {
        let bad = TrackGroupingEditRequest {
            track_id: track_id.into(),
            expected_old_grouping: expected_old_grouping.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_grouping_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong-target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Grouping=\"new &lt; crate\"", "Grouping=\"altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"2\"", "Entries=\"1\""),
        ("Key=\"100\"", "Key=\"101\""),
        (
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Grouping=\"unexpected\"",
        ),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation fixture did not match: {from}");
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted semantic mutation {from}"
        );
    }
}
