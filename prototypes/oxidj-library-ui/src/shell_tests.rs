//! W8 dense shell: production `LibraryApp::ui` rendered at 1440 and 900 px.
//! The track table fills what the visible panels leave, each panel toggles by
//! click and by shortcut, the "…" menu reaches the catalog/import/XML
//! controls, and nothing is clipped. Synthetic audio only.

use super::*;
use eframe::egui::{Pos2, Rect, accesskit::Role};
use std::path::Path;

/// 16-bit mono PCM WAV of a 440 Hz tone for `seconds`.
fn write_wav(path: &Path, seconds: f32) {
    let rate = 44_100u32;
    let n = (rate as f32 * seconds) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + n * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(n * 2).to_le_bytes());
    for i in 0..n {
        let v = ((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 16_000.0) as i16;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

/// A temporary catalog with three imported synthetic tracks.
fn catalog_root(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-w8-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let audio = root.join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    for (name, seconds) in [
        ("tone-a.wav", 0.6),
        ("tone-b.wav", 0.3),
        ("tone-c.wav", 0.9),
    ] {
        write_wav(&audio.join(name), seconds);
    }
    let db = root.join("catalog.sqlite");
    Catalog::open(&db)
        .unwrap()
        .import_audio_directory(audio.to_str().unwrap())
        .unwrap();
    (root, db)
}

type Node = (Role, Option<String>, Rect);

struct Shell {
    ctx: egui::Context,
    app: LibraryApp,
    screen: Rect,
}

impl Shell {
    fn new(db: &Path, width: f32) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = LibraryApp::new(
            db.to_path_buf(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        let mut shell = Self {
            ctx,
            app,
            screen: Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 900.0)),
        };
        shell.settle();
        shell
    }

    fn frame(&mut self, events: Vec<egui::Event>, modifiers: egui::Modifiers) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(self.screen),
            events,
            modifiers,
            ..Default::default()
        };
        let app = &mut self.app;
        self.ctx.run(input, |ctx| app.ui(ctx))
    }

    fn settle(&mut self) -> egui::FullOutput {
        self.frame(vec![], Default::default());
        self.frame(vec![], Default::default())
    }

    /// Real press and release on separate frames, then settle.
    fn click(&mut self, pos: Pos2) -> egui::FullOutput {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        self.frame(
            vec![egui::Event::PointerMoved(pos), button(true)],
            Default::default(),
        );
        self.frame(vec![button(false)], Default::default());
        self.settle()
    }

    /// Ctrl+`key` as winit reports it on Linux (ctrl and command both set).
    fn shortcut(&mut self, key: egui::Key) -> egui::FullOutput {
        let modifiers = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![event(true)], modifiers);
        self.frame(vec![event(false)], Default::default());
        self.settle()
    }

    fn table(&self) -> Rect {
        self.app.shell.table.expect("track table painted")
    }
}

fn nodes(output: &egui::FullOutput) -> Vec<Node> {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("AccessKit update")
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let r = node.bounds()?;
            Some((
                node.role(),
                node.label().map(str::to_owned),
                Rect::from_min_max(
                    Pos2::new(r.x0 as f32, r.y0 as f32),
                    Pos2::new(r.x1 as f32, r.y1 as f32),
                ),
            ))
        })
        .collect()
}

fn find<'a>(nodes: &'a [Node], label: &str) -> &'a Node {
    nodes
        .iter()
        .find(|n| n.1.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("{label:?} not in the AccessKit tree"))
}

fn controls(nodes: &[Node]) -> Vec<&Node> {
    nodes
        .iter()
        .filter(|n| matches!(n.0, Role::Button | Role::TextInput | Role::ComboBox))
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1.5
}

#[test]
fn defaults_one_row_toolbar_status_bar_and_full_width_table() {
    let (root, db) = catalog_root("defaults");
    for width in [1440.0, 900.0] {
        let mut shell = Shell::new(&db, width);
        // Many frames: a layout that fills the available height would grow
        // the toolbar a little every frame (seen on the first live build).
        for _ in 0..30 {
            shell.settle();
        }
        let out = shell.settle();
        let app = &shell.app;
        assert!(
            app.show_nav && !app.show_details && !app.show_decks,
            "defaults: nav on, details/decks off"
        );
        let layout = app.shell;
        let nav = layout.nav.expect("navigation shown by default");
        assert!(layout.details.is_none() && layout.decks.is_none());
        let toolbar = layout.toolbar.unwrap();
        assert!(
            toolbar.height() <= 33.0,
            "{width}: toolbar is one slim row: {toolbar:?}"
        );
        let status = layout.status.unwrap();
        assert!(
            status.height() <= 33.0 && close(status.bottom(), 900.0),
            "{width}: status bar {status:?}"
        );
        let table = shell.table();
        assert!(
            close(table.left(), nav.right()) && close(table.right(), width),
            "{width}: table {table:?} nav {nav:?}"
        );
        assert!(
            close(table.top(), toolbar.bottom()),
            "{width}: no gap under the toolbar: {table:?}"
        );

        let tree = nodes(&out);
        // Every toolbar control sits on the single toolbar row, unclipped,
        // without overlaps, and meets the 24 px target.
        let bar: Vec<_> = controls(&tree)
            .into_iter()
            .filter(|n| toolbar.contains_rect(n.2))
            .collect();
        assert!(
            bar.len() >= 6,
            "{width}: search, 3 toggles, theme, more: {bar:?}"
        );
        for (i, a) in bar.iter().enumerate() {
            assert!(
                shell.screen.contains_rect(a.2) && a.2.height() >= 24.0,
                "{width}: {a:?}"
            );
            assert!(
                bar[i + 1..].iter().all(|b| !a.2.intersects(b.2)),
                "{width}: overlap {a:?}"
            );
        }
        let search = bar
            .iter()
            .find(|n| n.0 == Role::TextInput)
            .expect("search field");
        assert!(
            search.2.width() >= 150.0,
            "{width}: search grows: {:?}",
            search.2
        );
        // Sortable header cells are clickable sort targets: >= 24 px tall,
        // inside the table, unclipped (review W8 round 1).
        for title in ["ID", "Filename", "Duration"] {
            let header = tree
                .iter()
                .find(|n| {
                    n.1.as_deref()
                        .is_some_and(|l| l == title || l.starts_with(&format!("{title}, sorted")))
                })
                .unwrap_or_else(|| panic!("{width}: header {title} not in the AccessKit tree"))
                .2;
            assert!(
                header.height() >= 24.0 && header.width() >= 24.0,
                "{width}: header {title} target {header:?}"
            );
            assert!(
                shell.screen.contains_rect(header)
                    && header.left() >= table.left() - 0.5
                    && header.right() <= table.right() + 0.5,
                "{width}: header {title} clipped {header:?} table {table:?}"
            );
        }
        // Rows are full-width >= 24 px hit targets.
        for name in ["tone-a.wav", "tone-b.wav", "tone-c.wav"] {
            let row = find(&tree, name).2;
            assert!(
                row.height() >= 24.0 && row.height() <= 25.0,
                "{width}: row {row:?}"
            );
            assert!(
                close(row.left(), table.left()) && close(row.right(), table.right()),
                "{width}: {row:?}"
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn each_panel_toggles_by_click_and_shortcut_and_resizes_the_table() {
    let (root, db) = catalog_root("toggles");
    for width in [1440.0, 900.0] {
        let mut shell = Shell::new(&db, width);
        // Both side panels off: the table spans the whole window width.
        let nav_toggle = shell.app.shell.toggles.unwrap()[0];
        shell.click(nav_toggle.center());
        assert!(!shell.app.show_nav && shell.app.shell.nav.is_none());
        let full = shell.table();
        assert!(
            close(full.left(), 0.0) && close(full.right(), width),
            "{width}: full-width table {full:?}"
        );

        let keys = [egui::Key::B, egui::Key::I, egui::Key::D];
        for (index, key) in keys.into_iter().enumerate() {
            let panel =
                |app: &LibraryApp| [app.shell.nav, app.shell.details, app.shell.decks][index];
            for via_shortcut in [false, true] {
                let before = shell.table();
                if via_shortcut {
                    shell.shortcut(key);
                } else {
                    shell.click(shell.app.shell.toggles.unwrap()[index].center());
                }
                let shown = panel(&shell.app).unwrap_or_else(|| {
                    panic!("{width}: panel {index} shown (shortcut {via_shortcut})")
                });
                let after = shell.table();
                assert!(
                    shell.screen.contains_rect(shown),
                    "{width}: panel {index} clipped {shown:?}"
                );
                if index == 2 {
                    assert!(
                        close(before.height() - after.height(), shown.height()),
                        "{width}: decks {before:?} -> {after:?}, {shown:?}"
                    );
                    assert!(close(after.width(), before.width()));
                } else {
                    assert!(
                        close(before.width() - after.width(), shown.width()),
                        "{width}: panel {index} {before:?} -> {after:?}, {shown:?}"
                    );
                }
                if via_shortcut {
                    shell.shortcut(key);
                } else {
                    shell.click(shell.app.shell.toggles.unwrap()[index].center());
                }
                assert!(
                    panel(&shell.app).is_none(),
                    "{width}: panel {index} hidden again"
                );
                assert_eq!(
                    shell.table(),
                    before,
                    "{width}: hiding panel {index} returns its space"
                );
            }
        }
        // All three on: everything still fits.
        for key in keys {
            shell.shortcut(key);
        }
        let out = shell.settle();
        let layout = shell.app.shell;
        for rect in [layout.nav, layout.details, layout.decks, layout.table]
            .into_iter()
            .flatten()
        {
            assert!(
                shell.screen.contains_rect(rect),
                "{width}: {rect:?} clipped"
            );
        }
        assert!(
            shell.table().width() >= 300.0,
            "{width}: table keeps room: {:?}",
            shell.table()
        );
        for control in controls(&nodes(&out))
            .into_iter()
            .filter(|n| layout.toolbar.unwrap().contains_rect(n.2))
        {
            assert!(
                shell.screen.contains_rect(control.2),
                "{width}: {control:?}"
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selecting_a_row_does_not_open_details_and_headers_sort() {
    let (root, db) = catalog_root("select");
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let row = find(&nodes(&out), "tone-b.wav").2;
    shell.click(row.center());
    let id = shell
        .app
        .tracks
        .iter()
        .find(|t| t.track.path.ends_with("tone-b.wav"))
        .unwrap()
        .track
        .id
        .as_u64();
    assert_eq!(shell.app.selected, Some(id));
    assert!(
        !shell.app.show_details && shell.app.shell.details.is_none(),
        "selection alone keeps details closed"
    );
    // Opening details then shows the selection.
    shell.shortcut(egui::Key::I);
    assert!(shell.app.shell.details.is_some());

    // Duration header: click sorts ascending, again descending.
    let out = shell.settle();
    let header = find(&nodes(&out), "Duration").2;
    let out = shell.click(header.center());
    assert!(shell.app.sort == Sort::Duration && !shell.app.descending);
    let top = |out: &egui::FullOutput| {
        let tree = nodes(out);
        ["tone-a.wav", "tone-b.wav", "tone-c.wav"]
            .into_iter()
            .min_by(|a, b| find(&tree, a).2.top().total_cmp(&find(&tree, b).2.top()))
            .unwrap()
    };
    assert_eq!(top(&out), "tone-b.wav", "shortest first");
    let header = find(&nodes(&out), "Duration, sorted ascending").2;
    let out = shell.click(header.center());
    assert!(shell.app.descending);
    assert_eq!(top(&out), "tone-c.wav", "longest first");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn more_menu_reaches_catalog_import_and_xml_controls() {
    let (root, db) = catalog_root("more");
    let xml = root.join("missing.xml");
    for width in [1440.0, 900.0] {
        let mut shell = Shell::new(&db, width);
        let out = shell.settle();
        assert!(
            nodes(&out)
                .iter()
                .all(|n| n.1.as_deref() != Some("Preview XML")),
            "fields live in the menu"
        );
        let out = shell.click(shell.app.shell.more.unwrap().center());
        let popup = shell.app.shell.more_popup.expect("… menu open");
        assert!(
            shell.screen.contains_rect(popup),
            "{width}: menu clipped {popup:?}"
        );
        // Right-aligned under "…" from its fixed width (not a stale size).
        let more = shell.app.shell.more.unwrap();
        assert!(
            close(popup.right(), more.right()) && popup.top() >= more.bottom(),
            "{width}: {popup:?} under {more:?}"
        );
        let tree = nodes(&out);
        for label in [
            "Reload",
            "Import references",
            "Preview XML",
            "Control concepts…",
            "Waveform view…",
        ] {
            let node = find(&tree, label);
            assert!(
                popup.contains_rect(node.2) && node.2.height() >= 24.0,
                "{width}: {label} {:?}",
                node.2
            );
        }
        let fields: Vec<_> = tree
            .iter()
            .filter(|n| n.0 == Role::TextInput && popup.contains_rect(n.2))
            .collect();
        assert_eq!(
            fields.len(),
            2,
            "{width}: import + XML fields in the menu: {fields:?}"
        );
        // The menu's XML action runs the existing preview workflow and closes.
        shell.app.xml_path = xml.display().to_string();
        let preview = find(&tree, "Preview XML").2;
        shell.click(preview.center());
        assert!(
            shell.app.show_xml_preview,
            "{width}: Preview XML opened the XML window"
        );
        assert!(
            shell.app.shell.more_popup.is_none(),
            "{width}: menu closed after the action"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// W9: the decks bar keeps both decks' Play/Cue inside the window and the
/// track name in its truncated column, even for a very long filename, at
/// 1440 and 900 px; Play still toggles by a real click.
#[test]
fn decks_bar_truncates_long_names_and_keeps_play_cue_reachable() {
    let (root, _) = catalog_root("decks");
    let long = format!("{}.wav", "very-long-synthetic-track-name-".repeat(5));
    write_wav(&root.join("audio").join(&long), 0.2);
    let db = root.join("catalog.sqlite");
    Catalog::open(&db)
        .unwrap()
        .import_audio_directory(root.join("audio").to_str().unwrap())
        .unwrap();
    for width in [1440.0, 900.0] {
        let mut shell = Shell::new(&db, width);
        shell.app.reload();
        let id = shell
            .app
            .tracks
            .iter()
            .find(|t| t.track.path.ends_with(&long))
            .expect("long-named track imported")
            .track
            .id
            .as_u64();
        shell.app.selected = Some(id);
        shell.app.show_decks = true;
        let out = shell.settle();
        let decks = shell.app.shell.decks.expect("decks panel");
        assert!(shell.screen.contains_rect(decks), "{width}: {decks:?}");
        let tree = nodes(&out);
        for label in ["Play", "Cue"] {
            let hits: Vec<_> = tree
                .iter()
                .filter(|n| n.1.as_deref() == Some(label))
                .collect();
            assert_eq!(hits.len(), 2, "{width}: one {label} per deck: {hits:?}");
            for n in hits {
                assert!(
                    decks.contains_rect(n.2),
                    "{width}: {label} {:?} outside {decks:?}",
                    n.2
                );
                assert!(n.2.height() >= 24.0, "{width}: {label} target {:?}", n.2);
            }
        }
        // The name is painted elided inside the window, never past its column.
        let names: Vec<Rect> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t)
                    if t.galley.job.text == long && decks.intersects(t.visual_bounding_rect()) =>
                {
                    Some(t.visual_bounding_rect())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names.len(), 2, "{width}: {names:?}");
        for name in names {
            assert!(
                name.width() <= view::DECK_NAME_WIDTH + 0.5,
                "{width}: name {name:?}"
            );
            assert!(shell.screen.contains_rect(name), "{width}: {name:?}");
        }
        let play = find(&tree, "Play").2;
        let deck_controls = |tree: &[Node]| -> Vec<Rect> {
            tree.iter()
                .filter(|n| matches!(n.1.as_deref(), Some("Play" | "Cue")))
                .map(|n| n.2)
                .collect()
        };
        let before = deck_controls(&tree);
        let out = shell.click(play.center());
        assert!(
            shell.app.deck_playing[0] && !shell.app.deck_playing[1],
            "{width}: Play toggles deck A"
        );
        // Toggling never moves the deck controls (the label stays "Play").
        assert_eq!(
            before,
            deck_controls(&nodes(&out)),
            "{width}: deck controls stay put"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
