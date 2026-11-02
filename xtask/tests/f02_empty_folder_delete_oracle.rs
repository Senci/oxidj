//! Independent post-edit XML check for synthetic F-02 empty-folder deletion.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{FolderDeleteRequest, prepare_folder_delete};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-empty-folder-delete/source.xml");
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
            "3".into(),
            vec!["100".into(), "200".into(), "300".into()],
        ),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Techno".into(),
            "0".into(),
            "2".into(),
            vec!["200".into(), "400".into()],
        ),
    ];
    if ps != expected {
        return Err(format!("playlists/order/memberships changed: {ps:?}"));
    }
    let sets = &root.named("PLAYLISTS")[0].named("NODE")[0].named("NODE")[0];
    if sets.attr("Count") != "2"
        || sets
            .children
            .iter()
            .map(|n| (n.attr("Type"), n.attr("Name")))
            .collect::<Vec<_>>()
            != [("1", "House"), ("1", "Techno")]
    {
        return Err("parent Count or child order incorrect".into());
    }
    if root.named("PLAYLISTS")[0].named("NODE")[0].attr("Count") != "1" {
        return Err("root Count changed".into());
    }
    let c = &root.named("COLLECTION")[0];
    if c.attr("Entries") != "4"
        || c.named("TRACK")
            .iter()
            .map(|t| t.attr("TrackID"))
            .collect::<Vec<_>>()
            != ["100", "200", "300", "400"]
    {
        return Err("collection changed".into());
    }
    Ok(())
}

#[test]
fn independently_decodes_empty_folder_deletion_and_rejects_structural_mutations() {
    let frozen = SOURCE.to_vec();
    let req = FolderDeleteRequest {
        folder_index: 1,
        expected_name: "Empty".into(),
    };
    let output = prepare_folder_delete(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    for mutated in [
        text.replacen("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"1\"", 1),
        text.replacen("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"", 1),
        text.replacen(
            "Name=\"House\" Entries=\"3\"",
            "Name=\"House\" Entries=\"2\"",
            1,
        ),
        text.replacen(
            "Name=\"Techno\" Entries=\"2\"",
            "Name=\"Wrong\" Entries=\"2\"",
            1,
        ),
        text.replacen(
            "Name=\"Techno\" Entries=\"2\"",
            "Name=\"Techno\" Entries=\"1\"",
            1,
        ),
        text.replacen("Key=\"200\"", "Key=\"999\"", 1),
    ] {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
}
