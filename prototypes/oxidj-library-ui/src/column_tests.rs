//! Production table gestures, accessible alternatives and a fresh-app restart.
use super::*;

fn drag(shell: &mut Shell, from: Pos2, to: Pos2) {
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
        vec![egui::Event::PointerMoved(from + (to - from) * 0.5)],
        Default::default(),
    );
    shell.frame(vec![egui::Event::PointerMoved(to)], Default::default());
    shell.frame(
        vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
        Default::default(),
    );
    shell.settle();
}

fn header(shell: &mut Shell, label: &str) -> Rect {
    let out = shell.settle();
    nodes(&out)
        .into_iter()
        .find(|n| {
            n.0 == Role::Button
                && n.1
                    .as_deref()
                    .is_some_and(|s| s == label || s.starts_with(&format!("{label}, sorted ")))
                && shell.table().contains(n.2.center())
        })
        .unwrap_or_else(|| panic!("header {label}"))
        .2
}

fn menu(shell: &mut Shell) -> egui::FullOutput {
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Columns").2.center())
}

fn checkbox(out: &egui::FullOutput, label: &str) -> Pos2 {
    nodes(out)
        .into_iter()
        .find(|n| n.0 == Role::CheckBox && n.1.as_deref() == Some(label))
        .unwrap()
        .2
        .center()
}

fn layout(shell: &Shell, playlist: bool) -> crate::settings::TableColumns {
    if playlist {
        shell.app.settings.playlist_columns
    } else {
        shell.app.settings.catalog_columns
    }
}

#[test]
fn column_drag_resize_visibility_restart_and_row_selection_in_both_tables() {
    for playlist in [false, true] {
        let (root, db) = catalog_root(if playlist {
            "columns-playlist"
        } else {
            "columns-catalog"
        });
        imported(&db, &root);
        let snapshot = Catalog::open_read_only(&db)
            .unwrap()
            .library_snapshot()
            .unwrap();
        let mut shell = Shell::new(&db, 1440.0);
        if playlist {
            let r = text_rect(&mut shell, "♫ Browse");
            shell.click(r.center());
        }
        let first = if playlist { "Title" } else { "Availability" };
        let second = if playlist { "Artist" } else { "ID" };
        let h1 = header(&mut shell, first);
        let h2 = header(&mut shell, second);
        drag(&mut shell, h1.center(), h2.center());
        assert_eq!(&layout(&shell, playlist).order[..2], &[1, 0]);
        assert!(matches!(shell.app.sort, Sort::Id));
        assert!(!shell.app.descending, "reorder must not trigger sort");
        let h = header(&mut shell, first);
        let width = layout(&shell, playlist).widths[0];
        let edge = egui::pos2(h.right() + 3.0, h.center().y);
        drag(&mut shell, edge, edge + egui::vec2(65.0, 0.0));
        assert_eq!(layout(&shell, playlist).widths[0], width + 65);
        assert!(!shell.app.descending, "resize must not trigger sort");
        let out = menu(&mut shell);
        shell.click(checkbox(&out, if playlist { "Album" } else { "Folder" }));
        assert!(!layout(&shell, playlist).visible[if playlist { 2 } else { 3 }]);
        key(&mut shell, egui::Key::Escape);
        let expected = shell.app.settings;
        // The real App::save path writes through eframe::Storage. Reconstruct
        // storage, egui context and LibraryApp, not just the layout value.
        let mut storage = FileStorage(root.join("settings.txt"));
        eframe::App::save(&mut shell.app, &mut storage);
        drop(shell);
        let storage = FileStorage(root.join("settings.txt"));
        let mut restarted = Shell::new(&db, 1440.0);
        restarted.app.settings = crate::settings::Settings::load(Some(&storage));
        assert_eq!(restarted.app.settings, expected);
        if playlist {
            let r = text_rect(&mut restarted, "♫ Browse");
            restarted.click(r.center());
        }
        assert!(header(&mut restarted, second).left() < header(&mut restarted, first).left());
        let row = text_rect(
            &mut restarted,
            if playlist {
                "1 · Imported title"
            } else {
                "tone-a.wav"
            },
        );
        restarted.click(row.center());
        if playlist {
            assert_eq!(restarted.app.imported_member, Some(0));
        } else {
            assert!(restarted.app.selected.is_some());
        }
        assert_eq!(
            Catalog::open_read_only(&db)
                .unwrap()
                .library_snapshot()
                .unwrap(),
            snapshot,
            "column gestures never edit catalog metadata/memberships"
        );
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn column_sortable_header_drag_reverse_cancel_and_resize_limits() {
    let (root, db) = catalog_root("column-gesture-limits");
    let mut shell = Shell::new(&db, 1440.0);
    let id = header(&mut shell, "ID").center();
    let filename = header(&mut shell, "Filename").center();
    drag(&mut shell, id, filename);
    assert_eq!(shell.app.settings.catalog_columns.order, [0, 2, 1, 3, 4, 5]);
    assert!(!shell.app.descending);
    let id = header(&mut shell, "ID").center();
    let filename = header(&mut shell, "Filename").center();
    drag(&mut shell, id, filename);
    assert_eq!(shell.app.settings.catalog_columns.order, [0, 1, 2, 3, 4, 5]);
    let original = shell.app.settings.catalog_columns;
    let id = header(&mut shell, "ID").center();
    let outside = egui::pos2(id.x, shell.table().bottom() - 10.0);
    drag(&mut shell, id, outside);
    assert_eq!(
        shell.app.settings.catalog_columns, original,
        "outside release cancels reorder"
    );
    for (delta, expected) in [(-1000.0, 24), (3000.0, 2000)] {
        let h = header(&mut shell, "ID");
        let edge = egui::pos2(h.right() + 3.0, h.center().y);
        drag(&mut shell, edge, edge + egui::vec2(delta, 0.0));
        assert_eq!(shell.app.settings.catalog_columns.widths[1], expected);
        assert!(!shell.app.descending);
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn column_narrow_table_scrolls_headers_and_rows_together_and_keeps_menu_reachable() {
    for playlist in [false, true] {
        let (root, db) = catalog_root(if playlist {
            "columns-narrow-playlist"
        } else {
            "columns-narrow-catalog"
        });
        imported(&db, &root);
        let mut shell = Shell::new(&db, 600.0);
        shell.app.show_nav = false;
        shell.app.show_details = false;
        shell.app.show_decks = false;
        if playlist {
            shell.app.imported_playlist = Some(vec![0]);
        }
        shell.settle();
        let pointer = shell.table().center();
        shell.frame(
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-1500.0, 0.0),
                    modifiers: Default::default(),
                },
            ],
            Default::default(),
        );
        // Let egui consume its smooth-scroll delta over multiple frames.
        for _ in 0..20 {
            shell.settle();
        }
        let out = shell.settle();
        let label = if playlist { "Duration" } else { "Format" };
        let h = header(&mut shell, label);
        assert!(
            shell.table().contains(h.center()),
            "last column is reachable by horizontal scroll"
        );
        let value = if playlist {
            "1:13"
        } else {
            "44.1 kHz · 16-bit · mono"
        };
        let r = texts(&out).into_iter().find(|(s, _)| s == value).unwrap().1;
        assert!(
            r.left() >= h.left() && r.right() <= h.right() + 5.0,
            "rows follow horizontally scrolled header"
        );
        let out = menu(&mut shell);
        assert!(
            nodes(&out).iter().any(|n| n.0 == Role::CheckBox),
            "pinned menu remains usable"
        );
        drop(shell);
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// A disk-backed single-key storage for the actual settings save/load boundary.
struct FileStorage(std::path::PathBuf);
impl eframe::Storage for FileStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        (key == "ui_settings")
            .then(|| std::fs::read_to_string(&self.0).ok())
            .flatten()
    }
    fn set_string(&mut self, key: &str, value: String) {
        if key == "ui_settings" {
            std::fs::write(&self.0, value).unwrap();
        }
    }
    fn flush(&mut self) {}
}

#[test]
fn column_menu_keyboard_access_autofit_reset_and_last_visible_guard() {
    let (root, db) = catalog_root("column-menu-actions");
    let mut shell = Shell::new(&db, 900.0);
    let out = shell.settle();
    let button = find(&nodes(&out), "Columns").2.center();
    // Click to acquire focus, close, then keyboard activation of the same
    // normal menu control (no right/middle mouse required).
    shell.click(button);
    key(&mut shell, egui::Key::Escape);
    let out = shell.settle();
    let button_id = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Columns"))
        .unwrap()
        .0;
    shell.frame(
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target: button_id,
                data: None,
            },
        )],
        Default::default(),
    );
    key(&mut shell, egui::Key::Enter);
    let out = shell.settle();
    assert!(
        nodes(&out)
            .iter()
            .any(|n| n.1.as_deref() == Some("Auto-fit columns"))
    );
    shell.click(find(&nodes(&out), "Auto-fit columns").2.center());
    assert!(
        shell.app.settings.catalog_columns.widths[3] < 160,
        "auto-fit measures actual folder contents rather than restoring defaults"
    );
    let out = menu(&mut shell);
    shell.click(find(&nodes(&out), "Reset columns").2.center());
    assert_eq!(
        shell.app.settings.catalog_columns,
        crate::settings::TableColumns::default()
    );
    shell.app.settings.catalog_columns.visible = [false, false, true, false, false, false];
    let out = menu(&mut shell);
    shell.click(checkbox(&out, "Filename"));
    assert!(
        shell.app.settings.catalog_columns.visible[2],
        "at least one column stays visible"
    );
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn column_middle_click_fits_one_column_and_catalog_sort_still_clicks() {
    let (root, db) = catalog_root("column-fit-sort");
    let mut shell = Shell::new(&db, 1440.0);
    let pos = header(&mut shell, "Folder").center();
    let before = shell.app.settings.catalog_columns;
    shell.frame(vec![egui::Event::PointerMoved(pos)], Default::default());
    for pressed in [true, false] {
        shell.frame(
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Middle,
                pressed,
                modifiers: Default::default(),
            }],
            Default::default(),
        );
    }
    shell.settle();
    assert!(shell.app.settings.catalog_columns.widths[3] < before.widths[3]);
    for i in [0, 1, 2, 4, 5] {
        assert_eq!(
            shell.app.settings.catalog_columns.widths[i],
            before.widths[i]
        );
    }
    let id = header(&mut shell, "ID").center();
    shell.click(id);
    assert!(shell.app.descending);
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_sort_changes_visible_order_without_changing_member_identity() {
    let (root, db) = catalog_root("imported-sort-members");
    Catalog::open(&db).unwrap().import_library(&LibrarySnapshot {
        tracks: vec![
            LibraryTrack { title: "Zulu".into(), location: "file://localhost/z.mp3".into(), ..Default::default() },
            LibraryTrack { title: "Alpha".into(), location: "file://localhost/a.mp3".into(), ..Default::default() },
        ],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Browse".into(), members: vec![0, 1, 0] }],
    }).unwrap();
    let before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let browse = text_rect(&mut shell, "♫ Browse");
    shell.click(browse.center());
    let title = header(&mut shell, "Title");
    shell.click(title.center());
    let alpha = text_rect(&mut shell, "2 · Alpha");
    let zulu = text_rect(&mut shell, "1 · Zulu");
    assert!(alpha.top() < zulu.top());
    shell.click(alpha.center());
    assert_eq!(shell.app.imported_member, Some(1));
    assert_eq!(shell.app.selected_imported, Some(1));
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), before);
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_header_click_changes_sort_direction() {
    let (root, db) = catalog_root("imported-header-sort");
    imported(&db, &root);
    let mut shell = Shell::new(&db, 1440.0);
    let browse = text_rect(&mut shell, "♫ Browse");
    shell.click(browse.center());
    let title = header(&mut shell, "Title");
    let out = shell.click(title.center());
    assert!(nodes(&out).iter().any(|n| n.1.as_deref() == Some("Title, sorted ascending")));
    let title = header(&mut shell, "Title");
    let out = shell.click(title.center());
    assert!(nodes(&out).iter().any(|n| n.1.as_deref() == Some("Title, sorted descending")));
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn column_missing_imported_values_are_not_invented() {
    let (root, db) = catalog_root("column-missing-values");
    Catalog::open(&db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            tracks: vec![LibraryTrack {
                title: "Unknown facts".into(),
                location: "file://localhost/not-present.wav".into(),
                ..Default::default()
            }],
            roots: vec![oxidj_library::LibraryNode::Playlist {
                name: "Unknown".into(),
                members: vec![0],
            }],
        })
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let r = text_rect(&mut shell, "♫ Unknown");
    shell.click(r.center());
    let out = shell.settle();
    let values: Vec<_> = texts(&out)
        .into_iter()
        .filter(|(_, r)| shell.table().contains(r.center()))
        .map(|(s, _)| s)
        .collect();
    assert_eq!(values.iter().filter(|s| *s == "—").count(), 5);
    assert!(!values.iter().any(|s| s == "0:00" || s == "0.00"));
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn old_and_malformed_column_settings_preserve_other_preferences() {
    for columns in [
        "",
        ";columns_catalog=0:1:100,0:1:100",
        ";columns_catalog=0:0:100,1:0:100,2:0:100,3:0:100,4:0:100,5:0:100",
        ";columns_playlist=0:1:23,1:1:80,2:1:200,3:1:160,4:1:90,5:1:100",
        ";columns_catalog=0:1:100,1:1:80,2:1:200,3:1:160,4:1:90,9:1:100",
    ] {
        let settings = crate::settings::Settings::decode(&format!("zoom=8{columns}"));
        assert_eq!(settings.zoom_bars, 8);
        assert_eq!(
            settings.catalog_columns,
            crate::settings::Settings::default().catalog_columns
        );
        assert_eq!(
            settings.playlist_columns,
            crate::settings::Settings::default().playlist_columns
        );
    }
}
