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
    DECKS, DeckColours, DeckLayout, GAP, MeterLevel, ROW, chip, deck_button, reset_clicked,
};
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
/// Level meter width.
const METER_W: f32 = 64.0;
/// Clip indicator at the meter's right end.
const CLIP_W: f32 = 8.0;
/// Meter scale floor (dBFS).
pub(crate) const METER_FLOOR_DB: f32 = -48.0;
/// Amber zone starts here (dBFS).
const METER_HOT_DB: f32 = -6.0;
/// One crossfader assign button.
const ASSIGN_W: f32 = 26.0;
const CELL_GAP: f32 = 6.0;
/// Crossfader icon toggle.
const ICON_W: f32 = 24.0;
/// Least slider travel a mixer knob keeps at any supported width.
pub(crate) const SLIDER_MIN_W: f32 = 40.0;
/// Least knob cell (label column for "FILTER" + travel).
const CELL_MIN_W: f32 = 40.0 + SLIDER_MIN_W;
/// Compact cell for short-label rotaries (cue mix, cue volume).
const CUE_CELL_W: f32 = 28.0 + SLIDER_MIN_W;
/// Deck chip column before a channel's first knob.
const CHIP_W: f32 = 24.0;
/// Least channel strip: four labelled rotary cells above FILTER, vertical
/// fader, compact meter, assign buttons and headphone CUE.
const CHANNEL_MIN_W: f32 = 378.0;
/// Least centre column (master + meter + cue mix + cue volume + cue meter).
const CENTRE_MIN_W: f32 = METER_W + CELL_MIN_W + 2.0 * CUE_CELL_W + METER_W + 4.0 * CELL_GAP;
/// Width of the CUE button in the mixer channel strip.
const CUE_W: f32 = 36.0;
/// Gap between a channel strip and its neighbour.
const SPLIT: f32 = 10.0;
/// Inset of the controls inside the section outline.
const INSET: f32 = 4.0;
/// Height of the vertical fader row (label + slider travel + margins).
const FADER_ROW_H: f32 = SLIDER_MIN_W + 28.0;
/// Arrow-key step of the trim knob (dB).
pub(crate) const KNOB_DB_STEP: f64 = 1.0;
/// Arrow-key step of the EQ knobs (position), filter, faders and crossfader.
pub(crate) const FADER_STEP: f64 = 0.05;

/// Consume vertical wheel input while the pointer is over a mixer control.
fn mixer_wheel(
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

/// Compact Minimal Line rotary with per-widget persistent drag state.
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
    mixer_rotary_range(
        ui,
        cell,
        label,
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

#[allow(clippy::too_many_arguments)]
fn mixer_rotary_range(
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
    let min = *range.start();
    let max = *range.end();
    let span = max - min;
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
    let rect = Rect::from_min_max(pos2(cell.left() + label_w, cell.top()), cell.max);
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
    let radius = (rect.width().min(rect.height()) * 0.5 - 3.0).max(0.0);
    painter.circle_filled(center, radius, tokens.background);
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            1.0_f32,
            if response.has_focus() {
                tokens.focus
            } else {
                tokens.border
            },
        ),
    );
    for tick in 0..=6 {
        let angle = start + tick as f32 / 6.0 * sweep;
        let dir = vec2(angle.cos(), angle.sin());
        painter.line_segment(
            [center + dir * (radius + 1.0), center + dir * (radius + 2.0)],
            Stroke::new(1.0_f32, tokens.muted_text),
        );
    }
    let angle = start + ((*value - min) / span).clamp(0.0, 1.0) * sweep;
    painter.line_segment(
        [
            center,
            center + vec2(angle.cos(), angle.sin()) * (radius * 0.7),
        ],
        Stroke::new(2.0_f32, fill),
    );
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
pub(crate) fn level_meter(
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
    let bar = Rect::from_min_max(
        rect.min + vec2(0.0, 6.0),
        pos2(rect.right() - CLIP_W - 2.0, rect.bottom() - 6.0),
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

#[allow(dead_code)]
/// How the mixer arranges itself for a section `width` wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MixerFit {
    /// A | master + crossfader | B, two rows.
    Wide,
    /// A | B on two rows, crossfader + master on a third row below.
    CentreBelow,
    /// A (two rows), crossfader + master, B (two rows): five rows.
    Stacked,
}

impl MixerFit {
    pub(crate) fn of(width: f32) -> Self {
        let inner = width - 2.0 * INSET;
        let centre = (inner * 0.24).clamp(CENTRE_MIN_W, CENTRE_MIN_W.max(300.0));
        if (inner - centre - 2.0 * SPLIT) / 2.0 >= CHANNEL_MIN_W {
            Self::Wide
        } else if (inner - SPLIT) / 2.0 >= CHANNEL_MIN_W {
            Self::CentreBelow
        } else {
            Self::Stacked
        }
    }
}

/// Mixer height accounts for the full vertical fader travel and labels.
pub(crate) fn mixer_height(width: f32) -> f32 {
    let bands = match MixerFit::of(width) {
        MixerFit::Wide => 1.0,
        MixerFit::CentreBelow => 2.0,
        MixerFit::Stacked => 3.0,
    };
    bands * (ROW + GAP + FADER_ROW_H) + (bands - 1.0) * GAP + 2.0 * INSET
}

impl LibraryApp {
    /// The mixer section (A left, master + crossfader centre, B right).
    pub(crate) fn mixer(
        &mut self,
        ui: &mut Ui,
        r: Rect,
        tokens: ColorTokens,
        colours: [DeckColours; 2],
        layout: &mut DeckLayout,
    ) {
        // Outline only: a surface fill would hide the slider rails.
        ui.painter().rect_stroke(
            r,
            3.0,
            Stroke::new(1.0_f32, tokens.border),
            egui::StrokeKind::Inside,
        );
        let inner = r.shrink(INSET);
        let halves = |band: Rect| {
            let w = (band.width() - SPLIT) / 2.0;
            [
                Rect::from_min_size(band.min, vec2(w, band.height())),
                Rect::from_min_max(pos2(band.right() - w, band.top()), band.max),
            ]
        };
        let h = ROW + GAP + FADER_ROW_H;
        let band = |y| Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), h));
        let (cols, centre) = match MixerFit::of(r.width()) {
            MixerFit::Wide => {
                let centre = Rect::from_center_size(inner.center(), vec2(CENTRE_MIN_W, h));
                (
                    [
                        Rect::from_min_max(inner.min, pos2(centre.left() - SPLIT, inner.bottom())),
                        Rect::from_min_max(pos2(centre.right() + SPLIT, inner.top()), inner.max),
                    ],
                    centre,
                )
            }
            MixerFit::CentreBelow => (halves(band(inner.top())), band(inner.top() + h + GAP)),
            MixerFit::Stacked => (
                [band(inner.top()), band(inner.top() + 2.0 * (h + GAP))],
                band(inner.top() + h + GAP),
            ),
        };
        let master_row = Rect::from_min_size(centre.min, vec2(centre.width(), FADER_ROW_H));
        let xf_row = Rect::from_min_size(
            pos2(centre.left(), master_row.bottom() + GAP),
            vec2(centre.width(), ROW),
        );
        // Creation order = Tab order: A, centre, B (left to right / top down).
        self.channel_strip(ui, 0, cols[0], tokens, colours[0], layout);
        self.mixer_centre(ui, master_row, xf_row, tokens, layout);
        self.channel_strip(ui, 1, cols[1], tokens, colours[1], layout);
        layout.mixer = Some(r);
    }

    fn channel_strip(
        &mut self,
        ui: &mut Ui,
        d: usize,
        col: Rect,
        tokens: ColorTokens,
        c: DeckColours,
        layout: &mut DeckLayout,
    ) {
        let letter = DECKS[d];
        let crossfader_on = self.decks.crossfader_on;
        let meter = self.decks.meters[d];
        let deck = &mut self.decks.decks[d];
        let top = Rect::from_min_size(col.min, vec2(col.width(), ROW));
        chip(
            ui.painter(),
            Rect::from_min_size(top.min + vec2(0.0, 3.0), vec2(18.0, 18.0)),
            letter,
            c,
        );
        let x0 = top.left() + CHIP_W;
        let cw = ((top.right() - x0) - 3.0 * CELL_GAP) / 4.0;
        let cell = |k: usize, rr: Rect| {
            Rect::from_min_size(
                pos2(x0 + k as f32 * (cw + CELL_GAP), rr.top()),
                vec2(cw, ROW),
            )
        };
        let ch = &mut deck.channel;
        let trim_r = cell(0, top);
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
        mixer_rotary_range(
            ui,
            trim_r,
            "TRIM",
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
        // HI, MID, LOW left to right, like a channel strip read top down.
        let mut eq = [Rect::NOTHING; 3];
        for (k, (band, label, name)) in [(2, "HI", "high"), (1, "MID", "mid"), (0, "LOW", "low")]
            .into_iter()
            .enumerate()
        {
            let r = cell(k + 1, top);
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
            mixer_rotary(
                ui,
                r,
                label,
                &mut pos,
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
        let bottom = Rect::from_min_size(
            pos2(col.left(), top.bottom() + GAP),
            vec2(col.width(), FADER_ROW_H),
        );
        let cue_r =
            Rect::from_min_size(pos2(bottom.right() - CUE_W, bottom.top()), vec2(CUE_W, ROW));
        let assign_w = 3.0 * ASSIGN_W + 4.0;
        let assign_x = cue_r.left() - CELL_GAP - assign_w;
        let meter_r = Rect::from_min_size(
            pos2(assign_x - CELL_GAP - 30.0, bottom.top()),
            vec2(30.0, FADER_ROW_H),
        );
        let filter_r = Rect::from_min_size(bottom.min, vec2(CELL_MIN_W + 2.0, ROW));
        let vol_r = Rect::from_min_max(
            pos2(filter_r.right() + CELL_GAP, bottom.top()),
            pos2(meter_r.left() - CELL_GAP, bottom.bottom()),
        );
        let side = match ch.filter {
            f if f.abs() < oxidj_audio::FILTER_DEADZONE => "off".to_owned(),
            f if f < 0.0 => format!("low-pass {:.0} %", -f * 100.0),
            f => format!("high-pass {:.0} %", f * 100.0),
        };
        let hover = format!(
            "Deck {letter} colour filter: {side} (left low-pass, right high-pass, centre off); double-click resets"
        );
        mixer_rotary(
            ui,
            filter_r,
            "FILTER",
            &mut ch.filter,
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
            vol_r,
            "VOL",
            fader,
            0.0..=1.0,
            FADER_STEP,
            1.0,
            &format!("Deck {letter} volume"),
            hover,
            c.outer,
            tokens,
        );
        layout.volume[d] = Some(vol_r);
        level_meter(ui, meter_r, meter, &format!("Deck {letter} level"), tokens);
        layout.meters[d] = Some(meter_r);
        let ch = &mut deck.channel;
        let mut rects = [Rect::NOTHING; 3];
        for (k, (assign, text)) in [(Assign::A, "A"), (Assign::Thru, "THRU"), (Assign::B, "B")]
            .into_iter()
            .enumerate()
        {
            let r = Rect::from_min_size(
                pos2(assign_x + k as f32 * (ASSIGN_W + 2.0), bottom.top()),
                vec2(ASSIGN_W, ROW),
            );
            rects[k] = r;
            let name = format!("Deck {letter} crossfader assign {text}");
            let hover = if crossfader_on {
                format!("Deck {letter} follows crossfader side {text}")
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
        if deck_button(
            ui,
            cue_r,
            "CUE",
            &format!("Deck {letter} cue bus"),
            ch.cue,
            true,
            tokens.warning,
            tokens,
        )
        .on_hover_text("Route to headphones (output channels 3–4)")
        .clicked()
        {
            ch.cue = !ch.cue;
        }
        layout.mixer_cue[d] = Some(cue_r);
    }

    /// Master volume + meter in `top`, crossfader toggle + rail in `bottom`.
    fn mixer_centre(
        &mut self,
        ui: &mut Ui,
        top: Rect,
        bottom: Rect,
        tokens: ColorTokens,
        layout: &mut DeckLayout,
    ) {
        let widths = [METER_W, CELL_MIN_W, CUE_CELL_W, CUE_CELL_W, METER_W];
        let mut x = top.left();
        let mut cell = |k: usize| {
            let r = Rect::from_min_size(pos2(x, top.top()), vec2(widths[k], top.height()));
            x += widths[k] + CELL_GAP;
            r
        };
        let meter_r = cell(0);
        level_meter(ui, meter_r, self.decks.meters[2], "Master level", tokens);
        layout.meters[2] = Some(meter_r);
        let master_r = cell(1);
        mixer_rotary_range(
            ui,
            master_r,
            "MASTER",
            &mut self.decks.master,
            0.0..=1.0,
            FADER_STEP,
            1.0,
            "Master volume",
            "Master volume; double-click resets".into(),
            tokens.primary,
            tokens,
        );
        layout.master = Some(master_r);
        let mix_r = cell(2);
        mixer_rotary_range(
            ui,
            mix_r,
            "MIX",
            &mut self.decks.cue_mix,
            0.0..=1.0,
            FADER_STEP,
            0.5,
            "Cue mix",
            "Headphones: A ↔ B; double-click centres".into(),
            tokens.warning,
            tokens,
        );
        layout.cue_mix = Some(mix_r);
        let volume_r = cell(3);
        mixer_rotary_range(
            ui,
            volume_r,
            "CUE",
            &mut self.decks.cue_volume,
            0.0..=1.0,
            FADER_STEP,
            1.0,
            "Cue volume",
            "Headphone volume; double-click resets".into(),
            tokens.success,
            tokens,
        );
        layout.cue_volume = Some(volume_r);
        let cue_meter = cell(4);
        level_meter(ui, cue_meter, self.decks.meters[3], "Cue level", tokens);
        layout.meters[3] = Some(cue_meter);
        // Crossfader icon toggle, then the crossfader.
        let icon_r = Rect::from_min_size(bottom.min, vec2(ICON_W, ROW));
        let on = self.settings.crossfader;
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
                pos2(icon_r.left() + 5.0, cy),
                pos2(icon_r.right() - 5.0, cy),
            ],
            Stroke::new(1.5_f32, ink),
        );
        let cap = Rect::from_center_size(pos2(icon_r.center().x, cy), vec2(5.0, 11.0));
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
        let rail = Rect::from_min_max(pos2(icon_r.right() + 4.0, bottom.top()), bottom.max);
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
            letters(ui.painter(), DECKS[0], rail.left() + 5.0);
            letters(ui.painter(), DECKS[1], rail.right() - 5.0);
            let slider_r = rail.shrink2(vec2(14.0, 0.0));
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
            // Centre detent mark.
            let mid = slider_r.center().x;
            ui.painter().add(Shape::line_segment(
                [pos2(mid, bottom.top() + 2.0), pos2(mid, bottom.top() + 6.0)],
                Stroke::new(1.0_f32, tokens.muted_text),
            ));
            layout.crossfader = Some(slider_r);
        } else {
            let g = ui.painter().layout_no_wrap(
                "crossfader hidden · out of the path".into(),
                FontId::proportional(11.0),
                tokens.muted_text,
            );
            ui.painter().galley(
                pos2(rail.left() + 4.0, cy - g.size().y / 2.0),
                g,
                tokens.muted_text,
            );
        }
    }
}
