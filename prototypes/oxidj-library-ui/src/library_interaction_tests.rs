//! Actual production multi-frame browsing, drop, and keyboard interactions.
use super::*;

fn key_event(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Default::default(),
    }
}
fn key(shell: &mut Shell, key: egui::Key) {
    shell.frame(vec![key_event(key, true)], Default::default());
    shell.app.decks.render(512);
    shell.frame(vec![key_event(key, false)], Default::default());
    shell.app.decks.render(512);
    shell.settle();
}
fn text_rect(shell: &mut Shell, text: &str) -> Rect {
    texts(&shell.settle())
        .into_iter()
        .find(|(s, r)| {
            s == text
                && (text.starts_with("♫")
                    || text == "All tracks"
                    || shell.table().contains(r.center()))
        })
        .unwrap_or_else(|| panic!("missing {text}"))
        .1
}
fn start_drag(shell: &mut Shell, from: Pos2) {
    shell.frame(vec![egui::Event::PointerMoved(from)], Default::default());
    shell.frame(
        vec![egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
        Default::default(),
    );
    shell.frame(
        vec![egui::Event::PointerMoved(from + egui::vec2(15.0, 0.0))],
        Default::default(),
    );
    assert!(
        shell.app.dragging_track.is_some(),
        "production row starts a drag"
    );
}
fn release(shell: &mut Shell, at: Pos2) {
    shell.frame(vec![egui::Event::PointerMoved(at)], Default::default());
    shell.frame(
        vec![egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
        Default::default(),
    );
    shell.settle();
    assert!(shell.app.dragging_track.is_none());
}
fn wait_ready(shell: &mut Shell) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while shell
        .app
        .decks
        .decks
        .iter()
        .any(|d| d.track.is_some() && !d.ready())
    {
        assert!(std::time::Instant::now() < deadline);
        shell.app.decks.render(512);
        shell.settle();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
fn imported(db: &Path, root: &Path) {
    let (location, _) =
        oxidj_application::location_uri(root.join("audio/tone-b.wav").to_str().unwrap());
    Catalog::open(db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            tracks: vec![LibraryTrack {
                title: "Imported title".into(),
                artist: Some("Synthetic artist".into()),
                album: Some("Synthetic album".into()),
                tempo_hundredths_bpm: 12850,
                key: Some("8A".into()),
                duration_seconds: 73,
                location,
                ..Default::default()
            }],
            roots: vec![oxidj_library::LibraryNode::Playlist {
                name: "Browse".into(),
                members: vec![0, 0],
            }],
        })
        .unwrap();
}

#[test]
fn selected_root_playlist_occurrence_can_move_down() {
    let (root, db) = catalog_root("move-root");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected up").2.center());
    assert_eq!(shell.app.imported_member, Some(0));
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected down").2.center());
    assert_eq!(shell.app.imported_member, Some(1));
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected down").2.center());
    assert_eq!(shell.app.imported_member, Some(1));
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn move_second_duplicate_preserves_other_occurrences() {
    let (root, _) = catalog_root("move-duplicate");
    let (pdb, _, _) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    Catalog::open(&db)
        .unwrap()
        .create_library_playlist("Browse", &[0, 1, 0])
        .unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "3 · Track 1");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected up").2.center());
    assert_eq!(shell.app.imported_member, Some(1));
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = expected.roots.last_mut().unwrap()
    else {
        panic!("fixture")
    };
    members.swap(1, 2);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_imported_track_can_append_to_selected_root_playlist() {
    let (root, db) = catalog_root("append-root");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Add selected to playlist…").2.center());
    shell.click(find(&nodes(&out), "Confirm addition").2.center());
    let snapshot = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    assert!(
        matches!(&snapshot.roots[0], oxidj_library::LibraryNode::Playlist { members, .. } if members == &[0, 0, 0])
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_imported_track_can_create_a_root_playlist() {
    let (root, db) = catalog_root("create-root");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = "New set".into();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap()
            .roots
            .len(),
        1
    );
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = String::new();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(shell.app.notice.contains("name"), "{}", shell.app.notice);
    shell.app.catalog_create_name = "Browse".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell.app.notice.contains("already exists"),
        "{}",
        shell.app.notice
    );
    shell.app.catalog_create_name = "New set".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist created"),
        "{}",
        shell.app.notice
    );
    let snapshot = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    assert!(
        matches!(&snapshot.roots[1], oxidj_library::LibraryNode::Playlist { name, members } if name == "New set" && members == &[0])
    );
    let row = text_rect(&mut shell, "♫ New set");
    shell.click(row.center());
    assert_eq!(shell.app.imported_playlist, Some(vec![1]));
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn creation_refuses_missing_catalog_without_recreating_it() {
    let (root, db) = catalog_root("create-missing");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = "New set".into();
    let moved = root.join("moved.sqlite");
    std::fs::rename(&db, &moved).unwrap();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell.app.notice.contains("catalog is no longer available"),
        "{}",
        shell.app.notice
    );
    assert!(!db.exists());
    drop(shell);
    std::fs::rename(moved, db).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn creation_refuses_replaced_track_even_if_metadata_is_identical() {
    let (root, db) = catalog_root("create-replacement");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = "New set".into();
    let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.executescript('CREATE TEMP TABLE saved AS SELECT * FROM library_tracks WHERE position=0; DELETE FROM library_tracks WHERE position=0; INSERT INTO library_tracks SELECT * FROM saved;'); c.commit()";
    let result = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(&db)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell
            .app
            .notice
            .contains("selected imported track is stale"),
        "{}",
        shell.app.notice
    );
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_import_can_be_cancelled_without_modifying_the_active_catalog() {
    let (root, db) = catalog_root("pdb-import-cancel");
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let dialog = shell.click(find(&nodes(&menu), "Import Rekordbox PDB…").2.center());
    shell.click(find(&nodes(&dialog), "Cancel").2.center());
    assert_eq!(shell.app.db_path, db);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selected_root_playlist_can_be_renamed_after_confirmation() {
    let (root, db) = catalog_root("rename-root");
    Catalog::open(&db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            tracks: vec![
                LibraryTrack {
                    title: "Imported title".into(),
                    artist: Some("Synthetic artist".into()),
                    album: Some("Synthetic album".into()),
                    tempo_hundredths_bpm: 12850,
                    key: Some("8A".into()),
                    duration_seconds: 73,
                    location: "file://localhost/Contents/a.mp3".into(),
                    ..Default::default()
                },
                LibraryTrack {
                    title: "Second".into(),
                    location: "file://localhost/Contents/b.mp3".into(),
                    ..Default::default()
                },
            ],
            roots: vec![
                oxidj_library::LibraryNode::Playlist {
                    name: "Browse".into(),
                    members: vec![0, 0],
                },
                oxidj_library::LibraryNode::Folder {
                    name: "Folder".into(),
                    children: vec![oxidj_library::LibraryNode::Playlist {
                        name: "Nested".into(),
                        members: vec![1, 0],
                    }],
                },
            ],
        })
        .unwrap();
    let mut expected_snapshot = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    if let oxidj_library::LibraryNode::Playlist { name, .. } = &mut expected_snapshot.roots[0] {
        *name = "Renamed".into();
    } else {
        panic!("fixture root");
    }
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let out = shell.settle();
    let rename = find(&nodes(&out), "Rename playlist…").2.center();
    let out = shell.click(rename);
    assert!(
        nodes(&out)
            .iter()
            .any(|node| node.1.as_deref() == Some("Confirm rename"))
    );
    shell.app.catalog_rename_name = "Renamed".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist renamed"),
        "{}",
        shell.app.notice
    );
    assert_eq!(shell.app.imported_playlist, Some(vec![0]));
    assert!(texts(&shell.settle()).iter().any(|(s, _)| s == "♫ Renamed"));
    let snapshot = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    assert_eq!(snapshot, expected_snapshot);
    assert!(
        matches!(&snapshot.roots[0], oxidj_library::LibraryNode::Playlist { name, members } if name == "Renamed" && members == &[0,0])
    );
    let output = root.join("renamed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = r#"import sys, xml.etree.ElementTree as E
expected = E.fromstring('''<DJ_PLAYLISTS Version="1.0.0"><PRODUCT Name="OxiDJ" Version="0.1.0" Company="OxiDJ"/><COLLECTION Entries="2"><TRACK TrackID="1" Name="Imported title" Artist="Synthetic artist" Album="Synthetic album" TotalTime="73" AverageBpm="128.50" Tonality="8A" Location="PLACEHOLDER"/><TRACK TrackID="2" Name="Second" TotalTime="0" AverageBpm="0.00" Location="file://localhost/Contents/b.mp3"/></COLLECTION><PLAYLISTS><NODE Type="0" Name="ROOT" Count="2"><NODE Type="1" Name="Renamed" KeyType="0" Entries="2"><TRACK Key="1"/><TRACK Key="1"/></NODE><NODE Type="0" Name="Folder" Count="1"><NODE Type="1" Name="Nested" KeyType="0" Entries="2"><TRACK Key="2"/><TRACK Key="1"/></NODE></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>''')
expected.find('./COLLECTION/TRACK').set('Location', 'file://localhost/Contents/a.mp3')
def tree(e): return e.tag, dict(e.attrib), [tree(c) for c in e]
actual = E.parse(sys.argv[1]).getroot()
assert tree(actual) == tree(expected), (E.tostring(actual), E.tostring(expected))
actual.find('./PLAYLISTS/NODE/NODE').remove(actual.find('./PLAYLISTS/NODE/NODE/TRACK'))
assert tree(actual) != tree(expected)
"#;
    let check = std::process::Command::new("python3")
        .args(["-c", oracle])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rename_cancel_and_same_name_replacement_fail_closed() {
    let (root, db) = catalog_root("rename-stale");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Changed".into();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap()
            .roots[0],
        oxidj_library::LibraryNode::Playlist {
            name: "Browse".into(),
            members: vec![0, 0]
        }
    );
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Changed".into();
    let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.executescript(\"DELETE FROM library_members WHERE node_id=1; DELETE FROM library_nodes WHERE id=1; INSERT INTO library_nodes VALUES(1,NULL,0,'Browse',0); INSERT INTO library_members VALUES(1,0,0); INSERT INTO library_members VALUES(1,1,0);\"); c.commit()";
    let replacement = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(&db)
        .output()
        .unwrap();
    assert!(
        replacement.status.success(),
        "{}",
        String::from_utf8_lossy(&replacement.stderr)
    );
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(
        shell.app.notice.contains("selected root playlist is stale"),
        "{}",
        shell.app.notice
    );
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_playlist_offers_rename_and_root_collision_refuses() {
    let (root, db) = catalog_root("rename-nested");
    Catalog::open(&db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            tracks: vec![LibraryTrack {
                title: "Synthetic".into(),
                location: "file://localhost/test.mp3".into(),
                ..Default::default()
            }],
            roots: vec![
                oxidj_library::LibraryNode::Playlist {
                    name: "Root".into(),
                    members: vec![0],
                },
                oxidj_library::LibraryNode::Folder {
                    name: "Folder".into(),
                    children: vec![oxidj_library::LibraryNode::Playlist {
                        name: "Nested".into(),
                        members: vec![0],
                    }],
                },
            ],
        })
        .unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let nested = text_rect(&mut shell, "♫ Nested");
    shell.click(nested.center());
    assert_eq!(shell.app.imported_playlist, Some(vec![1, 0]));
    assert!(shell.app.imported_root_guard.is_none());
    assert!(
        nodes(&shell.settle())
            .iter()
            .any(|n| n.1.as_deref() == Some("Rename playlist…"))
    );
    let row = text_rect(&mut shell, "♫ Root");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Folder".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(
        shell
            .app
            .notice
            .contains("a root node with this name already exists")
    );
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_remove_selected_occurrence_is_available() {
    let (root, db) = catalog_root("nested-remove-available");
    let fixture = LibrarySnapshot {
        tracks: vec![
            LibraryTrack {
                title: "Nested track".into(),
                location: "file://localhost/nested.mp3".into(),
                ..Default::default()
            },
            LibraryTrack {
                title: "Other track".into(),
                location: "file://localhost/other.mp3".into(),
                ..Default::default()
            },
        ],
        roots: ["First", "Second"]
            .into_iter()
            .map(|name| oxidj_library::LibraryNode::Folder {
                name: name.into(),
                children: vec![oxidj_library::LibraryNode::Playlist {
                    name: "Nested".into(),
                    members: vec![0, 0, 1, 0],
                }],
            })
            .collect(),
    };
    Catalog::open(&db)
        .unwrap()
        .import_library(&fixture)
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let rows: Vec<_> = texts(&shell.settle())
        .into_iter()
        .filter(|(s, _)| s == "♫ Nested")
        .collect();
    assert_eq!(rows.len(), 2);
    shell.click(rows[1].1.center());
    let row = text_rect(&mut shell, "2 · Nested track");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        fixture
    );
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist member removed"),
        "{}",
        shell.app.notice
    );
    let mut expected = fixture;
    let oxidj_library::LibraryNode::Folder { children, .. } = &mut expected.roots[1] else {
        panic!("folder")
    };
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut children[0] else {
        panic!("playlist")
    };
    members.remove(1);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    // The playlist remains selected: remove a second occurrence without reselecting it.
    let row = text_rect(&mut shell, "3 · Nested track");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    let oxidj_library::LibraryNode::Folder { children, .. } = &mut expected.roots[1] else {
        panic!("folder")
    };
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut children[0] else {
        panic!("playlist")
    };
    members.remove(2);
    assert_eq!(members, &[0, 1]);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn remove_selected_duplicate_requires_confirmation_and_preserves_catalog() {
    let (root, db) = catalog_root("remove-member");
    let (location, _) =
        oxidj_application::location_uri(root.join("audio/tone-b.wav").to_str().unwrap());
    let fixture = LibrarySnapshot {
        tracks: vec![
            LibraryTrack {
                title: "Imported title".into(),
                artist: Some("Synthetic artist".into()),
                album: Some("Synthetic album".into()),
                tempo_hundredths_bpm: 12850,
                key: Some("8A".into()),
                duration_seconds: 73,
                location,
                ..Default::default()
            },
            LibraryTrack {
                title: "Unrelated".into(),
                location: "file://localhost/unrelated.mp3".into(),
                ..Default::default()
            },
        ],
        roots: vec![
            oxidj_library::LibraryNode::Playlist {
                name: "Browse".into(),
                members: vec![0, 0],
            },
            oxidj_library::LibraryNode::Playlist {
                name: "Other".into(),
                members: vec![1, 0],
            },
        ],
    };
    Catalog::open(&db)
        .unwrap()
        .import_library(&fixture)
        .unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let row = text_rect(&mut shell, "♫ Browse");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Imported title");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[0] else {
        panic!("fixture")
    };
    members.remove(1);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    assert!(
        shell.app.notice.starts_with("Playlist member removed"),
        "{}",
        shell.app.notice
    );
    assert!(
        texts(&shell.settle())
            .iter()
            .any(|(s, _)| s == "1 · Imported title")
    );
    let output = root.join("removed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

// Share the existing CLI proof's synthetic DeviceSQL/ANLZ generator verbatim.
#[allow(dead_code)]
mod m7_fixture {
    use std::path::{Path, PathBuf};
    const ANLZ_A: &str = "/PIONEER/USBANLZ/P001/0000002A/ANLZ0000.DAT";
    const ANLZ_B: &str = "/PIONEER/USBANLZ/P001/00000007/ANLZ0000.DAT";
    include!("../../../crates/oxidj-cli/tests/support/library_pdb_fixture.rs");
    include!("../../../crates/oxidj-cli/tests/support/library_anlz_fixture.rs");
    include!("../../../crates/oxidj-cli/tests/support/library_m7_fixture.rs");

    pub fn write(device: &Path) -> (PathBuf, Vec<u8>, Vec<(PathBuf, Vec<u8>)>) {
        let pdb = device.join("PIONEER/rekordbox/export.pdb");
        std::fs::create_dir_all(pdb.parent().unwrap()).unwrap();
        let source = synthetic_pdb_m7();
        std::fs::write(&pdb, &source).unwrap();
        (pdb, source, write_analysis_m7(device))
    }
}

#[test]
fn pdb_move_refuses_stale_playlist_track_order_and_missing_catalog() {
    for case in ["track", "playlist", "order", "missing"] {
        let (root, _) = catalog_root(&format!("move-stale-{case}"));
        let (pdb, _, _) = m7_fixture::write(&root.join("device"));
        let db = root.join("imported.sqlite");
        oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
        let mut shell = Shell::new(&db, 1440.0);
        shell.screen.max.y = 1800.0;
        let row = text_rect(&mut shell, "♫ Playlist 1");
        shell.click(row.center());
        let row = text_rect(&mut shell, "2 · Track 2");
        shell.click(row.center());
        let moved = root.join("moved.sqlite");
        if case == "missing" {
            std::fs::rename(&db, &moved).unwrap();
        } else {
            let sql = match case {
                "track" => {
                    "CREATE TEMP TABLE saved AS SELECT * FROM library_tracks WHERE position=1; DELETE FROM library_tracks WHERE position=1; INSERT INTO library_tracks SELECT * FROM saved;"
                }
                "playlist" => {
                    "CREATE TEMP TABLE saved AS SELECT * FROM library_nodes WHERE id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1); DELETE FROM library_nodes WHERE id=(SELECT id FROM saved); INSERT INTO library_nodes SELECT * FROM saved;"
                }
                _ => {
                    "CREATE TEMP TABLE saved AS SELECT position,track_position FROM library_members WHERE node_id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1) AND position IN (0,1); UPDATE library_members SET track_position=(SELECT track_position FROM saved WHERE saved.position=1-library_members.position) WHERE node_id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1) AND position IN (0,1);"
                }
            };
            let result = std::process::Command::new("python3").args(["-c", "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.commit()"]).arg(&db).arg(sql).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        let path = if case == "missing" { &moved } else { &db };
        let before = Catalog::open_read_only(path)
            .unwrap()
            .library_snapshot()
            .unwrap();
        let out = shell.settle();
        shell.click(find(&nodes(&out), "Move selected up").2.center());
        let reason = match case {
            "track" => "selected imported track is stale",
            "playlist" => "selected root playlist is stale",
            "order" => "playlist membership order is stale",
            _ => "catalog is no longer available",
        };
        assert!(
            shell.app.notice.contains(reason),
            "{case}: {}",
            shell.app.notice
        );
        assert_eq!(
            Catalog::open_read_only(path)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            before
        );
        if case == "missing" {
            assert!(!db.exists());
        }
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn nested_remove_refuses_stale_identity_and_membership() {
    for case in ["parent", "playlist", "moved", "membership", "track"] {
        let (root, _) = catalog_root("nested-remove-stale");
        let (pdb, _, _) = m7_fixture::write(&root.join("device"));
        let db = root.join("imported.sqlite");
        oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
        let mut shell = Shell::new(&db, 1440.0);
        shell.screen.max.y = 1800.0;
        let row = text_rect(&mut shell, "♫ Playlist 19");
        shell.click(row.center());
        let row = text_rect(&mut shell, "2 · Track 812");
        shell.click(row.center());
        let out = shell.settle();
        let out = shell.click(
            find(&nodes(&out), "Remove selected from playlist…")
                .2
                .center(),
        );
        let script = r#"import sqlite3,sys
c=sqlite3.connect(sys.argv[1]); case=sys.argv[2]
p=c.execute("select id from library_nodes where name='Folder A'").fetchone()[0]
n=c.execute("select id from library_nodes where parent_id=? and name='Playlist 19'",(p,)).fetchone()[0]
if case in ('parent','playlist'):
 c.execute('pragma defer_foreign_keys=on'); i=p if case=='parent' else n
 row=c.execute('select * from library_nodes where id=?',(i,)).fetchone()
 c.execute('delete from library_nodes where id=?',(i,)); c.execute('insert into library_nodes values(?,?,?,?,?)',row)
elif case=='moved': c.execute("update library_nodes set parent_id=(select id from library_nodes where name='Folder B'),sibling_order=1 where id=?",(n,))
elif case=='membership': c.execute('update library_members set track_position=0 where node_id=? and position=1',(n,))
else: c.execute('update library_track_identity set token=randomblob(16) where track_position=811')
c.commit()"#;
        let result = std::process::Command::new("python3")
            .args(["-c", script])
            .arg(&db)
            .arg(case)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let before = Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap();
        shell.click(find(&nodes(&out), "Confirm removal").2.center());
        let reason = if case == "track" {
            "selected imported track is stale"
        } else {
            "selected nested playlist is stale"
        };
        assert!(
            shell.app.notice.contains(reason),
            "{case}: {}",
            shell.app.notice
        );
        assert_eq!(
            Catalog::open_read_only(&db)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            before
        );
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn pdb_nested_remove_ui_export_matches_oracle() {
    let (root, _) = catalog_root("nested-remove-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Playlist 19");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 812");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist member removed"),
        "{}",
        shell.app.notice
    );
    let output = root.join("nested-removed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            .args(["--remove-member", "118", "1", "812"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let wrong = root.join("wrong.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); n=next(x for x in t.iter('NODE') if x.get('Name')=='Playlist 19'); n.remove(n.find('TRACK')); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_nested_rename_ui_export_matches_independent_oracle() {
    let (root, _) = catalog_root("rename-pdb-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Playlist 19");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Renamed nested".into();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Renamed nested".into();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist renamed"),
        "{}",
        shell.app.notice
    );
    let output = root.join("renamed.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            .args(["--rename-playlist", "118", "Playlist 19", "Renamed nested"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let wrong = root.join("wrong.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); n=next(x for x in t.iter('NODE') if x.get('Name')=='Renamed nested'); n.set('Name','Wrong'); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_rename_selects_same_named_playlist_by_parent() {
    use oxidj_library::LibraryNode;
    let (root, db) = catalog_root("rename-same-name");
    let mut expected = LibrarySnapshot {
        tracks: vec![LibraryTrack {
            title: "Synthetic".into(),
            location: "file://localhost/test.wav".into(),
            ..Default::default()
        }],
        roots: vec!["First", "Second"]
            .into_iter()
            .map(|name| LibraryNode::Folder {
                name: name.into(),
                children: vec![LibraryNode::Playlist {
                    name: "Same".into(),
                    members: vec![0, 0],
                }],
            })
            .collect(),
    };
    Catalog::open(&db)
        .unwrap()
        .import_library(&expected)
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let rows: Vec<_> = texts(&shell.settle())
        .into_iter()
        .filter(|(label, _)| label == "♫ Same")
        .collect();
    assert_eq!(rows.len(), 2);
    shell.click(rows[1].1.center());
    assert_eq!(shell.app.imported_playlist, Some(vec![1, 0]));
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
    shell.app.catalog_rename_name = "Changed".into();
    shell.click(find(&nodes(&out), "Confirm rename").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist renamed"),
        "{}",
        shell.app.notice
    );
    if let LibraryNode::Folder { children, .. } = &mut expected.roots[1] {
        if let LibraryNode::Playlist { name, .. } = &mut children[0] {
            *name = "Changed".into();
        }
    }
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_rename_ui_refuses_replacement_membership_and_collision() {
    for case in ["parent", "playlist", "membership", "collision"] {
        let (root, _) = catalog_root("rename-ui-stale");
        let (pdb, _, _) = m7_fixture::write(&root.join("device"));
        let db = root.join("imported.sqlite");
        oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
        let mut shell = Shell::new(&db, 1440.0);
        shell.screen.max.y = 1800.0;
        let row = text_rect(&mut shell, "♫ Playlist 19");
        shell.click(row.center());
        let out = shell.settle();
        let out = shell.click(find(&nodes(&out), "Rename playlist…").2.center());
        shell.app.catalog_rename_name = "Clash".into();
        let script = r#"import sqlite3,sys
c=sqlite3.connect(sys.argv[1]); case=sys.argv[2]
p=c.execute("select id from library_nodes where name='Folder A'").fetchone()[0]
n=c.execute("select id from library_nodes where parent_id=? and name='Playlist 19'",(p,)).fetchone()[0]
if case in ('parent','playlist'):
 c.execute('pragma defer_foreign_keys=on'); i=p if case=='parent' else n
 row=c.execute('select * from library_nodes where id=?',(i,)).fetchone()
 c.execute('delete from library_nodes where id=?',(i,)); c.execute('insert into library_nodes values(?,?,?,?,?)',row)
elif case=='membership': c.execute('update library_members set track_position=0 where node_id=? and position=1',(n,))
else: c.execute("insert into library_nodes(parent_id,sibling_order,name,is_folder) values (?,1,'Clash',0)",(p,))
c.commit()"#;
        let mutation = std::process::Command::new("python3")
            .args(["-c", script])
            .arg(&db)
            .arg(case)
            .output()
            .unwrap();
        assert!(
            mutation.status.success(),
            "{}",
            String::from_utf8_lossy(&mutation.stderr)
        );
        let before = Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap();
        shell.click(find(&nodes(&out), "Confirm rename").2.center());
        let reason = if case == "collision" {
            "a sibling node with this name already exists"
        } else {
            "selected nested playlist is stale"
        };
        assert!(
            shell.app.notice.contains(reason),
            "{}: {}",
            case,
            shell.app.notice
        );
        assert_eq!(
            Catalog::open_read_only(&db)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            before
        );
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn pdb_move_ui_exports_exact_order_against_independent_oracle() {
    let (root, _) = catalog_root("move-pdb-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Playlist 1");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 2");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected up").2.center());
    assert_eq!(shell.app.imported_member, Some(0));
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected down").2.center());
    assert_eq!(shell.app.imported_member, Some(1));
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Move selected up").2.center());
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[0] else {
        panic!("fixture")
    };
    members.swap(0, 1);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    let output = root.join("moved.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            .args(["--move-member", "100", "1", "0", "2"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let wrong = root.join("wrong.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); n=t.find('./PLAYLISTS/NODE/NODE'); a=n[0].get('Key'); n[0].set('Key',n[1].get('Key')); n[1].set('Key',a); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_append_allows_selecting_a_different_root_target() {
    let (root, _) = catalog_root("append-other-target");
    let (pdb, _, _) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Playlist 1");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 2");
    shell.click(row.center());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Add selected to playlist…").2.center());
    shell.app.catalog_append_pending.as_mut().unwrap().2 = None;
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm addition").2.center());
    assert!(shell.app.notice.contains("choose a root playlist"));
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Target: Playlist 2").2.center());
    shell.click(find(&nodes(&out), "Confirm addition").2.center());
    assert!(
        shell.app.notice.starts_with("Track added"),
        "{}",
        shell.app.notice
    );
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[1] else {
        panic!("fixture")
    };
    members.push(1);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_append_refuses_stale_track_playlist_membership_and_missing_catalog() {
    for case in ["track", "playlist", "order", "missing"] {
        let (root, _) = catalog_root(&format!("append-stale-{case}"));
        let (pdb, _, _) = m7_fixture::write(&root.join("device"));
        let db = root.join("imported.sqlite");
        oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
        let mut shell = Shell::new(&db, 1440.0);
        shell.screen.max.y = 1800.0;
        let row = text_rect(&mut shell, "♫ Playlist 1");
        shell.click(row.center());
        let row = text_rect(&mut shell, "2 · Track 2");
        shell.click(row.center());
        let out = shell.settle();
        shell.click(find(&nodes(&out), "Add selected to playlist…").2.center());
        let moved = root.join("moved.sqlite");
        if case == "missing" {
            std::fs::rename(&db, &moved).unwrap();
        } else {
            let sql = match case {
                "track" => {
                    "CREATE TEMP TABLE saved AS SELECT * FROM library_tracks WHERE position=1; DELETE FROM library_tracks WHERE position=1; INSERT INTO library_tracks SELECT * FROM saved;"
                }
                "playlist" => {
                    "CREATE TEMP TABLE saved AS SELECT * FROM library_nodes WHERE id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1); DELETE FROM library_nodes WHERE id=(SELECT id FROM saved); INSERT INTO library_nodes SELECT * FROM saved;"
                }
                _ => {
                    "CREATE TEMP TABLE saved AS SELECT position,track_position FROM library_members WHERE node_id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1) AND position IN (0,1); UPDATE library_members SET track_position=(SELECT track_position FROM saved WHERE saved.position=1-library_members.position) WHERE node_id=(SELECT id FROM library_nodes WHERE parent_id IS NULL ORDER BY sibling_order,id LIMIT 1) AND position IN (0,1);"
                }
            };
            let result = std::process::Command::new("python3").args(["-c", "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.commit()"])
                .arg(&db).arg(sql).output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        let path = if case == "missing" { &moved } else { &db };
        let before = Catalog::open_read_only(path)
            .unwrap()
            .library_snapshot()
            .unwrap();
        let out = shell.settle();
        shell.click(find(&nodes(&out), "Confirm addition").2.center());
        let reason = match case {
            "track" => "selected imported track is stale",
            "playlist" => "selected root playlist is stale",
            "order" => "playlist membership order is stale",
            _ => "catalog is no longer available",
        };
        assert!(
            shell.app.notice.contains(reason),
            "{case}: {}",
            shell.app.notice
        );
        assert_eq!(
            Catalog::open_read_only(path)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            before
        );
        if case == "missing" {
            assert!(!db.exists());
        }
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn pdb_append_ui_export_passes_independent_oracle_and_rejects_mutation() {
    let (root, _) = catalog_root("append-pdb-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    let row = text_rect(&mut shell, "♫ Playlist 1");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 2");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Add selected to playlist…").2.center());
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(find(&nodes(&out), "Add selected to playlist…").2.center());
    shell.click(find(&nodes(&out), "Confirm addition").2.center());
    assert!(
        shell.app.notice.starts_with("Track added"),
        "{}",
        shell.app.notice
    );
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[0] else {
        panic!("fixture")
    };
    members.push(1);
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    let output = root.join("appended.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            .args(["--append-member", "100", "2"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let wrong = root.join("wrong.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); t.find('./PLAYLISTS/NODE/NODE')[-1].set('Key','1'); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_creation_ui_export_passes_existing_independent_oracle() {
    let (root, _) = catalog_root("create-pdb-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    shell.screen.max.y = 1800.0;
    shell.settle();
    let row = text_rect(&mut shell, "♫ Playlist 1");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 2");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = "Cancelled".into();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Create playlist from selected…")
            .2
            .center(),
    );
    shell.app.catalog_create_name = "Playlist 1".into();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell.app.notice.contains("already exists"),
        "{}",
        shell.app.notice
    );
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    shell.app.catalog_create_name = "Created set".into();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm creation").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist created"),
        "{}",
        shell.app.notice
    );
    let mut expected = before;
    expected.roots.push(oxidj_library::LibraryNode::Playlist {
        name: "Created set".into(),
        members: vec![1],
    });
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    let output = root.join("created.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            .args(["--create-playlist", "Created set", "2"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let wrong = root.join("wrong-member.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); t.find('./PLAYLISTS/NODE/NODE[@Name=\"Created set\"]/TRACK').set('Key','1'); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_member_removal_ui_export_passes_existing_independent_oracle() {
    let (root, _) = catalog_root("remove-pdb-oracle");
    let (pdb, source, analysis) = m7_fixture::write(&root.join("device"));
    let db = root.join("imported.sqlite");
    oxidj_application::import_library_from_pdb(&pdb, &db, None).unwrap();
    let before = Catalog::open_read_only(&db)
        .unwrap()
        .library_snapshot()
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    // Keep the full 20-playlist fixture and its action buttons visible.
    shell.screen.max.y = 1800.0;
    shell.settle();
    let row = text_rect(&mut shell, "♫ Playlist 1");
    shell.click(row.center());
    let row = text_rect(&mut shell, "2 · Track 2");
    shell.click(row.center());
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        before
    );
    let out = shell.settle();
    let out = shell.click(
        find(&nodes(&out), "Remove selected from playlist…")
            .2
            .center(),
    );
    shell.click(find(&nodes(&out), "Confirm removal").2.center());
    assert!(
        shell.app.notice.starts_with("Playlist member removed"),
        "{}",
        shell.app.notice
    );
    let mut expected = before;
    let oxidj_library::LibraryNode::Playlist { members, .. } = &mut expected.roots[0] else {
        panic!("fixture")
    };
    assert_eq!(members.remove(1), 1); // PDB TrackID 2, zero-based snapshot identity 1.
    assert_eq!(
        Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap(),
        expected
    );
    assert!(
        texts(&shell.settle())
            .iter()
            .any(|(s, _)| s == "2 · Track 3")
    );

    let output = root.join("removed-pdb.xml");
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    shell.app.catalog_export_path = output.display().to_string();
    shell.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(
        shell.app.notice.starts_with("Catalog exported"),
        "{}",
        shell.app.notice
    );
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_m7_export.py");
    let check = |xml: &Path| {
        std::process::Command::new("python3")
            .arg("-B")
            .arg(&oracle)
            .arg(&pdb)
            .arg(xml)
            // Independent expectation: PDB playlist 100, index 1, TrackID 2 only.
            .args(["--remove-member", "100", "1", "2"])
            .output()
            .unwrap()
    };
    let result = check(&output);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    // Mutate those exact UI bytes to emulate deleting the wrong occurrence,
    // retaining the membership count and every collection/cue/grid field.
    let wrong = root.join("wrong-occurrence.xml");
    let mutate = "import sys,xml.etree.ElementTree as E; t=E.parse(sys.argv[1]); t.find('./PLAYLISTS/NODE/NODE/TRACK').set('Key','2'); t.write(sys.argv[2])";
    assert!(
        std::process::Command::new("python3")
            .args(["-c", mutate])
            .arg(&output)
            .arg(&wrong)
            .status()
            .unwrap()
            .success()
    );
    let rejected = check(&wrong);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("tree: complete ordered nodes/members differ")
    );
    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    for (path, bytes) in analysis {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn removal_ui_refuses_stale_order_and_same_name_replacement() {
    for replacement in [false, true] {
        let (root, db) = catalog_root(if replacement {
            "remove-replacement"
        } else {
            "remove-stale"
        });
        imported(&db, &root);
        let mut shell = Shell::new(&db, 1440.0);
        let row = text_rect(&mut shell, "♫ Browse");
        shell.click(row.center());
        let row = text_rect(&mut shell, "2 · Imported title");
        shell.click(row.center());
        let out = shell.settle();
        shell.click(
            find(&nodes(&out), "Remove selected from playlist…")
                .2
                .center(),
        );
        let sql = if replacement {
            "DELETE FROM library_members WHERE node_id=1; DELETE FROM library_nodes WHERE id=1; INSERT INTO library_nodes VALUES(1,NULL,0,'Browse',0); INSERT INTO library_members VALUES(1,0,0); INSERT INTO library_members VALUES(1,1,0);"
        } else {
            "DELETE FROM library_members WHERE node_id=1 AND position=0;"
        };
        let script = "import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]); c.executescript(sys.argv[2]); c.commit()";
        let result = std::process::Command::new("python3")
            .args(["-c", script])
            .arg(&db)
            .arg(sql)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let before = Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap();
        let out = shell.settle();
        shell.click(find(&nodes(&out), "Confirm removal").2.center());
        assert!(
            shell.app.notice.contains(if replacement {
                "selected root playlist is stale"
            } else {
                "playlist membership order is stale"
            }),
            "{}",
            shell.app.notice
        );
        assert_eq!(
            Catalog::open_read_only(&db)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            before
        );
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
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
    for name in [
        "Title",
        "Artist",
        "Album",
        "BPM",
        "Key",
        "Duration",
        "1 · Imported title",
        "2 · Imported title",
        "Synthetic artist",
        "Synthetic album",
        "128.50",
        "8A",
        "1:13",
    ] {
        let matches: Vec<_> = texts(&out).into_iter().filter(|(s, _)| s == name).collect();
        assert!(!matches.is_empty(), "column/value {name}");
        assert!(
            matches
                .iter()
                .all(|(_, r)| shell.table().contains(r.center())),
            "only in main table before selection: {name}"
        );
    }
    let row = text_rect(&mut shell, "2 · Imported title");
    shell.click(row.center());
    assert_eq!(shell.app.imported_member, Some(1));
    assert_eq!(shell.app.selected_imported, Some(0));
    let out = shell.settle();
    assert!(texts(&out).iter().any(
        |(s, r)| s == "Imported title" && shell.app.shell.details.unwrap().contains(r.center())
    ));
    shell.app.search = "absent".into();
    assert!(
        !texts(&shell.settle())
            .iter()
            .any(|(s, _)| s.contains("· Imported title"))
    );
    shell.app.search.clear();
    let all = text_rect(&mut shell, "All tracks");
    shell.click(all.center());
    assert!(shell.app.imported_playlist.is_none());
    assert!(shell.app.selected_imported.is_none());
    assert!(
        texts(&shell.settle())
            .iter()
            .any(|(s, _)| s == "tone-a.wav")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn collection_and_playlist_rows_drop_on_each_deck_without_autoplay() {
    for playlist in [false, true] {
        let (root, db) = catalog_root(if playlist {
            "drop-playlist"
        } else {
            "drop-collection"
        });
        imported(&db, &root);
        let mut shell = Shell::new(&db, 1440.0);
        if playlist {
            let r = text_rect(&mut shell, "♫ Browse");
            shell.click(r.center());
        }
        for d in 0..2 {
            let row = text_rect(
                &mut shell,
                if playlist {
                    "1 · Imported title"
                } else {
                    "tone-b.wav"
                },
            );
            start_drag(&mut shell, row.center());
            let target = if d == 0 {
                shell.app.decks.layout.halves.unwrap()[d]
            } else {
                shell.app.decks.layout.overviews.unwrap()[d]
            };
            release(&mut shell, target.center());
            assert_eq!(
                shell.app.decks.decks[d].track.as_ref().unwrap().path,
                root.join("audio/tone-b.wav")
            );
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
        if mode == 0 {
            key(&mut shell, egui::Key::Escape);
        }
        if mode == 1 {
            shell.app.show_decks = false;
        }
        let end = if mode == 2 {
            shell.app.shell.status.unwrap().center()
        } else {
            target
        };
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
    let playlist = text_rect(&mut shell, "♫ Browse");
    shell.click(playlist.center());
    let row = text_rect(&mut shell, "1 · Imported title");
    shell.click(row.center());
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
        shell.app.decks.render(512);
        shell.settle();
        assert!(shell.app.decks.decks[d].cue_held);
        shell.frame(vec![key_event(cue, false)], Default::default());
        shell.app.decks.render(512);
        shell.settle();
        assert!(!shell.app.decks.decks[d].cue_held);
    }
    shell.frame(vec![key_event(egui::Key::Z, true)], Default::default());
    shell.app.decks.render(512);
    shell.settle();
    assert!(shell.app.decks.decks[0].cue_held);
    shell.click(shell.app.shell.search.unwrap().center());
    shell.app.decks.render(512);
    shell.settle();
    assert!(
        !shell.app.decks.decks[0].cue_held,
        "focus change ends cue preview"
    );
    shell.frame(vec![key_event(egui::Key::Z, false)], Default::default());
    for k in [
        egui::Key::X,
        egui::Key::M,
        egui::Key::Z,
        egui::Key::N,
        egui::Key::A,
        egui::Key::B,
    ] {
        key(&mut shell, k);
    }
    assert!(
        shell
            .app
            .decks
            .decks
            .iter()
            .all(|d| !d.playing && !d.cue_held)
    );
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
    let before = shell.app.decks.decks[0]
        .track
        .as_ref()
        .unwrap()
        .path
        .clone();
    let playlist = text_rect(&mut shell, "♫ Browse");
    shell.click(playlist.center());
    std::fs::remove_file(root.join("audio/tone-b.wav")).unwrap();
    let row = text_rect(&mut shell, "1 · Imported title");
    start_drag(&mut shell, row.center());
    let target = shell.app.decks.layout.halves.unwrap()[0].center();
    release(&mut shell, target);
    assert!(shell.app.notice.starts_with("Playlist load refused:"));
    assert_eq!(
        shell.app.decks.decks[0].track.as_ref().unwrap().path,
        before
    );
    assert!(shell.app.decks.decks[1].track.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
