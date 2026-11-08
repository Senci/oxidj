//! W8 dense shell: production `LibraryApp::ui` rendered at 1440 and 900 px.
//! The track table fills what the visible panels leave, each panel toggles by
//! click and by shortcut, the "…" menu reaches the catalog/import/XML
//! controls, and nothing is clipped. Synthetic audio only.

use super::*;
use eframe::egui::{Pos2, Rect, accesskit::Role};
use oxidj_library::{LibraryCue, LibrarySnapshot, LibraryTrack};
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
            app.show_nav && app.show_details && app.show_decks,
            "defaults: navigation, details, and decks on"
        );
        let layout = app.shell;
        let nav = layout.nav.expect("navigation shown by default");
        let details = layout.details.expect("details shown by default");
        let decks = layout.decks.expect("decks shown by default");
        assert!(details.width() > 0.0 && decks.width() > 0.0);
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
            close(table.left(), nav.right()) && close(table.right(), details.left()),
            "{width}: table {table:?} between nav {nav:?} and details {details:?}"
        );
        assert!(
            table.top() >= decks.bottom() && table.bottom() <= status.top(),
            "{width}: table {table:?} fits below decks {decks:?} and above status {status:?}"
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
            if width >= 1200.0 {
                assert!(
                    shell.screen.contains_rect(header)
                        && header.left() >= table.left() - 0.5
                        && header.right() <= table.right() + 0.5,
                    "{width}: header {title} clipped {header:?} table {table:?}"
                );
            }
        }
        // Rows are full-width >= 24 px hit targets.
        for name in ["tone-a.wav", "tone-b.wav", "tone-c.wav"] {
            let row = find(&tree, name).2;
            assert!(
                row.height() >= 24.0 && row.height() <= 25.0,
                "{width}: row {row:?}"
            );
            if width >= 1200.0 {
                assert!(
                    close(row.left(), table.left()) && close(row.right(), table.right()),
                    "{width}: {row:?}"
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn each_panel_toggles_by_click_and_shortcut_and_resizes_the_table() {
    let (root, db) = catalog_root("toggles");
    for width in [1440.0, 900.0] {
        let mut shell = Shell::new(&db, width);
        // The table spans the full window width when navigation is hidden;
        // Details and Decks remain independently toggleable from their defaults.
        let nav_toggle = shell.app.shell.toggles.unwrap()[0];
        shell.click(nav_toggle.center());
        shell.click(shell.app.shell.toggles.unwrap()[1].center());
        assert!(
            !shell.app.show_nav && !shell.app.show_details && shell.app.shell.nav.is_none()
                && shell.app.shell.details.is_none()
        );
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
                // Normalize to hidden so the same assertions cover all fresh defaults.
                if panel(&shell.app).is_some() {
                    if via_shortcut {
                        shell.shortcut(key);
                    } else {
                        shell.click(shell.app.shell.toggles.unwrap()[index].center());
                    }
                }
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
    shell.shortcut(egui::Key::I);
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

#[test]
fn waveform_job_failure_is_reported_as_failed_by_production_ui() {
    let (root, db) = catalog_root("waveform-failure-count");
    let mut shell = Shell::new(&db, 1200.0);
    shell.app.waveform_job = Some(waveform_analysis::Job::finished_for_test(vec![(
        1,
        Err("synthetic per-track failure".into()),
    )]));

    let _ = shell.settle();

    assert!(
        shell.app.notice.contains("0 succeeded, 1 failed (of 1)"),
        "failed track must not be reported as successful: {}",
        shell.app.notice
    );
    assert!(shell.app.waveform_job.is_none(), "finished job is consumed");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_references_queues_new_tracks_and_reloads_waveforms_on_completion() {
    let root = std::env::temp_dir().join(format!("oxidj-ui-waveform-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let audio = root.join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    let source = audio.join("new-tone.wav");
    write_wav(&source, 0.4);
    std::fs::write(audio.join("unsupported.txt"), b"not audio").unwrap();
    let before = std::fs::read(&source).unwrap();
    let db = root.join("catalog.sqlite");
    Catalog::open(&db).unwrap();
    let mut shell = Shell::new(&db, 1200.0);
    let worker_gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    shell.app.waveform_gate = Some(worker_gate.clone());
    shell.app.import_root = audio.display().to_string();
    shell.settle();
    let more = shell.app.shell.more.unwrap().center();
    let out = shell.click(more);
    let import = find(&nodes(&out), "Import references").2.center();
    shell.click(import);
    assert!(shell.app.waveform_job.is_some(), "newly imported tracks start background analysis");
    // Worker is deterministically held before catalog analysis; the production frame still runs.
    let _ = shell.settle();
    assert!(shell.app.waveform_job.is_some(), "job remains in flight while gated");
    assert!(shell.app.notice.contains("0/1"), "progress while gated: {}", shell.app.notice);
    let id = shell.app.tracks.iter().find(|track| track.track.path.ends_with("new-tone.wav")).unwrap().track.id;
    shell.app.selected = Some(id.as_u64());
    shell.app.show_details = true;
    let out = shell.settle();
    assert!(texts(&out).iter().any(|(text, _)| text == "No computed waveform: track not analysed"), "cache the not-yet-analysed state before completion");
    assert!(shell.app.track_waveform.layout.view.is_none());
    // Reopening the production menu must not discard the held job, even when
    // importing the same (now unchanged) directory again.
    let more = shell.app.shell.more.unwrap().center();
    let out = shell.click(more);
    let import = find(&nodes(&out), "Import references").2.center();
    shell.click(import);
    let retained_after_click = shell.app.waveform_job.is_some();
    // Also protect the command boundary, not only the disabled widget.
    shell.app.import();
    let retained_after_command = shell.app.waveform_job.is_some();
    worker_gate.wait();
    assert!(retained_after_click, "repeat Import click must retain the active job");
    assert!(retained_after_command, "repeat Import command must retain the active job");
    let started = std::time::Instant::now();
    while shell.app.waveform_job.is_some() {
        assert!(started.elapsed().as_secs() < 20, "analysis worker completes");
        let _ = shell.settle();
        std::thread::yield_now();
    }
    assert!(shell.app.notice.contains("1 succeeded, 0 failed"), "truthful summary: {}", shell.app.notice);
    let out = shell.settle();
    assert!(shell.app.track_waveform.layout.view.is_some(), "completion invalidates cached absence and renders computed peaks");
    assert!(!texts(&out).iter().any(|(text, _)| text == "No computed waveform: track not analysed"));
    assert!(shell.app.catalog.as_ref().unwrap().read_waveform(id).unwrap().is_some(), "completion reloads stored peaks");
    assert_eq!(std::fs::read(&source).unwrap(), before, "source bytes remain untouched");
    // A second production import reports the same file as unchanged and queues nothing.
    shell.app.import();
    assert!(shell.app.waveform_job.is_none(), "unchanged imports are not re-analysed");
    // A changed source is reported Conflict and is never queued for analysis.
    let mut changed = before.clone();
    changed[44] ^= 1;
    std::fs::write(&source, changed).unwrap();
    shell.app.import();
    assert!(shell.app.waveform_job.is_none(), "conflicted imports are not queued");
    assert!(shell.app.notice.contains("Import:"), "conflict import reported: {}", shell.app.notice);
    std::fs::remove_dir_all(root).unwrap();
}

/// and the loaded track name elided inside its deck column, even for a very
/// long filename, at 1440 and 900 px; PLAY still toggles by a real click
/// and never moves the controls.
#[test]
fn decks_bar_truncates_long_names_and_keeps_play_cue_reachable() {
    let (root, _) = catalog_root("decks");
    let long = format!("{}.wav", "very-long-synthetic-track-name-".repeat(5));
    write_wav(&root.join("audio").join(&long), 0.6);
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
        shell.app.load_deck(0, id);
        shell.app.load_deck(1, id);
        // D2: controls go live once the decoder worker finishes.
        let start = std::time::Instant::now();
        while !shell.app.decks.decks.iter().all(|d| d.ready()) {
            assert!(start.elapsed().as_secs() < 30, "{width}: decode did not finish");
            std::thread::sleep(std::time::Duration::from_millis(5));
            shell.settle();
        }
        let out = shell.settle();
        let decks = shell.app.shell.decks.expect("decks panel");
        assert!(shell.screen.contains_rect(decks), "{width}: {decks:?}");
        let tree = nodes(&out);
        for label in ["Deck A play/pause", "Deck A cue", "Deck B play/pause", "Deck B cue"] {
            let n = find(&tree, label);
            assert!(decks.contains_rect(n.2), "{width}: {label} {:?} outside {decks:?}", n.2);
            assert!(n.2.height() >= 24.0, "{width}: {label} target {:?}", n.2);
        }
        // Club layout: CUE outermost, then PLAY; A left, B right.
        let x = |l: &str| find(&tree, l).2.center().x;
        assert!(x("Deck A cue") < x("Deck A play/pause") && x("Deck A play/pause") < x("Deck B play/pause"));
        assert!(x("Deck B play/pause") < x("Deck B cue"), "{width}: deck B mirrored");
        // The name is painted elided inside its deck column, never past it.
        let strip = shell.app.decks.layout.strip.expect("strip");
        let names: Vec<Rect> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.job.text.starts_with("very-long") && strip.intersects(t.visual_bounding_rect()) => {
                    Some(t.visual_bounding_rect())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names.len(), 2, "{width}: {names:?}");
        for name in &names {
            assert!(name.width() <= strip.width() / 2.0, "{width}: name {name:?}");
            assert!(shell.screen.contains_rect(*name), "{width}: {name:?}");
        }
        let play = find(&tree, "Deck A play/pause").2;
        let deck_controls = |tree: &[Node]| -> Vec<Rect> {
            ["Deck A play/pause", "Deck A cue", "Deck B play/pause", "Deck B cue"].map(|l| find(tree, l).2).to_vec()
        };
        let before = deck_controls(&tree);
        let out = shell.click(play.center());
        // The engine's fake sink applies the command; the next frame reads it.
        let _ = shell.app.decks.render(800);
        shell.settle();
        assert!(
            shell.app.decks.decks[0].playing && !shell.app.decks.decks[1].playing,
            "{width}: PLAY toggles deck A"
        );
        assert_eq!(before, deck_controls(&nodes(&out)), "{width}: deck controls stay put");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// Painted text shapes: (text, visual rect).
fn texts(output: &egui::FullOutput) -> Vec<(String, Rect)> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            // Layout box (pos + galley size): what the code aligns.
            egui::Shape::Text(t) => Some((t.galley.job.text.clone(), Rect::from_min_size(t.pos, t.galley.size()))),
            _ => None,
        })
        .collect()
}

/// W11: ID and Duration cells and their header titles are right-aligned
/// to one column edge; Folder shows the folder (not the repeated path) and
/// Format is compact.
#[test]
fn numeric_columns_right_align_with_their_headers() {
    let (root, db) = catalog_root("align");
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let table = shell.table();
    let painted: Vec<_> = texts(&out).into_iter().filter(|(_, r)| table.contains_rect(*r)).collect();
    let header_bottom = table.top() + crate::theme::HEADER_HEIGHT;
    let header = |title: &str| {
        painted
            .iter()
            .find(|(t, r)| t == title && r.bottom() <= header_bottom)
            .unwrap_or_else(|| panic!("header {title}"))
            .1
    };
    let cells = |pred: &dyn Fn(&str) -> bool| -> Vec<Rect> {
        painted
            .iter()
            .filter(|(t, r)| r.top() >= header_bottom && pred(t))
            .map(|(_, r)| *r)
            .collect()
    };
    let ids = cells(&|t| t.chars().all(|c| c.is_ascii_digit()));
    let durations = cells(&|t| t.starts_with("0:"));
    assert_eq!((ids.len(), durations.len()), (3, 3), "{painted:?}");
    for (title, column) in [("ID", &ids), ("Duration", &durations)] {
        let edge = header(title).right();
        for cell in column.iter() {
            assert!(
                (cell.right() - edge).abs() <= 1.0,
                "{title} cell {cell:?} right edge != header edge {edge}"
            );
        }
    }
    // The filename column starts after the whole ID column.
    let filename = header("Filename").left();
    assert!(ids.iter().all(|r| r.right() < filename), "{ids:?} {filename}");
    assert!(painted.iter().any(|(t, _)| t == "audio"), "Folder column shows the folder");
    assert!(
        !painted.iter().any(|(t, _)| t.ends_with("tone-a.wav") && t.contains('/')),
        "no duplicated full path column"
    );
    assert!(painted.iter().any(|(t, _)| t == "44.1 kHz · 16-bit · mono"), "compact format");
    std::fs::remove_dir_all(root).unwrap();
}

/// W11: the Nav | Details | Decks group paints its "on" cells with the
/// selection tint and a primary underline, off cells with neither, and
/// AccessKit reports each cell's selected state.
#[test]
fn toggle_group_marks_active_cells() {
    let (root, db) = catalog_root("segmented");
    let mut shell = Shell::new(&db, 1440.0);
    let tokens = ThemeMode::TokyoNight.tokens();
    let check = |shell: &mut Shell, expected: [bool; 3]| {
        let out = shell.settle();
        let toggles = shell.app.shell.toggles.unwrap();
        for (index, on) in expected.into_iter().enumerate() {
            let cell = toggles[index];
            let tinted = out.shapes.iter().any(|c| {
                matches!(&c.shape, egui::Shape::Rect(r)
                    if r.fill == crate::theme::selection_fill(tokens) && r.rect == cell)
            });
            let underline = out.shapes.iter().any(|c| {
                matches!(&c.shape, egui::Shape::Rect(r)
                    if r.fill == tokens.primary && cell.contains_rect(r.rect)
                        && (r.rect.bottom() - cell.bottom()).abs() < 0.01)
            });
            assert_eq!((tinted, underline), (on, on), "cell {index} on={on}");
            let label = ["Nav", "Details", "Decks"][index];
            let node = out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .unwrap()
                .1
                .clone();
            let toggled = if on { egui::accesskit::Toggled::True } else { egui::accesskit::Toggled::False };
            assert_eq!(node.toggled(), Some(toggled), "{label} AccessKit toggled state");
        }
        // Cells are 1 px apart: adjacent hit rects never touch.
        assert!(toggles[0].right() < toggles[1].left() && toggles[1].right() < toggles[2].left());
    };
    check(&mut shell, [true, true, true]);
    shell.click(shell.app.shell.toggles.unwrap()[1].center());
    check(&mut shell, [true, false, true]);
    shell.click(shell.app.shell.toggles.unwrap()[0].center());
    check(&mut shell, [false, false, true]);
    std::fs::remove_dir_all(root).unwrap();
}

/// W11: typing into the search shows a clear (x) button that empties the
/// search and brings every row back; there is no button while empty.
#[test]
fn search_clear_button_appears_with_text_and_clears_it() {
    let (root, db) = catalog_root("search");
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    assert!(shell.app.shell.search_clear.is_none());
    assert!(nodes(&out).iter().all(|n| n.1.as_deref() != Some("Clear search")));
    let field = shell.app.shell.search.expect("search field");
    shell.click(field.center());
    shell.frame(vec![egui::Event::Text("tone-a".into())], Default::default());
    let out = shell.settle();
    assert_eq!(shell.app.search, "tone-a");
    let tree = nodes(&out);
    assert!(tree.iter().all(|n| n.1.as_deref() != Some("tone-b.wav")), "filtered");
    let clear = find(&tree, "Clear search");
    assert_eq!(clear.0, Role::Button);
    assert!(clear.2.height() >= 24.0 && clear.2.width() >= 24.0, "{clear:?}");
    assert!(!clear.2.intersects(field), "button beside the field: {clear:?} {field:?}");
    assert!(shell.app.shell.toolbar.unwrap().contains_rect(clear.2));
    let out = shell.click(clear.2.center());
    assert!(shell.app.search.is_empty(), "cleared");
    assert!(shell.app.shell.search_clear.is_none());
    let tree = nodes(&out);
    for name in ["tone-a.wav", "tone-b.wav", "tone-c.wav"] {
        find(&tree, name);
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// W11: the details SHA-256 is one middle-truncated line; hovering shows
/// the full digest and clicking copies it.
#[test]
fn details_sha256_is_truncated_with_full_hover_and_copy() {
    let (root, db) = catalog_root("sha");
    let mut shell = Shell::new(&db, 1440.0);
    shell.ctx.style_mut(|s| s.interaction.tooltip_delay = 0.0);
    let item = shell.app.tracks[0].clone();
    let hash: String = item.track.sha256.iter().map(|b| format!("{b:02x}")).collect();
    let short = view::truncate_middle(&hash, 12);
    assert_eq!(short.chars().count(), 25, "{short}");
    assert!(short.starts_with(&hash[..12]) && short.ends_with(&hash[52..]));
    shell.app.selected = Some(item.track.id.as_u64());
    shell.app.show_details = true;
    let out = shell.settle();
    let painted = texts(&out);
    assert!(!painted.iter().any(|(t, _)| t.contains(&hash)), "full hash not painted");
    let rect = painted.iter().find(|(t, _)| *t == short).expect("short hash painted").1;
    let details = shell.app.shell.details.unwrap();
    assert!(details.contains_rect(rect), "{rect:?} in {details:?}");
    // Even key/value rhythm: the badge and hash rows are no taller than
    // the plain rows above them (the first W11 build left both too tall).
    let top = |label: &str| painted.iter().find(|(t, r)| t == label && details.contains_rect(*r)).unwrap().1.top();
    let pitch = top("Size") - top("Bit depth");
    for (a, b) in [("Size", "Imported key"), ("Imported key", "Availability"), ("Availability", "SHA-256")] {
        assert!((top(b) - top(a) - pitch).abs() <= 1.0, "{a} -> {b}: {} vs {pitch}", top(b) - top(a));
    }
    let mut hovered = false;
    for step in 0..6 {
        let input = egui::RawInput {
            screen_rect: Some(shell.screen),
            time: Some(f64::from(step) * 0.5),
            events: vec![egui::Event::PointerMoved(rect.center())],
            ..Default::default()
        };
        let app = &mut shell.app;
        let out = shell.ctx.run(input, |ctx| app.ui(ctx));
        hovered |= texts(&out).iter().any(|(t, _)| t.contains(&hash));
    }
    assert!(hovered, "tooltip shows the full digest");
    let button = |pressed| egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let out = shell.frame(vec![button(true), button(false)], Default::default());
    let copied = out.platform_output.commands.iter().any(|c| {
        matches!(c, egui::OutputCommand::CopyText(t) if *t == hash)
    });
    assert!(copied, "click copies the full digest: {:?}", out.platform_output.commands);
    std::fs::remove_dir_all(root).unwrap();
}

/// W11: navigation counts are right-aligned, muted, in full-width rows.
#[test]
fn navigation_counts_are_right_aligned_in_full_width_rows() {
    let (root, db) = catalog_root("nav");
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let nav = shell.app.shell.nav.unwrap();
    let tree = nodes(&out);
    let rows: Vec<_> = ["All tracks · 3", "Available · 3", "Missing · 0"]
        .into_iter()
        .map(|l| find(&tree, l).2)
        .collect();
    let counts: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, r)| matches!(t.as_str(), "3" | "0") && nav.contains_rect(*r))
        .map(|(_, r)| r)
        .collect();
    assert_eq!(counts.len(), 3, "{counts:?}");
    for row in &rows {
        assert!(row.height() >= 24.0 && row.width() >= nav.width() - 17.0, "{row:?} {nav:?}");
    }
    let right = counts[0].right();
    assert!(counts.iter().all(|c| (c.right() - right).abs() <= 1.0), "{counts:?}");
    assert!(rows.iter().all(|r| (r.right() - 8.0 - right).abs() <= 1.5), "{rows:?} {right}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_ui_uses_cached_navigation_until_explicit_reload() {
    let (root, db) = catalog_root("navigation-cache");
    let audio = root.join("audio/tone-a.wav");
    let raw = audio.to_string_lossy().into_owned();
    let (location, _) = oxidj_application::location_uri(&raw);
    Catalog::open(&db).unwrap().import_library(&LibrarySnapshot {
        tracks: vec![LibraryTrack { title: "Cached title".into(), location, ..Default::default() }],
        roots: vec![oxidj_library::LibraryNode::Playlist { name: "Initial set".into(), members: vec![0, 0] }],
    }).unwrap();

    let mut shell = Shell::new(&db, 1440.0);
    shell.app.imported_playlist = Some(vec![0]);
    for _ in 0..4 {
        let output = shell.frame(vec![], Default::default());
        let shown = texts(&output);
        assert!(shown.iter().any(|(text, _)| text == "♫ Initial set"));
        assert!(shown.iter().any(|(text, _)| text == "1 · Cached title"));
        assert!(shown.iter().any(|(text, _)| text == "2 · Cached title"));
    }

    Catalog::open(&db).unwrap().create_library_playlist("Added set", &[0]).unwrap();
    let output = shell.settle();
    assert!(!texts(&output).iter().any(|(text, _)| text == "♫ Added set"), "frames must not reread the catalog");

    shell.app.reload();
    let output = shell.settle();
    assert!(texts(&output).iter().any(|(text, _)| text == "♫ Added set"), "explicit reload refreshes navigation");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn details_show_imported_cues_read_only_and_follow_selection() {
    let (root, db) = catalog_root("details-cues");
    let mut catalog = Catalog::open(&db).unwrap();
    let paths = ["tone-a.wav", "tone-b.wav", "tone-c.wav"].map(|name| root.join("audio").join(name));
    let tracks = paths.iter().enumerate().map(|(index, path)| {
        let raw = path.to_string_lossy().into_owned();
        let (uri, _) = oxidj_application::location_uri(&raw);
        LibraryTrack {
            title: format!("cue-track-{index}"), location: uri,
            key: match index { 0 => Some(" raw-Key ".into()), 2 => Some(String::new()), _ => None },
            cues: match index {
                0 => vec![
                    LibraryCue { hot_cue: Some(1), start_ms: 12_345, end_ms: None, name: "Drop".into(), colour: None },
                    LibraryCue { hot_cue: None, start_ms: 67_890, end_ms: Some(90_123), name: "Loop".into(), colour: None },
                ],
                2 => vec![LibraryCue { hot_cue: None, start_ms: 5_000, end_ms: None, name: String::new(), colour: None }],
                _ => vec![],
            },
            ..LibraryTrack::default()
        }
    }).collect();
    catalog.import_library(&LibrarySnapshot { tracks, ..Default::default() }).unwrap();
    drop(catalog);

    let mut shell = Shell::new(&db, 1440.0);
    shell.app.show_details = true;
    let id_a = shell.app.tracks[0].track.id.as_u64();
    let id_b = shell.app.tracks[1].track.id.as_u64();
    let id_c = shell.app.tracks[2].track.id.as_u64();
    let before = Catalog::open(&db).unwrap().library_snapshot().unwrap();
    shell.app.selected = Some(id_a);
    let output = shell.settle();
    let painted = texts(&output).into_iter().map(|(text, _)| text).collect::<Vec<_>>();
    for expected in ["Imported cues & loops", "Imported key", " raw-Key ", "Hot cue 1", "Memory cue", "0:12.345", "1:30.123", "Drop", "Loop"] {
        assert!(painted.iter().any(|text| text == expected), "missing {expected:?}: {painted:?}");
    }
    assert!(!painted.iter().any(|text| text == "8A"), "raw imported key was normalized: {painted:?}");
    assert_eq!(Catalog::open(&db).unwrap().library_snapshot().unwrap(), before);

    shell.app.selected = Some(id_b);
    let output = shell.settle();
    let painted = texts(&output);
    let key_label_y = painted.iter().find(|(text, _)| text == "Imported key").unwrap().1.center().y;
    assert!(painted.iter().any(|(text, rect)| text == "—" && (rect.center().y - key_label_y).abs() < 1.0), "absent imported key should be displayed distinctly: {painted:?}");
    assert!(!painted.iter().any(|(text, _)| text == " raw-Key " || text == "Drop" || text == "Loop" || text == "Hot cue 1"), "stale details: {painted:?}");

    shell.app.selected = Some(id_c);
    let output = shell.settle();
    let painted = texts(&output);
    let key_label_y = painted.iter().find(|(text, _)| text == "Imported key").unwrap().1.center().y;
    assert!(painted.iter().any(|(text, rect)| text.is_empty() && (rect.center().y - key_label_y).abs() < 1.0), "present-empty key should remain an empty value: {painted:?}");
    assert!(painted.iter().any(|(text, _)| text == "Memory cue"));
    assert!(painted.iter().any(|(text, _)| text == "0:05.000"));
    assert_eq!(Catalog::open(&db).unwrap().library_snapshot().unwrap(), before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_nested_playlist_loads_ordered_members_to_both_decks_and_refuses_missing_audio() {
    let (root, db) = catalog_root("playlist-load");
    let paths = ["tone-a.wav", "tone-b.wav", "tone-c.wav"].map(|n| root.join("audio").join(n));
    let mut tracks: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let raw = p.to_string_lossy().into_owned();
            let (location, _) = oxidj_application::location_uri(&raw);
            LibraryTrack {
                title: format!("member-{i}"),
                location,
                ..LibraryTrack::default()
            }
        })
        .collect();
    // A matching local filename/title must not substitute for an absent imported location.
    tracks.push(LibraryTrack {
        title: "tone-c.wav".into(),
        ..Default::default()
    });
    let before = LibrarySnapshot {
        tracks,
        roots: vec![oxidj_library::LibraryNode::Folder {
            name: "Nested".into(),
            children: vec![oxidj_library::LibraryNode::Playlist {
                name: "Set".into(),
                members: vec![2, 0, 2, 3],
            }],
        }],
    };
    Catalog::open(&db).unwrap().import_library(&before).unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let click_text = |shell: &mut Shell, needle: &str| {
        let out = shell.settle();
        let (_, rect) = texts(&out)
            .into_iter()
            .find(|(s, _)| s.contains(needle))
            .unwrap_or_else(|| panic!("visible UI text {needle:?} not found"));
        shell.click(rect.center());
    };
    click_text(&mut shell, "♫ Set");
    let shown = shell.settle();
    let member_texts: Vec<_> = texts(&shown)
        .into_iter()
        .map(|(s, _)| s)
        .filter(|s| s.contains("member-"))
        .collect();
    assert_eq!(
        member_texts,
        ["1 · member-2", "2 · member-0", "3 · member-2"]
    );
    click_text(&mut shell, "1 · member-2");
    assert!(
        shell.app.decks.decks.iter().all(|d| d.track.is_none()),
        "selection alone never loads"
    );
    click_text(&mut shell, "Load A");
    assert_eq!(
        shell.app.decks.decks[0].track.as_ref().unwrap().path,
        paths[2]
    );
    assert!(!shell.app.decks.decks[0].playing);
    assert!(shell.app.decks.decks[1].track.is_none());
    shell.app.decks.decks[0].cue_ms = 125.0;
    shell.app.decks.decks[0].position_ms = 250.0;
    shell.app.decks.decks[0].loop_range = Some((125.0, 375.0));
    click_text(&mut shell, "2 · member-0");
    click_text(&mut shell, "Load B");
    assert_eq!(
        shell.app.decks.decks[1].track.as_ref().unwrap().path,
        paths[0]
    );
    click_text(&mut shell, "3 · member-2");
    click_text(&mut shell, "Load B");
    assert_eq!(
        shell.app.decks.decks[1].track.as_ref().unwrap().path,
        paths[2]
    );
    assert!(!shell.app.decks.decks[1].playing);
    let before_ids = [
        shell.app.decks.decks[0].track.as_ref().map(|t| t.id),
        shell.app.decks.decks[1].track.as_ref().map(|t| t.id),
    ];
    click_text(&mut shell, "4 · tone-c.wav");
    click_text(&mut shell, "Load A");
    assert!(shell.app.notice.starts_with("Playlist load refused:"));
    assert_eq!(
        shell.app.decks.loaded(),
        before_ids,
        "no title/filename fallback"
    );
    // Removing a referenced local file makes location resolution refuse without replacing the loaded target.
    std::fs::remove_file(&paths[0]).unwrap();
    click_text(&mut shell, "2 · member-0");
    click_text(&mut shell, "Load A");
    assert!(shell.app.notice.starts_with("Playlist load refused:"));
    assert!(shell.app.decks.decks.iter().all(|d| !d.playing));
    assert_eq!(
        [
            shell.app.decks.decks[0].track.as_ref().map(|t| t.id),
            shell.app.decks.decks[1].track.as_ref().map(|t| t.id)
        ],
        before_ids
    );
    assert_eq!(shell.app.decks.decks[0].cue_ms, 125.0);
    assert_eq!(shell.app.decks.decks[0].position_ms, 250.0);
    assert_eq!(shell.app.decks.decks[0].loop_range, Some((125.0, 375.0)));
    assert_eq!(
        Catalog::open(&db).unwrap().library_snapshot().unwrap(),
        before
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_playlist_members_scroll_to_last_member_and_load_it() {
    let (root, _) = catalog_root("playlist-scroll");
    // Start without scanned audio references: loading must resolve the imported location.
    let db = root.join("imported-only.sqlite");
    let path = root.join("audio/tone-b.wav");
    let (location, _) = oxidj_application::location_uri(path.to_str().unwrap());
    Catalog::open(&db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            tracks: vec![LibraryTrack {
                title: "scroll-member".into(),
                location,
                ..Default::default()
            }],
            roots: vec![oxidj_library::LibraryNode::Playlist {
                name: "Long".into(),
                members: vec![0; 80],
            }],
        })
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let rect = texts(&out)
        .into_iter()
        .find(|(s, _)| s == "♫ Long")
        .unwrap()
        .1;
    shell.click(rect.center());
    let pos = shell.app.shell.nav.unwrap().center();
    for _ in 0..30 {
        shell.frame(
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -200.0),
                    modifiers: Default::default(),
                },
            ],
            Default::default(),
        );
    }
    let out = shell.settle();
    let rect = texts(&out)
        .into_iter()
        .find(|(s, r)| {
            s == "80 · scroll-member" && shell.app.shell.nav.unwrap().contains(r.center())
        })
        .expect("last member must be visible after scrolling")
        .1;
    shell.click(rect.center());
    // Selection inserts the load controls below the last row; scroll them into view too.
    for _ in 0..10 {
        shell.frame(
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -100.0),
                    modifiers: Default::default(),
                },
            ],
            Default::default(),
        );
    }
    let out = shell.settle();
    let rect = texts(&out)
        .into_iter()
        .find(|(s, r)| s == "Load B" && shell.app.shell.nav.unwrap().contains(r.center()))
        .unwrap()
        .1;
    shell.click(rect.center());
    assert_eq!(shell.app.decks.decks[1].track.as_ref().unwrap().path, path);
    assert!(shell.app.decks.decks[0].track.is_none());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !shell.app.decks.decks[1].ready() {
        assert!(std::time::Instant::now() < deadline, "imported audio must decode");
        shell.app.decks.render(512);
        shell.settle();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let audio = shell.app.decks.render(512);
    shell.settle();
    assert!(audio.iter().all(|sample| *sample == 0.0), "loading never autoplays on the real engine's fake sink");
    assert!(!shell.app.decks.decks[1].playing);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_same_named_folders_collapse_independently() {
    let (root, db) = catalog_root("playlist-folder-ids");
    Catalog::open(&db)
        .unwrap()
        .import_library(&LibrarySnapshot {
            roots: ["First", "Second"]
                .map(|name| oxidj_library::LibraryNode::Folder {
                    name: "Same".into(),
                    children: vec![oxidj_library::LibraryNode::Playlist {
                        name: name.into(),
                        members: vec![],
                    }],
                })
                .to_vec(),
            ..Default::default()
        })
        .unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let rect = texts(&out)
        .into_iter()
        .find(|(s, _)| s == "📁 Same")
        .unwrap()
        .1;
    shell.click(rect.center());
    // Complete the collapse animation without wall-clock sleeps.
    for i in 0..30 {
        let _ = shell.ctx.run(
            egui::RawInput {
                time: Some(10.0 + i as f64),
                screen_rect: Some(shell.screen),
                ..Default::default()
            },
            |ctx| shell.app.ui(ctx),
        );
    }
    let out = shell.settle();
    let labels: Vec<_> = texts(&out).into_iter().map(|(s, _)| s).collect();
    assert!(!labels.iter().any(|s| s == "♫ First"));
    assert!(
        labels.iter().any(|s| s == "♫ Second"),
        "collapsing first folder must not collapse its same-named sibling"
    );
    let rect = texts(&out)
        .into_iter()
        .find(|(s, _)| s == "♫ Second")
        .unwrap()
        .1;
    shell.click(rect.center());
    assert_eq!(
        shell.app.imported_playlist,
        Some(vec![1, 0]),
        "identical membership does not identify a playlist"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_playlist_read_failure_is_not_reported_as_empty() {
    let (root, _) = catalog_root("playlist-read-error");
    let db = root.join("invalid.sqlite");
    std::fs::write(&db, b"not a catalog").unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let out = shell.settle();
    let labels = texts(&out);
    assert!(
        labels
            .iter()
            .any(|(s, _)| s.starts_with("Imported playlists unavailable:"))
    );
    assert!(!labels.iter().any(|(s, _)| s == "No imported playlists"));
    std::fs::remove_dir_all(root).unwrap();
}
