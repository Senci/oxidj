//! Independent post-edit XML check for synthetic F-02 playlist rename.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistRenameRequest, prepare_playlist_rename};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-playlist-rename/source.xml");
#[path = "common/xml_tree.rs"]
mod xml_tree;
use xml_tree::{Node, decode};
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

/// Check bytes produced at an external boundary against the independent frozen F-02 expectation.
pub(crate) fn check(bytes: &[u8]) -> Result<(), String> {
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
    verify(bytes, &expected)
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
    check(&output).unwrap();
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
