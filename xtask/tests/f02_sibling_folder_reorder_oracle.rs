//! Independent post-edit XML check for one frozen synthetic F-02 sibling-folder reorder.
//! Decodes with test-only quick-xml, never the production importer.
use oxidj_rekordbox::{SiblingFolderReorderRequest, prepare_sibling_folder_reorder};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-sibling-folder-reorder/source.xml");
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
type Folder = (Vec<String>, String, String);
type Playlist = (Vec<String>, String, Vec<String>, String);
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
            n.attr("Entries").into(),
        ));
    }
    for c in &n.children {
        walk(c, path, fs, ps)
    }
    if folder {
        path.pop();
    }
}
fn check(b: &[u8]) -> Result<(), String> {
    let root = decode(b);
    let (mut fs, mut ps) = (vec![], vec![]);
    walk(&root, &mut vec![], &mut fs, &mut ps);
    let ef = vec![
        (vec!["ROOT".into()], "ROOT".into(), "1".into()),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Sets".into(),
            "3".into(),
        ),
        (
            vec!["ROOT".into(), "Sets".into(), "Beta".into()],
            "Beta".into(),
            "1".into(),
        ),
        (
            vec!["ROOT".into(), "Sets".into(), "Alpha".into()],
            "Alpha".into(),
            "2".into(),
        ),
        (
            vec![
                "ROOT".into(),
                "Sets".into(),
                "Alpha".into(),
                "Alpha-nested".into(),
            ],
            "Alpha-nested".into(),
            "1".into(),
        ),
    ];
    if fs != ef {
        return Err(format!("folder order/paths/direct Counts: {fs:?}"));
    }
    let ep = vec![
        (
            vec!["ROOT".into(), "Sets".into(), "Beta".into()],
            "Beta-child".into(),
            vec!["300".into()],
            "1".into(),
        ),
        (
            vec!["ROOT".into(), "Sets".into()],
            "Interleaved".into(),
            vec!["200".into(), "400".into()],
            "2".into(),
        ),
        (
            vec!["ROOT".into(), "Sets".into(), "Alpha".into()],
            "Alpha-child".into(),
            vec!["100".into(), "300".into()],
            "2".into(),
        ),
        (
            vec![
                "ROOT".into(),
                "Sets".into(),
                "Alpha".into(),
                "Alpha-nested".into(),
            ],
            "Deep".into(),
            vec!["400".into()],
            "1".into(),
        ),
    ];
    if ps != ep {
        return Err(format!("playlist paths/order/members/Entries: {ps:?}"));
    }
    let sets = root.named("PLAYLISTS")[0].named("NODE")[0].named("NODE")[0];
    let slots: Vec<_> = sets
        .children
        .iter()
        .map(|n| (n.attr("Type"), n.attr("Name")))
        .collect();
    if slots != [("0", "Beta"), ("1", "Interleaved"), ("0", "Alpha")] {
        return Err(format!("direct mixed child slots differ: {slots:?}"));
    }
    let c = root.named("COLLECTION")[0];
    let tracks: Vec<_> = c.named("TRACK").iter().map(|t| t.attr("TrackID")).collect();
    if c.attr("Entries") != "4" || tracks != ["100", "200", "300", "400"] {
        return Err("collection tracks/count changed".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_folder_reorder_and_rejects_complete_node_mutations() {
    let frozen = SOURCE.to_vec();
    let req = SiblingFolderReorderRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        expected_order: vec!["Alpha".into(), "Beta".into()],
        new_order: vec!["Beta".into(), "Alpha".into()],
    };
    let out = prepare_sibling_folder_reorder(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    check(&out).unwrap();
    let text = String::from_utf8(out).unwrap();
    let beta = text.find("<NODE Type=\"0\" Name=\"Beta\"").unwrap();
    let alpha = text.find("<NODE Type=\"0\" Name=\"Alpha\"").unwrap();
    assert!(beta < alpha);
    let beta_node = "<NODE Type=\"0\" Name=\"Beta\" Count=\"1\"><NODE Type=\"1\" Name=\"Beta-child\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"300\"/></NODE></NODE>";
    let alpha_node = "<NODE Type=\"0\" Name=\"Alpha\" Count=\"2\"><NODE Type=\"1\" Name=\"Alpha-child\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"100\"/><TRACK Key=\"300\"/></NODE><NODE Type=\"0\" Name=\"Alpha-nested\" Count=\"1\"><NODE Type=\"1\" Name=\"Deep\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE></NODE></NODE>";
    let swapped = text
        .replacen(beta_node, "__WHOLE_BETA__", 1)
        .replacen(alpha_node, beta_node, 1)
        .replacen("__WHOLE_BETA__", alpha_node, 1);
    for m in [
        swapped,
        text.replacen("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"", 1),
        text.replacen(
            "Name=\"Alpha-nested\" Count=\"1\"",
            "Name=\"Alpha-nested\" Count=\"0\"",
            1,
        ),
        text.replacen("<TRACK Key=\"100\"/>", "<TRACK Key=\"200\"/>", 1),
        text.replacen("Entries=\"2\"", "Entries=\"1\"", 1),
    ] {
        assert!(
            check(m.as_bytes()).is_err(),
            "oracle accepted mutation: {m}"
        )
    }
}
