//! F-09 independent post-edit oracle for one guarded synthetic F-07 mark deletion.
use oxidj_rekordbox::{PositionMarkDeleteRequest, prepare_position_mark_delete};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f07-position-mark-delete/source.xml");
#[derive(Debug)]
struct N {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<N>,
}
impl N {
    fn attr(&self, k: &str) -> &str {
        self.attrs.iter().find(|(n, _)| n == k).unwrap().1.as_str()
    }
    fn named(&self, k: &str) -> Vec<&N> {
        self.children.iter().filter(|n| n.name == k).collect()
    }
}
fn decode(b: &[u8]) -> N {
    let mut r = Reader::from_reader(b);
    let mut s: Vec<N> = vec![];
    let mut root = None;
    loop {
        match r.read_event().unwrap() {
            Event::Start(e) => {
                let n = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let a = e
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
                    name: n,
                    attrs: a,
                    children: vec![],
                })
            }
            Event::Empty(e) => {
                let n = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                let a = e
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
                let v = N {
                    name: n,
                    attrs: a,
                    children: vec![],
                };
                if let Some(p) = s.last_mut() {
                    p.children.push(v)
                } else {
                    root = Some(v)
                }
            }
            Event::End(_) => {
                let v = s.pop().unwrap();
                if let Some(p) = s.last_mut() {
                    p.children.push(v)
                } else {
                    root = Some(v)
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}
fn check(b: &[u8]) -> Result<(), String> {
    let r = decode(b);
    let c = &r.named("COLLECTION")[0];
    let t = &c.named("TRACK")[0];
    if c.attr("Entries") != "1"
        || t.attr("TrackID") != "700"
        || t.attr("Name") != "Synthetic"
        || t.attr("Artist") != "Artist"
        || t.attr("Location") != "file://localhost/700.mp3"
    {
        return Err("collection/track metadata changed".into());
    }
    let ts = t.named("TEMPO");
    if ts.len() != 2
        || ts
            .iter()
            .map(|x| {
                (
                    x.attr("Inizio"),
                    x.attr("Bpm"),
                    x.attr("Metro"),
                    x.attr("Battito"),
                )
            })
            .collect::<Vec<_>>()
            != [("0", "120", "4/4", "1"), ("16", "132", "7/8", "3")]
    {
        return Err("TEMPO records/order changed".into());
    }
    let m = t.named("POSITION_MARK");
    if m.len() != 1
        || m[0].attr("Type") != "0"
        || m[0].attr("Start") != "0"
        || m[0].attr("Name") != "Keep me"
        || m[0].attr("End") != "0"
    {
        return Err("retained mark incorrect".into());
    }
    let p = &r.named("PLAYLISTS")[0];
    let root = &p.named("NODE")[0];
    let sets = &root.named("NODE")[0];
    let h = &sets.named("NODE")[0];
    if root.attr("Count") != "1"
        || sets.attr("Count") != "1"
        || h.attr("Name") != "House"
        || h.attr("Entries") != "1"
        || h.named("TRACK")[0].attr("Key") != "700"
    {
        return Err("playlist tree changed".into());
    }
    Ok(())
}
#[test]
fn independently_verifies_guarded_deletion_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let req = PositionMarkDeleteRequest {
        track_id: "700".into(),
        mark_index: 0,
        expected_type: "4".into(),
        expected_start: "5".into(),
        expected_name: "Delete me".into(),
        expected_end: Some("6".into()),
    };
    let out = prepare_position_mark_delete(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&out).unwrap();
    let text = String::from_utf8(out).unwrap();
    for bad in [
        text.replace("Name=\"Keep me\"", "Name=\"Delete me\""),
        text.replace("Name=\"Keep me\"", "Name=\"Changed\""),
        text.replace("Bpm=\"132\"", "Bpm=\"131\""),
        text.replace("Key=\"700\"", "Key=\"701\""),
    ] {
        assert!(
            check(bad.as_bytes()).is_err(),
            "oracle accepted mutation: {bad}"
        )
    }
}
