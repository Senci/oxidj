//! Independent post-edit XML check for synthetic F-02 playlist-member removal.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistMemberDeleteRequest, prepare_playlist_member_delete};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-playlist-member-delete/source.xml");
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
            .find(|(name, _)| name == key)
            .map_or("", |(_, value)| value)
    }
    fn named(&self, key: &str) -> Vec<&Node> {
        self.children.iter().filter(|n| n.name == key).collect()
    }
}
fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = vec![];
    let mut root = None;
    loop {
        let event = reader.read_event().unwrap();
        let (name, attrs, empty) = match event {
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
                (name, attrs, false)
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
                (name, attrs, true)
            }
            Event::End(_) => {
                let node = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let node = Node {
            name,
            attrs,
            children: vec![],
        };
        if empty {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(node);
            } else {
                root = Some(node);
            }
        } else {
            stack.push(node);
        }
    }
    root.unwrap()
}
type Playlist = (Vec<String>, String, String, String, Vec<String>);
fn playlists(node: &Node, path: &mut Vec<String>, out: &mut Vec<Playlist>) {
    let folder = node.name == "NODE" && node.attr("Type") == "0";
    if folder {
        path.push(node.attr("Name").into());
    }
    if node.name == "NODE" && node.attr("Type") == "1" {
        out.push((
            path.clone(),
            node.attr("Name").into(),
            node.attr("KeyType").into(),
            node.attr("Entries").into(),
            node.named("TRACK")
                .iter()
                .map(|t| t.attr("Key").into())
                .collect(),
        ));
    }
    for child in &node.children {
        playlists(child, path, out);
    }
    if folder {
        path.pop();
    }
}
fn check(bytes: &[u8]) -> Result<(), String> {
    let root = decode(bytes);
    let mut actual = vec![];
    playlists(&root, &mut vec![], &mut actual);
    let expected = vec![
        (
            vec!["ROOT".into(), "Sets".into()],
            "House".into(),
            "0".into(),
            "2".into(),
            vec!["100".into(), "300".into()],
        ),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Techno".into(),
            "0".into(),
            "2".into(),
            vec!["200".into(), "400".into()],
        ),
    ];
    if actual != expected {
        return Err(format!(
            "playlist path/order/membership/Entries changed: {actual:?}"
        ));
    }
    let root_node = &root.named("PLAYLISTS")[0].named("NODE")[0];
    let sets = &root_node.named("NODE")[0];
    if root_node.attr("Count") != "1"
        || sets.attr("Count") != "2"
        || sets
            .children
            .iter()
            .map(|n| (n.attr("Type"), n.attr("Name")))
            .collect::<Vec<_>>()
            != [("1", "House"), ("1", "Techno")]
    {
        return Err("folder Count or child order changed".into());
    }
    let collection = &root.named("COLLECTION")[0];
    if collection.attr("Entries") != "4"
        || collection
            .named("TRACK")
            .iter()
            .map(|t| (t.attr("TrackID"), t.attr("Name"), t.attr("Location")))
            .collect::<Vec<_>>()
            != [
                ("100", "A", "file://localhost/music/a.flac"),
                ("200", "B", "file://localhost/music/b.flac"),
                ("300", "C", "file://localhost/music/c.flac"),
                ("400", "D", "file://localhost/music/d.flac"),
            ]
    {
        return Err("collection identity/order/metadata changed".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_member_removal_and_rejects_semantic_mutations() {
    let frozen = SOURCE.to_vec();
    let request = PlaylistMemberDeleteRequest {
        playlist_index: 0,
        track_id: "200".into(),
        expected_order: vec!["100".into(), "200".into(), "300".into()],
    };
    let output = prepare_playlist_member_delete(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "adapter must not mutate source bytes"
    );
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let mutations = [
        text.replacen("Key=\"100\"", "Key=\"200\"", 1),
        text.replacen("Key=\"400\"", "Key=\"999\"", 1),
        text.replacen("Entries=\"2\"", "Entries=\"1\"", 1),
        text.replacen("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"1\"", 1),
        text.replacen("TrackID=\"200\"", "TrackID=\"999\"", 1),
        text.replacen("Name=\"Techno\"", "Name=\"Other\"", 1),
        text.replacen("Name=\"House\"", "Name=\"Other\"", 1)
            .replacen("Name=\"Techno\"", "Name=\"House\"", 1)
            .replacen("Name=\"Other\"", "Name=\"Techno\"", 1),
    ];
    for mutated in mutations {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
    let wrong_playlist_output = prepare_playlist_member_delete(
        SOURCE,
        &PlaylistMemberDeleteRequest {
            playlist_index: 1,
            track_id: "200".into(),
            expected_order: vec!["200".into(), "400".into()],
        },
    )
    .unwrap()
    .bytes;
    assert!(
        check(&wrong_playlist_output).is_err(),
        "oracle accepted removal from Techno instead of House"
    );
    assert!(
        prepare_playlist_member_delete(
            SOURCE,
            &PlaylistMemberDeleteRequest {
                playlist_index: 1,
                track_id: "200".into(),
                expected_order: vec!["100".into(), "200".into(), "300".into()]
            }
        )
        .is_err()
    );
    assert!(
        prepare_playlist_member_delete(
            SOURCE,
            &PlaylistMemberDeleteRequest {
                playlist_index: 0,
                track_id: "999".into(),
                expected_order: vec!["100".into(), "200".into(), "300".into()]
            }
        )
        .is_err()
    );
}
