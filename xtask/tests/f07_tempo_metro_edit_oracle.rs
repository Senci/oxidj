//! F-09 independent post-edit oracle for one guarded A1 F-07 TEMPO Metro edit.
use oxidj_rekordbox::{TempoMeterEditRequest, prepare_tempo_meter_edit};
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
                let n = node(e, reader.decoder());
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
fn attr<'a>(n: &'a Node, key: &str) -> &'a str {
    n.attrs.iter().find(|(k, _)| k == key).unwrap().1.as_str()
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
    assert_eq!(attr(track, "TrackID"), "700");
    let mut tempos = track.children.iter_mut().filter(|n| n.name == "TEMPO");
    let first = tempos.next().unwrap();
    assert_eq!(
        (
            attr(first, "Inizio"),
            attr(first, "Bpm"),
            attr(first, "Metro"),
            attr(first, "Battito")
        ),
        ("0", "120", "4/4", "1")
    );
    let second = tempos.next().unwrap();
    assert_eq!(
        (
            attr(second, "Inizio"),
            attr(second, "Bpm"),
            attr(second, "Metro"),
            attr(second, "Battito")
        ),
        ("10", "128", "4/4", "1")
    );
    assert!(tempos.next().is_none());
    first
        .attrs
        .iter_mut()
        .find(|(k, _)| k == "Metro")
        .unwrap()
        .1 = "3/4".into();
    expected
}
fn verify(bytes: &[u8]) -> Result<(), String> {
    if decode(bytes) == expected_tree() {
        Ok(())
    } else {
        Err("decoded XML tree differs beyond target Metro".into())
    }
}
#[test]
fn independently_verifies_guarded_metro_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TempoMeterEditRequest {
        track_id: "700".into(),
        tempo_index: 0,
        expected_old_meter: "4/4".into(),
        new_meter: "3/4".into(),
    };
    let output = prepare_tempo_meter_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "source fixture must remain byte-identical"
    );
    verify(&output).unwrap();
    let wrong = TempoMeterEditRequest {
        tempo_index: 1,
        ..request.clone()
    };
    let wrong_output = prepare_tempo_meter_edit(SOURCE, &wrong).unwrap().bytes;
    assert!(
        verify(&wrong_output).is_err(),
        "oracle accepted wrong ordinal"
    );
    for (tempo_index, expected_old_meter) in [(0, "stale"), (1, "4/8")] {
        let stale = TempoMeterEditRequest {
            tempo_index,
            expected_old_meter: expected_old_meter.into(),
            ..request.clone()
        };
        assert!(
            prepare_tempo_meter_edit(SOURCE, &stale).is_err(),
            "adapter accepted stale guard at ordinal {tempo_index}"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Metro=\"3/4\"", "Metro=\"2/4\""),
        ("Inizio=\"0\"", "Inizio=\"1\""),
        ("Bpm=\"120\"", "Bpm=\"121\""),
        ("Battito=\"1\"", "Battito=\"2\""),
        ("Metro=\"4/4\"", "Metro=\"5/4\""),
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
        "<TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"3/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>",
        "<TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"0\" Bpm=\"120\" Metro=\"3/4\" Battito=\"1\"/>");
    assert_ne!(reordered, text, "expected tempo serialization pattern");
    assert!(
        verify(reordered.as_bytes()).is_err(),
        "oracle accepted reordered tempo points"
    );
}
