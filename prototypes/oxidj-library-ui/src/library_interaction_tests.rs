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
