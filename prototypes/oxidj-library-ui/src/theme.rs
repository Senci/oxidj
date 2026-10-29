use eframe::egui::{self, Color32, Stroke, Visuals};

const THEME_KEY: &str = "ui_theme";
const CUSTOM_KEY: &str = "ui_custom_tokens";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    TokyoNight,
    TokyoDay,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorTokens {
    pub background: Color32,
    pub surface: Color32,
    pub text: Color32,
    pub muted_text: Color32,
    pub primary: Color32,
    pub secondary: Color32,
    pub border: Color32,
    pub focus: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

impl ThemeMode {
    pub fn tokens(self) -> ColorTokens {
        match self {
            Self::TokyoNight => ColorTokens {
                background: Color32::from_rgb(0x1a, 0x1b, 0x26),
                surface: Color32::from_rgb(0x24, 0x25, 0x36),
                text: Color32::from_rgb(0xc0, 0xca, 0xf5),
                muted_text: Color32::from_rgb(0xa9, 0xb1, 0xd6),
                primary: Color32::from_rgb(0xbb, 0x9a, 0xf7),
                secondary: Color32::from_rgb(0x7d, 0xcf, 0xff),
                border: Color32::from_rgb(0x3b, 0x3d, 0x55),
                focus: Color32::from_rgb(0x7d, 0xcf, 0xff),
                success: Color32::from_rgb(0x9e, 0xce, 0x6a),
                warning: Color32::from_rgb(0xe0, 0xaf, 0x68),
                error: Color32::from_rgb(0xf7, 0x76, 0x8e),
            },
            Self::TokyoDay => ColorTokens {
                background: Color32::from_rgb(0xe1, 0xe2, 0xe7),
                surface: Color32::from_rgb(0xf4, 0xf4, 0xf5),
                text: Color32::from_rgb(0x37, 0x60, 0xbf),
                muted_text: Color32::from_rgb(0x61, 0x71, 0x9c),
                primary: Color32::from_rgb(0x98, 0x54, 0xf1),
                secondary: Color32::from_rgb(0x00, 0x71, 0x97),
                border: Color32::from_rgb(0xb8, 0xbe, 0xd0),
                focus: Color32::from_rgb(0x00, 0x71, 0x97),
                success: Color32::from_rgb(0x58, 0x75, 0x39),
                warning: Color32::from_rgb(0x8c, 0x6c, 0x3d),
                error: Color32::from_rgb(0xc6, 0x43, 0x43),
            },
            Self::Custom => Self::TokyoNight.tokens(),
        }
    }

    /// Replace custom tokens with the selected Tokyo preset.
    pub fn reset_custom_tokens(preset: Self) -> ColorTokens {
        match preset {
            Self::TokyoNight | Self::TokyoDay => preset.tokens(),
            Self::Custom => Self::TokyoNight.tokens(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TokyoNight => "Tokyo Night",
            Self::TokyoDay => "Tokyo Day",
            Self::Custom => "Custom",
        }
    }

    fn persisted_name(self) -> &'static str {
        match self {
            Self::TokyoNight => "tokyo-night",
            Self::TokyoDay => "tokyo-day",
            Self::Custom => "custom",
        }
    }

    fn from_persisted(value: Option<&str>) -> Self {
        match value {
            Some("tokyo-day") => Self::TokyoDay,
            Some("custom") => Self::Custom,
            _ => Self::TokyoNight,
        }
    }

    pub fn load(storage: Option<&dyn eframe::Storage>) -> (Self, ColorTokens) {
        let mode = Self::from_persisted(storage.and_then(|s| s.get_string(THEME_KEY)).as_deref());
        let tokens = storage
            .and_then(|s| s.get_string(CUSTOM_KEY))
            .and_then(|raw| ColorTokens::decode(&raw))
            .unwrap_or_else(|| Self::TokyoNight.tokens());
        (mode, tokens)
    }

    pub fn save(self, tokens: ColorTokens, storage: &mut dyn eframe::Storage) {
        storage.set_string(THEME_KEY, self.persisted_name().to_owned());
        storage.set_string(CUSTOM_KEY, tokens.encode());
    }

    pub fn apply(self, tokens: ColorTokens, ctx: &egui::Context) {
        let t = if self == Self::Custom {
            tokens
        } else {
            self.tokens()
        };
        let mut visuals = match self {
            Self::TokyoDay => Visuals::light(),
            Self::TokyoNight | Self::Custom => Visuals::dark(),
        };
        visuals.panel_fill = t.background;
        visuals.window_fill = t.surface;
        visuals.window_stroke = Stroke::new(1.0_f32, t.border);
        visuals.extreme_bg_color = t.background;
        visuals.faint_bg_color = t.surface;
        visuals.override_text_color = Some(t.text);
        visuals.weak_text_color = Some(t.muted_text);
        visuals.hyperlink_color = t.secondary;
        visuals.selection.bg_fill = t.primary.linear_multiply(0.28);
        visuals.selection.stroke = Stroke::new(1.0_f32, t.focus);
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.bg_fill = t.surface;
            widget.weak_bg_fill = t.surface;
            widget.bg_stroke.color = t.border;
            widget.fg_stroke.color = t.text;
        }
        visuals.widgets.hovered.bg_fill = t.surface;
        visuals.widgets.hovered.bg_stroke.color = t.focus;
        visuals.widgets.active.bg_fill = t.surface;
        visuals.widgets.active.bg_stroke.color = t.focus;
        visuals.widgets.open.bg_fill = t.surface;
        visuals.widgets.open.bg_stroke.color = t.focus;
        ctx.set_visuals(visuals);
    }
}

impl ColorTokens {
    fn encode(self) -> String {
        self.as_array()
            .iter()
            .map(|c| format!("{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()))
            .collect::<Vec<_>>()
            .join(",")
    }
    fn decode(raw: &str) -> Option<Self> {
        let values: Vec<Color32> = raw
            .split(',')
            .map(|part| {
                if part.len() != 6 || !part.is_ascii() {
                    return None;
                }
                Some(Color32::from_rgb(
                    u8::from_str_radix(&part[0..2], 16).ok()?,
                    u8::from_str_radix(&part[2..4], 16).ok()?,
                    u8::from_str_radix(&part[4..6], 16).ok()?,
                ))
            })
            .collect::<Option<_>>()?;
        if values.len() != 11 {
            return None;
        }
        Some(Self {
            background: values[0],
            surface: values[1],
            text: values[2],
            muted_text: values[3],
            primary: values[4],
            secondary: values[5],
            border: values[6],
            focus: values[7],
            success: values[8],
            warning: values[9],
            error: values[10],
        })
    }
    fn as_array(self) -> [Color32; 11] {
        [
            self.background,
            self.surface,
            self.text,
            self.muted_text,
            self.primary,
            self.secondary,
            self.border,
            self.focus,
            self.success,
            self.warning,
            self.error,
        ]
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
    fn default_is_tokyo_night_with_purple_and_cyan_accents() {
        assert_eq!(ThemeMode::default(), ThemeMode::TokyoNight);
        assert_eq!(
            ThemeMode::default().tokens().primary,
            Color32::from_rgb(0xbb, 0x9a, 0xf7)
        );
        assert_eq!(
            ThemeMode::default().tokens().secondary,
            Color32::from_rgb(0x7d, 0xcf, 0xff)
        );
        assert_ne!(
            ThemeMode::TokyoDay.tokens().background,
            ThemeMode::TokyoNight.tokens().background
        );
    }

    #[test]
    fn both_palettes_define_focus_and_status_tokens() {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            let tokens = mode.tokens();
            assert_eq!(tokens.focus, tokens.secondary);
            assert_ne!(tokens.success, tokens.background);
            assert_ne!(tokens.warning, tokens.background);
            assert_ne!(tokens.error, tokens.background);
        }
    }

    #[test]
    fn applied_visuals_use_shared_surface_border_and_focus_tokens() {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            let ctx = egui::Context::default();
            mode.apply(mode.tokens(), &ctx);
            let style = ctx.style();
            let visuals = &style.visuals;
            let tokens = mode.tokens();
            assert_eq!(visuals.panel_fill, tokens.background);
            assert_eq!(visuals.window_fill, tokens.surface);
            assert_eq!(visuals.window_stroke.color, tokens.border);
            for widget in [
                &visuals.widgets.noninteractive,
                &visuals.widgets.inactive,
                &visuals.widgets.hovered,
                &visuals.widgets.active,
                &visuals.widgets.open,
            ] {
                assert_eq!(widget.bg_fill, tokens.surface);
                assert_eq!(widget.weak_bg_fill, tokens.surface);
                assert_eq!(widget.fg_stroke.color, tokens.text);
            }
            assert_eq!(visuals.widgets.hovered.bg_stroke.color, tokens.focus);
            assert_eq!(visuals.widgets.active.bg_stroke.color, tokens.focus);
            assert_eq!(visuals.widgets.open.bg_stroke.color, tokens.focus);
        }
    }

    #[test]
    fn theme_selection_round_trips_through_persistent_storage() {
        let mut storage = MemoryStorage::default();
        let custom = ThemeMode::TokyoNight.tokens();
        ThemeMode::TokyoDay.save(custom, &mut storage);
        assert_eq!(
            ThemeMode::load(Some(&storage)),
            (ThemeMode::TokyoDay, custom)
        );
        assert_eq!(
            ThemeMode::load(None),
            (ThemeMode::TokyoNight, ThemeMode::TokyoNight.tokens())
        );
        assert_eq!(
            ThemeMode::from_persisted(Some("unknown")),
            ThemeMode::TokyoNight
        );
    }

    #[test]
    fn edited_custom_palette_persists_and_can_reset_to_either_preset() {
        let mut storage = MemoryStorage::default();
        let mut edited = ThemeMode::TokyoNight.tokens();
        edited.primary = Color32::from_rgb(0x12, 0x34, 0x56);
        ThemeMode::Custom.save(edited, &mut storage);
        assert_eq!(ThemeMode::load(Some(&storage)), (ThemeMode::Custom, edited));
        assert_eq!(
            ThemeMode::TokyoNight.tokens(),
            ThemeMode::default().tokens()
        );
        assert_ne!(edited, ThemeMode::TokyoNight.tokens());
        assert_ne!(ThemeMode::TokyoDay.tokens(), ThemeMode::TokyoNight.tokens());
    }

    #[test]
    fn edited_custom_palette_can_reset_to_each_preset() {
        let mut edited_night = ThemeMode::TokyoNight.tokens();
        edited_night.primary = Color32::from_rgb(0x12, 0x34, 0x56);
        edited_night.background = Color32::from_rgb(0x65, 0x43, 0x21);
        assert_ne!(edited_night, ThemeMode::TokyoNight.tokens());
        let reset_night = ThemeMode::reset_custom_tokens(ThemeMode::TokyoNight);
        assert_eq!(reset_night, ThemeMode::TokyoNight.tokens());

        let mut edited_day = ThemeMode::TokyoNight.tokens();
        edited_day.primary = Color32::from_rgb(0x12, 0x34, 0x56);
        edited_day.background = Color32::from_rgb(0x65, 0x43, 0x21);
        assert_ne!(edited_day, ThemeMode::TokyoDay.tokens());
        let reset_day = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
        assert_eq!(reset_day, ThemeMode::TokyoDay.tokens());
    }

    #[test]
    fn custom_token_encoding_round_trips_and_rejects_invalid_data() {
        let mut custom = ThemeMode::TokyoNight.tokens();
        custom.primary = Color32::from_rgb(1, 2, 3);
        assert_eq!(ColorTokens::decode(&custom.encode()), Some(custom));
        assert_eq!(ColorTokens::decode("invalid"), None);
        assert_eq!(ColorTokens::decode("aé000"), None);
    }
}
