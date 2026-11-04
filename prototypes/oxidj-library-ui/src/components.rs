use eframe::egui::{self, Color32, RichText, Stroke};

use crate::theme::ColorTokens;

/// A native egui button styled as the library's primary action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryActionButton<'a> {
    pub label: &'a str,
}

impl PrimaryActionButton<'_> {
    /// Draw the button; hover/focus grows a cyan underline from the centre.
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        let response = ui.add(
            egui::Button::new(RichText::new(self.label).color(tokens.text))
                .fill(tokens.primary.linear_multiply(0.18))
                .stroke(Stroke::new(1.0_f32, tokens.primary)),
        );
        let active = response.hovered() || response.has_focus();
        let progress = ui
            .ctx()
            .animate_bool(response.id.with("cyan-underline"), active);
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
    /// Draw the heading: a 3 px primary accent bar before strong body text,
    /// with a little air above so sections read as groups.
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let height = ui.text_style_height(&egui::TextStyle::Body);
            let (bar, _) = ui.allocate_exact_size(egui::vec2(3.0, height), egui::Sense::hover());
            ui.painter().rect_filled(bar, 1.5, tokens.primary);
            // Truncated (full title on hover): a heading must never widen
            // its panel past the panel's width.
            ui.add(
                egui::Label::new(RichText::new(self.title).strong().color(tokens.text)).truncate(),
            )
            .on_hover_text(self.title);
        });
    }
}

/// Semantic outcome of a status message.
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
    /// Draw the badge: token-tinted fill, accent outline, small strong text.
    ///
    /// The frame sits in a control-height row so a grid cell cannot stretch
    /// it (a bare `Frame` in a `Grid` grew by its margin every frame).
    pub fn show(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        ui.horizontal(|ui| self.frame(ui, tokens)).inner
    }

    fn frame(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
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

impl StatusBadge<'_> {
    /// The same badge sized to a body-text row (W11): the control-height
    /// badge made its details-grid row taller than the rows around it.
    pub fn show_inline(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        let accent = self.kind.color(tokens);
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(accent.linear_multiply(0.16))
                .stroke(Stroke::new(1.0_f32, accent))
                .corner_radius(3.0)
                .inner_margin(egui::Margin::symmetric(6, 0))
                .show(ui, |ui| {
                    ui.label(RichText::new(self.label).small().color(tokens.text))
                })
                .response
                .on_hover_text(self.label)
        })
        .inner
    }
}

/// Horizontal padding of a segmented-toggle cell.
const SEGMENT_PADDING: f32 = 10.0;
/// Corner radius shared by segmented groups and the search field.
pub const CONTROL_RADIUS: f32 = 4.0;

/// A segmented toggle group (W11): adjacent cells inside one outline, a
/// hairline between cells, the "on" cells filled with the selection tint
/// plus a 2 px primary underline, off cells in muted text. Each cell is a
/// native egui widget (selectable, keyboard-focusable, AccessKit
/// `selected`); the caller flips its state on `clicked()`.
///
/// Cells are 1 px apart (the separator sits in that gap), so their hit
/// rects never touch. A full-strength primary fill was not used: text on
/// it is below 4.5:1 in Tokyo Day (see the v12 README).
pub fn segmented_toggles(
    ui: &mut egui::Ui,
    tokens: ColorTokens,
    cells: &[(&str, bool)],
) -> Vec<egui::Response> {
    let height = crate::theme::CONTROL_HEIGHT;
    let galleys: Vec<_> = cells
        .iter()
        .map(|(label, on)| {
            // Explicit colour: `override_text_color` would otherwise be baked in.
            let color = if *on { tokens.text } else { tokens.muted_text };
            egui::WidgetText::from(RichText::new(*label).color(color)).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            )
        })
        .collect();
    let widths: Vec<f32> = galleys
        .iter()
        .map(|g| (g.size().x + 2.0 * SEGMENT_PADDING).ceil())
        .collect();
    let total = widths.iter().sum::<f32>() + (cells.len().saturating_sub(1)) as f32;
    // Unique per call site (two decks both have a "Play" cell).
    let base = ui.next_auto_id();
    let (group, _) = ui.allocate_exact_size(egui::vec2(total, height), egui::Sense::hover());
    let last = cells.len().saturating_sub(1);
    let mut x = group.left();
    let mut responses = Vec::with_capacity(cells.len());
    for (index, (((label, on), galley), width)) in
        cells.iter().zip(galleys).zip(widths).enumerate()
    {
        let rect = egui::Rect::from_min_size(egui::pos2(x, group.top()), egui::vec2(width, height));
        x += width + 1.0;
        let id = base.with(("segment", index));
        let response = ui.interact(rect, id, egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, *on, *label)
        });
        let r = CONTROL_RADIUS as u8;
        let radius = egui::CornerRadius {
            nw: if index == 0 { r } else { 0 },
            sw: if index == 0 { r } else { 0 },
            ne: if index == last { r } else { 0 },
            se: if index == last { r } else { 0 },
        };
        let painter = ui.painter();
        if *on {
            painter.rect_filled(rect, radius, crate::theme::selection_fill(tokens));
            let bar = egui::Rect::from_x_y_ranges(rect.x_range(), rect.bottom() - 2.0..=rect.bottom());
            painter.rect_filled(bar, 0.0, tokens.primary);
        } else if response.hovered() {
            painter.rect_filled(rect, radius, tokens.surface);
        }
        if response.has_focus() {
            painter.rect_stroke(rect, radius, Stroke::new(2.0_f32, tokens.focus), egui::StrokeKind::Inside);
        } else if response.hovered() {
            painter.rect_stroke(rect, radius, Stroke::new(1.0_f32, tokens.focus), egui::StrokeKind::Inside);
        }
        if index < last {
            painter.vline(rect.right() + 0.5, group.y_range(), Stroke::new(1.0_f32, tokens.border));
        }
        let pos = rect.center() - galley.size() / 2.0;
        painter.galley(pos, galley, tokens.text);
        responses.push(response);
    }
    ui.painter().rect_stroke(
        group,
        CONTROL_RADIUS,
        Stroke::new(1.0_f32, tokens.border),
        egui::StrokeKind::Inside,
    );
    responses
}

/// Single-line text field at the shared control height, text vertically
/// centred: `ui.text_edit_singleline` sizes to the font (~20 px), below the
/// 24 px target and shorter than the toolbar fields beside it.
pub fn text_field(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .min_size(egui::vec2(0.0, crate::theme::CONTROL_HEIGHT))
            .vertical_align(egui::Align::Center),
    )
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

        let underline = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Rect(rect)
                    if rect.fill == tokens.secondary
                        && rect.rect.bottom() == button_rect.bottom() - 1.0 =>
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
