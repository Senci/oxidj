//! F-09 independent post-edit oracle for one guarded A1 F-07 TEMPO Inizio edit.
use oxidj_rekordbox::{TempoStartEditRequest, prepare_tempo_start_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;

const SOURCE: &[u8] = include_bytes!("fixtures/rekordbox-a1/F-07/source.bin");

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
                    .map(|item| {
                        let item = item.unwrap();
                        (
                            String::from_utf8(item.key.local_name().as_ref().to_vec()).unwrap(),
                            item.decoded_and_normalized_value(
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
                    .map(|item| {
                        let item = item.unwrap();
                        (
                            String::from_utf8(item.key.local_name().as_ref().to_vec()).unwrap(),
                            item.decoded_and_normalized_value(
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

fn expected_after_edit() -> Node {
    let mut expected = decode(SOURCE);
    let collection = expected
        .children
        .iter_mut()
        .find(|node| node.name == "COLLECTION")
        .unwrap();
    let track = collection
        .children
        .iter_mut()
        .find(|node| node.name == "TRACK")
        .unwrap();
    assert_eq!(attr(track, "TrackID"), "700");
    let mut tempos: Vec<_> = track
        .children
        .iter_mut()
        .filter(|node| node.name == "TEMPO")
        .collect();
    assert_eq!(
        tempos.len(),
        2,
        "A1 fixture must retain its two ordered TEMPO points"
    );
    assert_eq!(
        (
            attr(tempos[0], "Inizio"),
            attr(tempos[0], "Bpm"),
            attr(tempos[0], "Metro"),
            attr(tempos[0], "Battito")
        ),
        ("0", "120", "4/4", "1")
    );
    assert_eq!(
        (
            attr(tempos[1], "Inizio"),
            attr(tempos[1], "Bpm"),
            attr(tempos[1], "Metro"),
            attr(tempos[1], "Battito")
        ),
        ("10", "128", "4/4", "1")
    );
    tempos[1]
        .attrs
        .iter_mut()
        .find(|(name, _)| name == "Inizio")
        .unwrap()
        .1 = "11".into();
    expected
}

fn check(bytes: &[u8]) -> Result<(), String> {
    if decode(bytes) != expected_after_edit() {
        return Err("output differs beyond TrackID 700 TEMPO ordinal 1 Inizio".into());
    }
    Ok(())
}

#[test]
fn independently_verifies_guarded_inizio_edit_and_rejects_mutations() {
    let frozen_source = SOURCE.to_vec();
    let request = TempoStartEditRequest {
        track_id: "700".into(),
        tempo_index: 1,
        expected_old_start_seconds: "10".into(),
        new_start_seconds: "11".into(),
    };
    let output = prepare_tempo_start_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(
        SOURCE,
        frozen_source.as_slice(),
        "adapter must not mutate frozen input"
    );
    check(&output).unwrap();

    let wrong_ordinal = TempoStartEditRequest {
        tempo_index: 0,
        expected_old_start_seconds: "0".into(),
        ..request.clone()
    };
    let wrong_output = prepare_tempo_start_edit(SOURCE, &wrong_ordinal)
        .unwrap()
        .bytes;
    assert!(
        check(&wrong_output).is_err(),
        "oracle accepted wrong ordinal"
    );
    for (tempo_index, expected_old_start_seconds) in [(1, "9"), (0, "10")] {
        let stale = TempoStartEditRequest {
            tempo_index,
            expected_old_start_seconds: expected_old_start_seconds.into(),
            ..request.clone()
        };
        assert!(
            prepare_tempo_start_edit(SOURCE, &stale).is_err(),
            "adapter accepted stale/wrong target guard"
        );
    }

    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Inizio=\"11\"", "Inizio=\"12\""),
        ("Inizio=\"0\"", "Inizio=\"1\""),
        ("Bpm=\"128\"", "Bpm=\"127\""),
        ("Metro=\"4/4\"", "Metro=\"3/4\""),
        ("Battito=\"1\"", "Battito=\"2\""),
        ("Start=\"5\"", "Start=\"6\""),
        ("Type=\"0\"", "Type=\"1\""),
        ("Name=\"Intro\"", "Name=\"Changed\""),
        ("Entries=\"1\"", "Entries=\"2\""),
        ("TrackID=\"700\"", "TrackID=\"701\""),
        ("Name=\"ROOT\"", "Name=\"Changed\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation source must exist: {from}");
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation {from}"
        );
    }
    let reordered = text.replace(
        "<TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"11\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>",
        "<TEMPO Inizio=\"11\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"1\"/>",
    );
    assert_ne!(
        reordered, text,
        "tempo-order mutation must match fixture serialization"
    );
    assert!(
        check(reordered.as_bytes()).is_err(),
        "oracle accepted reordered tempo points"
    );
}
