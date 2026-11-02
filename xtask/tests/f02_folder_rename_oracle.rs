//! Independent post-edit XML check for synthetic F-02 folder rename.
//! Uses test-only quick-xml, never the production importer.
use oxidj_rekordbox::{FolderRenameRequest, prepare_folder_rename};
use serde_json as _;
use sha2 as _;
const SOURCE: &[u8] = include_bytes!("fixtures/f02-folder-rename/source.xml");
#[path = "common/xml_tree.rs"]
mod xml_tree;
use xml_tree::{Node, decode};
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
fn verify(bytes: &[u8], expected: &Node) -> Result<(), String> {
    let actual = decode(bytes);
    if &actual != expected {
        return Err("decoded XML differs from folder-rename-only expectation".into());
    }
    Ok(())
}
/// Check UI-exported bytes against the independent frozen F-02 semantic baseline.
pub fn check_ui_export(bytes: &[u8], new_name: &str) -> Result<(), String> {
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
    sets.set_attr("Name", new_name);
    verify(bytes, &expected)
}

/// Ensure the same actual output is rejected after a semantic mutation.
pub fn check_ui_export_mutation_rejected(bytes: &[u8], new_name: &str) {
    check_ui_export(bytes, new_name).expect("unmutated UI export satisfies independent oracle");
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
    sets.set_attr("Name", new_name);
    let source = String::from_utf8(bytes.to_vec()).unwrap();
    let mutated = source.replacen(
        &format!("Name=\"{new_name}\""),
        "Name=\"Oracle mutation\"",
        1,
    );
    assert_ne!(
        source, mutated,
        "mutation target exists in actual UI output"
    );
    assert!(
        verify(mutated.as_bytes(), &expected).is_err(),
        "independent oracle must reject mutation to actual UI output"
    );
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
