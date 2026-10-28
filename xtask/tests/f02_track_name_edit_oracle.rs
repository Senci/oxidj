//! F-09 independent post-edit oracle for the accepted A1 F-02 guarded Track Name edit.
use oxidj_rekordbox::{TrackNameEditRequest, prepare_track_name_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/rekordbox-a1/F-02/source.bin");
#[derive(Debug, PartialEq, Eq)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<N>,
}
fn decode(b: &[u8]) -> N {
    let mut r = Reader::from_reader(b);
    let mut stack: Vec<N> = vec![];
    let mut root = None;
    loop {
        match r.read_event().unwrap() {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
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
                    .collect();
                stack.push(N {
                    name,
                    attrs,
                    children: vec![],
                });
            }
            Event::Empty(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
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
                    .collect();
                let n = N {
                    name,
                    attrs,
                    children: vec![],
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(n)
                } else {
                    root = Some(n)
                }
            }
            Event::End(_) => {
                let n = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(n)
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
fn set_target_name(n: &mut N, value: &str) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let name = n.attrs.iter_mut().find(|(k, _)| k == "Name").unwrap();
        assert_eq!(name.1, "A");
        name.1 = value.into();
        return true;
    }
    let mut found = false;
    for child in &mut n.children {
        found |= set_target_name(child, value);
    }
    found
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(
        set_target_name(&mut expected, "New < title"),
        "fixture lacks TrackID 100"
    );
    if actual != expected {
        return Err("output differs beyond TrackID 100 Name".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_track_name_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackNameEditRequest {
        track_id: "100".into(),
        expected_old_name: "A".into(),
        new_name: "New < title".into(),
    };
    let output = prepare_track_name_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    check(&output).unwrap();
    for (track_id, expected_old_name) in [("100", "stale"), ("999", "A")] {
        let bad = TrackNameEditRequest {
            track_id: track_id.into(),
            expected_old_name: expected_old_name.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_name_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Name=\"New &lt; title\"", "Name=\"Altered\""),
        ("Name=\"B\"", "Name=\"Altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"3\"", "Entries=\"2\""),
        ("Key=\"100\"", "Key=\"101\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation fixture did not match: {from}");
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted semantic mutation {from}"
        );
    }
}
