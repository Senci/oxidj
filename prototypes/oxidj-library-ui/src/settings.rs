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

const SETTINGS_KEY: &str = "ui_settings";
/// Default-zoom choices in bars (seconds = 2 × bars without a beat grid).
pub const ZOOM_BARS: [usize; 4] = [1, 2, 4, 8];
/// Window title (also its AccessKit name).
pub const WINDOW_TITLE: &str = "Settings";
/// Read-only shortcut list shown in the Keyboard group.
pub const SHORTCUTS: [(&str, &str); 7] = [
    ("Ctrl+,", "Open or close Settings"),
    ("Esc", "Close Settings"),
    ("Ctrl+B", "Show or hide Navigation"),
    ("Ctrl+I", "Show or hide Details"),
    ("Ctrl+D", "Show or hide Decks"),
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
        }
    }
}

const STYLES: [(Style, &str); 3] = [(Style::Mono, "mono"), (Style::ThreeBand, "3-band"), (Style::Rgb, "rgb")];
const PALETTES: [(DeckPalette, &str); 2] = [(DeckPalette::WarmCool, "warm-cool"), (DeckPalette::Classic, "classic")];
const PLAYHEADS: [(Playhead, &str); 2] = [(Playhead::Centre, "centre"), (Playhead::LeftThird, "left-third")];
const TEMPOS: [(TempoRange, &str); 3] = [(TempoRange::Six, "6"), (TempoRange::Ten, "10"), (TempoRange::Sixteen, "16")];

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
        format!(
            "style={};smooth={};auto_gain={};beat_lines={};zoom={};deck_palette={};playhead={};phase={};tempo={}",
            name_of(&STYLES, self.waveform_style),
            b(self.smooth),
            b(self.auto_gain),
            b(self.beat_lines),
            self.zoom_bars,
            name_of(&PALETTES, self.deck_palette),
            name_of(&PLAYHEADS, self.playhead),
            b(self.show_phase),
            name_of(&TEMPOS, self.tempo_range),
        )
    }

    /// Never fails: unknown names are ignored and an invalid value keeps
    /// that field's default.
    pub fn decode(raw: &str) -> Self {
        let mut s = Self::default();
        for (key, value) in raw.split(';').filter_map(|pair| pair.split_once('=')) {
            let value = value.trim();
            match key.trim() {
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
                _ => {}
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
    /// Theme segment cells: Tokyo Night, Tokyo Day, Custom.
    pub(crate) theme: Option<[egui::Rect; 3]>,
    pub(crate) style: Option<[egui::Rect; 3]>,
    pub(crate) smooth: Option<egui::Rect>,
    pub(crate) beat_lines: Option<egui::Rect>,
    pub(crate) zoom: Option<[egui::Rect; 4]>,
    pub(crate) reset: Option<egui::Rect>,
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
        SectionHeading { title: "Appearance" }.show(ui, tokens);
        let themes = [ThemeMode::TokyoNight, ThemeMode::TokyoDay, ThemeMode::Custom];
        let mut index = themes.iter().position(|t| *t == self.theme).unwrap_or(0);
        let cells = themes.map(ThemeMode::label);
        let responses = segmented_row(ui, tokens, "Theme", &cells, &mut index);
        layout.first_control = responses.first().map(|r| r.id);
        layout.theme = Some(rects(&responses));
        if themes[index] != self.theme {
            self.select_theme(themes[index]);
        }

        SectionHeading { title: "Waveform" }.show(ui, tokens);
        let s = &mut self.settings;
        let mut index = Style::ALL.iter().position(|v| *v == s.waveform_style).unwrap_or(0);
        let responses = segmented_row(ui, tokens, "Style", &Style::ALL.map(Style::label), &mut index);
        s.waveform_style = Style::ALL[index];
        layout.style = Some(rects(&responses));
        layout.smooth = Some(toggle_row(ui, tokens, "Smooth edges", &mut s.smooth).rect);
        toggle_row(ui, tokens, "Auto-gain (decks, D1)", &mut s.auto_gain);
        layout.beat_lines = Some(toggle_row(ui, tokens, "Beat and bar lines", &mut s.beat_lines).rect);
        let mut index = ZOOM_BARS.iter().position(|z| *z == s.zoom_bars).unwrap_or(2);
        let labels = ["1 bar", "2 bars", "4 bars", "8 bars"];
        let responses = segmented_row(ui, tokens, "Default zoom", &labels, &mut index);
        s.zoom_bars = ZOOM_BARS[index];
        layout.zoom = Some(rects(&responses));

        SectionHeading { title: "Decks (used from D1)" }.show(ui, tokens);
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

        SectionHeading { title: "Keyboard" }.show(ui, tokens);
        egui::Grid::new("settings-shortcuts").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (keys, action) in SHORTCUTS {
                ui.label(egui::RichText::new(keys).monospace().color(tokens.text));
                ui.label(egui::RichText::new(action).color(tokens.muted_text));
                ui.end_row();
            }
        });
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
