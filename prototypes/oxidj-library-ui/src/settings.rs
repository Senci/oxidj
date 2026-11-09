//! M8/D0: the typed, persisted [`Settings`] model and the Settings window
//! (Ctrl+, or the toolbar gear).
//!
//! Settings persist through eframe storage under one string key as
//! `name=value` pairs; a missing, unknown or invalid value falls back to that
//! field's default, so old or corrupt storage never breaks start-up. The
//! theme mode is not duplicated here: the Appearance group edits the existing
//! persisted [`ThemeMode`] (key `ui_theme`), which keeps old stores valid.
//!
//! Wired today: theme, waveform style, smooth, beat/bar lines, default zoom.
//! Auto-gain and the Decks group are stored for the D1+ deck view only.

use crate::components::{segmented_row, toggle_row};
use crate::theme::{self, ThemeMode};
use crate::waveform_view::Style;
use crate::{LibraryApp, SectionHeading};
use eframe::egui;
use oxidj_audio::Curve;

const SETTINGS_KEY: &str = "ui_settings";
/// Default-zoom choices in bars (seconds = 2 × bars without a beat grid).
pub const ZOOM_BARS: [usize; 4] = [1, 2, 4, 8];
/// Window title (also its AccessKit name).
pub const WINDOW_TITLE: &str = "Settings";
/// Read-only shortcut list shown in the Keyboard group.
pub const SHORTCUTS: [(&str, &str); 14] = [
    ("Ctrl+,", "Open or close Settings"),
    ("Esc", "Close Settings"),
    ("Ctrl+B", "Show or hide Navigation"),
    ("Ctrl+I", "Show or hide Details"),
    ("Ctrl+D", "Show or hide Decks"),
    ("A / B", "Load the selected track to deck A / B"),
    ("Space", "Play or pause the active deck"),
    ("Tab", "Switch the active deck (Shift+Tab enters the controls)"),
    ("Q / P", "Cue deck A / deck B (standard mode)"),
    ("1 – 8", "Hot-cue pads of the active deck"),
    ("+ / −, wheel", "Zoom the deck waveform (1–8 bars)"),
    ("Ctrl+Shift+1 – 5", "Show or hide a deck section (5 = crossfader)"),
    ("Ctrl+Alt+1 / 2", "Tokyo Night / Tokyo Day theme"),
    ("Ctrl+Alt+3", "Custom theme"),
];

/// Deck colour palette preset (D1 reads it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeckPalette {
    /// A warm (amber/orange), B cool (blue/cyan).
    #[default]
    WarmCool,
    /// Classic cyan A / purple B.
    Classic,
}

/// Where the playhead sits in the deck waveform (D1 reads it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Playhead {
    #[default]
    Centre,
    LeftThird,
}

/// Tempo fader range in ± percent (D1 reads it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TempoRange {
    Six,
    #[default]
    Ten,
    Sixteen,
}

/// Every user-configurable option except the theme (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub waveform_style: Style,
    /// Anti-aliased waveform edges.
    pub smooth: bool,
    /// Per-track display auto-gain (stored; D1).
    pub auto_gain: bool,
    /// Beat and bar lines over the detail waveform.
    pub beat_lines: bool,
    /// Detail window length in bars, one of [`ZOOM_BARS`].
    pub zoom_bars: usize,
    pub deck_palette: DeckPalette,
    pub playhead: Playhead,
    /// Phase offset readout between the decks (stored; D1).
    pub show_phase: bool,
    pub tempo_range: TempoRange,
    /// D1 deck-area sections (collapsible; the mixer is hidden by default).
    pub sections: DeckSections,
    /// D4: the crossfader inside the mixer is shown (and in the path).
    pub crossfader: bool,
    /// D4 crossfader curve.
    pub xfader_curve: Curve,
    /// Opt-in compact Rekordbox-inspired transport pairs.
    pub simple_dj_keys: bool,
}

/// Which optional deck-area sections are expanded (D1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeckSections {
    pub detail: bool,
    pub overviews: bool,
    pub strip: bool,
    pub mixer: bool,
}

impl Default for DeckSections {
    fn default() -> Self {
        Self { detail: true, overviews: true, strip: true, mixer: false }
    }
}

impl DeckSections {
    /// Section names in display order (tooltips, AccessKit, Settings).
    pub const NAMES: [&'static str; 4] = ["Detail waveform", "Overviews", "Control strip", "Mixer / crossfader"];
    const KEYS: [&'static str; 4] = ["sec_detail", "sec_overviews", "sec_strip", "sec_mixer"];

    pub fn get(self, index: usize) -> bool {
        [self.detail, self.overviews, self.strip, self.mixer][index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut bool {
        match index {
            0 => &mut self.detail,
            1 => &mut self.overviews,
            2 => &mut self.strip,
            _ => &mut self.mixer,
        }
    }

    /// The combination with bit `i` = section `i` expanded (tests).
    #[cfg(test)]
    pub fn from_bits(bits: u8) -> Self {
        let mut s = Self::default();
        for i in 0..4 {
            *s.get_mut(i) = bits & (1 << i) != 0;
        }
        s
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            waveform_style: Style::Mono,
            smooth: true,
            auto_gain: true,
            beat_lines: true,
            zoom_bars: 4,
            deck_palette: DeckPalette::WarmCool,
            playhead: Playhead::Centre,
            show_phase: true,
            tempo_range: TempoRange::Ten,
            sections: DeckSections::default(),
            crossfader: true,
            xfader_curve: Curve::Smooth,
            simple_dj_keys: false,
        }
    }
}

const STYLES: [(Style, &str); 3] = [(Style::Mono, "mono"), (Style::ThreeBand, "3-band"), (Style::Rgb, "rgb")];
const PALETTES: [(DeckPalette, &str); 2] = [(DeckPalette::WarmCool, "warm-cool"), (DeckPalette::Classic, "classic")];
const PLAYHEADS: [(Playhead, &str); 2] = [(Playhead::Centre, "centre"), (Playhead::LeftThird, "left-third")];
const TEMPOS: [(TempoRange, &str); 3] = [(TempoRange::Six, "6"), (TempoRange::Ten, "10"), (TempoRange::Sixteen, "16")];
const CURVES: [(Curve, &str); 3] = [(Curve::Smooth, "smooth"), (Curve::Sharp, "sharp"), (Curve::Cut, "cut")];

fn name_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table.iter().find(|(v, _)| *v == value).map_or("", |(_, n)| n)
}

fn parse_in<T: Copy>(table: &[(T, &str)], raw: &str) -> Option<T> {
    table.iter().find(|(_, n)| *n == raw).map(|(v, _)| *v)
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

impl Settings {
    /// `name=value;…` (stable names, independent of labels).
    pub fn encode(&self) -> String {
        let b = |v: bool| if v { "1" } else { "0" };
        let sections: String = (0..4)
            .map(|i| format!(";{}={}", DeckSections::KEYS[i], b(self.sections.get(i))))
            .collect();
        format!(
            "style={};smooth={};auto_gain={};beat_lines={};zoom={};deck_palette={};playhead={};phase={};tempo={};sec_xfader={};xf_curve={};simple_keys={}",
            name_of(&STYLES, self.waveform_style),
            b(self.smooth),
            b(self.auto_gain),
            b(self.beat_lines),
            self.zoom_bars,
            name_of(&PALETTES, self.deck_palette),
            name_of(&PLAYHEADS, self.playhead),
            b(self.show_phase),
            name_of(&TEMPOS, self.tempo_range),
            b(self.crossfader),
            name_of(&CURVES, self.xfader_curve),
            b(self.simple_dj_keys),
        ) + &sections
    }

    /// Never fails: unknown names are ignored and an invalid value keeps
    /// that field's default.
    pub fn decode(raw: &str) -> Self {
        let mut s = Self::default();
        for (key, value) in raw.split(';').filter_map(|pair| pair.split_once('=')) {
            let value = value.trim();
            match key.trim() {
                "simple_keys" => s.simple_dj_keys = parse_bool(value).unwrap_or(s.simple_dj_keys),
                "style" => s.waveform_style = parse_in(&STYLES, value).unwrap_or(s.waveform_style),
                "smooth" => s.smooth = parse_bool(value).unwrap_or(s.smooth),
                "auto_gain" => s.auto_gain = parse_bool(value).unwrap_or(s.auto_gain),
                "beat_lines" => s.beat_lines = parse_bool(value).unwrap_or(s.beat_lines),
                "zoom" => {
                    s.zoom_bars = value
                        .parse()
                        .ok()
                        .filter(|z| ZOOM_BARS.contains(z))
                        .unwrap_or(s.zoom_bars)
                }
                "deck_palette" => s.deck_palette = parse_in(&PALETTES, value).unwrap_or(s.deck_palette),
                "playhead" => s.playhead = parse_in(&PLAYHEADS, value).unwrap_or(s.playhead),
                "phase" => s.show_phase = parse_bool(value).unwrap_or(s.show_phase),
                "tempo" => s.tempo_range = parse_in(&TEMPOS, value).unwrap_or(s.tempo_range),
                "sec_xfader" => s.crossfader = parse_bool(value).unwrap_or(s.crossfader),
                "xf_curve" => s.xfader_curve = parse_in(&CURVES, value).unwrap_or(s.xfader_curve),
                other => {
                    if let Some(i) = DeckSections::KEYS.iter().position(|k| *k == other) {
                        let slot = s.sections.get_mut(i);
                        *slot = parse_bool(value).unwrap_or(*slot);
                    }
                }
            }
        }
        s
    }

    /// Restore from eframe storage (defaults when absent).
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        storage
            .and_then(|s| s.get_string(SETTINGS_KEY))
            .map_or_else(Self::default, |raw| Self::decode(&raw))
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(SETTINGS_KEY, self.encode());
    }
}

/// Window state that is not a setting (focus return, test rects).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SettingsUi {
    pub(crate) open: bool,
    pub(crate) tab: usize,
    /// Widget focused before the window opened; refocused on close.
    return_focus: Option<egui::Id>,
    /// Focus the first control on the frame after opening.
    focus_first: bool,
    /// Last frame's window and control rects (tests drive real input).
    pub(crate) layout: SettingsLayout,
}

#[derive(Clone, Debug, Default, Copy)]
pub(crate) struct SettingsLayout {
    pub(crate) window: Option<egui::Rect>,
    pub(crate) tabs: Option<[egui::Rect; 4]>,
    /// Theme segment cells: Tokyo Night, Tokyo Day, Custom.
    pub(crate) theme: Option<[egui::Rect; 3]>,
    pub(crate) style: Option<[egui::Rect; 3]>,
    pub(crate) smooth: Option<egui::Rect>,
    pub(crate) beat_lines: Option<egui::Rect>,
    pub(crate) zoom: Option<[egui::Rect; 4]>,
    pub(crate) reset: Option<egui::Rect>,
    /// D1 deck-section toggles (detail, overviews, strip, mixer).
    pub(crate) sections: Option<[egui::Rect; 4]>,
    /// D4 crossfader curve cells (Smooth, Sharp, Cut).
    pub(crate) curve: Option<[egui::Rect; 3]>,
    pub(crate) first_control: Option<egui::Id>,
}

fn rects<const N: usize>(responses: &[egui::Response]) -> [egui::Rect; N] {
    core::array::from_fn(|i| responses.get(i).map_or(egui::Rect::NOTHING, |r| r.rect))
}

impl LibraryApp {
    /// Ctrl+, toggles the window; Esc closes it while open. Runs before any
    /// widget so a focused text field cannot swallow the chord.
    pub(crate) fn handle_settings_shortcuts(&mut self, ctx: &egui::Context) {
        let open = self.settings_ui.open;
        let (toggle, escape) = ctx.input_mut(|input| {
            let toggle = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma);
            let escape = open && !toggle && input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            (toggle, escape)
        });
        if toggle {
            self.set_settings_open(ctx, !open);
        } else if escape {
            self.set_settings_open(ctx, false);
        }
    }

    /// Open (remembering the focused widget) or close (returning focus).
    pub(crate) fn set_settings_open(&mut self, ctx: &egui::Context, open: bool) {
        let state = &mut self.settings_ui;
        if open && !state.open {
            state.return_focus = ctx.memory(|m| m.focused());
            state.focus_first = true;
        } else if !open && state.open {
            let back = state.return_focus.take();
            ctx.memory_mut(|m| match back {
                Some(id) => m.request_focus(id),
                None => {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                }
            });
        }
        state.open = open;
    }

    /// Reset every setting and the theme mode (custom colours are kept).
    pub(crate) fn reset_settings(&mut self) {
        self.settings = Settings::default();
        self.select_theme(ThemeMode::default());
    }

    pub(crate) fn settings_window(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
        self.settings_ui.layout = SettingsLayout::default();
        if !self.settings_ui.open {
            return;
        }
        let mut open = true;
        let mut layout = SettingsLayout::default();
        let shown = egui::Window::new(WINDOW_TITLE)
            .id(egui::Id::new("oxidj-settings"))
            .open(&mut open)
            .collapsible(false)
            // Leave room for the title bar and window frame on small screens.
            .default_height((ctx.screen_rect().height() - 64.0).max(1.0))
            .max_height((ctx.screen_rect().height() - 64.0).max(1.0))
            .vscroll(true)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                self.settings_contents(ui, tokens, &mut layout);
                // Native keyboard focus can advance to a clipped widget;
                // keep that control visible within the window's scroll area.
                if let Some(response) = ui.memory(|m| m.focused()).and_then(|id| ui.ctx().read_response(id)) {
                    if response.gained_focus() && response.layer_id == ui.layer_id() && !ui.clip_rect().contains_rect(response.rect) {
                        ui.scroll_to_rect(response.rect, None);
                    }
                }
            });
        layout.window = shown.map(|w| w.response.rect);
        if self.settings_ui.focus_first {
            if let Some(id) = layout.first_control {
                ctx.memory_mut(|m| m.request_focus(id));
                self.settings_ui.focus_first = false;
            }
        }
        self.settings_ui.layout = layout;
        if !open {
            self.set_settings_open(ctx, false);
        }
    }

    fn settings_contents(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens, layout: &mut SettingsLayout) {
        let mut tabs = [egui::Rect::NOTHING; 4];
        ui.horizontal_wrapped(|ui| {
            for (i, name) in ["Appearance", "Waveform", "Deck view", "Keyboard"].iter().enumerate() {
                let response = ui.selectable_label(self.settings_ui.tab == i, *name);
                tabs[i] = response.rect;
                if response.clicked() {
                    self.settings_ui.tab = i;
                }
                if i == 0 {
                    layout.first_control = Some(response.id);
                }
            }
        });
        layout.tabs = Some(tabs);
        ui.separator();
        if self.settings_ui.tab == 0 {
            SectionHeading { title: "Appearance" }.show(ui, tokens);
            let themes = [ThemeMode::TokyoNight, ThemeMode::TokyoDay, ThemeMode::Custom];
            let mut index = themes.iter().position(|t| *t == self.theme).unwrap_or(0);
            let cells = themes.map(ThemeMode::label);
            let responses = segmented_row(ui, tokens, "Theme", &cells, &mut index);
            layout.theme = Some(rects(&responses));
            if themes[index] != self.theme {
                self.select_theme(themes[index]);
            }
            if ui.button("Edit custom palette…").clicked() {
                self.select_theme(ThemeMode::Custom);
            }
        }

        if self.settings_ui.tab == 1 {
            SectionHeading { title: "Waveform" }.show(ui, tokens);
            let s = &mut self.settings;
            let mut index = Style::ALL.iter().position(|v| *v == s.waveform_style).unwrap_or(0);
            let responses = segmented_row(ui, tokens, "Style", &Style::ALL.map(Style::label), &mut index);
            s.waveform_style = Style::ALL[index];
            layout.style = Some(rects(&responses));
            layout.smooth = Some(toggle_row(ui, tokens, "Smooth edges", &mut s.smooth).rect);
            toggle_row(ui, tokens, "Auto-gain (decks)", &mut s.auto_gain);
            layout.beat_lines = Some(toggle_row(ui, tokens, "Beat and bar lines", &mut s.beat_lines).rect);
            let mut index = ZOOM_BARS.iter().position(|z| *z == s.zoom_bars).unwrap_or(2);
            let labels = ["1 bar", "2 bars", "4 bars", "8 bars"];
            let responses = segmented_row(ui, tokens, "Default zoom", &labels, &mut index);
            s.zoom_bars = ZOOM_BARS[index];
            layout.zoom = Some(rects(&responses));
        }

        if self.settings_ui.tab == 2 {
            SectionHeading { title: "Deck view" }.show(ui, tokens);
            let s = &mut self.settings;
            let mut pick = |label: &str, cells: &[&str], index: &mut usize| {
                segmented_row(ui, tokens, label, cells, index);
            };
            let mut i = usize::from(s.deck_palette == DeckPalette::Classic);
            pick("Deck colours", &["Warm / Cool", "Classic cyan / purple"], &mut i);
            s.deck_palette = PALETTES[i].0;
            let mut i = usize::from(s.playhead == Playhead::LeftThird);
            pick("Playhead", &["Centre", "Left third"], &mut i);
            s.playhead = PLAYHEADS[i].0;
            let mut i = TEMPOS.iter().position(|(t, _)| *t == s.tempo_range).unwrap_or(1);
            pick("Tempo range", &["±6 %", "±10 %", "±16 %"], &mut i);
            s.tempo_range = TEMPOS[i].0;
            toggle_row(ui, tokens, "Phase readout", &mut s.show_phase);
            let mut sections = [egui::Rect::NOTHING; 4];
            for (i, name) in DeckSections::NAMES.iter().enumerate() {
                sections[i] = toggle_row(ui, tokens, &format!("Show {name}"), s.sections.get_mut(i)).rect;
            }
            layout.sections = Some(sections);
            let mut i = CURVES.iter().position(|(c, _)| *c == s.xfader_curve).unwrap_or(0);
            let responses = segmented_row(ui, tokens, "Crossfader curve", &["Smooth", "Sharp", "Cut"], &mut i);
            s.xfader_curve = CURVES[i].0;
            layout.curve = Some(rects(&responses));
        }

        if self.settings_ui.tab == 3 {
            SectionHeading { title: "Keyboard" }.show(ui, tokens);
            let s = &mut self.settings;
            toggle_row(ui, tokens, "Simple DJ keys (Rekordbox-inspired)", &mut s.simple_dj_keys)
                .on_hover_text("Z / X: cue / play A; N / M: cue / play B. A / B loads the selected track. Esc cancels a track drag. Not a complete Rekordbox mapping.");
            egui::Grid::new("settings-shortcuts").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                for (keys, action) in SHORTCUTS {
                    ui.label(egui::RichText::new(keys).monospace().color(tokens.text));
                    ui.label(egui::RichText::new(action).color(tokens.muted_text));
                    ui.end_row();
                }
            });
        }
        ui.add_space(8.0);
        let reset = ui.button("Reset to defaults").on_hover_text("Restore every setting and the Tokyo Night theme");
        layout.reset = Some(reset.rect);
        if reset.clicked() {
            self.reset_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryStorage(HashMap<String, String>);

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn defaults_when_storage_is_absent_or_empty() {
        assert_eq!(Settings::load(None), Settings::default());
        assert_eq!(Settings::load(Some(&MemoryStorage::default())), Settings::default());
        let d = Settings::default();
        assert_eq!((d.waveform_style, d.smooth, d.beat_lines, d.zoom_bars), (Style::Mono, true, true, 4));
        assert_eq!((d.deck_palette, d.playhead, d.tempo_range), (DeckPalette::WarmCool, Playhead::Centre, TempoRange::Ten));
    }

    #[test]
    fn every_field_round_trips_through_storage() {
        let edited = Settings {
            waveform_style: Style::Rgb,
            smooth: false,
            auto_gain: false,
            beat_lines: false,
            zoom_bars: 8,
            deck_palette: DeckPalette::Classic,
            playhead: Playhead::LeftThird,
            show_phase: false,
            tempo_range: TempoRange::Sixteen,
            sections: DeckSections { detail: false, overviews: false, strip: false, mixer: true },
            crossfader: false,
            xfader_curve: Curve::Cut,
            simple_dj_keys: true,
        };
        let mut storage = MemoryStorage::default();
        edited.save(&mut storage);
        assert_eq!(Settings::load(Some(&storage)), edited);
        for style in Style::ALL {
            let s = Settings { waveform_style: style, ..Settings::default() };
            assert_eq!(Settings::decode(&s.encode()), s);
        }
        for zoom in ZOOM_BARS {
            let s = Settings { zoom_bars: zoom, ..Settings::default() };
            assert_eq!(Settings::decode(&s.encode()), s);
        }
        for (curve, _) in CURVES {
            for crossfader in [false, true] {
                let s = Settings { xfader_curve: curve, crossfader, ..Settings::default() };
                assert_eq!(Settings::decode(&s.encode()), s);
            }
        }
        for bits in 0..16 {
            let s = Settings { sections: DeckSections::from_bits(bits), ..Settings::default() };
            assert_eq!(Settings::decode(&s.encode()), s, "sections {bits:04b}");
        }
        // Stores written before D1 keep the section defaults (mixer hidden).
        let old = "style=mono;smooth=1;auto_gain=1;beat_lines=1;zoom=4;deck_palette=warm-cool;playhead=centre;phase=1;tempo=10";
        assert_eq!(Settings::decode(old).sections, DeckSections::default());
        assert!(!DeckSections::default().mixer);
        // ...and D4's defaults: crossfader shown, Smooth curve.
        assert!(Settings::decode(old).crossfader);
        assert_eq!(Settings::decode(old).xfader_curve, Curve::Smooth);
        assert_eq!(Settings::decode("xf_curve=wobbly").xfader_curve, Curve::Smooth);
    }

    #[test]
    fn corrupt_unknown_and_old_values_fall_back_per_field() {
        let d = Settings::default();
        for raw in ["", "garbage", ";;;==", "\u{0}\u{fffd}", "style", "zoom=4=4"] {
            let s = Settings::decode(raw);
            // "zoom=4=4" splits once: value "4=4" is invalid → default.
            assert_eq!(s, d, "{raw:?}");
        }
        // Invalid values keep defaults; valid neighbours still apply;
        // unknown (future/removed) keys are ignored.
        let s = Settings::decode("style=wavy;smooth=0;zoom=3;tempo=12;future_option=1;playhead=left-third");
        assert_eq!(s.waveform_style, d.waveform_style);
        assert!(!s.smooth);
        assert_eq!(s.zoom_bars, d.zoom_bars);
        assert_eq!(s.tempo_range, d.tempo_range);
        assert_eq!(s.playhead, Playhead::LeftThird);
        let mut storage = MemoryStorage::default();
        eframe::Storage::set_string(&mut storage, SETTINGS_KEY, "beat_lines=yes;zoom=-1".into());
        assert_eq!(Settings::load(Some(&storage)), d);
    }
}
