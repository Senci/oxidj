//! Independent post-edit XML check for synthetic F-02 folder rename.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{FolderRenameRequest, prepare_folder_rename};
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-folder-rename/source.xml");
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
            .find(|(n, _)| n == key)
            .map_or("", |(_, v)| v)
    }
    fn set_attr(&mut self, key: &str, value: &str) {
        self.attrs.iter_mut().find(|(n, _)| n == key).unwrap().1 = value.into();
    }
}
fn decode(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = vec![];
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
                let n = Node {
                    name,
                    attrs,
                    children: vec![],
                };
                if let Some(p) = stack.last_mut() {
                    p.children.push(n)
                } else {
                    root = Some(n)
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
fn verify(bytes: &[u8], expected: &Node) -> Result<(), String> {
    let actual = decode(bytes);
    if &actual != expected {
        return Err("decoded XML differs from folder-rename-only expectation".into());
    }
    Ok(())
}
#[test]
fn independently_decodes_guarded_folder_rename_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let req = FolderRenameRequest {
        folder_index: 0,
        expected_old_name: "Sets".into(),
        new_name: "Sets & Practice".into(),
    };
    let output = prepare_folder_rename(SOURCE, &req).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice());
    let mut expected = decode(SOURCE);
    let playlists = &mut expected
        .children
        .iter_mut()
        .find(|n| n.name == "PLAYLISTS")
        .unwrap()
        .children[0];
    let sets = &mut playlists
        .children
        .iter_mut()
        .find(|n| n.name == "NODE" && n.attr("Name") == "Sets")
        .unwrap();
    sets.set_attr("Name", "Sets & Practice");
    verify(&output, &expected).unwrap();
    let text = String::from_utf8(output).unwrap();
    for mutated in [
        text.replacen("Name=\"Sets &amp; Practice\"", "Name=\"Wrong\"", 1),
        text.replacen("Name=\"Sets &amp; Practice\"", "Name=\"Sets\"", 1),
        text.replacen("Name=\"House\"", "Name=\"Changed\"", 1),
        text.replacen("Key=\"200\"", "Key=\"999\"", 1),
        text.replacen("Count=\"2\"", "Count=\"3\"", 1),
        text.replacen("Name=\"Techno\"", "Name=\"House\"", 1),
    ] {
        assert!(
            verify(mutated.as_bytes(), &expected).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
}
