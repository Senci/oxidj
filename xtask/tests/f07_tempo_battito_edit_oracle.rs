//! F-09 independent post-edit oracle for one guarded A1 F-07 TEMPO Battito edit.
use oxidj_rekordbox::{TempoBeatEditRequest, prepare_tempo_beat_edit};
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
fn node(e: quick_xml::events::BytesStart<'_>, decoder: quick_xml::encoding::Decoder) -> Node {
    let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
    let attrs = e
        .attributes()
        .map(|a| {
            let a = a.unwrap();
            (
                String::from_utf8(a.key.local_name().as_ref().to_vec()).unwrap(),
                a.decoded_and_normalized_value(XmlVersion::Explicit1_0, decoder)
                    .unwrap()
                    .into_owned(),
            )
        })
        .collect();
    Node {
        name,
        attrs,
        children: Vec::new(),
    }
}
fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) => stack.push(node(e, reader.decoder())),
            Event::Empty(e) => {
                let node = node(e, reader.decoder());
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
fn attribute<'a>(node: &'a Node, key: &str) -> &'a str {
    node.attrs
        .iter()
        .find(|(name, _)| name == key)
        .unwrap()
        .1
        .as_str()
}
fn expected_tree() -> Node {
    let mut expected = decode(SOURCE);
    let collection = expected
        .children
        .iter_mut()
        .find(|n| n.name == "COLLECTION")
        .unwrap();
    let track = collection
        .children
        .iter_mut()
        .find(|n| n.name == "TRACK")
        .unwrap();
    assert_eq!(attribute(track, "TrackID"), "700");
    let mut tempos = track.children.iter_mut().filter(|n| n.name == "TEMPO");
    let first = tempos.next().unwrap();
    assert_eq!(
        (
            attribute(first, "Inizio"),
            attribute(first, "Bpm"),
            attribute(first, "Metro"),
            attribute(first, "Battito")
        ),
        ("0", "120", "4/4", "1")
    );
    let second = tempos.next().unwrap();
    assert_eq!(
        (
            attribute(second, "Inizio"),
            attribute(second, "Bpm"),
            attribute(second, "Metro"),
            attribute(second, "Battito")
        ),
        ("10", "128", "4/4", "1")
    );
    assert!(tempos.next().is_none());
    first
        .attrs
        .iter_mut()
        .find(|(name, _)| name == "Battito")
        .unwrap()
        .1 = "2".into();
    expected
}
fn verify(bytes: &[u8]) -> Result<(), String> {
    if decode(bytes) == expected_tree() {
        Ok(())
    } else {
        Err("decoded XML tree differs beyond target Battito".into())
    }
}
#[test]
fn independently_verifies_guarded_battito_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TempoBeatEditRequest {
        track_id: "700".into(),
        tempo_index: 0,
        expected_old_beat: "1".into(),
        new_beat: "2".into(),
    };
    let output = prepare_tempo_beat_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "source fixture must remain byte-identical"
    );
    verify(&output).unwrap();

    let wrong_ordinal = TempoBeatEditRequest {
        tempo_index: 1,
        expected_old_beat: "1".into(),
        ..request.clone()
    };
    let wrong_output = prepare_tempo_beat_edit(SOURCE, &wrong_ordinal)
        .unwrap()
        .bytes;
    assert!(
        verify(&wrong_output).is_err(),
        "oracle accepted wrong ordinal"
    );
    for (tempo_index, expected_old_beat) in [(0, "0"), (1, "2")] {
        let stale = TempoBeatEditRequest {
            tempo_index,
            expected_old_beat: expected_old_beat.into(),
            ..request.clone()
        };
        assert!(
            prepare_tempo_beat_edit(SOURCE, &stale).is_err(),
            "adapter accepted stale guard at ordinal {tempo_index}"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Battito=\"2\"", "Battito=\"3\""),
        ("Inizio=\"0\"", "Inizio=\"1\""),
        ("Bpm=\"120\"", "Bpm=\"121\""),
        ("Metro=\"4/4\"", "Metro=\"3/4\""),
        ("Battito=\"1\"", "Battito=\"3\""),
        ("Inizio=\"10\"", "Inizio=\"11\""),
        ("Start=\"5\"", "Start=\"6\""),
        ("Name=\"Intro\"", "Name=\"Changed\""),
        ("TrackID=\"700\"", "TrackID=\"701\""),
        ("Entries=\"1\"", "Entries=\"2\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation source {from} must occur");
        assert!(
            verify(mutated.as_bytes()).is_err(),
            "oracle accepted mutation {from}"
        );
    }
    let reordered = text.replace(
        "<TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"2\"/>\n      <TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>",
        "<TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"4/4\" Battito=\"2\"/>",
    );
    assert_ne!(reordered, text, "expected tempo serialization pattern");
    assert!(
        verify(reordered.as_bytes()).is_err(),
        "oracle accepted reordered tempo points"
    );
}
