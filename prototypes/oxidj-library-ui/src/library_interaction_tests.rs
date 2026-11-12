//! Actual production multi-frame browsing, drop, and keyboard interactions.
use super::*;

fn key_event(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Default::default() }
}
fn key(shell: &mut Shell, key: egui::Key) {
    shell.frame(vec![key_event(key, true)], Default::default());
    shell.app.decks.render(512);
    shell.frame(vec![key_event(key, false)], Default::default());
    shell.app.decks.render(512);
    shell.settle();
}
fn text_rect(shell: &mut Shell, text: &str) -> Rect {
    texts(&shell.settle()).into_iter().find(|(s, r)| s == text && (text.starts_with("♫") || text == "All tracks" || shell.table().contains(r.center()))).unwrap_or_else(|| panic!("missing {text}")).1
}
fn start_drag(shell: &mut Shell, from: Pos2) {
    shell.frame(vec![egui::Event::PointerMoved(from)], Default::default());
    shell.frame(vec![egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }], Default::default());
    shell.frame(vec![egui::Event::PointerMoved(from + egui::vec2(15.0, 0.0))], Default::default());
    assert!(shell.app.dragging_track.is_some(), "production row starts a drag");
}
fn release(shell: &mut Shell, at: Pos2) {
    shell.frame(vec![egui::Event::PointerMoved(at)], Default::default());
    shell.frame(vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], Default::default());
    shell.settle();
    assert!(shell.app.dragging_track.is_none());
}
fn wait_ready(shell: &mut Shell) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while shell.app.decks.decks.iter().any(|d| d.track.is_some() && !d.ready()) {
        assert!(std::time::Instant::now() < deadline);
        shell.app.decks.render(512);
        shell.settle();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
fn imported(db: &Path, root: &Path) {
    let (location, _) = oxidj_application::location_uri(root.join("audio/tone-b.wav").to_str().unwrap());
    Catalog::open(db).unwrap().import_library(&LibrarySnapshot {
        tracks: vec![LibraryTrack { title: "Imported title".into(), artist: Some("Synthetic artist".into()), album: Some("Synthetic album".into()), tempo_hundredths_bpm: 12850, key: Some("8A".into()), duration_seconds: 73, location, ..Default::default() }],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Browse".into(), members: vec![0, 0] }],
    }).unwrap();
}

#[test]
fn pdb_import_can_be_cancelled_without_modifying_the_active_catalog() {
    let (root, db) = catalog_root("pdb-import-cancel");
    let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let dialog = shell.click(find(&nodes(&menu), "Import Rekordbox PDB…").2.center());
    shell.click(find(&nodes(&dialog), "Cancel").2.center());
    assert_eq!(shell.app.db_path, db);
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_root_playlist_can_be_renamed_after_confirmation() {
    let (root, db) = catalog_root("rename-root");
    Catalog::open(&db).unwrap().import_library(&LibrarySnapshot {
        tracks: vec![LibraryTrack { title: "Imported title".into(), artist: Some("Synthetic artist".into()), album: Some("Synthetic album".into()), tempo_hundredths_bpm: 12850, key: Some("8A".into()), duration_seconds: 73, location: "file://localhost/Contents/a.mp3".into(), ..Default::default() }, LibraryTrack { title: "Second".into(), location: "file://localhost/Contents/b.mp3".into(), ..Default::default() }],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Browse".into(), members: vec![0,0] }, oxidj_library::LibraryNode::Folder { name: "Folder".into(), children: vec![oxidj_library::LibraryNode::Playlist { name: "Nested".into(), members: vec![1,0] }] }],
    }).unwrap();
    let mut expected_snapshot = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    if let oxidj_library::LibraryNode::Playlist { name, .. } = &mut expected_snapshot.roots[0] { *name = "Renamed".into(); } else { panic!("fixture root"); }
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let out = shell.settle();
    let rename = find(&nodes(&out), "Rename playlist…").2.center();
    let out = shell.click(rename);
    assert!(nodes(&out).iter().any(|node| node.1.as_deref() == Some("Confirm rename")));
    shell.app.catalog_rename_name = "Renamed".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(shell.app.notice.starts_with("Playlist renamed"), "{}", shell.app.notice);
    assert_eq!(shell.app.imported_playlist, Some(vec![0]));
    assert!(texts(&shell.settle()).iter().any(|(s, _)| s == "♫ Renamed"));
    let snapshot = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    assert_eq!(snapshot, expected_snapshot);
    assert!(matches!(&snapshot.roots[0], oxidj_library::LibraryNode::Playlist { name, members } if name == "Renamed" && members == &[0,0]));
    let output = root.join("renamed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(shell.app.notice.starts_with("Catalog exported"), "{}", shell.app.notice);
    let oracle = r#"import sys, xml.etree.ElementTree as E
expected = E.fromstring('''<DJ_PLAYLISTS Version="1.0.0"><PRODUCT Name="OxiDJ" Version="0.1.0" Company="OxiDJ"/><COLLECTION Entries="2"><TRACK TrackID="1" Name="Imported title" Artist="Synthetic artist" Album="Synthetic album" TotalTime="73" AverageBpm="128.50" Tonality="8A" Location="PLACEHOLDER"/><TRACK TrackID="2" Name="Second" TotalTime="0" AverageBpm="0.00" Location="file://localhost/Contents/b.mp3"/></COLLECTION><PLAYLISTS><NODE Type="0" Name="ROOT" Count="2"><NODE Type="1" Name="Renamed" KeyType="0" Entries="2"><TRACK Key="1"/><TRACK Key="1"/></NODE><NODE Type="0" Name="Folder" Count="1"><NODE Type="1" Name="Nested" KeyType="0" Entries="2"><TRACK Key="2"/><TRACK Key="1"/></NODE></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>''')
expected.find('./COLLECTION/TRACK').set('Location', 'file://localhost/Contents/a.mp3')
def tree(e): return e.tag, dict(e.attrib), [tree(c) for c in e]
actual = E.parse(sys.argv[1]).getroot()
assert tree(actual) == tree(expected), (E.tostring(actual), E.tostring(expected))
actual.find('./PLAYLISTS/NODE/NODE').remove(actual.find('./PLAYLISTS/NODE/NODE/TRACK'))
assert tree(actual) != tree(expected)
"#;
    let check = std::process::Command::new("python3").args(["-c", oracle]).arg(&output).output().unwrap();
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rename_cancel_and_same_name_replacement_fail_closed() {
    let (root, db) = catalog_root("rename-stale");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse"); shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Changed".into();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap().roots[0], oxidj_library::LibraryNode::Playlist { name: "Browse".into(), members: vec![0,0] });
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Changed".into();
    let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.executescript(\"DELETE FROM library_members WHERE node_id=1; DELETE FROM library_nodes WHERE id=1; INSERT INTO library_nodes VALUES(1,NULL,0,'Browse',0); INSERT INTO library_members VALUES(1,0,0); INSERT INTO library_members VALUES(1,1,0);\"); c.commit()";
    let replacement = std::process::Command::new("python3").args(["-c", script]).arg(&db).output().unwrap();
    assert!(replacement.status.success(), "{}", String::from_utf8_lossy(&replacement.stderr));
    let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let out = shell.settle(); shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(shell.app.notice.contains("selected root playlist is stale"), "{}", shell.app.notice);
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
    drop(shell); std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_playlist_is_not_a_renameable_root_and_collision_refuses() {
    let (root, db) = catalog_root("rename-nested");
    Catalog::open(&db).unwrap().import_library(&LibrarySnapshot {
        tracks: vec![LibraryTrack { title: "Synthetic".into(), location: "file://localhost/test.mp3".into(), ..Default::default() }],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Root".into(), members: vec![0] }, oxidj_library::LibraryNode::Folder { name: "Folder".into(), children: vec![oxidj_library::LibraryNode::Playlist { name: "Nested".into(), members: vec![0] }] }],
    }).unwrap();
    let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let nested = text_rect(&mut shell, "♫ Nested"); shell.click(nested.center());
    assert_eq!(shell.app.imported_playlist, Some(vec![1,0]));
    assert!(shell.app.imported_root_guard.is_none());
    assert!(!nodes(&shell.settle()).iter().any(|n| n.1.as_deref() == Some("Rename playlist…")));
    let row = text_rect(&mut shell, "♫ Root"); shell.click(row.center());
    let out = shell.settle(); shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Folder".into();
    let out = shell.settle(); shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(shell.app.notice.contains("a root node with this name already exists"));
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
    drop(shell); std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn remove_selected_duplicate_requires_confirmation_and_preserves_catalog() {
    let (root, db) = catalog_root("remove-member");
    let (location, _) = oxidj_application::location_uri(root.join("audio/tone-b.wav").to_str().unwrap());
    let fixture = LibrarySnapshot {
        tracks: vec![LibraryTrack { title: "Imported title".into(), artist: Some("Synthetic artist".into()), album: Some("Synthetic album".into()), tempo_hundredths_bpm: 12850, key: Some("8A".into()), duration_seconds: 73, location, ..Default::default() }, LibraryTrack { title: "Unrelated".into(), location: "file://localhost/unrelated.mp3".into(), ..Default::default() }],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Browse".into(), members: vec![0, 0] }, oxidj_library::LibraryNode::Playlist { name: "Other".into(), members: vec![1, 0] }],
    };
    Catalog::open(&db).unwrap().import_library(&fixture).unwrap();
    let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse"); shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Imported title"); shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Remove selected from playlist…").2.center());
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Remove selected from playlist…").2.center());
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[0] else { panic!("fixture") };
    members.remove(1);
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), expected);
    assert!(shell.app.notice.starts_with("Playlist member removed"), "{}", shell.app.notice);
    assert!(texts(&shell.settle()).iter().any(|(s, _)| s == "1 · Imported title"));
    let output = root.join("removed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(shell.app.notice.starts_with("Catalog exported"), "{}", shell.app.notice);
    // The same independent ElementTree whole-tree oracle used by the rename test.
    let oracle = r#"import sys, xml.etree.ElementTree as E
actual=E.parse(sys.argv[1]).getroot()
expected=E.fromstring('''<DJ_PLAYLISTS Version="1.0.0"><PRODUCT Name="OxiDJ" Version="0.1.0" Company="OxiDJ"/><COLLECTION Entries="2"><TRACK TrackID="1" Name="Imported title" Artist="Synthetic artist" Album="Synthetic album" TotalTime="73" AverageBpm="128.50" Tonality="8A" Location="PLACEHOLDER"/><TRACK TrackID="2" Name="Unrelated" TotalTime="0" AverageBpm="0.00" Location="file://localhost/unrelated.mp3"/></COLLECTION><PLAYLISTS><NODE Type="0" Name="ROOT" Count="2"><NODE Type="1" Name="Browse" KeyType="0" Entries="1"><TRACK Key="1"/></NODE><NODE Type="1" Name="Other" KeyType="0" Entries="2"><TRACK Key="2"/><TRACK Key="1"/></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>''')
expected.find('./COLLECTION/TRACK').set('Location',sys.argv[2])
def tree(e): return e.tag,dict(e.attrib),[tree(c) for c in e]
assert tree(actual)==tree(expected), (E.tostring(actual),E.tostring(expected))
actual.find('./PLAYLISTS/NODE/NODE/TRACK').set('Key','999')
assert tree(actual)!=tree(expected)
"#;
    let location = &expected.tracks[0].location;
    let check = std::process::Command::new("python3").args(["-c", oracle]).arg(&output).arg(location).output().unwrap();
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    drop(shell); std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn removal_ui_refuses_stale_order_and_same_name_replacement() {
    for replacement in [false, true] {
        let (root, db) = catalog_root(if replacement { "remove-replacement" } else { "remove-stale" });
        imported(&db, &root);
        let mut shell = Shell::new(&db, 1440.0);
        let row = text_rect(&mut shell, "♫ Browse"); shell.click(row.center());
        let row = text_rect(&mut shell, "2 · Imported title"); shell.click(row.center());
        let out = shell.settle(); shell.click(find(&nodes(&out), "Remove selected from playlist…").2.center());
        let sql = if replacement {
            "DELETE FROM library_members WHERE node_id=1; DELETE FROM library_nodes WHERE id=1; INSERT INTO library_nodes VALUES(1,NULL,0,'Browse',0); INSERT INTO library_members VALUES(1,0,0); INSERT INTO library_members VALUES(1,1,0);"
        } else {
            "DELETE FROM library_members WHERE node_id=1 AND position=0;"
        };
        let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.commit()";
        let result = std::process::Command::new("python3").args(["-c", script]).arg(&db).arg(sql).output().unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
        let out = shell.settle(); shell.click(find(&nodes(&out), "Confirm removal").2.center());
        assert!(shell.app.notice.contains(if replacement { "selected root playlist is stale" } else { "playlist membership order is stale" }), "{}", shell.app.notice);
        assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
        drop(shell); std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn playlist_has_full_columns_selection_details_search_and_return_to_collection() {
    let (root, db) = catalog_root("full-playlist");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let playlist = text_rect(&mut shell, "♫ Browse");
    shell.click(playlist.center());
    let out = shell.settle();
    for name in ["Title", "Artist", "Album", "BPM", "Key", "Duration", "1 · Imported title", "2 · Imported title", "Synthetic artist", "Synthetic album", "128.50", "8A", "1:13"] {
        let matches: Vec<_> = texts(&out).into_iter().filter(|(s, _)| s == name).collect();
        assert!(!matches.is_empty(), "column/value {name}");
        assert!(matches.iter().all(|(_, r)| shell.table().contains(r.center())), "only in main table before selection: {name}");
    }
    let row = text_rect(&mut shell, "2 · Imported title");
    shell.click(row.center());
    assert_eq!(shell.app.imported_member, Some(1));
    assert_eq!(shell.app.selected_imported, Some(0));
    let out = shell.settle();
    assert!(texts(&out).iter().any(|(s, r)| s == "Imported title" && shell.app.shell.details.unwrap().contains(r.center())));
    shell.app.search = "absent".into();
    assert!(!texts(&shell.settle()).iter().any(|(s, _)| s.contains("· Imported title")));
    shell.app.search.clear();
    let all = text_rect(&mut shell, "All tracks");
    shell.click(all.center());
    assert!(shell.app.imported_playlist.is_none());
    assert!(shell.app.selected_imported.is_none());
    assert!(texts(&shell.settle()).iter().any(|(s, _)| s == "tone-a.wav"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn collection_and_playlist_rows_drop_on_each_deck_without_autoplay() {
    for playlist in [false, true] {
        let (root, db) = catalog_root(if playlist { "drop-playlist" } else { "drop-collection" });
        imported(&db, &root);
        let mut shell = Shell::new(&db, 1440.0);
        if playlist { let r = text_rect(&mut shell, "♫ Browse"); shell.click(r.center()); }
        for d in 0..2 {
            let row = text_rect(&mut shell, if playlist { "1 · Imported title" } else { "tone-b.wav" });
            start_drag(&mut shell, row.center());
            let target = if d == 0 { shell.app.decks.layout.halves.unwrap()[d] } else { shell.app.decks.layout.overviews.unwrap()[d] };
            release(&mut shell, target.center());
            assert_eq!(shell.app.decks.decks[d].track.as_ref().unwrap().path, root.join("audio/tone-b.wav"));
            assert!(!shell.app.decks.decks[d].playing);
        }
        wait_ready(&mut shell);
        assert!(shell.app.decks.render(512).iter().all(|s| *s == 0.0));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn track_drop_cancel_escape_outside_and_hidden_decks_never_load() {
    let (root, db) = catalog_root("drop-cancel");
    let mut shell = Shell::new(&db, 1440.0);
    for mode in 0..3 {
        let row = text_rect(&mut shell, "tone-b.wav");
        start_drag(&mut shell, row.center());
        let target = shell.app.decks.layout.halves.unwrap()[0].center();
        if mode == 0 { key(&mut shell, egui::Key::Escape); }
        if mode == 1 { shell.app.show_decks = false; }
        let end = if mode == 2 { shell.app.shell.status.unwrap().center() } else { target };
        release(&mut shell, end);
        assert!(shell.app.decks.decks.iter().all(|d| d.track.is_none()));
        shell.app.show_decks = true;
        shell.settle();
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn simple_dj_keys_load_playlist_and_drive_both_decks_but_not_text_or_settings() {
    let (root, db) = catalog_root("simple-keys");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    shell.app.settings.simple_dj_keys = true;
    let playlist = text_rect(&mut shell, "♫ Browse"); shell.click(playlist.center());
    let row = text_rect(&mut shell, "1 · Imported title"); shell.click(row.center());
    // Rows are selectable, not a keyboard-editable control.
    key(&mut shell, egui::Key::A);
    key(&mut shell, egui::Key::B);
    assert!(shell.app.decks.decks.iter().all(|d| d.track.is_some()));
    wait_ready(&mut shell);
    key(&mut shell, egui::Key::X);
    key(&mut shell, egui::Key::M);
    assert!(shell.app.decks.decks.iter().all(|d| d.playing));
    key(&mut shell, egui::Key::X);
    key(&mut shell, egui::Key::M);
    assert!(shell.app.decks.decks.iter().all(|d| !d.playing));
    for (d, cue) in [(0, egui::Key::Z), (1, egui::Key::N)] {
        shell.frame(vec![key_event(cue, true)], Default::default());
        shell.app.decks.render(512); shell.settle();
        assert!(shell.app.decks.decks[d].cue_held);
        shell.frame(vec![key_event(cue, false)], Default::default());
        shell.app.decks.render(512); shell.settle();
        assert!(!shell.app.decks.decks[d].cue_held);
    }
    shell.frame(vec![key_event(egui::Key::Z, true)], Default::default());
    shell.app.decks.render(512); shell.settle();
    assert!(shell.app.decks.decks[0].cue_held);
    shell.click(shell.app.shell.search.unwrap().center());
    shell.app.decks.render(512); shell.settle();
    assert!(!shell.app.decks.decks[0].cue_held, "focus change ends cue preview");
    shell.frame(vec![key_event(egui::Key::Z, false)], Default::default());
    for k in [egui::Key::X, egui::Key::M, egui::Key::Z, egui::Key::N, egui::Key::A, egui::Key::B] { key(&mut shell, k); }
    assert!(shell.app.decks.decks.iter().all(|d| !d.playing && !d.cue_held));
    shell.shortcut(egui::Key::Comma);
    key(&mut shell, egui::Key::X);
    assert!(!shell.app.decks.decks[0].playing);
    std::fs::remove_dir_all(root).unwrap();
}


#[test]
fn unavailable_imported_drop_preserves_loaded_deck_and_plain_click_never_loads() {
    let (root, db) = catalog_root("drop-refusal");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "tone-a.wav");
    shell.click(row.center());
    assert!(shell.app.decks.decks.iter().all(|d| d.track.is_none()));
    key(&mut shell, egui::Key::A);
    let before = shell.app.decks.decks[0].track.as_ref().unwrap().path.clone();
    let playlist = text_rect(&mut shell, "♫ Browse"); shell.click(playlist.center());
    std::fs::remove_file(root.join("audio/tone-b.wav")).unwrap();
    let row = text_rect(&mut shell, "1 · Imported title");
    start_drag(&mut shell, row.center());
    let target = shell.app.decks.layout.halves.unwrap()[0].center();
    release(&mut shell, target);
    assert!(shell.app.notice.starts_with("Playlist load refused:"));
    assert_eq!(shell.app.decks.decks[0].track.as_ref().unwrap().path, before);
    assert!(shell.app.decks.decks[1].track.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
