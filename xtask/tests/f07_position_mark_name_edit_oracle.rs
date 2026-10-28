//! F-09 independent post-edit oracle for one guarded synthetic F-07 Name edit.
use oxidj_rekordbox::{PositionMarkNameEditRequest, prepare_position_mark_name_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;

const SOURCE: &[u8] = include_bytes!("fixtures/f07-position-mark-start-edit/source.xml");

#[derive(Debug, PartialEq, Eq)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(event) => {
                let name = String::from_utf8(event.local_name().as_ref().to_vec()).unwrap();
                let attrs = event
                    .attributes()
                    .map(|a| {
                        let a = a.unwrap();
                        (
                            String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
                            a.decoded_and_normalized_value(
                                XmlVersion::Explicit1_0,
                                reader.decoder(),
                            )
                            .unwrap()
                            .into_owned(),
                        )
                    })
                    .collect();
                stack.push(Node {
                    name,
                    attrs,
                    children: vec![],
                });
            }
            Event::Empty(event) => {
                let name = String::from_utf8(event.local_name().as_ref().to_vec()).unwrap();
                let attrs = event
                    .attributes()
                    .map(|a| {
                        let a = a.unwrap();
                        (
                            String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
                            a.decoded_and_normalized_value(
                                XmlVersion::Explicit1_0,
                                reader.decoder(),
                            )
                            .unwrap()
                            .into_owned(),
                        )
                    })
                    .collect();
                let node = Node {
                    name,
                    attrs,
                    children: vec![],
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::End(_) => {
                let node = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}

fn attr<'a>(node: &'a Node, key: &str) -> &'a str {
    node.attrs
        .iter()
        .find(|(name, _)| name == key)
        .unwrap()
        .1
        .as_str()
}

fn expected() -> Node {
    let mut tree = decode(SOURCE);
    let collection = tree
        .children
        .iter_mut()
        .find(|n| n.name == "COLLECTION")
        .unwrap();
    let track = collection
        .children
        .iter_mut()
        .find(|n| n.name == "TRACK")
        .unwrap();
    assert_eq!(attr(track, "TrackID"), "700");
    let marks: Vec<_> = track
        .children
        .iter_mut()
        .filter(|n| n.name == "POSITION_MARK")
        .collect();
    assert_eq!(marks.len(), 2, "fixture must contain exactly two marks");
    assert_eq!(
        (
            attr(marks[0], "Type"),
            attr(marks[0], "Start"),
            attr(marks[0], "Num"),
            attr(marks[0], "End"),
            attr(marks[0], "Name")
        ),
        ("4", "5", "0", "10", "Delete me")
    );
    assert_eq!(
        (
            attr(marks[1], "Type"),
            attr(marks[1], "Start"),
            attr(marks[1], "Num"),
            attr(marks[1], "End"),
            attr(marks[1], "Name")
        ),
        ("0", "0", "1", "0", "Keep me")
    );
    marks
        .into_iter()
        .next()
        .unwrap()
        .attrs
        .iter_mut()
        .find(|(name, _)| name == "Name")
        .unwrap()
        .1 = "Edited cue".into();
    tree
}

fn check(bytes: &[u8]) -> Result<(), String> {
    if decode(bytes) == expected() {
        Ok(())
    } else {
        Err("output differs beyond target Name".into())
    }
}

#[test]
fn independently_verifies_guarded_name_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = PositionMarkNameEditRequest {
        track_id: "700".into(),
        mark_index: 0,
        expected_old_name: "Delete me".into(),
        new_name: "Edited cue".into(),
    };
    let output = prepare_position_mark_name_edit(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();

    for (mark_index, expected_old_name) in [(0, "Stale"), (1, "Delete me"), (0, "")] {
        let bad = PositionMarkNameEditRequest {
            mark_index,
            expected_old_name: expected_old_name.into(),
            ..request.clone()
        };
        assert!(prepare_position_mark_name_edit(SOURCE, &bad).is_err());
    }
    let wrong = PositionMarkNameEditRequest {
        mark_index: 1,
        expected_old_name: "Keep me".into(),
        new_name: "Edited cue".into(),
        ..request.clone()
    };
    assert!(
        check(
            &prepare_position_mark_name_edit(SOURCE, &wrong)
                .unwrap()
                .bytes
        )
        .is_err()
    );

    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Name=\"Edited cue\"", "Name=\"Other cue\""),
        ("Type=\"4\"", "Type=\"3\""),
        ("Start=\"5\"", "Start=\"6\""),
        ("Num=\"0\"", "Num=\"2\""),
        ("End=\"10\"", "End=\"11\""),
        ("End=\"0\"", "End=\"1\""),
        ("Name=\"Keep me\"", "Name=\"Changed\""),
        ("Bpm=\"132\"", "Bpm=\"131\""),
        ("Inizio=\"16\"", "Inizio=\"17\""),
        ("Key=\"700\"", "Key=\"701\""),
        ("Name=\"House\"", "Name=\"Changed\""),
    ] {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text);
        assert!(
            check(changed.as_bytes()).is_err(),
            "oracle accepted mutation {from}"
        );
    }
}
