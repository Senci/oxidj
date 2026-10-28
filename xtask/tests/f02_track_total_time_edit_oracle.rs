//! F-09 independent post-edit oracle for guarded F-02 TRACK.TotalTime editing.
use oxidj_rekordbox::{TrackTotalTimeEditRequest, prepare_track_total_time_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-track-total-time-edit/source.xml");
#[derive(Debug, PartialEq, Eq)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Child>,
}
#[derive(Debug, PartialEq, Eq)]
enum Child {
    Element(N),
    Text(String),
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
                    p.children.push(Child::Element(n))
                } else {
                    root = Some(n)
                }
            }
            Event::End(_) => {
                let n = stack.pop().unwrap();
                if let Some(p) = stack.last_mut() {
                    p.children.push(Child::Element(n))
                } else {
                    root = Some(n)
                }
            }
            Event::Text(e) => {
                if let Some(n) = stack.last_mut() {
                    n.children
                        .push(Child::Text(e.decode().unwrap().into_owned()));
                }
            }
            Event::CData(e) => {
                if let Some(n) = stack.last_mut() {
                    n.children
                        .push(Child::Text(e.decode().unwrap().into_owned()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}
fn change_expected(n: &mut N) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let v = n.attrs.iter_mut().find(|(k, _)| k == "TotalTime").unwrap();
        assert_eq!(v.1, "old & time");
        v.1 = "new < time".into();
        return true;
    }
    let mut found = false;
    for c in &mut n.children {
        if let Child::Element(c) = c {
            found |= change_expected(c)
        }
    }
    found
}
fn has_field(n: &N, id: &str) -> bool {
    (n.name == "TRACK"
        && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == id)
        && n.attrs.iter().any(|(k, _)| k == "TotalTime"))
        || n.children
            .iter()
            .any(|c| matches!(c, Child::Element(c) if has_field(c, id)))
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(change_expected(&mut expected));
    if actual != expected {
        Err("output differs beyond TrackID 100 TotalTime".into())
    } else {
        Ok(())
    }
}
#[test]
fn independently_verifies_guarded_track_total_time_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackTotalTimeEditRequest {
        track_id: "100".into(),
        expected_old_total_time: "old & time".into(),
        new_total_time: "new < time".into(),
    };
    let output = prepare_track_total_time_edit(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    let before = decode(SOURCE);
    assert!(has_field(&before, "100"));
    assert!(!has_field(&before, "200"));
    check(&output).unwrap();
    for (track_id, expected_old_total_time) in [
        ("100", "stale"),
        ("200", "old & time"),
        ("999", "old & time"),
    ] {
        let bad = TrackTotalTimeEditRequest {
            track_id: track_id.into(),
            expected_old_total_time: expected_old_total_time.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_total_time_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong-target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("TotalTime=\"new &lt; time\"", "TotalTime=\"altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"2\"", "Entries=\"1\""),
        ("Key=\"100\"", "Key=\"101\""),
        (
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" TotalTime=\"unexpected\"",
        ),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation fixture did not match: {from}");
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted semantic mutation {from}"
        );
    }
    let inserted = text.replacen(
        "<COLLECTION Entries=\"2\">",
        "<COLLECTION Entries=\"2\">injected",
        1,
    );
    assert_ne!(inserted, text);
    assert!(
        check(inserted.as_bytes()).is_err(),
        "oracle accepted inserted text node"
    );
}
