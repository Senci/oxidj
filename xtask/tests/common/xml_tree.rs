// Test-only quick-xml tree decoder shared by independent F-02 oracles.
// Never the production importer. `decode` keeps elements only (local names,
// decoded and normalized attribute values, document order); `decode_with_text`
// additionally keeps text/CDATA children at their position among elements.
#![allow(dead_code)]
use quick_xml::{
    Reader, XmlVersion,
    events::{BytesStart, Event},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// Text/CDATA children as (number of preceding element children, text).
    /// Always empty for `decode`.
    pub texts: Vec<(usize, String)>,
}

fn node(e: &BytesStart<'_>, r: &Reader<&[u8]>) -> Node {
    Node {
        name: String::from_utf8(e.local_name().as_ref().to_vec()).unwrap(),
        attrs: e
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
            .collect(),
        children: vec![],
        texts: vec![],
    }
}

fn decode_tree(bytes: &[u8], keep_text: bool) -> Node {
    let mut r = Reader::from_reader(bytes);
    let mut stack: Vec<Node> = vec![];
    let mut root = None;
    let mut close = |n: Node, stack: &mut Vec<Node>| {
        if let Some(p) = stack.last_mut() {
            p.children.push(n)
        } else {
            root = Some(n)
        }
    };
    loop {
        match r.read_event().unwrap() {
            Event::Start(e) => stack.push(node(&e, &r)),
            Event::Empty(e) => close(node(&e, &r), &mut stack),
            Event::End(_) => {
                let n = stack.pop().unwrap();
                close(n, &mut stack)
            }
            Event::Text(e) if keep_text => {
                if let Some(n) = stack.last_mut() {
                    let at = n.children.len();
                    n.texts.push((at, e.decode().unwrap().into_owned()))
                }
            }
            Event::CData(e) if keep_text => {
                if let Some(n) = stack.last_mut() {
                    let at = n.children.len();
                    n.texts.push((at, e.decode().unwrap().into_owned()))
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.unwrap()
}

/// Element-only decoded tree.
pub fn decode(bytes: &[u8]) -> Node {
    decode_tree(bytes, false)
}

/// Decoded tree that also keeps text/CDATA children in document order.
pub fn decode_with_text(bytes: &[u8]) -> Node {
    decode_tree(bytes, true)
}
