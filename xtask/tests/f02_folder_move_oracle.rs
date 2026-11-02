//! Independent post-edit XML check for one frozen synthetic F-02 folder-subtree move.
//! Decodes with test-only quick-xml, never the production importer.
use oxidj_rekordbox::{FolderMoveRequest, prepare_folder_move};
use serde_json as _;
use sha2 as _;

const SOURCE: &[u8] = include_bytes!("fixtures/f02-folder-move/source.xml");
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
type Folder = (Vec<String>, String, String);
type Playlist = (Vec<String>, String, Vec<String>);
fn walk(n: &Node, path: &mut Vec<String>, fs: &mut Vec<Folder>, ps: &mut Vec<Playlist>) {
    let folder = n.name == "NODE" && n.attr("Type") == "0";
    if folder {
        path.push(n.attr("Name").into());
        fs.push((path.clone(), n.attr("Name").into(), n.attr("Count").into()));
    }
    if n.name == "NODE" && n.attr("Type") == "1" {
        ps.push((
            path.clone(),
            n.attr("Name").into(),
            n.named("TRACK")
                .iter()
                .map(|t| t.attr("Key").into())
                .collect(),
        ));
    }
    for c in &n.children {
        walk(c, path, fs, ps)
    }
    if folder {
        path.pop();
    }
}
fn observed(b: &[u8]) -> (Vec<Folder>, Vec<Playlist>, Vec<String>) {
    let root = decode(b);
    let mut f = vec![];
    let mut p = vec![];
    walk(&root, &mut vec![], &mut f, &mut p);
    let tracks = root
        .children
        .iter()
        .find(|n| n.name == "COLLECTION")
        .unwrap()
        .named("TRACK")
        .iter()
        .map(|t| t.attr("TrackID").into())
        .collect();
    (f, p, tracks)
}
pub(crate) fn check(b: &[u8]) -> Result<(), String> {
    let (f, p, t) = observed(b);
    let ef = vec![
        (vec!["ROOT".into()], "ROOT".into(), "2".into()),
        (
            vec!["ROOT".into(), "From".into()],
            "From".into(),
            "1".into(),
        ),
        (
            vec!["ROOT".into(), "Dest".into()],
            "Dest".into(),
            "2".into(),
        ),
        (
            vec!["ROOT".into(), "Dest".into(), "ToMove".into()],
            "ToMove".into(),
            "1".into(),
        ),
        (
            vec![
                "ROOT".into(),
                "Dest".into(),
                "ToMove".into(),
                "Nested".into(),
            ],
            "Nested".into(),
            "1".into(),
        ),
    ];
    let ep = vec![
        (
            vec!["ROOT".into(), "From".into()],
            "Remain".into(),
            vec!["2".into()],
        ),
        (
            vec!["ROOT".into(), "Dest".into()],
            "Existing".into(),
            vec!["3".into()],
        ),
        (
            vec![
                "ROOT".into(),
                "Dest".into(),
                "ToMove".into(),
                "Nested".into(),
            ],
            "Inside".into(),
            vec!["1".into(), "3".into()],
        ),
    ];
    if f != ef {
        return Err(format!("folder paths/order/direct Counts mismatch: {f:?}"));
    }
    if p != ep {
        return Err(format!("playlist paths/order/membership mismatch: {p:?}"));
    }
    if t != ["1", "2", "3"] {
        return Err(format!("collection changed: {t:?}"));
    }
    Ok(())
}
#[test]
fn independently_decodes_folder_subtree_move_and_rejects_order_count_subtree_mutations() {
    let frozen = SOURCE.to_vec();
    let request = FolderMoveRequest {
        folder_index: 1,
        expected_folder_name: "ToMove".into(),
        source_parent_index: 0,
        expected_source_parent_name: "From".into(),
        expected_source_order: vec!["0:ToMove".into(), "1:Remain".into()],
        target_parent_index: 3,
        expected_target_parent_name: "Dest".into(),
        expected_target_order: vec!["1:Existing".into()],
    };
    let output = prepare_folder_move(SOURCE, &request).unwrap().bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "frozen source must remain unchanged"
    );
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let mut order = text.clone();
    let a = order.find("<NODE Type=\"1\" Name=\"Existing\"").unwrap();
    let b = order[a..].find("</NODE>").unwrap() + a + 7;
    let c = order.find("<NODE Type=\"0\" Name=\"ToMove\"").unwrap();
    let d = order[c..].find("</NODE></NODE>").unwrap() + c + 14;
    let x = order[a..b].to_string();
    let y = order[c..d].to_string();
    order.replace_range(c..d, &x);
    order.replace_range(a..b, &y);
    for m in [
        order,
        text.replacen("Name=\"Dest\" Count=\"2\"", "Name=\"Dest\" Count=\"3\"", 1),
        text.replacen(
            "Name=\"Nested\" Count=\"1\"",
            "Name=\"Nested\" Count=\"0\"",
            1,
        ),
        text.replacen("<TRACK Key=\"1\"/>", "<TRACK Key=\"2\"/>", 1),
    ] {
        assert!(
            check(m.as_bytes()).is_err(),
            "oracle accepted mutation: {m}"
        )
    }
}
