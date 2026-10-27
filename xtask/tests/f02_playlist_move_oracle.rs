//! Independent post-edit XML check for one frozen synthetic F-02 playlist move.
//! This test uses a test-only quick-xml tree reader, never the production importer.
use oxidj_rekordbox::{PlaylistMoveRequest, prepare_playlist_move};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;

const SOURCE: &[u8] = include_bytes!("fixtures/f02-playlist-move/source.xml");

#[derive(Debug)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}
impl Node {
    fn attr(&self, key: &str) -> &str {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .unwrap()
            .1
            .as_str()
    }
    fn children_named(&self, key: &str) -> Vec<&Node> {
        self.children.iter().filter(|n| n.name == key).collect()
    }
}
fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = Vec::new();
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
                stack.push(Node {
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
                let node = Node {
                    name,
                    attrs,
                    children: vec![],
                };
                if let Some(p) = stack.last_mut() {
                    p.children.push(node);
                } else {
                    root = Some(node);
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
fn folders<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    if node.name == "NODE" && node.attr("Type") == "0" {
        out.push(node);
    }
    for c in &node.children {
        folders(c, out);
    }
}
type Playlist = (Vec<String>, String, Vec<String>);
type Snapshot = (Vec<Playlist>, Vec<(String, String)>);
fn playlists(node: &Node, path: &mut Vec<String>, out: &mut Vec<Playlist>) {
    if node.name == "NODE" && node.attr("Type") == "0" {
        path.push(node.attr("Name").to_owned());
    }
    if node.name == "NODE" && node.attr("Type") == "1" {
        let members = node
            .children_named("TRACK")
            .iter()
            .map(|t| t.attr("Key").to_owned())
            .collect();
        out.push((path.clone(), node.attr("Name").to_owned(), members));
    }
    for c in &node.children {
        playlists(c, path, out);
    }
    if node.name == "NODE" && node.attr("Type") == "0" {
        path.pop();
    }
}
fn observed(bytes: &[u8]) -> Result<Snapshot, String> {
    let root = decode(bytes);
    let mut fs = vec![];
    folders(&root, &mut fs);
    let mut ps = vec![];
    playlists(&root, &mut vec![], &mut ps);
    let counts = fs
        .iter()
        .map(|f| (f.attr("Name").to_owned(), f.attr("Count").to_owned()))
        .collect();
    Ok((ps, counts))
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let (ps, counts) = observed(bytes)?;
    let expected = vec![
        (
            vec!["ROOT".into(), "Sets".into()],
            "Techno".into(),
            vec!["200".into(), "400".into()],
        ),
        (
            vec!["ROOT".into(), "Target".into()],
            "Other".into(),
            vec!["400".into()],
        ),
        (
            vec!["ROOT".into(), "Target".into()],
            "House".into(),
            vec!["100".into(), "200".into(), "300".into()],
        ),
    ];
    if ps != expected {
        return Err(format!("playlist path/order/membership mismatch: {ps:?}"));
    }
    let expected_counts = vec![
        ("ROOT".into(), "2".into()),
        ("Sets".into(), "1".into()),
        ("Target".into(), "2".into()),
    ];
    if counts != expected_counts {
        return Err(format!("direct Counts mismatch: {counts:?}"));
    }
    Ok(())
}
#[test]
fn independently_decodes_playlist_move_and_rejects_order_count_membership_mutations() {
    let source = String::from_utf8(SOURCE.to_vec()).unwrap();
    let request = PlaylistMoveRequest {
        playlist_index: 0,
        expected_playlist_name: "House".into(),
        expected_members: vec!["100".into(), "200".into(), "300".into()],
        source_parent_index: 0,
        expected_source_parent_name: "Sets".into(),
        target_parent_index: 1,
        expected_target_parent_name: "Target".into(),
        expected_target_order: vec!["Other".into()],
    };
    let output = prepare_playlist_move(SOURCE, &request).unwrap().bytes;
    assert_eq!(
        SOURCE,
        source.as_bytes(),
        "adapter must not mutate frozen source"
    );
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    for mutated in [
        text.replacen("Name=\"Other\"", "Name=\"House\"", 1),
        text.replacen("Name=\"Sets\" Count=\"1\"", "Name=\"Sets\" Count=\"2\"", 1),
        text.replacen("<TRACK Key=\"100\"/>", "<TRACK Key=\"999\"/>", 1),
    ] {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
}
