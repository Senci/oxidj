//! Independent post-edit XML check for synthetic F-02 playlist-member insertion.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistMemberInsertRequest, prepare_playlist_member_insert};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-playlist-member-insert/source.xml");
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
            "3".into(),
            vec!["100".into(), "200".into(), "300".into()],
        ),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Techno".into(),
            "0".into(),
            "3".into(),
            vec!["200".into(), "100".into(), "400".into()],
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
fn independently_decodes_member_insertion_and_rejects_semantic_mutations() {
    let frozen = SOURCE.to_vec();
    let request = PlaylistMemberInsertRequest {
        playlist_index: 1,
        expected_order: vec!["200".into(), "400".into()],
        track_id: "100".into(),
        position: 1,
    };
    let output = prepare_playlist_member_insert(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "adapter must not mutate source bytes"
    );
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let target_start = text.find("Name=\"Techno\"").unwrap();
    let target = &text[target_start..];
    let key_100 = target.find("<TRACK Key=\"100\"/>").unwrap() + target_start;
    let key_400 = target.find("<TRACK Key=\"400\"/>").unwrap() + target_start;
    let member = "<TRACK Key=\"100\"/>";
    let mut reordered = text.clone();
    reordered.replace_range(key_400..key_400 + member.len(), "<TRACK Key=\"100\"/>");
    reordered.replace_range(key_100..key_100 + member.len(), "<TRACK Key=\"400\"/>");
    let mutations = [
        reordered,
        text.replacen(
            "Name=\"Techno\" Entries=\"3\"",
            "Name=\"Techno\" Entries=\"2\"",
            1,
        ),
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
    let wrong_playlist_output = prepare_playlist_member_insert(
        SOURCE,
        &PlaylistMemberInsertRequest {
            playlist_index: 0,
            track_id: "400".into(),
            expected_order: vec!["100".into(), "200".into(), "300".into()],
            position: 1,
        },
    )
    .unwrap()
    .bytes;
    assert!(
        check(&wrong_playlist_output).is_err(),
        "oracle accepted insertion into wrong playlist"
    );
    let wrong_position_output = prepare_playlist_member_insert(
        SOURCE,
        &PlaylistMemberInsertRequest {
            position: 2,
            ..request.clone()
        },
    )
    .unwrap()
    .bytes;
    assert!(
        check(&wrong_position_output).is_err(),
        "oracle accepted insertion at wrong position"
    );
    assert!(
        prepare_playlist_member_insert(
            SOURCE,
            &PlaylistMemberInsertRequest {
                playlist_index: 1,
                track_id: "100".into(),
                expected_order: vec!["999".into()],
                position: 1,
            }
        )
        .is_err()
    );
    assert!(
        prepare_playlist_member_insert(
            SOURCE,
            &PlaylistMemberInsertRequest {
                playlist_index: 0,
                track_id: "999".into(),
                expected_order: vec!["200".into(), "400".into()],
                position: 1,
            }
        )
        .is_err()
    );
}
