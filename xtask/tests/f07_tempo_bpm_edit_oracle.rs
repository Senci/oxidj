//! F-09 independent post-edit oracle for one guarded synthetic F-07 TEMPO BPM edit.
use oxidj_rekordbox::{TempoEditRequest, prepare_tempo_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/rekordbox-a1/F-07/source.bin");
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
fn attr<'a>(n: &'a N, key: &str) -> &'a str {
    n.attrs
        .iter()
        .find(|(name, _)| name == key)
        .unwrap()
        .1
        .as_str()
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
    assert_eq!(
        track.attrs.iter().find(|(n, _)| n == "TrackID").unwrap().1,
        "700"
    );
    let mark = track
        .children
        .iter()
        .find(|n| n.name == "POSITION_MARK")
        .unwrap();
    assert_eq!(attr(mark, "Type"), "0");
    assert_eq!(attr(mark, "Start"), "5");
    assert_eq!(attr(mark, "Name"), "Intro");
    assert_eq!(attr(mark, "Num"), "0");
    assert!(!mark.attrs.iter().any(|(name, _)| name == "End"));
    let mut tempos: Vec<_> = track
        .children
        .iter_mut()
        .filter(|n| n.name == "TEMPO")
        .collect();
    assert_eq!(
        tempos.len(),
        2,
        "fixture must contain exactly two tempo points"
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
    tempos[0]
        .attrs
        .iter_mut()
        .find(|(n, _)| n == "Bpm")
        .unwrap()
        .1 = "125".into();
    if actual != expected {
        return Err("output differs beyond target Bpm".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_bpm_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let req = TempoEditRequest {
        track_id: "700".into(),
        tempo_index: 0,
        expected_old_bpm: "120".into(),
        new_bpm: "125".into(),
    };
    let output = prepare_tempo_edit(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();
    let wrong_ordinal = TempoEditRequest {
        tempo_index: 1,
        expected_old_bpm: "128".into(),
        new_bpm: "125".into(),
        ..req.clone()
    };
    let wrong_output = prepare_tempo_edit(SOURCE, &wrong_ordinal).unwrap().bytes;
    assert!(
        check(&wrong_output).is_err(),
        "oracle accepted wrong ordinal"
    );
    for (tempo_index, expected_old_bpm) in [(0, "119"), (1, "120")] {
        let bad = TempoEditRequest {
            tempo_index,
            expected_old_bpm: expected_old_bpm.into(),
            ..req.clone()
        };
        assert!(prepare_tempo_edit(SOURCE, &bad).is_err());
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Bpm=\"125\"", "Bpm=\"126\""),
        ("Inizio=\"0\"", "Inizio=\"1\""),
        ("Bpm=\"128\"", "Bpm=\"127\""),
        ("Metro=\"4/4\"", "Metro=\"3/4\""),
        ("Battito=\"1\"", "Battito=\"2\""),
        ("Start=\"5\"", "Start=\"6\""),
        ("Type=\"0\"", "Type=\"1\""),
        ("Num=\"0\"", "Num=\"2\""),
        ("Name=\"Intro\"", "Name=\"Changed\""),
        ("Inizio=\"10\"", "Inizio=\"11\""),
        ("Entries=\"1\"", "Entries=\"2\""),
        ("TrackID=\"700\"", "TrackID=\"701\""),
        ("Name=\"ROOT\"", "Name=\"Changed\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text);
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation {from}"
        );
    }
    let reordered = text.replace(
        "<TEMPO Inizio=\"0\" Bpm=\"125\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>",
        "<TEMPO Inizio=\"10\" Bpm=\"128\" Metro=\"4/4\" Battito=\"1\"/>\n      <TEMPO Inizio=\"0\" Bpm=\"125\" Metro=\"4/4\" Battito=\"1\"/>",
    );
    assert_ne!(reordered, text);
    assert!(
        check(reordered.as_bytes()).is_err(),
        "oracle accepted reordered tempo points"
    );
}
