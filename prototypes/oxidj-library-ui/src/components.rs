use eframe::egui::{self, Color32, RichText, Stroke};

use crate::theme::ColorTokens;

/// A native egui button styled as the library's primary action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryActionButton<'a> {
    pub label: &'a str,
}

impl PrimaryActionButton<'_> {
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        let response = ui.add(
            egui::Button::new(RichText::new(self.label).color(tokens.text))
                .fill(tokens.primary.linear_multiply(0.18))
                .stroke(Stroke::new(1.0_f32, tokens.primary)),
        );
        let active = response.hovered() || response.has_focus();
        let progress = ui.ctx().animate_bool(response.id.with("cyan-underline"), active);
        if progress > 0.0 {
            let rect = response.rect;
            let width = rect.width() * progress;
            let underline = egui::Rect::from_min_max(
                egui::pos2(rect.center().x - width * 0.5, rect.bottom() - 2.0),
                egui::pos2(rect.center().x + width * 0.5, rect.bottom() - 1.0),
            );
            ui.painter().rect_filled(underline, 0.5, tokens.secondary);
        }
        response
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
    fn primary_action_paints_layout_sized_cyan_underline_when_hovered() {
        let ctx = egui::Context::default();
        let tokens = ThemeMode::TokyoNight.tokens();
        let mut button_rect = egui::Rect::NOTHING;
        let _ = ctx.run(
            egui::RawInput {
                time: Some(0.0),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    button_rect = PrimaryActionButton { label: "Import" }
                        .show(ui, tokens)
                        .rect;
                });
            },
        );
        let pos = button_rect.center();
        let output = ctx.run(
            egui::RawInput {
                time: Some(1.0),
                events: vec![egui::Event::PointerMoved(pos)],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    button_rect = PrimaryActionButton { label: "Import" }
                        .show(ui, tokens)
                        .rect;
                });
            },
        );

        let underline = output.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Rect(rect)
                if rect.fill == tokens.secondary && rect.rect.bottom() == button_rect.bottom() - 1.0 =>
            {
                Some(rect.rect)
            }
            _ => None,
        });
        let underline = underline.expect("hovered action paints Tokyo cyan underline");
        assert!((underline.width() - button_rect.width()).abs() < 0.01);
        assert_eq!(underline.center().x, button_rect.center().x);
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
