//! Independent post-edit XML check for synthetic F-02 empty-playlist creation.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistCreateRequest, prepare_playlist_create};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-empty-playlist-create/source.xml");
#[derive(Debug)]
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
type P = (Vec<String>, String, String, String, Vec<String>);
fn playlists(n: &Node, path: &mut Vec<String>, out: &mut Vec<P>) {
    let folder = n.name == "NODE" && n.attr("Type") == "0";
    if folder {
        path.push(n.attr("Name").into())
    }
    if n.name == "NODE" && n.attr("Type") == "1" {
        out.push((
            path.clone(),
            n.attr("Name").into(),
            n.attr("KeyType").into(),
            n.attr("Entries").into(),
            n.named("TRACK")
                .iter()
                .map(|t| t.attr("Key").into())
                .collect(),
        ))
    }
    for c in &n.children {
        playlists(c, path, out)
    }
    if folder {
        path.pop();
    }
}
fn check(b: &[u8]) -> Result<(), String> {
    let root = decode(b);
    let mut ps = vec![];
    playlists(&root, &mut vec![], &mut ps);
    let expected = vec![
        (
            vec!["ROOT".into(), "Sets".into()],
            "House".into(),
            "0".into(),
            "1".into(),
            vec!["100".into()],
        ),
        (
            vec!["ROOT".into(), "Sets".into(), "Nested".into()],
            "Deep".into(),
            "0".into(),
            "1".into(),
            vec!["200".into()],
        ),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Practice".into(),
            "0".into(),
            "0".into(),
            vec![],
        ),
    ];
    if ps != expected {
        return Err(format!("playlist order/fields/members: {ps:?}"));
    }
    let sets = &root.named("PLAYLISTS")[0].named("NODE")[0].named("NODE")[0];
    if sets.attr("Count") != "3"
        || sets
            .children
            .iter()
            .map(|n| (n.attr("Type"), n.attr("Name")))
            .collect::<Vec<_>>()
            != [("1", "House"), ("0", "Nested"), ("1", "Practice")]
    {
        return Err("selected parent Count/order changed incorrectly".into());
    }
    if root.named("PLAYLISTS")[0].named("NODE")[0].attr("Count") != "1" {
        return Err("unselected root Count changed".into());
    }
    let c = &root.named("COLLECTION")[0];
    if c.attr("Entries") != "2"
        || c.named("TRACK")
            .iter()
            .map(|t| t.attr("TrackID"))
            .collect::<Vec<_>>()
            != ["100", "200"]
    {
        return Err("collection changed".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_empty_playlist_creation_and_rejects_structural_mutations() {
    let frozen = SOURCE.to_vec();
    let req = PlaylistCreateRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        name: "Practice".into(),
    };
    let output = prepare_playlist_create(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    for mutated in [
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"2\"", 1),
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"", 1),
        text.replacen(
            "Name=\"Practice\" KeyType=\"0\" Entries=\"0\"",
            "Name=\"Practice\" KeyType=\"1\" Entries=\"0\"",
            1,
        ),
        text.replacen(
            "Name=\"Practice\" KeyType=\"0\" Entries=\"0\"",
            "Name=\"Practice\" KeyType=\"0\" Entries=\"1\"",
            1,
        ),
        text.replacen(
            "Name=\"House\" Entries=\"1\"",
            "Name=\"House\" Entries=\"0\"",
            1,
        ),
        text.replacen(
            "Name=\"Sets\" Count=\"3\"><NODE Type=\"1\" Name=\"House\"",
            "Name=\"Sets\" Count=\"3\"><NODE Type=\"1\" Name=\"Practice\"",
            1,
        ),
    ] {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        )
    }
}
