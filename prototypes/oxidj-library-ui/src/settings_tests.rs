//! M8/D0: the Settings window through the production `LibraryApp::ui`
//! path: Ctrl+, / gear / Esc, focus return, keyboard reach, AccessKit
//! names, and every wired option changing the next rendered frame.
//! Synthetic audio only.

use super::*;
use crate::settings::{Settings, WINDOW_TITLE};
use crate::waveform_view::Style;
use eframe::egui::{Pos2, Rect, Shape, accesskit};
use oxidj_library::{LibraryBeat, LibrarySnapshot, LibraryTrack};
use std::path::{Path, PathBuf};

type Frame = egui::FullOutput;

#[test]
fn output_device_selection_persists_and_combines_with_environment() {
    let root = temp_root("device-settings");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    h.app.settings_ui.outputs = vec![oxidj_audio::OutputInfo {
        host: "Host;=".into(), device: "Synthetic".into(), id: "id;=雪".into(),
        default: false, max_channels: 2, rates: (44100, 48000), buffer_frames: None,
    }];
    h.ctrl_comma();
    let out = h.click(h.layout().tabs.unwrap()[2].center());
    let cell = texts(&out).into_iter().find(|(text, _)| text.contains("Synthetic")).unwrap();
    h.click(cell.1.center());
    let mut storage = MemoryStorage::default();
    eframe::App::save(&mut h.app, &mut storage);
    let settings = Settings::load(Some(&storage));
    let env = oxidj_audio::OutputConfig { host: Some("env".into()), device: Some("env-id".into()), sample_rate: Some(44100), buffer_frames: Some(128), ..Default::default() };
    let configured = settings.output_config(env.clone());
    assert_eq!(configured.host.as_deref(), Some("Host;="));
    assert_eq!(configured.device.as_deref(), Some("id;=雪"));
    assert_eq!(configured.sample_rate, env.sample_rate);
    assert_eq!(configured.buffer_frames, env.buffer_frames);
    assert_eq!(Settings::default().output_config(env.clone()), env);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn output_device_empty_list_keeps_saved_choice_visible() {
    let root = temp_root("device-empty");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    h.app.settings.audio_device = Some(("Missing".into(), "id".into()));
    h.ctrl_comma();
    let out = h.click(h.layout().tabs.unwrap()[2].center());
    let labels = texts(&out);
    assert!(labels.iter().any(|(s, _)| s.contains("No output devices found")));
    assert!(labels.iter().any(|(s, _)| s.contains("Saved output not found in last enumeration")));
    assert_eq!(h.app.settings.audio_device.as_ref().unwrap().0, "Missing");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn buffer_choice_is_selectable_and_persisted_in_production_settings() {
    let root = temp_root("buffer-settings");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    h.ctrl_comma();
    let out = h.click(h.layout().tabs.unwrap()[2].center());
    let cell = texts(&out).into_iter().find(|(text, _)| text == "256 frames");
    assert!(cell.is_some(), "buffer selection is missing");
    h.click(cell.unwrap().1.center());
    let mut storage = MemoryStorage::default();
    eframe::App::save(&mut h.app, &mut storage);
    assert_eq!(Settings::load(Some(&storage)).audio_buffer, Some(Some(256)));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn master_limiter_toggle_persists_through_the_settings_ui() {
    let root = temp_root("limiter-settings");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    assert!(h.app.settings.limiter);
    h.ctrl_comma();
    h.click(h.layout().tabs.unwrap()[2].center());
    h.click(h.layout().limiter.unwrap().center());
    assert!(!h.app.settings.limiter);
    let mut storage = MemoryStorage::default();
    eframe::App::save(&mut h.app, &mut storage);
    assert!(!Settings::load(Some(&storage)).limiter);
    assert!(Settings::decode("").limiter);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn master_limiter_toggle_changes_rendered_audio() {
    use oxidj_audio::{Command, TrackPcm};
    use std::sync::Arc;
    let root = temp_root("limiter-audio");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    let samples = (0..96_000).flat_map(|i| {
        let v = (i as f32 * 1_000.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.8;
        [v, v]
    }).collect();
    let track = Arc::new(TrackPcm::new(48_000, samples).unwrap());
    for deck in 0..2 {
        assert!(h.app.decks.engine.send(Command::Load { deck, id: deck as u64, track: track.clone() }));
        assert!(h.app.decks.engine.send(Command::TogglePlay { deck }));
    }
    h.ctrl_comma();
    h.click(h.layout().tabs.unwrap()[2].center());
    h.click(h.layout().limiter.unwrap().center());
    let off = h.app.decks.render(9_600);
    h.click(h.layout().limiter.unwrap().center());
    let on = h.app.decks.render(9_600);
    let flat = |samples: &[f32]| samples[9_600..].iter().filter(|v| v.abs() >= 0.9999).count();
    assert!(flat(&off) > flat(&on) * 2, "off={} on={}", flat(&off), flat(&on));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn settings_tabs_show_only_the_selected_group() {
    let root = temp_root("tabs");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    let out = h.ctrl_comma();
    let labels: Vec<_> = texts(&out).into_iter().map(|(text, _)| text).collect();
    assert!(labels.iter().any(|s| s == "Theme"));
    assert!(!labels.iter().any(|s| s == "Smooth edges"), "waveform controls belong on their own tab");
    let out = h.click(h.layout().tabs.unwrap()[2].center());
    assert!(texts(&out).iter().any(|(s, _)| s == "Crossfader curve"));
    assert!(h.layout().theme.is_none());
    let out = h.click(h.layout().tabs.unwrap()[3].center());
    assert!(texts(&out).iter().any(|(s, _)| s == "Ctrl+,"));
    assert!(h.layout().curve.is_none());
}

#[test]
fn every_theme_is_selectable_persisted_and_truthfully_exposed_in_settings() {
    let root = temp_root("all-presets");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    h.app.custom_tokens.primary = egui::Color32::from_rgb(12, 34, 56);
    let custom = h.app.custom_tokens;
    h.app.settings.smooth = false;
    let settings = h.app.settings.clone();
    h.ctrl_comma();
    for (i, mode) in ThemeMode::ALL.into_iter().enumerate() {
        let cell = h.layout().theme.unwrap()[i];
        assert!(h.layout().window.unwrap().contains_rect(cell));
        let out = h.click(cell.center());
        assert_eq!(h.app.theme, mode, "{} pointer selection", mode.label());
        assert_eq!(h.app.custom_tokens, custom);
        assert_eq!(h.app.settings, settings);
        let tokens = if mode == ThemeMode::Custom {
            custom
        } else {
            mode.tokens()
        };
        assert_eq!(theme::current_tokens(&h.ctx), tokens);
        assert_eq!(h.ctx.style().visuals.panel_fill, tokens.background);
        let nodes = accesskit_nodes(&out);
        for candidate in ThemeMode::ALL {
            let node = nodes
                .iter()
                .find(|n| n.label().as_deref() == Some(candidate.label()))
                .expect("preset accessible by name");
            assert_eq!(
                node.toggled(),
                Some(if candidate == mode {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                }),
                "{} selected state",
                candidate.label()
            );
        }
        let mut storage = MemoryStorage::default();
        eframe::App::save(&mut h.app, &mut storage);
        let (restored, saved_custom) = ThemeMode::load(Some(&storage));
        assert_eq!((restored, saved_custom), (mode, custom));
        assert_eq!(Settings::load(Some(&storage)), settings);
        let reopened = LibraryApp::new(root.join("catalog.sqlite"), restored, saved_custom);
        assert_eq!(reopened.theme, mode);
        assert_eq!(reopened.custom_tokens, custom);
        if mode == ThemeMode::Custom {
            // Exercise the existing Esc/Settings shortcut path after opening Custom.
            h.key(egui::Key::Escape, Default::default());
            if !h.app.settings_ui.open {
                h.ctrl_comma();
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn selecting_custom_in_settings_preserves_saved_palette() {
    let root = temp_root("saved-palette");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    h.app.custom_tokens.primary = egui::Color32::from_rgb(12, 34, 56);
    let saved = h.app.custom_tokens;
    h.ctrl_comma();
    h.click(h.layout().theme.unwrap()[1].center());
    h.click(h.layout().theme.unwrap()[2].center());
    assert_eq!(h.app.theme, ThemeMode::Custom);
    assert_eq!(h.app.custom_tokens, saved);
    assert!(h.app.show_theme_editor);
    let mut storage = MemoryStorage::default();
    eframe::App::save(&mut h.app, &mut storage);
    assert_eq!(ThemeMode::load(Some(&storage)), (ThemeMode::Custom, saved));
}

#[test]
fn custom_palette_editor_is_available_from_appearance() {
    let root = temp_root("edit-palette");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    let toolbar = h.settle();
    assert!(!texts(&toolbar).iter().any(|(s, _)| s == "Theme"), "toolbar has no theme picker");
    let out = h.ctrl_comma();
    h.click(text_rect(&out, "Edit custom palette…").center());
    assert_eq!(h.app.theme, ThemeMode::Custom);
    assert!(h.app.show_theme_editor);
}

/// 16-bit mono PCM WAV: a click every 500 ms (120 BPM) for `seconds`.
fn write_click_wav(path: &Path, seconds: f32) {
    let rate = 44_100u32;
    let n = (rate as f32 * seconds) as u32;
    let mut bytes = Vec::new();
    for chunk in [&b"RIFF"[..], &(36 + n * 2).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes()] {
        bytes.extend_from_slice(chunk);
    }
    for v in [1u16, 1] {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(n * 2).to_le_bytes());
    for i in 0..n {
        let t = i as f32 / rate as f32;
        let phase = (t * 2.0).fract();
        let v = if phase < 0.02 { (t * 3_000.0 * std::f32::consts::TAU).sin() * 0.9 } else { 0.05 * (t * 200.0 * std::f32::consts::TAU).sin() };
        bytes.extend_from_slice(&((v * 32_000.0) as i16).to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-d0-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(root.join("audio")).unwrap();
    root
}

/// One 20 s analysed track with an imported 120 BPM grid (10 bars).
fn grid_catalog() -> PathBuf {
    let root = temp_root("grid");
    let audio = root.join("audio");
    write_click_wav(&audio.join("d0-grid.wav"), 20.0);
    let db = root.join("catalog.sqlite");
    let mut catalog = Catalog::open(&db).unwrap();
    catalog.import_audio_directory(audio.to_str().unwrap()).unwrap();
    let track = catalog.list_track_details().unwrap().remove(0).track;
    oxidj_application::analyze_track_waveform(&mut catalog, track.id, false).unwrap();
    let (location, _) = oxidj_application::location_uri(&track.path);
    catalog
        .import_library(&LibrarySnapshot {
            tracks: vec![LibraryTrack {
                title: "synthetic grid".into(),
                location,
                beat_grid: (0..40u32)
                    .map(|i| LibraryBeat { time_ms: 500 * i, tempo_hundredths_bpm: 12_000, beat_in_bar: (i % 4 + 1) as u8 })
                    .collect(),
                ..LibraryTrack::default()
            }],
            roots: vec![],
        })
        .unwrap();
    db
}

struct Harness {
    ctx: egui::Context,
    app: LibraryApp,
    viewport: egui::Vec2,
}

impl Harness {
    fn new(db: PathBuf) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = LibraryApp::new(db, ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens());
        let mut h = Self { ctx, app, viewport: egui::vec2(1440.0, 900.0) };
        h.settle();
        h
    }

    fn frame_with(&mut self, events: Vec<egui::Event>, modifiers: egui::Modifiers) -> Frame {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.viewport)),
            events,
            modifiers,
            ..Default::default()
        };
        let app = &mut self.app;
        self.ctx.run(input, |ctx| app.ui(ctx))
    }

    fn settle(&mut self) -> Frame {
        self.frame_with(vec![], Default::default());
        self.frame_with(vec![], Default::default())
    }

    fn click(&mut self, pos: Pos2) -> Frame {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        self.frame_with(vec![egui::Event::PointerMoved(pos), button(true)], Default::default());
        self.frame_with(vec![button(false)], Default::default());
        self.settle()
    }

    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> Frame {
        let event = |pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
        self.frame_with(vec![event(true)], modifiers);
        self.frame_with(vec![event(false)], Default::default());
        self.settle()
    }

    /// Ctrl+, as winit reports it on Linux (ctrl and command both set).
    fn ctrl_comma(&mut self) -> Frame {
        self.key(egui::Key::Comma, egui::Modifiers { ctrl: true, command: true, ..Default::default() })
    }

    fn focused(&self) -> Option<egui::Id> {
        self.ctx.memory(|m| m.focused())
    }

    fn layout(&self) -> crate::settings::SettingsLayout {
        self.app.settings_ui.layout
    }
}

fn texts(out: &Frame) -> Vec<(String, Rect)> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Text(t) => Some((t.galley.job.text.clone(), t.visual_bounding_rect())),
            _ => None,
        })
        .collect()
}

fn text_rect(out: &Frame, label: &str) -> Rect {
    let hits: Vec<_> = texts(out).into_iter().filter(|(t, _)| t == label).collect();
    assert_eq!(hits.len(), 1, "{label:?}: {hits:?}");
    hits[0].1
}

fn waveform_vertices(out: &Frame, view: Rect) -> Vec<(Pos2, egui::Color32)> {
    let mut v = Vec::new();
    for clipped in &out.shapes {
        if let Shape::Mesh(mesh) = &clipped.shape {
            if view.expand(0.5).contains_rect(mesh.calc_bounds()) {
                v.extend(mesh.vertices.iter().map(|x| (x.pos, x.color)));
            }
        }
    }
    v
}

fn bar_labels_in(out: &Frame, view: Rect) -> usize {
    texts(out).iter().filter(|(t, r)| t.starts_with("Bar ") && view.contains_rect(*r)).count()
}

fn accesskit_nodes(out: &Frame) -> Vec<accesskit::Node> {
    out.platform_output.accesskit_update.as_ref().expect("accesskit enabled").nodes.iter().map(|(_, n)| n.clone()).collect()
}

#[test]
fn ctrl_comma_and_gear_toggle_settings_esc_closes_and_focus_returns() {
    let root = temp_root("shell");
    let mut h = Harness::new(root.join("catalog.sqlite"));
    assert!(!h.app.settings_ui.open);
    assert!(h.layout().window.is_none());

    // Focus the search field, as a user typing a query would.
    let search = h.app.shell.search.expect("search field");
    h.click(search.center());
    let before = h.focused().expect("search focused");

    // Ctrl+, opens the window and moves focus to its first control.
    let out = h.ctrl_comma();
    assert!(h.app.settings_ui.open);
    let window = h.layout().window.expect("settings window painted");
    let first = h.layout().first_control.expect("first control");
    assert_eq!(h.focused(), Some(first), "focus moves into the window");
    assert!(h.layout().tabs.unwrap().iter().all(|r| window.contains_rect(*r)));

    // AccessKit: titled window, labelled switches with their checked state.
    let nodes = accesskit_nodes(&out);
    let named = |name: &str| nodes.iter().find(|n| n.label().as_deref() == Some(name)).cloned();
    assert!(named(WINDOW_TITLE).is_some(), "window title exposed");

    assert!(
        nodes.iter().any(|n| n.label().as_deref() == Some("Tokyo Night") && n.toggled() == Some(accesskit::Toggled::True)),
        "selected theme cell exposed"
    );

    // Keyboard reachable: Tab moves focus to the next control in the window,
    // and Space on a focused switch flips it.
    h.key(egui::Key::Tab, Default::default());
    let next = h.focused().expect("tab focus");
    assert_ne!(next, first);
    h.key(egui::Key::Space, Default::default());
    assert!(h.layout().smooth.is_some(), "keyboard activates Waveform tab");
    let mut guard = 0;
    while h.layout().smooth.is_some_and(|r| {
        h.ctx.memory(|m| m.focused()).and_then(|id| h.ctx.read_response(id)).map(|r| r.rect) != Some(r)
    }) {
        h.key(egui::Key::Tab, Default::default());
        guard += 1;
        assert!(guard < 12, "Smooth switch reachable by Tab");
    }
    h.key(egui::Key::Space, Default::default());
    assert!(!h.app.settings.smooth, "Space flips the focused switch");

    // Esc closes and returns focus to the search field.
    h.key(egui::Key::Escape, Default::default());
    assert!(!h.app.settings_ui.open);
    assert!(h.layout().window.is_none());
    assert_eq!(h.focused(), Some(before), "focus returned");

    // Ctrl+, toggles open and closed again.
    h.ctrl_comma();
    assert!(h.app.settings_ui.open);
    h.ctrl_comma();
    assert!(!h.app.settings_ui.open);

    // Toolbar gear (AccessKit name "Settings") opens it too.
    let gear = h.app.shell.settings.expect("gear button");
    let out = h.click(gear.center());
    assert!(h.app.settings_ui.open);
    assert!(
        accesskit_nodes(&out).iter().any(|n| n.role() == accesskit::Role::Button && n.label().as_deref() == Some("Settings")),
        "gear named for AccessKit"
    );
    // Esc does nothing to the app when Settings is already closed.
    h.key(egui::Key::Escape, Default::default());
    h.key(egui::Key::Escape, Default::default());
    assert!(!h.app.settings_ui.open);
}

#[test]
fn each_wired_option_changes_the_next_rendered_frame_and_persists() {
    let mut h = Harness::new(grid_catalog());
    h.app.show_details = true;
    h.app.show_decks = false;
    let out = h.settle();
    let out = h.click(text_rect(&out, "d0-grid.wav").center());
    let view = h.app.track_waveform.layout.view.expect("detail view");
    let base = waveform_vertices(&out, view);
    assert!(!base.is_empty());
    assert!(bar_labels_in(&out, view) > 0, "bar lines on by default");
    let (window, _) = h.app.track_waveform.layout.grid_window.expect("imported grid");
    assert_eq!(window.1 - window.0, 8_000.0, "default 4 bars at 120 BPM");

    h.ctrl_comma();
    h.click(h.layout().tabs.unwrap()[1].center());

    // Waveform style: RGB.
    let out = h.click(h.layout().style.unwrap()[2].center());
    assert_eq!(h.app.settings.waveform_style, Style::Rgb);
    assert_eq!(h.app.track_waveform.style, Style::Rgb);
    let with_lines = waveform_vertices(&out, view);
    assert_ne!(with_lines, base, "style changes the painted waveform");

    // Beat and bar lines off: no labels and no line mesh (only the waveform's).
    let out = h.click(h.layout().beat_lines.unwrap().center());
    assert!(!h.app.settings.beat_lines);
    assert_eq!(bar_labels_in(&out, view), 0, "no bar labels");
    let rgb = waveform_vertices(&out, view);
    assert!(rgb.len() < with_lines.len(), "line mesh gone");

    // Smooth off: same style, hard edges (fringe vertices collapse onto the body).
    let out = h.click(h.layout().smooth.unwrap().center());
    assert!(!h.app.settings.smooth);
    let hard = waveform_vertices(&out, view);
    assert_eq!(hard.len(), rgb.len());
    assert_ne!(hard, rgb, "smooth changes the painted edges");
    for quad in hard.chunks(4) {
        assert_eq!(quad[0].0, quad[1].0, "top fringe on the body edge");
        assert_eq!(quad[3].0, quad[2].0, "bottom fringe on the body edge");
    }


    // Default zoom 8 bars: the detail window spans 8 bars (16 s).
    let out = h.click(h.layout().zoom.unwrap()[3].center());
    assert_eq!(h.app.settings.zoom_bars, 8);
    let (window, bar) = h.app.track_waveform.layout.grid_window.unwrap();
    assert_eq!(window.1 - window.0, 16_000.0);
    assert!(texts(&out).iter().any(|(t, _)| t.starts_with(&format!("Detail 8 bars from Bar {}", bar + 1))));

    // Theme: Tokyo Day repaints the waveform plate in Day background.
    let day = ThemeMode::TokyoDay.tokens();
    h.click(h.layout().tabs.unwrap()[0].center());
    let out = h.click(h.layout().theme.unwrap()[1].center());
    assert_eq!(h.app.theme, ThemeMode::TokyoDay);
    assert!(
        out.shapes.iter().any(|c| matches!(&c.shape, Shape::Rect(r) if r.fill == day.background && r.rect == view)),
        "plate painted in the new theme"
    );

    // Persisted through the production eframe save path.
    let mut storage = MemoryStorage::default();
    eframe::App::save(&mut h.app, &mut storage);
    let saved = Settings::load(Some(&storage));
    assert_eq!(saved, h.app.settings);
    assert_eq!((saved.waveform_style, saved.smooth, saved.beat_lines, saved.zoom_bars), (Style::Rgb, false, false, 8));
    assert_eq!(ThemeMode::load(Some(&storage)).0, ThemeMode::TokyoDay);

    // Reset to defaults restores everything, live.
    let reset = h.layout().reset.expect("reset button");
    assert!(h.layout().window.unwrap().contains_rect(reset), "Reset visible in the window");
    h.click(reset.center());
    assert_eq!(h.app.settings, Settings::default());
    assert_eq!(h.app.theme, ThemeMode::TokyoNight);
    let out = h.settle();
    assert!(bar_labels_in(&out, view) > 0, "lines back");
    assert_eq!(waveform_vertices(&out, view), base, "default rendering restored");
}

#[test]
fn small_viewport_settings_controls_are_reachable_by_keyboard_and_pointer() {
    for viewport in [egui::vec2(800.0, 600.0), egui::vec2(640.0, 480.0)] {
        let root = temp_root("small-settings");
        let mut h = Harness::new(root.join("catalog.sqlite"));
        h.viewport = viewport;
        h.settle();
        h.ctrl_comma();
        let screen = Rect::from_min_size(Pos2::ZERO, viewport);
        assert!(screen.contains_rect(h.layout().window.unwrap()), "window bounded at {viewport:?}: {:?}", h.layout().window);
        let day = h.layout().theme.unwrap()[1];
        assert!(screen.contains_rect(day), "first row visible");
        h.click(day.center());
        assert_eq!(h.app.theme, ThemeMode::TokyoDay, "first row activates");

        // Start at the first control and traverse actual focus order, not a
        // direct focus request to Reset. Every focused control must scroll
        // into view, including the last button below the shortcut list.
        h.ctx.memory_mut(|m| m.request_focus(h.layout().first_control.unwrap()));
        h.settle();
        let mut reached_reset = false;
        for _ in 0..40 {
            h.key(egui::Key::Tab, Default::default());
            // Allow egui's focus-scroll animation to settle.
            for _ in 0..30 {
                h.settle();
            }
            let focused = h.focused().and_then(|id| h.ctx.read_response(id)).expect("focused control");
            let visible = focused.rect.intersect(focused.interact_rect);
            assert!(screen.contains_rect(focused.rect), "focused control on screen: {:?}", focused.rect);
            assert!(visible.contains_rect(focused.rect), "focused control not clipped");
            if focused.rect == h.layout().reset.unwrap() {
                reached_reset = true;
                h.key(egui::Key::Space, Default::default());
                break;
            }
        }
        assert!(reached_reset, "Reset reachable by Tab at {viewport:?}");
        assert_eq!(h.app.theme, ThemeMode::TokyoNight, "keyboard Reset activates");
        // Reset is also pointer-operable after scrolling into view.
        h.app.settings.smooth = false;
        h.click(h.layout().reset.unwrap().center());
        assert_eq!(h.app.settings, Settings::default());
        h.key(egui::Key::Escape, Default::default());
        assert!(!h.app.settings_ui.open);
    }
}

#[derive(Default)]
struct MemoryStorage(std::collections::HashMap<String, String>);

impl eframe::Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn flush(&mut self) {}
}
