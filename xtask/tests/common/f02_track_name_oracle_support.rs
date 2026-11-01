//! Shared independent F-02 Track Name output oracle for CLI and adapter tests.
use quick_xml::{Reader, XmlVersion, events::Event};

const SOURCE: &[u8] = include_bytes!("../fixtures/rekordbox-a1/F-02/source.bin");

#[derive(Debug, PartialEq, Eq)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
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
                    .map(|attr| {
                        let attr = attr.unwrap();
                        (
                            String::from_utf8(attr.key.local_name().as_ref().to_vec()).unwrap(),
                            attr.decoded_and_normalized_value(
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
                    .map(|attr| {
                        let attr = attr.unwrap();
                        (
                            String::from_utf8(attr.key.local_name().as_ref().to_vec()).unwrap(),
                            attr.decoded_and_normalized_value(
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
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::End(_) => {
                let node = stack.pop().unwrap();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}

fn set_target_name(node: &mut Node, value: &str) -> bool {
    if node.name == "TRACK"
        && node
            .attrs
            .iter()
            .any(|(key, value)| key == "TrackID" && value == "100")
    {
        let name = node
            .attrs
            .iter_mut()
            .find(|(key, _)| key == "Name")
            .unwrap();
        assert_eq!(name.1, "A");
        name.1 = value.into();
        return true;
    }
    let mut found = false;
    for child in &mut node.children {
        found |= set_target_name(child, value);
    }
    found
}

pub fn check(bytes: &[u8]) -> Result<(), String> {
    let actual = decode(bytes);
    let mut expected = decode(SOURCE);
    assert!(
        set_target_name(&mut expected, "New < title"),
        "fixture lacks TrackID 100"
    );
    if actual != expected {
        return Err("output differs beyond TrackID 100 Name".into());
    }
    Ok(())
}
