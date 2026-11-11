//! M8/D4: the mixer section of the deck area.
//!
//! Club layout in two thin rows: channel A on the left, channel B on the
//! right (each: deck chip, TRIM, HI, MID, LOW on the first row; FILTER,
//! channel fader, level meter and the crossfader assign A / THRU / B on
//! the second), the master volume with its meter and the crossfader in
//! the centre. Narrower windows reflow instead of squeezing the knobs
//! (see [`MixerFit`]): first the centre moves to a row below both
//! channels, then the channels stack (A, centre, B), so every slider
//! keeps at least [`SLIDER_MIN_W`] of travel and nothing overlaps.
//! The crossfader has its own small icon toggle (also
//! Ctrl+Shift+5 and Settings); hidden, it is out of the audio path.
//!
//! Every control is a native egui widget (sliders take arrow keys, the
//! buttons Space/Enter), so the whole mixer is keyboard reachable; double
//! click resets a knob. Meters are painted with theme tokens only.

#![allow(deprecated)]

use crate::LibraryApp;
use crate::decks::{
    DECKS, DeckColours, DeckLayout, GAP, MeterLevel, chip, deck_button, icon_button, reset_clicked,
};
use crate::icons::Icon;
use crate::settings::{MixerDensity, MixerPrefs, MonitorPlacement};
use crate::theme::ColorTokens;
use eframe::egui::{self, FontId, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};
use oxidj_audio::{Assign, EQ_KILL_DB, EQ_MAX_DB, TRIM_DB};
use std::ops::RangeInclusive;

#[cfg(test)]
mod wheel_tests {
    use super::*;

    #[test]
    fn mixer_wheel_respects_parent_scroll_and_disabled_controls() {
        let ctx = egui::Context::default();
        let mut value = 0.5;
        let mut pos = egui::Pos2::ZERO;
        let mut offset = 0.0;
        let mut frame = |events: Vec<egui::Event>, enabled: bool, rotary: bool| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0))),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let output = egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            let (cell, _) =
                                ui.allocate_exact_size(vec2(180.0, 28.0), Sense::hover());
                            let tokens = crate::theme::current_tokens(ctx);
                            ui.add_enabled_ui(enabled, |ui| {
                                let response = if rotary {
                                    mixer_rotary(
                                        ui,
                                        cell,
                                        "FILTER",
                                        &mut value,
                                        "filter",
                                        String::new(),
                                        tokens.primary,
                                        tokens,
                                    )
                                } else {
                                    mixer_knob(
                                        ui,
                                        cell,
                                        "VOL",
                                        &mut value,
                                        0.0..=1.0,
                                        FADER_STEP,
                                        1.0,
                                        "volume",
                                        String::new(),
                                        tokens.primary,
                                        tokens,
                                    )
                                };
                                pos = response.rect.center();
                            });
                            ui.add_space(500.0);
                        });
                    offset = output.state.offset.y;
                });
            });
            (pos, offset, value)
        };
        for rotary in [true, false] {
            let (p, _, _) = frame(vec![], true, rotary);
            let wheel = |p| {
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0.0, -4.0),
                        modifiers: Default::default(),
                    },
                ]
            };
            let (_, before_offset, before_value) =
                frame(vec![egui::Event::PointerMoved(p)], true, rotary);
            let (_, after_offset, after_value) = frame(wheel(p), true, rotary);
            assert_eq!(after_offset, before_offset);
            assert!(after_value < before_value);
            let (_, tail_start, _) = frame(
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: vec2(0.0, -1.0),
                        modifiers: Default::default(),
                    },
                ],
                true,
                rotary,
            );
            for _ in 0..5 {
                let (_, tail_offset, _) = frame(
                    vec![egui::Event::PointerMoved(egui::pos2(300.0, 50.0))],
                    true,
                    rotary,
                );
                assert_eq!(
                    tail_offset, tail_start,
                    "wheel smoothing must not scroll parent after pointer leaves"
                );
            }
            let (_, _, disabled_before) = frame(vec![egui::Event::PointerMoved(p)], false, rotary);
            let (_, disabled_offset, disabled_after) = frame(wheel(p), false, rotary);
            assert_eq!(disabled_before, disabled_after);
            assert!(
                disabled_offset > after_offset,
                "disabled control must leave scroll to parent"
            );
            let (_, _, outside_before) = frame(vec![], true, rotary);
            let (_, _, outside_after) = frame(wheel(egui::pos2(300.0, 50.0)), true, rotary);
            assert_eq!(outside_before, outside_after);
        }
    }
}

/// Label column of a mixer knob.
const LABEL_W: f32 = 34.0;
/// Clip indicator at the meter's right end.
const CLIP_W: f32 = 8.0;
/// Meter scale floor (dBFS).
pub(crate) const METER_FLOOR_DB: f32 = -48.0;
/// Amber zone starts here (dBFS).
const METER_HOT_DB: f32 = -6.0;
/// Crossfader icon toggle.
const ICON_W: f32 = 24.0;
/// Least slider travel a mixer knob keeps at any supported width.
pub(crate) const SLIDER_MIN_W: f32 = 40.0;
/// Gap between a channel strip and its neighbour.
const SPLIT: f32 = 10.0;
/// Inset of the controls inside the section outline.
pub(crate) const INSET: f32 = 4.0;
/// Arrow-key step of the trim knob (dB).
pub(crate) const KNOB_DB_STEP: f64 = 1.0;
/// Arrow-key step of the EQ knobs (position), filter, faders and crossfader.
pub(crate) const FADER_STEP: f64 = 0.05;

/// Consume vertical wheel input while the pointer is over a mixer control.
pub(crate) fn mixer_wheel(
    ui: &mut Ui,
    response: &egui::Response,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
) -> bool {
    let pending_id = response.id.with("mixer-wheel-smoothing");
    let pending = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(pending_id).unwrap_or(false));
    if !response.enabled() || !response.hovered() {
        // egui smooths line-wheel deltas over subsequent frames. Do not let
        // their tail scroll an ancestor after the pointer leaves the control.
        if pending {
            let (raw, smooth) = ui.input_mut(|input| {
                let raw = input.raw_scroll_delta.y;
                let smooth = input.smooth_scroll_delta.y;
                if raw == 0.0 {
                    input.smooth_scroll_delta.y = 0.0;
                }
                (raw, smooth)
            });
            if raw != 0.0 || smooth == 0.0 {
                ui.ctx().data_mut(|data| data.remove::<bool>(pending_id));
            }
        }
        return false;
    }
    let delta = ui.input_mut(|input| {
        let delta = input.raw_scroll_delta.y;
        input.raw_scroll_delta.y = 0.0;
        input.smooth_scroll_delta.y = 0.0;
        delta / 40.0
    });
    if delta != 0.0 {
        ui.ctx().data_mut(|data| data.insert_temp(pending_id, true));
    }
    let before = *value;
    *value = (*value + delta * step as f32).clamp(*range.start(), *range.end());
    *value != before
}

/// A labelled, compact horizontal knob (native egui slider): exact rect,
/// AccessKit name, arrow keys move by `step`, double click resets to
/// `default`. Returns the response.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn mixer_knob(
    ui: &mut Ui,
    cell: Rect,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
    default: f32,
    name: &str,
    hover: String,
    fill: egui::Color32,
    tokens: ColorTokens,
) -> egui::Response {
    let g = ui.painter().layout_no_wrap(
        label.to_owned(),
        FontId::proportional(10.5),
        tokens.muted_text,
    );
    let label_w = LABEL_W.max(g.size().x + 6.0);
    ui.painter().galley(
        pos2(cell.left(), cell.center().y - g.size().y / 2.0),
        g,
        tokens.muted_text,
    );
    let slider_r = Rect::from_min_max(pos2(cell.left() + label_w, cell.top()), cell.max);
    let mut v = *value;
    let mut slider = ui
        .scope(|ui| {
            crate::track_waveform::slider_look(ui, tokens);
            ui.visuals_mut().selection.bg_fill = fill;
            ui.spacing_mut().slider_width = slider_r.width().max(8.0);
            ui.put(
                slider_r,
                egui::Slider::new(&mut v, range.clone())
                    .show_value(false)
                    .step_by(step),
            )
        })
        .inner;
    if mixer_wheel(ui, &slider, &mut v, range, step) {
        slider.mark_changed();
    }
    slider.widget_info(|| egui::WidgetInfo::slider(true, f64::from(v), name));
    let slider = slider.on_hover_text(hover);
    if reset_clicked(&slider) {
        v = default;
    }
    *value = v;
    slider
}

/// A labelled, compact vertical fader (native egui slider): exact rect,
/// AccessKit name, arrow keys move by `step` (Up = increase), double click resets to
/// `default`. Returns the response. The label is drawn at the bottom.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mixer_vertical_fader(
    ui: &mut Ui,
    cell: Rect,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
    default: f32,
    name: &str,
    hover: String,
    fill: egui::Color32,
    tokens: ColorTokens,
) -> egui::Response {
    // Label at the bottom of the cell.
    // An empty label leaves the whole cell to the slider (mixer v2: the
    // channel faders span the full channel height).
    let label_y = if label.is_empty() {
        cell.bottom() + 4.0
    } else {
        let g = ui.painter().layout_no_wrap(
            label.to_owned(),
            FontId::proportional(10.5),
            tokens.muted_text,
        );
        let label_y = cell.bottom() - g.size().y - 2.0;
        ui.painter().galley(
            pos2(cell.center().x - g.size().x / 2.0, label_y),
            g,
            tokens.muted_text,
        );
        label_y
    };
    // Slider track area: above the label, full cell width, minimum SLIDER_MIN_W height.
    let slider_h = (label_y - cell.top() - 4.0).max(SLIDER_MIN_W);
    let slider_r = Rect::from_min_size(pos2(cell.left(), cell.top()), vec2(cell.width(), slider_h));
    let id = ui.id().with(("mixer-vertical-fader", name));
    let mut v = *value;
    let min = *range.start();
    let max = *range.end();
    let span = max - min;
    let before = v;
    // Single interact for hover, wheel, click, and drag.
    let mut response = ui.interact(slider_r, id, Sense::click_and_drag());
    // Wheel input.
    mixer_wheel(ui, &response, &mut v, range.clone(), step);
    let drag_id = id.with("drag");
    // Pointer drag: vertical movement changes value.
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.drag_started() || response.dragged() {
            if let Some(previous_y) = ui.ctx().data(|d| d.get_temp::<f32>(drag_id)) {
                let delta_y = previous_y - pointer.y; // Up = increase
                v = (v + span * delta_y / slider_h).clamp(min, max);
            }
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, pointer.y));
        } else if response.clicked() {
            let t = (slider_r.bottom() - pointer.y) / slider_h;
            v = (min + span * t.clamp(0.0, 1.0)).clamp(min, max);
            response.request_focus();
        } else if response.hovered() && ui.input(|i| i.pointer.primary_down()) {
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, pointer.y));
        }
    }
    if response.drag_stopped() {
        ui.ctx().data_mut(|d| d.remove::<f32>(drag_id));
    }
    // Keyboard: Up/Down arrows.
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let step_size = step as f32;
        let step_dir = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowRight)) as i32
                - (i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowLeft))
                    as i32
        });
        if step_dir != 0 {
            v = (((v / step_size).round() + step_dir as f32) * step_size).clamp(min, max);
        }
    }
    // AccessKit actions: SetValue, Increment, Decrement.
    ui.input(|input| {
        use egui::accesskit::{Action, ActionData};
        let delta = input.num_accesskit_action_requests(id, Action::Increment) as i32
            - input.num_accesskit_action_requests(id, Action::Decrement) as i32;
        if delta != 0 {
            v = (v + delta as f32 * step as f32).clamp(min, max);
        }
        for request in input.accesskit_action_requests(id, Action::SetValue) {
            if let Some(ActionData::NumericValue(val)) = request.data {
                if val.is_finite() {
                    v = val.clamp(f64::from(min), f64::from(max)) as f32;
                }
            }
        }
    });
    if reset_clicked(&response) {
        v = default;
    }
    if v != before {
        response.mark_changed();
    }
    *value = v;
    // Paint the slider track and thumb.
    let painter = ui.painter_at(slider_r);
    let track_w = (slider_r.width() * 0.3).max(4.0).min(8.0);
    let track_r = Rect::from_center_size(slider_r.center(), vec2(track_w, slider_h - 8.0));
    painter.rect_filled(track_r, 2.0, tokens.border);
    let thumb_y = egui::lerp(track_r.bottom()..=track_r.top(), (v - min) / span);
    let thumb_r = Rect::from_center_size(
        pos2(slider_r.center().x, thumb_y),
        vec2(slider_r.width(), 6.0),
    );
    painter.rect_filled(thumb_r, 3.0, fill);
    painter.rect_stroke(
        thumb_r,
        3.0,
        Stroke::new(
            1.5_f32,
            if response.has_focus() {
                tokens.focus
            } else {
                tokens.border
            },
        ),
        egui::StrokeKind::Inside,
    );
    // AccessKit node metadata.
    ui.ctx().accesskit_node_builder(id, |node| {
        use egui::accesskit::Action;
        node.set_label(name.to_owned());
        node.set_min_numeric_value(f64::from(min));
        node.set_max_numeric_value(f64::from(max));
        node.set_numeric_value_step(step);
        node.add_action(Action::SetValue);
        if v < max {
            node.add_action(Action::Increment);
        }
        if v > min {
            node.add_action(Action::Decrement);
        }
        // Explicit bounds for AccessKit (match slider track area).
        let bounds = egui::accesskit::Rect {
            x0: slider_r.left() as f64,
            y0: slider_r.top() as f64,
            x1: slider_r.right() as f64,
            y1: slider_r.bottom() as f64,
        };
        node.set_bounds(bounds);
    });
    response.widget_info(|| egui::WidgetInfo::slider(true, f64::from(v), name));
    response.on_hover_text(hover)
}

/// A labelled, compact horizontal fader (native egui slider): exact rect,
/// AccessKit name, arrow keys move by `step` (Right = increase), double click resets to
/// `default`. Returns the response. The label is drawn at the left.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mixer_horizontal_fader(
    ui: &mut Ui,
    cell: Rect,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
    default: f32,
    name: &str,
    hover: String,
    fill: egui::Color32,
    tokens: ColorTokens,
) -> egui::Response {
    // Label at the left of the cell.
    let g = ui.painter().layout_no_wrap(
        label.to_owned(),
        FontId::proportional(10.5),
        tokens.muted_text,
    );
    let label_x = cell.left() + g.size().x + 6.0;
    ui.painter().galley(
        pos2(cell.left() + 2.0, cell.center().y - g.size().y / 2.0),
        g,
        tokens.muted_text,
    );
    // Slider track area: right of the label, full cell height, minimum SLIDER_MIN_W width.
    let slider_w = (cell.right() - label_x - 4.0).max(SLIDER_MIN_W);
    let slider_r = Rect::from_min_size(pos2(label_x, cell.top()), vec2(slider_w, cell.height()));
    let id = ui.id().with(("mixer-horizontal-fader", name));
    let mut v = *value;
    let min = *range.start();
    let max = *range.end();
    let span = max - min;
    let before = v;
    let mut response = ui.interact(slider_r, id, Sense::click_and_drag());
    mixer_wheel(ui, &response, &mut v, range.clone(), step);
    let drag_id = id.with("drag");
    // Pointer drag: horizontal movement changes value.
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.drag_started() || response.dragged() {
            if let Some(previous_x) = ui.ctx().data(|d| d.get_temp::<f32>(drag_id)) {
                let delta_x = pointer.x - previous_x; // Right = increase
                v = (v + span * delta_x / slider_w).clamp(min, max);
            }
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, pointer.x));
        } else if response.clicked() {
            let t = (pointer.x - slider_r.left()) / slider_w;
            v = (min + span * t.clamp(0.0, 1.0)).clamp(min, max);
            response.request_focus();
        } else if response.hovered() && ui.input(|i| i.pointer.primary_down()) {
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, pointer.x));
        }
    }
    if response.drag_stopped() {
        ui.ctx().data_mut(|d| d.remove::<f32>(drag_id));
    }
    // Keyboard: Left/Right arrows.
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            )
        });
        let step_size = step as f32;
        let step_dir = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowUp)) as i32
                - (i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowDown))
                    as i32
        });
        if step_dir != 0 {
            v = (((v / step_size).round() + step_dir as f32) * step_size).clamp(min, max);
        }
    }
    // AccessKit actions: SetValue, Increment, Decrement.
    ui.input(|input| {
        use egui::accesskit::{Action, ActionData};
        let delta = input.num_accesskit_action_requests(id, Action::Increment) as i32
            - input.num_accesskit_action_requests(id, Action::Decrement) as i32;
        if delta != 0 {
            v = (v + delta as f32 * step as f32).clamp(min, max);
        }
        for request in input.accesskit_action_requests(id, Action::SetValue) {
            if let Some(ActionData::NumericValue(val)) = request.data {
                if val.is_finite() {
                    v = val.clamp(f64::from(min), f64::from(max)) as f32;
                }
            }
        }
    });
    if reset_clicked(&response) {
        v = default;
    }
    if v != before {
        response.mark_changed();
    }
    *value = v;
    // Paint the slider track and thumb.
    let painter = ui.painter_at(slider_r);
    let track_h = (slider_r.height() * 0.3).max(4.0).min(8.0);
    let track_r = Rect::from_center_size(slider_r.center(), vec2(slider_w - 8.0, track_h));
    painter.rect_filled(track_r, 2.0, tokens.border);
    let thumb_x = egui::lerp(track_r.left()..=track_r.right(), (v - min) / span);
    let thumb_r = Rect::from_center_size(
        pos2(thumb_x, slider_r.center().y),
        vec2(6.0, slider_r.height()),
    );
    painter.rect_filled(thumb_r, 3.0, fill);
    painter.rect_stroke(
        thumb_r,
        3.0,
        Stroke::new(
            1.5_f32,
            if response.has_focus() {
                tokens.focus
            } else {
                tokens.border
            },
        ),
        egui::StrokeKind::Inside,
    );
    // AccessKit node metadata.
    ui.ctx().accesskit_node_builder(id, |node| {
        use egui::accesskit::Action;
        node.set_label(name.to_owned());
        node.set_min_numeric_value(f64::from(min));
        node.set_max_numeric_value(f64::from(max));
        node.set_numeric_value_step(step);
        node.add_action(Action::SetValue);
        if v < max {
            node.add_action(Action::Increment);
        }
        if v > min {
            node.add_action(Action::Decrement);
        }
        // Explicit bounds for AccessKit (match slider track area).
        let bounds = egui::accesskit::Rect {
            x0: slider_r.left() as f64,
            y0: slider_r.top() as f64,
            x1: slider_r.right() as f64,
            y1: slider_r.bottom() as f64,
        };
        node.set_bounds(bounds);
    });
    response.widget_info(|| egui::WidgetInfo::slider(true, f64::from(v), name));
    response.on_hover_text(hover)
}

/// Compact Minimal Line rotary (label left of the knob), -1..=1.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn mixer_rotary(
    ui: &mut Ui,
    cell: Rect,
    label: &str,
    value: &mut f32,
    name: &str,
    hover: String,
    fill: egui::Color32,
    tokens: ColorTokens,
) -> egui::Response {
    mixer_rotary_styled(
        ui,
        cell,
        label,
        LabelAt::Left,
        KnobShape::Round,
        value,
        -1.0..=1.0,
        FADER_STEP,
        0.0,
        name,
        hover,
        fill,
        tokens,
    )
}

/// Where a rotary's label sits relative to its knob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelAt {
    Left,
    Above,
    Below,
    /// No label: the cell is the knob.
    None,
}

/// Knob body: the round club knob, or the small square TRIM trimpot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnobShape {
    Round,
    Square,
}

/// Label height of an above/below-labelled knob.
pub(crate) const KNOB_LABEL_H: f32 = 10.0;

#[allow(clippy::too_many_arguments)]
fn mixer_rotary_styled(
    ui: &mut Ui,
    cell: Rect,
    label: &str,
    label_at: LabelAt,
    shape: KnobShape,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step: f64,
    default: f32,
    name: &str,
    hover: String,
    fill: egui::Color32,
    tokens: ColorTokens,
) -> egui::Response {
    let min = *range.start();
    let max = *range.end();
    let span = max - min;
    let rect = match label_at {
        LabelAt::Left => {
            let g = ui.painter().layout_no_wrap(
                label.to_owned(),
                FontId::proportional(10.5),
                tokens.muted_text,
            );
            let label_w = LABEL_W.max(g.size().x + 6.0);
            ui.painter().galley(
                pos2(cell.left(), cell.center().y - g.size().y / 2.0),
                g,
                tokens.muted_text,
            );
            Rect::from_min_max(pos2(cell.left() + label_w, cell.top()), cell.max)
        }
        LabelAt::None => cell,
        LabelAt::Above | LabelAt::Below => {
            let g = ui.painter().layout_no_wrap(
                label.to_owned(),
                FontId::proportional(9.0),
                tokens.muted_text,
            );
            let (label_y, rect) = if label_at == LabelAt::Above {
                (
                    cell.top(),
                    Rect::from_min_max(pos2(cell.left(), cell.top() + KNOB_LABEL_H), cell.max),
                )
            } else {
                (
                    cell.bottom() - KNOB_LABEL_H,
                    Rect::from_min_max(cell.min, pos2(cell.right(), cell.bottom() - KNOB_LABEL_H)),
                )
            };
            ui.painter().galley(
                pos2(
                    cell.center().x - g.size().x / 2.0,
                    label_y + (KNOB_LABEL_H - g.size().y) / 2.0,
                ),
                g,
                tokens.muted_text,
            );
            rect
        }
    };
    let id = ui.id().with(("mixer-rotary", name));
    let mut response = ui.interact(rect, id, Sense::click_and_drag());
    if response.clicked() {
        response.request_focus();
    }
    let drag_id = id.with("angle");
    let center = rect.center();
    let start = -std::f32::consts::FRAC_PI_2 - 3.0 * std::f32::consts::FRAC_PI_4;
    let sweep = 3.0 * std::f32::consts::FRAC_PI_2;
    let before = *value;
    mixer_wheel(ui, &response, value, range.clone(), step);
    if let Some(pointer) = response.interact_pointer_pos() {
        let angle = (pointer.y - center.y).atan2(pointer.x - center.x);
        if response.drag_started() || response.dragged() {
            if let Some(previous) = ui.ctx().data(|d| d.get_temp::<f32>(drag_id)) {
                let delta = (angle - previous + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                *value = (*value + span * delta / sweep).clamp(min, max);
            }
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, angle));
        } else if response.clicked() {
            let a = (angle - start).rem_euclid(std::f32::consts::TAU);
            *value = if a <= sweep {
                min + span * a / sweep
            } else if a - sweep <= (std::f32::consts::TAU - sweep) / 2.0 {
                max
            } else {
                min
            };
        } else if response.hovered() && ui.input(|i| i.pointer.primary_down()) {
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, angle));
        }
    }
    if response.drag_stopped() {
        ui.ctx().data_mut(|d| d.remove::<f32>(drag_id));
    }
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let step_size = step as f32;
        let step = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowUp)) as i32
                - (i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowDown))
                    as i32
        });
        if step != 0 {
            *value = (((*value / step_size).round() + step as f32) * step_size).clamp(min, max);
        }
    }
    ui.input(|input| {
        use egui::accesskit::{Action, ActionData};
        let delta = input.num_accesskit_action_requests(id, Action::Increment) as i32
            - input.num_accesskit_action_requests(id, Action::Decrement) as i32;
        if delta != 0 {
            *value = (*value + delta as f32 * step as f32).clamp(min, max);
        }
        for request in input.accesskit_action_requests(id, Action::SetValue) {
            if let Some(ActionData::NumericValue(v)) = request.data {
                if v.is_finite() {
                    *value = v.clamp(f64::from(min), f64::from(max)) as f32;
                }
            }
        }
    });
    if reset_clicked(&response) {
        *value = default;
    }
    if *value != before {
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::slider(true, f64::from(*value), name));
    ui.ctx().accesskit_node_builder(id, |node| {
        use egui::accesskit::Action;
        node.set_min_numeric_value(f64::from(min));
        node.set_max_numeric_value(f64::from(max));
        node.set_numeric_value_step(step);
        node.add_action(Action::SetValue);
        if *value < max {
            node.add_action(Action::Increment);
        }
        if *value > min {
            node.add_action(Action::Decrement);
        }
    });
    let painter = ui.painter_at(rect);
    let compact = label_at != LabelAt::Left;
    let angle = start + ((*value - min) / span).clamp(0.0, 1.0) * sweep;
    let dir = vec2(angle.cos(), angle.sin());
    let ring = if response.has_focus() {
        tokens.focus
    } else {
        tokens.border
    };
    if shape == KnobShape::Square {
        // Small filled trimpot: a rounded square in the channel's trim ink
        // with a dark inset rim, a pointer line and a dot at its tip.
        let side = (rect.width().min(rect.height()) - 2.0).max(0.0);
        let body = Rect::from_center_size(center, vec2(side, side));
        painter.rect_filled(body, 3.0, fill);
        painter.rect_stroke(
            body,
            3.0,
            Stroke::new(
                if response.has_focus() {
                    2.0_f32
                } else {
                    1.5_f32
                },
                if response.has_focus() {
                    tokens.focus
                } else {
                    tokens.background
                },
            ),
            egui::StrokeKind::Inside,
        );
        let tip = center + dir * (side * 0.28);
        painter.line_segment([center, tip], Stroke::new(1.5_f32, tokens.background));
        painter.circle_filled(tip, 1.2, tokens.background);
    } else {
        let radius = if compact {
            (rect.width().min(rect.height()) * 0.5 - 1.0).max(0.0)
        } else {
            (rect.width().min(rect.height()) * 0.5 - 3.0).max(0.0)
        };
        painter.circle_filled(center, radius, tokens.background);
        painter.circle_stroke(center, radius, Stroke::new(1.0_f32, ring));
        if !compact {
            for tick in 0..=6 {
                let angle = start + tick as f32 / 6.0 * sweep;
                let dir = vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [center + dir * (radius + 1.0), center + dir * (radius + 2.0)],
                    Stroke::new(1.0_f32, tokens.muted_text),
                );
            }
        }
        painter.line_segment(
            [center, center + dir * (radius * 0.7)],
            Stroke::new(2.0_f32, fill),
        );
    }
    response.on_hover_text(hover)
}

/// EQ knob position −1 (kill) .. 0 (centre, 0 dB) .. +1 (+6 dB) to dB:
/// linear in dB on each side, like an isolator knob at 12 o'clock.
pub(crate) fn eq_db_at(pos: f32) -> f32 {
    let pos = if pos.is_nan() {
        0.0
    } else {
        pos.clamp(-1.0, 1.0)
    };
    // The left stop is kill even after float drift of the slider steps.
    if pos <= -0.999 {
        return EQ_KILL_DB;
    }
    if pos >= 0.0 {
        pos * EQ_MAX_DB
    } else {
        -pos * EQ_KILL_DB
    }
}

/// Inverse of [`eq_db_at`].
pub(crate) fn eq_pos_of(db: f32) -> f32 {
    if db >= 0.0 {
        (db / EQ_MAX_DB).min(1.0)
    } else {
        (db / EQ_KILL_DB).min(1.0) * -1.0
    }
}

/// Fraction of the meter scale `level` (linear) fills.
pub(crate) fn meter_fraction(level: f32) -> f32 {
    if level <= 0.0 || level.is_nan() {
        return 0.0;
    }
    let db = 20.0 * level.log10();
    ((db - METER_FLOOR_DB) / -METER_FLOOR_DB).clamp(0.0, 1.0)
}

fn db_text(level: f32) -> String {
    if level <= 0.0 {
        "−∞ dB".into()
    } else {
        format!("{:+.1} dB", 20.0 * level.log10())
    }
}

/// A peak meter with clip indicator, tokens only: green body, amber above
/// [`METER_HOT_DB`], red clip box while the clip is held.
/// A horizontal peak meter with `pad` above and below the bar.
fn level_meter_padded(
    ui: &mut Ui,
    rect: Rect,
    meter: MeterLevel,
    name: &str,
    tokens: ColorTokens,
    pad: f32,
) {
    let response = ui.interact(rect, ui.id().with(("level-meter", name)), Sense::hover());
    let text = format!(
        "{name}: {}{}",
        db_text(meter.level),
        if meter.clipping() { ", clip" } else { "" }
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ProgressIndicator, true, &text)
    });
    let painter = ui.painter_at(rect);
    let bar = Rect::from_min_max(
        rect.min + vec2(0.0, pad),
        pos2(rect.right() - CLIP_W - 2.0, rect.bottom() - pad),
    );
    painter.rect_filled(bar, 2.0, tokens.background);
    let f = meter_fraction(meter.level);
    let hot = meter_fraction(10f32.powf(METER_HOT_DB / 20.0));
    let x = |frac: f32| bar.left() + bar.width() * frac;
    if f > 0.0 {
        let green = Rect::from_min_max(bar.min, pos2(x(f.min(hot)), bar.bottom()));
        painter.rect_filled(green, 1.0, tokens.success);
        if f > hot {
            painter.rect_filled(
                Rect::from_min_max(pos2(x(hot), bar.top()), pos2(x(f), bar.bottom())),
                1.0,
                tokens.warning,
            );
        }
    }
    painter.rect_stroke(
        bar,
        2.0,
        Stroke::new(1.0_f32, tokens.border),
        egui::StrokeKind::Inside,
    );
    let clip = Rect::from_min_max(
        pos2(rect.right() - CLIP_W, bar.top()),
        pos2(rect.right(), bar.bottom()),
    );
    painter.rect_filled(
        clip,
        2.0,
        if meter.clipping() {
            tokens.error
        } else {
            tokens.background
        },
    );
    painter.rect_stroke(
        clip,
        2.0,
        Stroke::new(1.0_f32, tokens.border),
        egui::StrokeKind::Inside,
    );
    response.on_hover_text(text);
}

/// A vertical peak meter filling `rect` (clip box on top, scale bottom up),
/// tokens only; same accessible name as [`level_meter_padded`].
pub(crate) fn level_meter_v(
    ui: &mut Ui,
    rect: Rect,
    meter: MeterLevel,
    name: &str,
    tokens: ColorTokens,
) {
    let response = ui.interact(rect, ui.id().with(("level-meter", name)), Sense::hover());
    let text = format!(
        "{name}: {}{}",
        db_text(meter.level),
        if meter.clipping() { ", clip" } else { "" }
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ProgressIndicator, true, &text)
    });
    let painter = ui.painter_at(rect);
    let clip = Rect::from_min_size(rect.min, vec2(rect.width(), CLIP_H));
    let bar = Rect::from_min_max(pos2(rect.left(), clip.bottom() + 1.0), rect.max);
    painter.rect_filled(bar, 1.0, tokens.background);
    let f = meter_fraction(meter.level);
    let hot = meter_fraction(10f32.powf(METER_HOT_DB / 20.0));
    let y = |frac: f32| bar.bottom() - bar.height() * frac;
    if f > 0.0 {
        painter.rect_filled(
            Rect::from_min_max(pos2(bar.left(), y(f.min(hot))), bar.max),
            0.0,
            tokens.success,
        );
        if f > hot {
            painter.rect_filled(
                Rect::from_min_max(pos2(bar.left(), y(f)), pos2(bar.right(), y(hot))),
                0.0,
                tokens.warning,
            );
        }
    }
    painter.rect_stroke(
        bar,
        1.0,
        Stroke::new(1.0_f32, tokens.border),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        clip,
        1.0,
        if meter.clipping() {
            tokens.error
        } else {
            tokens.background
        },
    );
    painter.rect_stroke(
        clip,
        1.0,
        Stroke::new(1.0_f32, tokens.border),
        egui::StrokeKind::Inside,
    );
    response.on_hover_text(text);
}

// ---- Mixer v2 geometry (v19): one narrow mixer, sized as a share of the
// screen width, with the knobs outside the faders (default) or above them.

/// Clip box height of a vertical meter.
const CLIP_H: f32 = 4.0;
/// Master / cue controls sit in a small column right of the mixer block.
const MON_W: f32 = 100.0;
/// Master / cue knob cells (about 60 % of the former 56x54 and 46x40 cells).
const MASTER_CELL: (f32, f32) = (34.0, 36.0);
const SMALL_CELL: (f32, f32) = (30.0, 27.0);
/// Height of the horizontal master / cue strip above or below the mixer.
const STRIP_H: f32 = 38.0;
/// Narrowest strip: three knobs and two short meters.
const STRIP_MIN_W: f32 = 190.0;
/// Knob diameter-giving height of a top-bar knob (toolbar row is 24 px).
const BAR_KNOB: f32 = 22.0;
/// Crossfader area: separator, label row, slider row, assign row.
const XF_H: f32 = 1.0 + 3.0 + 14.0 + 2.0 + 14.0 + 2.0 + 14.0;
/// Narrowest mixer block (screen share is clamped to this on tiny windows).
const MIN_MIXER_W: f32 = 120.0;
/// Optional input-selector and A/B label rows.
const SEL_H: f32 = 13.0;
const AB_H: f32 = 14.0;
const OPT_GAP: f32 = 2.0;
/// Small gap above and below the EQ group; gap between EQ knobs.
const EQ_GAP: f32 = 6.0;
const ITEM_GAP: f32 = 1.0;
/// Headphone-cue icon button.
const HP_S: f32 = 18.0;
/// Least fader height of the knobs-above configuration.
const FADER_ABOVE_MIN: f32 = 64.0;

#[derive(Clone, Copy)]
struct KnobDims {
    trim: f32,
    eq: f32,
    filter: f32,
}

fn knob_dims(density: MixerDensity) -> KnobDims {
    match density {
        MixerDensity::Ultra => KnobDims {
            trim: 12.0,
            eq: 18.0,
            filter: 24.0,
        },
        MixerDensity::Compact => KnobDims {
            trim: 13.0,
            eq: 20.0,
            filter: 28.0,
        },
        MixerDensity::Roomy => KnobDims {
            trim: 15.0,
            eq: 24.0,
            filter: 34.0,
        },
    }
}

fn item_h(knob: f32) -> f32 {
    knob + 2.0 + KNOB_LABEL_H
}

/// Height of the knob column (selector, A/B label, TRIM, EQ group, FILTER).
fn stack_h(prefs: MixerPrefs) -> f32 {
    let d = knob_dims(prefs.density);
    let mut h = item_h(d.trim) + EQ_GAP + 3.0 * item_h(d.eq) + 2.0 * ITEM_GAP + EQ_GAP;
    h += item_h(d.filter);
    if prefs.inputs {
        h += SEL_H + OPT_GAP;
    }
    if prefs.ab_labels {
        h += AB_H + OPT_GAP;
    }
    h
}

fn channel_h(prefs: MixerPrefs) -> f32 {
    if prefs.knobs_outside {
        stack_h(prefs) + OPT_GAP + HP_S
    } else {
        stack_h(prefs) + 4.0 + FADER_ABOVE_MIN + OPT_GAP + HP_S
    }
}

/// Total mixer section height for the chosen configuration.
pub(crate) fn mixer_height(prefs: MixerPrefs) -> f32 {
    let strip = match prefs.monitor {
        MonitorPlacement::Above | MonitorPlacement::Below => STRIP_H + GAP,
        _ => 0.0,
    };
    2.0 * INSET + channel_h(prefs) + GAP + XF_H + strip
}

/// Width of the mixer block (outline inset included): a share of the
/// screen width, never wider than `avail`.
pub(crate) fn mixer_width(prefs: MixerPrefs, screen_w: f32, avail: f32) -> f32 {
    (screen_w * prefs.density.share())
        .max(MIN_MIXER_W)
        .min(avail.max(MIN_MIXER_W))
}

/// Width of the whole mixer section (block, monitor column, insets) in the
/// Layout B bottom row.
pub(crate) fn mixer_total_width(prefs: MixerPrefs, screen_w: f32) -> f32 {
    let block = mixer_width(prefs, screen_w, f32::MAX / 4.0) + 2.0 * INSET;
    match prefs.monitor {
        MonitorPlacement::Beside => block + SPLIT + MON_W,
        MonitorPlacement::TopBar => block,
        _ => block.max(STRIP_MIN_W),
    }
}

/// Where one channel's parts sit.
struct StripGeo {
    knobs: Rect,
    fader: Rect,
    cue: Rect,
}

impl LibraryApp {
    /// The mixer section: a narrow block (A | meters | B, crossfader below)
    /// with the master and headphone controls in a small column beside it.
    pub(crate) fn mixer(
        &mut self,
        ui: &mut Ui,
        r: Rect,
        tokens: ColorTokens,
        colours: [DeckColours; 2],
        layout: &mut DeckLayout,
    ) {
        let prefs = self.settings.mixer;
        let screen_w = ui.ctx().screen_rect().width();
        // The strip placements split `r` into the strip and the mixer block.
        let side = if prefs.monitor == MonitorPlacement::Beside {
            MON_W + SPLIT
        } else {
            0.0
        };
        let mut strip_r = None;
        let mut r = r;
        if matches!(
            prefs.monitor,
            MonitorPlacement::Above | MonitorPlacement::Below
        ) {
            let w = r.width();
            if prefs.monitor == MonitorPlacement::Above {
                strip_r = Some(Rect::from_min_size(r.min, vec2(w, STRIP_H)));
                r.set_top(r.top() + STRIP_H + GAP);
            } else {
                strip_r = Some(Rect::from_min_size(
                    pos2(r.left(), r.bottom() - STRIP_H),
                    vec2(w, STRIP_H),
                ));
                r.set_bottom(r.bottom() - STRIP_H - GAP);
            }
        }
        if prefs.monitor != MonitorPlacement::Beside {
            let bw = mixer_width(prefs, screen_w, r.width() - 2.0 * INSET) + 2.0 * INSET;
            r = Rect::from_center_size(r.center(), vec2(bw, r.height()));
        }
        // Outline only: a surface fill would hide the slider rails.
        ui.painter().rect_stroke(
            r,
            self.settings.deck_theme.radius(),
            Stroke::new(1.0_f32, tokens.border),
            egui::StrokeKind::Inside,
        );
        let inner = r.shrink(INSET);
        let mw = mixer_width(prefs, screen_w, inner.width() - side);
        let bw = mw - 2.0 * INSET;
        let x0 = (r.center().x - bw / 2.0)
            .min(inner.right() - side - bw)
            .max(inner.left());
        let ch_h = channel_h(prefs);
        let ca = Rect::from_min_size(pos2(x0, inner.top()), vec2(bw, ch_h));
        let xf = Rect::from_min_size(pos2(x0, inner.bottom() - XF_H), vec2(bw, XF_H));
        let mon = Rect::from_min_size(pos2(ca.right() + SPLIT, inner.top()), vec2(MON_W, ch_h));
        let dims = knob_dims(prefs.density);
        let stack = stack_h(prefs);
        let (geo, meters) = if prefs.knobs_outside {
            let g = 3.0;
            let fw = (0.14 * bw).clamp(14.0, 28.0);
            let mt = (0.07 * bw).clamp(7.0, 14.0);
            let cw = (bw - 4.0 * g - 2.0 * fw - 2.0 * mt - 2.0) / 2.0;
            let col = |x: f32, w: f32| Rect::from_min_size(pos2(x, ca.top()), vec2(w, ch_h));
            let knobs_a = col(ca.left(), cw);
            let fader_a = col(knobs_a.right() + g, fw);
            let meter_a = col(fader_a.right() + g, mt);
            let meter_b = col(meter_a.right() + 2.0, mt);
            let fader_b = col(meter_b.right() + g, fw);
            let knobs_b = col(fader_b.right() + g, cw);
            let cue = |k: Rect| {
                Rect::from_center_size(
                    pos2(k.center().x, k.bottom() - HP_S / 2.0),
                    vec2(HP_S, HP_S),
                )
            };
            (
                [
                    StripGeo {
                        knobs: knobs_a,
                        fader: fader_a,
                        cue: cue(knobs_a),
                    },
                    StripGeo {
                        knobs: knobs_b,
                        fader: fader_b,
                        cue: cue(knobs_b),
                    },
                ],
                [meter_a, meter_b],
            )
        } else {
            let hw = bw / 2.0;
            let fw = (0.16 * bw).clamp(16.0, 30.0);
            let mt = (0.07 * bw).clamp(7.0, 14.0);
            let strip = |axis: f32| {
                let fader_top = ca.top() + stack + 4.0;
                let fader_bottom = ca.bottom() - HP_S - OPT_GAP;
                StripGeo {
                    knobs: Rect::from_center_size(
                        pos2(axis, ca.top() + stack / 2.0),
                        vec2(hw - 2.0, stack),
                    ),
                    fader: Rect::from_min_max(
                        pos2(axis - fw / 2.0, fader_top),
                        pos2(axis + fw / 2.0, fader_bottom),
                    ),
                    cue: Rect::from_center_size(
                        pos2(axis, ca.bottom() - HP_S / 2.0),
                        vec2(HP_S, HP_S),
                    ),
                }
            };
            let geo = [strip(ca.left() + hw / 2.0), strip(ca.right() - hw / 2.0)];
            let (top, bottom) = (geo[0].fader.top(), geo[0].fader.bottom());
            let c = ca.center().x;
            let meter = |x0: f32| Rect::from_min_max(pos2(x0, top), pos2(x0 + mt, bottom));
            (geo, [meter(c - 1.0 - mt), meter(c + 1.0)])
        };
        // Creation order = Tab order: A, B, headphone/master, crossfader.
        for d in 0..2 {
            self.channel_strip(ui, d, &geo[d], dims, prefs, tokens, colours[d], layout);
            level_meter_v(
                ui,
                meters[d],
                self.decks.meters[d],
                &format!("Deck {} level", DECKS[d]),
                tokens,
            );
            layout.meters[d] = Some(meters[d]);
        }
        match (prefs.monitor, strip_r) {
            (MonitorPlacement::Beside, _) => self.mixer_column(ui, mon, tokens, layout),
            (_, Some(sr)) => self.mixer_strip(ui, sr, tokens, layout),
            _ => {}
        }
        self.crossfader_area(ui, xf, tokens, layout);
        layout.mixer = Some(r);
    }

    #[allow(clippy::too_many_arguments)]
    fn channel_strip(
        &mut self,
        ui: &mut Ui,
        d: usize,
        geo: &StripGeo,
        dims: KnobDims,
        prefs: MixerPrefs,
        tokens: ColorTokens,
        c: DeckColours,
        layout: &mut DeckLayout,
    ) {
        let letter = DECKS[d];
        let label_at = if prefs.knobs_outside {
            LabelAt::Above
        } else {
            LabelAt::Below
        };
        let deck = &mut self.decks.decks[d];
        let col = geo.knobs;
        let mut y = col.top();
        if prefs.inputs {
            // Placeholder: the engine has a single input path, so this is
            // a painted label, not a control.
            let r = Rect::from_min_size(
                pos2(col.left() + 2.0, y),
                vec2((col.width() - 4.0).max(8.0), SEL_H),
            );
            let p = ui.painter_at(r);
            p.rect_stroke(
                r,
                2.0,
                Stroke::new(1.0_f32, tokens.border),
                egui::StrokeKind::Inside,
            );
            let g = p.layout_no_wrap(
                "LINE ▾".into(),
                FontId::proportional(8.5),
                tokens.muted_text,
            );
            p.galley(r.center() - g.size() / 2.0, g, tokens.muted_text);
            y += SEL_H + OPT_GAP;
        }
        if prefs.ab_labels {
            let side = AB_H;
            chip(
                ui.painter(),
                Rect::from_center_size(pos2(col.center().x, y + AB_H / 2.0), vec2(side, side)),
                letter,
                c,
            );
            y += AB_H + OPT_GAP;
        }
        let cell = |y: &mut f32, knob: f32, gap_after: f32| {
            let r = Rect::from_min_size(pos2(col.left(), *y), vec2(col.width(), item_h(knob)));
            *y += item_h(knob) + gap_after;
            r
        };
        let ch = &mut deck.channel;
        let trim_r = cell(&mut y, dims.trim, EQ_GAP);
        let hover = format!(
            "Deck {letter} trim {:+.1} dB (±{TRIM_DB:.0} dB); double-click resets",
            ch.trim_db
        );
        // Classic Deck B can share the theme primary; keep TRIM distinct from FILTER.
        let trim_ink = if tokens.primary == c.outer {
            tokens.secondary
        } else {
            tokens.primary
        };
        mixer_rotary_styled(
            ui,
            trim_r,
            "TRIM",
            label_at,
            KnobShape::Square,
            &mut ch.trim_db,
            -TRIM_DB..=TRIM_DB,
            KNOB_DB_STEP,
            0.0,
            &format!("Deck {letter} trim"),
            hover,
            trim_ink,
            tokens,
        );
        layout.trim[d] = Some(trim_r);
        let mut eq = [Rect::NOTHING; 3];
        for (k, (band, label, name)) in [(2, "HIGH", "high"), (1, "MID", "mid"), (0, "LOW", "low")]
            .into_iter()
            .enumerate()
        {
            let r = cell(&mut y, dims.eq, if k == 2 { EQ_GAP } else { ITEM_GAP });
            let db = ch.eq_db[band];
            let shown = if db <= EQ_KILL_DB {
                "kill (−∞)".to_owned()
            } else {
                format!("{db:+.1} dB")
            };
            let hover = format!(
                "Deck {letter} EQ {name}: {shown}; centre 0 dB, left stop kills the band, right +{EQ_MAX_DB:.0} dB; double-click resets"
            );
            let mut pos = eq_pos_of(db);
            let before = pos;
            mixer_rotary_styled(
                ui,
                r,
                label,
                label_at,
                KnobShape::Round,
                &mut pos,
                -1.0..=1.0,
                FADER_STEP,
                0.0,
                &format!("Deck {letter} EQ {name}"),
                hover,
                c.outer,
                tokens,
            );
            if pos != before {
                ch.eq_db[band] = eq_db_at(pos);
            }
            eq[band] = r;
        }
        layout.eq[d] = Some(eq);
        let filter_r = cell(&mut y, dims.filter, 0.0);
        let side = match ch.filter {
            f if f.abs() < oxidj_audio::FILTER_DEADZONE => "off".to_owned(),
            f if f < 0.0 => format!("low-pass {:.0} %", -f * 100.0),
            f => format!("high-pass {:.0} %", f * 100.0),
        };
        let hover = format!(
            "Deck {letter} colour filter: {side} (left low-pass, right high-pass, centre off); double-click resets"
        );
        mixer_rotary_styled(
            ui,
            filter_r,
            "FILTER",
            label_at,
            KnobShape::Round,
            &mut ch.filter,
            -1.0..=1.0,
            FADER_STEP,
            0.0,
            &format!("Deck {letter} filter"),
            hover,
            c.outer,
            tokens,
        );
        layout.filter[d] = Some(filter_r);
        let fader = &mut deck.volume.0;
        let hover = format!(
            "Deck {letter} channel fader {:.0} % (unity at the top; double-click resets)",
            *fader * 100.0
        );
        mixer_vertical_fader(
            ui,
            geo.fader,
            "",
            fader,
            0.0..=1.0,
            FADER_STEP,
            1.0,
            &format!("Deck {letter} volume"),
            hover,
            c.outer,
            tokens,
        );
        layout.volume[d] = Some(geo.fader);
        let ch = &mut deck.channel;
        if icon_button(
            ui,
            geo.cue,
            Icon::Headphones,
            &format!("Deck {letter} cue bus"),
            ch.cue,
            true,
            tokens.warning,
            tokens.text,
            tokens,
        )
        .on_hover_text("Route to headphones (output channels 3–4)")
        .clicked()
        {
            ch.cue = !ch.cue;
        }
        layout.mixer_cue[d] = Some(geo.cue);
    }

    /// The three knobs in the given cells (master, cue mix, cue volume).
    fn monitor_knobs(
        &mut self,
        ui: &mut Ui,
        cells: [Rect; 3],
        label_at: LabelAt,
        tokens: ColorTokens,
        layout: &mut DeckLayout,
    ) {
        let [master_r, mix_r, volume_r] = cells;
        let knob = |ui: &mut Ui,
                    cell: Rect,
                    label: &str,
                    value: &mut f32,
                    default: f32,
                    name: &str,
                    hover: &str,
                    fill| {
            mixer_rotary_styled(
                ui,
                cell,
                label,
                label_at,
                KnobShape::Round,
                value,
                0.0..=1.0,
                FADER_STEP,
                default,
                name,
                hover.into(),
                fill,
                tokens,
            );
        };
        knob(
            ui,
            master_r,
            "MASTER",
            &mut self.decks.master,
            1.0,
            "Master volume",
            "Master volume; double-click resets",
            tokens.primary,
        );
        layout.master = Some(master_r);
        knob(
            ui,
            mix_r,
            "MIX",
            &mut self.decks.cue_mix,
            0.5,
            "Cue mix",
            "Headphones: A ↔ B; double-click centres",
            tokens.warning,
        );
        layout.cue_mix = Some(mix_r);
        knob(
            ui,
            volume_r,
            "CUE",
            &mut self.decks.cue_volume,
            1.0,
            "Cue volume",
            "Headphone volume; double-click resets",
            tokens.success,
        );
        layout.cue_volume = Some(volume_r);
    }

    fn monitor_meters(
        &mut self,
        ui: &mut Ui,
        rects: [Rect; 2],
        pad: f32,
        tokens: ColorTokens,
        layout: &mut DeckLayout,
    ) {
        level_meter_padded(
            ui,
            rects[0],
            self.decks.meters[2],
            "Master level",
            tokens,
            pad,
        );
        layout.meters[2] = Some(rects[0]);
        level_meter_padded(ui, rects[1], self.decks.meters[3], "Cue level", tokens, pad);
        layout.meters[3] = Some(rects[1]);
    }

    /// Compact column beside the mixer: master over cue mix | cue volume,
    /// then the master and cue meters.
    fn mixer_column(
        &mut self,
        ui: &mut Ui,
        mon: Rect,
        tokens: ColorTokens,
        layout: &mut DeckLayout,
    ) {
        let master_r = Rect::from_center_size(
            pos2(mon.center().x, mon.top() + MASTER_CELL.1 / 2.0),
            vec2(MASTER_CELL.0, MASTER_CELL.1),
        );
        let y = master_r.bottom() + 4.0;
        let small = vec2(SMALL_CELL.0, SMALL_CELL.1);
        let mix_r = Rect::from_min_size(pos2(mon.left() + 6.0, y), small);
        let volume_r = Rect::from_min_size(pos2(mon.right() - 6.0 - small.x, y), small);
        self.monitor_knobs(
            ui,
            [master_r, mix_r, volume_r],
            LabelAt::Above,
            tokens,
            layout,
        );
        let m = Rect::from_min_size(pos2(mon.left(), mix_r.bottom() + 4.0), vec2(MON_W, 24.0));
        let c = Rect::from_min_size(pos2(mon.left(), m.bottom()), vec2(MON_W, 24.0));
        self.monitor_meters(ui, [m, c], 6.0, tokens, layout);
    }

    /// Horizontal strip above or below the mixer: three knobs, then the two
    /// meters stacked in the remaining width.
    fn mixer_strip(&mut self, ui: &mut Ui, sr: Rect, tokens: ColorTokens, layout: &mut DeckLayout) {
        let g = 6.0;
        let top = |h: f32| sr.top() + (sr.height() - h) / 2.0;
        let master_r = Rect::from_min_size(
            pos2(sr.left() + g, top(MASTER_CELL.1)),
            vec2(MASTER_CELL.0, MASTER_CELL.1),
        );
        let small = vec2(SMALL_CELL.0, SMALL_CELL.1);
        let mix_r = Rect::from_min_size(pos2(master_r.right() + g, top(small.y)), small);
        let volume_r = Rect::from_min_size(pos2(mix_r.right() + g, top(small.y)), small);
        self.monitor_knobs(
            ui,
            [master_r, mix_r, volume_r],
            LabelAt::Above,
            tokens,
            layout,
        );
        let x = volume_r.right() + g;
        let w = (sr.right() - g - x).max(20.0);
        let m = Rect::from_min_size(pos2(x, sr.top() + 2.0), vec2(w, 16.0));
        let c = Rect::from_min_size(pos2(x, m.bottom() + 2.0), vec2(w, 16.0));
        self.monitor_meters(ui, [m, c], 5.0, tokens, layout);
    }

    /// Master and cue controls inside the top bar's one-control-high row.
    /// Drops the labels first, then the meters, when `avail` is short;
    /// draws nothing when even the knobs do not fit.
    pub(crate) fn toolbar_monitor(&mut self, ui: &mut Ui, tokens: ColorTokens, avail: f32) {
        let h = crate::theme::CONTROL_HEIGHT;
        let font = FontId::proportional(10.5);
        let label_w = |ui: &Ui, t: &str| {
            let g = ui
                .painter()
                .layout_no_wrap(t.to_owned(), font.clone(), tokens.muted_text);
            LABEL_W.max(g.size().x + 6.0)
        };
        let g = 6.0;
        let labelled = [
            label_w(ui, "MASTER") + BAR_KNOB,
            label_w(ui, "MIX") + BAR_KNOB,
            label_w(ui, "CUE") + BAR_KNOB,
        ];
        let meters_w = 64.0;
        let bare = [BAR_KNOB; 3];
        let sum = |w: &[f32; 3]| w.iter().sum::<f32>() + 2.0 * g;
        let (widths, label_at, meters) = if sum(&labelled) + g + meters_w <= avail {
            (labelled, LabelAt::Left, true)
        } else if sum(&bare) + g + meters_w <= avail {
            (bare, LabelAt::None, true)
        } else if sum(&bare) <= avail {
            (bare, LabelAt::None, false)
        } else {
            return;
        };
        let total = sum(&widths) + if meters { g + meters_w } else { 0.0 };
        let (outer, _) = ui.allocate_exact_size(vec2(total, h), Sense::hover());
        // Right-to-left parent: `outer` is already placed; fill it left to right.
        let mut x = outer.left();
        let mut cells = [Rect::NOTHING; 3];
        for (i, w) in widths.iter().enumerate() {
            cells[i] = Rect::from_min_size(pos2(x, outer.top()), vec2(*w, h));
            x += w + g;
        }
        let mut local = DeckLayout::default();
        self.monitor_knobs(ui, cells, label_at, tokens, &mut local);
        let mut rects = [Rect::NOTHING; 5];
        rects[..3].copy_from_slice(&cells);
        if meters {
            let m = Rect::from_min_size(pos2(x, outer.top()), vec2(meters_w, h / 2.0));
            let c = Rect::from_min_size(pos2(x, m.bottom()), vec2(meters_w, h / 2.0));
            self.monitor_meters(ui, [m, c], 4.0, tokens, &mut local);
            rects[3] = m;
            rects[4] = c;
        }
        self.shell.monitor = Some(rects);
    }

    /// Below the faders, under a separator line: show/hide toggle and the
    /// CROSSFADER label, the A <-> B rail, and the per-channel assign buttons.
    fn crossfader_area(
        &mut self,
        ui: &mut Ui,
        xf: Rect,
        tokens: ColorTokens,
        layout: &mut DeckLayout,
    ) {
        ui.painter().line_segment(
            [xf.left_top(), xf.right_top()],
            Stroke::new(1.0_f32, tokens.border),
        );
        let on = self.settings.crossfader;
        let label_row =
            Rect::from_min_size(pos2(xf.left(), xf.top() + 4.0), vec2(xf.width(), 14.0));
        let rail_row = Rect::from_min_size(
            pos2(xf.left(), label_row.bottom() + 2.0),
            vec2(xf.width(), 14.0),
        );
        let assign_y = rail_row.bottom() + 2.0;
        let icon_r = Rect::from_min_size(label_row.min, vec2(ICON_W - 6.0, 14.0));
        let response = ui.interact(icon_r, ui.id().with("crossfader-toggle"), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, "Crossfader section")
        });
        let painter = ui.painter_at(icon_r);
        if response.hovered() || response.has_focus() {
            painter.rect_filled(icon_r, 3.0, tokens.background);
        }
        if response.has_focus() {
            painter.rect_stroke(
                icon_r,
                3.0,
                Stroke::new(2.0_f32, tokens.focus),
                egui::StrokeKind::Inside,
            );
        }
        // Icon: a short rail with a cap (crossfader); dimmed when hidden.
        let ink = if on { tokens.text } else { tokens.muted_text };
        let cy = icon_r.center().y;
        painter.line_segment(
            [
                pos2(icon_r.left() + 3.0, cy),
                pos2(icon_r.right() - 3.0, cy),
            ],
            Stroke::new(1.5_f32, ink),
        );
        let cap = Rect::from_center_size(pos2(icon_r.center().x, cy), vec2(4.0, 9.0));
        if on {
            painter.rect_filled(cap, 1.5, ink);
        } else {
            painter.rect_stroke(
                cap,
                1.5,
                Stroke::new(1.0_f32, ink),
                egui::StrokeKind::Inside,
            );
        }
        let verb = if on { "Hide" } else { "Show" };
        if response
            .on_hover_text(format!(
                "{verb} the crossfader (Ctrl+Shift+5; also in Settings); hidden = out of the path"
            ))
            .clicked()
        {
            self.settings.crossfader = !on;
        }
        layout.crossfader_toggle = Some(icon_r);
        let g = ui.painter().layout_no_wrap(
            "CROSSFADER".into(),
            FontId::monospace(9.0),
            tokens.muted_text,
        );
        ui.painter().galley(
            pos2(
                label_row.center().x - g.size().x / 2.0,
                label_row.center().y - g.size().y / 2.0,
            ),
            g,
            tokens.muted_text,
        );
        let cy = rail_row.center().y;
        if on {
            let letters = |painter: &egui::Painter, text: &str, x: f32| {
                let g = painter.layout_no_wrap(
                    text.to_owned(),
                    FontId::proportional(10.5),
                    tokens.muted_text,
                );
                painter.galley(
                    pos2(x - g.size().x / 2.0, cy - g.size().y / 2.0),
                    g,
                    tokens.muted_text,
                );
            };
            letters(ui.painter(), DECKS[0], rail_row.left() + 5.0);
            letters(ui.painter(), DECKS[1], rail_row.right() - 5.0);
            let slider_r = rail_row.shrink2(vec2(14.0, 0.0));
            mixer_horizontal_fader(
                ui,
                slider_r,
                "",
                &mut self.decks.crossfader,
                0.0..=1.0,
                FADER_STEP,
                0.5,
                "Crossfader",
                "Crossfader A ↔ B; double-click centres".into(),
                tokens.primary,
                tokens,
            );
            let mid = slider_r.center().x;
            ui.painter().add(Shape::line_segment(
                [
                    pos2(mid, rail_row.top() + 1.0),
                    pos2(mid, rail_row.top() + 4.0),
                ],
                Stroke::new(1.0_f32, tokens.muted_text),
            ));
            layout.crossfader = Some(slider_r);
        } else {
            let g = ui.painter().layout_no_wrap(
                "hidden · out of the path".into(),
                FontId::proportional(10.0),
                tokens.muted_text,
            );
            ui.painter().galley(
                pos2(
                    rail_row.center().x - g.size().x / 2.0,
                    cy - g.size().y / 2.0,
                ),
                g,
                tokens.muted_text,
            );
        }
        // Assign A / THRU / B per channel (THRU shown as T: the cells are narrow).
        let crossfader_on = self.decks.crossfader_on;
        let half = (xf.width() - SPLIT) / 2.0;
        let btn_w = ((half - 4.0) / 3.0).max(8.0);
        for d in 0..2 {
            let letter = DECKS[d];
            let gx = if d == 0 { xf.left() } else { xf.right() - half };
            let ch = &mut self.decks.decks[d].channel;
            let mut rects = [Rect::NOTHING; 3];
            for (k, (assign, text, word)) in [
                (Assign::A, "A", "A"),
                (Assign::Thru, "T", "THRU"),
                (Assign::B, "B", "B"),
            ]
            .into_iter()
            .enumerate()
            {
                let r = Rect::from_min_size(
                    pos2(gx + k as f32 * (btn_w + 2.0), assign_y),
                    vec2(btn_w, 14.0),
                );
                rects[k] = r;
                let name = format!("Deck {letter} crossfader assign {word}");
                let hover = if crossfader_on {
                    format!("Deck {letter} follows crossfader side {word}")
                } else {
                    "Crossfader hidden: not in the path".into()
                };
                if deck_button(
                    ui,
                    r,
                    text,
                    &name,
                    ch.assign == assign,
                    crossfader_on,
                    tokens.primary,
                    tokens,
                )
                .on_hover_text(hover)
                .clicked()
                {
                    ch.assign = assign;
                }
            }
            layout.assign[d] = Some(rects);
        }
    }
}
