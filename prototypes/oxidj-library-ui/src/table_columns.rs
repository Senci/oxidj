//! Shared track-table header controller.
use super::*;

/// Shared column menu: a normal keyboard-focusable menu button is always
/// reachable, even when the horizontal header is scrolled out of view.
fn column_options(
    ui: &mut egui::Ui,
    columns: &mut crate::settings::TableColumns,
    labels: [&str; 6],
    defaults: crate::settings::TableColumns,
) -> bool {
    for (index, name) in labels.into_iter().enumerate() {
        let enabled = !columns.visible[index] || columns.visible.iter().filter(|v| **v).count() > 1;
        ui.add_enabled(
            enabled,
            egui::Checkbox::new(&mut columns.visible[index], name),
        );
    }
    ui.separator();
    let fit = ui.button("Auto-fit columns").clicked();
    if ui.button("Reset columns").clicked() {
        *columns = defaults;
        ui.close();
    }
    ui.label("Drag headers to reorder; drag edges to resize.");
    fit
}

pub(super) fn column_dropdown(
    ui: &mut egui::Ui,
    columns: &mut crate::settings::TableColumns,
    labels: [&str; 6],
    defaults: crate::settings::TableColumns,
    fit: &dyn Fn(&egui::Ui) -> [u16; 6],
    viewport: egui::Rect,
) {
    // Keep the existing header band and the parent's keyboard focus scope.
    // A child layout here would steal a row or isolate the menu from Tab.
    let rect = egui::Rect::from_min_size(
        egui::pos2(viewport.right() - 96.0, viewport.top()),
        egui::vec2(96.0, theme::HEADER_HEIGHT),
    );
    let response = ui.interact(
        rect,
        ui.scope_id().with("column-dropdown"),
        egui::Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Columns"));
    let visuals = ui.style().interact(&response);
    ui.painter().rect_filled(rect, 0.0, visuals.bg_fill);
    if response.has_focus() || response.hovered() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            visuals.fg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center() - egui::vec2(7.0, 0.0),
        egui::Align2::CENTER_CENTER,
        "Columns",
        egui::TextStyle::Button.resolve(ui.style()),
        visuals.text_color(),
    );
    let c = egui::pos2(rect.right() - 12.0, rect.center().y);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(-3.0, -2.0),
            c + egui::vec2(3.0, -2.0),
            c + egui::vec2(0.0, 2.0),
        ],
        visuals.text_color(),
        egui::Stroke::NONE,
    ));
    egui::Popup::menu(&response).show(|ui| {
        if column_options(ui, columns, labels, defaults) {
            columns.widths = fit(ui);
            ui.close();
        }
    });
}
/// Disjoint edge/body hit regions prevent resize from sorting. A reorder is
/// committed on release against this frame's rectangles, not hover ownership.
pub(super) fn column_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    columns: &mut crate::settings::TableColumns,
    labels: [&str; 6],
    defaults: crate::settings::TableColumns,
    tokens: theme::ColorTokens,
    active: Option<(usize, bool)>,
    fit: &dyn Fn(&egui::Ui) -> [u16; 6],
) -> Option<usize> {
    let old_clip = ui.clip_rect();
    let mut clip = old_clip;
    clip.max.x = (clip.max.x - 100.0).max(clip.min.x);
    ui.set_clip_rect(clip);
    ui.painter().rect_filled(rect, 0.0, tokens.surface);
    let drag_id = ui.scope_id().with("column-reorder");
    let mut clicked = None;
    let mut x = rect.left();
    let mut cells = Vec::new();
    let snapshot = *columns;
    for index in snapshot.order.map(usize::from) {
        if !snapshot.visible[index] {
            continue;
        }
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            egui::vec2(f32::from(snapshot.widths[index]), rect.height()),
        );
        x = cell.right();
        cells.push((index, cell));
        let body =
            egui::Rect::from_min_max(cell.min, egui::pos2(cell.right() - 5.0, cell.bottom()));
        let response = ui.interact(
            body,
            ui.scope_id().with(("column", index)),
            egui::Sense::click_and_drag(),
        );
        let name = if let Some((_, descending)) = active.filter(|(i, _)| *i == index) {
            format!(
                "{}, sorted {}",
                labels[index],
                if descending {
                    "descending"
                } else {
                    "ascending"
                }
            )
        } else {
            labels[index].to_owned()
        };
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        if response.has_focus() || response.hovered() {
            ui.painter().rect_stroke(
                body.shrink(1.0),
                0.0,
                egui::Stroke::new(1.0_f32, tokens.focus),
                egui::StrokeKind::Inside,
            );
        }
        let label = labels[index];
        let galley = egui::WidgetText::from(
            egui::RichText::new(label)
                .small()
                .strong()
                .color(tokens.text),
        )
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (body.width() - 2.0 * CELL_INSET).max(0.0),
            egui::TextStyle::Small,
        );
        let right_aligned = active.is_some() && RIGHT_ALIGNED[index];
        let tx = if right_aligned {
            cell.right() - CELL_INSET - galley.size().x
        } else {
            body.left() + CELL_INSET
        };
        if let Some((_, descending)) = active.filter(|(i, _)| *i == index) {
            let ax = if right_aligned {
                tx - 8.0
            } else {
                tx + galley.size().x + 8.0
            };
            let c = egui::pos2(ax, cell.center().y);
            let (tip, base) = if descending { (3.0, -3.0) } else { (-3.0, 3.0) };
            ui.painter()
                .with_clip_rect(cell.intersect(ui.clip_rect()))
                .add(egui::Shape::convex_polygon(
                    vec![
                        c + egui::vec2(0.0, tip),
                        c + egui::vec2(4.0, base),
                        c + egui::vec2(-4.0, base),
                    ],
                    tokens.text,
                    egui::Stroke::NONE,
                ));
        }
        ui.painter()
            .with_clip_rect(cell.intersect(ui.clip_rect()))
            .galley(
                egui::pos2(tx, body.center().y - galley.size().y / 2.0),
                galley,
                tokens.text,
            );
        if response.clicked() {
            clicked = Some(index);
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            ui.data_mut(|d| d.insert_temp(drag_id, index));
        }
        if response.clicked_by(egui::PointerButton::Middle) {
            columns.widths[index] = fit(ui)[index];
        }
        response.context_menu(|ui| {
            if column_options(ui, columns, labels, defaults) {
                columns.widths = fit(ui);
                ui.close();
            }
        });
        let edge = egui::Rect::from_min_max(egui::pos2(cell.right() - 5.0, cell.top()), cell.max);
        let edge_response = ui
            .interact(
                edge,
                ui.scope_id().with(("column-edge", index)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        let width_id = ui.scope_id().with(("column-start-width", index));
        if edge_response.drag_started() {
            ui.data_mut(|d| d.insert_temp(width_id, f32::from(snapshot.widths[index])));
        }
        if edge_response.dragged() {
            let initial = ui
                .data(|d| d.get_temp::<f32>(width_id))
                .unwrap_or(f32::from(snapshot.widths[index]));
            let delta = ui.input(|i| {
                i.pointer
                    .interact_pos()
                    .zip(i.pointer.press_origin())
                    .map_or(0.0, |(pos, origin)| pos.x - origin.x)
            });
            columns.widths[index] = (initial + delta).round().clamp(24.0, 2000.0) as u16;
        }
        ui.painter().vline(
            cell.right() - 0.5,
            cell.y_range(),
            egui::Stroke::new(1.0_f32, theme::hairline(tokens)),
        );
    }
    if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
        if let Some(source) = ui.data_mut(|d| d.remove_temp::<usize>(drag_id)) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some((target, _)) = cells.iter().find(|(_, cell)| cell.contains(pos)) {
                    let from = columns
                        .order
                        .iter()
                        .position(|&i| usize::from(i) == source)
                        .unwrap();
                    let to = columns
                        .order
                        .iter()
                        .position(|&i| usize::from(i) == *target)
                        .unwrap();
                    if from < to {
                        columns.order[from..=to].rotate_left(1);
                    } else {
                        columns.order[to..=from].rotate_right(1);
                    }
                }
            }
            clicked = None;
        }
    } else if !ui.input(|i| i.pointer.primary_down()) {
        ui.data_mut(|d| d.remove::<usize>(drag_id));
    }
    ui.set_clip_rect(old_clip);
    clicked
}

pub(super) fn fit_column_values(
    ui: &egui::Ui,
    labels: [&str; 6],
    values: impl Iterator<Item = [String; 6]>,
) -> [u16; 6] {
    let mut widths = labels.map(|s| s.len() as f32 * 8.0 + 32.0);
    for row in values {
        for (i, value) in row.into_iter().enumerate() {
            let galley = ui.painter().layout_no_wrap(
                value,
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().text_color(),
            );
            widths[i] = widths[i].max(galley.size().x + 2.0 * CELL_INSET + 40.0);
        }
    }
    widths.map(|w| w.ceil().clamp(24.0, 2000.0) as u16)
}
