//! F-09 independent post-edit oracle for one guarded synthetic F-07 Start edit.
use oxidj_rekordbox::{PositionMarkEditRequest, prepare_position_mark_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f07-position-mark-start-edit/source.xml");
#[derive(Debug, PartialEq, Eq)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<N>,
}
fn decode(b: &[u8]) -> N {
    let mut r = Reader::from_reader(b);
    let mut s: Vec<N> = vec![];
    let mut root = None;
    loop {
        match r.read_event().unwrap() {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
                    .attributes()
                    .map(|x| {
                        let x = x.unwrap();
                        (
                            String::from_utf8(x.key.local_name().as_ref().to_vec()).unwrap(),
                            x.decoded_and_normalized_value(XmlVersion::Explicit1_0, r.decoder())
                                .unwrap()
                                .into_owned(),
                        )
                    })
                    .collect();
                s.push(N {
                    name,
                    attrs,
                    children: vec![],
                });
            }
            Event::Empty(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
                    .attributes()
                    .map(|x| {
                        let x = x.unwrap();
                        (
                            String::from_utf8(x.key.local_name().as_ref().to_vec()).unwrap(),
                            x.decoded_and_normalized_value(XmlVersion::Explicit1_0, r.decoder())
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
                if let Some(p) = s.last_mut() {
                    p.children.push(n)
                } else {
                    root = Some(n)
                }
            }
            Event::End(_) => {
                let n = s.pop().unwrap();
                if let Some(p) = s.last_mut() {
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
fn expected() -> N {
    decode(SOURCE)
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = expected();
    let collection = &mut expected
        .children
        .iter_mut()
        .find(|n| n.name == "COLLECTION")
        .unwrap();
    let track = &mut collection
        .children
        .iter_mut()
        .find(|n| n.name == "TRACK")
        .unwrap();
    let target = track
        .children
        .iter_mut()
        .find(|n| n.name == "POSITION_MARK")
        .unwrap();
    target
        .attrs
        .iter_mut()
        .find(|(n, _)| n == "Start")
        .unwrap()
        .1 = "6".into();
    if actual != expected {
        return Err("output differs beyond target Start".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_start_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let req = PositionMarkEditRequest {
        track_id: "700".into(),
        mark_index: 0,
        expected_old_start_seconds: "5".into(),
        new_start_seconds: "6".into(),
    };
    let output = prepare_position_mark_edit(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();
    for (mark_index, expected_old_start_seconds) in [(0, "4"), (1, "5")] {
        let bad = PositionMarkEditRequest {
            mark_index,
            expected_old_start_seconds: expected_old_start_seconds.into(),
            ..req.clone()
        };
        assert!(prepare_position_mark_edit(SOURCE, &bad).is_err());
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Start=\"6\"", "Start=\"7\""),
        ("End=\"10\"", "End=\"11\""),
        ("Name=\"Keep me\"", "Name=\"Changed\""),
        ("Bpm=\"132\"", "Bpm=\"131\""),
        ("Key=\"700\"", "Key=\"701\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text);
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation {from}"
        );
    }
}
