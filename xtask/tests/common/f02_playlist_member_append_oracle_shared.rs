// Independent post-edit XML check for synthetic F-02 playlist-member append.
// Uses test-only quick-xml, never the production importer.
pub const SOURCE: &[u8] = include_bytes!("../fixtures/f02-playlist-member-append/source.xml");
#[path = "xml_tree.rs"]
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
        self.children.iter().filter(|n| n.name == key).collect()
    }
}
type Playlist = (Vec<String>, String, String, String, Vec<String>);
fn playlists(node: &Node, path: &mut Vec<String>, out: &mut Vec<Playlist>) {
    let folder = node.name == "NODE" && node.attr("Type") == "0";
    if folder {
        path.push(node.attr("Name").into());
    }
    if node.name == "NODE" && node.attr("Type") == "1" {
        out.push((
            path.clone(),
            node.attr("Name").into(),
            node.attr("KeyType").into(),
            node.attr("Entries").into(),
            node.named("TRACK")
                .iter()
                .map(|t| t.attr("Key").into())
                .collect(),
        ));
    }
    for child in &node.children {
        playlists(child, path, out);
    }
    if folder {
        path.pop();
    }
}
pub fn check(bytes: &[u8]) -> Result<(), String> {
    let root = decode(bytes);
    let mut actual = vec![];
    playlists(&root, &mut vec![], &mut actual);
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
            "3".into(),
            vec!["200".into(), "400".into(), "100".into()],
        ),
    ];
    if actual != expected {
        return Err(format!(
            "playlist path/order/membership/Entries changed: {actual:?}"
        ));
    }
    let root_node = &root.named("PLAYLISTS")[0].named("NODE")[0];
    let sets = &root_node.named("NODE")[0];
    if root_node.attr("Count") != "1"
        || sets.attr("Count") != "2"
        || sets
            .children
            .iter()
            .map(|n| (n.attr("Type"), n.attr("Name")))
            .collect::<Vec<_>>()
            != [("1", "House"), ("1", "Techno")]
    {
        return Err("folder Count or child order changed".into());
    }
    let collection = &root.named("COLLECTION")[0];
    if collection.attr("Entries") != "4"
        || collection
            .named("TRACK")
            .iter()
            .map(|t| (t.attr("TrackID"), t.attr("Name"), t.attr("Location")))
            .collect::<Vec<_>>()
            != [
                ("100", "A", "file://localhost/music/a.flac"),
                ("200", "B", "file://localhost/music/b.flac"),
                ("300", "C", "file://localhost/music/c.flac"),
                ("400", "D", "file://localhost/music/d.flac"),
            ]
    {
        return Err("collection identity/order/metadata changed".into());
    }
    Ok(())
}
