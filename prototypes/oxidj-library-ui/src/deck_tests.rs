//! M8/D1: the deck area through the production `LibraryApp::ui` path
//! (plus the eframe raw-input hook): load, seek, zoom, merged geometry,
//! overview separation, grid fallback, palette, pads, transport keys and
//! every collapse combination with persistence. Synthetic audio only.

use super::*;
use crate::decks::{self, DeckColours, NOT_READY, NO_AUDIO};
use crate::settings::{DeckPalette, DeckSections, Settings};
use eframe::egui::{Pos2, Rect, Shape, accesskit};
use oxidj_library::{LibraryBeat, LibraryCue, LibrarySnapshot, LibraryTrack};
use std::path::{Path, PathBuf};

type Frame = egui::FullOutput;

/// 16-bit mono PCM WAV: kick on every beat at `bpm`, off-beat hat, quiet
/// bass, scaled by `level` (a quiet master exercises auto-gain).
pub(crate) fn write_loop_wav(path: &Path, seconds: f32, bpm: f32, level: f32) {
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
    let tau = std::f32::consts::TAU;
    for i in 0..n {
        let t = i as f32 / rate as f32;
        let beat = t * bpm / 60.0;
        let ph = beat.fract();
        let phrase = 0.6 + 0.4 * (beat / 16.0 * tau).sin().abs();
        let kick = (-ph * 18.0).exp() * (t * tau * (50.0 + 90.0 * (-ph * 30.0).exp())).sin();
        let hat = if (ph - 0.5).abs() < 0.04 { ((u64::from(i) * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5 } else { 0.0 };
        let bass = 0.25 * phrase * (t * tau * 55.0).sin();
        let v = (0.8 * kick + hat + bass).clamp(-1.0, 1.0) * level;
        bytes.extend_from_slice(&((v * 32_000.0) as i16).to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "oxidj-ui-d1-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(root.join("audio")).unwrap();
    root
}

fn grid(bpm: f64, seconds: f64, first_ms: f64) -> Vec<LibraryBeat> {
    let beat = 60_000.0 / bpm;
    (0..)
        .map(|i| (i, first_ms + i as f64 * beat))
        .take_while(|(_, t)| *t < seconds * 1000.0)
        .map(|(i, t)| LibraryBeat {
            time_ms: t.round() as u32,
            tempo_hundredths_bpm: (bpm * 100.0).round() as u32,
            beat_in_bar: (i % 4 + 1) as u8,
        })
        .collect()
}

fn hot(n: u32, ms: u32, end: Option<u32>, colour: Option<[u8; 3]>) -> LibraryCue {
    LibraryCue { hot_cue: Some(n), start_ms: ms, end_ms: end, name: String::new(), colour }
}

fn memory(ms: u32) -> LibraryCue {
    LibraryCue { hot_cue: None, start_ms: ms, end_ms: None, name: String::new(), colour: None }
}

/// Synthetic catalog: `a-120` (imported 120 BPM grid, hot cues 1/2/3-loop,
/// two memory cues), `b-124` (imported 124 BPM grid, hot cue 1), `c-nogrid`
/// (analysed, no grid, quiet master) and `d-pending` (not analysed).
pub(crate) fn deck_catalog(root: &Path, seconds: f32) -> PathBuf {
    let audio = root.join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    write_loop_wav(&audio.join("a-120.wav"), seconds, 120.0, 0.9);
    write_loop_wav(&audio.join("b-124.wav"), seconds, 124.0, 0.9);
    write_loop_wav(&audio.join("c-nogrid.wav"), seconds, 126.0, 0.3);
    write_loop_wav(&audio.join("d-pending.wav"), seconds, 128.0, 0.9);
    let db = root.join("catalog.sqlite");
    let mut catalog = Catalog::open(&db).unwrap();
    catalog.import_audio_directory(audio.to_str().unwrap()).unwrap();
    let tracks = catalog.list_track_details().unwrap();
    let by = |name: &str| tracks.iter().find(|t| t.track.path.ends_with(name)).unwrap().track.clone();
    for name in ["a-120.wav", "b-124.wav", "c-nogrid.wav"] {
        oxidj_application::analyze_track_waveform(&mut catalog, by(name).id).unwrap();
    }
    let s = f64::from(seconds);
    let lib = |name: &str, beat_grid: Vec<LibraryBeat>, cues: Vec<LibraryCue>| LibraryTrack {
        title: name.into(),
        location: oxidj_application::location_uri(&by(name).path).0,
        beat_grid,
        cues,
        ..LibraryTrack::default()
    };
    catalog
        .import_library(&LibrarySnapshot {
            tracks: vec![
                lib(
                    "a-120.wav",
                    grid(120.0, s, 0.0),
                    vec![
                        memory(0),
                        hot(1, 2_000, None, None),
                        hot(2, 6_000, None, Some([0x28, 0xe2, 0x14])),
                        hot(3, 8_000, Some(10_000), None),
                        memory(12_000),
                    ],
                ),
                lib("b-124.wav", grid(124.0, s, 120.0), vec![hot(1, 1_088, None, None)]),
            ],
            roots: vec![],
        })
        .unwrap();
    db
}

struct Harness {
    ctx: egui::Context,
    app: LibraryApp,
    viewport: egui::Vec2,
    time: f64,
}

impl Harness {
    fn new(db: PathBuf) -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let app = LibraryApp::new(db, ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens());
        let mut h = Self { ctx, app, viewport: egui::vec2(1440.0, 900.0), time: 0.0 };
        h.app.show_decks = true;
        h.settle();
        h
    }

    /// One frame through the production path: raw-input hook, then `ui`.
    fn frame_with(&mut self, events: Vec<egui::Event>, modifiers: egui::Modifiers) -> Frame {
        self.time += 1.0 / 60.0;
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.viewport)),
            events,
            modifiers,
            time: Some(self.time),
            ..Default::default()
        };
        self.app.deck_input_hook(&self.ctx, &mut input);
        let app = &mut self.app;
        self.ctx.run(input, |ctx| app.ui(ctx))
    }

    fn settle(&mut self) -> Frame {
        self.frame_with(vec![], Default::default());
        self.frame_with(vec![], Default::default())
    }

    fn press(&mut self, pos: Pos2, button: egui::PointerButton) -> Frame {
        let ev = |pressed| egui::Event::PointerButton { pos, button, pressed, modifiers: Default::default() };
        self.frame_with(vec![egui::Event::PointerMoved(pos), ev(true)], Default::default());
        self.frame_with(vec![ev(false)], Default::default());
        self.settle()
    }

    fn click(&mut self, pos: Pos2) -> Frame {
        self.press(pos, egui::PointerButton::Primary)
    }

    fn drag(&mut self, from: Pos2, to: Pos2) -> Frame {
        let ev = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        self.frame_with(vec![egui::Event::PointerMoved(from), ev(from, true)], Default::default());
        for k in 1..=4 {
            let p = from + (to - from) * (k as f32 / 4.0);
            self.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
        }
        self.frame_with(vec![ev(to, false)], Default::default());
        self.settle()
    }

    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> Frame {
        let event = |pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
        self.frame_with(vec![event(true)], modifiers);
        self.frame_with(vec![event(false)], Default::default());
        self.settle()
    }

    fn tap(&mut self, key: egui::Key) -> Frame {
        self.key(key, Default::default())
    }

    fn id_of(&self, name: &str) -> u64 {
        self.app.tracks.iter().find(|t| t.track.path.ends_with(name)).unwrap().track.id.as_u64()
    }

    fn select_and_load(&mut self, name: &str, key: egui::Key) {
        self.app.selected = Some(self.id_of(name));
        self.tap(key);
    }

    fn layout(&self) -> crate::decks::DeckLayout {
        self.app.decks.layout
    }

    fn deck(&self, d: usize) -> &crate::decks::Deck {
        &self.app.decks.decks[d]
    }

    fn focused_rect(&self) -> Option<Rect> {
        self.ctx.memory(|m| m.focused()).and_then(|f| self.ctx.read_response(f)).map(|r| r.rect)
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

fn accesskit_nodes(out: &Frame) -> Vec<accesskit::Node> {
    out.platform_output.accesskit_update.as_ref().expect("accesskit enabled").nodes.iter().map(|(_, n)| n.clone()).collect()
}

fn node(out: &Frame, name: &str) -> accesskit::Node {
    accesskit_nodes(out)
        .into_iter()
        .find(|n| n.label().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name:?} not in the AccessKit tree"))
}

/// Mesh vertices (with colours) of meshes clipped to exactly `clip`.
fn meshes_in(out: &Frame, clip: Rect) -> Vec<(Pos2, egui::Color32)> {
    out.shapes
        .iter()
        .filter(|c| c.clip_rect == clip)
        .filter_map(|c| match &c.shape {
            Shape::Mesh(m) => Some(m.vertices.iter().map(|v| (v.pos, v.color)).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn fresh(tag: &str, seconds: f32) -> (PathBuf, Harness) {
    let root = temp_root(tag);
    let db = deck_catalog(&root, seconds);
    let h = Harness::new(db);
    (root, h)
}

fn track_of(catalog: &Catalog, name: &str) -> TrackDetails {
    catalog.list_track_details().unwrap().into_iter().find(|t| t.track.path.ends_with(name)).unwrap()
}

#[test]
fn load_by_keys_button_and_context_menu_with_table_badges() {
    let (root, mut h) = fresh("load", 20.0);
    let out = h.settle();
    assert!(texts(&out).iter().any(|(t, _)| t == NO_AUDIO), "status bar says NO AUDIO");
    assert!(h.deck(0).track.is_none() && h.deck(1).track.is_none());
    // A key loads the selected track to deck A.
    h.select_and_load("a-120.wav", egui::Key::A);
    assert_eq!(h.deck(0).track.as_ref().map(|t| t.id), Some(h.id_of("a-120.wav")));
    assert!(h.app.notice.contains("deck A"), "{}", h.app.notice);
    // Imported hot cues seed the pads.
    assert_eq!(h.deck(0).pads[0], Some(2_000.0));
    assert_eq!(h.deck(0).pads[2], Some(8_000.0));
    assert_eq!(h.deck(0).pads[3], None);
    // The Load button of deck B loads the selected track there.
    h.app.selected = Some(h.id_of("b-124.wav"));
    h.settle();
    let load_b = h.layout().load[1].expect("deck B Load button");
    h.click(load_b.center());
    assert_eq!(h.deck(1).track.as_ref().map(|t| t.id), Some(h.id_of("b-124.wav")));
    // Context menu: secondary click on a row, then "Load to deck B".
    let table = h.app.shell.table.expect("table");
    let row_c = {
        let out = h.settle();
        texts(&out).into_iter().find(|(t, r)| t == "c-nogrid.wav" && table.contains_rect(*r)).expect("c row").1
    };
    let out = h.press(row_c.center(), egui::PointerButton::Secondary);
    let item = texts(&out).into_iter().find(|(t, _)| t == "Load to deck B").expect("context menu item").1;
    h.click(item.center());
    assert_eq!(h.deck(1).track.as_ref().map(|t| t.id), Some(h.id_of("c-nogrid.wav")));
    assert_eq!(h.app.selected, Some(h.id_of("c-nogrid.wav")));
    // Badges: an "A" chip in a-120's row, a "B" chip in c-nogrid's row.
    let out = h.settle();
    let t = texts(&out);
    let row = |name: &str| t.iter().find(|(s, r)| s == name && table.contains_rect(*r)).expect("row").1;
    let badge = |letter: &str, row: Rect| t.iter().any(|(s, r)| s == letter && table.contains_rect(*r) && (r.center().y - row.center().y).abs() < 6.0);
    assert!(badge("A", row("a-120.wav")) && !badge("B", row("a-120.wav")));
    assert!(badge("B", row("c-nogrid.wav")) && !badge("A", row("c-nogrid.wav")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn merged_detail_halves_share_one_seam_playhead_and_paint_inside_the_clip() {
    let (root, mut h) = fresh("geometry", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let duration = [0, 1].map(|d| h.deck(d).track.as_ref().unwrap().duration_ms);
    for (case, frac) in [("min", 0.0), ("mid", 0.5), ("max", 1.0)] {
        for d in 0..2 {
            h.app.decks.decks[d].seek(duration[d] * frac);
        }
        let out = h.settle();
        let l = h.layout();
        let detail = l.detail.expect("detail view");
        let [a, b] = l.halves.expect("halves");
        let area = l.area.expect("area");
        assert!(area.contains_rect(detail), "{case}");
        // A above B, sharing the seam, one time axis (same width).
        assert_eq!(a.bottom(), b.top(), "{case}: one seam");
        assert!(a.top() == detail.top() && b.bottom() == detail.bottom() && a.width() == b.width());
        let ph = l.playhead_x.expect("playhead");
        assert!((ph - detail.center().x).abs() <= 1.0, "{case}: centre playhead (default)");
        // Each half's window is centred on its deck position.
        for d in 0..2 {
            let (t0, t1) = l.windows[d].expect("window");
            let at_ph = t0 + (t1 - t0) * f64::from((ph - detail.left()) / detail.width());
            assert!((at_ph - h.deck(d).position_ms).abs() < 30.0, "{case}: deck {d} under playhead");
        }
        // Every shape painted with a deck-area clip lies inside that clip.
        let mut checked = 0;
        for c in out.shapes.iter().filter(|c| area.contains_rect(c.clip_rect)) {
            let bounds = c.shape.visual_bounding_rect();
            if bounds.is_positive() {
                assert!(c.clip_rect.expand(0.01).contains_rect(bounds), "{case}: {bounds:?} outside {:?}", c.clip_rect);
                checked += 1;
            }
        }
        assert!(checked > 20, "{case}: {checked}");
        // Waveforms grow away from the seam: A up, B down, only inside its half.
        let va = meshes_in(&out, a);
        let vb = meshes_in(&out, b);
        assert!(!va.is_empty() && !vb.is_empty(), "{case}: both halves painted");
        assert!(va.iter().all(|(p, _)| a.expand(0.01).contains(*p)), "{case}: A inside its half");
        assert!(vb.iter().all(|(p, _)| b.expand(0.01).contains(*p)), "{case}: B inside its half");
        let reach_a = va.iter().filter(|(_, c)| c.a() == 255).map(|(p, _)| a.bottom() - p.y).fold(0.0, f32::max);
        let reach_b = vb.iter().filter(|(_, c)| c.a() == 255).map(|(p, _)| p.y - b.top()).fold(0.0, f32::max);
        if case == "mid" {
            assert!(reach_a > a.height() * 0.4 && reach_b > b.height() * 0.4, "{case}: {reach_a} {reach_b}");
        }
        // At min the lane left of the playhead (before the track) is empty.
        if case == "min" {
            let opaque_left = va.iter().filter(|(p, c)| c.a() == 255 && p.x < ph - 2.0 && p.y < a.bottom() - 1.5).count();
            assert_eq!(opaque_left, 0, "nothing before the track start");
        }
    }
    // Left-third playhead from Settings.
    h.app.settings.playhead = crate::settings::Playhead::LeftThird;
    h.settle();
    let l = h.layout();
    let detail = l.detail.unwrap();
    assert!((l.playhead_x.unwrap() - (detail.left() + detail.width() / 3.0)).abs() <= 1.0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn zoom_by_keys_and_wheel_uses_bars_or_seconds_and_drag_scrubs() {
    let (root, mut h) = fresh("zoom", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.app.decks.decks[0].seek(10_000.0);
    h.settle();
    let span = |h: &Harness, d: usize| {
        let (a, b) = h.layout().windows[d].unwrap();
        b - a
    };
    assert_eq!(h.app.settings.zoom_bars, 4);
    // 120 BPM: one bar = 2000 ms.
    assert!((span(&h, 0) - 8_000.0).abs() < 20.0, "{}", span(&h, 0));
    h.tap(egui::Key::Plus);
    assert_eq!(h.app.settings.zoom_bars, 2);
    assert!((span(&h, 0) - 4_000.0).abs() < 20.0);
    h.tap(egui::Key::Plus);
    h.tap(egui::Key::Plus);
    assert_eq!(h.app.settings.zoom_bars, 1, "clamped at 1 bar");
    for _ in 0..4 {
        h.tap(egui::Key::Minus);
    }
    assert_eq!(h.app.settings.zoom_bars, 8, "clamped at 8 bars");
    assert!((span(&h, 0) - 16_000.0).abs() < 40.0);
    // Wheel over the detail view zooms in.
    let detail = h.layout().detail.unwrap();
    h.frame_with(vec![egui::Event::PointerMoved(detail.center())], Default::default());
    h.frame_with(
        vec![egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, 1.0), modifiers: Default::default() }],
        Default::default(),
    );
    h.settle();
    assert_eq!(h.app.settings.zoom_bars, 4, "wheel up zooms in one step");
    // Without a grid: 2 s per bar (deck B active with a no-grid track).
    h.select_and_load("c-nogrid.wav", egui::Key::B);
    h.app.decks.decks[0].track = None;
    h.app.decks.active = 1;
    h.app.decks.decks[1].seek(10_000.0);
    for (bars, seconds) in [(1, 2.0), (2, 4.0), (4, 8.0), (8, 16.0)] {
        h.app.settings.zoom_bars = bars;
        h.settle();
        assert!((span(&h, 1) - seconds * 1000.0).abs() < 40.0, "{bars}: {}", span(&h, 1));
    }
    // Dragging the B half right scrubs deck B backwards (content follows).
    let [_, b] = h.layout().halves.unwrap();
    let before = h.deck(1).position_ms;
    h.drag(b.center(), b.center() + egui::vec2(100.0, 0.0));
    assert!(h.deck(1).position_ms < before - 500.0, "{} -> {}", before, h.deck(1).position_ms);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn overviews_are_two_separate_strips_at_the_bottom_and_seek_their_own_deck() {
    let (root, mut h) = fresh("overview", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let out = h.settle();
    let l = h.layout();
    let area = l.area.unwrap();
    let [oa, ob] = l.overviews.expect("two overview strips");
    assert!(oa.bottom() < ob.top(), "A first, B second, not merged");
    assert!((ob.bottom() - area.bottom()).abs() < 1.0, "overviews at the bottom of the deck area");
    assert!(oa.top() > l.strip.unwrap().bottom() && oa.top() > l.detail.unwrap().bottom());
    assert!((oa.width() - area.width()).abs() < 1.0 && (ob.width() - area.width()).abs() < 1.0, "full width");
    for (d, r) in [oa, ob].into_iter().enumerate() {
        assert!(!meshes_in(&out, r).is_empty(), "deck {d} overview waveform");
    }
    let name = |d: &str| format!("Deck {d} overview: click or drag to seek");
    assert_eq!(node(&out, &name("A")).role(), accesskit::Role::Slider);
    node(&out, &name("B"));
    // Click on A's overview seeks only deck A.
    let inner = |r: Rect| (r.left() + decks::OVERVIEW_INSET, r.right());
    let (l0, r0) = inner(oa);
    h.click(Pos2::new(l0 + (r0 - l0) * 0.5, oa.center().y));
    let (da, db) = (h.deck(0).track.as_ref().unwrap().duration_ms, h.deck(1).track.as_ref().unwrap().duration_ms);
    assert!((h.deck(0).position_ms - da * 0.5).abs() < da * 0.01, "{}", h.deck(0).position_ms);
    assert_eq!(h.deck(1).position_ms, 0.0);
    // Drag on B's overview seeks only deck B, to where the drag ends.
    let (l1, r1) = inner(ob);
    h.drag(Pos2::new(l1 + 10.0, ob.center().y), Pos2::new(l1 + (r1 - l1) * 0.75, ob.center().y));
    assert!((h.deck(1).position_ms - db * 0.75).abs() < db * 0.01, "{}", h.deck(1).position_ms);
    assert!((h.deck(0).position_ms - da * 0.5).abs() < da * 0.01, "A unchanged");
    // Played part dimmed: one translucent rect from the content edge to the playhead.
    let out = h.settle();
    let x = l0 + (r0 - l0) * 0.5;
    let dims = out
        .shapes
        .iter()
        .filter(|c| c.clip_rect == oa)
        .filter(|c| matches!(&c.shape, Shape::Rect(r) if r.fill.a() > 0 && r.fill.a() < 255 && (r.rect.left() - l0).abs() < 0.5 && (r.rect.right() - x).abs() < 1.5))
        .count();
    assert_eq!(dims, 1, "played part dimmed up to the playhead");
    // View-window box: a stroked, unfilled rect inside the strip.
    assert!(
        out.shapes.iter().any(|c| c.clip_rect == oa && matches!(&c.shape, Shape::Rect(r) if r.stroke.width > 0.0 && r.fill.a() == 0)),
        "detail window box drawn"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn time_grid_when_no_waveform_or_grid_and_the_waveform_replaces_it_in_place() {
    let root = temp_root("fallback");
    let db = deck_catalog(&root, 20.0);
    let mut h = Harness::new(db.clone());
    h.select_and_load("d-pending.wav", egui::Key::A);
    h.select_and_load("c-nogrid.wav", egui::Key::B);
    h.app.decks.decks[0].seek(8_000.0);
    h.app.decks.decks[1].seek(8_000.0);
    let out = h.settle();
    let [a, b] = h.layout().halves.unwrap();
    let t = texts(&out);
    let clock_in = |r: Rect| t.iter().filter(|(s, rr)| r.contains_rect(*rr) && s.contains(':') && s.len() <= 5).count();
    assert!(clock_in(a) >= 1 && clock_in(b) >= 1, "mm:ss time grid in both halves");
    assert!(t.iter().any(|(s, r)| s == NOT_READY && a.contains_rect(*r)), "quiet analysing state in A");
    assert!(!t.iter().any(|(s, r)| s == NOT_READY && b.contains_rect(*r)), "B has its waveform");
    assert!(meshes_in(&out, a).iter().all(|(_, c)| c.a() < 255), "no waveform yet in A");
    assert!(meshes_in(&out, b).iter().any(|(_, c)| c.a() == 255), "B waveform drawn with the time grid");
    assert!(h.deck(1).track.as_ref().unwrap().grid.is_none());
    // Analyse the pending track; a reload swaps the waveform in, same half.
    {
        let mut catalog = Catalog::open(&db).unwrap();
        let id = track_of(&catalog, "d-pending.wav").track.id;
        oxidj_application::analyze_track_waveform(&mut catalog, id).unwrap();
    }
    h.app.reload();
    let out = h.settle();
    assert_eq!(h.layout().halves.unwrap(), [a, b], "same place");
    assert!(!texts(&out).iter().any(|(s, r)| s == NOT_READY && a.contains_rect(*r)));
    assert!(meshes_in(&out, a).iter().any(|(_, c)| c.a() == 255), "waveform replaced the placeholder");
    assert!((h.deck(0).position_ms - 8_000.0).abs() < 1.0, "position kept across reload");
    // An imported grid wins over a computed one.
    {
        let mut catalog = Catalog::open(&db).unwrap();
        let id = track_of(&catalog, "a-120.wav").track.id;
        oxidj_application::analyze_track_tempo(&mut catalog, id, oxidj_application::DEFAULT_TEMPO_RANGE).unwrap();
        assert!(catalog.computed_beat_grid(id).unwrap().is_some());
    }
    h.app.reload();
    h.select_and_load("a-120.wav", egui::Key::A);
    let (_, source) = h.deck(0).track.as_ref().unwrap().grid.as_ref().expect("grid");
    assert_eq!(*source, crate::track_waveform::GridSource::Library);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deck_palettes_are_warm_and_cool_with_contrast_in_both_themes() {
    for theme in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
        let tokens = theme.tokens();
        for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
            let [a, b]: [DeckColours; 2] = [0, 1].map(|d| decks::deck_colours(palette, d, tokens));
            assert_ne!(a.outer, b.outer, "{theme:?} {palette:?}");
            for (d, c) in [a, b].into_iter().enumerate() {
                let what = format!("{theme:?} {palette:?} deck {d}");
                // Graphics (waveform outer/mid, rail, bar lines) >= 3:1 on the plate.
                assert!(decks::contrast(c.outer, tokens.background) >= 3.0, "{what}: outer");
                assert!(decks::contrast(c.mid, tokens.background) >= 3.0, "{what}: mid");
                // Chip letters >= 4.5:1.
                assert!(decks::contrast(c.chip_text, c.outer) >= 4.5, "{what}: chip text");
                // The wash and watermark stay faint.
                assert!(c.wash.a() <= 30 && c.watermark.a() <= 40, "{what}");
            }
            if palette == DeckPalette::WarmCool {
                assert!(a.outer.r() > a.outer.b() && a.mid.r() > a.mid.b(), "{theme:?}: A warm");
                assert!(b.outer.b() > b.outer.r() && b.mid.b() > b.mid.r(), "{theme:?}: B cool");
            }
        }
    }
    // The production frame paints each deck rail in its palette colour, and
    // the Classic preset from Settings swaps them.
    let (root, mut h) = fresh("palette", 10.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    let rail_colour = |out: &Frame, h: &Harness, d: usize| {
        let half = h.layout().halves.unwrap()[d];
        out.shapes
            .iter()
            .find_map(|c| match &c.shape {
                Shape::Rect(r) if c.clip_rect == half && r.rect.left() == half.left() && (r.rect.width() - decks::RAIL).abs() < 0.01 => Some(r.fill),
                _ => None,
            })
            .expect("rail")
    };
    let tokens = ThemeMode::TokyoNight.tokens();
    for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
        h.app.settings.deck_palette = palette;
        let out = h.settle();
        for d in 0..2 {
            assert_eq!(rail_colour(&out, &h, d), decks::deck_colours(palette, d, tokens).outer, "{palette:?} {d}");
        }
        // Large faint watermark letter per half.
        let [a, b] = h.layout().halves.unwrap();
        let t = texts(&out);
        assert!(t.iter().any(|(s, r)| s == "A" && a.contains_rect(*r) && r.height() > a.height() * 0.5));
        assert!(t.iter().any(|(s, r)| s == "B" && b.contains_rect(*r) && r.height() > b.height() * 0.5));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pads_cue_play_tab_and_loops_drive_the_silent_transport() {
    let (root, mut h) = fresh("pads", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    assert_eq!(h.app.decks.active, 0);
    // Key 1 jumps deck A to its imported hot cue 1.
    h.tap(egui::Key::Num1);
    assert_eq!(h.deck(0).position_ms, 2_000.0);
    // An empty pad stores the position; clicking that pad later jumps back.
    h.app.decks.decks[0].seek(5_000.0);
    h.tap(egui::Key::Num4);
    assert_eq!(h.deck(0).pads[3], Some(5_000.0));
    h.app.decks.decks[0].seek(0.0);
    let pad4 = h.layout().pads[0].unwrap()[3];
    h.click(pad4.center());
    assert_eq!(h.deck(0).position_ms, 5_000.0);
    let out = h.settle();
    let pad = node(&out, "Deck A hot cue 4 (set)");
    assert_eq!(pad.role(), accesskit::Role::Button);
    node(&out, "Deck B hot cue 2 (empty)");
    // Space plays the active deck; the playhead advances by wall clock.
    h.tap(egui::Key::Space);
    assert!(h.deck(0).playing && !h.deck(1).playing);
    let start = h.deck(0).position_ms;
    for _ in 0..30 {
        h.frame_with(vec![], Default::default());
    }
    let moved = h.deck(0).position_ms - start;
    assert!((450.0..=600.0).contains(&moved), "30 frames ≈ 0.5 s: {moved}");
    // Tempo +10 % speeds the silent transport up.
    h.app.decks.decks[0].tempo_pct = 10.0;
    let start = h.deck(0).position_ms;
    for _ in 0..60 {
        h.frame_with(vec![], Default::default());
    }
    let moved = h.deck(0).position_ms - start;
    assert!((1050.0..=1150.0).contains(&moved), "1 s at +10 %: {moved}");
    // Q while playing: stop and return to the cue point (0 by default).
    h.tap(egui::Key::Q);
    assert!(!h.deck(0).playing && h.deck(0).position_ms == 0.0);
    // Tab switches the active deck (no widget focused); Space then plays B.
    h.tap(egui::Key::Tab);
    assert_eq!(h.app.decks.active, 1);
    assert!(h.ctx.memory(|m| m.focused()).is_none(), "Tab did not start focus traversal");
    h.tap(egui::Key::Space);
    assert!(h.deck(1).playing && !h.deck(0).playing);
    // P cues deck B.
    h.tap(egui::Key::P);
    assert!(!h.deck(1).playing);
    // Pads 1-8 act on the active deck (B): its hot cue 1.
    h.tap(egui::Key::Num1);
    assert_eq!(h.deck(1).position_ms, 1_088.0);
    // Loops: IN at 4 s, OUT at 6 s, ½ → 1 s, ×2 → 2 s; playback wraps.
    h.tap(egui::Key::Tab);
    assert_eq!(h.app.decks.active, 0);
    let loops = h.layout().loops[0].unwrap();
    h.app.decks.decks[0].seek(4_000.0);
    h.click(loops[0].center());
    h.app.decks.decks[0].seek(6_000.0);
    h.click(loops[1].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 6_000.0)));
    h.click(loops[2].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 5_000.0)));
    h.click(loops[3].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 6_000.0)));
    h.app.decks.decks[0].tempo_pct = 0.0;
    h.app.decks.decks[0].seek(5_900.0);
    h.app.decks.decks[0].playing = true;
    for _ in 0..12 {
        h.frame_with(vec![], Default::default());
    }
    let p = h.deck(0).position_ms;
    assert!((4_000.0..4_300.0).contains(&p), "wrapped into the loop: {p}");
    h.app.decks.decks[0].playing = false;
    // SYNC and key-lock exist but are disabled until D3.
    let out = h.settle();
    for name in ["Deck A sync (D3)", "Deck B key lock (D3)"] {
        assert!(node(&out, name).is_disabled(), "{name}");
    }
    assert_eq!(node(&out, "Deck A tempo").role(), accesskit::Role::Slider);
    // Seam readout appears when both decks have a grid.
    assert!(h.layout().readout.is_some());
    assert!(texts(&out).iter().any(|(s, _)| s.starts_with("Δ ") && s.contains("BPM")));
    h.app.decks.decks[1].track = None;
    h.settle();
    assert!(h.layout().readout.is_none(), "no readout without both grids");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_cues_and_loops_draw_on_detail_and_overview() {
    let (root, mut h) = fresh("cues", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.app.settings.zoom_bars = 8; // 16 s window around 8 s: every cue visible
    h.app.decks.decks[0].seek(8_000.0);
    let out = h.settle();
    let a = h.layout().halves.unwrap()[0];
    let oa = h.layout().overviews.unwrap()[0];
    let oa_inner = Rect::from_min_max(Pos2::new(oa.left() + decks::OVERVIEW_INSET, oa.top()), oa.max);
    let t = texts(&out);
    for (clip, what) in [(a, "detail"), (oa, "overview")] {
        for n in ["1", "2", "3"] {
            assert!(t.iter().any(|(s, r)| s == n && clip.contains_rect(*r)), "{what}: hot cue {n} number");
        }
        // Hot cue 2's stored colour paints its marker.
        let green = egui::Color32::from_rgb(0x28, 0xe2, 0x14);
        assert!(out.shapes.iter().any(|c| c.clip_rect == clip && matches!(&c.shape, Shape::Rect(r) if r.fill == green)), "{what}: stored colour");
        // Two memory cues: triangles.
        let triangles = out.shapes.iter().filter(|c| c.clip_rect == clip && matches!(&c.shape, Shape::Path(p) if p.points.len() == 3)).count();
        assert!(triangles >= 2, "{what}: memory cues {triangles}");
        // Loop 8–10 s shaded (translucent fill).
        assert!(
            out.shapes.iter().any(|c| c.clip_rect == clip && matches!(&c.shape, Shape::Rect(r) if r.fill.a() == 46)),
            "{what}: loop region"
        );
    }
    let _ = oa_inner;
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn auto_gain_lifts_a_quiet_master_and_follows_the_setting() {
    let (root, mut h) = fresh("gain", 20.0);
    h.select_and_load("c-nogrid.wav", egui::Key::A);
    h.app.decks.decks[0].seek(10_000.0);
    let reach = |h: &mut Harness| {
        let out = h.settle();
        let a = h.layout().halves.unwrap()[0];
        meshes_in(&out, a).iter().filter(|(_, c)| c.a() == 255).map(|(p, _)| a.bottom() - p.y).fold(0.0, f32::max)
    };
    let on = reach(&mut h);
    h.app.settings.auto_gain = false;
    let off = reach(&mut h);
    assert!(h.deck(0).track.as_ref().unwrap().gain(true) > 1.5);
    assert!(on > off * 1.5, "auto-gain {on} vs raw {off}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_section_combination_collapses_cleanly_toggles_and_persists() {
    let (root, mut h) = fresh("sections", 10.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let mut heights = std::collections::HashMap::new();
    for bits in 0..16u8 {
        let sections = DeckSections::from_bits(bits);
        h.app.settings.sections = sections;
        let out = h.settle();
        let l = h.layout();
        let area = l.area.expect("deck area");
        let screen = Rect::from_min_size(Pos2::ZERO, h.viewport);
        assert!(screen.contains_rect(h.app.shell.decks.unwrap()), "{bits:04b}");
        assert_eq!(l.detail.is_some(), sections.detail, "{bits:04b}");
        assert_eq!(l.overviews.is_some(), sections.overviews, "{bits:04b}");
        assert_eq!(l.strip.is_some(), sections.strip, "{bits:04b}");
        assert_eq!(l.mixer.is_some(), sections.mixer, "{bits:04b}");
        // Stacking order: toggles, detail, strip, mixer, overviews; no overlap.
        let order: Vec<Rect> = [l.toggles.map(|t| t[0]), l.detail, l.strip, l.mixer, l.overviews.map(|o| o[0].union(o[1]))]
            .into_iter()
            .flatten()
            .collect();
        for w in order.windows(2) {
            assert!(w[0].bottom() <= w[1].top() + 0.01, "{bits:04b}: {w:?}");
        }
        for r in &order {
            assert!(area.expand(0.5).contains_rect(*r), "{bits:04b}");
        }
        // Toggles stay reachable with their state and tooltip-able names exposed.
        for (i, name) in DeckSections::NAMES.iter().enumerate() {
            let n = node(&out, &format!("{name} section"));
            let want = if sections.get(i) { accesskit::Toggled::True } else { accesskit::Toggled::False };
            assert_eq!(n.toggled(), Some(want), "{bits:04b}: {name}");
        }
        heights.insert(bits, h.app.shell.table.unwrap().height());
    }
    // Collapsing frees space for the library: every expanded section costs height.
    for bits in 0..16u8 {
        for i in 0..4 {
            if bits & (1 << i) != 0 {
                assert!(heights[&(bits & !(1 << i))] > heights[&bits] + 10.0, "{bits:04b} minus section {i}");
            }
        }
    }
    // Mixer hidden by default; clicking its toggle shows the placeholder.
    h.app.settings.sections = DeckSections::default();
    h.settle();
    assert!(h.layout().mixer.is_none());
    let mixer_toggle = h.layout().toggles.unwrap()[3];
    h.click(mixer_toggle.center());
    assert!(h.app.settings.sections.mixer && h.layout().mixer.is_some());
    // Keyboard: Ctrl+Shift+1 hides the detail view.
    let chord = egui::Modifiers { ctrl: true, command: true, shift: true, ..Default::default() };
    h.key(egui::Key::Num1, chord);
    assert!(!h.app.settings.sections.detail && h.layout().detail.is_none());
    // Shift+Tab enters the controls; walk to the Detail toggle and press Space.
    let toggle = h.layout().toggles.unwrap()[0];
    assert!(h.ctx.memory(|m| m.focused()).is_none());
    let shift = egui::Modifiers { shift: true, ..Default::default() };
    h.key(egui::Key::Tab, shift);
    let mut guard = 0;
    while h.focused_rect() != Some(toggle) {
        h.key(egui::Key::Tab, shift);
        guard += 1;
        assert!(guard < 200, "Detail toggle reachable by keyboard");
    }
    h.tap(egui::Key::Space);
    assert!(h.app.settings.sections.detail, "Space on the focused toggle shows the detail view");
    // Persistence through the eframe storage of the production app.
    #[derive(Default)]
    struct Store(std::collections::HashMap<String, String>);
    impl eframe::Storage for Store {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn flush(&mut self) {}
    }
    h.app.settings.sections = DeckSections::from_bits(0b1010);
    let mut store = Store::default();
    eframe::App::save(&mut h.app, &mut store);
    assert_eq!(Settings::load(Some(&store)).sections, DeckSections::from_bits(0b1010));
    // Ctrl+D still hides the whole deck area.
    let ctrl = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
    h.key(egui::Key::D, ctrl);
    assert!(!h.app.show_decks && h.app.shell.decks.is_none() && h.layout().area.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// Not a behaviour test: writes the synthetic capture catalog used for the
/// v0.14 Bot Screen captures into `$OXIDJ_D1_CAPTURE_DIR`.
#[test]
#[ignore]
fn seed_capture_catalog() {
    let Ok(dir) = std::env::var("OXIDJ_D1_CAPTURE_DIR") else { return };
    let db = deck_catalog(Path::new(&dir), 240.0);
    println!("{}", db.display());
}
