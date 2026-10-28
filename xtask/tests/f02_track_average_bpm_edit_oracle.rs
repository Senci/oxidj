//! F-09 independent post-edit oracle for guarded F-02 TRACK.AverageBpm editing.
use oxidj_rekordbox::{TrackAverageBpmEditRequest, prepare_track_average_bpm_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-track-average-bpm-edit/source.xml");
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
fn change_expected(n: &mut N) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let v = n.attrs.iter_mut().find(|(k, _)| k == "AverageBpm").unwrap();
        assert_eq!(v.1, "old & bpm");
        v.1 = "new < bpm".into();
        return true;
    }
    let mut found = false;
    for c in &mut n.children {
        found |= change_expected(c)
    }
    found
}
fn has_field(n: &N, id: &str) -> bool {
    (n.name == "TRACK"
        && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == id)
        && n.attrs.iter().any(|(k, _)| k == "AverageBpm"))
        || n.children.iter().any(|c| has_field(c, id))
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(change_expected(&mut expected));
    if actual != expected {
        Err("output differs beyond TrackID 100 AverageBpm".into())
    } else {
        Ok(())
    }
}
#[test]
fn independently_verifies_guarded_track_play_bpm_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackAverageBpmEditRequest {
        track_id: "100".into(),
        expected_old_average_bpm: "old & bpm".into(),
        new_average_bpm: "new < bpm".into(),
    };
    let output = prepare_track_average_bpm_edit(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    let before = decode(SOURCE);
    assert!(has_field(&before, "100"));
    assert!(!has_field(&before, "200"));
    check(&output).unwrap();
    for (track_id, expected_old_average_bpm) in
        [("100", "stale"), ("200", "old & bpm"), ("999", "old & bpm")]
    {
        let bad = TrackAverageBpmEditRequest {
            track_id: track_id.into(),
            expected_old_average_bpm: expected_old_average_bpm.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_average_bpm_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong-target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("AverageBpm=\"new &lt; bpm\"", "AverageBpm=\"altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"2\"", "Entries=\"1\""),
        ("Key=\"100\"", "Key=\"101\""),
        (
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" AverageBpm=\"unexpected\"",
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
