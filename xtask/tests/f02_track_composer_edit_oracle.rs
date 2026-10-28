//! F-09 independent post-edit oracle for guarded F-02 TRACK.Composer editing.
use oxidj_rekordbox::{TrackComposerEditRequest, prepare_track_composer_edit};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-track-composer-edit/source.xml");
#[derive(Debug, PartialEq, Eq)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<N>,
}
fn decode(bytes: &[u8]) -> N {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<N> = vec![];
    let mut root = None;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) => {
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let attrs = e
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
                            a.decoded_and_normalized_value(
                                XmlVersion::Explicit1_0,
                                reader.decoder(),
                            )
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
                    parent.children.push(n);
                } else {
                    root = Some(n);
                }
            }
            Event::End(_) => {
                let n = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(n);
                } else {
                    root = Some(n);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}
fn change_expected_composer(n: &mut N) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let composer = n.attrs.iter_mut().find(|(k, _)| k == "Composer").unwrap();
        assert_eq!(composer.1, "old & composer");
        composer.1 = "new < composer".into();
        return true;
    }
    let mut found = false;
    for child in &mut n.children {
        found |= change_expected_composer(child);
    }
    found
}
fn has_composer(n: &N, id: &str) -> bool {
    (n.name == "TRACK"
        && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == id)
        && n.attrs.iter().any(|(k, _)| k == "Composer"))
        || n.children.iter().any(|c| has_composer(c, id))
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(
        change_expected_composer(&mut expected),
        "fixture lacks TrackID 100 Composer"
    );
    if actual != expected {
        return Err("output differs beyond TrackID 100 Composer".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_track_composer_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackComposerEditRequest {
        track_id: "100".into(),
        expected_old_composer: "old & composer".into(),
        new_composer: "new < composer".into(),
    };
    let output = prepare_track_composer_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    let before = decode(SOURCE);
    assert!(
        has_composer(&before, "100"),
        "fixture must contain target Composer"
    );
    assert!(
        !has_composer(&before, "200"),
        "fixture must preserve absent Composer state"
    );
    check(&output).unwrap();
    for (track_id, expected_old_composer) in [
        ("100", "stale"),
        ("200", "old & composer"),
        ("999", "old & composer"),
    ] {
        let bad = TrackComposerEditRequest {
            track_id: track_id.into(),
            expected_old_composer: expected_old_composer.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_composer_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong-target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Composer=\"new &lt; composer\"", "Composer=\"altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"2\"", "Entries=\"1\""),
        ("Key=\"100\"", "Key=\"101\""),
        (
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Composer=\"unexpected\"",
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
