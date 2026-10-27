//! Independent post-edit XML check for synthetic F-02 playlist rename.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistRenameRequest, prepare_playlist_rename};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-playlist-rename/source.xml");
#[derive(Debug, Clone, PartialEq, Eq)]
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
        self.children
            .iter()
            .filter(|node| node.name == key)
            .collect()
    }
    fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs
            .iter_mut()
            .find(|(name, _)| name == key)
            .unwrap()
            .1 = value.into();
    }
}
fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = vec![];
    let mut root = None;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(event) => {
                let name = String::from_utf8(event.local_name().as_ref().to_vec()).unwrap();
                let attrs = event
                    .attributes()
                    .map(|item| {
                        let a = item.unwrap();
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
            Event::Empty(event) => {
                let name = String::from_utf8(event.local_name().as_ref().to_vec()).unwrap();
                let attrs = event
                    .attributes()
                    .map(|item| {
                        let a = item.unwrap();
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
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node)
                } else {
                    root = Some(node)
                }
            }
            Event::End(_) => {
                let node = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node)
                } else {
                    root = Some(node)
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}
fn verify(bytes: &[u8], expected: &Node) -> Result<(), String> {
    let actual = decode(bytes);
    if &actual != expected {
        return Err("decoded XML tree differs from rename-only expectation".into());
    }
    let sets = &actual.named("PLAYLISTS")[0].named("NODE")[0].named("NODE")[0];
    let playlists = sets
        .children
        .iter()
        .filter(|n| n.name == "NODE" && n.attr("Type") == "1")
        .map(|n| {
            (
                n.attr("Name"),
                n.attr("KeyType"),
                n.attr("Entries"),
                n.named("TRACK")
                    .iter()
                    .map(|t| t.attr("Key"))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    if playlists
        != [
            ("Summer & Sun", "0", "3", vec!["100", "200", "300"]),
            ("Techno", "0", "2", vec!["200", "400"]),
        ]
    {
        return Err("playlist name/order/metadata/membership mismatch".into());
    }
    if sets.attr("Count") != "2" {
        return Err("folder Count changed".into());
    }
    let collection = &actual.named("COLLECTION")[0];
    if collection.attr("Entries") != "4"
        || collection
            .named("TRACK")
            .iter()
            .map(|t| (t.attr("TrackID"), t.attr("Location")))
            .collect::<Vec<_>>()
            != [
                ("100", "file://localhost/music/a.flac"),
                ("200", "file://localhost/music/b.flac"),
                ("300", "file://localhost/music/c.flac"),
                ("400", "file://localhost/music/d.flac"),
            ]
    {
        return Err("collection changed".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_guarded_playlist_rename_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = PlaylistRenameRequest {
        playlist_index: 0,
        expected_old_name: "House".into(),
        new_name: "Summer & Sun".into(),
    };
    let output = prepare_playlist_rename(SOURCE, &request).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    let mut expected = decode(SOURCE);
    let playlists = &mut expected
        .children
        .iter_mut()
        .find(|n| n.name == "PLAYLISTS")
        .unwrap()
        .children[0]
        .children[0]
        .children[0];
    playlists.set_attr("Name", "Summer & Sun");
    verify(&output, &expected).unwrap();
    let text = String::from_utf8(output).unwrap();
    let mutations = [
        text.replacen("Name=\"Summer &amp; Sun\"", "Name=\"House\"", 1),
        text.replacen("Name=\"Techno\"", "Name=\"Summer &amp; Sun\"", 1),
        text.replacen("Key=\"200\"", "Key=\"999\"", 1),
        text.replacen("Name=\"Summer &amp; Sun\"", "Name=\"TEMP\"", 1)
            .replacen("Name=\"Techno\"", "Name=\"Summer &amp; Sun\"", 1)
            .replacen("Name=\"TEMP\"", "Name=\"Techno\"", 1),
        text.replacen("Count=\"2\"", "Count=\"3\"", 1),
    ];
    for mutated in mutations {
        assert!(
            verify(mutated.as_bytes(), &expected).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
}
