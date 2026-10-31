use eframe::egui::{self, Color32, RichText, Stroke};

use crate::theme::ColorTokens;

/// A native egui button styled as the library's primary action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryActionButton<'a> {
    pub label: &'a str,
}

impl PrimaryActionButton<'_> {
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        ui.add(
            egui::Button::new(RichText::new(self.label).color(tokens.text))
                .fill(tokens.primary.linear_multiply(0.18))
                .stroke(Stroke::new(1.0_f32, tokens.primary)),
        )
    }
}

/// A compact, themed label for a named area of the library UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionHeading<'a> {
    pub title: &'a str,
}

impl SectionHeading<'_> {
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) {
        ui.horizontal(|ui| {
            ui.add(
                egui::Separator::default()
                    .vertical()
                    .spacing(2.0)
                    .shrink(2.0),
            );
            ui.label(RichText::new(self.title).strong().color(tokens.text));
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Success,
    Warning,
    Error,
}

impl StatusKind {
    fn color(self, tokens: ColorTokens) -> Color32 {
        match self {
            Self::Success => tokens.success,
            Self::Warning => tokens.warning,
            Self::Error => tokens.error,
        }
    }
}

/// A readable status label with a semantic token-colored accent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusBadge<'a> {
    pub label: &'a str,
    pub kind: StatusKind,
}

impl StatusBadge<'_> {
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        let accent = self.kind.color(tokens);
        egui::Frame::new()
            .fill(accent.linear_multiply(0.16))
            .stroke(Stroke::new(1.0_f32, accent))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(7, 3))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(self.label)
                        .small()
                        .strong()
                        .color(tokens.text),
                )
            })
            .response
            .on_hover_text(self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn status_kinds_resolve_to_semantic_theme_tokens() {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            let tokens = mode.tokens();
            assert_eq!(StatusKind::Success.color(tokens), tokens.success);
            assert_eq!(StatusKind::Warning.color(tokens), tokens.warning);
            assert_eq!(StatusKind::Error.color(tokens), tokens.error);
        }
    }

    #[test]
    fn components_render_without_panicking_in_both_presets() {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let tokens = mode.tokens();
                    SectionHeading {
                        title: "Track details",
                    }
                    .show(ui, tokens);
                    for kind in [StatusKind::Success, StatusKind::Warning, StatusKind::Error] {
                        StatusBadge {
                            label: "Available",
                            kind,
                        }
                        .show(ui, tokens);
                    }
                });
            });
        }
    }

    #[test]
    fn primary_action_is_clickable_in_both_presets() {
        for mode in [ThemeMode::TokyoNight, ThemeMode::TokyoDay] {
            let ctx = egui::Context::default();
            let tokens = mode.tokens();
            let mut rect = egui::Rect::NOTHING;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    rect = PrimaryActionButton {
                        label: "Import references",
                    }
                    .show(ui, tokens)
                    .rect;
                });
            });
            let pos = rect.center();
            let mut clicked = false;
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::default(),
                        },
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        clicked = PrimaryActionButton {
                            label: "Import references",
                        }
                        .show(ui, tokens)
                        .clicked();
                    });
                },
            );
            assert!(clicked, "native egui button should report clicks");
        }
    }
}
