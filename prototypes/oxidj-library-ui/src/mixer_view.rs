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

use crate::decks::{chip, deck_button, reset_clicked, DeckColours, DeckLayout, MeterLevel, DECKS, GAP, ROW};
use crate::theme::ColorTokens;
use crate::LibraryApp;
use eframe::egui::{self, pos2, vec2, FontId, Rect, Sense, Shape, Stroke, Ui};
use oxidj_audio::{Assign, EQ_KILL_DB, EQ_MAX_DB, TRIM_DB};
use std::ops::RangeInclusive;

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
const ASSIGN_W: f32 = 34.0;
const CELL_GAP: f32 = 6.0;
/// Crossfader icon toggle.
const ICON_W: f32 = 24.0;
/// Least slider travel a mixer knob keeps at any supported width.
pub(crate) const SLIDER_MIN_W: f32 = 40.0;
/// Least knob cell (label column for "FILTER" + travel).
const CELL_MIN_W: f32 = 40.0 + SLIDER_MIN_W;
/// Deck chip column before a channel's first knob.
const CHIP_W: f32 = 24.0;
/// Least channel-strip width: the wider of its two rows at minimum cells.
const CHANNEL_MIN_W: f32 = {
    let top = CHIP_W + 4.0 * CELL_MIN_W + 3.0 * CELL_GAP;
    let bottom = CHIP_W + 2.0 * CELL_MIN_W + 3.0 * CELL_GAP + METER_W + 3.0 * ASSIGN_W + 4.0;
    if top > bottom { top } else { bottom }
};
/// Least centre column (master + meter) in the wide layout.
const CENTRE_MIN_W: f32 = 180.0;
/// Gap between a channel strip and its neighbour.
const SPLIT: f32 = 10.0;
/// Inset of the controls inside the section outline.
const INSET: f32 = 4.0;
/// Arrow-key step of the trim knob (dB).
pub(crate) const KNOB_DB_STEP: f64 = 1.0;
/// Arrow-key step of the EQ knobs (position), filter, faders and crossfader.
pub(crate) const FADER_STEP: f64 = 0.05;

/// A labelled, compact horizontal knob (native egui slider): exact rect,
/// AccessKit name, arrow keys move by `step`, double click resets to
/// `default`. Returns the response.
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
    let g = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(10.5), tokens.muted_text);
    let label_w = LABEL_W.max(g.size().x + 6.0);
    ui.painter().galley(pos2(cell.left(), cell.center().y - g.size().y / 2.0), g, tokens.muted_text);
    let slider_r = Rect::from_min_max(pos2(cell.left() + label_w, cell.top()), cell.max);
    let mut v = *value;
    let slider = ui
        .scope(|ui| {
            crate::track_waveform::slider_look(ui, tokens);
            ui.visuals_mut().selection.bg_fill = fill;
            ui.spacing_mut().slider_width = slider_r.width().max(8.0);
            ui.put(slider_r, egui::Slider::new(&mut v, range).show_value(false).step_by(step))
        })
        .inner;
    slider.widget_info(|| egui::WidgetInfo::slider(true, f64::from(v), name));
    let slider = slider.on_hover_text(hover);
    if reset_clicked(&slider) {
        v = default;
    }
    *value = v;
    slider
}

/// Compact Minimal Line rotary with per-widget persistent drag state.
#[allow(clippy::too_many_arguments)]
fn mixer_rotary(ui: &mut Ui, cell: Rect, label: &str, value: &mut f32, name: &str, hover: String, fill: egui::Color32, tokens: ColorTokens) -> egui::Response {
    let g = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(10.5), tokens.muted_text);
    let label_w = LABEL_W.max(g.size().x + 6.0);
    ui.painter().galley(pos2(cell.left(), cell.center().y - g.size().y / 2.0), g, tokens.muted_text);
    let rect = Rect::from_min_max(pos2(cell.left() + label_w, cell.top()), cell.max);
    let id = ui.id().with(("mixer-rotary", name));
    let mut response = ui.interact(rect, id, Sense::click_and_drag());
    let drag_id = id.with("angle");
    let center = rect.center();
    let start = -std::f32::consts::FRAC_PI_2 - 3.0 * std::f32::consts::FRAC_PI_4;
    let sweep = 3.0 * std::f32::consts::FRAC_PI_2;
    let before = *value;
    if let Some(pointer) = response.interact_pointer_pos() {
        let angle = (pointer.y - center.y).atan2(pointer.x - center.x);
        if response.drag_started() || response.dragged() {
            if let Some(previous) = ui.ctx().data(|d| d.get_temp::<f32>(drag_id)) {
                let delta = (angle - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                *value = (*value + 2.0 * delta / sweep).clamp(-1.0, 1.0);
            }
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, angle));
        } else if response.clicked() {
            let a = (angle - start).rem_euclid(std::f32::consts::TAU);
            *value = if a <= sweep { 2.0 * a / sweep - 1.0 } else if a - sweep <= (std::f32::consts::TAU - sweep) / 2.0 { 1.0 } else { -1.0 };
        } else if response.hovered() && ui.input(|i| i.pointer.primary_down()) {
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, angle));
        }
    }
    if response.drag_stopped() { ui.ctx().data_mut(|d| d.remove::<f32>(drag_id)); }
    if response.has_focus() {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() }));
        let step = ui.input(|i| (i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowUp)) as i32 - (i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowDown)) as i32);
        if step != 0 { *value = (((*value / FADER_STEP as f32).round() + step as f32) * FADER_STEP as f32).clamp(-1.0, 1.0); }
    }
    ui.input(|input| {
        use egui::accesskit::{Action, ActionData};
        let delta = input.num_accesskit_action_requests(id, Action::Increment) as i32
            - input.num_accesskit_action_requests(id, Action::Decrement) as i32;
        if delta != 0 {
            *value = (*value + delta as f32 * FADER_STEP as f32).clamp(-1.0, 1.0);
        }
        for request in input.accesskit_action_requests(id, Action::SetValue) {
            if let Some(ActionData::NumericValue(v)) = request.data {
                if v.is_finite() { *value = v.clamp(-1.0, 1.0) as f32; }
            }
        }
    });
    if reset_clicked(&response) { *value = 0.0; }
    if *value != before { response.mark_changed(); }
    response.widget_info(|| egui::WidgetInfo::slider(true, f64::from(*value), name));
    ui.ctx().accesskit_node_builder(id, |node| {
        use egui::accesskit::Action;
        node.set_min_numeric_value(-1.0);
        node.set_max_numeric_value(1.0);
        node.set_numeric_value_step(FADER_STEP);
        node.add_action(Action::SetValue);
        if *value < 1.0 { node.add_action(Action::Increment); }
        if *value > -1.0 { node.add_action(Action::Decrement); }
    });
    let painter = ui.painter_at(rect);
    let radius = (rect.width().min(rect.height()) * 0.5 - 3.0).max(0.0);
    painter.circle_filled(center, radius, tokens.background);
    painter.circle_stroke(center, radius, Stroke::new(1.0_f32, if response.has_focus() { tokens.focus } else { tokens.border }));
    for tick in 0..=6 {
        let angle = start + tick as f32 / 6.0 * sweep;
        let dir = vec2(angle.cos(), angle.sin());
        painter.line_segment([center + dir * (radius + 1.0), center + dir * (radius + 2.0)], Stroke::new(1.0_f32, tokens.muted_text));
    }
    let angle = start + ((*value + 1.0) * 0.5).clamp(0.0, 1.0) * sweep;
    painter.line_segment([center, center + vec2(angle.cos(), angle.sin()) * (radius * 0.7)], Stroke::new(2.0_f32, fill));
    response.on_hover_text(hover)
}

/// EQ knob position −1 (kill) .. 0 (centre, 0 dB) .. +1 (+6 dB) to dB:
/// linear in dB on each side, like an isolator knob at 12 o'clock.
pub(crate) fn eq_db_at(pos: f32) -> f32 {
    let pos = if pos.is_nan() { 0.0 } else { pos.clamp(-1.0, 1.0) };
    // The left stop is kill even after float drift of the slider steps.
    if pos <= -0.999 {
        return EQ_KILL_DB;
    }
    if pos >= 0.0 { pos * EQ_MAX_DB } else { -pos * EQ_KILL_DB }
}

/// Inverse of [`eq_db_at`].
pub(crate) fn eq_pos_of(db: f32) -> f32 {
    if db >= 0.0 { (db / EQ_MAX_DB).min(1.0) } else { (db / EQ_KILL_DB).min(1.0) * -1.0 }
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
pub(crate) fn level_meter(ui: &mut Ui, rect: Rect, meter: MeterLevel, name: &str, tokens: ColorTokens) {
    let response = ui.interact(rect, ui.id().with(("level-meter", name)), Sense::hover());
    let text = format!("{name}: {}{}", db_text(meter.level), if meter.clipping() { ", clip" } else { "" });
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ProgressIndicator, true, &text));
    let painter = ui.painter_at(rect);
    let bar = Rect::from_min_max(rect.min + vec2(0.0, 6.0), pos2(rect.right() - CLIP_W - 2.0, rect.bottom() - 6.0));
    painter.rect_filled(bar, 2.0, tokens.background);
    let f = meter_fraction(meter.level);
    let hot = meter_fraction(10f32.powf(METER_HOT_DB / 20.0));
    let x = |frac: f32| bar.left() + bar.width() * frac;
    if f > 0.0 {
        let green = Rect::from_min_max(bar.min, pos2(x(f.min(hot)), bar.bottom()));
        painter.rect_filled(green, 1.0, tokens.success);
        if f > hot {
            painter.rect_filled(Rect::from_min_max(pos2(x(hot), bar.top()), pos2(x(f), bar.bottom())), 1.0, tokens.warning);
        }
    }
    painter.rect_stroke(bar, 2.0, Stroke::new(1.0_f32, tokens.border), egui::StrokeKind::Inside);
    let clip = Rect::from_min_max(pos2(rect.right() - CLIP_W, bar.top()), pos2(rect.right(), bar.bottom()));
    painter.rect_filled(clip, 2.0, if meter.clipping() { tokens.error } else { tokens.background });
    painter.rect_stroke(clip, 2.0, Stroke::new(1.0_f32, tokens.border), egui::StrokeKind::Inside);
    response.on_hover_text(text);
}

fn row(rect: Rect, k: usize) -> Rect {
    rows(rect, k, 1)
}

/// Rows `k .. k + n` of `rect` (ROW high, GAP apart).
fn rows(rect: Rect, k: usize, n: usize) -> Rect {
    let top = rect.top() + k as f32 * (ROW + GAP);
    Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), n as f32 * ROW + (n as f32 - 1.0) * GAP))
}

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
        let centre = (inner * 0.24).clamp(CENTRE_MIN_W, 300.0);
        if (inner - centre - 2.0 * SPLIT) / 2.0 >= CHANNEL_MIN_W {
            Self::Wide
        } else if (inner - SPLIT) / 2.0 >= CHANNEL_MIN_W {
            Self::CentreBelow
        } else {
            Self::Stacked
        }
    }

    fn rows(self) -> usize {
        match self {
            Self::Wide => 2,
            Self::CentreBelow => 3,
            Self::Stacked => 5,
        }
    }
}

/// Mixer section height for a section `width` wide.
pub(crate) fn mixer_height(width: f32) -> f32 {
    let n = MixerFit::of(width).rows() as f32;
    n * ROW + (n - 1.0) * GAP + 2.0 * INSET
}

/// One centre row split into (master + meter on the right, crossfader on
/// the left), the master keeping a full-size knob.
fn split_centre(r: Rect) -> (Rect, Rect) {
    let master_w = (r.width() * 0.4).max(CELL_MIN_W + CELL_GAP + METER_W);
    let master = Rect::from_min_max(pos2(r.right() - master_w, r.top()), r.max);
    let xf = Rect::from_min_max(r.min, pos2(master.left() - 2.0 * CELL_GAP, r.bottom()));
    (master, xf)
}

impl LibraryApp {
    /// The mixer section (A left, master + crossfader centre, B right).
    pub(crate) fn mixer(&mut self, ui: &mut Ui, r: Rect, tokens: ColorTokens, colours: [DeckColours; 2], layout: &mut DeckLayout) {
        // Outline only: a surface fill would hide the slider rails.
        ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0_f32, tokens.border), egui::StrokeKind::Inside);
        let inner = r.shrink(INSET);
        let halves = |band: Rect| {
            let w = (band.width() - SPLIT) / 2.0;
            [Rect::from_min_size(band.min, vec2(w, band.height())), Rect::from_min_max(pos2(band.right() - w, band.top()), band.max)]
        };
        let (cols, master_row, xf_row) = match MixerFit::of(r.width()) {
            MixerFit::Wide => {
                let centre_w = (inner.width() * 0.24).clamp(CENTRE_MIN_W, 300.0);
                let centre = Rect::from_center_size(inner.center(), vec2(centre_w, inner.height()));
                let cols = [
                    Rect::from_min_max(inner.min, pos2(centre.left() - SPLIT, inner.bottom())),
                    Rect::from_min_max(pos2(centre.right() + SPLIT, inner.top()), inner.max),
                ];
                (cols, row(centre, 0), row(centre, 1))
            }
            MixerFit::CentreBelow => {
                let (m, x) = split_centre(row(inner, 2));
                (halves(rows(inner, 0, 2)), m, x)
            }
            MixerFit::Stacked => {
                let (m, x) = split_centre(row(inner, 2));
                ([rows(inner, 0, 2), rows(inner, 3, 2)], m, x)
            }
        };
        // Creation order = Tab order: A, centre, B (left to right / top down).
        self.channel_strip(ui, 0, cols[0], tokens, colours[0], layout);
        self.mixer_centre(ui, master_row, xf_row, tokens, layout);
        self.channel_strip(ui, 1, cols[1], tokens, colours[1], layout);
        layout.mixer = Some(r);
    }

    fn channel_strip(&mut self, ui: &mut Ui, d: usize, col: Rect, tokens: ColorTokens, c: DeckColours, layout: &mut DeckLayout) {
        let letter = DECKS[d];
        let crossfader_on = self.decks.crossfader_on;
        let meter = self.decks.meters[d];
        let deck = &mut self.decks.decks[d];
        let top = row(col, 0);
        chip(ui.painter(), Rect::from_min_size(top.min + vec2(0.0, 3.0), vec2(18.0, 18.0)), letter, c);
        let x0 = top.left() + CHIP_W;
        let cw = ((top.right() - x0) - 3.0 * CELL_GAP) / 4.0;
        let cell = |k: usize, rr: Rect| Rect::from_min_size(pos2(x0 + k as f32 * (cw + CELL_GAP), rr.top()), vec2(cw, ROW));
        let ch = &mut deck.channel;
        let trim_r = cell(0, top);
        let hover = format!("Deck {letter} trim {:+.1} dB (±{TRIM_DB:.0} dB); double-click resets", ch.trim_db);
        mixer_knob(ui, trim_r, "TRIM", &mut ch.trim_db, -TRIM_DB..=TRIM_DB, KNOB_DB_STEP, 0.0, &format!("Deck {letter} trim"), hover, c.outer, tokens);
        layout.trim[d] = Some(trim_r);
        // HI, MID, LOW left to right, like a channel strip read top down.
        let mut eq = [Rect::NOTHING; 3];
        for (k, (band, label, name)) in [(2, "HI", "high"), (1, "MID", "mid"), (0, "LOW", "low")].into_iter().enumerate() {
            let r = cell(k + 1, top);
            let db = ch.eq_db[band];
            let shown = if db <= EQ_KILL_DB { "kill (−∞)".to_owned() } else { format!("{db:+.1} dB") };
            let hover = format!("Deck {letter} EQ {name}: {shown}; centre 0 dB, left stop kills the band, right +{EQ_MAX_DB:.0} dB; double-click resets");
            let mut pos = eq_pos_of(db);
            let before = pos;
            mixer_rotary(ui, r, label, &mut pos, &format!("Deck {letter} EQ {name}"), hover, c.outer, tokens);
            if pos != before {
                ch.eq_db[band] = eq_db_at(pos);
            }
            eq[band] = r;
        }
        layout.eq[d] = Some(eq);
        // Row 2: FILTER, VOL, meter, assign.
        let bottom = row(col, 1);
        let assign_w = 3.0 * ASSIGN_W + 2.0 * 2.0;
        let assign_x = bottom.right() - assign_w;
        let meter_r = Rect::from_min_size(pos2(assign_x - CELL_GAP - METER_W, bottom.top()), vec2(METER_W, ROW));
        let rest = (meter_r.left() - CELL_GAP - x0 - CELL_GAP) / 2.0;
        let filter_r = Rect::from_min_size(pos2(x0, bottom.top()), vec2(rest, ROW));
        let vol_r = Rect::from_min_size(pos2(filter_r.right() + CELL_GAP, bottom.top()), vec2(rest, ROW));
        let side = match ch.filter {
            f if f.abs() < oxidj_audio::FILTER_DEADZONE => "off".to_owned(),
            f if f < 0.0 => format!("low-pass {:.0} %", -f * 100.0),
            f => format!("high-pass {:.0} %", f * 100.0),
        };
        let hover = format!("Deck {letter} colour filter: {side} (left low-pass, right high-pass, centre off); double-click resets");
        mixer_rotary(ui, filter_r, "FILTER", &mut ch.filter, &format!("Deck {letter} filter"), hover, c.outer, tokens);
        layout.filter[d] = Some(filter_r);
        let fader = &mut deck.volume.0;
        let hover = format!("Deck {letter} channel fader {:.0} % (unity at the top; double-click resets)", *fader * 100.0);
        mixer_knob(ui, vol_r, "VOL", fader, 0.0..=1.0, FADER_STEP, 1.0, &format!("Deck {letter} volume"), hover, c.outer, tokens);
        layout.volume[d] = Some(vol_r);
        level_meter(ui, meter_r, meter, &format!("Deck {letter} level"), tokens);
        layout.meters[d] = Some(meter_r);
        let ch = &mut deck.channel;
        let mut rects = [Rect::NOTHING; 3];
        for (k, (assign, text)) in [(Assign::A, "A"), (Assign::Thru, "THRU"), (Assign::B, "B")].into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(assign_x + k as f32 * (ASSIGN_W + 2.0), bottom.top()), vec2(ASSIGN_W, ROW));
            rects[k] = r;
            let name = format!("Deck {letter} crossfader assign {text}");
            let hover = if crossfader_on { format!("Deck {letter} follows crossfader side {text}") } else { "Crossfader hidden: not in the path".into() };
            if deck_button(ui, r, text, &name, ch.assign == assign, crossfader_on, tokens.primary, tokens).on_hover_text(hover).clicked() {
                ch.assign = assign;
            }
        }
        layout.assign[d] = Some(rects);
    }

    /// Master volume + meter in `top`, crossfader toggle + rail in `bottom`.
    fn mixer_centre(&mut self, ui: &mut Ui, top: Rect, bottom: Rect, tokens: ColorTokens, layout: &mut DeckLayout) {
        let meter_r = Rect::from_min_size(pos2(top.right() - METER_W, top.top()), vec2(METER_W, ROW));
        let master_r = Rect::from_min_max(top.min, pos2(meter_r.left() - CELL_GAP, top.bottom()));
        let hover = format!("Master volume {:.0} % (double-click resets)", self.decks.master * 100.0);
        mixer_knob(ui, master_r, "MASTER", &mut self.decks.master, 0.0..=1.0, FADER_STEP, 1.0, "Master volume", hover, tokens.primary, tokens);
        layout.master = Some(master_r);
        level_meter(ui, meter_r, self.decks.meters[2], "Master level", tokens);
        layout.meters[2] = Some(meter_r);
        // Crossfader icon toggle, then the crossfader.
        let icon_r = Rect::from_min_size(bottom.min, vec2(ICON_W, ROW));
        let on = self.settings.crossfader;
        let response = ui.interact(icon_r, ui.id().with("crossfader-toggle"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, "Crossfader section"));
        let painter = ui.painter_at(icon_r);
        if response.hovered() || response.has_focus() {
            painter.rect_filled(icon_r, 3.0, tokens.background);
        }
        if response.has_focus() {
            painter.rect_stroke(icon_r, 3.0, Stroke::new(2.0_f32, tokens.focus), egui::StrokeKind::Inside);
        }
        // Icon: a short rail with a cap (crossfader); dimmed when hidden.
        let ink = if on { tokens.text } else { tokens.muted_text };
        let cy = icon_r.center().y;
        painter.line_segment([pos2(icon_r.left() + 5.0, cy), pos2(icon_r.right() - 5.0, cy)], Stroke::new(1.5_f32, ink));
        let cap = Rect::from_center_size(pos2(icon_r.center().x, cy), vec2(5.0, 11.0));
        if on {
            painter.rect_filled(cap, 1.5, ink);
        } else {
            painter.rect_stroke(cap, 1.5, Stroke::new(1.0_f32, ink), egui::StrokeKind::Inside);
        }
        let verb = if on { "Hide" } else { "Show" };
        if response.on_hover_text(format!("{verb} the crossfader (Ctrl+Shift+5; also in Settings); hidden = out of the path")).clicked() {
            self.settings.crossfader = !on;
        }
        layout.crossfader_toggle = Some(icon_r);
        let rail = Rect::from_min_max(pos2(icon_r.right() + 4.0, bottom.top()), bottom.max);
        if on {
            let letters = |painter: &egui::Painter, text: &str, x: f32| {
                let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(10.5), tokens.muted_text);
                painter.galley(pos2(x - g.size().x / 2.0, cy - g.size().y / 2.0), g, tokens.muted_text);
            };
            letters(ui.painter(), DECKS[0], rail.left() + 5.0);
            letters(ui.painter(), DECKS[1], rail.right() - 5.0);
            let slider_r = rail.shrink2(vec2(14.0, 0.0));
            let mut x = self.decks.crossfader;
            let slider = ui
                .scope(|ui| {
                    crate::track_waveform::slider_look(ui, tokens);
                    ui.spacing_mut().slider_width = slider_r.width().max(8.0);
                    ui.put(slider_r, egui::Slider::new(&mut x, 0.0..=1.0).show_value(false).step_by(FADER_STEP))
                })
                .inner;
            slider.widget_info(|| egui::WidgetInfo::slider(true, f64::from(x), "Crossfader"));
            let slider = slider.on_hover_text(format!("Crossfader A ↔ B ({:?} curve, Settings); centre = both at unity; double-click centres", self.settings.xfader_curve));
            if reset_clicked(&slider) {
                x = 0.5;
            }
            self.decks.crossfader = x;
            // Centre detent mark.
            let mid = slider_r.center().x;
            ui.painter().add(Shape::line_segment([pos2(mid, bottom.top() + 2.0), pos2(mid, bottom.top() + 6.0)], Stroke::new(1.0_f32, tokens.muted_text)));
            layout.crossfader = Some(slider_r);
        } else {
            let g = ui.painter().layout_no_wrap("crossfader hidden · out of the path".into(), FontId::proportional(11.0), tokens.muted_text);
            ui.painter().galley(pos2(rail.left() + 4.0, cy - g.size().y / 2.0), g, tokens.muted_text);
        }
    }
}
