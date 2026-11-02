//! Independent post-edit XML check for synthetic F-02 empty-folder creation.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{FolderCreateRequest, prepare_folder_create};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-empty-folder-create/source.xml");
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
    if sets.attr("Count") != "3"
        || sets
            .children
            .iter()
            .map(|n| (n.attr("Type"), n.attr("Name")))
            .collect::<Vec<_>>()
            != [("1", "House"), ("1", "Techno"), ("0", "Practice")]
    {
        return Err(format!(
            "parent Count/order incorrect: count={} children={:?}",
            sets.attr("Count"),
            sets.children
                .iter()
                .map(|n| (n.attr("Type"), n.attr("Name")))
                .collect::<Vec<_>>()
        ));
    }
    let created = &sets.children[2];
    if created.attr("Count") != "0" || !created.children.is_empty() {
        return Err("created folder is not empty with Count=0".into());
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
fn independently_decodes_empty_folder_creation_and_rejects_structural_mutations() {
    let frozen = SOURCE.to_vec();
    let req = FolderCreateRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        name: "Practice".into(),
    };
    let output = prepare_folder_create(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    for mutated in [
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"2\"", 1),
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"", 1),
        text.replacen(
            "Name=\"Practice\" Count=\"0\"",
            "Name=\"Wrong\" Count=\"0\"",
            1,
        ),
        text.replacen(
            "Name=\"Practice\" Count=\"0\"",
            "Name=\"Practice\" Count=\"1\"",
            1,
        ),
        {
            let folder = "      <NODE Type=\"0\" Name=\"Practice\" Count=\"0\"/>\n";
            text.replacen(folder, "", 1)
                .replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"2\"", 1)
                .replacen("Name=\"ROOT\" Count=\"1\"", "Name=\"ROOT\" Count=\"2\"", 1)
                .replacen(
                    "</NODE>\n    </NODE>",
                    "</NODE>\n      <NODE Type=\"0\" Name=\"Practice\" Count=\"0\"/>\n    </NODE>",
                    1,
                )
        },
        {
            let folder = "      <NODE Type=\"0\" Name=\"Practice\" Count=\"0\"/>\n";
            text.replacen(folder, "", 1).replacen(
                "<NODE Type=\"1\" Name=\"House\"",
                &format!("{folder}<NODE Type=\"1\" Name=\"House\""),
                1,
            )
        },
        text.replacen("Key=\"200\"", "Key=\"999\"", 1),
    ] {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
}
