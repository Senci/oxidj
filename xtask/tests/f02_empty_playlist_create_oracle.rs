//! Independent post-edit XML check for synthetic F-02 empty-playlist creation.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{PlaylistCreateRequest, prepare_playlist_create};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-empty-playlist-create/source.xml");
#[path = "common/xml_tree.rs"]
mod xml_tree;
use xml_tree::{Node, decode};
impl Node {
    fn attr(&self, k: &str) -> &str {
        self.attrs.iter().find(|(n, _)| n == k).unwrap().1.as_str()
    }
    fn named(&self, k: &str) -> Vec<&Node> {
        self.children.iter().filter(|n| n.name == k).collect()
    }
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
pub(crate) fn check(b: &[u8]) -> Result<(), String> {
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
