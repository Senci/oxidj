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
    /// Low-emphasis hint (e.g. a "low" confidence badge): muted-text accent.
    Muted,
}

impl StatusKind {
    fn color(self, tokens: ColorTokens) -> Color32 {
        match self {
            Self::Success => tokens.success,
            Self::Warning => tokens.warning,
            Self::Error => tokens.error,
            Self::Muted => tokens.muted_text,
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
    /// Painted at an exact, non-wrapping size, so a grid row never takes a
    /// stale (wrapped) height from the previous frame.
    pub fn show_inline(self, ui: &mut egui::Ui, tokens: ColorTokens) -> egui::Response {
        let accent = self.kind.color(tokens);
        let galley = egui::WidgetText::from(RichText::new(self.label).small().color(tokens.text)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        );
        let height = ui.text_style_height(&egui::TextStyle::Body);
        let size = egui::vec2(galley.size().x + 12.0, height);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, self.label));
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, accent.linear_multiply(0.16));
        painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, accent), egui::StrokeKind::Inside);
        painter.galley(rect.center() - galley.size() / 2.0, galley, tokens.text);
        response.on_hover_text(self.label)
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

/// Label column of a settings row (fits "Beat and bar lines").
pub const SETTING_LABEL_WIDTH: f32 = 150.0;
/// On/off switch hit target (one control high); the painted track is
/// [`SWITCH_TRACK_HEIGHT`] tall, centred in it.
const SWITCH_SIZE: egui::Vec2 = egui::vec2(40.0, crate::theme::CONTROL_HEIGHT);
const SWITCH_TRACK_HEIGHT: f32 = 20.0;

/// A settings row (D0): `label` in a fixed muted column, then the control
/// added by `add`; the row is one control high. Returns the label response
/// and the control's output.
pub fn setting_row<R>(
    ui: &mut egui::Ui,
    tokens: ColorTokens,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    ui.horizontal(|ui| {
        let label = ui
            .allocate_ui_with_layout(
                egui::vec2(SETTING_LABEL_WIDTH, crate::theme::CONTROL_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(SETTING_LABEL_WIDTH);
                    ui.add(egui::Label::new(RichText::new(label).color(tokens.muted_text)).truncate())
                },
            )
            .inner;
        let inner = add(ui);
        (label, inner)
    })
    .inner
}

/// A labelled segmented choice (one [`segmented_toggles`] group): clicking
/// a cell (or Space/Enter on it) sets `selected`. Each cell keeps its own
/// AccessKit name and `selected` state.
pub fn segmented_row(
    ui: &mut egui::Ui,
    tokens: ColorTokens,
    label: &str,
    options: &[&str],
    selected: &mut usize,
) -> Vec<egui::Response> {
    let (_, responses) = setting_row(ui, tokens, label, |ui| {
        let cells: Vec<_> = options.iter().enumerate().map(|(i, o)| (*o, i == *selected)).collect();
        segmented_toggles(ui, tokens, &cells)
    });
    if let Some(i) = responses.iter().position(egui::Response::clicked) {
        *selected = i;
    }
    responses
}

/// A labelled on/off switch: a native focusable widget (Space/Enter or a
/// click flips `value`; AccessKit checkbox with its checked state). On: the
/// selection tint with the knob in primary at the right; off: surface with
/// a muted knob at the left. The painted shapes stay inside the response rect.
pub fn toggle_row(ui: &mut egui::Ui, tokens: ColorTokens, label: &str, value: &mut bool) -> egui::Response {
    let (label_response, response) = setting_row(ui, tokens, label, |ui| {
        let (rect, mut response) = ui.allocate_exact_size(SWITCH_SIZE, egui::Sense::click());
        if response.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        let on = *value;
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, label));
        let t = ui.ctx().animate_bool_responsive(response.id, on);
        let painter = ui.painter();
        let rect = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), SWITCH_TRACK_HEIGHT));
        let radius = rect.height() / 2.0;
        let fill = if on { crate::theme::selection_fill(tokens) } else { tokens.surface };
        painter.rect_filled(rect, radius, fill);
        let outline = if response.has_focus() {
            Stroke::new(2.0_f32, tokens.focus)
        } else if response.hovered() {
            Stroke::new(1.0_f32, tokens.focus)
        } else {
            Stroke::new(1.0_f32, tokens.border)
        };
        painter.rect_stroke(rect, radius, outline, egui::StrokeKind::Inside);
        let knob_r = radius - 4.0;
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        let knob = if on { tokens.primary } else { tokens.muted_text };
        painter.circle_filled(egui::pos2(x, rect.center().y), knob_r, knob);
        response
    });
    response.labelled_by(label_response.id)
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
            assert_eq!(StatusKind::Muted.color(tokens), tokens.muted_text);
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
                    for kind in [
                        StatusKind::Success,
                        StatusKind::Warning,
                        StatusKind::Error,
                        StatusKind::Muted,
                    ] {
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
    fn settings_rows_paint_inside_their_rects_and_flip_on_click() {
        let ctx = egui::Context::default();
        let tokens = ThemeMode::TokyoNight.tokens();
        let mut value = true;
        let mut choice = 0usize;
        let rects = std::cell::RefCell::new((egui::Rect::NOTHING, Vec::new()));
        let run = |events: Vec<egui::Event>, value: &mut bool, choice: &mut usize| {
            ctx.run(egui::RawInput { events, ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let switch = toggle_row(ui, tokens, "Smooth edges", value).rect;
                    let cells = segmented_row(ui, tokens, "Style", &["Mono", "3-band", "RGB"], choice);
                    *rects.borrow_mut() = (switch, cells.iter().map(|c| c.rect).collect());
                });
            })
        };
        for _ in 0..2 {
            run(vec![], &mut value, &mut choice);
        }
        let (switch, cells) = rects.borrow().clone();
        assert_eq!(switch.height(), crate::theme::CONTROL_HEIGHT);
        let click = |pos| {
            let b = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            vec![egui::Event::PointerMoved(pos), b(true), b(false)]
        };
        for expect in [false, true] {
            run(click(switch.center()), &mut value, &mut choice);
            assert_eq!(value, expect);
            let out = run(vec![], &mut value, &mut choice);
            let knob = out.shapes.iter().find_map(|c| match &c.shape {
                egui::Shape::Circle(k) => Some(k),
                _ => None,
            });
            let knob = knob.expect("knob painted");
            let bounds = egui::Rect::from_center_size(knob.center, egui::Vec2::splat(2.0 * knob.radius));
            assert!(switch.contains_rect(bounds), "knob inside the switch: {bounds:?}");
            assert_eq!(knob.fill, if expect { tokens.primary } else { tokens.muted_text });
        }
        run(click(cells[2].center()), &mut value, &mut choice);
        assert_eq!(choice, 2);
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
