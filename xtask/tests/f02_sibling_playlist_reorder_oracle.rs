//! Independent post-edit oracle for one frozen synthetic F-02 sibling-playlist reorder.
use oxidj_rekordbox::{SiblingPlaylistReorderRequest, prepare_sibling_playlist_reorder};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-sibling-playlist-reorder/source.xml");
#[derive(Debug, PartialEq, Eq)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}
impl Node {
    fn attr(&self, k: &str) -> &str {
        self.attrs.iter().find(|(n, _)| n == k).unwrap().1.as_str()
    }
    fn named(&self, k: &str) -> Vec<&Node> {
        self.children.iter().filter(|n| n.name == k).collect()
    }
}
fn decode(b: &[u8]) -> Node {
    let mut r = Reader::from_reader(b);
    let mut s: Vec<Node> = vec![];
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
                s.push(Node {
                    name,
                    attrs,
                    children: vec![],
                })
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
                let n = Node {
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
fn verify(b: &[u8]) -> Result<(), String> {
    let root = decode(b);
    let c = root.named("COLLECTION")[0];
    let tracks: Vec<_> = c
        .named("TRACK")
        .iter()
        .map(|t| (t.attr("TrackID"), t.attr("Name"), t.attr("Location")))
        .collect();
    if tracks
        != [
            ("100", "A", "file://localhost/music/a.flac"),
            ("200", "B", "file://localhost/music/b.flac"),
            ("300", "C", "file://localhost/music/c.flac"),
            ("400", "D", "file://localhost/music/d.flac"),
        ]
        || c.attr("Entries") != "4"
    {
        return Err("collection changed".into());
    }
    let sets = root.named("PLAYLISTS")[0].named("NODE")[0].named("NODE")[0];
    if sets.attr("Count") != "3" {
        return Err("Sets Count changed".into());
    }
    let kids: Vec<_> = sets
        .children
        .iter()
        .map(|n| (n.attr("Type"), n.attr("Name")))
        .collect();
    if kids != [("1", "Techno"), ("0", "Interleaved"), ("1", "House")] {
        return Err(format!("slot order differs: {kids:?}"));
    }
    let folder = &sets.children[1];
    if folder.attr("Count") != "1" || folder.named("NODE")[0].named("TRACK")[0].attr("Key") != "400"
    {
        return Err("interleaved folder changed".into());
    }
    let members = |p: &Node| {
        p.named("TRACK")
            .iter()
            .map(|t| t.attr("Key").to_owned())
            .collect::<Vec<_>>()
    };
    let techno = &sets.children[0];
    let house = &sets.children[2];
    if techno.attr("Entries") != "2"
        || members(techno) != ["200", "400"]
        || house.attr("Entries") != "3"
        || members(house) != ["100", "200", "300"]
    {
        return Err("playlist memberships or Entries changed".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_sibling_reorder_and_rejects_output_mutations() {
    let frozen = SOURCE.to_vec();
    let req = SiblingPlaylistReorderRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        expected_order: vec!["House".into(), "Techno".into()],
        new_order: vec!["Techno".into(), "House".into()],
    };
    let output = prepare_sibling_playlist_reorder(SOURCE, &req)
        .unwrap()
        .bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    verify(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let swapped = text
        .replace("Name=\"Techno\"", "Name=\"TEMP\"")
        .replace("Name=\"House\"", "Name=\"Techno\"")
        .replace("Name=\"TEMP\"", "Name=\"House\"");
    for m in [
        swapped,
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"", 1),
        text.replacen("Entries=\"2\"", "Entries=\"1\"", 1),
        text.replacen("<TRACK Key=\"200\"/>", "<TRACK Key=\"999\"/>", 1),
    ] {
        assert!(verify(m.as_bytes()).is_err(), "oracle accepted mutation");
    }
}
