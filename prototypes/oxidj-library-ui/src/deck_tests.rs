//! M8/D1 + D2: the deck area through the production `LibraryApp::ui` path
//! (plus the eframe raw-input hook): load, seek, zoom, merged geometry,
//! overview separation, grid fallback, palette, pads, transport keys and
//! every collapse combination with persistence. Synthetic audio only.
//!
//! D2: the app runs the real `oxidj-audio` engine on its fake sink — the
//! harness renders 1/60 s of audio (800 frames at 48 kHz) after every UI
//! frame, exactly the code a device callback runs — so the playhead,
//! cue, loops and end of track are the engine's, never the frame clock's.

use super::*;
use crate::decks::{self, DeckAudio, DeckColours, NOT_READY};
use crate::settings::{DeckPalette, DeckSections, Settings};
use eframe::egui::{Pos2, Rect, Shape, accesskit};
use oxidj_library::{LibraryBeat, LibraryCue, LibrarySnapshot, LibraryTrack};
use std::path::{Path, PathBuf};

type Frame = egui::FullOutput;

/// 1/60 s at the fake sink's 48 kHz.
const AUDIO_FRAMES_PER_UI_FRAME: usize = 800;

/// 16-bit mono PCM WAV: kick on every beat at `bpm`, off-beat hat, quiet
/// bass, scaled by `level` (a quiet master exercises auto-gain).
pub(crate) fn write_loop_wav(path: &Path, seconds: f32, bpm: f32, level: f32) {
    let rate = 44_100u32;
    let n = (rate as f32 * seconds) as u32;
    let mut bytes = Vec::new();
    for chunk in [
        &b"RIFF"[..],
        &(36 + n * 2).to_le_bytes(),
        b"WAVEfmt ",
        &16u32.to_le_bytes(),
    ] {
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
        let hat = if (ph - 0.5).abs() < 0.04 {
            ((u64::from(i) * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5
        } else {
            0.0
        };
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
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
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
    LibraryCue {
        hot_cue: Some(n),
        start_ms: ms,
        end_ms: end,
        name: String::new(),
        colour,
    }
}

fn memory(ms: u32) -> LibraryCue {
    LibraryCue {
        hot_cue: None,
        start_ms: ms,
        end_ms: None,
        name: String::new(),
        colour: None,
    }
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
    catalog
        .import_audio_directory(audio.to_str().unwrap())
        .unwrap();
    let tracks = catalog.list_track_details().unwrap();
    let by = |name: &str| {
        tracks
            .iter()
            .find(|t| t.track.path.ends_with(name))
            .unwrap()
            .track
            .clone()
    };
    for name in ["a-120.wav", "b-124.wav", "c-nogrid.wav"] {
        oxidj_application::analyze_track_waveform(&mut catalog, by(name).id, false).unwrap();
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
                lib(
                    "b-124.wav",
                    grid(124.0, s, 120.0),
                    vec![hot(1, 1_088, None, None)],
                ),
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
        let mut h = Self {
            ctx,
            app,
            viewport: egui::vec2(1440.0, 900.0),
            time: 0.0,
        };
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
        let out = self.ctx.run(input, |ctx| app.ui(ctx));
        // The fake sink: 1/60 s of audio per UI frame.
        let _ = self.app.decks.render(AUDIO_FRAMES_PER_UI_FRAME);
        out
    }

    /// Frames until every loaded deck finished decoding (real worker).
    fn wait_audio(&mut self) {
        let start = std::time::Instant::now();
        while self
            .app
            .decks
            .decks
            .iter()
            .any(|d| matches!(d.audio, DeckAudio::Loading(_)))
        {
            assert!(start.elapsed().as_secs() < 30, "decode did not finish");
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.frame_with(vec![], Default::default());
        }
        self.settle();
    }

    fn settle(&mut self) -> Frame {
        self.frame_with(vec![], Default::default());
        self.frame_with(vec![], Default::default())
    }

    fn press(&mut self, pos: Pos2, button: egui::PointerButton) -> Frame {
        let ev = |pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Default::default(),
        };
        self.frame_with(
            vec![egui::Event::PointerMoved(pos), ev(true)],
            Default::default(),
        );
        self.frame_with(vec![ev(false)], Default::default());
        self.settle()
    }

    fn click(&mut self, pos: Pos2) -> Frame {
        self.press(pos, egui::PointerButton::Primary)
    }

    fn drag(&mut self, from: Pos2, to: Pos2) -> Frame {
        let ev = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        self.frame_with(
            vec![egui::Event::PointerMoved(from), ev(from, true)],
            Default::default(),
        );
        for k in 1..=4 {
            let p = from + (to - from) * (k as f32 / 4.0);
            self.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
        }
        self.frame_with(vec![ev(to, false)], Default::default());
        self.settle()
    }

    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> Frame {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame_with(vec![event(true)], modifiers);
        self.frame_with(vec![event(false)], Default::default());
        self.settle()
    }

    fn tap(&mut self, key: egui::Key) -> Frame {
        self.key(key, Default::default())
    }

    fn id_of(&self, name: &str) -> u64 {
        self.app
            .tracks
            .iter()
            .find(|t| t.track.path.ends_with(name))
            .unwrap()
            .track
            .id
            .as_u64()
    }

    fn select_and_load(&mut self, name: &str, key: egui::Key) {
        self.app.selected = Some(self.id_of(name));
        self.tap(key);
        self.wait_audio();
    }

    /// Seek deck `d` through its production method, applied by the engine.
    fn seek(&mut self, d: usize, ms: f64) {
        self.app.decks.decks[d].seek(ms);
        self.settle();
    }

    fn layout(&self) -> crate::decks::DeckLayout {
        self.app.decks.layout
    }

    fn deck(&self, d: usize) -> &crate::decks::Deck {
        &self.app.decks.decks[d]
    }

    fn focused_rect(&self) -> Option<Rect> {
        self.ctx
            .memory(|m| m.focused())
            .and_then(|f| self.ctx.read_response(f))
            .map(|r| r.rect)
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
    out.platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit enabled")
        .nodes
        .iter()
        .map(|(_, n)| n.clone())
        .collect()
}

fn node(out: &Frame, name: &str) -> accesskit::Node {
    accesskit_nodes(out)
        .into_iter()
        .find(|n| n.label().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name:?} not in the AccessKit tree"))
}

fn rotary_drag(h: &mut Harness, name: &str, angles: &[f32]) {
    let out = h.settle();
    let b = node(&out, name).bounds().unwrap();
    let center = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    let point = |degrees: f32| {
        let a = degrees.to_radians();
        center + egui::vec2(a.cos(), a.sin()) * 10.0
    };
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let first = point(angles[0]);
    h.frame_with(
        vec![egui::Event::PointerMoved(first), button(first, true)],
        Default::default(),
    );
    for angle in &angles[1..] {
        h.frame_with(
            vec![egui::Event::PointerMoved(point(*angle))],
            Default::default(),
        );
    }
    let last = point(*angles.last().unwrap());
    h.frame_with(vec![button(last, false)], Default::default());
    h.settle();
}

/// Mesh vertices (with colours) of meshes clipped to exactly `clip`.
fn meshes_in(out: &Frame, clip: Rect) -> Vec<(Pos2, egui::Color32)> {
    out.shapes
        .iter()
        .filter(|c| c.clip_rect == clip)
        .filter_map(|c| match &c.shape {
            Shape::Mesh(m) => Some(
                m.vertices
                    .iter()
                    .map(|v| (v.pos, v.color))
                    .collect::<Vec<_>>(),
            ),
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
    catalog
        .list_track_details()
        .unwrap()
        .into_iter()
        .find(|t| t.track.path.ends_with(name))
        .unwrap()
}

#[test]
fn load_by_keys_button_and_context_menu_with_table_badges() {
    let (root, mut h) = fresh("load", 20.0);
    let out = h.settle();
    assert!(
        texts(&out).iter().any(|(t, _)| t == "AUDIO · manual"),
        "status bar names the output"
    );
    assert!(h.deck(0).track.is_none() && h.deck(1).track.is_none());
    // A key loads the selected track to deck A.
    h.select_and_load("a-120.wav", egui::Key::A);
    assert_eq!(
        h.deck(0).track.as_ref().map(|t| t.id),
        Some(h.id_of("a-120.wav"))
    );
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
    assert_eq!(
        h.deck(1).track.as_ref().map(|t| t.id),
        Some(h.id_of("b-124.wav"))
    );
    // Context menu: secondary click on a row, then "Load to deck B".
    let table = h.app.shell.table.expect("table");
    let row_c = {
        let out = h.settle();
        texts(&out)
            .into_iter()
            .find(|(t, r)| t == "c-nogrid.wav" && table.contains_rect(*r))
            .expect("c row")
            .1
    };
    let out = h.press(row_c.center(), egui::PointerButton::Secondary);
    let item = texts(&out)
        .into_iter()
        .find(|(t, _)| t == "Load to deck B")
        .expect("context menu item")
        .1;
    h.click(item.center());
    assert_eq!(
        h.deck(1).track.as_ref().map(|t| t.id),
        Some(h.id_of("c-nogrid.wav"))
    );
    assert_eq!(h.app.selected, Some(h.id_of("c-nogrid.wav")));
    // Badges: an "A" chip in a-120's row, a "B" chip in c-nogrid's row.
    let out = h.settle();
    let t = texts(&out);
    let row = |name: &str| {
        t.iter()
            .find(|(s, r)| s == name && table.contains_rect(*r))
            .expect("row")
            .1
    };
    let badge = |letter: &str, row: Rect| {
        t.iter().any(|(s, r)| {
            s == letter && table.contains_rect(*r) && (r.center().y - row.center().y).abs() < 6.0
        })
    };
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
        assert!(
            (ph - detail.center().x).abs() <= 1.0,
            "{case}: centre playhead (default)"
        );
        // Each half's window is centred on its deck position.
        for d in 0..2 {
            let (t0, t1) = l.windows[d].expect("window");
            let at_ph = t0 + (t1 - t0) * f64::from((ph - detail.left()) / detail.width());
            assert!(
                (at_ph - h.deck(d).position_ms).abs() < 30.0,
                "{case}: deck {d} under playhead"
            );
        }
        // Every shape painted with a deck-area clip lies inside that clip.
        let mut checked = 0;
        for c in out
            .shapes
            .iter()
            .filter(|c| area.contains_rect(c.clip_rect))
        {
            let bounds = c.shape.visual_bounding_rect();
            if bounds.is_positive() {
                assert!(
                    c.clip_rect.expand(0.01).contains_rect(bounds),
                    "{case}: {bounds:?} outside {:?}",
                    c.clip_rect
                );
                checked += 1;
            }
        }
        assert!(checked > 20, "{case}: {checked}");
        // Waveforms grow away from the seam: A up, B down, only inside its half.
        let va = meshes_in(&out, a);
        let vb = meshes_in(&out, b);
        assert!(
            !va.is_empty() && !vb.is_empty(),
            "{case}: both halves painted"
        );
        assert!(
            va.iter().all(|(p, _)| a.expand(0.01).contains(*p)),
            "{case}: A inside its half"
        );
        assert!(
            vb.iter().all(|(p, _)| b.expand(0.01).contains(*p)),
            "{case}: B inside its half"
        );
        let reach_a = va
            .iter()
            .filter(|(_, c)| c.a() == 255)
            .map(|(p, _)| a.bottom() - p.y)
            .fold(0.0, f32::max);
        let reach_b = vb
            .iter()
            .filter(|(_, c)| c.a() == 255)
            .map(|(p, _)| p.y - b.top())
            .fold(0.0, f32::max);
        if case == "mid" {
            assert!(
                reach_a > a.height() * 0.4 && reach_b > b.height() * 0.4,
                "{case}: {reach_a} {reach_b}"
            );
        }
        // At min the lane left of the playhead (before the track) is empty.
        if case == "min" {
            let opaque_left = va
                .iter()
                .filter(|(p, c)| c.a() == 255 && p.x < ph - 2.0 && p.y < a.bottom() - 1.5)
                .count();
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
    h.frame_with(
        vec![egui::Event::PointerMoved(detail.center())],
        Default::default(),
    );
    h.frame_with(
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: Default::default(),
        }],
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
        assert!(
            (span(&h, 1) - seconds * 1000.0).abs() < 40.0,
            "{bars}: {}",
            span(&h, 1)
        );
    }
    // Dragging the B half right scrubs deck B backwards (content follows).
    let [_, b] = h.layout().halves.unwrap();
    let before = h.deck(1).position_ms;
    h.drag(b.center(), b.center() + egui::vec2(100.0, 0.0));
    assert!(
        h.deck(1).position_ms < before - 500.0,
        "{} -> {}",
        before,
        h.deck(1).position_ms
    );
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
    assert!(
        (ob.bottom() - area.bottom()).abs() < 1.0,
        "overviews at the bottom of the deck area"
    );
    assert!(oa.top() > l.strip.unwrap().bottom() && oa.top() > l.detail.unwrap().bottom());
    assert!(
        (oa.width() - area.width()).abs() < 1.0 && (ob.width() - area.width()).abs() < 1.0,
        "full width"
    );
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
    let (da, db) = (
        h.deck(0).track.as_ref().unwrap().duration_ms,
        h.deck(1).track.as_ref().unwrap().duration_ms,
    );
    assert!(
        (h.deck(0).position_ms - da * 0.5).abs() < da * 0.01,
        "{}",
        h.deck(0).position_ms
    );
    assert_eq!(h.deck(1).position_ms, 0.0);
    // Drag on B's overview seeks only deck B, to where the drag ends.
    let (l1, r1) = inner(ob);
    h.drag(
        Pos2::new(l1 + 10.0, ob.center().y),
        Pos2::new(l1 + (r1 - l1) * 0.75, ob.center().y),
    );
    assert!(
        (h.deck(1).position_ms - db * 0.75).abs() < db * 0.01,
        "{}",
        h.deck(1).position_ms
    );
    assert!(
        (h.deck(0).position_ms - da * 0.5).abs() < da * 0.01,
        "A unchanged"
    );
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
        out.shapes.iter().any(|c| c.clip_rect == oa
            && matches!(&c.shape, Shape::Rect(r) if r.stroke.width > 0.0 && r.fill.a() == 0)),
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
    let clock_in = |r: Rect| {
        t.iter()
            .filter(|(s, rr)| r.contains_rect(*rr) && s.contains(':') && s.len() <= 5)
            .count()
    };
    assert!(
        clock_in(a) >= 1 && clock_in(b) >= 1,
        "mm:ss time grid in both halves"
    );
    assert!(
        t.iter().any(|(s, r)| s == NOT_READY && a.contains_rect(*r)),
        "quiet analysing state in A"
    );
    assert!(
        !t.iter().any(|(s, r)| s == NOT_READY && b.contains_rect(*r)),
        "B has its waveform"
    );
    assert!(
        meshes_in(&out, a).iter().all(|(_, c)| c.a() < 255),
        "no waveform yet in A"
    );
    assert!(
        meshes_in(&out, b).iter().any(|(_, c)| c.a() == 255),
        "B waveform drawn with the time grid"
    );
    assert!(h.deck(1).track.as_ref().unwrap().grid.is_none());
    // Analyse the pending track; a reload swaps the waveform in, same half.
    {
        let mut catalog = Catalog::open(&db).unwrap();
        let id = track_of(&catalog, "d-pending.wav").track.id;
        oxidj_application::analyze_track_waveform(&mut catalog, id, false).unwrap();
    }
    h.app.reload();
    let out = h.settle();
    assert_eq!(h.layout().halves.unwrap(), [a, b], "same place");
    assert!(
        !texts(&out)
            .iter()
            .any(|(s, r)| s == NOT_READY && a.contains_rect(*r))
    );
    assert!(
        meshes_in(&out, a).iter().any(|(_, c)| c.a() == 255),
        "waveform replaced the placeholder"
    );
    assert!(
        (h.deck(0).position_ms - 8_000.0).abs() < 1.0,
        "position kept across reload"
    );
    // An imported grid wins over a computed one.
    {
        let mut catalog = Catalog::open(&db).unwrap();
        let id = track_of(&catalog, "a-120.wav").track.id;
        oxidj_application::analyze_track_tempo(
            &mut catalog,
            id,
            oxidj_application::DEFAULT_TEMPO_RANGE,
        )
        .unwrap();
        assert!(catalog.computed_beat_grid(id).unwrap().is_some());
    }
    h.app.reload();
    h.select_and_load("a-120.wav", egui::Key::A);
    let (_, source) = h
        .deck(0)
        .track
        .as_ref()
        .unwrap()
        .grid
        .as_ref()
        .expect("grid");
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
                assert!(
                    decks::contrast(c.outer, tokens.background) >= 3.0,
                    "{what}: outer"
                );
                assert!(
                    decks::contrast(c.mid, tokens.background) >= 3.0,
                    "{what}: mid"
                );
                // Chip letters >= 4.5:1.
                assert!(
                    decks::contrast(c.chip_text, c.outer) >= 4.5,
                    "{what}: chip text"
                );
                // The wash and watermark stay faint.
                assert!(c.wash.a() <= 30 && c.watermark.a() <= 40, "{what}");
            }
            if palette == DeckPalette::WarmCool {
                assert!(
                    a.outer.r() > a.outer.b() && a.mid.r() > a.mid.b(),
                    "{theme:?}: A warm"
                );
                assert!(
                    b.outer.b() > b.outer.r() && b.mid.b() > b.mid.r(),
                    "{theme:?}: B cool"
                );
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
                Shape::Rect(r)
                    if c.clip_rect == half
                        && r.rect.left() == half.left()
                        && (r.rect.width() - decks::RAIL).abs() < 0.01 =>
                {
                    Some(r.fill)
                }
                _ => None,
            })
            .expect("rail")
    };
    let tokens = ThemeMode::TokyoNight.tokens();
    for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
        h.app.settings.deck_palette = palette;
        let out = h.settle();
        for d in 0..2 {
            assert_eq!(
                rail_colour(&out, &h, d),
                decks::deck_colours(palette, d, tokens).outer,
                "{palette:?} {d}"
            );
        }
        // Large faint watermark letter per half.
        let [a, b] = h.layout().halves.unwrap();
        let t = texts(&out);
        assert!(
            t.iter()
                .any(|(s, r)| s == "A" && a.contains_rect(*r) && r.height() > a.height() * 0.5)
        );
        assert!(
            t.iter()
                .any(|(s, r)| s == "B" && b.contains_rect(*r) && r.height() > b.height() * 0.5)
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pads_cue_play_tab_and_loops_drive_the_engine() {
    let (root, mut h) = fresh("pads", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    assert!(
        h.deck(0).ready() && h.deck(1).ready(),
        "both decks decoded to RAM"
    );
    assert_eq!(
        h.app.decks.engine.deck(0).id,
        Some(h.id_of("a-120.wav")),
        "engine holds A's track"
    );
    assert_eq!(h.app.decks.active, 0);
    // Key 1 jumps deck A to its imported hot cue 1 (engine position).
    h.tap(egui::Key::Num1);
    assert_eq!(h.deck(0).position_ms, 2_000.0);
    assert_eq!(
        h.app.decks.engine.deck(0).frame,
        88_200.0,
        "2 s at 44.1 kHz, sample exact"
    );
    // An empty pad stores the position; clicking that pad later jumps back.
    h.seek(0, 5_000.0);
    h.tap(egui::Key::Num4);
    assert_eq!(h.deck(0).pads[3], Some(5_000.0));
    h.seek(0, 0.0);
    let pad4 = h.layout().pads[0].unwrap()[3];
    h.click(pad4.center());
    assert_eq!(h.deck(0).position_ms, 5_000.0);
    let out = h.settle();
    let pad = node(&out, "Deck A hot cue 4 (set)");
    assert_eq!(pad.role(), accesskit::Role::Button);
    node(&out, "Deck B hot cue 2 (empty)");
    // Space plays the active deck; the playhead is the engine's position:
    // 30 UI frames render 30 × 800 frames at 48 kHz = 0.5 s exactly.
    h.tap(egui::Key::Space);
    assert!(h.deck(0).playing && !h.deck(1).playing);
    let start = h.app.decks.engine.deck(0).frame;
    for _ in 0..30 {
        h.frame_with(vec![], Default::default());
    }
    let moved = h.app.decks.engine.deck(0).frame - start;
    assert!(
        (moved - 22_050.0).abs() < 1e-6,
        "0.5 s of 44.1 kHz source: {moved}"
    );
    // Tempo +10 % is plain varispeed on the same step.
    h.app.decks.decks[0].tempo_pct = 10.0;
    h.settle();
    let start = h.app.decks.engine.deck(0).frame;
    for _ in 0..60 {
        h.frame_with(vec![], Default::default());
    }
    let moved = h.app.decks.engine.deck(0).frame - start;
    assert!(
        (moved - 44_100.0 * 1.1).abs() < 1e-3,
        "1 s at +10 %: {moved}"
    );
    h.app.decks.decks[0].tempo_pct = 0.0;
    // Q while playing: stop and return to the cue point (0 by default).
    h.tap(egui::Key::Q);
    assert!(!h.deck(0).playing && h.deck(0).position_ms == 0.0);
    // Tab switches the active deck (no widget focused); Space then plays B.
    h.tap(egui::Key::Tab);
    assert_eq!(h.app.decks.active, 1);
    assert!(
        h.ctx.memory(|m| m.focused()).is_none(),
        "Tab did not start focus traversal"
    );
    h.tap(egui::Key::Space);
    assert!(h.deck(1).playing && !h.deck(0).playing);
    // P cues deck B.
    h.tap(egui::Key::P);
    assert!(!h.deck(1).playing);
    // Pads 1-8 act on the active deck (B): its hot cue 1.
    h.tap(egui::Key::Num1);
    assert!((h.deck(1).position_ms - 1_088.0).abs() < 1e-9);
    // Loops: IN at 4 s, OUT at 6 s, ½ → 1 s, ×2 → 2 s; playback wraps.
    h.tap(egui::Key::Tab);
    assert_eq!(h.app.decks.active, 0);
    let loops = h.layout().loops[0].unwrap();
    h.seek(0, 4_000.0);
    h.click(loops[0].center());
    assert_eq!(h.deck(0).loop_in, Some(4_000.0));
    h.seek(0, 6_000.0);
    h.click(loops[1].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 6_000.0)));
    h.click(loops[2].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 5_000.0)));
    h.click(loops[3].center());
    assert_eq!(h.deck(0).loop_range, Some((4_000.0, 6_000.0)));
    // Play from 5.9 s: 12 UI frames (200 ms) wrap 100 ms into the loop.
    h.seek(0, 5_900.0);
    h.app.decks.decks[0].toggle_play();
    h.frame_with(vec![], Default::default());
    for _ in 0..12 {
        h.frame_with(vec![], Default::default());
    }
    let p = h.deck(0).position_ms;
    assert!(
        (4_000.0..4_300.0).contains(&p),
        "wrapped into the loop: {p}"
    );
    // OUT on an active loop exits it; playback continues past 6 s.
    h.click(loops[1].center());
    assert_eq!(h.deck(0).loop_range, None);
    h.app.decks.decks[0].toggle_play();
    h.settle();
    // SYNC and key lock are live controls (D3).
    let out = h.settle();
    for name in ["Deck A sync", "Deck B key lock"] {
        assert!(!node(&out, name).is_disabled(), "{name}");
    }
    assert_eq!(node(&out, "Deck A tempo").role(), accesskit::Role::Slider);
    // Seam readout appears when both decks have a grid.
    assert!(h.layout().readout.is_some());
    assert!(
        texts(&out)
            .iter()
            .any(|(s, _)| s.starts_with("Δ ") && s.contains("BPM"))
    );
    h.app.decks.decks[1].track = None;
    h.settle();
    assert!(
        h.layout().readout.is_none(),
        "no readout without both grids"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// CDJ CUE through the real button: paused off the cue point a click sets
/// it; holding CUE on the cue point previews, releasing returns; PLAY
/// during the hold keeps playing.
#[test]
fn cue_button_sets_previews_while_held_and_returns() {
    let (root, mut h) = fresh("cue", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    let cue = h.layout().cue[0].unwrap().center();
    h.seek(0, 3_000.0);
    h.click(cue);
    assert_eq!(h.deck(0).cue_ms, 3_000.0, "set at stop");
    assert!(!h.deck(0).playing);
    // Hold: press, then 30 frames with the button down (0.5 s preview).
    let ev = |pressed| egui::Event::PointerButton {
        pos: cue,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    h.frame_with(
        vec![egui::Event::PointerMoved(cue), ev(true)],
        Default::default(),
    );
    for _ in 0..30 {
        h.frame_with(vec![], Default::default());
    }
    assert!(
        h.deck(0).playing && h.deck(0).previewing,
        "previewing while held"
    );
    assert!(h.deck(0).position_ms > 3_400.0, "{}", h.deck(0).position_ms);
    h.frame_with(vec![ev(false)], Default::default());
    h.settle();
    assert!(!h.deck(0).playing && !h.deck(0).previewing);
    assert_eq!(
        h.deck(0).position_ms,
        3_000.0,
        "released: back to the cue point"
    );
    // Q held + Space: PLAY during the preview keeps playing after release.
    let key = |key, pressed| egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Default::default(),
    };
    h.frame_with(vec![key(egui::Key::Q, true)], Default::default());
    h.frame_with(vec![], Default::default());
    assert!(h.deck(0).previewing);
    h.frame_with(vec![key(egui::Key::Space, true)], Default::default());
    h.frame_with(vec![key(egui::Key::Space, false)], Default::default());
    h.frame_with(vec![key(egui::Key::Q, false)], Default::default());
    h.settle();
    assert!(
        h.deck(0).playing && !h.deck(0).previewing,
        "PLAY latched the preview"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Hover `pos` and send one wheel notch of `y` lines through the real input
/// path (raw-input hook, then `ui`).
fn wheel_at(h: &mut Harness, pos: Pos2, y: f32) {
    h.frame_with(vec![egui::Event::PointerMoved(pos)], Default::default());
    h.frame_with(
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, y),
            modifiers: Default::default(),
        }],
        Default::default(),
    );
}

/// The deck tempo fader takes the hover wheel in both directions (M8/D3):
/// wheel up raises the tempo, wheel down lowers it, through the real UI.
#[test]
fn tempo_wheel_adjusts_both_decks_in_both_directions() {
    let (root, mut h) = fresh("tempo-wheel-both", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.settle();
    for (d, letter) in [(0, "A"), (1, "B")] {
        let p = h.layout().tempo[d].unwrap().center();
        let before = h.deck(d).tempo_pct;
        wheel_at(&mut h, p, 1.0);
        h.settle();
        assert!(
            h.deck(d).tempo_pct > before,
            "deck {letter}: wheel up {before} -> {}",
            h.deck(d).tempo_pct
        );
        let up = h.deck(d).tempo_pct;
        wheel_at(&mut h, p, -1.0);
        h.settle();
        assert!(
            h.deck(d).tempo_pct < up,
            "deck {letter}: wheel down {up} -> {}",
            h.deck(d).tempo_pct
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tempo_wheel_accumulates_fractional_point_events() {
    let (root, mut h) = fresh("tempo-wheel-fractional", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let p = h.layout().tempo[0].unwrap().center();
    h.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
    for _ in 0..10 {
        h.frame_with(
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 4.0),
                modifiers: Default::default(),
            }],
            Default::default(),
        );
    }
    assert!(
        (h.deck(0).tempo_pct - 0.1).abs() < 1e-4,
        "fractional events must accumulate: {}",
        h.deck(0).tempo_pct
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tempo_wheel_reverses_immediately_at_both_limits() {
    let (root, mut h) = fresh("tempo-wheel-reversal", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let range = crate::decks::tempo_range(h.app.settings.tempo_range);
    let p = h.layout().tempo[0].unwrap().center();
    for sign in [1.0, -1.0] {
        h.app.decks.decks[0].tempo_pct = sign * range;
        h.settle();
        h.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
        for delta in [sign * 36.0, -sign * 40.0] {
            h.frame_with(
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, delta),
                    modifiers: Default::default(),
                }],
                Default::default(),
            );
        }
        assert!(
            (h.deck(0).tempo_pct - sign * (range - 0.1)).abs() < 1e-4,
            "outward remainder delayed reversal: {}",
            h.deck(0).tempo_pct
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tempo_wheel_fractional_state_is_scoped_and_sync_waits_for_step() {
    let (root, mut h) = fresh("tempo-wheel-fractional-scope", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.settle();
    h.click(h.layout().sync[1].unwrap().center());
    assert!(h.deck(1).sync);
    let a = h.layout().tempo[0].unwrap().center();
    let b = h.layout().tempo[1].unwrap().center();
    let wheel = |h: &mut Harness, delta| {
        h.frame_with(
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, delta),
                modifiers: Default::default(),
            }],
            Default::default(),
        );
    };
    h.frame_with(vec![egui::Event::PointerMoved(a)], Default::default());
    wheel(&mut h, -20.0);
    assert_eq!(h.deck(0).tempo_pct, 0.0);
    h.frame_with(vec![egui::Event::PointerMoved(b)], Default::default());
    wheel(&mut h, -20.0);
    assert!(h.deck(1).sync, "a half-step must not release sync");
    wheel(&mut h, -20.0);
    assert!(!h.deck(1).sync, "a full step releases sync");
    h.frame_with(vec![egui::Event::PointerMoved(a)], Default::default());
    wheel(&mut h, -20.0);
    assert_eq!(h.deck(0).tempo_pct, 0.0, "leaving clears deck A remainder");
    wheel(&mut h, -20.0);
    assert!((h.deck(0).tempo_pct + 0.1).abs() < 1e-4);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tempo_wheel_discards_pending_input_when_controls_are_hidden() {
    let (root, mut h) = fresh("tempo-wheel-hidden", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let p = h.layout().tempo[0].unwrap().center();
    let half = || egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 20.0),
        modifiers: Default::default(),
    };
    h.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
    h.frame_with(vec![half()], Default::default());
    h.app.settings.sections.strip = false;
    h.settle();
    h.frame_with(
        vec![egui::Event::PointerMoved(egui::pos2(1.0, 1.0))],
        Default::default(),
    );
    h.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
    h.app.settings.sections.strip = true;
    h.settle();
    h.frame_with(vec![half()], Default::default());
    assert_eq!(
        h.deck(0).tempo_pct,
        0.0,
        "hidden controls must clear old wheel input"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Wheel past either end of the Settings tempo range clamps instead of
/// running off it, in both directions.
#[test]
fn tempo_wheel_clamps_at_both_range_limits() {
    let (root, mut h) = fresh("tempo-wheel-bounds", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let range = crate::decks::tempo_range(h.app.settings.tempo_range);
    let p = h.layout().tempo[0].unwrap().center();
    wheel_at(&mut h, p, 100.0);
    h.settle();
    assert!(
        (h.deck(0).tempo_pct - range).abs() < 1e-4,
        "upper bound {} vs {range}",
        h.deck(0).tempo_pct
    );
    wheel_at(&mut h, p, -200.0);
    h.settle();
    assert!(
        (h.deck(0).tempo_pct + range).abs() < 1e-4,
        "lower bound {} vs -{range}",
        h.deck(0).tempo_pct
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Wheel away from the fader changes nothing, and a hovered fader consumes
/// the wheel so the deck area does not scroll.
#[test]
fn tempo_wheel_ignores_outside_input_and_consumes_parent_scroll() {
    let (root, mut h) = fresh("tempo-wheel-outside", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let before = h.deck(0).tempo_pct;
    wheel_at(&mut h, egui::pos2(10.0, 10.0), 1.0);
    h.settle();
    assert_eq!(
        h.deck(0).tempo_pct,
        before,
        "wheel away from the fader changes nothing"
    );
    // A short window puts the deck area in its scroll area.
    h.app.settings.sections.mixer = true;
    h.viewport = egui::vec2(640.0, 480.0);
    h.settle();
    let area = h.layout().area.unwrap();
    let mixer_before = h.layout().mixer.unwrap().top();
    // Hovered fader: the wheel adjusts the tempo and does not scroll.
    let p = h.layout().tempo[0].unwrap().center();
    let tempo_before = h.deck(0).tempo_pct;
    wheel_at(&mut h, p, 1.0);
    h.settle();
    assert!(
        h.deck(0).tempo_pct > tempo_before,
        "wheel reaches the tempo fader"
    );
    assert_eq!(
        h.layout().mixer.unwrap().top(),
        mixer_before,
        "hovered fader consumes the wheel"
    );
    // Away from the fader: the same wheel still scrolls the deck area.
    wheel_at(&mut h, area.left_top() + egui::vec2(5.0, 5.0), -100.0);
    for _ in 0..8 {
        h.settle();
    }
    assert!(
        h.layout().mixer.unwrap().top() < mixer_before,
        "wheel over a non-control still scrolls the deck area"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A wheel adjustment takes the tempo back from SYNC, like the pointer fader.
#[test]
fn tempo_wheel_disengages_sync() {
    let (root, mut h) = fresh("tempo-wheel-sync", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.settle();
    h.click(h.layout().sync[1].unwrap().center());
    assert!(h.deck(1).sync, "fixture enables sync");
    // A at 0 %: B follows A's 120 BPM, so its fader reads 120/124 - 1.
    let synced_pct = (120.0_f64 / 124.0 - 1.0) * 100.0;
    let fader_while_synced = (synced_pct * 10.0).round() / 10.0;
    let p = h.layout().tempo[1].unwrap().center();
    wheel_at(&mut h, p, 1.0);
    h.settle();
    assert!(!h.deck(1).sync, "manual tempo adjustment disengages sync");
    assert!(
        (h.deck(1).tempo_pct - (fader_while_synced as f32 + 0.1)).abs() < 1e-4,
        "the fader keeps the synced tempo plus one wheel step: {} vs {}",
        h.deck(1).tempo_pct,
        fader_while_synced + 0.1
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The wheel path leaves the fader a native control: named slider with its
/// range and step, AccessKit increment/decrement, and Tab focus with arrows.
#[test]
fn tempo_wheel_preserves_accessibility_and_keyboard() {
    let (root, mut h) = fresh("tempo-wheel-keys", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.settle();
    let range = crate::decks::tempo_range(h.app.settings.tempo_range);
    let out = h.settle();
    let (id, n) = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label().as_deref() == Some("Deck A tempo"))
        .expect("Deck A tempo in the AccessKit tree");
    let id = *id;
    assert_eq!(n.role(), accesskit::Role::Slider);
    assert_eq!(n.min_numeric_value(), Some(f64::from(-range)));
    assert_eq!(n.max_numeric_value(), Some(f64::from(range)));
    assert_eq!(n.numeric_value_step(), Some(0.1));
    let send = |h: &mut Harness, action, data| {
        h.frame_with(
            vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    action,
                    target: id,
                    data,
                },
            )],
            Default::default(),
        );
    };
    send(&mut h, accesskit::Action::Increment, None);
    assert!(
        (h.deck(0).tempo_pct - 0.1).abs() < 1e-5,
        "AccessKit increment: {}",
        h.deck(0).tempo_pct
    );
    send(&mut h, accesskit::Action::Decrement, None);
    assert!(
        h.deck(0).tempo_pct.abs() < 1e-5,
        "AccessKit decrement: {}",
        h.deck(0).tempo_pct
    );
    // Tab into the controls and walk to the fader; arrows move it.
    let p = h.layout().tempo[0].unwrap().center();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    h.key(egui::Key::Tab, shift);
    let mut guard = 0;
    while !h.focused_rect().is_some_and(|r| r.contains(p)) {
        assert!(guard < 200, "Deck A tempo Tab reachable");
        h.key(egui::Key::Tab, shift);
        guard += 1;
    }
    // Vertical fader, CDJ-style: Down raises the tempo, Up lowers it.
    h.tap(egui::Key::ArrowDown);
    assert!(
        (h.deck(0).tempo_pct - 0.1).abs() < 1e-5,
        "ArrowDown: {}",
        h.deck(0).tempo_pct
    );
    h.tap(egui::Key::ArrowUp);
    assert!(
        h.deck(0).tempo_pct.abs() < 1e-5,
        "ArrowUp: {}",
        h.deck(0).tempo_pct
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The playhead is the engine's sample position, not the frame clock: a
/// long UI gap without rendered audio moves nothing; rendered audio moves
/// it exactly; the end of the track stops it exactly.
#[test]
fn playhead_follows_the_engine_not_wall_clock() {
    let (root, mut h) = fresh("clock", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.tap(egui::Key::Space);
    assert!(h.deck(0).playing);
    h.app.decks.pump();
    let start = h.deck(0).position_ms;
    // 2 s of UI time pass in one frame, but the device rendered 1/60 s.
    h.time += 2.0;
    h.frame_with(vec![], Default::default());
    h.app.decks.pump();
    let moved = h.deck(0).position_ms - start;
    assert!(
        (moved - 1000.0 / 60.0).abs() < 1e-6,
        "one block, not the 2 s gap: {moved}"
    );
    // One rendered second advances exactly one second.
    let start = h.deck(0).position_ms;
    let _ = h.app.decks.render(48_000);
    h.app.decks.pump();
    assert!((h.deck(0).position_ms - start - 1000.0).abs() < 1e-6);
    // Rendering past the end stops exactly at the track end.
    h.seek(0, 19_000.0);
    let _ = h.app.decks.render(48_000 * 3);
    h.settle();
    assert!(!h.deck(0).playing);
    assert!(
        (h.deck(0).position_ms - h.deck(0).duration()).abs() < 0.05,
        "{} vs {}",
        h.deck(0).position_ms,
        h.deck(0).duration()
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Mixer volume reaches the engine only while the mixer is visible; a
/// hidden mixer is exact unity (D4: bit-identical bypass). A decode failure is shown on its deck and
/// leaves it inert; no device = a visible NO AUDIO status, decks still run.
#[test]
fn volume_unity_bypass_decode_failure_and_silent_fallback() {
    let (root, mut h) = fresh("vol", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    // 100 ms of deck A from 4 s, as the device would receive it.
    let block = |h: &mut Harness| {
        let mixer = h.app.settings.sections.mixer;
        h.app.decks.decks[0].seek(4_000.0);
        h.app.decks.decks[0].toggle_play();
        h.app.decks.flush(mixer);
        let out = h.app.decks.render(4_800);
        h.app.decks.decks[0].toggle_play();
        h.app.decks.flush(mixer);
        h.app.decks.render(1);
        h.settle();
        out
    };
    let unity = block(&mut h);
    assert!(unity.iter().any(|s| s.abs() > 0.1), "deck A is audible");
    h.app.decks.decks[0].volume.0 = 0.25;
    h.settle();
    assert_eq!(
        block(&mut h),
        unity,
        "mixer hidden: exact unity whatever the fader says"
    );
    h.app.settings.sections.mixer = true;
    h.settle();
    assert!(h.layout().volume[0].is_some() && h.layout().volume[1].is_some());
    let quiet = block(&mut h);
    let rms =
        |v: &[f32]| (v.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
    // D4: the channel goes through the isolator EQ (flat all-pass at 0 dB),
    // so the fader scales the level, not each sample bit for bit.
    let ratio = rms(&quiet[2_400..]) / rms(&unity[2_400..]);
    assert!(
        (ratio - 0.25).abs() < 0.01,
        "volume 0.25 scales the output: {ratio}"
    );
    let out = h.settle();
    assert_eq!(node(&out, "Deck A volume").role(), accesskit::Role::Slider);
    h.app.settings.sections.mixer = false;
    h.settle();
    assert_eq!(block(&mut h), unity);
    // Decode failure: the file vanished between import and load.
    let path = h
        .app
        .tracks
        .iter()
        .find(|t| t.track.path.ends_with("b-124.wav"))
        .unwrap()
        .track
        .path
        .clone();
    std::fs::remove_file(&path).unwrap();
    h.select_and_load("b-124.wav", egui::Key::B);
    assert!(matches!(&h.deck(1).audio, DeckAudio::Failed(r) if r.contains("unreadable")));
    let out = h.settle();
    assert!(
        texts(&out)
            .iter()
            .any(|(t, _)| t.contains("cannot play: file missing or unreadable"))
    );
    h.app.decks.decks[1].toggle_play();
    h.settle();
    assert!(!h.deck(1).playing && !h.deck(1).ready());
    assert!(h.deck(0).ready(), "deck A unaffected");
    // No device: the silent output keeps the decks running, visibly.
    h.app
        .decks
        .attach(oxidj_audio::Engine::silent("no audio output device"));
    h.app.decks.decks[0].audio = DeckAudio::None;
    h.app.selected = Some(h.id_of("a-120.wav"));
    h.tap(egui::Key::A);
    h.wait_audio();
    h.tap(egui::Key::Space);
    let start = std::time::Instant::now();
    while h.deck(0).position_ms < 100.0 {
        assert!(
            start.elapsed().as_secs() < 10,
            "silent clock moves the deck"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.frame_with(vec![], Default::default());
    }
    let out = h.settle();
    assert!(
        texts(&out)
            .iter()
            .any(|(t, _)| t == "NO AUDIO · no audio output device · decks run silently")
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Sol D2 round 1: replacing a playing deck stops and unloads its old
/// audio at once, so a failed decode never leaves hidden, uncontrollable
/// sound; the other deck plays on untouched.
#[test]
fn replacing_a_playing_deck_unloads_old_audio_on_success_and_failure() {
    let (root, mut h) = fresh("replace", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let (a_id, b_id, c_id) = (
        h.id_of("a-120.wav"),
        h.id_of("b-124.wav"),
        h.id_of("c-nogrid.wav"),
    );
    h.app.decks.active = 0;
    h.tap(egui::Key::Space);
    h.app.decks.active = 1;
    h.tap(egui::Key::Space);
    assert!(h.app.decks.engine.deck(0).playing && h.app.decks.engine.deck(1).playing);
    assert_eq!(h.app.decks.engine.deck(0).id, Some(a_id));
    let b_before = h.app.decks.engine.deck(1).frame;
    // Success: c-nogrid replaces a-120 on A while it plays.
    h.app.selected = Some(c_id);
    h.tap(egui::Key::A);
    let a = h.app.decks.engine.deck(0);
    assert!(
        a.id != Some(a_id) && !a.playing,
        "old audio unloaded at once: {a:?}"
    );
    h.wait_audio();
    let a = h.app.decks.engine.deck(0);
    assert_eq!(
        (a.id, a.playing),
        (Some(c_id), false),
        "replacement loaded, stopped"
    );
    assert!(h.deck(0).ready() && !h.deck(0).playing);
    let b = h.app.decks.engine.deck(1);
    assert!(
        b.playing && b.id == Some(b_id) && b.frame > b_before,
        "deck B plays on"
    );
    // Failure: A plays again, then an unreadable file replaces it.
    h.app.decks.active = 0;
    h.tap(egui::Key::Space);
    assert!(h.app.decks.engine.deck(0).playing);
    let path = h
        .app
        .tracks
        .iter()
        .find(|t| t.track.id.as_u64() == a_id)
        .unwrap()
        .track
        .path
        .clone();
    std::fs::remove_file(&path).unwrap();
    h.select_and_load("a-120.wav", egui::Key::A);
    assert!(matches!(&h.deck(0).audio, DeckAudio::Failed(_)));
    let a = h.app.decks.engine.deck(0);
    assert!(
        a.id.is_none() && !a.playing,
        "no hidden audio after a failed load: {a:?}"
    );
    assert!(!h.deck(0).playing && !h.deck(0).ready());
    let b = h.app.decks.engine.deck(1);
    assert!(
        b.playing && b.id == Some(b_id),
        "deck B unaffected by A's failure"
    );
    // With B paused, the output is silent: nothing left of A.
    h.app.decks.active = 1;
    h.tap(egui::Key::Space);
    assert!(!h.app.decks.engine.deck(1).playing);
    assert!(
        h.app.decks.render(4_800).iter().all(|s| *s == 0.0),
        "output silent"
    );
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
    let oa_inner = Rect::from_min_max(
        Pos2::new(oa.left() + decks::OVERVIEW_INSET, oa.top()),
        oa.max,
    );
    let t = texts(&out);
    for (clip, what) in [(a, "detail"), (oa, "overview")] {
        for n in ["1", "2", "3"] {
            assert!(
                t.iter().any(|(s, r)| s == n && clip.contains_rect(*r)),
                "{what}: hot cue {n} number"
            );
        }
        // Hot cue 2's stored colour paints its marker.
        let green = egui::Color32::from_rgb(0x28, 0xe2, 0x14);
        assert!(
            out.shapes
                .iter()
                .any(|c| c.clip_rect == clip
                    && matches!(&c.shape, Shape::Rect(r) if r.fill == green)),
            "{what}: stored colour"
        );
        // Two memory cues: triangles.
        let triangles = out
            .shapes
            .iter()
            .filter(|c| {
                c.clip_rect == clip && matches!(&c.shape, Shape::Path(p) if p.points.len() == 3)
            })
            .count();
        assert!(triangles >= 2, "{what}: memory cues {triangles}");
        // Loop 8–10 s shaded (translucent fill).
        assert!(
            out.shapes
                .iter()
                .any(|c| c.clip_rect == clip
                    && matches!(&c.shape, Shape::Rect(r) if r.fill.a() == 46)),
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
        meshes_in(&out, a)
            .iter()
            .filter(|(_, c)| c.a() == 255)
            .map(|(p, _)| a.bottom() - p.y)
            .fold(0.0, f32::max)
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
        assert!(
            screen.contains_rect(h.app.shell.decks.unwrap()),
            "{bits:04b}"
        );
        assert_eq!(l.detail.is_some(), sections.detail, "{bits:04b}");
        assert_eq!(l.overviews.is_some(), sections.overviews, "{bits:04b}");
        assert_eq!(l.strip.is_some(), sections.strip, "{bits:04b}");
        assert_eq!(l.mixer.is_some(), sections.mixer, "{bits:04b}");
        // Stacking order: toggles, detail, strip, mixer, overviews; no overlap.
        let order: Vec<Rect> = [
            l.toggles.map(|t| t[0]),
            l.detail,
            l.strip,
            l.mixer,
            l.overviews.map(|o| o[0].union(o[1])),
        ]
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
            let want = if sections.get(i) {
                accesskit::Toggled::True
            } else {
                accesskit::Toggled::False
            };
            assert_eq!(n.toggled(), Some(want), "{bits:04b}: {name}");
        }
        heights.insert(bits, h.app.shell.table.unwrap().height());
    }
    // Collapsing frees space for the library: every expanded section costs height.
    for bits in 0..16u8 {
        for i in 0..4 {
            if bits & (1 << i) != 0 {
                assert!(
                    heights[&(bits & !(1 << i))] > heights[&bits] + 10.0,
                    "{bits:04b} minus section {i}"
                );
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
    let chord = egui::Modifiers {
        ctrl: true,
        command: true,
        shift: true,
        ..Default::default()
    };
    h.key(egui::Key::Num1, chord);
    assert!(!h.app.settings.sections.detail && h.layout().detail.is_none());
    // Shift+Tab enters the controls; walk to the Detail toggle and press Space.
    let toggle = h.layout().toggles.unwrap()[0];
    assert!(h.ctx.memory(|m| m.focused()).is_none());
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    h.key(egui::Key::Tab, shift);
    let mut guard = 0;
    while h.focused_rect() != Some(toggle) {
        h.key(egui::Key::Tab, shift);
        guard += 1;
        assert!(guard < 200, "Detail toggle reachable by keyboard");
    }
    h.tap(egui::Key::Space);
    assert!(
        h.app.settings.sections.detail,
        "Space on the focused toggle shows the detail view"
    );
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
    assert_eq!(
        Settings::load(Some(&store)).sections,
        DeckSections::from_bits(0b1010)
    );
    // Ctrl+D still hides the whole deck area.
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    h.key(egui::Key::D, ctrl);
    assert!(!h.app.show_decks && h.app.shell.decks.is_none() && h.layout().area.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// Not a behaviour test: writes the synthetic capture catalog used for the
/// v0.14 Bot Screen captures into `$OXIDJ_D1_CAPTURE_DIR`.
#[test]
#[ignore]
fn seed_capture_catalog() {
    let Ok(dir) = std::env::var("OXIDJ_D1_CAPTURE_DIR") else {
        return;
    };
    let db = deck_catalog(Path::new(&dir), 240.0);
    println!("{}", db.display());
}

// ------------------------------------------------------------------ D3

/// Real-time phase difference (ms) of the two decks from the engine's
/// published positions and each track's constant grid fit.
fn engine_phase_error_ms(h: &Harness) -> f64 {
    let mut phase = [0.0; 2];
    let mut beat_real = [0.0; 2];
    for (d, rate) in [(0, 44_100.0), (1, 44_100.0)] {
        let fit = h.deck(d).track.as_ref().unwrap().fit.unwrap();
        let ms = h.app.decks.engine.deck(d).frame * 1000.0 / rate;
        phase[d] = fit.phase(ms);
        beat_real[d] = fit.beat_ms / h.deck(d).speed();
    }
    assert!(
        (beat_real[0] - beat_real[1]).abs() < 1e-6,
        "tempo matched: {beat_real:?}"
    );
    ((phase[0] - phase[1] + 0.5).rem_euclid(1.0) - 0.5).abs() * beat_real[0]
}

/// The constant fit recovers sub-millisecond tempo and first beat from a
/// grid of whole-millisecond beat times, and flags a tempo change.
#[test]
fn grid_fit_is_sub_millisecond_and_detects_variable_grids() {
    let g = crate::beat_grid::BeatGrid::new(&grid(124.0, 60.0, 120.0), 60_000.0).unwrap();
    let fit = decks::GridFit::of(&g).unwrap();
    assert!((fit.beat_ms - 60_000.0 / 124.0).abs() < 0.001, "{fit:?}");
    assert!((fit.anchor_ms - 120.0).abs() < 0.3, "{fit:?}");
    assert!(fit.constant() && fit.residual_ms <= 0.5);
    let mut beats = grid(120.0, 20.0, 0.0);
    let shift = beats.last().unwrap().time_ms;
    beats.extend(grid(128.0, 20.0, 0.0).into_iter().skip(1).map(|mut b| {
        b.time_ms += shift;
        b
    }));
    let g = crate::beat_grid::BeatGrid::new(&beats, 60_000.0).unwrap();
    let fit = decks::GridFit::of(&g).unwrap();
    assert!(
        !fit.constant(),
        "tempo change must not fit one tempo: {fit:?}"
    );
}

/// KEY and SYNC through the real buttons: key lock reaches the engine,
/// SYNC matches B's tempo to A's (track BPM × fader) and aligns B to A's
/// beat phase within 1 ms, the BPM readouts agree, the seam readout shows
/// the live offset, B follows A's fader, and moving B's fader releases it.
#[test]
fn key_lock_and_sync_buttons_match_tempo_and_phase() {
    let (root, mut h) = fresh("sync", 30.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let (key_b, sync_b) = (h.layout().key_lock[1].unwrap(), h.layout().sync[1].unwrap());
    h.click(key_b.center());
    assert!(
        h.deck(1).keylock && h.app.decks.engine.deck(1).keylock,
        "key lock reached the engine"
    );
    let out = h.settle();
    assert_eq!(
        node(&out, "Deck B key lock").toggled(),
        Some(accesskit::Toggled::True)
    );
    // A at +2 %, both playing, B off-phase.
    h.app.decks.decks[0].tempo_pct = 2.0;
    h.seek(1, 5_000.0);
    h.app.decks.decks[0].toggle_play();
    h.app.decks.decks[1].toggle_play();
    for _ in 0..20 {
        h.frame_with(vec![], Default::default());
    }
    let before = engine_phase_error_ms_unmatched(&h);
    assert!(before > 20.0, "starts out of phase: {before} ms");
    h.click(sync_b.center());
    assert!(
        h.deck(1).sync && h.deck(1).sync_note.is_none(),
        "{:?} {:?}",
        h.deck(1).sync_note,
        h.deck(1).track.as_ref().map(|t| t.fit)
    );
    let want = 120.0 * 1.02 / 124.0;
    assert!(
        (h.deck(1).speed() - want).abs() < 1e-5,
        "B speed {}",
        h.deck(1).speed()
    );
    for _ in 0..30 {
        h.frame_with(vec![], Default::default());
    }
    let err = engine_phase_error_ms(&h);
    assert!(err < 1.0, "phase error after SYNC: {err} ms");
    let out = h.settle();
    assert_eq!(
        node(&out, "Deck B sync").toggled(),
        Some(accesskit::Toggled::True)
    );
    // Warped beat lines: each half's window is scaled by its deck's speed,
    // so after SYNC every beat line of A has a B line at the same x (< 1 px).
    let halves = h.layout().halves.unwrap();
    let mut xs = [vec![], vec![]];
    for d in 0..2 {
        let window = h.layout().windows[d].unwrap();
        let grid = h.deck(d).track.as_ref().unwrap().beat_grid().unwrap();
        xs[d] = grid
            .beats_in(window)
            .iter()
            .map(|(t, _)| crate::beat_grid::x_of(*t, window, halves[d]))
            .collect();
    }
    assert!(xs[0].len() >= 8, "lines in view: {:?}", xs[0]);
    for x in &xs[0] {
        let nearest = xs[1]
            .iter()
            .map(|b| (b - x).abs())
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 1.0 || *x < halves[0].left() + 2.0 || *x > halves[0].right() - 2.0,
            "A line at {x} has no B line (nearest {nearest} px)"
        );
    }
    let t = texts(&out);
    assert_eq!(
        t.iter().filter(|(s, _)| s == "122.4").count(),
        2,
        "both decks read 120 × 1.02"
    );
    assert!(
        t.iter().any(|(s, _)| s == "-1.3%"),
        "B's fader shows the synced pitch"
    );
    let readout: Vec<&String> = t
        .iter()
        .map(|(s, _)| s)
        .filter(|s| s.starts_with('Δ'))
        .collect();
    assert!(
        readout
            .iter()
            .any(|s| (s.starts_with("Δ +0 ms") || s.starts_with("Δ -0 ms"))
                && s.ends_with("0.00 BPM")),
        "live seam readout shows the synced offset: {readout:?}"
    );
    // A's fader moves: B follows its tempo.
    h.app.decks.decks[0].tempo_pct = -1.0;
    h.settle();
    assert!((h.deck(1).speed() - 120.0 * 0.99 / 124.0).abs() < 1e-5);
    // Dragging B's fader takes the tempo back from SYNC.
    let fader = h.layout().tempo[1].unwrap();
    h.drag(fader.center(), fader.center() + egui::vec2(10.0, 0.0));
    assert!(!h.deck(1).sync);
    let out = h.settle();
    assert_eq!(
        node(&out, "Deck B sync").toggled(),
        Some(accesskit::Toggled::False)
    );
    // Double-click resets the fader to 0 %.
    let at = fader.center() + egui::vec2(20.0, 0.0);
    let ev = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut trace = vec![];
    h.frame_with(
        vec![egui::Event::PointerMoved(at), ev(true)],
        Default::default(),
    );
    trace.push(h.deck(1).tempo_pct);
    h.frame_with(vec![ev(false)], Default::default());
    trace.push(h.deck(1).tempo_pct);
    h.frame_with(vec![ev(true)], Default::default());
    trace.push(h.deck(1).tempo_pct);
    h.frame_with(vec![ev(false)], Default::default());
    trace.push(h.deck(1).tempo_pct);
    h.settle();
    trace.push(h.deck(1).tempo_pct);
    assert_eq!(h.deck(1).tempo_pct, 0.0, "double-click resets: {trace:?}");
    // Key lock off again.
    h.click(key_b.center());
    assert!(!h.app.decks.engine.deck(1).keylock);
    std::fs::remove_dir_all(root).unwrap();
}

/// Phase difference (ms of A's beat) without assuming matched tempo.
fn engine_phase_error_ms_unmatched(h: &Harness) -> f64 {
    let mut phase = [0.0; 2];
    for d in 0..2 {
        let fit = h.deck(d).track.as_ref().unwrap().fit.unwrap();
        phase[d] = fit.phase(h.app.decks.engine.deck(d).frame * 1000.0 / 44_100.0);
    }
    ((phase[0] - phase[1] + 0.5).rem_euclid(1.0) - 0.5).abs() * 500.0
}

/// SYNC refuses, visibly and with the exact reason, when a deck is empty,
/// has no grid, a low-confidence computed grid, a variable-tempo grid, or
/// would need a tempo beyond the fader range; nothing is sent then.
#[test]
fn sync_refuses_with_exact_visible_reasons() {
    let (root, mut h) = fresh("sync-refuse", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    let sync_a = h.layout().sync[0].unwrap();
    let refused = |h: &mut Harness, want: &str| {
        let sent = h.app.decks.engine.deck(0).applied;
        h.click(sync_a.center());
        assert!(!h.deck(0).sync, "{want}");
        assert_eq!(h.deck(0).sync_note.as_deref(), Some(want));
        let out = h.settle();
        assert!(
            texts(&out).iter().any(|(s, _)| s.ends_with(want)),
            "{want} is shown"
        );
        assert_eq!(
            node(&out, "Deck A sync").toggled(),
            Some(accesskit::Toggled::False)
        );
        assert_eq!(
            h.app.decks.engine.deck(0).applied,
            sent,
            "{want}: nothing sent"
        );
    };
    refused(&mut h, "SYNC: deck B is empty");
    h.select_and_load("c-nogrid.wav", egui::Key::B);
    refused(&mut h, "SYNC: deck B has no beat grid");
    h.select_and_load("b-124.wav", egui::Key::B);
    // A computed grid below the confidence threshold.
    h.app.decks.decks[1].track.as_mut().unwrap().confidence = Some(0.3);
    refused(&mut h, "SYNC: deck B grid confidence is low (0.30)");
    h.app.decks.decks[1].track.as_mut().unwrap().confidence = None;
    // A tempo change in deck A's grid.
    let mut beats = grid(120.0, 10.0, 0.0);
    let shift = beats.last().unwrap().time_ms;
    beats.extend(grid(128.0, 10.0, 0.0).into_iter().skip(1).map(|mut b| {
        b.time_ms += shift;
        b
    }));
    let variable = crate::beat_grid::BeatGrid::new(&beats, 20_000.0).unwrap();
    let a = h.app.decks.decks[0].track.as_mut().unwrap();
    let saved = (a.grid.clone(), a.fit);
    a.fit = decks::GridFit::of(&variable);
    a.grid = Some((variable, crate::track_waveform::GridSource::Library));
    refused(&mut h, "SYNC: deck A has a variable-tempo grid");
    let a = h.app.decks.decks[0].track.as_mut().unwrap();
    (a.grid, a.fit) = saved;
    // ±6 %: B at -6 % → A needs 124 × 0.94 / 120 = -2.9 % (fine), but with
    // B at +6 % → A needs +9.5 %.
    h.app.settings.tempo_range = crate::settings::TempoRange::Six;
    h.app.decks.decks[1].tempo_pct = 6.0;
    refused(&mut h, "SYNC: needs +9.5 %, beyond the ±6 % tempo range");
    // In range: accepted, and the note clears.
    h.app.decks.decks[1].tempo_pct = -6.0;
    h.click(sync_a.center());
    assert!(h.deck(0).sync && h.deck(0).sync_note.is_none());
    assert!((h.deck(0).speed() - 124.0 * 0.94 / 120.0).abs() < 1e-5);
    // Pressing again releases SYNC; the fader keeps the tempo.
    h.click(sync_a.center());
    assert!(!h.deck(0).sync);
    assert!((h.deck(0).speed() - 124.0 * 0.94 / 120.0).abs() < 1e-6);
    std::fs::remove_dir_all(root).unwrap();
}

// -------------------------------------------------------------------- D4

/// 100 ms of both decks from 4 s, as the device would receive them, with
/// the mixer state the production frame sent.
fn mix_block(h: &mut Harness) -> Vec<f32> {
    for d in 0..2 {
        h.app.decks.decks[d].seek(4_000.0);
        h.app.decks.decks[d].toggle_play();
    }
    h.settle();
    let out = h.app.decks.render(4_800);
    for d in 0..2 {
        h.app.decks.decks[d].toggle_play();
    }
    h.settle();
    out
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|s| s.to_bits()).collect()
}

/// D4 acceptance 4: the mixer is hidden by default and then exactly out of
/// the path whatever its knobs say; shown, its knobs change the sound;
/// hidden again (by its own toggle), bit-identical once more.
#[test]
fn hidden_mixer_is_bit_identical_whatever_the_knobs_say() {
    let (root, mut h) = fresh("mixbypass", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    assert!(!h.app.settings.sections.mixer, "mixer hidden by default");
    let reference = mix_block(&mut h);
    assert!(reference.iter().any(|s| s.abs() > 0.1));
    // Knobs moved while hidden: no effect at all.
    {
        let a = &mut h.app.decks.decks[0];
        a.volume.0 = 0.2;
        a.channel.eq_db = [oxidj_audio::EQ_KILL_DB, 6.0, -10.0];
        a.channel.filter = -0.7;
        a.channel.trim_db = 9.0;
    }
    h.app.decks.crossfader = 1.0;
    h.app.decks.master = 0.3;
    assert_eq!(
        bits(&mix_block(&mut h)),
        bits(&reference),
        "hidden mixer: bit-identical"
    );
    // Shown through its section toggle: the knobs act.
    let toggle = h.layout().toggles.unwrap()[3];
    h.click(toggle.center());
    assert!(h.app.settings.sections.mixer && h.layout().mixer.is_some());
    let mixed = mix_block(&mut h);
    assert_ne!(bits(&mixed), bits(&reference));
    // Hidden again by the same toggle: exact bypass.
    let toggle = h.layout().toggles.unwrap()[3];
    h.click(toggle.center());
    assert!(!h.app.settings.sections.mixer && h.layout().mixer.is_none());
    assert_eq!(
        bits(&mix_block(&mut h)),
        bits(&reference),
        "hidden again: bit-identical"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D4 acceptance 1-3 through the production UI: every control exists with
/// its AccessKit name inside the mixer, pointer input moves it, and the
/// engine applies it (kill, crossfader, assign, meters); the crossfader's
/// own icon toggle hides it and takes it out of the path; persisted.
#[test]
fn mixer_eq_and_filter_paint_rotary_controls() {
    let (root, mut h) = fresh("mixrotary", 10.0);
    h.app.settings.sections.mixer = true;
    for value in [-1.0, 0.0, 1.0] {
        for deck in &mut h.app.decks.decks {
            deck.channel.filter = value;
            deck.channel.trim_db = value * oxidj_audio::TRIM_DB;
            deck.channel.eq_db = [crate::mixer_view::eq_db_at(value); 3];
        }
        h.app.decks.master = (value + 1.0) / 2.0;
        let out = h.settle();
        let layout = h.layout();
        for d in 0..2 {
            // Rotary controls: EQ, filter, trim (master is now a vertical fader).
            for cell in layout.eq[d]
                .unwrap()
                .into_iter()
                .chain([layout.filter[d].unwrap(), layout.trim[d].unwrap()])
            {
                let shapes: Vec<_> = out
                    .shapes
                    .iter()
                    .filter(|s| {
                        cell.contains_rect(s.clip_rect)
                            && matches!(s.shape, Shape::Circle(_) | Shape::LineSegment { .. })
                    })
                    .collect();
                assert!(
                    shapes.iter().any(|s| matches!(s.shape, Shape::Circle(_))),
                    "rotary circle missing from {cell:?}"
                );
                for shape in shapes {
                    assert!(
                        shape
                            .clip_rect
                            .contains_rect(shape.shape.visual_bounding_rect()),
                        "paint exceeds clip: {:?}",
                        shape.shape
                    );
                }
            }
            // Master is a vertical fader: check it paints a slider track, not a rotary circle.
            let master_cell = layout.master.unwrap();
            let master_shapes: Vec<_> = out
                .shapes
                .iter()
                .filter(|s| {
                    master_cell.contains_rect(s.clip_rect)
                        && matches!(s.shape, Shape::Rect(_) | Shape::LineSegment { .. })
                })
                .collect();
            assert!(
                !master_shapes.is_empty(),
                "master vertical fader paints track/rect"
            );
            for shape in master_shapes {
                assert!(
                    shape
                        .clip_rect
                        .contains_rect(shape.shape.visual_bounding_rect()),
                    "paint exceeds clip: {:?}",
                    shape.shape
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_trim_indicator_distinct_from_filter_across_presets() {
    let (root, mut h) = fresh("trim_colours", 1.0);
    h.app.settings.sections.mixer = true;
    for theme in ThemeMode::ALL {
        h.app.select_theme(theme);
        for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
            h.app.settings.deck_palette = palette;
            let out = h.settle();
            let layout = h.layout();
            let indicator = |cell: Rect| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        Shape::LineSegment { stroke, .. }
                            if cell.contains_rect(s.clip_rect) && stroke.width == 2.0 =>
                        {
                            Some(stroke.color)
                        }
                        _ => None,
                    })
                    .expect("production rotary indicator")
            };
            for d in 0..2 {
                let trim = indicator(layout.trim[d].unwrap());
                let filter = indicator(layout.filter[d].unwrap());
                assert_ne!(trim, filter, "{theme:?} {palette:?} deck {d}");
                let tokens = theme.tokens();
                assert_eq!(
                    trim,
                    if tokens.primary == filter {
                        tokens.secondary
                    } else {
                        tokens.primary
                    }
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_wheel_changes_hovered_controls_and_clamps() {
    let (root, mut h) = fresh("mixwheel", 10.0);
    h.app.settings.sections.mixer = true;
    for name in ["Deck A filter", "Deck A volume", "Crossfader"] {
        let out = h.settle();
        let b = node(&out, name).bounds().unwrap();
        let p = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
        let get = |h: &Harness| match name {
            "Deck A filter" => h.deck(0).channel.filter,
            "Deck A volume" => h.deck(0).volume.0,
            _ => h.app.decks.crossfader,
        };
        let wheel = |h: &mut Harness, y| {
            h.frame_with(
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, y),
                        modifiers: Default::default(),
                    },
                ],
                Default::default(),
            );
        };
        let initial = get(&h);
        wheel(&mut h, -1.0);
        let down = get(&h);
        assert!(down < initial, "{name}: wheel down {initial} -> {down}");
        wheel(&mut h, 1.0);
        assert!(get(&h) > down, "{name}: wheel up");
        wheel(&mut h, 100.0);
        assert_eq!(get(&h), 1.0, "{name}: upper bound");
        wheel(&mut h, -100.0);
        assert_eq!(
            get(&h),
            if name.ends_with("filter") { -1.0 } else { 0.0 },
            "{name}: lower bound"
        );
    }
    let values = (
        h.deck(0).channel.filter,
        h.deck(0).volume.0,
        h.app.decks.crossfader,
    );
    let outside = h.layout().mixer.unwrap().left_top() - egui::vec2(10.0, 10.0);
    h.frame_with(
        vec![
            egui::Event::PointerMoved(outside),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                modifiers: Default::default(),
            },
        ],
        Default::default(),
    );
    assert_eq!(
        values,
        (
            h.deck(0).channel.filter,
            h.deck(0).volume.0,
            h.app.decks.crossfader
        )
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_rotary_accesskit_value_actions() {
    let (root, mut h) = fresh("mixrotarya11y", 10.0);
    h.app.settings.sections.mixer = true;
    for name in ["Deck A EQ high", "Deck B filter"] {
        let out = h.settle();
        let (id, n) = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label().as_deref() == Some(name))
            .unwrap();
        let id = *id;
        assert_eq!(n.min_numeric_value(), Some(-1.0));
        assert_eq!(n.max_numeric_value(), Some(1.0));
        assert_eq!(n.numeric_value_step(), Some(crate::mixer_view::FADER_STEP));
        for action in [
            accesskit::Action::SetValue,
            accesskit::Action::Increment,
            accesskit::Action::Decrement,
        ] {
            assert!(n.supports_action(action), "{name} lacks {action:?}");
        }
        let send = |h: &mut Harness, action, data| {
            h.frame_with(
                vec![egui::Event::AccessKitActionRequest(
                    accesskit::ActionRequest {
                        action,
                        target: id,
                        data,
                    },
                )],
                Default::default(),
            );
        };
        let get = |h: &Harness| {
            if name.ends_with("filter") {
                h.deck(1).channel.filter
            } else {
                crate::mixer_view::eq_pos_of(h.deck(0).channel.eq_db[2])
            }
        };
        send(
            &mut h,
            accesskit::Action::SetValue,
            Some(accesskit::ActionData::NumericValue(0.5)),
        );
        assert!((get(&h) - 0.5).abs() < 1e-5);
        send(&mut h, accesskit::Action::Increment, None);
        assert!((get(&h) - 0.55).abs() < 1e-5);
        send(&mut h, accesskit::Action::Decrement, None);
        assert!((get(&h) - 0.5).abs() < 1e-5);
        send(
            &mut h,
            accesskit::Action::SetValue,
            Some(accesskit::ActionData::NumericValue(-2.0)),
        );
        assert!((get(&h) + 1.0).abs() < 1e-5);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn trim_and_master_rotaries_accessibility_input_and_audio() {
    let (root, mut h) = fresh("trim_master_rotary", 10.0);
    h.app.settings.sections.mixer = true;
    for (name, min, max, step, set) in [
        ("Deck A trim", -12.0, 12.0, 1.0, 6.0),
        ("Deck B trim", -12.0, 12.0, 1.0, 6.0),
        ("Master volume", 0.0, 1.0, 0.05, 0.5),
    ] {
        let out = h.settle();
        let (id, n) = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label().as_deref() == Some(name))
            .unwrap();
        let id = *id;
        assert_eq!(
            (
                n.min_numeric_value(),
                n.max_numeric_value(),
                n.numeric_value_step()
            ),
            (Some(min), Some(max), Some(step))
        );
        let send = |h: &mut Harness, action, data| {
            h.frame_with(
                vec![egui::Event::AccessKitActionRequest(
                    accesskit::ActionRequest {
                        action,
                        target: id,
                        data,
                    },
                )],
                Default::default(),
            );
        };
        let value = |h: &Harness| match name {
            "Deck A trim" => h.deck(0).channel.trim_db,
            "Deck B trim" => h.deck(1).channel.trim_db,
            _ => h.app.decks.master,
        };
        send(
            &mut h,
            accesskit::Action::SetValue,
            Some(accesskit::ActionData::NumericValue(set)),
        );
        assert!((f64::from(value(&h)) - set).abs() < 1e-5);
        let out = h.settle();
        let n = node(&out, name);
        for action in [
            accesskit::Action::SetValue,
            accesskit::Action::Increment,
            accesskit::Action::Decrement,
        ] {
            assert!(n.supports_action(action), "{name}: {action:?}");
        }
        send(&mut h, accesskit::Action::Increment, None);
        assert!((f64::from(value(&h)) - set - step).abs() < 1e-5);
        send(&mut h, accesskit::Action::Decrement, None);
        assert!((f64::from(value(&h)) - set).abs() < 1e-5);
        let params = h.app.decks.mixer_params();
        if name == "Master volume" {
            assert!((params.master - set as f32).abs() < 1e-5);
        } else {
            let d = usize::from(name == "Deck B trim");
            assert!((params.channels[d].trim - oxidj_audio::trim_gain(set as f32)).abs() < 1e-5);
        }
        let out = h.settle();
        let b = node(&out, name).bounds().unwrap();
        let p = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
        h.frame_with(
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -1.0),
                    modifiers: Default::default(),
                },
            ],
            Default::default(),
        );
        assert!(value(&h) < set as f32, "{name}: wheel");
        h.time += 1.0;
        h.click(p);
        h.click(p);
        assert_eq!(
            value(&h),
            if name == "Master volume" { 1.0 } else { 0.0 },
            "{name}: reset"
        );
        let cell = match name {
            "Deck A trim" => h.layout().trim[0].unwrap(),
            "Deck B trim" => h.layout().trim[1].unwrap(),
            _ => h.layout().master.unwrap(),
        };
        let mut guard = 0;
        h.key(
            egui::Key::Tab,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        while !h
            .focused_rect()
            .is_some_and(|r| cell.contains_rect(r.shrink(1.0)))
        {
            h.key(egui::Key::Tab, Default::default());
            guard += 1;
            assert!(guard < 200, "{name}: keyboard focus");
        }
        h.key(egui::Key::ArrowLeft, Default::default());
        assert!(
            value(&h) < if name == "Master volume" { 1.0 } else { 0.0 },
            "{name}: keyboard"
        );
        rotary_drag(
            &mut h,
            name,
            &[-90.0, -135.0, -180.0, -225.0, -270.0, -315.0, -360.0],
        );
        assert!(
            (f64::from(value(&h)) - min).abs() < 1e-5,
            "{name}: pointer reaches minimum"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_rotary_dead_sector_reversal_and_double_click_reset() {
    let (root, mut h) = fresh("mixrotarygestures", 10.0);
    h.app.settings.sections.mixer = true;
    h.settle();
    for (start, path, expected) in [
        (1.0, vec![45.0, 89.0, 91.0, 135.0, 160.0], 1.0),
        (-1.0, vec![135.0, 91.0, 89.0, 45.0, 20.0], -1.0),
    ] {
        h.app.decks.decks[0].channel.filter = start;
        rotary_drag(&mut h, "Deck A filter", &path);
        assert_eq!(h.deck(0).channel.filter, expected);
        let mut reverse = path.clone();
        reverse.push(if start > 0.0 { 130.0 } else { 50.0 });
        h.app.decks.decks[0].channel.filter = start;
        rotary_drag(&mut h, "Deck A filter", &reverse);
        let v = h.deck(0).channel.filter;
        assert!(
            if start > 0.0 {
                v < 1.0 && v > 0.7
            } else {
                v > -1.0 && v < -0.7
            },
            "reverse from {start}: {v}"
        );
    }
    h.app.decks.decks[0].channel.filter = 0.8;
    let out = h.settle();
    let b = node(&out, "Deck A filter").bounds().unwrap();
    let p = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    h.time += 1.0;
    h.click(p);
    h.click(p);
    assert_eq!(h.deck(0).channel.filter, 0.0, "double click resets filter");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn short_window_scroll_reaches_headphone_cue_without_changing_layout_preferences() {
    let (root, mut h) = fresh("cue-scroll", 10.0);
    h.app.settings.sections.mixer = true;
    h.viewport = egui::vec2(640.0, 480.0);
    h.settle();
    let before = h.layout().mixer.unwrap().top();
    let area = h.layout().area.unwrap();
    h.frame_with(
        vec![egui::Event::PointerMoved(
            area.left_top() + egui::vec2(5.0, 5.0),
        )],
        Default::default(),
    );
    h.frame_with(
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -1000.0),
            modifiers: Default::default(),
        }],
        Default::default(),
    );
    for _ in 0..8 {
        h.settle();
    }
    assert!(h.layout().mixer.unwrap().top() < before);
    let cue = h.layout().mixer_cue[1].unwrap();
    assert!(h.app.shell.decks.unwrap().contains_rect(cue));
    h.click(cue.center());
    assert!(h.deck(1).channel.cue);
    assert!(
        h.app.settings.sections.detail
            && h.app.settings.sections.strip
            && h.app.settings.sections.overviews
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn headphone_cue_ui_routes_blends_scales_and_exposes_lit_controls() {
    let (root, mut h) = fresh("headphonecue", 30.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.app.settings.sections.mixer = true;
    h.settle();
    let capture = |h: &mut Harness| {
        for d in 0..2 {
            h.app.decks.decks[d].seek(4_000.0);
            h.app.decks.decks[d].toggle_play();
        }
        h.settle();
        let out = h.app.decks.render_channels(4_800, 4);
        for d in 0..2 {
            h.app.decks.decks[d].toggle_play();
        }
        h.settle();
        out
    };
    let set = |h: &mut Harness, name: &str, value: f64| {
        let out = h.settle();
        let id = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(name))
            .unwrap()
            .0;
        h.frame_with(
            vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    action: accesskit::Action::SetValue,
                    target: id,
                    data: Some(accesskit::ActionData::NumericValue(value)),
                },
            )],
            Default::default(),
        );
        h.settle();
    };
    let off = capture(&mut h);
    assert!(off.chunks_exact(4).all(|f| f[2..] == [0.0, 0.0]));
    for d in 0..2 {
        let out = h.click(h.layout().mixer_cue[d].unwrap().center());
        assert!(h.deck(d).channel.cue);
        assert_eq!(
            node(&out, &format!("Deck {} cue bus", decks::DECKS[d])).toggled(),
            Some(accesskit::Toggled::True)
        );
    }
    set(&mut h, "Cue mix", 0.0);
    let a = capture(&mut h);
    set(&mut h, "Cue mix", 1.0);
    let b = capture(&mut h);
    set(&mut h, "Cue mix", 0.25);
    set(&mut h, "Cue volume", 0.5);
    let blend = capture(&mut h);
    assert!(a.chunks_exact(4).any(|f| f[2].abs() > 0.01));
    assert!(b.chunks_exact(4).any(|f| f[2].abs() > 0.01));
    for ((actual, a), b) in blend
        .chunks_exact(4)
        .zip(a.chunks_exact(4))
        .zip(b.chunks_exact(4))
    {
        for side in 0..2 {
            assert!(
                (actual[2 + side] - (a[2 + side] * 0.75 + b[2 + side] * 0.25) * 0.5).abs() < 1e-4
            );
            assert!((actual[side] - a[side]).abs() < 1e-4, "main unaffected");
        }
    }
    // Native knobs: named sliders, range, SetValue, keyboard, wheel, reset.
    for (name, default) in [("Cue mix", 0.5), ("Cue volume", 1.0)] {
        let out = h.settle();
        let n = node(&out, name);
        assert_eq!(n.role(), accesskit::Role::Slider);
        assert_eq!(n.min_numeric_value(), Some(0.0));
        assert_eq!(n.max_numeric_value(), Some(1.0));
        set(&mut h, name, -10.0);
        let out = h.settle();
        let bounds = node(&out, name).bounds().unwrap();
        let p = egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        );
        // Tab actually reaches the control, not programmatic focus.
        h.key(
            egui::Key::Tab,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        for guard in 0..200 {
            if h.focused_rect().is_some_and(|r| r.contains(p)) {
                break;
            }
            assert!(guard < 199, "{name} Tab reachable");
            h.tap(egui::Key::Tab);
        }
        h.tap(egui::Key::ArrowRight);
        assert!((node(&h.settle(), name).numeric_value().unwrap() - 0.05).abs() < 1e-5);
        h.frame_with(vec![egui::Event::PointerMoved(p)], Default::default());
        let out = h.frame_with(
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 40.0),
                modifiers: Default::default(),
            }],
            Default::default(),
        );
        assert!(node(&out, name).numeric_value().unwrap() > 0.05);
        h.click(p);
        let out = h.click(p);
        assert_eq!(node(&out, name).numeric_value(), Some(default));
    }
    // Toggle off through the same actual button; selected/lit state clears.
    let out = h.click(h.layout().mixer_cue[0].unwrap().center());
    assert!(!h.deck(0).channel.cue);
    assert_eq!(
        node(&out, "Deck A cue bus").toggled(),
        Some(accesskit::Toggled::False)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixer_controls_drive_the_engine_and_meters_through_the_ui() {
    let (root, mut h) = fresh("mixui", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.app.settings.sections.mixer = true;
    let out = h.settle();
    let l = h.layout();
    let mixer = l.mixer.expect("mixer shown");
    let mut rects: Vec<Rect> = vec![
        l.master.unwrap(),
        l.crossfader.unwrap(),
        l.crossfader_toggle.unwrap(),
        l.cue_mix.unwrap(),
        l.cue_volume.unwrap(),
    ];
    for d in 0..2 {
        rects.extend([
            l.trim[d].unwrap(),
            l.filter[d].unwrap(),
            l.volume[d].unwrap(),
        ]);
        rects.extend(l.eq[d].unwrap());
        rects.extend(l.assign[d].unwrap());
    }
    rects.extend(l.meters.iter().map(|m| m.unwrap()));
    for r in &rects {
        assert!(mixer.contains_rect(*r), "{r:?} inside the mixer {mixer:?}");
    }
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            assert!(
                !a.shrink(0.5).intersects(b.shrink(0.5)),
                "{a:?} overlaps {b:?}"
            );
        }
    }
    // Deck A left of the centre, deck B right of it.
    assert!(l.volume[0].unwrap().right() < l.master.unwrap().left());
    assert!(l.volume[1].unwrap().left() > l.master.unwrap().right());
    for letter in ["A", "B"] {
        for name in ["trim", "EQ high", "EQ mid", "EQ low", "filter", "volume"] {
            assert_eq!(
                node(&out, &format!("Deck {letter} {name}")).role(),
                accesskit::Role::Slider,
                "{letter} {name}"
            );
        }
        for side in ["A", "THRU", "B"] {
            assert_eq!(
                node(&out, &format!("Deck {letter} crossfader assign {side}")).role(),
                accesskit::Role::Button
            );
        }
    }
    assert_eq!(node(&out, "Master volume").role(), accesskit::Role::Slider);
    assert_eq!(node(&out, "Crossfader").role(), accesskit::Role::Slider);
    assert_eq!(
        node(&out, "Crossfader section").toggled(),
        Some(accesskit::Toggled::True)
    );
    // Centre default: both decks at unity; deck A's 120 BPM click track
    // and deck B both audible.
    let both = mix_block(&mut h);
    // Pointer: drag deck A's LOW knob to its left stop = kill.
    rotary_drag(
        &mut h,
        "Deck A EQ low",
        &[-90.0, -135.0, -180.0, -225.0, -240.0],
    );
    assert_eq!(
        h.deck(0).channel.eq_db[0],
        oxidj_audio::EQ_KILL_DB,
        "drag to the stop kills"
    );
    // Crossfader hard right (pointer): deck A (side A) silent, B unchanged.
    let xf = h.layout().crossfader.unwrap();
    h.drag(xf.center(), egui::pos2(xf.right() + 30.0, xf.center().y));
    assert_eq!(h.app.decks.crossfader, 1.0);
    let b_only = mix_block(&mut h);
    let reference_b = {
        // Same block with deck A's fader at 0 instead: only B.
        h.app.decks.crossfader = 0.5;
        h.app.decks.decks[0].volume.0 = 0.0;
        let v = mix_block(&mut h);
        h.app.decks.decks[0].volume.0 = 1.0;
        h.app.decks.crossfader = 1.0;
        v
    };
    let diff = b_only
        .iter()
        .zip(&reference_b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(diff < 1e-3, "crossfader right = only deck B ({diff})");
    assert_ne!(bits(&both), bits(&b_only), "deck A was in the centre mix");
    // Assign deck A to THRU by click: back in the mix despite the crossfader.
    let thru = h.layout().assign[0].unwrap()[1];
    h.click(thru.center());
    assert_eq!(h.deck(0).channel.assign, oxidj_audio::Assign::Thru);
    let thru_block = mix_block(&mut h);
    assert!(
        thru_block
            .iter()
            .zip(&b_only)
            .any(|(a, b)| (a - b).abs() > 0.05),
        "THRU ignores the crossfader"
    );
    // Meters: deck A meter reads its level; trim +12 dB clips channel and
    // master; tokens-only drawing inside the meter rects.
    h.app.decks.decks[0].channel.eq_db = [0.0; 3];
    h.app.decks.decks[0].channel.trim_db = 12.0;
    h.app.decks.master = 1.0;
    for d in 0..2 {
        h.app.decks.decks[d].seek(4_000.0);
        h.app.decks.decks[d].toggle_play();
    }
    h.settle();
    h.app.decks.render(4_800);
    let out = h.settle();
    let m = h.app.decks.meters;
    assert!(
        m[0].level > 0.5 && m[0].clipping(),
        "deck A meter {:?}",
        m[0]
    );
    assert!(m[2].clipping(), "master clip {:?}", m[2]);
    let labels: Vec<String> = accesskit_nodes(&out)
        .iter()
        .filter_map(|n| n.label().map(|l| l.to_string()))
        .collect();
    assert!(
        labels
            .iter()
            .any(|t| t.starts_with("Deck A level: ") && t.ends_with(", clip")),
        "meter name carries the clip state"
    );
    assert!(
        labels
            .iter()
            .any(|t| t.starts_with("Master level: ") && t.ends_with(", clip"))
    );
    let tokens = ThemeMode::TokyoNight.tokens();
    let allowed = [
        tokens.background,
        tokens.success,
        tokens.warning,
        tokens.error,
        tokens.border,
        tokens.surface,
    ];
    let meter_r = h.layout().meters[0].unwrap();
    let mut painted = 0;
    for c in out.shapes.iter().filter(|c| c.clip_rect == meter_r) {
        if let Shape::Rect(r) = &c.shape {
            painted += 1;
            assert!(meter_r.expand(0.5).contains_rect(r.rect));
            assert!(
                allowed.contains(&r.fill) || r.fill == egui::Color32::TRANSPARENT,
                "token fill {:?}",
                r.fill
            );
        }
    }
    assert!(painted >= 3, "meter painted ({painted} shapes)");
    for d in 0..2 {
        h.app.decks.decks[d].toggle_play();
    }
    h.settle();
    // The crossfader's own icon toggle: hidden = out of the path, persisted.
    let icon = h.layout().crossfader_toggle.unwrap();
    h.click(icon.center());
    assert!(!h.app.settings.crossfader && h.layout().crossfader.is_none());
    assert_eq!(h.app.decks.mixer_params().crossfader, None);
    let mut store = MemStore::default();
    eframe::App::save(&mut h.app, &mut store);
    assert!(!Settings::load(Some(&store)).crossfader);
    // Ctrl+Shift+5 brings it back.
    let chord = egui::Modifiers {
        ctrl: true,
        command: true,
        shift: true,
        ..Default::default()
    };
    h.key(egui::Key::Num5, chord);
    assert!(h.app.settings.crossfader && h.layout().crossfader.is_some());
    // The curve is a Setting (Ctrl+,): Cut reaches the engine's params.
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    h.key(egui::Key::Comma, ctrl);
    h.click(h.app.settings_ui.layout.tabs.unwrap()[2].center());
    let cells = h
        .app
        .settings_ui
        .layout
        .curve
        .expect("curve row in Settings");
    assert!(
        h.app
            .settings_ui
            .layout
            .window
            .unwrap()
            .contains_rect(cells[2]),
        "Cut visible"
    );
    h.click(cells[2].center());
    assert_eq!(h.app.settings.xfader_curve, oxidj_audio::Curve::Cut);
    assert_eq!(h.app.decks.mixer_params().curve, oxidj_audio::Curve::Cut);
    std::fs::remove_dir_all(root).unwrap();
}

#[derive(Default)]
struct MemStore(std::collections::HashMap<String, String>);
impl eframe::Storage for MemStore {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.into(), value);
    }
    fn flush(&mut self) {}
}

/// D4 review round 1: the mixer reflows instead of squeezing. At every
/// supported window size each slider (actual AccessKit bounds) keeps
/// usable travel inside the mixer and the window, no control overlaps
/// another, a real pointer drag reaches both ends of FILTER and VOL with
/// the right sign, and the arrow keys reach the filter's stops.
#[test]
fn mixer_keeps_usable_non_overlapping_controls_at_narrow_windows() {
    use crate::mixer_view::{MixerFit, SLIDER_MIN_W};
    let (root, mut h) = fresh("mixnarrow", 10.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.app.settings.sections.mixer = true;
    let sliders = |letter: &str| {
        ["trim", "EQ high", "EQ mid", "EQ low", "filter", "volume"]
            .map(|n| format!("Deck {letter} {n}"))
    };
    let mut fits = Vec::new();
    for (w, ht) in [
        (1440.0, 900.0),
        (1024.0, 768.0),
        (800.0, 600.0),
        (640.0, 480.0),
    ] {
        h.viewport = egui::vec2(w, ht);
        // At 480 px, exercise the optional mixer alone; the complete deck
        // area scrolls rather than compressing fader travel below 40 px.
        h.app.settings.sections.detail = ht > 480.0;
        h.app.settings.sections.strip = ht > 480.0;
        h.app.settings.sections.overviews = ht > 480.0;
        let out = h.settle();
        let l = h.layout();
        let mixer = l.mixer.expect("mixer shown");
        fits.push(MixerFit::of(mixer.width()));
        assert!(
            mixer.bottom() <= ht && mixer.right() <= w,
            "{w}x{ht}: mixer {mixer:?} inside the window"
        );
        // Slider bounds as the platform sees them.
        let mut names: Vec<String> = sliders("A").into_iter().chain(sliders("B")).collect();
        names.extend(["Master volume".to_owned(), "Crossfader".to_owned()]);
        let mut rects: Vec<(String, Rect)> = names
            .iter()
            .map(|n| {
                let b = node(&out, n)
                    .bounds()
                    .unwrap_or_else(|| panic!("{n} has bounds"));
                (
                    n.clone(),
                    Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    ),
                )
            })
            .collect();
        for (n, r) in &rects {
            // Vertical faders (volume, master) have travel in height; horizontal (crossfader) in width.
            let is_vertical = n.contains("volume") || n == "Master volume";
            let travel = if is_vertical { r.height() } else { r.width() };
            assert!(travel >= SLIDER_MIN_W, "{w}x{ht}: {n} travel {travel} px");
            assert!(
                mixer.contains_rect(*r),
                "{w}x{ht}: {n} {r:?} inside the mixer {mixer:?}"
            );
        }
        // Controls and their labelled cells never overlap each other.
        for d in 0..2 {
            rects.extend([
                ("trim".into(), l.trim[d].unwrap()),
                ("filter".into(), l.filter[d].unwrap()),
                ("vol".into(), l.volume[d].unwrap()),
            ]);
            rects.extend(l.eq[d].unwrap().map(|r| ("eq".into(), r)));
            rects.extend(l.assign[d].unwrap().map(|r| ("assign".into(), r)));
        }
        rects.extend(l.meters.iter().map(|m| ("meter".into(), m.unwrap())));
        rects.extend([
            ("master".into(), l.master.unwrap()),
            ("xf".into(), l.crossfader.unwrap()),
            ("xf toggle".into(), l.crossfader_toggle.unwrap()),
        ]);
        for (i, (na, a)) in rects.iter().enumerate() {
            assert!(
                mixer.contains_rect(*a),
                "{w}x{ht}: {na} {a:?} inside the mixer"
            );
            for (nb, b) in &rects[i + 1..] {
                // A slider sits inside its own labelled cell.
                if a.contains_rect(b.shrink(0.5)) || b.contains_rect(a.shrink(0.5)) {
                    continue;
                }
                assert!(
                    !a.shrink(0.5).intersects(b.shrink(0.5)),
                    "{w}x{ht}: {na} {a:?} overlaps {nb} {b:?}"
                );
            }
        }
        // Real pointer drags from the actual slider centre reach both ends.
        for (d, letter) in decks::DECKS.iter().enumerate() {
            let at = |out: &Frame, n: &str| {
                let b = node(out, n).bounds().unwrap();
                egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
            };
            h.app.decks.decks[d].channel.filter = 0.0;
            rotary_drag(
                &mut h,
                &format!("Deck {letter} filter"),
                &[-90.0, -135.0, -180.0, -225.0, -240.0],
            );
            assert_eq!(
                h.deck(d).channel.filter,
                -1.0,
                "{w}x{ht}: deck {letter} filter dragged left = low-pass end"
            );
            h.app.decks.decks[d].channel.filter = 0.0;
            rotary_drag(
                &mut h,
                &format!("Deck {letter} filter"),
                &[-90.0, -45.0, 0.0, 45.0, 60.0],
            );
            assert_eq!(
                h.deck(d).channel.filter,
                1.0,
                "{w}x{ht}: deck {letter} filter dragged right = high-pass end"
            );
            let out = h.settle();
            let c = at(&out, &format!("Deck {letter} volume"));
            // Vertical fader: drag up for unity, down for closed.
            h.drag(c, c - egui::vec2(0.0, 200.0));
            assert_eq!(
                h.deck(d).volume.0,
                1.0,
                "{w}x{ht}: deck {letter} fader dragged up = unity"
            );
            h.drag(c, c + egui::vec2(0.0, 200.0));
            assert_eq!(
                h.deck(d).volume.0,
                0.0,
                "{w}x{ht}: deck {letter} fader dragged down = closed"
            );
            h.app.decks.decks[d].channel.filter = 0.0;
        }
        // Keyboard: Tab to deck A's filter, arrows reach both stops.
        let filter = h.layout().filter[0].unwrap();
        // Enter the focus chain (as a fresh window does), then Tab.
        h.key(
            egui::Key::Tab,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        let mut guard = 0;
        while !h
            .focused_rect()
            .is_some_and(|r| filter.contains_rect(r.shrink(1.0)))
        {
            h.key(egui::Key::Tab, Default::default());
            guard += 1;
            assert!(guard < 200, "{w}x{ht}: filter reachable by Tab");
        }
        for _ in 0..45 {
            h.key(egui::Key::ArrowLeft, Default::default());
        }
        assert_eq!(
            h.deck(0).channel.filter,
            -1.0,
            "{w}x{ht}: arrows reach the low-pass stop"
        );
        for _ in 0..45 {
            h.key(egui::Key::ArrowRight, Default::default());
        }
        assert_eq!(
            h.deck(0).channel.filter,
            1.0,
            "{w}x{ht}: arrows reach the high-pass stop"
        );
        h.app.decks.decks[0].channel.filter = 0.0;
        // Keyboard: click deck A's volume fader, arrow keys reach both ends.
        let out = h.settle();
        let vol_b = node(&out, "Deck A volume").bounds().unwrap();
        let vol_c = egui::pos2(
            ((vol_b.x0 + vol_b.x1) / 2.0) as f32,
            ((vol_b.y0 + vol_b.y1) / 2.0) as f32,
        );
        h.click(vol_c);
        for _ in 0..45 {
            h.key(egui::Key::ArrowUp, Default::default());
        }
        assert!(
            (h.deck(0).volume.0 - 1.0).abs() < 1e-3,
            "{w}x{ht}: arrows reach unity"
        );
        for _ in 0..45 {
            h.key(egui::Key::ArrowDown, Default::default());
        }
        assert!(
            (h.deck(0).volume.0 - 0.0).abs() < 1e-3,
            "{w}x{ht}: arrows reach closed"
        );
        h.app.decks.decks[0].volume.0 = 0.5;
    }
    assert_eq!(
        fits,
        [
            MixerFit::Wide,
            MixerFit::CentreBelow,
            MixerFit::CentreBelow,
            MixerFit::Stacked
        ],
        "every arrangement exercised"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D4 acceptance 4: the whole mixer is keyboard reachable (Tab reaches
/// every control) and operable (arrow keys turn a knob to its kill stop,
/// Space presses an assign button and the crossfader toggle). Vertical faders
/// (channel volume, master) are operable via click + arrow keys.
#[test]
fn mixer_is_fully_keyboard_reachable_and_operable() {
    let (root, mut h) = fresh("mixkeys", 10.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.app.settings.sections.mixer = true;
    h.settle();
    let l = h.layout();
    let mut want: Vec<Rect> = vec![
        l.master.unwrap(),
        l.crossfader.unwrap(),
        l.crossfader_toggle.unwrap(),
        l.cue_mix.unwrap(),
        l.cue_volume.unwrap(),
    ];
    for d in 0..2 {
        want.extend([
            l.trim[d].unwrap(),
            l.filter[d].unwrap(),
            l.volume[d].unwrap(),
            l.mixer_cue[d].unwrap(),
        ]);
        want.extend(l.eq[d].unwrap());
        want.extend(l.assign[d].unwrap());
    }
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    h.key(egui::Key::Tab, shift);
    let mut seen = Vec::new();
    for _ in 0..400 {
        if let Some(r) = h.focused_rect() {
            if !seen.contains(&r) {
                seen.push(r);
            }
        }
        let tab_reachable = &want;
        if tab_reachable.iter().all(|w| {
            seen.iter()
                .any(|s| s.contains_rect(w.shrink(1.0)) || w.contains_rect(s.shrink(1.0)))
        }) {
            break;
        }
        h.key(egui::Key::Tab, Default::default());
    }
    for w in &want {
        assert!(
            seen.iter().any(|s| w.contains_rect(s.shrink(1.0))),
            "{w:?} not reachable by Tab"
        );
    }
    // Focus deck A's LOW knob by Tab and turn it to kill with the arrows.
    let low = h.layout().eq[0].unwrap()[0];
    let mut guard = 0;
    while !h
        .focused_rect()
        .is_some_and(|r| low.contains_rect(r.shrink(1.0)))
    {
        h.key(egui::Key::Tab, Default::default());
        guard += 1;
        assert!(guard < 200, "LOW knob reachable");
    }
    for _ in 0..40 {
        h.key(egui::Key::ArrowLeft, Default::default());
    }
    assert_eq!(
        h.deck(0).channel.eq_db[0],
        oxidj_audio::EQ_KILL_DB,
        "arrows reach the kill stop"
    );
    assert_eq!(h.app.decks.mixer_params().channels[0].eq[0], 0.0);
    h.key(egui::Key::ArrowRight, Default::default());
    let db = h.deck(0).channel.eq_db[0];
    assert!(
        db > oxidj_audio::EQ_KILL_DB && db < -20.0,
        "one step up from kill: {db}"
    );
    for _ in 0..19 {
        h.key(egui::Key::ArrowRight, Default::default());
    }
    assert!(
        h.deck(0).channel.eq_db[0].abs() < 1e-4,
        "centre detent = 0 dB: {}",
        h.deck(0).channel.eq_db[0]
    );
    // Space on the THRU assign button and on the crossfader toggle.
    let thru = h.layout().assign[0].unwrap()[1];
    while h.focused_rect() != Some(thru) {
        h.key(egui::Key::Tab, Default::default());
        guard += 1;
        assert!(guard < 400, "THRU reachable");
    }
    h.tap(egui::Key::Space);
    assert_eq!(h.deck(0).channel.assign, oxidj_audio::Assign::Thru);
    let icon = h.layout().crossfader_toggle.unwrap();
    while h.focused_rect() != Some(icon) {
        h.key(egui::Key::Tab, Default::default());
        guard += 1;
        assert!(guard < 600, "crossfader toggle reachable");
    }
    h.tap(egui::Key::Space);
    assert!(!h.app.settings.crossfader, "Space hides the crossfader");
    // Vertical faders: click to focus, then arrow keys operate.
    let at = |out: &Frame, n: &str| {
        let b = node(out, n).bounds().unwrap();
        egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    };
    for d in 0..2 {
        let vol_c = at(&h.settle(), &format!("Deck {} volume", decks::DECKS[d]));
        h.click(vol_c);
        for _ in 0..45 {
            h.key(egui::Key::ArrowUp, Default::default());
        }
        assert!(
            (h.deck(d).volume.0 - 1.0).abs() < 1e-3,
            "deck {d} volume arrows reach unity"
        );
        for _ in 0..45 {
            h.key(egui::Key::ArrowDown, Default::default());
        }
        assert!(
            (h.deck(d).volume.0 - 0.0).abs() < 1e-3,
            "deck {d} volume arrows reach closed"
        );
        h.app.decks.decks[d].volume.0 = 0.5;
    }
    // Master volume: click to focus, then arrow keys operate.
    let master_c = at(&h.settle(), "Master volume");
    h.click(master_c);
    for _ in 0..45 {
        h.key(egui::Key::ArrowUp, Default::default());
    }
    assert!(
        (h.app.decks.master - 1.0).abs() < 1e-3,
        "master arrows reach unity"
    );
    for _ in 0..45 {
        h.key(egui::Key::ArrowDown, Default::default());
    }
    assert!(
        (h.app.decks.master - 0.0).abs() < 1e-3,
        "master arrows reach closed"
    );
    h.app.decks.master = 0.5;
    std::fs::remove_dir_all(root).unwrap();
}

// ------------------------------------------------------------ W12 bands

#[test]
fn analyzed_transients_remain_distinct_in_scrolling_one_bar_detail() {
    let rate = 48_000u32;
    let mut signal = vec![0.0f32; rate as usize * 4];
    for ms in [1000, 1004] {
        signal[ms * rate as usize / 1000] = 1.0;
    }
    let mut builder =
        oxidj_analysis::WaveformBuilder::new(rate, Some(signal.len() as u64)).unwrap();
    builder.push(&signal).unwrap();
    let peaks = builder.finish().unwrap();
    assert!(
        peaks.samples_per_peak() <= 40,
        "detail source needs at least 1200 columns/s"
    );
    let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1880.0, 80.0));
    let colour = decks::deck_colours(DeckPalette::WarmCool, 0, ThemeMode::TokyoNight.tokens());
    for start in [0.0, 500.0] {
        let mut layers: [egui::Mesh; 3] = Default::default();
        decks::build_half(
            &peaks,
            (start, start + 2000.0),
            1.0,
            rect,
            true,
            colour,
            0.0,
            decks::Envelope::Peak,
            &mut layers,
        );
        let height = |ms: f32| {
            let x = (ms - start as f32) * rect.width() / 2000.0;
            layers[0]
                .vertices
                .iter()
                .filter(|v| v.color.a() == 255 && (v.pos.x - x.round()).abs() < 0.1)
                .map(|v| rect.bottom() - v.pos.y)
                .fold(0.0, f32::max)
        };
        assert!(
            height(1000.0) > 10.0 && height(1004.0) > 10.0,
            "both impulses visible"
        );
        assert!(
            height(1002.0) < height(1000.0) * 0.5,
            "separate impulses after scroll: {start}: {} {} {}",
            height(1000.0),
            height(1002.0),
            height(1004.0)
        );
        assert!(layers[0].vertices.len() <= 3 * (rect.width() as usize + 1));
    }
}

/// Constant-per-section synthetic peaks: `sections` of `(seconds, [low, mid, high])`.
fn band_peaks(sections: &[(u32, [f32; 3])]) -> oxidj_application::WaveformPeaks {
    const RATE: u32 = 44_100;
    const SPP: u32 = 294; // 150 columns per second
    let mut bands: [Vec<oxidj_application::WaveformPeak>; 3] = Default::default();
    for &(seconds, amp) in sections {
        for _ in 0..seconds * 150 {
            for b in 0..3 {
                let a = amp[b].clamp(0.0, 1.0);
                bands[b].push(oxidj_application::WaveformPeak::from_f32(-a, a).unwrap());
            }
        }
    }
    let samples = bands[0].len() as u64 * u64::from(SPP);
    oxidj_application::WaveformPeaks::from_base(RATE, SPP, samples, bands).unwrap()
}

/// Body (opaque) vertices of a layer mesh as `(reach from base, colour)`.
fn body(mesh: &egui::Mesh, rect: Rect, up: bool) -> Vec<(f32, egui::Color32)> {
    mesh.vertices
        .iter()
        .filter(|v| v.color.a() == 255)
        .map(|v| {
            (
                if up {
                    rect.bottom() - v.pos.y
                } else {
                    v.pos.y - rect.top()
                },
                v.color,
            )
        })
        .collect()
}

#[test]
fn deck_band_layers_keep_their_own_colour_and_nest_at_min_mid_max() {
    let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(300.0, 80.0));
    for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
        for theme in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            for d in 0..2 {
                let c = decks::deck_colours(palette, d, theme.tokens());
                let colours = [c.low, c.mid, c.high];
                assert!(
                    colours[0] != colours[1]
                        && colours[1] != colours[2]
                        && colours[0] != colours[2],
                    "three hues"
                );
                // Typical spectrum (low > mid > high) at min / mid / max
                // amplitude, plus a hat-only column (high loudest).
                for (case, amp) in [
                    ("min", [0.0, 0.0, 0.0]),
                    ("mid", [0.5, 0.3, 0.15]),
                    ("max", [1.0, 0.8, 0.5]),
                    ("hat", [0.1, 0.2, 0.6]),
                ] {
                    let peaks = band_peaks(&[(4, amp)]);
                    let want = decks::stack(amp, 1.0);
                    for up in [true, false] {
                        let mut layers: [egui::Mesh; 3] = Default::default();
                        decks::build_half(
                            &peaks,
                            (0.0, 4_000.0),
                            1.0,
                            rect,
                            up,
                            c,
                            1.0,
                            decks::Envelope::Peak,
                            &mut layers,
                        );
                        let what = format!("{palette:?} {theme:?} deck {d} {case} up={up}");
                        let mut reach = [0.0f32; 3];
                        for (layer, mesh) in layers.iter().enumerate() {
                            // Bounds inside the clip, fringe included.
                            assert!(
                                mesh.vertices
                                    .iter()
                                    .all(|v| rect.expand(0.01).contains(v.pos)),
                                "{what}: inside"
                            );
                            let b = body(mesh, rect, up);
                            assert!(!b.is_empty(), "{what}: layer {layer} painted");
                            // Each layer is exactly its band's colour: low, mid, high.
                            assert!(
                                b.iter().all(|(_, col)| *col == colours[layer]),
                                "{what}: layer {layer} colour"
                            );
                            reach[layer] = b.iter().map(|(r, _)| *r).fold(0.0, f32::max);
                        }
                        // Nested: low >= mid >= high, each ring its band's share.
                        assert!(
                            reach[0] >= reach[1] && reach[1] >= reach[2],
                            "{what}: nesting {reach:?}"
                        );
                        let full = rect.height() - 1.5;
                        for k in 0..3 {
                            let w = want[k] * full;
                            let w = if k == 0 { w.max(1.0) } else { w };
                            assert!(
                                (reach[k] - w).abs() <= 1.0,
                                "{what}: layer {k} reach {} vs {w}",
                                reach[k]
                            );
                        }
                        let outer = amp.iter().copied().fold(0.0, f32::max);
                        assert!(
                            (want[0] - outer).abs() < 1e-6,
                            "{what}: outer edge = loudest band"
                        );
                        if case != "min" {
                            for k in 0..3 {
                                let ring = want[k] - if k < 2 { want[k + 1] } else { 0.0 };
                                let w = decks::SHARE_WEIGHTS;
                                let share = outer * amp[k] * w[k]
                                    / (0..3).map(|j| amp[j] * w[j]).sum::<f32>();
                                assert!(
                                    (ring - share).abs() < 1e-5,
                                    "{what}: band {k} ring = its share"
                                );
                            }
                        }
                        if case == "max" {
                            assert!(reach[0] >= full - 1.0, "{what}: max fills the half");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn overview_mean_envelope_auto_gains_a_quiet_track_and_shows_breaks() {
    let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(400.0, 17.0));
    let c = decks::deck_colours(DeckPalette::WarmCool, 0, ThemeMode::TokyoNight.tokens());
    // Quiet master: intro, drop, break (near silent), drop — all at 0.15 peak.
    let quiet = band_peaks(&[
        (20, [0.08, 0.05, 0.03]),
        (40, [0.15, 0.1, 0.06]),
        (20, [0.01, 0.01, 0.01]),
        (40, [0.15, 0.1, 0.06]),
    ]);
    let window = (0.0, 120_000.0);
    let gain = decks::overview_gain(&quiet, window, rect.width(), true);
    assert_eq!(
        decks::overview_gain(&quiet, window, rect.width(), false),
        1.0,
        "auto-gain off = unity"
    );
    let mut layers: [egui::Mesh; 3] = Default::default();
    decks::build_half(
        &quiet,
        window,
        gain,
        rect,
        true,
        c,
        1.0,
        decks::Envelope::Mean,
        &mut layers,
    );
    let at = |layers: &[egui::Mesh; 3], x0: f32, x1: f32| {
        layers[0]
            .vertices
            .iter()
            .filter(|v| v.color.a() == 255 && v.pos.x >= x0 && v.pos.x <= x1)
            .map(|v| rect.bottom() - v.pos.y)
            .fold(0.0, f32::max)
    };
    let x = |s: f32| rect.width() * s / 120.0;
    let drop = at(&layers, x(25.0), x(55.0));
    let brk = at(&layers, x(63.0), x(77.0));
    assert!(
        drop >= 0.6 * rect.height(),
        "quiet drop fills >= 60 %: {drop} of {}",
        rect.height()
    );
    assert!(
        brk <= 0.2 * rect.height(),
        "the break stays visibly low: {brk}"
    );
    // Without auto-gain the same track barely shows.
    decks::build_half(
        &quiet,
        window,
        1.0,
        rect,
        true,
        c,
        1.0,
        decks::Envelope::Mean,
        &mut layers,
    );
    assert!(at(&layers, x(25.0), x(55.0)) < 0.25 * rect.height());
    // A loud master is not pushed past the half.
    let loud = band_peaks(&[(60, [1.0, 0.9, 0.7])]);
    let g = decks::overview_gain(&loud, (0.0, 60_000.0), rect.width(), true);
    decks::build_half(
        &loud,
        (0.0, 60_000.0),
        g,
        rect,
        true,
        c,
        1.0,
        decks::Envelope::Mean,
        &mut layers,
    );
    assert!(
        layers
            .iter()
            .flat_map(|m| &m.vertices)
            .all(|v| rect.expand(0.01).contains(v.pos))
    );
}

#[test]
fn deck_band_palette_contrast_in_both_themes() {
    for theme in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
        let tokens = theme.tokens();
        for palette in [DeckPalette::WarmCool, DeckPalette::Classic] {
            for d in 0..2 {
                let c = decks::deck_colours(palette, d, tokens);
                let what = format!("{theme:?} {palette:?} deck {d}");
                for (name, band) in [("low", c.low), ("mid", c.mid), ("high", c.high)] {
                    assert!(
                        decks::contrast(band, tokens.background) >= 3.0,
                        "{what}: {name} on the plate"
                    );
                }
                assert!(decks::contrast(c.low, c.mid) >= 1.5, "{what}: low vs mid");
                assert!(decks::contrast(c.mid, c.high) >= 1.5, "{what}: mid vs high");
            }
            if palette == DeckPalette::WarmCool {
                let [a, b] = [0, 1].map(|d| decks::deck_colours(palette, d, tokens));
                for band in [a.low, a.mid] {
                    assert!(band.r() > band.b(), "{theme:?}: A bands warm");
                }
                for band in [b.low, b.mid] {
                    assert!(band.b() > band.r(), "{theme:?}: B bands cool");
                }
            }
        }
    }
}

#[test]
fn phrase_ticks_every_16_bars_inside_the_overview() {
    let grid = crate::beat_grid::BeatGrid::new(&grid(120.0, 140.0, 0.0), 140_000.0).unwrap(); // 70 bars
    let rect = Rect::from_min_size(Pos2::new(26.0, 500.0), egui::vec2(700.0, 34.0));
    let mut mesh = egui::Mesh::default();
    decks::phrase_lines(
        &grid,
        (0.0, 140_000.0),
        rect,
        egui::Color32::WHITE,
        &mut mesh,
    );
    // Bars 1, 17, 33, 49, 65 → five 1 px quads (4 vertices each).
    assert_eq!(mesh.vertices.len(), 5 * 4);
    assert!(mesh.vertices.iter().all(|v| rect.contains(v.pos)));
    let xs: Vec<f32> = mesh.vertices.chunks(4).map(|q| q[0].pos.x).collect();
    let bar_px = rect.width() * 2_000.0 / 140_000.0;
    for w in xs.windows(2) {
        assert!(
            ((w[1] - w[0]) - 16.0 * bar_px).abs() < 1.0,
            "16-bar spacing {xs:?}"
        );
    }
}

#[test]
fn production_decks_paint_three_bands_and_readable_overview_cues_with_hover_names() {
    let (root, mut h) = fresh("bands", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("c-nogrid.wav", egui::Key::B);
    h.app.decks.decks[0].seek(5_000.0);
    let out = h.settle();
    let tokens = ThemeMode::TokyoNight.tokens();
    let [ha, hb] = h.layout().halves.unwrap();
    let [oa, ob] = h.layout().overviews.unwrap();
    for (d, half, ov) in [(0, ha, oa), (1, hb, ob)] {
        let c = decks::deck_colours(DeckPalette::WarmCool, d, tokens);
        for (clip, what) in [(half, "detail"), (ov, "overview")] {
            let painted: Vec<egui::Color32> = meshes_in(&out, clip)
                .into_iter()
                .map(|(_, col)| col)
                .filter(|col| col.a() == 255)
                .collect();
            for (name, band) in [("low", c.low), ("mid", c.mid), ("high", c.high)] {
                assert!(
                    painted.contains(&band),
                    "deck {d} {what}: {name} band painted"
                );
            }
        }
    }
    // Overview auto-gain: deck B's quiet master (0.3) still fills >= 60 % of its upper half.
    let inner = Rect::from_min_max(
        Pos2::new(ob.left() + decks::OVERVIEW_INSET, ob.top()),
        ob.max,
    );
    let mid = inner.center().y.round();
    let cb = decks::deck_colours(DeckPalette::WarmCool, 1, tokens);
    let reach = meshes_in(&out, ob)
        .into_iter()
        .filter(|(_, col)| *col == cb.low || *col == cb.mid || *col == cb.high)
        .filter(|(p, _)| p.y <= mid)
        .map(|(p, _)| mid - p.y)
        .fold(0.0, f32::max);
    assert!(
        reach >= 0.6 * (mid - inner.top()),
        "quiet overview reach {reach}"
    );
    // Cue markers on the overview: hot-cue flags and memory triangles >= 6 px tall.
    let flags: Vec<Rect> = out
        .shapes
        .iter()
        .filter(|c| c.clip_rect == oa)
        .filter_map(|c| match &c.shape {
            Shape::Rect(r)
                if r.fill == egui::Color32::from_rgb(0x28, 0xe2, 0x14) && r.rect.width() > 4.0 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    assert!(
        flags.iter().any(|r| r.height() >= 6.0),
        "hot cue 2 flag >= 6 px: {flags:?}"
    );
    let triangles: Vec<f32> = out
        .shapes
        .iter()
        .filter(|c| c.clip_rect == oa)
        .filter_map(|c| match &c.shape {
            Shape::Path(p) if p.points.len() == 3 => Some(
                p.points.iter().map(|q| q.y).fold(f32::MIN, f32::max)
                    - p.points.iter().map(|q| q.y).fold(f32::MAX, f32::min),
            ),
            _ => None,
        })
        .collect();
    assert!(
        triangles.len() >= 2 && triangles.iter().all(|t| *t >= 6.0),
        "memory cues >= 6 px: {triangles:?}"
    );
    // Hover over hot cue 2 (6 s of 20 s) names it in a tooltip.
    let inner_a = Rect::from_min_max(
        Pos2::new(oa.left() + decks::OVERVIEW_INSET, oa.top()),
        oa.max,
    );
    let duration = h.deck(0).track.as_ref().unwrap().duration_ms;
    let x = inner_a.left() + inner_a.width() * (6_000.0 / duration) as f32;
    let pos = Pos2::new(x + 2.0, oa.center().y + 8.0);
    h.frame_with(vec![egui::Event::PointerMoved(pos)], Default::default());
    let mut found = false;
    for _ in 0..40 {
        let out = h.frame_with(vec![], Default::default());
        if texts(&out).iter().any(|(t, _)| t == "Hot cue 2 · 0:06") {
            found = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(found, "hover names hot cue 2");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_overview_brackets_a_manual_loop_and_names_its_endpoints_on_hover() {
    let (root, mut h) = fresh("manual-loop", 20.0);
    h.select_and_load("c-nogrid.wav", egui::Key::B);
    assert!(
        h.deck(1)
            .track
            .as_ref()
            .unwrap()
            .cues
            .iter()
            .all(|c| c.end_ms.is_none()),
        "no imported loop on c"
    );
    let loops = h.layout().loops[1].unwrap();
    h.seek(1, 4_000.0);
    h.click(loops[0].center());
    h.seek(1, 6_000.0);
    h.click(loops[1].center());
    assert_eq!(h.deck(1).loop_range, Some((4_000.0, 6_000.0)));
    let out = h.settle();
    let ob = h.layout().overviews.unwrap()[1];
    let inner = Rect::from_min_max(
        Pos2::new(ob.left() + decks::OVERVIEW_INSET, ob.top()),
        ob.max,
    );
    let duration = h.deck(1).track.as_ref().unwrap().duration_ms;
    let x_of = |ms: f64| inner.left() + inner.width() * (ms / duration) as f32;
    let text = ThemeMode::TokyoNight.tokens().text;
    let ticks: Vec<Rect> = out
        .shapes
        .iter()
        .filter(|c| c.clip_rect == ob)
        .filter_map(|c| match &c.shape {
            Shape::Rect(r) if r.fill == text && r.rect.width() <= 2.5 && r.rect.height() >= 6.0 => {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    for ms in [4_000.0, 6_000.0] {
        let x = x_of(ms);
        assert!(
            ticks
                .iter()
                .any(|r| (r.center().x - x).abs() <= 2.5 && ob.contains_rect(*r)),
            "opaque >= 6 px tick at {ms} ms inside the overview: {ticks:?}"
        );
    }
    for (ms, want) in [
        (4_000.0, "Loop IN · 0:04–0:06"),
        (6_000.0, "Loop OUT · 0:04–0:06"),
    ] {
        h.frame_with(
            vec![egui::Event::PointerMoved(Pos2::new(
                x_of(ms),
                ob.center().y + 8.0,
            ))],
            Default::default(),
        );
        let mut found = false;
        for _ in 0..40 {
            let out = h.frame_with(vec![], Default::default());
            if texts(&out).iter().any(|(t, _)| t == want) {
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(found, "hover names {want}");
        h.frame_with(vec![egui::Event::PointerGone], Default::default());
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ---- deck album artwork ----------------------------------------------------

/// A deterministic JPEG of `width` x `height` (row/column graded).
fn artwork_jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut image = image::RgbImage::new(width, height);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 40]);
    }
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    bytes
}

/// A one-track catalog; when `with_cover` is set, `cover.jpg` sits beside the
/// audio so the artwork fallback has something to find.
fn artwork_catalog(root: &Path, with_cover: bool) -> PathBuf {
    let audio = root.join("audio");
    std::fs::create_dir_all(&audio).unwrap();
    write_loop_wav(&audio.join("track.wav"), 2.0, 120.0, 0.9);
    if with_cover {
        std::fs::write(audio.join("cover.jpg"), artwork_jpeg(4, 2)).unwrap();
    }
    let db = root.join("catalog.sqlite");
    let mut catalog = Catalog::open(&db).unwrap();
    catalog
        .import_audio_directory(audio.to_str().unwrap())
        .unwrap();
    db
}

/// Pump frames until the deck's artwork job finishes (or the timeout hits).
fn wait_artwork(h: &mut Harness, deck: usize) {
    let start = std::time::Instant::now();
    while h.deck(deck).art.is_loading() {
        assert!(start.elapsed().as_secs() < 20, "artwork job did not finish");
        h.frame_with(vec![], Default::default());
    }
    h.settle();
}

#[test]
fn deck_artwork_loads_async_and_swaps_the_placeholder() {
    let root = temp_root("artwork");
    let db = artwork_catalog(&root, true);
    let mut h = Harness::new(db);
    let id = h.app.tracks[0].track.id.as_u64();
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    h.app.artwork_gate = Some(gate.clone());
    h.app.load_deck(0, id);
    // The worker is held: the placeholder is drawn, no texture yet.
    h.settle();
    assert!(h.app.decks.layout.artwork[0].is_some(), "placeholder rect");
    assert!(
        h.deck(0).art.texture().is_none(),
        "texture waits for the worker"
    );
    assert_eq!(h.deck(0).art.size(), [0, 0]);
    // Release the worker; the artwork swaps in.
    gate.wait();
    wait_artwork(&mut h, 0);
    assert_eq!(h.deck(0).art.size(), [4, 2], "decoded dimensions");
    // The cover is the band's square at the left of the deck panel, never
    // over the waveform.
    let half = h.app.decks.layout.halves.unwrap()[0];
    let rect = h.app.decks.layout.artwork[0].unwrap();
    let strip = h.app.decks.layout.strip.unwrap();
    assert!(!rect.intersects(half), "artwork obscures the waveform");
    assert!(strip.contains_rect(rect), "artwork stays in deck controls");
    assert!((rect.width() - rect.height()).abs() < 0.01, "square cover");
    assert_eq!(rect.height(), crate::decks::BAND_H, "cover spans the band");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deck_artwork_budget_change_releases_both_textures_and_rejects_stale_jobs() {
    let root = temp_root("artwork-budget");
    let db = artwork_catalog(&root, true);
    std::fs::write(root.join("audio/cover.jpg"), artwork_jpeg(240, 120)).unwrap();
    let mut h = Harness::new(db);
    let id = h.app.tracks[0].track.id.as_u64();
    h.app.load_deck(0, id);
    h.app.load_deck(1, id);
    wait_artwork(&mut h, 0);
    wait_artwork(&mut h, 1);
    assert!(h.deck(0).art.texture().is_some());
    assert!(h.deck(1).art.texture().is_some());
    assert_eq!(h.deck(0).art.size(), [240, 120]);
    assert!(h.app.artwork_cache.bytes() > 0);
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    h.app.artwork_gate = Some(gate.clone());
    h.app.load_deck(0, id);
    h.app.settings.show_artwork = false;
    h.settle();
    assert_eq!(h.app.artwork_cache.bytes(), 0);
    for d in 0..2 {
        assert!(!h.deck(d).art.is_loading());
        assert!(h.deck(d).art.texture().is_none());
    }
    gate.wait(); // the cancelled old request must not repopulate either deck
    h.app.settings.artwork_cache_mib = 1;
    h.app.settings.show_artwork = true;
    h.settle();
    wait_artwork(&mut h, 0);
    wait_artwork(&mut h, 1);
    for d in 0..2 {
        assert_eq!(h.deck(d).art.size(), [90, 45]);
        let texture = h.deck(d).art.texture().unwrap();
        assert!(texture.size()[0] * texture.size()[1] * 4 <= 1024 * 1024 / 32);
    }
    assert!(h.app.artwork_cache.bytes() <= 1024 * 1024 / 4);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deck_artwork_placeholder_is_shown_when_the_track_has_none() {
    let root = temp_root("artwork-none");
    let db = artwork_catalog(&root, false);
    let mut h = Harness::new(db);
    let id = h.app.tracks[0].track.id.as_u64();
    h.app.load_deck(0, id);
    wait_artwork(&mut h, 0);
    assert!(h.app.decks.layout.artwork[0].is_some(), "placeholder rect");
    assert!(h.deck(0).art.texture().is_none(), "no artwork");
    assert_eq!(h.deck(0).art.size(), [0, 0]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deck_artwork_toggle_hides_and_shows_the_artwork() {
    let root = temp_root("artwork-toggle");
    let db = artwork_catalog(&root, true);
    let mut h = Harness::new(db);
    let id = h.app.tracks[0].track.id.as_u64();
    h.app.load_deck(0, id);
    wait_artwork(&mut h, 0);
    assert!(h.deck(0).art.texture().is_some(), "artwork loaded");
    assert!(h.app.decks.layout.artwork[0].is_some());

    h.app.settings.show_artwork = false;
    h.settle();
    assert!(
        h.app.decks.layout.artwork[0].is_none(),
        "toggle hides the artwork"
    );

    h.app.settings.show_artwork = true;
    h.settle();
    assert!(
        h.app.decks.layout.artwork[0].is_some(),
        "toggle shows the artwork again"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn deck_artwork_toggle_in_settings_hides_and_shows_the_artwork() {
    let root = temp_root("artwork-setting");
    let db = artwork_catalog(&root, true);
    let mut h = Harness::new(db);
    let id = h.app.tracks[0].track.id.as_u64();
    h.app.load_deck(0, id);
    wait_artwork(&mut h, 0);
    assert!(h.app.decks.layout.artwork[0].is_some());

    // Ctrl+, opens Settings; the Deck view tab holds the artwork toggle.
    h.key(
        egui::Key::Comma,
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    );
    assert!(h.app.settings_ui.open, "Settings opens");
    let tabs = h.app.settings_ui.layout.tabs.unwrap();
    h.click(tabs[2].center());
    let budget = h
        .app
        .settings_ui
        .layout
        .artwork_budget
        .expect("artwork memory budget");
    assert!(
        h.app
            .settings_ui
            .layout
            .window
            .unwrap()
            .contains_rect(budget)
    );
    h.drag(budget.center(), budget.center() + egui::vec2(30.0, 0.0));
    assert!(
        h.app.settings.artwork_cache_mib > 100,
        "real budget control changes Settings"
    );
    assert_eq!(
        Settings::decode(&h.app.settings.encode()).artwork_cache_mib,
        h.app.settings.artwork_cache_mib
    );
    let toggle = h.app.settings_ui.layout.artwork.expect("artwork toggle");
    assert!(
        h.app
            .settings_ui
            .layout
            .window
            .unwrap()
            .contains_rect(toggle)
    );

    assert!(h.app.settings.show_artwork, "default on");
    h.click(toggle.center());
    assert!(!h.app.settings.show_artwork, "click turns it off");
    assert!(h.app.decks.layout.artwork[0].is_none(), "artwork hidden");
    h.click(h.app.settings_ui.layout.artwork.unwrap().center());
    assert!(h.app.settings.show_artwork, "click turns it back on");
    assert!(
        h.app.decks.layout.artwork[0].is_some(),
        "artwork shown again"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ---------------------------------------------------------- deck panel v2

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// Icon strokes (Lucide paths) painted inside `rect`, as their colours.
fn stroke_colours_in(out: &Frame, rect: Rect) -> Vec<egui::Color32> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Path(p) if rect.contains_rect(p.visual_bounding_rect()) => {
                match p.stroke.color {
                    egui::epaint::ColorMode::Solid(colour) => Some(colour),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// One content band per deck: cover top = metadata top = BPM top = pitch
/// fader top; cover bottom = hot-cue pads bottom = PLAY bottom = fader bottom.
#[test]
fn deck_panel_v2_content_band_edges_align_for_both_decks() {
    let (root, mut h) = fresh("band-align", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    for (case, artwork) in [("metadata + cover shown", true), ("cover hidden", false)] {
        h.app.settings.show_artwork = artwork;
        h.settle();
        let l = h.layout();
        let strip = l.strip.unwrap();
        for d in 0..2 {
            let meta = l.metadata[d].unwrap();
            let bpm = l.bpm[d].unwrap();
            let fader = l.tempo[d].unwrap();
            let play = l.play[d].unwrap();
            let pads = l.pads[d].unwrap();
            let top = strip.top();
            let bottom = strip.top() + decks::BAND_H;
            assert!(near(meta.top(), top), "{case} deck {d}: metadata top");
            assert!(near(bpm.top(), top), "{case} deck {d}: BPM top");
            assert!(near(fader.top(), top), "{case} deck {d}: fader top");
            assert!(
                near(fader.bottom(), bottom),
                "{case} deck {d}: fader bottom"
            );
            assert!(near(play.bottom(), bottom), "{case} deck {d}: PLAY bottom");
            for pad in &pads[4..] {
                assert!(near(pad.bottom(), bottom), "{case} deck {d}: pad bottom");
            }
            match l.artwork[d] {
                Some(cover) => {
                    assert!(artwork, "{case}");
                    assert!(near(cover.top(), top) && near(cover.bottom(), bottom));
                    assert!(near(cover.width(), cover.height()), "square cover");
                    assert!(cover.right() <= meta.left() && cover.right() <= play.left());
                }
                None => assert!(!artwork, "{case} deck {d}: cover missing"),
            }
        }
    }
    // Empty decks keep the same band.
    std::fs::remove_dir_all(root).unwrap();
    let (root, mut h) = fresh("band-align-empty", 20.0);
    h.settle();
    let l = h.layout();
    let strip = l.strip.unwrap();
    for d in 0..2 {
        assert!(
            near(l.metadata[d].unwrap().top(), strip.top()),
            "empty metadata"
        );
        assert!(near(l.bpm[d].unwrap().top(), strip.top()), "empty BPM");
        assert!(near(
            l.tempo[d].unwrap().bottom(),
            strip.top() + decks::BAND_H
        ));
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// Loop row above CUE above PLAY; eight square labelled pads in 4 x 2.
#[test]
fn deck_panel_v2_transport_stack_and_square_hot_cue_pads() {
    let (root, mut h) = fresh("transport-stack", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    let out = h.settle();
    let l = h.layout();
    for d in 0..2 {
        let loops = l.loops[d].unwrap();
        let (cue, play) = (l.cue[d].unwrap(), l.play[d].unwrap());
        for r in loops {
            assert!(r.bottom() <= cue.top(), "deck {d}: loop row above CUE");
        }
        assert!(cue.bottom() <= play.top(), "deck {d}: CUE above PLAY");
        assert!(near(cue.left(), play.left()), "deck {d}: one column");
        let pads = l.pads[d].unwrap();
        for (p, r) in pads.iter().enumerate() {
            assert!(near(r.width(), r.height()), "deck {d} pad {p} is square");
            assert!(near(r.width(), pads[0].width()));
            let (col, row) = (p % 4, p / 4);
            assert!(near(
                r.left(),
                pads[0].left() + col as f32 * (r.width() + 2.0)
            ));
            assert!(near(
                r.top(),
                pads[0].top() + row as f32 * (r.height() + 2.0)
            ));
            let letter = ["A", "B"][d];
            let n = p + 1;
            assert!(
                accesskit_nodes(&out).iter().any(|node| node
                    .label()
                    .is_some_and(|l| l.starts_with(&format!("Deck {letter} hot cue {n} ")))),
                "pad {n} named"
            );
            assert!(
                texts(&out)
                    .iter()
                    .any(|(s, rect)| *s == n.to_string() && r.contains_rect(*rect)),
                "pad {n} labelled with its number"
            );
        }
        assert!(pads[0].right() < cue.left() + 400.0 && pads[3].bottom() < pads[4].top());
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// Pitch column: BPM number without a label, SYNC beside key lock, KEY,
/// remaining time, range button and a vertical fader over the full band.
#[test]
fn deck_panel_v2_pitch_column_contents_and_vertical_fader() {
    let (root, mut h) = fresh("pitch-column", 20.0);
    h.select_and_load("a-120.wav", egui::Key::A);
    h.select_and_load("b-124.wav", egui::Key::B);
    h.seek(0, 5_000.0);
    let out = h.settle();
    let l = h.layout();
    let t = texts(&out);
    for d in 0..2 {
        let bpm = l.bpm[d].unwrap();
        let (sync, lock) = (l.sync[d].unwrap(), l.key_lock[d].unwrap());
        let time = l.time[d].unwrap();
        let range = l.range[d].unwrap();
        let fader = l.tempo[d].unwrap();
        assert!(
            sync.right() < lock.left() + 4.0 && near(sync.top(), lock.top()),
            "SYNC beside lock"
        );
        assert!(bpm.bottom() <= sync.top(), "BPM above SYNC");
        assert!(sync.bottom() <= time.top() && time.bottom() <= range.top());
        assert!(
            fader.height() >= 4.0 * fader.width(),
            "vertical fader {fader:?}"
        );
        assert!(fader.left() > range.right() && fader.left() > lock.right());
        let in_col = |s: &(String, Rect)| l.strip.unwrap().contains_rect(s.1);
        assert!(
            t.iter().any(|s| in_col(s)
                && bpm.expand(1.0).contains_rect(s.1)
                && s.0.parse::<f32>().is_ok()),
            "BPM shows a bare number"
        );
        assert!(
            t.iter()
                .any(|s| s.0 == "KEY —" && s.1.left() > l.metadata[d].unwrap().left())
        );
        assert!(
            t.iter()
                .any(|s| s.0.starts_with('−') && time.expand(1.0).contains_rect(s.1)),
            "time remaining"
        );
        assert!(t.iter().any(|s| s.0 == "±10%" && range.contains_rect(s.1)));
    }
    assert!(!t.iter().any(|s| s.0.trim() == "BPM"), "no BPM label");
    // Fader drag runs the full band and changes the tempo.
    let fader = l.tempo[0].unwrap();
    let before = h.deck(0).tempo_pct;
    h.drag(fader.center(), fader.center() + egui::vec2(0.0, -20.0));
    assert!(
        h.deck(0).tempo_pct != before,
        "vertical drag moves the fader"
    );
    // Range button cycles the shared range.
    let range = h.layout().range[1].unwrap();
    h.click(range.center());
    assert_eq!(
        h.app.settings.tempo_range,
        crate::settings::TempoRange::Sixteen
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Lucide icons are drawn in the button's own colour; no emoji anywhere.
#[test]
fn deck_panel_v2_icons_use_the_button_colour_and_no_emoji() {
    let (root, mut h) = fresh("icons", 20.0);
    let tokens = ThemeMode::TokyoNight.tokens();
    // Empty decks: controls disabled, icons muted.
    let out = h.settle();
    let l = h.layout();
    let cue = l.cue[0].unwrap();
    assert!(
        stroke_colours_in(&out, cue)
            .iter()
            .all(|c| *c == tokens.muted_text),
        "disabled cue icon is muted"
    );
    h.select_and_load("a-120.wav", egui::Key::A);
    h.seek(0, 3_000.0);
    let out = h.settle();
    let l = h.layout();
    let cue = stroke_colours_in(&out, l.cue[0].unwrap());
    assert!(cue.len() >= 2, "circle-dot paints two paths");
    assert!(
        cue.iter().all(|c| *c == tokens.warning),
        "cue in its colour: {cue:?}"
    );
    let play = stroke_colours_in(&out, l.play[0].unwrap());
    assert!(
        !play.is_empty() && play.iter().all(|c| *c == tokens.success),
        "play {play:?}"
    );
    for (name, rect) in [
        ("loop in", l.loops[0].unwrap()[0]),
        ("loop out", l.loops[0].unwrap()[1]),
        ("key lock", l.key_lock[0].unwrap()),
    ] {
        let colours = stroke_colours_in(&out, rect);
        assert!(!colours.is_empty(), "{name} icon painted");
        assert!(
            colours.iter().all(|c| *c == tokens.text),
            "{name}: {colours:?}"
        );
    }
    // Pause icon (two bars) while playing, in the contrasting ink.
    h.app.decks.decks[0].toggle_play();
    let out = h.settle();
    let play_r = h.layout().play[0].unwrap();
    let pause = stroke_colours_in(&out, play_r);
    assert_eq!(pause.len(), 2, "pause draws two bars");
    assert!(pause.iter().all(|c| *c != tokens.success));
    // Timer icon beside the remaining time.
    let time = h.layout().time[0].unwrap();
    let timer = stroke_colours_in(&out, time.expand2(egui::vec2(16.0, 2.0)));
    assert!(timer.len() >= 3, "timer icon painted");
    // No emoji or pictographs in the deck panel's painted text.
    let strip = h.layout().strip.unwrap();
    for (s, _) in texts(&out)
        .into_iter()
        .filter(|(_, r)| strip.contains_rect(*r))
    {
        assert!(
            !s.chars()
                .any(|c| matches!(c as u32, 0x2300..=0x23FF | 0x2600..=0x27BF | 0x1F000..)),
            "emoji in {s:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
