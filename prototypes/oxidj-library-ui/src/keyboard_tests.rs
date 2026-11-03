//! UI acceptance tests (moved verbatim from main.rs).

use super::*;

#[path = "../../../xtask/tests/f02_empty_folder_create_oracle.rs"]
mod f02_empty_folder_create_oracle;
#[path = "../../../xtask/tests/f02_empty_folder_delete_oracle.rs"]
mod f02_empty_folder_delete_oracle;
#[path = "../../../xtask/tests/f02_empty_playlist_create_oracle.rs"]
mod f02_empty_playlist_create_oracle;
#[path = "../../../xtask/tests/f02_folder_move_oracle.rs"]
mod f02_folder_move_oracle;
#[path = "../../../xtask/tests/f02_folder_rename_oracle.rs"]
mod f02_folder_rename_oracle;
#[path = "../../../xtask/tests/f02_playlist_clear_oracle.rs"]
mod f02_playlist_clear_oracle;
#[path = "../../../xtask/tests/f02_playlist_delete_oracle.rs"]
mod f02_playlist_delete_oracle;
#[path = "../../../xtask/tests/f02_playlist_member_delete_oracle.rs"]
mod f02_playlist_member_delete_oracle;
#[path = "../../../xtask/tests/f02_playlist_move_oracle.rs"]
mod f02_playlist_move_oracle;
#[path = "../../../xtask/tests/f02_playlist_rename_oracle.rs"]
mod f02_playlist_rename_oracle;
#[path = "../../../xtask/tests/f02_sibling_folder_reorder_oracle.rs"]
mod f02_sibling_folder_reorder_oracle;
#[path = "../../../xtask/tests/f02_sibling_playlist_reorder_oracle.rs"]
mod f02_sibling_playlist_reorder_oracle;
#[path = "../../../xtask/tests/f02_track_attribute_edit_oracle.rs"]
mod f02_track_attribute_edit_oracle;
#[path = "../../../xtask/tests/common/f02_track_name_oracle_support.rs"]
mod f02_track_name_oracle_support;

fn app() -> LibraryApp {
    LibraryApp::new(
        PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    )
}

#[test]
fn sibling_folder_reorder_ui_exports_verified_bytes_and_guards_refusals() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-sibling-folder-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-sibling-folder-reorder/source.xml"
    );
    let source = root.join("source.xml");
    std::fs::copy(fixture, &source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let parent = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Sets")
        .unwrap();
    app.xml_folder_sibling_parent = Some(parent.ordinal);
    app.xml_folder_sibling_order = "Beta, Alpha".into();
    app.prepare_xml_sibling_folder_reorder();
    assert!(
        app.xml_folder_sibling_pending.is_some(),
        "{:?}",
        app.xml_edit_error
    );
    let pending = app.xml_folder_sibling_pending.as_ref().unwrap();
    assert_eq!(pending.expected_order, ["Alpha", "Beta"]);
    assert_eq!(pending.new_order, ["Beta", "Alpha"]);
    let dest = root.join("output.xml");
    app.xml_folder_sibling_export_path = dest.display().to_string();
    app.export_xml_sibling_folder_reorder();
    let bytes = std::fs::read(&dest).unwrap();
    f02_sibling_folder_reorder_oracle::check(&bytes).unwrap();
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let set = reopened.folders.iter().find(|f| f.name == "Sets").unwrap();
    let children: Vec<_> = reopened
        .folders
        .iter()
        .filter(|f| f.path.len() > 2 && f.path[..f.path.len() - 1] == set.path)
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(children, ["Beta", "Alpha"]);
    assert!(
        reopened
            .playlists
            .iter()
            .any(|p| p.name == "Interleaved" && p.tracks == ["200", "400"])
    );
    assert_eq!(
        std::fs::read(&source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    let mutated = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"");
    assert!(f02_sibling_folder_reorder_oracle::check(mutated.as_bytes()).is_err());
    app.export_xml_sibling_folder_reorder();
    assert_eq!(std::fs::read(&dest).unwrap(), bytes);

    let source2 = root.join("stale.xml");
    std::fs::copy(fixture, &source2).unwrap();
    let mut stale = LibraryApp::new(
        PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    stale.xml_path = source2.display().to_string();
    stale.preview_xml();
    stale.xml_folder_sibling_parent = Some(
        stale
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Sets")
            .unwrap()
            .ordinal,
    );
    stale.xml_folder_sibling_order = "Beta, Alpha".into();
    stale.prepare_xml_sibling_folder_reorder();
    stale.xml_document_source = Some(root.join("other.xml"));
    stale.xml_folder_sibling_export_path = root.join("stale-output.xml").display().to_string();
    stale.export_xml_sibling_folder_reorder();
    assert!(stale.xml_folder_sibling_pending.is_none());
    assert!(!root.join("stale-output.xml").exists());
    let setup = |label: &str, order: &str| {
        let path = root.join(format!("{label}.xml"));
        std::fs::copy(fixture, &path).unwrap();
        let mut candidate = LibraryApp::new(
            PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        candidate.xml_path = path.display().to_string();
        candidate.preview_xml();
        candidate.xml_folder_sibling_parent = Some(
            candidate
                .xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .folders
                .iter()
                .find(|f| f.name == "Sets")
                .unwrap()
                .ordinal,
        );
        candidate.xml_folder_sibling_order = order.into();
        candidate.prepare_xml_sibling_folder_reorder();
        (candidate, path)
    };
    for stale_kind in ["parent", "order"] {
        let (mut candidate, source) = setup("changed", "Beta, Alpha");
        if stale_kind == "parent" {
            candidate
                .xml_preview
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()
                .folders
                .iter_mut()
                .find(|f| f.name == "Sets")
                .unwrap()
                .name = "Changed".into();
        } else {
            let folders = &mut candidate
                .xml_preview
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()
                .folders;
            let alpha = folders.iter().position(|f| f.name == "Alpha").unwrap();
            let beta = folders.iter().position(|f| f.name == "Beta").unwrap();
            folders.swap(alpha, beta);
        }
        let output = root.join(format!("{stale_kind}-output.xml"));
        candidate.xml_folder_sibling_export_path = output.display().to_string();
        candidate.export_xml_sibling_folder_reorder();
        assert!(candidate.xml_folder_sibling_pending.is_none());
        assert!(!output.exists());
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
    }
    let (mut root_parent, root_source) = setup("root-parent", "Beta, Alpha");
    root_parent.xml_folder_sibling_parent = Some(0);
    root_parent.prepare_xml_sibling_folder_reorder();
    assert!(root_parent.xml_folder_sibling_pending.is_none());
    assert_eq!(
        std::fs::read(root_source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    for invalid in ["Beta, Beta", "Beta, Missing", "Beta"] {
        let (candidate, path) = setup("invalid", invalid);
        assert!(
            candidate.xml_folder_sibling_pending.is_none(),
            "accepted {invalid}"
        );
        assert_eq!(
            std::fs::read(path).unwrap(),
            std::fs::read(fixture).unwrap()
        );
    }
    let (mut cancelled, cancel_source) = setup("cancel", "Beta, Alpha");
    cancelled.cancel_xml_sibling_folder_reorder();
    let cancelled_output = root.join("cancel-output.xml");
    cancelled.xml_folder_sibling_export_path = cancelled_output.display().to_string();
    cancelled.export_xml_sibling_folder_reorder();
    assert!(!cancelled_output.exists());
    assert_eq!(
        std::fs::read(cancel_source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    let (mut alias, alias_source) = setup("alias", "Beta, Alpha");
    alias.xml_folder_sibling_export_path = alias_source.display().to_string();
    alias.export_xml_sibling_folder_reorder();
    assert_eq!(
        std::fs::read(alias_source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    let (mut existing, existing_source) = setup("existing", "Beta, Alpha");
    let protected = root.join("protected.xml");
    std::fs::write(&protected, b"keep").unwrap();
    existing.xml_folder_sibling_export_path = protected.display().to_string();
    existing.export_xml_sibling_folder_reorder();
    assert_eq!(std::fs::read(protected).unwrap(), b"keep");
    assert_eq!(
        std::fs::read(existing_source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_subtree_move_ui_prepares_cancels_exports_and_reopens_oracle_checked_bytes() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-move-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let index = preview
        .folders
        .iter()
        .position(|f| f.name == "ToMove")
        .unwrap();
    let target = preview
        .folders
        .iter()
        .find(|f| f.name == "Dest")
        .unwrap()
        .ordinal;
    app.xml_folder_move_selection = Some(index);
    app.xml_folder_move_destination = Some(target);
    app.prepare_xml_folder_move(index);
    let pending = app.xml_pending_folder_move.as_ref().unwrap();
    assert_eq!(pending.old_path, ["ROOT", "From", "ToMove"]);
    assert_eq!(pending.new_path, ["Dest", "ToMove"]);
    assert_eq!(
        app.xml_pending_folder_move
            .as_ref()
            .unwrap()
            .expected_source_order,
        ["0:ToMove", "1:Remain"]
    );
    assert_eq!(
        app.xml_pending_folder_move
            .as_ref()
            .unwrap()
            .expected_target_order,
        ["1:Existing"]
    );
    let prepared = pending.prepared.bytes().to_vec();
    app.cancel_xml_folder_move();
    assert!(app.xml_pending_folder_move.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    app.prepare_xml_folder_move(index);
    let output = root.join("moved.xml");
    app.xml_folder_move_export_path = output.display().to_string();
    app.export_xml_folder_move();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_folder_move_oracle::check(&bytes).unwrap();
    let moved_subtree = b"<NODE Type=\"0\" Name=\"ToMove\" Count=\"1\"><NODE Type=\"0\" Name=\"Nested\" Count=\"1\"><NODE Type=\"1\" Name=\"Inside\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"1\"/><TRACK Key=\"3\"/></NODE></NODE></NODE>";
    assert!(
        bytes
            .windows(moved_subtree.len())
            .any(|window| window == moved_subtree)
    );
    let collection = b"<COLLECTION Entries=\"3\"><TRACK TrackID=\"1\" Name=\"one\" Location=\"file://localhost/one\"/><TRACK TrackID=\"2\" Name=\"two\" Location=\"file://localhost/two\"/><TRACK TrackID=\"3\" Name=\"three\" Location=\"file://localhost/three\"/></COLLECTION>";
    assert!(
        bytes
            .windows(collection.len())
            .any(|window| window == collection)
    );
    assert_eq!(
        direct_xml_typed_child_order(&output, &["From".into()]).unwrap(),
        ["1:Remain"]
    );
    assert_eq!(
        direct_xml_typed_child_order(&output, &["Dest".into()]).unwrap(),
        ["1:Existing", "0:ToMove"]
    );
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert!(
        reopened
            .folders
            .iter()
            .any(|f| f.path == ["ROOT", "Dest", "ToMove", "Nested"])
    );
    assert!(
        reopened
            .playlists
            .iter()
            .any(|p| p.path == ["Dest", "ToMove", "Nested", "Inside"]
                && p.name == "Inside"
                && p.tracks == ["1", "3"])
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.name == "From")
            .unwrap()
            .count
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.name == "Dest")
            .unwrap()
            .count
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.name == "Nested")
            .unwrap()
            .count
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.path == ["ROOT"])
            .unwrap()
            .count
            .as_deref(),
        Some("2")
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_subtree_move_ui_rejects_stale_typed_child_order_and_preserves_aliases() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-move-stale-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
    );
    std::fs::copy(fixture, &source).unwrap();
    let app_factory = || app();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let index = preview
        .folders
        .iter()
        .position(|f| f.name == "ToMove")
        .unwrap();
    let target = preview
        .folders
        .iter()
        .find(|f| f.name == "Dest")
        .unwrap()
        .ordinal;
    app.xml_folder_move_selection = Some(index);
    app.xml_folder_move_destination = Some(target);
    app.prepare_xml_folder_move(index);
    let original = std::fs::read_to_string(&source).unwrap();
    let left = "<NODE Type=\"0\" Name=\"ToMove\" Count=\"1\"><NODE Type=\"0\" Name=\"Nested\" Count=\"1\"><NODE Type=\"1\" Name=\"Inside\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"1\"/><TRACK Key=\"3\"/></NODE></NODE></NODE>";
    let right = "<NODE Type=\"1\" Name=\"Remain\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"2\"/></NODE>";
    assert!(original.contains(&format!("{left}{right}")));
    std::fs::write(
        &source,
        original.replacen(&format!("{left}{right}"), &format!("{right}{left}"), 1),
    )
    .unwrap();
    let changed = std::fs::read(&source).unwrap();
    app.xml_folder_move_export_path = root.join("stale.xml").display().to_string();
    app.export_xml_folder_move();
    assert!(app.xml_pending_folder_move.is_none());
    assert!(!root.join("stale.xml").exists());
    assert_eq!(std::fs::read(&source).unwrap(), changed);
    app.xml_document_source = Some(source.clone());
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let alias_index = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .position(|f| f.name == "ToMove")
        .unwrap();
    let dest = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Dest")
        .unwrap()
        .ordinal;
    app.xml_folder_move_selection = Some(alias_index);
    app.xml_folder_move_destination = Some(dest);
    app.prepare_xml_folder_move(alias_index);
    app.xml_folder_move_export_path = source.display().to_string();
    app.export_xml_folder_move();
    assert_eq!(std::fs::read(&source).unwrap(), changed);

    for stale_kind in ["source", "selection", "source-parent", "target-parent"] {
        let stale_source = root.join(format!("{stale_kind}-source.xml"));
        std::fs::copy(fixture, &stale_source).unwrap();
        let mut candidate = app_factory();
        candidate.xml_path = stale_source.display().to_string();
        candidate.preview_xml();
        let preview = candidate.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let stale_index = preview
            .folders
            .iter()
            .position(|folder| folder.name == "ToMove")
            .unwrap();
        let stale_target = preview
            .folders
            .iter()
            .find(|folder| folder.name == "Dest")
            .unwrap()
            .ordinal;
        candidate.xml_folder_move_selection = Some(stale_index);
        candidate.xml_folder_move_destination = Some(stale_target);
        candidate.prepare_xml_folder_move(stale_index);
        assert!(candidate.xml_pending_folder_move.is_some());
        match stale_kind {
            "source" => candidate.xml_document_source = Some(root.join("other.xml")),
            "selection" => candidate.xml_folder_move_selection = Some(stale_index + 1),
            "source-parent" | "target-parent" => {
                let parent_name = if stale_kind == "source-parent" {
                    "From"
                } else {
                    "Dest"
                };
                candidate
                    .xml_preview
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .folders
                    .iter_mut()
                    .find(|folder| folder.name == parent_name)
                    .unwrap()
                    .name = "Changed".into();
            }
            _ => unreachable!(),
        }
        let output = root.join(format!("{stale_kind}-output.xml"));
        candidate.xml_folder_move_export_path = output.display().to_string();
        candidate.export_xml_folder_move();
        assert!(candidate.xml_pending_folder_move.is_none(), "{stale_kind}");
        assert!(!output.exists(), "{stale_kind}");
        assert_eq!(
            std::fs::read(stale_source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_subtree_move_ui_refuses_typed_child_order_read_errors() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-move-read-error-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
    );

    for fail_during_export in [false, true] {
        let source = root.join(if fail_during_export {
            "export-source.xml"
        } else {
            "prepare-source.xml"
        });
        std::fs::copy(fixture, &source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|folder| folder.name == "ToMove")
            .unwrap();
        let target = preview
            .folders
            .iter()
            .find(|folder| folder.name == "Dest")
            .unwrap()
            .ordinal;
        app.xml_folder_move_selection = Some(index);
        app.xml_folder_move_destination = Some(target);

        if fail_during_export {
            app.prepare_xml_folder_move(index);
            assert!(app.xml_pending_folder_move.is_some());
        }
        std::fs::write(&source, b"<NODE Type='0' Name=\"broken>").unwrap();
        let output = root.join(format!("output-{fail_during_export}.xml"));
        app.xml_folder_move_export_path = output.display().to_string();
        if fail_during_export {
            app.export_xml_folder_move();
            assert!(app.xml_pending_folder_move.is_none());
        } else {
            app.prepare_xml_folder_move(index);
            assert!(app.xml_pending_folder_move.is_none());
        }
        assert!(app.xml_edit_error.as_deref().is_some_and(
            |error| error.contains("cannot read") && error.contains("child order")
        ));
        assert!(!output.exists());
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_subtree_move_ui_refuses_invalid_sources_parents_and_duplicate_children() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-move-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
    );
    let fixture_xml = std::fs::read_to_string(fixture).unwrap();
    let setup = |name: &str, xml: &str| {
        let source = root.join(format!("{name}.xml"));
        std::fs::write(&source, xml).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|folder| folder.name == "ToMove")
            .unwrap();
        let source_parent = preview
            .folders
            .iter()
            .find(|folder| folder.name == "From")
            .unwrap()
            .ordinal;
        let target = preview
            .folders
            .iter()
            .find(|folder| folder.name == "Dest")
            .unwrap()
            .ordinal;
        let nested = preview
            .folders
            .iter()
            .find(|folder| folder.name == "Nested")
            .unwrap()
            .ordinal;
        app.xml_folder_move_selection = Some(index);
        app.xml_folder_move_destination = Some(target);
        (app, source, index, source_parent, target, nested)
    };

    for (name, source_index, destination) in [
        ("root-source", Some(0), None),
        ("out-of-range", Some(usize::MAX), None),
        ("same-parent", None, Some("source")),
        ("cycle", None, Some("descendant")),
        ("root-target", None, Some("root")),
    ] {
        let (mut candidate, source, index, source_parent, _target, nested) =
            setup(name, &fixture_xml);
        if let Some(destination) = destination {
            candidate.xml_folder_move_destination = Some(match destination {
                "source" => source_parent,
                "descendant" => nested,
                "root" => {
                    candidate
                        .xml_preview
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .folders
                        .iter()
                        .find(|folder| folder.path.len() == 1)
                        .unwrap()
                        .ordinal
                }
                _ => unreachable!(),
            });
        }
        let selected = source_index.unwrap_or(index);
        candidate.prepare_xml_folder_move(selected);
        assert!(candidate.xml_pending_folder_move.is_none(), "{name}");
        assert_eq!(std::fs::read(source).unwrap(), fixture_xml.as_bytes());
    }

    let duplicate_xml = fixture_xml.replacen(
        "</NODE><NODE Type=\"1\" Name=\"Remain\"",
        "</NODE><NODE Type=\"0\" Name=\"ToMove\" Count=\"0\"/><NODE Type=\"1\" Name=\"Remain\"",
        1,
    );
    assert_ne!(duplicate_xml, fixture_xml);
    let (mut duplicate, duplicate_source, index, _, _, _) =
        setup("duplicate-child", &duplicate_xml);
    duplicate.prepare_xml_folder_move(index);
    assert!(duplicate.xml_pending_folder_move.is_none());
    assert_eq!(
        std::fs::read(duplicate_source).unwrap(),
        duplicate_xml.as_bytes()
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_subtree_move_ui_supports_self_closing_destination_and_protects_existing_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-move-self-closing-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
    );
    let fixture_xml = std::fs::read_to_string(fixture).unwrap();
    let xml = fixture_xml.replace(
        "<NODE Type=\"0\" Name=\"Dest\" Count=\"1\"><NODE Type=\"1\" Name=\"Existing\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"3\"/></NODE></NODE>",
        "<NODE Type=\"0\" Name=\"Dest\" Count=\"0\"/>",
    );
    assert_ne!(xml, fixture_xml);
    let source = root.join("source.xml");
    std::fs::write(&source, xml.as_bytes()).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let index = preview
        .folders
        .iter()
        .position(|folder| folder.name == "ToMove")
        .unwrap();
    let target = preview
        .folders
        .iter()
        .find(|folder| folder.name == "Dest")
        .unwrap();
    let target_ordinal = target.ordinal;
    assert_eq!(target.count.as_deref(), Some("0"));
    app.xml_folder_move_selection = Some(index);
    app.xml_folder_move_destination = Some(target_ordinal);
    app.prepare_xml_folder_move(index);
    let pending = app.xml_pending_folder_move.as_ref().unwrap();
    assert_eq!(pending.expected_target_order, Vec::<String>::new());
    assert_eq!(pending.target_count.as_deref(), Some("0"));

    let protected = root.join("protected.xml");
    std::fs::write(&protected, b"keep").unwrap();
    app.xml_folder_move_export_path = protected.display().to_string();
    app.export_xml_folder_move();
    assert_eq!(std::fs::read(&protected).unwrap(), b"keep");
    assert_eq!(std::fs::read(&source).unwrap(), xml.as_bytes());

    let output = root.join("moved.xml");
    app.xml_folder_move_export_path = output.display().to_string();
    app.export_xml_folder_move();
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert!(
        reopened
            .folders
            .iter()
            .any(|folder| folder.path == ["ROOT", "Dest", "ToMove"])
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|folder| folder.name == "Dest")
            .unwrap()
            .count
            .as_deref(),
        Some("1")
    );
    assert_eq!(std::fs::read(&source).unwrap(), xml.as_bytes());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_move_ui_exports_prepared_bytes_and_runs_independent_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-move-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let source_tracks: Vec<_> = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .tracks
        .iter()
        .map(|track| (track.id.clone(), track.name.clone(), track.location.clone()))
        .collect();
    let target = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Target")
        .unwrap()
        .ordinal;
    app.xml_selected_playlist = Some(0);
    app.xml_move_destination = Some(target);
    app.prepare_xml_playlist_move(0);
    let prepared = app
        .xml_pending_move
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(
        app.xml_pending_move.as_ref().unwrap().old_path,
        ["Sets", "House"]
    );
    assert_eq!(
        app.xml_pending_move.as_ref().unwrap().new_path,
        ["Target", "House"]
    );
    app.cancel_xml_playlist_move();
    assert!(app.xml_pending_move.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    app.prepare_xml_playlist_move(0);
    app.xml_move_export_path = source.display().to_string();
    app.export_xml_playlist_move();
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "source alias cannot overwrite"
    );
    assert!(app.xml_pending_move.is_none());
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.prepare_xml_playlist_move(0);
    app.xml_move_export_path = existing.display().to_string();
    app.export_xml_playlist_move();
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    assert!(app.xml_pending_move.is_none());
    app.prepare_xml_playlist_move(0);
    let out = root.join("moved.xml");
    app.xml_move_export_path = out.display().to_string();
    app.export_xml_playlist_move();
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes, prepared);
    f02_playlist_move_oracle::check(&bytes).unwrap();
    let reopened = oxidj_application::preview_rekordbox_xml(&out).unwrap();
    assert!(
        reopened
            .playlists
            .iter()
            .any(|p| p.path == ["Target", "House"]
                && p.name == "House"
                && p.tracks == ["100", "200", "300"]),
        "reopened: {:?}",
        reopened.playlists
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.name == "Sets")
            .unwrap()
            .count
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .count
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        reopened
            .tracks
            .iter()
            .map(|track| (track.id.clone(), track.name.clone(), track.location.clone()))
            .collect::<Vec<_>>(),
        source_tracks,
        "collection tracks must be unchanged"
    );
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "source bytes must remain unchanged"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_move_ui_invalidates_source_selection_and_destination_changes() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-move-stale-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let target = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Target")
        .unwrap()
        .ordinal;
    let sets = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Sets")
        .unwrap()
        .ordinal;
    app.xml_selected_playlist = Some(0);
    app.xml_move_destination = Some(target);
    app.prepare_xml_playlist_move(0);
    app.select_xml_playlist(1);
    assert!(
        app.xml_pending_move.is_none(),
        "selection change cancels pending move"
    );
    app.xml_selected_playlist = Some(0);
    app.xml_move_destination = Some(target);
    app.prepare_xml_playlist_move(0);
    app.xml_move_destination = Some(sets);
    app.xml_move_export_path = root.join("destination-stale.xml").display().to_string();
    app.export_xml_playlist_move();
    assert!(app.xml_pending_move.is_none());
    assert!(!root.join("destination-stale.xml").exists());
    app.xml_move_destination = Some(target);
    app.prepare_xml_playlist_move(0);
    let mut changed = std::fs::read(&source).unwrap();
    changed.push(b' ');
    std::fs::write(&source, changed.clone()).unwrap();
    app.xml_move_export_path = root.join("source-stale.xml").display().to_string();
    app.export_xml_playlist_move();
    assert!(app.xml_pending_move.is_none());
    assert!(!root.join("source-stale.xml").exists());
    assert_eq!(std::fs::read(&source).unwrap(), changed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_move_ui_refuses_invalid_destinations_without_mutation() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-move-refuse-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
    );
    let source = root.join("source.xml");
    std::fs::copy(fixture, &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut candidate = app();
    candidate.xml_path = source.display().to_string();
    candidate.preview_xml();
    let folders = &candidate
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders;
    let sets = folders.iter().find(|f| f.name == "Sets").unwrap().ordinal;
    let root_ordinal = folders.iter().find(|f| f.path.len() == 1).unwrap().ordinal;
    candidate.xml_move_destination = Some(sets);
    candidate.prepare_xml_playlist_move(0);
    assert!(candidate.xml_pending_move.is_none(), "same-parent refused");
    candidate.xml_move_destination = Some(root_ordinal);
    candidate.prepare_xml_playlist_move(0);
    assert!(candidate.xml_pending_move.is_none(), "root refused");
    candidate.xml_move_destination = Some(usize::MAX);
    candidate.prepare_xml_playlist_move(0);
    assert!(candidate.xml_pending_move.is_none(), "out-of-range refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(!root.join("output.xml").exists());

    let duplicate = root.join("duplicate.xml");
    let mut bytes = original.clone();
    let marker = b"<NODE Type=\"1\" Name=\"Other\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE>";
    let marker_at = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    bytes.splice(marker_at..marker_at, marker.iter().copied());
    std::fs::write(&duplicate, &bytes).unwrap();
    let mut ambiguous = app();
    ambiguous.xml_path = duplicate.display().to_string();
    ambiguous.preview_xml();
    let target = ambiguous
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Target")
        .unwrap()
        .ordinal;
    ambiguous.xml_move_destination = Some(target);
    ambiguous.prepare_xml_playlist_move(0);
    assert!(
        ambiguous.xml_pending_move.is_none(),
        "ambiguous duplicate child path refused"
    );
    assert_eq!(std::fs::read(&duplicate).unwrap(), bytes);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_move_ui_refuses_source_and_destination_order_changes() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-move-order-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
    ))
    .unwrap();
    let house = "<NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\"><TRACK Key=\"100\"/><TRACK Key=\"200\"/><TRACK Key=\"300\"/></NODE>";
    let techno = "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"200\"/><TRACK Key=\"400\"/></NODE>";
    let other = "<NODE Type=\"1\" Name=\"Other\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE>";
    let extra = "<NODE Type=\"1\" Name=\"Extra\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"300\"/></NODE>";
    for (label, initial, before, after) in [
        (
            "source",
            String::from_utf8(fixture.clone()).unwrap(),
            format!("{house}{techno}"),
            format!("{techno}{house}"),
        ),
        (
            "destination",
            String::from_utf8(fixture.clone()).unwrap().replace(
                "Name=\"Target\" Count=\"1\"><NODE",
                &format!("Name=\"Target\" Count=\"2\">{extra}<NODE"),
            ),
            format!("{extra}{other}"),
            format!("{other}{extra}"),
        ),
    ] {
        std::fs::write(&source, initial).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let target = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .ordinal;
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        assert!(
            app.xml_pending_move.is_some(),
            "valid initial preparation for {label}"
        );
        let mut xml = std::fs::read_to_string(&source).unwrap();
        assert!(
            xml.contains(&before),
            "expected original direct-child block order for {label}"
        );
        xml = xml.replacen(&before, &after, 1);
        std::fs::write(&source, xml.as_bytes()).unwrap();
        let changed = std::fs::read(&source).unwrap();
        app.xml_move_export_path = root
            .join(format!("{label}-order.xml"))
            .display()
            .to_string();
        app.export_xml_playlist_move();
        assert!(app.xml_pending_move.is_none());
        assert!(!root.join(format!("{label}-order.xml")).exists());
        assert_eq!(std::fs::read(&source).unwrap(), changed);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_move_ui_accepts_mixed_folder_and_playlist_children() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-move-mixed-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
    ))
    .unwrap();
    let xml = String::from_utf8(fixture).unwrap().replace(
        "Name=\"Target\" Count=\"1\"><NODE",
        "Name=\"Target\" Count=\"2\"><NODE Type=\"0\" Name=\"Sub\" Count=\"0\"/><NODE",
    );
    std::fs::write(&source, xml).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let target = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .find(|f| f.name == "Target")
        .unwrap()
        .ordinal;
    assert_eq!(
        direct_xml_node_order(&source, &["Target".into()]).unwrap(),
        ["Sub", "Other"]
    );
    app.xml_selected_playlist = Some(0);
    app.xml_move_destination = Some(target);
    app.prepare_xml_playlist_move(0);
    assert!(
        app.xml_pending_move.is_some(),
        "mixed direct child order should prepare successfully"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sibling_playlist_reorder_ui_export_bytes_pass_existing_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-sibling-order-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_sibling_parent = Some(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|folder| folder.name == "Sets")
            .unwrap()
            .ordinal,
    );
    app.xml_sibling_order = "Techno, House".into();
    app.prepare_xml_sibling_playlist_reorder();
    assert_eq!(
        app.xml_sibling_pending.as_ref().unwrap().expected_order,
        ["House", "Techno"]
    );
    app.xml_sibling_export_path = root.join("reordered.xml").display().to_string();
    app.export_xml_sibling_playlist_reorder();
    let bytes = std::fs::read(root.join("reordered.xml")).unwrap();
    f02_sibling_playlist_reorder_oracle::verify(&bytes).unwrap();
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let reopened_order: Vec<_> = reopened
        .playlists
        .iter()
        .filter(|playlist| playlist.path.len() == 2 && playlist.path[0] == "Sets")
        .map(|playlist| playlist.name.as_str())
        .collect();
    assert_eq!(reopened_order, ["Techno", "House"]);
    assert_eq!(
        app.xml_document_source.as_ref().unwrap(),
        &root.join("reordered.xml")
    );
    // A second export to the same path is refused and cannot overwrite the valid export.
    let exported = std::fs::read(root.join("reordered.xml")).unwrap();
    app.export_xml_sibling_playlist_reorder();
    assert_eq!(std::fs::read(root.join("reordered.xml")).unwrap(), exported);
    let mutated = String::from_utf8(bytes)
        .unwrap()
        .replace("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"");
    assert!(f02_sibling_playlist_reorder_oracle::verify(mutated.as_bytes()).is_err());
    assert_eq!(
        std::fs::read(source).unwrap(),
        include_bytes!("../../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sibling_playlist_reorder_ui_guards_reject_stale_invalid_cancel_and_existing_destination() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-sibling-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml"
    );
    let setup = |label: &str| {
        let source = root.join(format!("{label}.xml"));
        std::fs::copy(fixture, &source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_sibling_parent = Some(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .folders
                .iter()
                .find(|folder| folder.name == "Sets")
                .unwrap()
                .ordinal,
        );
        app.xml_sibling_order = "Techno, House".into();
        app.prepare_xml_sibling_playlist_reorder();
        (app, source)
    };

    for invalid in ["House, House", "House, Missing", "House"] {
        let (mut app, source) = setup("invalid");
        app.xml_sibling_order = invalid.into();
        app.prepare_xml_sibling_playlist_reorder();
        assert!(app.xml_sibling_pending.is_none(), "accepted {invalid}");
        let destination = root.join("invalid-output.xml");
        app.xml_sibling_export_path = destination.display().to_string();
        app.export_xml_sibling_playlist_reorder();
        assert!(!destination.exists());
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
    }

    for stale in ["source", "parent", "order"] {
        let (mut app, source) = setup(stale);
        match stale {
            "source" => app.xml_document_source = Some(root.join("different.xml")),
            "parent" => {
                app.xml_preview
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .folders
                    .iter_mut()
                    .find(|folder| folder.name == "Sets")
                    .unwrap()
                    .name = "Renamed".into()
            }
            "order" => {
                let preview = app.xml_preview.as_mut().unwrap().as_mut().unwrap();
                let house = preview
                    .playlists
                    .iter()
                    .position(|p| p.name == "House")
                    .unwrap();
                let techno = preview
                    .playlists
                    .iter()
                    .position(|p| p.name == "Techno")
                    .unwrap();
                preview.playlists.swap(house, techno);
            }
            _ => unreachable!(),
        }
        let destination = root.join(format!("{stale}-output.xml"));
        app.xml_sibling_export_path = destination.display().to_string();
        app.export_xml_sibling_playlist_reorder();
        assert!(app.xml_sibling_pending.is_none());
        assert!(!destination.exists(), "stale {stale} created output");
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
    }

    let (mut boundary, source) = setup("boundary");
    let original = std::fs::read(&source).unwrap();
    boundary.xml_sibling_parent = Some(usize::MAX);
    boundary.prepare_xml_sibling_playlist_reorder();
    assert!(boundary.xml_sibling_pending.is_none());
    let destination = root.join("boundary-output.xml");
    boundary.xml_sibling_export_path = destination.display().to_string();
    boundary.export_xml_sibling_playlist_reorder();
    assert!(!destination.exists());
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let (mut cancelled, source) = setup("cancel");
    let original = std::fs::read(&source).unwrap();
    cancelled.cancel_xml_sibling_playlist_reorder();
    assert!(cancelled.xml_sibling_pending.is_none());
    let destination = root.join("cancel-output.xml");
    cancelled.xml_sibling_export_path = destination.display().to_string();
    cancelled.export_xml_sibling_playlist_reorder();
    assert!(!destination.exists());
    assert_eq!(std::fs::read(source).unwrap(), original);

    let (mut refused, source) = setup("destination");
    let destination = root.join("already-exists.xml");
    std::fs::write(&destination, b"do not overwrite").unwrap();
    refused.xml_sibling_export_path = destination.display().to_string();
    refused.export_xml_sibling_playlist_reorder();
    assert_eq!(std::fs::read(destination).unwrap(), b"do not overwrite");
    assert_eq!(
        std::fs::read(source).unwrap(),
        std::fs::read(fixture).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn control_concepts_toolbar_button_opens_gallery_on_next_frame() {
    let mut app = app();
    let ctx = egui::Context::default();
    let render = |ctx: &egui::Context, app: &mut LibraryApp, input| {
        let mut button_rect = None;
        let output = ctx.run(input, |ctx| {
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                button_rect = Some(app.show_control_gallery_button(ui).rect);
            });
            app.show_control_gallery_window(ctx, ThemeMode::TokyoNight.tokens());
        });
        (button_rect, output)
    };
    let (button_rect, before_click) = render(&ctx, &mut app, egui::RawInput::default());
    let gallery_text = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(
        !gallery_text(&before_click)
            .iter()
            .any(|text| text == "OxiDJ · Control concept gallery")
    );
    let rect = button_rect.expect("toolbar action rendered");
    let pointer = |pressed| egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..Default::default()
    };
    let _ = render(&ctx, &mut app, pointer(true));
    let _ = render(&ctx, &mut app, pointer(false));
    assert!(app.show_control_gallery);
    let (_, output) = render(&ctx, &mut app, egui::RawInput::default());
    assert!(app.show_control_gallery);
    let rendered_text = gallery_text(&output);
    assert!(
        rendered_text
            .iter()
            .any(|text| text == "OxiDJ · Control concept gallery"),
        "gallery window title was not rendered: {rendered_text:?}"
    );
    assert!(
        rendered_text.iter().any(|text| text == "Studio rack"),
        "gallery content was not rendered: {rendered_text:?}"
    );
}

fn temporary_xml() -> (PathBuf, PathBuf) {
    use std::time::SystemTime;
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-name-edit-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("library.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin"),
    )
    .unwrap();
    (root, source)
}

#[test]
fn playlist_rename_ui_exports_prepared_bytes_and_reopens_with_memberships_unchanged() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-playlist-rename/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let output = root.join("renamed.xml");
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_playlist(0);
    let current = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .playlists[0]
        .name
        .clone();
    app.xml_playlist_rename_name = "Summer & Sun".into();
    app.prepare_xml_playlist_rename(0, &current);
    let expected = app
        .xml_pending_rename
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    app.cancel_xml_playlist_rename();
    assert!(app.xml_pending_rename.is_none());
    app.prepare_xml_playlist_rename(0, &current);
    app.xml_rename_export_path = output.display().to_string();
    app.export_xml_playlist_rename();
    let actual = std::fs::read(&output).unwrap();
    assert_eq!(actual, expected);
    f02_playlist_rename_oracle::check(&actual)
        .expect("actual UI bytes satisfy independent F-02 oracle");
    let text = String::from_utf8(actual.clone()).unwrap();
    let mutated = text.replacen("Summer &amp; Sun", "Tampered", 1);
    assert!(
        f02_playlist_rename_oracle::check(mutated.as_bytes()).is_err(),
        "oracle rejects mutation of actual UI output"
    );
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[0].name, "Summer & Sun");
    assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_export_writes_exact_prepared_bytes_and_preserves_source_and_catalog() {
    let (root, source) = temporary_xml();
    let output = root.join("edited.xml");
    let original = std::fs::read(&source).unwrap();
    let catalog_path = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog_path).unwrap());
    let catalog_before = std::fs::read(&catalog_path).unwrap();
    let mut app = LibraryApp::new(
        catalog_path.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_edit_name = "New < title".into();
    app.prepare_xml_name_edit("100", "A");
    let previewed_bytes = app
        .xml_pending_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.xml_export_path = output.display().to_string();

    app.export_xml_name_edit();

    let exported_bytes = std::fs::read(&output).unwrap();
    assert_eq!(exported_bytes, previewed_bytes);
    f02_track_name_oracle_support::check(&exported_bytes)
        .expect("independent F-02 oracle accepts only the selected Name change");
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .name,
        "New < title",
        "reopened output exposes the exported Name"
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(
        std::fs::read(&catalog_path).unwrap(),
        catalog_before,
        "XML export changed catalog bytes"
    );
    assert!(app.xml_pending_edit.is_none());
    assert!(app.notice.contains(&output.display().to_string()));
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_clear_preview_cancel_export_reopen_and_oracle_are_source_bound() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-clear-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-clear/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_playlist(0);
    app.prepare_xml_playlist_clear(0);
    let prepared = app
        .xml_pending_clear
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "preparation is read-only"
    );
    let cancelled = root.join("cancelled.xml");
    app.xml_clear_export_path = cancelled.display().to_string();
    app.cancel_xml_playlist_clear();
    assert!(app.xml_pending_clear.is_none(), "cancel clears proposal");
    assert!(!cancelled.exists(), "cancel writes nothing");
    app.prepare_xml_playlist_clear(0);
    let output = root.join("cleared.xml");
    app.xml_clear_export_path = output.display().to_string();
    app.select_xml_track("200", "B");
    assert!(
        app.xml_pending_clear.is_none(),
        "track selection invalidates pending clear"
    );
    app.export_xml_playlist_clear();
    assert!(!output.exists(), "invalidated proposal cannot export");
    app.select_xml_playlist(0);
    app.prepare_xml_playlist_clear(0);
    app.xml_clear_export_path = output.display().to_string();
    app.export_xml_playlist_clear();
    let exported = std::fs::read(&output).unwrap();
    assert_eq!(exported, prepared);
    f02_playlist_clear_oracle::check(&exported)
        .expect("UI-exported bytes satisfy independent F-02 oracle");
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[0].name, "House");
    assert!(reopened.playlists[0].tracks.is_empty());
    assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
    assert_eq!(
        reopened
            .tracks
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["100", "200", "300", "400"]
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert!(
        app.notice
            .contains("collection tracks and other playlists are unchanged")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_clear_refuses_changed_source_order_invalid_selection_and_existing_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-clear-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-clear/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.prepare_xml_playlist_clear(99);
    assert!(app.xml_pending_clear.is_none());
    app.prepare_xml_playlist_clear(2);
    assert!(
        app.xml_pending_clear.is_none(),
        "empty playlist is rejected"
    );
    app.select_xml_playlist(0);
    app.prepare_xml_playlist_clear(0);
    if let Some(Ok(preview)) = app.xml_preview.as_mut() {
        preview.playlists[0].tracks.swap(0, 1);
    }
    app.xml_clear_export_path = root.join("changed-order.xml").display().to_string();
    app.export_xml_playlist_clear();
    assert!(!PathBuf::from(&app.xml_clear_export_path).exists());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("membership order changed")
    );
    app.preview_xml();
    app.select_xml_playlist(0);
    app.prepare_xml_playlist_clear(0);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_clear_export_path = existing.display().to_string();
    app.export_xml_playlist_clear();
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
    app.prepare_xml_playlist_clear(0);
    std::fs::write(&source, b"changed source").unwrap();
    let stale_output = root.join("stale.xml");
    app.xml_clear_export_path = stale_output.display().to_string();
    app.export_xml_playlist_clear();
    assert!(!stale_output.exists());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn whole_playlist_delete_preview_cancel_export_and_oracle_are_source_bound() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-delete-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-delete/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let mut app = LibraryApp::new(
        catalog,
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_playlist(1);
    app.prepare_xml_playlist_delete(1);
    let prepared = app
        .xml_pending_playlist_delete
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    let prepared_export = app
        .xml_pending_playlist_delete
        .as_ref()
        .unwrap()
        .prepared
        .clone();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let stale_output = root.join("stale-source.xml");
    std::fs::write(&source, b"changed source").unwrap();
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_delete(
            &prepared_export,
            &stale_output
        )
        .is_err()
    );
    assert!(!stale_output.exists());
    std::fs::write(&source, &original).unwrap();
    app.select_xml_playlist(0);
    assert!(
        app.xml_pending_playlist_delete.is_none(),
        "selection change invalidates proposal"
    );
    app.select_xml_playlist(1);
    app.prepare_xml_playlist_delete(1);
    let cancelled = root.join("cancelled.xml");
    app.xml_playlist_delete_export_path = cancelled.display().to_string();
    app.cancel_xml_playlist_delete();
    assert!(!cancelled.exists());
    app.prepare_xml_playlist_delete(1);
    let preview_before = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let removed = &preview_before.playlists[1];
    let mut collision_preview = preview_before.clone();
    collision_preview.folders.push(oxidj_rekordbox::Folder {
        ordinal: 99,
        path: vec!["ROOT".into(), "Other".into(), "Sets".into()],
        name: "Sets".into(),
        count: Some("99".into()),
    });
    let parent = immediate_parent_folder(&collision_preview, removed).unwrap_or_else(|| {
        panic!(
            "playlist path {:?}, folders {:?}",
            removed.path, collision_preview.folders
        )
    });
    assert_eq!(parent.path, ["ROOT", "Sets"]);
    assert_eq!(parent.name, "Sets");
    assert_eq!(parent.count.as_deref(), Some("3"));
    assert_eq!(
        parent.count.as_deref().unwrap().parse::<usize>().unwrap() - 1,
        2
    );
    let output = root.join("deleted.xml");
    app.xml_playlist_delete_export_path = output.display().to_string();
    app.export_xml_playlist_delete();
    let exported = std::fs::read(&output).unwrap();
    assert_eq!(exported, prepared);
    f02_playlist_delete_oracle::check(&exported)
        .expect("actual UI-exported bytes pass independent F-02 oracle");
    assert_eq!(app.xml_document_source.as_ref(), Some(&output));
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        reopened
            .playlists
            .iter()
            .map(|playlist| playlist.name.as_str())
            .collect::<Vec<_>>(),
        ["House"]
    );
    assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
    let reopened_parent = reopened
        .folders
        .iter()
        .find(|folder| folder.name == "Sets")
        .unwrap();
    assert_eq!(reopened_parent.count.as_deref(), Some("2"));
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_delete(
            &prepared_export,
            &existing
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_removal_preview_cancel_export_and_oracle_are_source_bound() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-delete-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    app.prepare_xml_member_delete(0, 1);
    let prepared = app
        .xml_pending_delete
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_delete_export_path = cancelled.display().to_string();
    app.cancel_xml_member_delete();
    assert!(!cancelled.exists());
    app.prepare_xml_member_delete(0, 1);
    assert_eq!(
        app.xml_pending_delete.as_ref().unwrap().prepared.bytes(),
        prepared
    );
    let output = root.join("removed.xml");
    app.xml_delete_export_path = output.display().to_string();
    app.export_xml_member_delete();
    let exported = std::fs::read(&output).unwrap();
    assert_eq!(exported, prepared);
    f02_playlist_member_delete_oracle::check(&exported)
        .expect("actual UI-exported bytes pass independent F-02 oracle");
    let mut mutation = exported.clone();
    let index = mutation
        .windows(9)
        .position(|w| w == b"Key=\"100\"")
        .unwrap();
    mutation[index + 5] = b'9';
    assert!(
        f02_playlist_member_delete_oracle::check(&mutation).is_err(),
        "oracle rejects semantic mutation"
    );
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[0].tracks, ["100", "300"]);
    assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_removal_proposal_invalidation_clears_stale_preview() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-delete-invalidate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();

    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    app.prepare_xml_member_delete(0, 1);
    assert!(app.xml_pending_delete.is_some());
    app.on_xml_source_changed();
    assert!(
        app.xml_pending_delete.is_none(),
        "source change clears proposal"
    );
    assert_eq!(app.xml_selected_playlist, None);
    assert_eq!(app.xml_selected_member, None);

    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    app.prepare_xml_member_delete(0, 1);
    assert!(app.xml_pending_delete.is_some());
    app.select_xml_playlist(1);
    assert!(
        app.xml_pending_delete.is_none(),
        "playlist selection clears proposal"
    );
    assert_eq!(app.xml_selected_playlist, Some(1));
    assert_eq!(app.xml_selected_member, None);

    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    app.prepare_xml_member_delete(0, 1);
    assert!(app.xml_pending_delete.is_some());
    app.select_xml_member(0);
    assert!(
        app.xml_pending_delete.is_none(),
        "member selection clears proposal"
    );
    assert_eq!(app.xml_selected_member, Some(0));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_removal_refuses_stale_source_membership_and_existing_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-delete-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
    );
    std::fs::copy(fixture, &source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    if let Some(Ok(preview)) = app.xml_preview.as_mut() {
        preview.playlists[0].tracks[1] = "999".into();
    }
    app.prepare_xml_member_delete(0, 1);
    assert!(
        app.xml_pending_delete.is_none(),
        "stale membership accepted"
    );
    app.xml_preview =
        Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
    app.prepare_xml_member_delete(0, 99);
    assert!(app.xml_pending_delete.is_none());
    app.prepare_xml_member_delete(0, 1);
    let pending = app.xml_pending_delete.take().unwrap();
    let stale = root.join("stale.xml");
    std::fs::write(&source, b"stale source").unwrap();
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_member_delete(
            &pending.prepared,
            &stale
        )
        .is_err()
    );
    assert!(!stale.exists());
    std::fs::copy(fixture, &source).unwrap();
    app.xml_preview =
        Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
    app.prepare_xml_member_delete(0, 1);
    let pending = app.xml_pending_delete.take().unwrap();
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_member_delete(
            &pending.prepared,
            &existing
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_insertion_preview_cancel_export_and_oracle_are_source_bound() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-insert-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let output = root.join("inserted.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(1);
    app.xml_selected_track = Some("100".into());
    app.xml_insert_position = 1;
    app.prepare_xml_member_insert(1, "100", 1);
    let expected = app
        .xml_pending_insert
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.cancel_xml_member_insert();
    assert!(app.xml_pending_insert.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(!output.exists(), "cancel wrote a destination");
    app.prepare_xml_member_insert(1, "100", 1);
    app.xml_insert_export_path = output.display().to_string();
    app.export_xml_member_insert();
    let exported = std::fs::read(&output).unwrap();
    assert_eq!(exported, expected);
    assert_eq!(
        super::f02_playlist_member_insert_oracle::SOURCE,
        original.as_slice()
    );
    super::f02_playlist_member_insert_oracle::check(&exported).unwrap();
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[1].tracks, ["200", "100", "400"]);
    assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_insertion_invalidates_changed_source_playlist_track_slot_and_order() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-insert-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(1);
    app.xml_selected_track = Some("100".into());
    app.prepare_xml_member_insert(1, "100", 1);
    assert!(app.xml_pending_insert.is_some());
    app.select_xml_playlist(0);
    assert!(
        app.xml_pending_insert.is_none(),
        "playlist change retained proposal"
    );
    app.select_xml_playlist(1);
    app.prepare_xml_member_insert(1, "100", 1);
    app.xml_selected_track = Some("300".into());
    app.xml_insert_export_path = root.join("track-change.xml").display().to_string();
    app.export_xml_member_insert();
    assert!(
        app.xml_pending_insert.is_none(),
        "track change retained proposal"
    );
    assert!(!root.join("track-change.xml").exists());
    app.xml_selected_track = Some("100".into());
    app.prepare_xml_member_insert(1, "100", 1);
    app.xml_insert_position = 0;
    app.xml_insert_export_path = root.join("slot-change.xml").display().to_string();
    app.export_xml_member_insert();
    assert!(
        app.xml_pending_insert.is_none(),
        "slot change retained proposal"
    );
    assert!(!root.join("slot-change.xml").exists());
    app.xml_insert_position = 1;
    app.prepare_xml_member_insert(1, "100", 1);
    if let Some(Ok(preview)) = app.xml_preview.as_mut() {
        preview.playlists[1].tracks.swap(0, 1);
    }
    app.xml_insert_export_path = root.join("order-change.xml").display().to_string();
    app.export_xml_member_insert();
    assert!(
        app.xml_pending_insert.is_none(),
        "order change retained proposal"
    );
    assert!(!root.join("order-change.xml").exists());
    app.prepare_xml_member_insert(1, "100", 1);
    app.on_xml_source_changed();
    assert!(
        app.xml_pending_insert.is_none(),
        "source change retained proposal"
    );
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
        ),
        &source,
    )
    .unwrap();
    app.preview_xml();
    app.xml_selected_playlist = Some(1);
    app.xml_selected_track = Some("100".into());
    app.prepare_xml_member_insert(1, "100", 1);
    let stale_output = root.join("stale.xml");
    std::fs::write(&source, b"changed after prepare").unwrap();
    app.xml_insert_export_path = stale_output.display().to_string();
    app.export_xml_member_insert();
    assert!(
        app.xml_pending_insert.is_some(),
        "failed revalidation should retain preview"
    );
    assert!(!stale_output.exists(), "stale source published output");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
        ),
        &source,
    )
    .unwrap();
    app.preview_xml();
    app.xml_selected_playlist = Some(1);
    app.xml_selected_track = Some("100".into());
    app.prepare_xml_member_insert(1, "100", 1);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_insert_export_path = existing.display().to_string();
    app.export_xml_member_insert();
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        b"keep",
        "existing output was overwritten"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_append_preview_cancel_and_export_are_source_bound() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-append-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_track = Some("100".into());
    app.xml_selected_playlist = Some(1);
    app.prepare_xml_member_append(1, "100");
    let prepared = app
        .xml_pending_append
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert!(app.xml_pending_append.is_some());
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "prepare must not write source"
    );
    let cancelled = root.join("cancelled.xml");
    app.xml_append_export_path = cancelled.display().to_string();
    app.cancel_xml_member_append();
    assert!(!cancelled.exists(), "cancel must not write output");
    app.prepare_xml_member_append(1, "100");
    assert_eq!(
        app.xml_pending_append.as_ref().unwrap().prepared.bytes(),
        prepared
    );
    let output = root.join("appended.xml");
    app.xml_append_export_path = output.display().to_string();
    app.export_xml_member_append();
    mod f02_oracle {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/common/f02_playlist_member_append_oracle_shared.rs"
        ));
    }
    assert_eq!(f02_oracle::SOURCE, original.as_slice());
    f02_oracle::check(&std::fs::read(&output).unwrap()).unwrap();
    let mutated = std::fs::read(&output)
        .unwrap()
        .windows(9)
        .position(|w| w == b"Key=\"100\"");
    assert!(mutated.is_some());
    let mut bad = std::fs::read(&output).unwrap();
    let at = mutated.unwrap();
    bad[at + 5] = b'9';
    assert!(
        f02_oracle::check(&bad).is_err(),
        "oracle must reject semantic mutation"
    );
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[1].tracks, ["200", "400", "100"]);
    assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_append_refuses_stale_source_membership_duplicate_unknown_and_existing_output()
 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-append-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
        ),
        &source,
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(1);
    app.prepare_xml_member_append(1, "200");
    assert!(
        app.xml_pending_append.is_none(),
        "duplicate membership accepted"
    );
    app.prepare_xml_member_append(1, "999");
    assert!(app.xml_pending_append.is_none(), "unknown track accepted");
    if let Some(Ok(preview)) = app.xml_preview.as_mut() {
        preview.playlists[1].tracks = vec!["999".into()];
    }
    app.prepare_xml_member_append(1, "100");
    assert!(
        app.xml_pending_append.is_none(),
        "stale membership accepted"
    );
    app.xml_preview =
        Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
    app.prepare_xml_member_append(1, "100");
    let pending = app.xml_pending_append.take().unwrap();
    std::fs::write(&source, b"stale source").unwrap();
    let stale_output = root.join("stale.xml");
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_member_append(
            &pending.prepared,
            &stale_output
        )
        .is_err()
    );
    assert!(!stale_output.exists());
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
        ),
        &source,
    )
    .unwrap();
    app.prepare_xml_member_append(1, "100");
    let pending = app.xml_pending_append.take().unwrap();
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    assert!(
        oxidj_application::export_prepared_rekordbox_playlist_member_append(
            &pending.prepared,
            &existing
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_reorder_preview_exports_exact_prepared_bytes_and_reopens_order() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-member-reorder-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-reorder/source.xml"
        ),
        &source,
    )
    .unwrap();
    let intermediate = root.join("intermediate.xml");
    let output = root.join("reordered.xml");
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(2);
    app.prepare_xml_member_reorder(0, 2, -1);
    let first_bytes = app
        .xml_pending_reorder
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(
        app.xml_pending_reorder.as_ref().unwrap().prepared.source(),
        source
    );
    app.xml_reorder_export_path = intermediate.display().to_string();
    app.export_xml_member_reorder();
    assert_eq!(std::fs::read(&intermediate).unwrap(), first_bytes);
    app.xml_document_source = Some(intermediate.clone());
    app.xml_preview = Some(
        oxidj_application::preview_rekordbox_xml(&intermediate).map_err(|e| e.to_string()),
    );
    app.xml_selected_playlist = Some(0);
    app.xml_selected_member = Some(1);
    app.prepare_xml_member_reorder(0, 1, -1);
    let expected = app
        .xml_pending_reorder
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.xml_reorder_export_path = output.display().to_string();
    app.export_xml_member_reorder();
    let exported = std::fs::read(&output).unwrap();
    assert_eq!(exported, expected);
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert_eq!(reopened.playlists[0].tracks, ["300", "100", "200"]);
    assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
    assert_eq!(
        std::fs::read(&source).unwrap(),
        include_bytes!("../../../xtask/tests/fixtures/f02-playlist-member-reorder/source.xml")
    );
    assert!(app.xml_pending_reorder.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn playlist_member_reorder_refuses_boundary_and_clears_pending_proposal() {
    let (root, source) = temporary_xml();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_selected_playlist = Some(0);
    app.prepare_xml_member_reorder(0, 1, 1);
    assert!(app.xml_pending_reorder.is_some());
    app.prepare_xml_member_reorder(0, 0, -1);
    assert!(app.xml_pending_reorder.is_none());
    assert!(app.xml_edit_error.as_deref().unwrap().contains("boundary"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_refuses_stale_source_alias_existing_output_and_surfaces_write_failure() {
    let (root, source) = temporary_xml();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_edit_name = "New < title".into();
    app.prepare_xml_name_edit("100", "A");
    let prepared_bytes = app
        .xml_pending_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();

    app.xml_export_path = source.display().to_string();
    app.export_xml_name_edit();
    assert!(app.xml_pending_edit.is_some());
    assert!(app.xml_edit_error.as_deref().unwrap().contains("same file"));
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep me").unwrap();
    app.xml_export_path = existing.display().to_string();
    app.export_xml_name_edit();
    assert!(app.xml_pending_edit.is_some());
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");

    std::fs::write(&source, b"stale source").unwrap();
    let stale_out = root.join("stale.xml");
    app.xml_export_path = stale_out.display().to_string();
    app.export_xml_name_edit();
    assert!(app.xml_pending_edit.is_some());
    assert!(!stale_out.exists());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert_ne!(prepared_bytes, std::fs::read(&source).unwrap());

    std::fs::write(&source, &original).unwrap();
    let failed_out = root.join("missing-parent").join("failed.xml");
    app.xml_export_path = failed_out.display().to_string();
    app.export_xml_name_edit();
    assert!(app.xml_pending_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .starts_with("Track Name export failed:")
    );
    assert!(
        app.notice.is_empty(),
        "failure must not acknowledge success"
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_track_name_edit_is_source_and_track_bound_and_cancelable() {
    let (root, source) = temporary_xml();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_path = root.join("typed-but-not-loaded.xml").display().to_string();
    app.xml_selected_track = Some("100".into());
    app.xml_edit_name = "Renamed".into();
    app.prepare_xml_name_edit("100", "A");
    let pending = app
        .xml_pending_edit
        .as_ref()
        .expect("prepared pending edit");
    assert_eq!(pending.source, source);
    assert_eq!(pending.track_id, "100");
    assert_eq!(pending.prepared.change().track_id, "100");
    assert_eq!(pending.prepared.change().old_name, "A");
    assert_eq!(pending.prepared.change().new_name, "Renamed");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    app.cancel_xml_name_edit();
    assert!(app.xml_pending_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    assert!(app.notice.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_artist_edit_ui_exports_prepared_bytes_and_checks_independent_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-artist-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_artist, "Old & exact");
    app.xml_edit_artist = "New Artist".into();
    app.prepare_xml_artist_edit("100", Some("Old & exact"));
    let prepared_bytes = app
        .xml_pending_artist_edit
        .as_ref()
        .expect("prepared Artist edit")
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

    let cancelled = root.join("cancelled.xml");
    app.xml_artist_export_path = cancelled.display().to_string();
    app.cancel_xml_artist_edit();
    assert!(app.xml_pending_artist_edit.is_none());
    assert!(!cancelled.exists(), "cancel must not write output");

    let output = root.join("edited.xml");
    app.prepare_xml_artist_edit("100", Some("Old & exact"));
    app.xml_artist_export_path = output.display().to_string();
    app.export_xml_artist_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared_bytes);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::ARTIST, &bytes)
        .expect("independent F-02 oracle accepts only the selected Artist change");
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Artist=\"New Artist\"",
        "Artist=\"Altered\"",
        1,
    );
    assert_ne!(mutated.as_bytes(), bytes.as_slice());
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::ARTIST, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .artist
            .as_deref(),
        Some("New Artist")
    );
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "200")
            .unwrap()
            .artist,
        None,
        "absent Artist remains absent on other tracks"
    );
    assert!(app.xml_pending_artist_edit.is_none());
    assert!(app.notice.contains(&output.display().to_string()));
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_artist_ui_escapes_xml_and_refuses_changed_proposals_and_absent_values() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-artist-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();

    app.select_xml_track("200", "B");
    app.prepare_xml_artist_edit("200", None);
    assert!(app.xml_pending_artist_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("no Artist attribute")
    );

    app.select_xml_track("100", "A");
    app.xml_edit_artist = "New < Artist & \"guest\"".into();
    app.prepare_xml_artist_edit("100", Some("Old & exact"));
    let pending = app.xml_pending_artist_edit.as_ref().unwrap();
    let expected = pending.prepared.change().new_artist.clone();
    let output = root.join("escaped.xml");
    app.xml_artist_export_path = output.display().to_string();
    app.xml_edit_artist.push('!');
    app.export_xml_artist_edit();
    assert!(app.xml_pending_artist_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("proposed value changed")
    );
    assert!(!output.exists());

    app.xml_edit_artist = expected;
    app.prepare_xml_artist_edit("100", Some("Old & exact"));
    app.export_xml_artist_edit();
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("New &lt; Artist &amp; &quot;guest&quot;"));
    f02_track_attribute_edit_oracle::check_with(&f02_track_attribute_edit_oracle::ARTIST, &bytes, "New < Artist & \"guest\"")
        .expect("independent whole-tree oracle accepts escaped UI output");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_artist_ui_rejects_stale_source_without_creating_output() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_artist = "Changed".into();
    app.prepare_xml_artist_edit("100", Some("Old & exact"));
    std::fs::write(&source, b"<changed/>").unwrap();
    let output = root.join("stale.xml");
    app.xml_artist_export_path = output.display().to_string();
    app.export_xml_artist_edit();
    assert!(app.xml_pending_artist_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!output.exists());
    std::fs::write(&source, original).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_comments_edit_ui_exports_previewed_bytes_through_independent_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-comments-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_comments, "old & note");
    app.xml_edit_comments = "new < note".into();
    app.prepare_xml_comments_edit("100", Some("old & note"));
    let prepared_bytes = app
        .xml_pending_comments_edit
        .as_ref()
        .expect("prepared Comments edit")
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

    let cancelled = root.join("cancelled.xml");
    app.xml_comments_export_path = cancelled.display().to_string();
    app.cancel_xml_comments_edit();
    assert!(app.xml_pending_comments_edit.is_none());
    assert!(!cancelled.exists(), "cancel must not write output");

    app.prepare_xml_comments_edit("100", Some("old & note"));
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_comments_export_path = existing.display().to_string();
    app.export_xml_comments_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "export errors must be surfaced"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    assert_eq!(std::fs::read(&source).unwrap(), original);

    app.xml_comments_export_path = root.join("edited.xml").display().to_string();
    app.export_xml_comments_edit();
    let output = PathBuf::from(&app.xml_comments_export_path);
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared_bytes);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::COMMENTS, &bytes)
        .expect("independent F-02 oracle accepts only selected Comments edit");
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Comments=\"new &lt; note\"",
        "Comments=\"altered\"",
        1,
    );
    assert_ne!(mutated.as_bytes(), bytes.as_slice());
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::COMMENTS, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .comments
            .as_deref(),
        Some("new < note")
    );
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "200")
            .unwrap()
            .comments,
        None,
        "absent Comments remains absent on other tracks"
    );
    assert!(app.xml_pending_comments_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_comments_edit_ui_refuses_absent_and_changed_proposal() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-comments-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();

    app.select_xml_track("200", "B");
    app.prepare_xml_comments_edit("200", None);
    assert!(app.xml_pending_comments_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("no Comments attribute")
    );

    app.select_xml_track("100", "A");
    app.xml_edit_comments = "new < note".into();
    app.prepare_xml_comments_edit("100", Some("old & note"));
    app.xml_edit_comments.push('!');
    app.xml_comments_export_path = root.join("changed.xml").display().to_string();
    app.export_xml_comments_edit();
    assert!(app.xml_pending_comments_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("proposed value changed")
    );
    assert!(!root.join("changed.xml").exists());

    app.xml_edit_comments = "new < note".into();
    app.prepare_xml_comments_edit("100", Some("old & note"));
    assert!(app.xml_pending_comments_edit.is_some());
    app.select_xml_track("200", "B");
    assert!(app.xml_pending_comments_edit.is_none());

    app.select_xml_track("100", "A");
    app.prepare_xml_comments_edit("100", Some("old & note"));
    assert!(app.xml_pending_comments_edit.is_some());
    app.preview_xml();
    assert!(app.xml_pending_comments_edit.is_none());

    let empty_comments = std::fs::read(&source)
        .unwrap()
        .windows(b"Comments=\"old &amp; note\"".len())
        .position(|window| window == b"Comments=\"old &amp; note\"")
        .expect("fixture Comments attribute") as usize;
    let mut empty_fixture = std::fs::read(&source).unwrap();
    empty_fixture.splice(
        empty_comments..empty_comments + b"Comments=\"old &amp; note\"".len(),
        b"Comments=\"\"".iter().copied(),
    );
    std::fs::write(&source, empty_fixture).unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert!(app.xml_edit_comments.is_empty());
    app.prepare_xml_comments_edit("100", Some(""));
    assert!(app.xml_pending_comments_edit.is_some());
    app.cancel_xml_comments_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_comments_edit_ui_rejects_stale_source_without_output() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_comments = "new < note".into();
    app.prepare_xml_comments_edit("100", Some("old & note"));
    std::fs::write(&source, b"<changed/>").unwrap();
    let output = root.join("stale.xml");
    app.xml_comments_export_path = output.display().to_string();
    app.export_xml_comments_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!output.exists());
    std::fs::write(&source, original).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_genre_edit_ui_exports_previewed_bytes_and_reopens_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-genre-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_genre, "old & style");
    app.xml_edit_genre = "new < style".into();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    let prepared = app
        .xml_pending_genre_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_genre_export_path = cancelled.display().to_string();
    app.cancel_xml_genre_edit();
    assert!(app.xml_pending_genre_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_genre_edit("100", Some("old & style"));
    app.xml_genre_export_path.clear();
    app.export_xml_genre_edit();
    assert!(app.xml_pending_genre_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_genre_export_path = source.display().to_string();
    app.export_xml_genre_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_genre_export_path = existing.display().to_string();
    app.export_xml_genre_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "existing output must be refused"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_genre_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_genre_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");
    app.xml_genre_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_genre_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    app.xml_edit_genre = "changed after preview".into();
    let stale_proposal = root.join("stale-proposal.xml");
    app.xml_genre_export_path = stale_proposal.display().to_string();
    app.export_xml_genre_edit();
    assert!(!stale_proposal.exists());
    assert!(app.xml_pending_genre_edit.is_none());
    app.xml_edit_genre = "new < style".into();
    app.prepare_xml_genre_edit("100", Some("stale old value"));
    assert!(app.xml_pending_genre_edit.is_none());
    app.prepare_xml_genre_edit("200", Some("old & style"));
    assert!(
        app.xml_pending_genre_edit.is_none(),
        "absent Genre insertion is refused"
    );
    app.prepare_xml_genre_edit("100", Some("old & style"));
    let changed_selection = root.join("changed-selection.xml");
    app.xml_genre_export_path = changed_selection.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_genre_edit();
    assert!(!changed_selection.exists());
    app.prepare_xml_genre_edit("100", Some("old & style"));
    let changed_source = root.join("different.xml");
    app.xml_document_source = Some(changed_source);
    let stale_source = root.join("stale-source.xml");
    app.xml_genre_export_path = stale_source.display().to_string();
    app.export_xml_genre_edit();
    assert!(!stale_source.exists());
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_genre = "new < style".into();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    let output = root.join("edited.xml");
    app.xml_genre_export_path = output.display().to_string();
    app.export_xml_genre_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::GENRE, &bytes)
        .expect("independent F-02 oracle accepts UI-exported Genre bytes");
    assert!(String::from_utf8_lossy(&bytes).contains("Genre=\"new &lt; style\""));
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Genre=\"new &lt; style\"",
        "Genre=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::GENRE, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .genre
            .as_deref(),
        Some("new < style")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .genre,
        None,
        "absent Genre remains absent on other tracks"
    );
    assert!(app.xml_pending_genre_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn track_label_edit_ui_exports_previewed_bytes_and_reopens_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-label-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_label, "old & imprint");
    app.xml_edit_label = "new < imprint".into();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    let prepared = app
        .xml_pending_label_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_label_export_path = cancelled.display().to_string();
    app.cancel_xml_label_edit();
    assert!(app.xml_pending_label_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    app.xml_label_export_path.clear();
    app.export_xml_label_edit();
    assert!(app.xml_pending_label_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_label_export_path = source.display().to_string();
    app.export_xml_label_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_label_export_path = existing.display().to_string();
    app.export_xml_label_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "existing output must be refused"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_label_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_label_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");
    app.xml_label_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_label_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    app.xml_edit_label = "changed after preview".into();
    let stale_proposal = root.join("stale-proposal.xml");
    app.xml_label_export_path = stale_proposal.display().to_string();
    app.export_xml_label_edit();
    assert!(!stale_proposal.exists());
    assert!(app.xml_pending_label_edit.is_none());
    app.xml_edit_label = "new < imprint".into();
    app.prepare_xml_label_edit("100", Some("stale old value"));
    assert!(app.xml_pending_label_edit.is_none());
    app.prepare_xml_label_edit("200", Some("old & imprint"));
    assert!(
        app.xml_pending_label_edit.is_none(),
        "absent Label insertion is refused"
    );
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    let changed_selection = root.join("changed-selection.xml");
    app.xml_label_export_path = changed_selection.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_label_edit();
    assert!(!changed_selection.exists());
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    let changed_source = root.join("different.xml");
    app.xml_document_source = Some(changed_source);
    let stale_source = root.join("stale-source.xml");
    app.xml_label_export_path = stale_source.display().to_string();
    app.export_xml_label_edit();
    assert!(!stale_source.exists());
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_label = "new < imprint".into();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    let output = root.join("edited.xml");
    app.xml_label_export_path = output.display().to_string();
    app.export_xml_label_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::LABEL, &bytes)
        .expect("independent F-02 oracle accepts UI-exported Label bytes");
    assert!(String::from_utf8_lossy(&bytes).contains("Label=\"new &lt; imprint\""));
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Label=\"new &lt; imprint\"",
        "Label=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::LABEL, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .label
            .as_deref(),
        Some("new < imprint")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .label,
        None,
        "absent Label remains absent on other tracks"
    );
    assert!(app.xml_pending_label_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn track_genre_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_genre = "Changed".into();
    app.prepare_xml_genre_edit("999", Some("old & style"));
    assert!(app.xml_pending_genre_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_genre = "bad\u{1}".into();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    assert!(app.xml_pending_genre_edit.is_none());

    app.xml_edit_genre = "Changed".into();
    app.prepare_xml_genre_edit("100", Some("stale"));
    assert!(app.xml_pending_genre_edit.is_none());

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\""),
    )
    .unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_genre = "Changed".into();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    assert!(
        app.xml_pending_genre_edit.is_none(),
        "duplicate target must refuse"
    );

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("Genre=\"old &amp; style\"", "Genre=\"\""),
    )
    .unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(
        app.xml_edit_genre, "",
        "present-empty Genre must stay editable"
    );
    app.xml_edit_genre = "Restored".into();
    app.prepare_xml_genre_edit("100", Some(""));
    assert!(
        app.xml_pending_genre_edit.is_some(),
        "present-empty Genre is editable"
    );
    app.cancel_xml_genre_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_composer_edit_ui_exports_previewed_bytes_and_reopens_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-composer-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_composer, "old & composer");
    app.xml_edit_composer = "new < composer".into();
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    let prepared = app
        .xml_pending_composer_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_composer_export_path = cancelled.display().to_string();
    app.cancel_xml_composer_edit();
    assert!(app.xml_pending_composer_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    app.xml_composer_export_path.clear();
    app.export_xml_composer_edit();
    assert!(app.xml_pending_composer_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_composer_export_path = source.display().to_string();
    app.export_xml_composer_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_composer_export_path = existing.display().to_string();
    app.export_xml_composer_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "existing output must be refused"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_composer_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_composer_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");
    app.xml_composer_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_composer_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    app.xml_edit_composer = "changed after preview".into();
    let stale_proposal = root.join("stale-proposal.xml");
    app.xml_composer_export_path = stale_proposal.display().to_string();
    app.export_xml_composer_edit();
    assert!(!stale_proposal.exists());
    assert!(app.xml_pending_composer_edit.is_none());
    app.xml_edit_composer = "new < composer".into();
    app.prepare_xml_composer_edit("100", Some("stale old value"));
    assert!(app.xml_pending_composer_edit.is_none());
    app.prepare_xml_composer_edit("200", Some("old & composer"));
    assert!(
        app.xml_pending_composer_edit.is_none(),
        "absent Composer insertion is refused"
    );
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    let changed_selection = root.join("changed-selection.xml");
    app.xml_composer_export_path = changed_selection.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_composer_edit();
    assert!(!changed_selection.exists());
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    let changed_source = root.join("different.xml");
    app.xml_document_source = Some(changed_source);
    let stale_source = root.join("stale-source.xml");
    app.xml_composer_export_path = stale_source.display().to_string();
    app.export_xml_composer_edit();
    assert!(!stale_source.exists());
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_composer = "new < composer".into();
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    let output = root.join("edited.xml");
    app.xml_composer_export_path = output.display().to_string();
    app.export_xml_composer_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::COMPOSER, &bytes)
        .expect("independent F-02 oracle accepts UI-exported Composer bytes");
    assert!(String::from_utf8_lossy(&bytes).contains("Composer=\"new &lt; composer\""));
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Composer=\"new &lt; composer\"",
        "Composer=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::COMPOSER, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .composer
            .as_deref(),
        Some("new < composer")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .composer,
        None,
        "absent Composer remains absent on other tracks"
    );
    assert!(app.xml_pending_composer_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn track_grouping_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_grouping = "Changed".into();
    app.prepare_xml_grouping_edit("999", Some("old & crate"));
    assert!(app.xml_pending_grouping_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_grouping = "bad\u{1}".into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    assert!(app.xml_pending_grouping_edit.is_none());

    app.xml_edit_grouping = "Changed".into();
    app.prepare_xml_grouping_edit("100", Some("stale"));
    assert!(app.xml_pending_grouping_edit.is_none());

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\""),
    )
    .unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_grouping = "Changed".into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    assert!(
        app.xml_pending_grouping_edit.is_none(),
        "duplicate target must refuse"
    );

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("Grouping=\"old &amp; crate\"", "Grouping=\"\""),
    )
    .unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(
        app.xml_edit_grouping, "",
        "present-empty Grouping must stay editable"
    );
    app.xml_edit_grouping = "Restored".into();
    app.prepare_xml_grouping_edit("100", Some(""));
    assert!(
        app.xml_pending_grouping_edit.is_some(),
        "present-empty Grouping is editable"
    );
    app.cancel_xml_grouping_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_grouping_edit_ui_exports_previewed_bytes_and_reopens_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-grouping-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_grouping, "old & crate");
    app.xml_edit_grouping = "new < crate".into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    let prepared = app
        .xml_pending_grouping_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_grouping_export_path = cancelled.display().to_string();
    app.cancel_xml_grouping_edit();
    assert!(app.xml_pending_grouping_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    app.xml_grouping_export_path.clear();
    app.export_xml_grouping_edit();
    assert!(app.xml_pending_grouping_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_grouping_export_path = source.display().to_string();
    app.export_xml_grouping_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_grouping_export_path = existing.display().to_string();
    app.export_xml_grouping_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "existing output must be refused"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_grouping_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_grouping_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");
    app.xml_grouping_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_grouping_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    app.xml_edit_grouping = "changed after preview".into();
    let stale_proposal = root.join("stale-proposal.xml");
    app.xml_grouping_export_path = stale_proposal.display().to_string();
    app.export_xml_grouping_edit();
    assert!(!stale_proposal.exists());
    assert!(app.xml_pending_grouping_edit.is_none());
    app.xml_edit_grouping = "new < crate".into();
    app.prepare_xml_grouping_edit("100", Some("stale old value"));
    assert!(app.xml_pending_grouping_edit.is_none());
    app.prepare_xml_grouping_edit("200", Some("old & crate"));
    assert!(
        app.xml_pending_grouping_edit.is_none(),
        "absent Grouping insertion is refused"
    );
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    let changed_selection = root.join("changed-selection.xml");
    app.xml_grouping_export_path = changed_selection.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_grouping_edit();
    assert!(!changed_selection.exists());
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    let changed_source = root.join("different.xml");
    app.xml_document_source = Some(changed_source);
    let stale_source = root.join("stale-source.xml");
    app.xml_grouping_export_path = stale_source.display().to_string();
    app.export_xml_grouping_edit();
    assert!(!stale_source.exists());
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_grouping = "new < crate".into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    let output = root.join("edited.xml");
    app.xml_grouping_export_path = output.display().to_string();
    app.export_xml_grouping_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::GROUPING, &bytes)
        .expect("independent F-02 oracle accepts UI-exported Grouping bytes");
    assert!(String::from_utf8_lossy(&bytes).contains("Grouping=\"new &lt; crate\""));
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Grouping=\"new &lt; crate\"",
        "Grouping=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::GROUPING, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .grouping
            .as_deref(),
        Some("new < crate")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .grouping,
        None,
        "absent Grouping remains absent on other tracks"
    );
    assert!(app.xml_pending_grouping_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn track_remixer_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_remixer = "Changed".into();
    app.prepare_xml_remixer_edit("999", Some("old & mix"));
    assert!(app.xml_pending_remixer_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "bad\u{1}".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert!(app.xml_pending_remixer_edit.is_none());

    app.xml_edit_remixer = "Changed".into();
    app.prepare_xml_remixer_edit("100", Some("stale"));
    assert!(app.xml_pending_remixer_edit.is_none());

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\""),
    )
    .unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "Changed".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "duplicate target must refuse"
    );

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("Remixer=\"old &amp; mix\"", "Remixer=\"\""),
    )
    .unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(
        app.xml_edit_remixer, "",
        "present-empty Remixer must stay editable"
    );
    app.xml_edit_remixer = "Restored".into();
    app.prepare_xml_remixer_edit("100", Some(""));
    assert!(
        app.xml_pending_remixer_edit.is_some(),
        "present-empty Remixer is editable"
    );
    app.cancel_xml_remixer_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_exports_previewed_bytes_and_reopens_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-remixer-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_remixer, "old & mix");
    app.xml_edit_remixer = "new < mix".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_remixer_export_path = cancelled.display().to_string();
    app.cancel_xml_remixer_edit();
    assert!(app.xml_pending_remixer_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    app.xml_remixer_export_path.clear();
    app.export_xml_remixer_edit();
    assert!(app.xml_pending_remixer_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_remixer_export_path = source.display().to_string();
    app.export_xml_remixer_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_remixer_export_path = existing.display().to_string();
    app.export_xml_remixer_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "existing output must be refused"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_remixer_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_remixer_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");
    app.xml_remixer_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_remixer_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    app.xml_edit_remixer = "changed after preview".into();
    let stale_proposal = root.join("stale-proposal.xml");
    app.xml_remixer_export_path = stale_proposal.display().to_string();
    app.export_xml_remixer_edit();
    assert!(!stale_proposal.exists());
    assert!(app.xml_pending_remixer_edit.is_none());
    app.xml_edit_remixer = "new < crate".into();
    app.prepare_xml_remixer_edit("100", Some("stale old value"));
    assert!(app.xml_pending_remixer_edit.is_none());
    app.select_xml_track("200", "B");
    app.prepare_xml_remixer_edit("200", Some("old & mix"));
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "absent Remixer insertion is refused"
    );
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    let changed_selection = root.join("changed-selection.xml");
    app.xml_remixer_export_path = changed_selection.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_remixer_edit();
    assert!(!changed_selection.exists());
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    let changed_source = root.join("different.xml");
    app.xml_document_source = Some(changed_source);
    let stale_source = root.join("stale-source.xml");
    app.xml_remixer_export_path = stale_source.display().to_string();
    app.export_xml_remixer_edit();
    assert!(!stale_source.exists());
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "new < mix".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    let prepared = app
        .xml_pending_remixer_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    let output = root.join("edited.xml");
    app.xml_remixer_export_path = output.display().to_string();
    app.export_xml_remixer_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::REMIXER, &bytes)
        .expect("independent F-02 remixer oracle accepts UI-exported Remixer bytes");
    assert!(String::from_utf8_lossy(&bytes).contains("Remixer=\"new &lt; mix\""));
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Remixer=\"new &lt; mix\"",
        "Remixer=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::REMIXER, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .remixer
            .as_deref(),
        Some("new < mix")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .remixer,
        None,
        "absent Remixer remains absent on other tracks"
    );
    assert!(app.xml_pending_remixer_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn track_remixer_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let remixer = "new < style & mix\tfirst\nsecond\r";
    app.xml_edit_remixer = remixer.into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert!(app.xml_pending_remixer_edit.is_some());
    app.xml_remixer_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_remixer_edit();
    let output = PathBuf::from(&app.xml_remixer_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("Remixer=\"new &lt; style &amp; mix&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .remixer
            .as_deref(),
        Some(remixer)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_refuses_export_after_xml_path_changed() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "new < mix".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    app.xml_remixer_export_path = root.join("cand.xml").display().to_string();
    app.xml_path = root.join("other.xml").display().to_string();
    app.export_xml_remixer_edit();
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "pending Remixer edit must be refused after xml_path change"
    );
    assert!(
        !root.join("cand.xml").exists(),
        "refused export must not write a candidate"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_discards_pending_after_source_path_change() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "new < mix".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert!(app.xml_pending_remixer_edit.is_some());
    // Change the source path through on_xml_source_changed.
    app.xml_path = root.join("other.xml").display().to_string();
    app.on_xml_source_changed();
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "pending Remixer edit must be discarded after source path change"
    );
    assert!(
        app.xml_edit_error.is_none(),
        "discarded pending edit must not leave an error"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_refuses_export_after_preview_old_value_changed() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "new < mix".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    // Re-point the track's Remixer in the source, then reload the preview.
    let mut text = std::fs::read_to_string(&source).unwrap();
    text = text.replace("Remixer=\"old &amp; mix\"", "Remixer=\"stale\"");
    std::fs::write(&source, text).unwrap();
    app.preview_xml();
    app.export_xml_remixer_edit();
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "pending Remixer export must be refused after preview old value changed"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_exports_and_reopens_present_empty_edit() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_remixer = "".into();
    app.prepare_xml_remixer_edit("100", Some("old & mix"));
    assert!(
        app.xml_pending_remixer_edit.is_some(),
        "present-empty Remixer must be prepareable"
    );
    app.xml_remixer_export_path = root.join("empty.xml").display().to_string();
    app.export_xml_remixer_edit();
    assert_eq!(app.xml_edit_remixer, "");
    assert_eq!(app.xml_path, root.join("empty.xml").display().to_string());
    app.preview_xml();
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .remixer
            .as_deref(),
        Some("")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_remixer_edit_ui_refuses_malformed_xml() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><DJ_PLAYLISTS><COLLECTION><TRACK",
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    assert!(
        app.xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .is_none(),
        "malformed XML must fail to preview"
    );
    app.select_xml_track("999", "X");
    assert!(
        app.xml_edit_remixer.is_empty(),
        "no Remixer shown for malformed preview"
    );
    app.xml_edit_remixer = "new".into();
    app.prepare_xml_remixer_edit("999", Some("old & mix"));
    assert!(
        app.xml_pending_remixer_edit.is_none(),
        "prepare must refuse malformed XML"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_refuses_unknown_invalid_stale_and_absent_values() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_mix = "Changed".into();
    app.prepare_xml_mix_edit("999", Some("old & set"));
    assert!(app.xml_pending_mix_edit.is_none());
    app.select_xml_track("100", "A");
    app.xml_edit_mix = "bad\u{1}".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    assert!(app.xml_pending_mix_edit.is_none());
    app.xml_edit_mix = "Changed".into();
    app.prepare_xml_mix_edit("100", Some("stale"));
    assert!(app.xml_pending_mix_edit.is_none());
    app.select_xml_track("200", "B");
    app.prepare_xml_mix_edit("200", Some("old & set"));
    assert!(
        app.xml_pending_mix_edit.is_none(),
        "absent Mix insertion must refuse"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_tonality_edit_ui_exports_actual_bytes_through_independent_oracle() {
    let (root, source) = temporary_xml();
    std::fs::write(&source, include_bytes!("../../../xtask/tests/fixtures/f02-track-tonality-edit/source.xml")).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_tonality, "old & key");
    app.xml_edit_tonality = "new < key".into();
    app.prepare_xml_tonality_edit("100", Some("old & key"));
    assert!(app.xml_pending_tonality_edit.is_some());
    let output = root.join("edited.xml");
    app.xml_tonality_export_path = output.display().to_string();
    app.cancel_xml_tonality_edit();
    assert!(app.xml_pending_tonality_edit.is_none());
    assert!(!output.exists());
    app.prepare_xml_tonality_edit("100", Some("old & key"));
    app.export_xml_tonality_edit();
    assert_eq!(app.xml_edit_tonality, "new < key");
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::TONALITY, &std::fs::read(&output).unwrap()).is_ok());
    app.select_xml_track("200", "B");
    app.prepare_xml_tonality_edit("200", None);
    assert!(app.xml_pending_tonality_edit.is_none(), "absent Tonality insertion must refuse");
    assert!(app.xml_edit_error.as_deref().unwrap().contains("no Tonality attribute"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_tonality_ui_refuses_invalid_unknown_stale_duplicate_and_invalidates_pending() {
    let (root, source) = temporary_xml();
    let fixture = include_bytes!("../../../xtask/tests/fixtures/f02-track-tonality-edit/source.xml");
    std::fs::write(&source, fixture).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_tonality = "Changed".into();
    app.prepare_xml_tonality_edit("999", Some("old & key"));
    assert!(app.xml_pending_tonality_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_tonality = "bad\u{1}".into();
    app.prepare_xml_tonality_edit("100", Some("old & key"));
    assert!(app.xml_pending_tonality_edit.is_none());
    app.xml_edit_tonality = "Changed".into();
    app.prepare_xml_tonality_edit("100", Some("stale"));
    assert!(app.xml_pending_tonality_edit.is_none());

    let duplicate = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\"");
    std::fs::write(&source, duplicate).unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_tonality = "Changed".into();
    app.prepare_xml_tonality_edit("100", Some("old & key"));
    assert!(app.xml_pending_tonality_edit.is_none(), "duplicate target must refuse");

    std::fs::write(&source, fixture).unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_tonality = "Changed".into();
    app.prepare_xml_tonality_edit("100", Some("old & key"));
    assert!(app.xml_pending_tonality_edit.is_some());
    app.preview_xml();
    assert!(app.xml_pending_tonality_edit.is_none(), "reload must invalidate pending edit");

    let empty = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("Tonality=\"old &amp; key\"", "Tonality=\"\"");
    std::fs::write(&source, empty).unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_tonality, "", "present-empty Tonality must stay distinct from absent");
    app.xml_edit_tonality = "new raw value".into();
    app.prepare_xml_tonality_edit("100", Some(""));
    assert!(app.xml_pending_tonality_edit.is_some(), "present-empty Tonality is editable");
    app.cancel_xml_tonality_edit();
    assert!(app.xml_pending_tonality_edit.is_none());

    std::fs::write(&source, "<broken>").unwrap();
    app.preview_xml();
    assert!(app.xml_preview.as_ref().unwrap().is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_kind_edit_ui_exports_actual_bytes_through_independent_oracle() {
    let (root, source) = temporary_xml();
    std::fs::write(&source, include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml")).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_kind, "old & kind");
    app.xml_edit_kind = "new < kind".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    assert!(app.xml_pending_kind_edit.is_some());
    let output = root.join("edited.xml");
    app.xml_kind_export_path = output.display().to_string();
    app.cancel_xml_kind_edit();
    assert!(app.xml_pending_kind_edit.is_none());
    assert!(!output.exists());
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    app.export_xml_kind_edit();
    assert_eq!(app.xml_edit_kind, "new < kind");
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::KIND, &std::fs::read(&output).unwrap()).is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_kind_edit_ui_refuses_invalid_unknown_stale_duplicate_and_invalidates_pending() {
    let (root, source) = temporary_xml();
    let fixture = include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml");
    std::fs::write(&source, fixture).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_kind = "Changed".into();
    app.prepare_xml_kind_edit("999", Some("old & kind"));
    assert!(app.xml_pending_kind_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_kind = "bad\u{1}".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    assert!(app.xml_pending_kind_edit.is_none());
    app.xml_edit_kind = "Changed".into();
    app.prepare_xml_kind_edit("100", Some("stale"));
    assert!(app.xml_pending_kind_edit.is_none());

    let duplicate = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\"");
    std::fs::write(&source, duplicate).unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_kind = "Changed".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    assert!(app.xml_pending_kind_edit.is_none(), "duplicate target must refuse");

    std::fs::write(&source, fixture).unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_kind = "Changed".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    assert!(app.xml_pending_kind_edit.is_some());
    app.preview_xml();
    assert!(app.xml_pending_kind_edit.is_none(), "reload must invalidate pending edit");

    let empty = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("Kind=\"old &amp; kind\"", "Kind=\"\"");
    std::fs::write(&source, empty).unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_kind, "", "present-empty Kind must stay distinct from absent");
    app.xml_edit_kind = "new raw value".into();
    app.prepare_xml_kind_edit("100", Some(""));
    assert!(app.xml_pending_kind_edit.is_some(), "present-empty Kind is editable");
    app.cancel_xml_kind_edit();
    assert!(app.xml_pending_kind_edit.is_none());

    std::fs::write(&source, "<broken>").unwrap();
    app.preview_xml();
    assert!(app.xml_preview.as_ref().unwrap().is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_kind_edit_ui_exports_and_reopens_present_empty_value() {
    let (root, source) = temporary_xml();
    let mut xml = String::from_utf8(
        include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml").to_vec(),
    )
    .unwrap();
    xml = xml.replace("Kind=\"old &amp; kind\"", "Kind=\"\"");
    std::fs::write(&source, xml).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_kind, "", "present-empty Kind must stay distinct from absent");
    app.xml_edit_kind = "new < kind".into();
    app.prepare_xml_kind_edit("100", Some(""));
    assert!(app.xml_pending_kind_edit.is_some(), "present-empty Kind is editable");
    let output = root.join("edited.xml");
    app.xml_kind_export_path = output.display().to_string();
    app.export_xml_kind_edit();
    let text = String::from_utf8(std::fs::read(&output).unwrap()).unwrap();
    assert!(text.contains("Kind=\"new &lt; kind\""));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_kind_edit_ui_refuses_changed_path_selection_and_proposal() {
    let (root, source) = temporary_xml();
    std::fs::write(&source, include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml")).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_kind = "new < kind".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    let path_output = root.join("changed-path.xml");
    app.xml_kind_export_path = path_output.display().to_string();
    app.xml_path = root.join("other.xml").display().to_string();
    app.export_xml_kind_edit();
    assert!(
        !path_output.exists(),
        "changed editable path must refuse export"
    );
    assert!(app.xml_pending_kind_edit.is_none());

    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_kind = "new < kind".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    let selection_output = root.join("changed-selection.xml");
    app.xml_kind_export_path = selection_output.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_kind_edit();
    assert!(
        !selection_output.exists(),
        "changed selection must refuse export"
    );

    app.select_xml_track("100", "A");
    app.xml_edit_kind = "new < kind".into();
    app.prepare_xml_kind_edit("100", Some("old & kind"));
    let proposal_output = root.join("changed-proposal.xml");
    app.xml_kind_export_path = proposal_output.display().to_string();
    app.xml_edit_kind = "changed after preview".into();
    app.export_xml_kind_edit();
    assert!(
        !proposal_output.exists(),
        "changed proposal must refuse export"
    );
    assert!(app.xml_pending_kind_edit.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_path_change_discards_pending_kind_grouping_label_and_name_edits() {
    let (root, source) = temporary_xml();
    let fixtures: [(&[u8], &str); 4] = [
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml"), "kind"),
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"), "grouping"),
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"), "label"),
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml"), "name"),
    ];
    for (fixture, field) in fixtures {
        std::fs::write(&source, fixture).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        match field {
            "kind" => {
                app.xml_edit_kind = "new".into();
                app.prepare_xml_kind_edit("100", Some("old & kind"));
                assert!(app.xml_pending_kind_edit.is_some());
            }
            "grouping" => {
                app.xml_edit_grouping = "new".into();
                app.prepare_xml_grouping_edit("100", Some("old & crate"));
                assert!(app.xml_pending_grouping_edit.is_some());
            }
            "label" => {
                app.xml_edit_label = "new".into();
                app.prepare_xml_label_edit("100", Some("old & imprint"));
                assert!(app.xml_pending_label_edit.is_some());
            }
            _ => {
                app.xml_edit_name = "new".into();
                app.prepare_xml_name_edit("100", "A");
                assert!(app.xml_pending_edit.is_some());
            }
        }
        app.xml_path = root.join("other.xml").display().to_string();
        app.on_xml_source_changed();
        assert!(
            app.xml_pending_kind_edit.is_none()
                && app.xml_pending_grouping_edit.is_none()
                && app.xml_pending_label_edit.is_none()
                && app.xml_pending_edit.is_none(),
            "source path change must discard pending {field} edit"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reload_and_source_change_discard_composer_grouping_label_and_playlist_move() {
    let (root, source) = temporary_xml();
    let fixtures: [(&[u8], &str); 3] = [
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"), "composer"),
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"), "grouping"),
        (include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"), "label"),
    ];
    for (fixture, field) in fixtures {
        std::fs::write(&source, fixture).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        match field {
            "composer" => {
                app.xml_edit_composer = "new".into();
                app.prepare_xml_composer_edit("100", Some("old & composer"));
                assert!(app.xml_pending_composer_edit.is_some());
            }
            "grouping" => {
                app.xml_edit_grouping = "new".into();
                app.prepare_xml_grouping_edit("100", Some("old & crate"));
                assert!(app.xml_pending_grouping_edit.is_some());
            }
            _ => {
                app.xml_edit_label = "new".into();
                app.prepare_xml_label_edit("100", Some("old & imprint"));
                assert!(app.xml_pending_label_edit.is_some());
            }
        }
        app.preview_xml();
        assert!(
            app.xml_pending_composer_edit.is_none()
                && app.xml_pending_grouping_edit.is_none()
                && app.xml_pending_label_edit.is_none(),
            "reload must discard pending {field} edit"
        );
    }

    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-playlist-move/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let target = app.xml_preview.as_ref().unwrap().as_ref().unwrap().folders.iter()
        .find(|f| f.name == "Target").unwrap().ordinal;
    let prepare = |app: &mut LibraryApp| {
        app.xml_selected_playlist = Some(0);
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        assert!(app.xml_pending_move.is_some(), "{:?}", app.xml_edit_error);
    };
    prepare(&mut app);
    app.preview_xml();
    assert!(app.xml_pending_move.is_none(), "reload must discard pending playlist move");
    prepare(&mut app);
    app.xml_path = root.join("other.xml").display().to_string();
    app.on_xml_source_changed();
    assert!(app.xml_pending_move.is_none(), "source change must discard pending playlist move");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_tonality_ui_refuses_changed_path_selection_proposal_stale_source_and_overwrite() {
    let (root, source) = temporary_xml();
    let fixture =
        include_bytes!("../../../xtask/tests/fixtures/f02-track-tonality-edit/source.xml");
    std::fs::write(&source, fixture).unwrap();
    let mut app = app();
    let prepare = |app: &mut LibraryApp| {
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_tonality = "new < key".into();
        app.prepare_xml_tonality_edit("100", Some("old & key"));
        assert!(app.xml_pending_tonality_edit.is_some());
    };

    prepare(&mut app);
    let path_output = root.join("changed-path.xml");
    app.xml_tonality_export_path = path_output.display().to_string();
    app.xml_path = root.join("other.xml").display().to_string();
    app.export_xml_tonality_edit();
    assert!(!path_output.exists(), "changed editable path must refuse export");
    assert!(app.xml_pending_tonality_edit.is_none());

    prepare(&mut app);
    let selection_output = root.join("changed-selection.xml");
    app.xml_tonality_export_path = selection_output.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_tonality_edit();
    assert!(!selection_output.exists(), "changed selection must refuse export");

    prepare(&mut app);
    let proposal_output = root.join("changed-proposal.xml");
    app.xml_tonality_export_path = proposal_output.display().to_string();
    app.xml_edit_tonality = "changed after preview".into();
    app.export_xml_tonality_edit();
    assert!(!proposal_output.exists(), "changed proposal must refuse export");
    assert!(app.xml_pending_tonality_edit.is_none());

    prepare(&mut app);
    let stale_output = root.join("stale-source.xml");
    app.xml_tonality_export_path = stale_output.display().to_string();
    let mut changed = fixture.to_vec();
    changed.extend_from_slice(b"\n<!-- changed after preparation -->\n");
    std::fs::write(&source, &changed).unwrap();
    app.export_xml_tonality_edit();
    assert!(!stale_output.exists(), "changed source bytes must refuse export");
    assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
    assert_eq!(std::fs::read(&source).unwrap(), changed);
    std::fs::write(&source, fixture).unwrap();

    prepare(&mut app);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_tonality_export_path = existing.display().to_string();
    app.export_xml_tonality_edit();
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep", "export must not overwrite");
    assert_eq!(std::fs::read(&source).unwrap(), fixture);

    let output = root.join("edited.xml");
    app.xml_tonality_export_path = output.display().to_string();
    app.export_xml_tonality_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::TONALITY, &bytes).is_ok());
    let mutated = String::from_utf8(bytes.clone())
        .unwrap()
        .replacen("new &lt; key", "new &lt; KEY", 1);
    assert_ne!(mutated.as_bytes(), bytes.as_slice());
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::TONALITY, mutated.as_bytes()).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_kind_ui_refuses_absent_insertion_stale_source_and_overwrite_with_oracle_mutation() {
    let (root, source) = temporary_xml();
    let fixture = include_bytes!("../../../xtask/tests/fixtures/f02-track-kind-edit/source.xml");
    std::fs::write(&source, fixture).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("200", "B");
    app.xml_edit_kind = "inserted".into();
    app.prepare_xml_kind_edit("200", None);
    assert!(app.xml_pending_kind_edit.is_none(), "absent Kind insertion must refuse");
    assert!(app.xml_edit_error.as_deref().unwrap().contains("no Kind attribute"));

    let prepare = |app: &mut LibraryApp| {
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_kind = "new < kind".into();
        app.prepare_xml_kind_edit("100", Some("old & kind"));
        assert!(app.xml_pending_kind_edit.is_some());
    };
    prepare(&mut app);
    let stale_output = root.join("stale-source.xml");
    app.xml_kind_export_path = stale_output.display().to_string();
    let mut changed = fixture.to_vec();
    changed.extend_from_slice(b"\n<!-- changed after preparation -->\n");
    std::fs::write(&source, &changed).unwrap();
    app.export_xml_kind_edit();
    assert!(!stale_output.exists(), "changed source bytes must refuse export");
    assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
    std::fs::write(&source, fixture).unwrap();

    prepare(&mut app);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_kind_export_path = existing.display().to_string();
    app.export_xml_kind_edit();
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep", "export must not overwrite");
    assert_eq!(std::fs::read(&source).unwrap(), fixture);

    let output = root.join("edited.xml");
    app.xml_kind_export_path = output.display().to_string();
    app.export_xml_kind_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::KIND, &bytes).is_ok());
    let mutated = String::from_utf8(bytes.clone())
        .unwrap()
        .replacen("new &lt; kind", "new &lt; KIND", 1);
    assert_ne!(mutated.as_bytes(), bytes.as_slice());
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::KIND, mutated.as_bytes()).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_exports_previewed_bytes_through_independent_oracle() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_mix, "old & set");
    app.xml_edit_mix = "new < set".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let cancelled = root.join("cancelled.xml");
    app.xml_mix_export_path = cancelled.display().to_string();
    app.cancel_xml_mix_edit();
    assert!(app.xml_pending_mix_edit.is_none());
    assert!(!cancelled.exists());
    app.prepare_xml_mix_edit("100", Some("old & set"));
    app.xml_mix_export_path.clear();
    app.export_xml_mix_edit();
    assert!(app.xml_pending_mix_edit.is_some());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    app.xml_mix_export_path = source.display().to_string();
    app.export_xml_mix_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must refuse");
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_mix_export_path = existing.display().to_string();
    app.export_xml_mix_edit();
    assert!(app.xml_edit_error.is_some(), "existing output must refuse");
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    app.xml_mix_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_mix_edit();
    assert!(app.xml_edit_error.is_some());
    assert!(app.notice.is_empty());
    app.xml_mix_export_path = root.join("stale-source.xml").display().to_string();
    std::fs::write(&source, b"<changed/>").unwrap();
    app.export_xml_mix_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.xml_preview =
        Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
    app.xml_document_source = Some(source.clone());
    app.select_xml_track("100", "A");
    app.xml_edit_mix = "new < set".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    let prepared = app
        .xml_pending_mix_edit
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    let output = root.join("edited.xml");
    app.xml_mix_export_path = output.display().to_string();
    app.export_xml_mix_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::MIX, &bytes)
        .expect("independent F-02 Mix oracle accepts UI bytes");
    let mutated = String::from_utf8(bytes).unwrap().replacen(
        "Mix=\"new &lt; set\"",
        "Mix=\"altered\"",
        1,
    );
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::MIX, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .mix
            .as_deref(),
        Some("new < set")
    );
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "200")
            .unwrap()
            .mix,
        None
    );
    assert!(app.xml_pending_mix_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let mix = "new < style & set\tfirst\nsecond\r";
    app.xml_edit_mix = mix.into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    app.xml_mix_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_mix_edit();
    let output = PathBuf::from(&app.xml_mix_export_path);
    let text = std::fs::read_to_string(output).unwrap();
    assert!(text.contains("Mix=\"new &lt; style &amp; set&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .mix
            .as_deref(),
        Some(mix)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_refuses_changed_path_selection_and_proposal() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_mix = "new < set".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    let path_output = root.join("changed-path.xml");
    app.xml_mix_export_path = path_output.display().to_string();
    app.xml_path = root.join("other.xml").display().to_string();
    app.export_xml_mix_edit();
    assert!(
        !path_output.exists(),
        "changed editable path must refuse export"
    );
    assert!(app.xml_pending_mix_edit.is_none());

    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_mix = "new < set".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    let selection_output = root.join("changed-selection.xml");
    app.xml_mix_export_path = selection_output.display().to_string();
    app.select_xml_track("200", "B");
    app.export_xml_mix_edit();
    assert!(
        !selection_output.exists(),
        "changed selection must refuse export"
    );

    app.select_xml_track("100", "A");
    app.xml_edit_mix = "new < set".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    let proposal_output = root.join("changed-proposal.xml");
    app.xml_mix_export_path = proposal_output.display().to_string();
    app.xml_edit_mix = "changed after preview".into();
    app.export_xml_mix_edit();
    assert!(
        !proposal_output.exists(),
        "changed proposal must refuse export"
    );
    assert!(app.xml_pending_mix_edit.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_exports_and_reopens_present_empty_value() {
    let (root, source) = temporary_xml();
    let xml = String::from_utf8(
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml").to_vec(),
    )
    .unwrap()
    .replace("Mix=\"old &amp; set\"", "Mix=\"\"");
    std::fs::write(&source, xml).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_mix, "");
    app.xml_edit_mix = "restored".into();
    app.prepare_xml_mix_edit("100", Some(""));
    assert!(
        app.xml_pending_mix_edit.is_some(),
        "present-empty Mix must be editable"
    );
    app.xml_mix_export_path = root.join("present-empty.xml").display().to_string();
    app.export_xml_mix_edit();
    assert_eq!(app.xml_edit_mix, "restored");
    app.preview_xml();
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .mix
            .as_deref(),
        Some("restored")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_mix_edit_ui_refuses_malformed_xml_and_discards_pending_on_reload() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_mix = "Changed".into();
    app.prepare_xml_mix_edit("100", Some("old & set"));
    assert!(app.xml_pending_mix_edit.is_some());
    app.preview_xml();
    assert!(
        app.xml_pending_mix_edit.is_none(),
        "reload must discard pending Mix edit"
    );

    std::fs::write(
        &source,
        "<?xml version=\"1.0\"?><DJ_PLAYLISTS><COLLECTION><TRACK",
    )
    .unwrap();
    app.preview_xml();
    assert!(
        app.xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .is_none()
    );
    app.select_xml_track("100", "A");
    app.prepare_xml_mix_edit("100", Some("old & set"));
    assert!(
        app.xml_pending_mix_edit.is_none(),
        "malformed XML must refuse preparation"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_label_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("999", "Unknown");
    app.xml_edit_label = "Changed".into();
    app.prepare_xml_label_edit("999", Some("old & imprint"));
    assert!(app.xml_pending_label_edit.is_none());

    app.select_xml_track("100", "A");
    app.xml_edit_label = "bad\u{1}".into();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    assert!(app.xml_pending_label_edit.is_none());

    app.xml_edit_label = "Changed".into();
    app.prepare_xml_label_edit("100", Some("stale"));
    assert!(app.xml_pending_label_edit.is_none());

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\""),
    )
    .unwrap();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_label = "Changed".into();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    assert!(
        app.xml_pending_label_edit.is_none(),
        "duplicate target must refuse"
    );

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("Label=\"old &amp; imprint\"", "Label=\"\""),
    )
    .unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(
        app.xml_edit_label, "",
        "present-empty Label must stay editable"
    );
    app.xml_edit_label = "Restored".into();
    app.prepare_xml_label_edit("100", Some(""));
    assert!(
        app.xml_pending_label_edit.is_some(),
        "present-empty Label is editable"
    );
    app.cancel_xml_label_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_genre_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let genre = "new < style & mix\tfirst\nsecond\r";
    app.xml_edit_genre = genre.into();
    app.prepare_xml_genre_edit("100", Some("old & style"));
    assert!(app.xml_pending_genre_edit.is_some());
    app.xml_genre_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_genre_edit();
    let output = PathBuf::from(&app.xml_genre_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("Genre=\"new &lt; style &amp; mix&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .genre
            .as_deref(),
        Some(genre)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_label_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let label = "new < imprint & mix\tfirst\nsecond\r";
    app.xml_edit_label = label.into();
    app.prepare_xml_label_edit("100", Some("old & imprint"));
    assert!(app.xml_pending_label_edit.is_some());
    app.xml_label_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_label_edit();
    let output = PathBuf::from(&app.xml_label_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("Label=\"new &lt; imprint &amp; mix&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .label
            .as_deref(),
        Some(label)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_album_edit_ui_exports_previewed_bytes_through_independent_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-track-album-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_album, "old & record");
    app.xml_edit_album = "new < record".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    let prepared_bytes = app
        .xml_pending_album_edit
        .as_ref()
        .expect("prepared Album edit")
        .prepared
        .bytes()
        .to_vec();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

    app.xml_album_export_path.clear();
    app.export_xml_album_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("choose a new XML destination")
    );
    assert!(app.xml_pending_album_edit.is_some());

    let cancelled = root.join("cancelled.xml");
    app.xml_album_export_path = cancelled.display().to_string();
    app.cancel_xml_album_edit();
    assert!(app.xml_pending_album_edit.is_none());
    assert!(!cancelled.exists(), "cancel must not write output");

    app.prepare_xml_album_edit("100", Some("old & record"));
    app.xml_album_export_path = source.display().to_string();
    app.export_xml_album_edit();
    assert!(app.xml_edit_error.is_some(), "source alias must be refused");
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve").unwrap();
    app.xml_album_export_path = existing.display().to_string();
    app.export_xml_album_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "output refusal must be surfaced"
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
    assert_eq!(std::fs::read(&source).unwrap(), original);

    app.xml_album_export_path = root.join("missing/output.xml").display().to_string();
    app.export_xml_album_edit();
    assert!(
        app.xml_edit_error.is_some(),
        "write failure must be surfaced"
    );
    assert!(app.notice.is_empty(), "failure must not claim success");

    std::fs::write(&source, b"<changed/>").unwrap();
    app.xml_album_export_path = root.join("stale-source.xml").display().to_string();
    app.export_xml_album_edit();
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("source changed")
    );
    assert!(!root.join("stale-source.xml").exists());
    std::fs::write(&source, &original).unwrap();
    app.prepare_xml_album_edit("100", Some("old & record"));

    let output = root.join("edited.xml");
    app.xml_album_export_path = output.display().to_string();
    app.export_xml_album_edit();
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(bytes, prepared_bytes);
    f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::ALBUM, &bytes)
        .expect("independent F-02 oracle accepts only selected Album edit");
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Album=\"new &lt; record\"",
        "Album=\"altered\"",
        1,
    );
    assert_ne!(mutated.as_bytes(), bytes.as_slice());
    assert!(f02_track_attribute_edit_oracle::check(&f02_track_attribute_edit_oracle::ALBUM, mutated.as_bytes()).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .album
            .as_deref(),
        Some("new < record")
    );
    assert_eq!(
        reopened
            .tracks
            .iter()
            .find(|track| track.id == "200")
            .unwrap()
            .album,
        None,
        "absent Album remains absent on other tracks"
    );
    assert!(app.xml_pending_album_edit.is_none());
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_composer_edit_ui_refuses_changed_editable_path_without_output() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_composer = "new < composer".into();
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    assert!(app.xml_pending_composer_edit.is_some());
    let output = root.join("stale-path.xml");
    app.xml_composer_export_path = output.display().to_string();
    app.xml_path = root.join("different.xml").display().to_string();
    app.export_xml_composer_edit();
    assert!(app.xml_pending_composer_edit.is_none());
    assert!(app.xml_edit_error.as_deref().unwrap().contains("source"));
    assert!(
        !output.exists(),
        "changed editable XML path must refuse export"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_grouping_edit_ui_refuses_changed_editable_path_without_output() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_grouping = "new < crate".into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    assert!(app.xml_pending_grouping_edit.is_some());
    let output = root.join("stale-path.xml");
    app.xml_grouping_export_path = output.display().to_string();
    app.xml_path = root.join("different.xml").display().to_string();
    app.export_xml_grouping_edit();
    assert!(app.xml_pending_grouping_edit.is_none());
    assert!(app.xml_edit_error.as_deref().unwrap().contains("source"));
    assert!(
        !output.exists(),
        "changed editable XML path must refuse export"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_composer_edit_ui_allows_present_empty_but_not_absent_insertion() {
    let (root, source) = temporary_xml();
    let fixture =
        include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml");
    let xml = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("Composer=\"old &amp; composer\"", "Composer=\"\"");
    std::fs::write(&source, xml).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_composer, "");
    app.xml_edit_composer = "Composed".into();
    app.prepare_xml_composer_edit("100", Some(""));
    assert!(app.xml_pending_composer_edit.is_some());
    app.xml_composer_export_path = root.join("empty-edited.xml").display().to_string();
    app.export_xml_composer_edit();
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .composer
            .as_deref(),
        Some("Composed")
    );
    app.prepare_xml_composer_edit("200", None);
    assert!(
        app.xml_pending_composer_edit.is_none(),
        "absent Composer insertion is refused"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_grouping_edit_ui_allows_present_empty_but_not_absent_insertion() {
    let (root, source) = temporary_xml();
    let fixture =
        include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml");
    let xml = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("Grouping=\"old &amp; crate\"", "Grouping=\"\"");
    std::fs::write(&source, xml).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_grouping, "");
    app.xml_edit_grouping = "Composed".into();
    app.prepare_xml_grouping_edit("100", Some(""));
    assert!(app.xml_pending_grouping_edit.is_some());
    app.xml_grouping_export_path = root.join("empty-edited.xml").display().to_string();
    app.export_xml_grouping_edit();
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.id == "100")
            .unwrap()
            .grouping
            .as_deref(),
        Some("Composed")
    );
    app.prepare_xml_grouping_edit("200", None);
    assert!(
        app.xml_pending_grouping_edit.is_none(),
        "absent Grouping insertion is refused"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_composer_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let composer = "new < composer & mix\tfirst\nsecond\r";
    app.xml_edit_composer = composer.into();
    app.prepare_xml_composer_edit("100", Some("old & composer"));
    assert!(app.xml_pending_composer_edit.is_some());
    app.xml_composer_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_composer_edit();
    let output = PathBuf::from(&app.xml_composer_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("Composer=\"new &lt; composer &amp; mix&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .composer
            .as_deref(),
        Some(composer)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_grouping_edit_ui_round_trips_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let grouping = "new < crate & mix\tfirst\nsecond\r";
    app.xml_edit_grouping = grouping.into();
    app.prepare_xml_grouping_edit("100", Some("old & crate"));
    assert!(app.xml_pending_grouping_edit.is_some());
    app.xml_grouping_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_grouping_edit();
    let output = PathBuf::from(&app.xml_grouping_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("Grouping=\"new &lt; crate &amp; mix&#9;first&#10;second&#13;\""));
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .grouping
            .as_deref(),
        Some(grouping)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_album_edit_ui_guards_absent_stale_proposal_and_invalid_xml() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();

    app.select_xml_track("200", "B");
    app.prepare_xml_album_edit("200", None);
    assert!(app.xml_pending_album_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("no Album attribute")
    );

    app.select_xml_track("999", "Unknown");
    app.prepare_xml_album_edit("999", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_none());
    app.select_xml_track("100", "A");
    app.prepare_xml_album_edit("100", Some("stale"));
    assert!(app.xml_pending_album_edit.is_none());
    app.xml_edit_album = "bad\u{1}".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_none());

    app.xml_edit_album = "Changed".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_some());
    app.xml_edit_album.push('!');
    app.xml_album_export_path = root.join("changed-proposal.xml").display().to_string();
    app.export_xml_album_edit();
    assert!(app.xml_pending_album_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("proposed value changed")
    );
    assert!(!root.join("changed-proposal.xml").exists());

    app.xml_edit_album = "Changed".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    let preview = app.xml_preview.as_mut().unwrap().as_mut().unwrap();
    preview
        .tracks
        .iter_mut()
        .find(|track| track.id == "100")
        .unwrap()
        .album = Some("newer current value".into());
    app.xml_album_export_path = root.join("changed-old-value.xml").display().to_string();
    app.export_xml_album_edit();
    assert!(app.xml_pending_album_edit.is_none());
    assert!(
        app.xml_edit_error
            .as_deref()
            .unwrap()
            .contains("proposed value changed")
    );
    assert!(!root.join("changed-old-value.xml").exists());

    app.xml_edit_album = "Changed".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_some());
    app.select_xml_track("200", "B");
    assert!(
        app.xml_pending_album_edit.is_none(),
        "track change invalidates pending edit"
    );

    app.select_xml_track("100", "A");
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_some());
    app.xml_path = root.join("other.xml").display().to_string();
    app.on_xml_source_changed();
    assert!(
        app.xml_pending_album_edit.is_none(),
        "source path change invalidates pending edit"
    );

    std::fs::write(
        &source,
        String::from_utf8(
            include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml")
                .to_vec(),
        )
        .unwrap()
        .replace("TrackID=\"200\"", "TrackID=\"100\""),
    )
    .unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    app.xml_edit_album = "Changed".into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(
        app.xml_pending_album_edit.is_none(),
        "duplicate target ID must refuse preparation"
    );

    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
    )
    .unwrap();
    let empty_fixture = std::fs::read(&source)
        .unwrap()
        .windows(b"Album=\"old &amp; record\"".len())
        .position(|window| window == b"Album=\"old &amp; record\"")
        .expect("fixture Album attribute");
    let mut bytes = std::fs::read(&source).unwrap();
    bytes.splice(
        empty_fixture..empty_fixture + b"Album=\"old &amp; record\"".len(),
        b"Album=\"\"".iter().copied(),
    );
    std::fs::write(&source, bytes).unwrap();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    assert_eq!(app.xml_edit_album, "");
    app.xml_edit_album = "restored".into();
    app.prepare_xml_album_edit("100", Some(""));
    assert!(
        app.xml_pending_album_edit.is_some(),
        "present-empty Album is editable"
    );
    app.cancel_xml_album_edit();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn track_album_edit_ui_round_trips_xml_entities_and_whitespace() {
    let (root, source) = temporary_xml();
    std::fs::write(
        &source,
        include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
    )
    .unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.select_xml_track("100", "A");
    let album = "new < record & mix\tfirst\nsecond\r";
    app.xml_edit_album = album.into();
    app.prepare_xml_album_edit("100", Some("old & record"));
    assert!(app.xml_pending_album_edit.is_some());
    app.xml_album_export_path = root.join("escaped.xml").display().to_string();
    app.export_xml_album_edit();
    let output = PathBuf::from(&app.xml_album_export_path);
    let bytes = std::fs::read(&output).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("Album=\"new &lt; record &amp; mix&#9;first&#10;second&#13;\""));
    f02_track_attribute_edit_oracle::check_with(&f02_track_attribute_edit_oracle::ALBUM, &bytes, album)
        .expect("independent F-02 oracle preserves escaped Album text");
    assert_eq!(
        app.xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|track| track.id == "100")
            .unwrap()
            .album
            .as_deref(),
        Some(album)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn previewing_another_document_clears_pending_edit() {
    let (root, source) = temporary_xml();
    let second = root.join("other.xml");
    std::fs::write(&second, std::fs::read(&source).unwrap()).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    app.xml_edit_name = "New".into();
    app.prepare_xml_name_edit("100", "A");
    assert!(app.xml_pending_edit.is_some());
    app.xml_path = second.display().to_string();
    app.preview_xml();
    assert!(app.xml_pending_edit.is_none());
    assert_eq!(app.xml_selected_track, None);
    assert!(app.xml_edit_error.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_source_guard_is_shown_as_preparation_error() {
    let (root, source) = temporary_xml();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    std::fs::write(&source, br#"<DJ_PLAYLISTS Version="1.0.0"><COLLECTION Entries="1"><TRACK TrackID="7" Name="Changed" Location="x"/></COLLECTION></DJ_PLAYLISTS>"#).unwrap();
    app.xml_edit_name = "New".into();
    app.prepare_xml_name_edit("7", "Old");
    let error = app.xml_edit_error.as_ref().expect("guard refusal surfaced");
    assert!(
        error.starts_with("Track Name preparation refused:"),
        "{error}"
    );
    assert!(app.xml_pending_edit.is_none());
    let ctx = egui::Context::default();
    let tokens = ThemeMode::TokyoNight.tokens();
    let error = app.xml_edit_error.clone().unwrap();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            StatusBadge {
                label: &error,
                kind: StatusKind::Error,
            }
            .show(ui, tokens);
        });
    });
    let labels = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        labels
            .iter()
            .any(|label| label.contains("Track Name preparation refused")),
        "{labels:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn toolbar_controls_fit_narrow_and_wide_viewports() {
    // Renders the production toolbar (both the 1-group and 2-group branch)
    // and checks every interactive control from its AccessKit bounds.
    for width in [800.0, 1440.0] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = app();
        app.db_path = std::path::PathBuf::from(
            "/a/very/long/catalog/path/that/must/not/push/actions/offscreen/catalog.sqlite",
        );
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0))),
            ..Default::default()
        };
        let mut rects = Vec::new();
        for _ in 0..2 {
            let output = ctx.run(input(), |ctx| {
                let tokens = app.active_tokens();
                egui::TopBottomPanel::top("test-toolbar").show(ctx, |ui| app.toolbar(ui, ctx, tokens));
            });
            let update = output.platform_output.accesskit_update.expect("AccessKit update");
            rects = update
                .nodes
                .iter()
                .filter(|(_, node)| {
                    matches!(
                        node.role(),
                        egui::accesskit::Role::Button
                            | egui::accesskit::Role::TextInput
                            | egui::accesskit::Role::ComboBox
                    )
                })
                .filter_map(|(_, node)| node.bounds())
                .map(|r| egui::Rect::from_min_max(egui::pos2(r.x0 as f32, r.y0 as f32), egui::pos2(r.x1 as f32, r.y1 as f32)))
                .collect::<Vec<_>>();
        }
        assert!(rects.len() >= 8, "expected the production toolbar controls at {width}px: {rects:?}");
        let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
        assert!(
            rects.iter().all(|rect| content.contains_rect(*rect)),
            "toolbar control clipped at {width}px: {rects:?}"
        );
        assert!(
            rects.iter().enumerate().all(|(i, a)| rects[i + 1..].iter().all(|b| !a.intersects(*b))),
            "toolbar controls overlap at {width}px: {rects:?}"
        );
    }
}

#[test]
fn day_to_custom_combo_selection_initializes_tokens_and_opens_editor() {
    let mut app = app();
    app.theme = ThemeMode::TokyoDay;
    app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
    app.custom_tokens.secondary = egui::Color32::from_rgb(4, 5, 6);
    let ctx = egui::Context::default();
    let render = |ctx: &egui::Context, app: &mut LibraryApp, input| {
        let mut result = None;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                result = Some(app.theme_selection_ui(ui));
            });
        });
        result.unwrap()
    };
    let (combo_rect, _) = render(&ctx, &mut app, egui::RawInput::default());
    let click = |pos| egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..Default::default()
    };
    let _ = render(&ctx, &mut app, click(combo_rect.center()));
    let (_, option_rects) = render(&ctx, &mut app, egui::RawInput::default());
    let custom_pos = option_rects.unwrap()[2].center();
    let _ = render(&ctx, &mut app, click(custom_pos));
    assert_eq!(app.theme, ThemeMode::Custom);
    assert!(app.show_theme_editor);
    assert_eq!(app.custom_tokens, ThemeMode::TokyoDay.tokens());
}

#[test]
fn theme_controls_have_accesskit_names_and_control_roles() {
    let mut app = app();
    app.theme = ThemeMode::Custom;
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.theme_selection_ui(ui);
            app.show_palette_fields(ui, egui::Color32::RED);
        });
    });
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update");
    let nodes = update
        .nodes
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();
    let assert_named = |name, role| {
        assert!(
            nodes.values().any(|control| {
                control.role() == role
                    && control.labelled_by().iter().any(|label_id| {
                        nodes
                            .get(label_id)
                            .is_some_and(|label| label.value() == Some(name))
                    })
            }),
            "missing {role:?} labelled by {name:?}"
        );
    };
    assert_named("Theme (Ctrl+Alt+1/2/3)", egui::accesskit::Role::ComboBox);
    assert_named("Primary", egui::accesskit::Role::TextInput);
    assert_named("Secondary", egui::accesskit::Role::TextInput);
}

#[test]
fn shortcut_raw_input_selects_custom_and_preserves_existing_custom_tokens() {
    let mut app = app();
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input.modifiers = egui::Modifiers {
        ctrl: true,
        alt: true,
        ..Default::default()
    };
    input.events.push(egui::Event::Key {
        key: egui::Key::Num3,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: input.modifiers,
    });
    let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
    assert_eq!(app.theme, ThemeMode::Custom);
    assert!(app.show_theme_editor);
    app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
    app.show_theme_editor = false;
    let mut input = egui::RawInput::default();
    input.modifiers = egui::Modifiers {
        ctrl: true,
        alt: true,
        ..Default::default()
    };
    input.events.push(egui::Event::Key {
        key: egui::Key::Num3,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: input.modifiers,
    });
    let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
    assert!(app.show_theme_editor);
    assert_eq!(app.custom_tokens.primary, egui::Color32::from_rgb(1, 2, 3));
}

#[test]
fn raw_text_events_commit_field_and_storage_round_trip() {
    use std::collections::HashMap;
    #[derive(Default)]
    struct Store(HashMap<String, String>);
    impl eframe::Storage for Store {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn flush(&mut self) {}
    }
    let mut app = app();
    app.select_theme(ThemeMode::Custom);
    let ctx = egui::Context::default();
    let run_editor = |ctx: &egui::Context, app: &mut LibraryApp, input| {
        ctx.run(input, |ctx| {
            egui::CentralPanel::default()
                .show(ctx, |ui| app.show_palette_fields(ui, egui::Color32::RED));
        })
    };
    app.primary_hex.clear();
    let _ = run_editor(&ctx, &mut app, egui::RawInput::default());
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
    let mut input = egui::RawInput::default();
    input.modifiers = egui::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    input.events.push(egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: input.modifiers,
    });
    let _ = run_editor(&ctx, &mut app, input);
    let mut input = egui::RawInput::default();
    input.events.push(egui::Event::Text("#123456".into()));
    let _ = run_editor(&ctx, &mut app, input);
    assert_eq!(app.primary_hex, "#123456");
    let mut enter = egui::RawInput::default();
    enter.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    });
    let _ = run_editor(&ctx, &mut app, enter);
    assert_eq!(
        app.custom_tokens.primary,
        egui::Color32::from_rgb(0x12, 0x34, 0x56)
    );
    assert_eq!(app.theme, ThemeMode::Custom);
    let mut store = Store::default();
    eframe::App::save(&mut app, &mut store);
    assert_eq!(
        ThemeMode::load(Some(&store)),
        (app.theme, app.custom_tokens)
    );

    // Invalid focused input is rejected on Enter and leaves the committed token intact.
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
    let mut invalid = egui::RawInput::default();
    invalid.events.extend([
        egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
        },
        egui::Event::Text("#12".into()),
        egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        },
    ]);
    let _ = run_editor(&ctx, &mut app, invalid);
    assert_eq!(
        app.custom_tokens.primary,
        egui::Color32::from_rgb(0x12, 0x34, 0x56)
    );
    assert!(app.accent_error.is_some());
}

#[test]
fn palette_swatches_and_reset_keep_text_synchronized() {
    let mut app = app();
    app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
    app.sync_accent_fields();
    assert_eq!(app.primary_hex, "#010203");
    app.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
    app.sync_accent_fields();
    assert_eq!(
        app.primary_hex,
        color_hex(ThemeMode::TokyoDay.tokens().primary)
    );
    assert_eq!(
        app.secondary_hex,
        color_hex(ThemeMode::TokyoDay.tokens().secondary)
    );
}
#[test]
fn empty_folder_delete_ui_exports_prepared_bytes_and_reopens_without_stale_selection() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-delete-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-empty-folder-delete/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let mut index = preview
        .folders
        .iter()
        .position(|f| f.name == "Empty" && f.path == ["ROOT", "Sets", "Empty"])
        .unwrap();
    let mut same_named_other_parent = preview
        .folders
        .iter()
        .find(|f| f.path == ["ROOT", "Sets"])
        .unwrap()
        .clone();
    same_named_other_parent.path = vec!["ROOT".into(), "Other".into(), "Sets".into()];
    same_named_other_parent.count = Some("99".into());
    app.xml_preview
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .folders
        .insert(0, same_named_other_parent);
    index += 1;
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    assert!(
        app.xml_pending_folder_delete.is_some(),
        "prepare error: {:?}",
        app.xml_edit_error
    );
    let bytes = app
        .xml_pending_folder_delete
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.cancel_xml_folder_delete();
    assert!(app.xml_pending_folder_delete.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    app.xml_folder_delete_export_path = source.display().to_string();
    app.export_xml_folder_delete();
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "source alias must not overwrite input"
    );
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    let changed_source = root.join("changed.xml");
    app.xml_folder_delete_export_path = changed_source.display().to_string();
    let mut changed = original.clone();
    changed.push(b' ');
    std::fs::write(&source, &changed).unwrap();
    app.export_xml_folder_delete();
    assert!(
        !changed_source.exists(),
        "source revalidation rejects stale preparation"
    );
    assert!(
        app.xml_pending_folder_delete.is_none(),
        "stale source invalidates the pending proposal"
    );
    assert_eq!(std::fs::read(&source).unwrap(), changed);
    std::fs::write(&source, &original).unwrap();
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    app.xml_selected_folder_delete = Some(0);
    app.export_xml_folder_delete();
    assert!(
        app.xml_pending_folder_delete.is_none(),
        "changed selection invalidates"
    );
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    app.xml_folder_delete_export_path = existing.display().to_string();
    app.export_xml_folder_delete();
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        b"keep",
        "existing destination is preserved"
    );
    app.xml_selected_folder_delete = Some(index);
    app.prepare_xml_folder_delete();
    let output = root.join("deleted.xml");
    app.xml_folder_delete_export_path = output.display().to_string();
    app.export_xml_folder_delete();
    assert_eq!(std::fs::read(&output).unwrap(), bytes);
    f02_empty_folder_delete_oracle::check(&bytes)
        .expect("actual UI bytes satisfy independent F-02 oracle");
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert!(
        !reopened
            .folders
            .iter()
            .any(|f| f.path == ["ROOT", "Sets", "Empty"])
    );
    assert_eq!(
        reopened
            .folders
            .iter()
            .find(|f| f.path == ["ROOT", "Sets"])
            .unwrap()
            .count
            .as_deref(),
        Some("2")
    );
    assert_eq!(app.xml_selected_folder_delete, None);
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_document_source = Some(output.clone());
    app.xml_selected_folder_delete = Some(0);
    app.prepare_xml_folder_delete();
    app.xml_folder_delete_export_path = output.display().to_string();
    app.export_xml_folder_delete();
    assert_eq!(
        std::fs::read(&output).unwrap(),
        bytes,
        "source alias cannot overwrite"
    );
    assert_eq!(std::fs::read(existing).unwrap(), b"keep");
    let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
        "Name=\"Sets\" Count=\"2\"",
        "Name=\"Sets\" Count=\"3\"",
        1,
    );
    assert!(f02_empty_folder_delete_oracle::check(mutated.as_bytes()).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_rename_ui_refuses_stale_and_invalid_exports_without_overwriting() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-rename-guards-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../xtask/tests/fixtures/f02-folder-rename/source.xml"
    ))
    .unwrap();
    let with_sibling = String::from_utf8(fixture).unwrap().replace(
        "    </NODE>\n  </PLAYLISTS>",
        "      <NODE Type=\"0\" Name=\"Other\" Count=\"0\"/>\n    </NODE>\n  </PLAYLISTS>",
    );
    std::fs::write(&source, with_sibling).unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = LibraryApp::new(
        root.join("catalog.sqlite"),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let folders = &app.xml_preview.as_ref().unwrap().as_ref().unwrap().folders;
    let index = folders
        .iter()
        .position(|f| f.name == "Sets" && f.path.len() > 1)
        .unwrap();
    app.xml_selected_folder_rename = Some(index);
    app.xml_folder_rename_name = "Renamed".into();
    app.prepare_xml_folder_rename();
    app.xml_selected_folder_rename = Some(index + 1);
    app.xml_folder_rename_export_path = root.join("selection.xml").display().to_string();
    app.export_xml_folder_rename();
    assert!(app.xml_pending_folder_rename.is_none());
    assert!(!root.join("selection.xml").exists());

    for (selection, name) in [
        (Some(0), "Renamed"),
        (Some(usize::MAX), "Renamed"),
        (Some(index), ""),
        (Some(index), "bad\0name"),
        (Some(index), "Other"),
    ] {
        app.xml_selected_folder_rename = selection;
        app.xml_folder_rename_name = name.into();
        app.prepare_xml_folder_rename();
        assert!(
            app.xml_pending_folder_rename.is_none(),
            "selection={selection:?}, name={name:?}"
        );
    }
    app.xml_selected_folder_rename = Some(index);
    app.xml_folder_rename_name = "Renamed".into();
    app.prepare_xml_folder_rename();
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"preserve me").unwrap();
    app.xml_folder_rename_export_path = existing.display().to_string();
    app.export_xml_folder_rename();
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserve me");

    app.xml_selected_folder_rename = Some(index);
    app.prepare_xml_folder_rename();
    app.xml_folder_rename_export_path = source.display().to_string();
    app.export_xml_folder_rename();
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "source alias preserved"
    );

    app.xml_folder_rename_export_path = root.join("stale.xml").display().to_string();
    app.prepare_xml_folder_rename();
    let mut changed = original.clone();
    changed.push(b' ');
    std::fs::write(&source, &changed).unwrap();
    app.export_xml_folder_rename();
    assert!(!root.join("stale.xml").exists());
    assert_eq!(std::fs::read(&source).unwrap(), changed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_rename_ui_prepares_exports_reopens_and_runs_independent_oracle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-rename-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-rename/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let index = preview
        .folders
        .iter()
        .position(|f| f.name == "Sets" && f.path.len() > 1)
        .unwrap();
    app.xml_selected_folder_rename = Some(index);
    app.xml_folder_rename_name = "Renamed".into();
    app.prepare_xml_folder_rename();
    let prepared = app
        .xml_pending_folder_rename
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.xml_pending_folder_rename = None;
    assert_eq!(
        std::fs::read(&source).unwrap(),
        original,
        "preview/cancel are write-free"
    );
    app.prepare_xml_folder_rename();
    app.xml_folder_rename_name.push('x');
    app.export_xml_folder_rename();
    assert!(
        app.xml_pending_folder_rename.is_none(),
        "name changes invalidate proposal"
    );
    app.xml_folder_rename_name = "Renamed".into();
    app.prepare_xml_folder_rename();
    let output = root.join("renamed.xml");
    app.xml_folder_rename_export_path = output.display().to_string();
    app.export_xml_folder_rename();
    let exported_bytes = std::fs::read(&output).unwrap();
    assert_eq!(exported_bytes, prepared);
    f02_folder_rename_oracle::check_ui_export_mutation_rejected(&exported_bytes, "Renamed");
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert!(reopened.folders.iter().any(|f| f.name == "Renamed"));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(catalog).unwrap(), catalog_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn empty_folder_create_ui_exports_prepared_bytes_and_reopens_folder() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-folder-create-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-empty-folder-create/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let catalog = root.join("catalog.sqlite");
    drop(Catalog::open(&catalog).unwrap());
    let catalog_before = std::fs::read(&catalog).unwrap();
    let mut app = LibraryApp::new(
        catalog.clone(),
        ThemeMode::TokyoNight,
        ThemeMode::TokyoNight.tokens(),
    );
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
    let parent_index = preview
        .folders
        .iter()
        .position(|f| f.name == "Sets" && f.path.len() > 1)
        .unwrap();
    app.xml_selected_parent = Some(parent_index);
    app.xml_folder_create_name = "Practice".into();
    app.prepare_xml_folder_create();
    assert!(
        app.xml_pending_folder_create.is_some(),
        "preparation: {:?}",
        app.xml_edit_error
    );
    let prepared = app
        .xml_pending_folder_create
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.cancel_xml_folder_create();
    assert!(app.xml_pending_folder_create.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), original);

    app.xml_selected_parent = Some(parent_index);
    app.prepare_xml_folder_create();
    app.xml_folder_create_name.push('x');
    app.export_xml_folder_create();
    assert!(
        app.xml_pending_folder_create.is_none(),
        "name change invalidates proposal"
    );
    app.xml_folder_create_name = "Practice".into();
    app.prepare_xml_folder_create();
    let output = root.join("created.xml");
    app.xml_folder_create_export_path = output.display().to_string();
    app.export_xml_folder_create();
    assert_eq!(std::fs::read(&output).unwrap(), prepared);
    f02_empty_folder_create_oracle::check(&prepared)
        .expect("actual UI bytes satisfy independent F-02 oracle");
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    let folder = reopened
        .folders
        .iter()
        .find(|f| f.name == "Practice")
        .unwrap();
    assert_eq!(folder.path, vec!["ROOT", "Sets", "Practice"]);
    assert_eq!(folder.count.as_deref(), Some("0"));
    let parent = reopened
        .folders
        .iter()
        .find(|f| f.path == vec!["ROOT", "Sets"])
        .unwrap();
    assert_eq!(parent.count.as_deref(), Some("3"));
    assert_eq!(std::fs::read(&source).unwrap(), original);

    let mut mutated = String::from_utf8(prepared.clone()).unwrap();
    mutated = mutated.replacen("Name=\"Practice\"", "Name=\"Other\"", 1);
    assert!(f02_empty_folder_create_oracle::check(mutated.as_bytes()).is_err());
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_folder_create_name = "Another".into();
    app.xml_selected_parent = Some(parent_index);
    app.prepare_xml_folder_create();
    app.xml_folder_create_export_path = existing.display().to_string();
    app.export_xml_folder_create();
    assert_eq!(std::fs::read(existing).unwrap(), b"keep");
    assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn empty_playlist_create_ui_exports_prepared_bytes_and_reopens_selectable_playlist() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-playlist-create-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.xml");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-empty-playlist-create/source.xml"
        ),
        &source,
    )
    .unwrap();
    let original = std::fs::read(&source).unwrap();
    let mut app = app();
    app.xml_path = source.display().to_string();
    app.preview_xml();
    let parent_index = app
        .xml_preview
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .folders
        .iter()
        .position(|folder| folder.name == "Sets" && folder.path.len() > 1)
        .unwrap();
    app.xml_selected_parent = Some(parent_index);
    app.xml_create_name = "Practice".into();
    app.prepare_xml_playlist_create();
    assert!(
        app.xml_pending_create.is_some(),
        "preparation failed: {:?}",
        app.xml_edit_error
    );
    let prepared = app
        .xml_pending_create
        .as_ref()
        .unwrap()
        .prepared
        .bytes()
        .to_vec();
    app.cancel_xml_playlist_create();
    assert!(
        std::fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .all(|e| e.file_name() != "cancelled.xml")
    );
    app.xml_selected_parent = Some(parent_index);
    app.prepare_xml_playlist_create();
    let output = root.join("created.xml");
    app.xml_create_export_path = output.display().to_string();
    app.export_xml_playlist_create();
    let actual = std::fs::read(&output).unwrap();
    assert_eq!(actual, prepared);
    f02_empty_playlist_create_oracle::check(&actual)
        .expect("UI bytes satisfy independent F-02 oracle");
    let mutated = String::from_utf8(actual.clone()).unwrap().replacen(
        "Name=\"Practice\"",
        "Name=\"Changed\"",
        1,
    );
    assert!(
        f02_empty_playlist_create_oracle::check(mutated.as_bytes()).is_err(),
        "oracle rejects mutation of actual UI output"
    );
    let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
    assert!(
        reopened
            .playlists
            .iter()
            .any(|p| p.name == "Practice" && p.tracks.is_empty())
    );
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let existing = root.join("existing.xml");
    std::fs::write(&existing, b"keep").unwrap();
    app.xml_selected_parent = Some(parent_index);
    app.xml_create_name = "Another".into();
    app.prepare_xml_playlist_create();
    app.xml_create_export_path = existing.display().to_string();
    app.export_xml_playlist_create();
    assert_eq!(std::fs::read(existing).unwrap(), b"keep");
    std::fs::remove_dir_all(root).unwrap();
}
