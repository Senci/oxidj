use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

use eframe::egui::{self, ScrollArea};

#[derive(Default)]
struct TableState {
    focused: usize,
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
    has_focus: bool,
    scroll_to_focused: bool,
    widget_focused: Option<usize>,
    last_drop: Option<(usize, usize)>,
}

impl TableState {
    fn select_only(&mut self, index: usize) {
        self.selected.clear();
        self.selected.insert(index);
        self.anchor = Some(index);
    }

    fn select_range(&mut self, extend: bool) {
        let anchor = self.anchor.unwrap_or(self.focused);
        if !extend {
            self.selected.clear();
        }
        for index in anchor.min(self.focused)..=anchor.max(self.focused) {
            self.selected.insert(index);
        }
        self.anchor = Some(anchor);
    }

    fn click(&mut self, index: usize, shift: bool, command: bool) {
        self.focused = index;
        self.has_focus = true;
        if shift {
            self.select_range(command);
        } else if command {
            if !self.selected.insert(index) {
                self.selected.remove(&index);
            }
            self.anchor = Some(index);
        } else {
            self.select_only(index);
        }
    }

    fn key(&mut self, key: egui::Key, shift: bool, command: bool, count: usize) {
        if count == 0 {
            return;
        }
        match key {
            egui::Key::ArrowDown => self.focused = (self.focused + 1).min(count - 1),
            egui::Key::ArrowUp => self.focused = self.focused.saturating_sub(1),
            egui::Key::Home => self.focused = 0,
            egui::Key::End => self.focused = count - 1,
            egui::Key::Space if shift => self.select_range(command),
            egui::Key::Space if command => {
                if !self.selected.insert(self.focused) {
                    self.selected.remove(&self.focused);
                }
                self.anchor = Some(self.focused);
            }
            egui::Key::Space => self.select_only(self.focused),
            _ => return,
        }
        if !matches!(key, egui::Key::Space) {
            if shift {
                self.select_range(command);
            } else if !command {
                self.select_only(self.focused);
            }
            self.scroll_to_focused = true;
        }
    }
}

fn track_row(ui: &mut egui::Ui, index: usize, table: &mut TableState) -> egui::Rect {
    let label = format!("Track {index:06} — Synthetic Artist — Synthetic Album");
    let (drop_response, dropped) = ui.dnd_drop_zone::<usize, _>(egui::Frame::default(), |ui| {
        ui.dnd_drag_source(egui::Id::new(("track-drag", index)), index, |ui| {
            let row = ui.selectable_label(table.selected.contains(&index), &label);
            if table.has_focus && table.focused == index {
                row.request_focus();
            }
            if row.has_focus() {
                table.widget_focused = Some(index);
            }
            row.on_hover_text("Synthetic row; drag onto another row to record a drop")
        })
    });
    if drop_response.inner.inner.clicked() {
        let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
        table.click(index, shift, command);
    }
    if drop_response.inner.inner.has_focus() {
        table.focused = index;
        table.has_focus = true;
    }
    if let Some(source) = dropped {
        table.last_drop = Some((*source, index));
        table.focused = index;
        table.has_focus = true;
    }
    drop_response.response.rect
}

fn show_tracks(ui: &mut egui::Ui, count: usize, table: &mut TableState) -> f32 {
    const ROW_HEIGHT: f32 = 24.0;
    let row_height_with_spacing = ROW_HEIGHT + ui.spacing().item_spacing.y;
    let scroll_area = ScrollArea::vertical().auto_shrink([false, false]);
    let scroll_area = if table.scroll_to_focused {
        scroll_area.vertical_scroll_offset(table.focused as f32 * row_height_with_spacing)
    } else {
        scroll_area
    };
    let output = scroll_area.show_rows(ui, ROW_HEIGHT, count, |ui, rows| {
        for index in rows {
            track_row(ui, index, table);
        }
    });
    table.scroll_to_focused = false;
    output.state.offset.y
}

fn render_tracks(ui: &mut egui::Ui, count: usize, table: &mut TableState) -> f32 {
    ui.heading(format!("Synthetic virtualized track list: {count} rows"));
    ui.label(format!(
        "Focused row: {} | selected: {} | last drop: {:?}",
        table.focused,
        table.selected.len(),
        table.last_drop
    ));
    ui.label("Click selects; Up/Down, Home/End navigate; Space selects; Ctrl/Command+Space toggles; Shift+Up/Down extends selection; drag a row onto another to record a drop.");
    show_tracks(ui, count, table)
}

struct Probe {
    count: usize,
    started: Instant,
    frames: u64,
    reported: bool,
    keyboard_events: u32,
    table: TableState,
    scroll_offset: f32,
}

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        let (window_focused, keys) = ctx.input(|i| {
            (
                i.focused,
                i.events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => Some((*key, modifiers.shift, modifiers.command)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            )
        });
        self.table.has_focus = window_focused;
        self.table.widget_focused = None;
        if self.table.has_focus {
            for (key, shift, command) in keys {
                if matches!(
                    key,
                    egui::Key::ArrowDown
                        | egui::Key::ArrowUp
                        | egui::Key::Home
                        | egui::Key::End
                        | egui::Key::Space
                ) {
                    self.keyboard_events += 1;
                    self.table.key(key, shift, command, self.count);
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.scroll_offset = render_tracks(ui, self.count, &mut self.table);
        });
        if !self.reported && self.started.elapsed() >= Duration::from_secs(15) {
            self.reported = true;
            eprintln!(
                "probe_finished rows={} update_callbacks={} elapsed_ms={} callback_fps={:.1} keyboard_events={} focused_row={} widget_focused_row={:?} scroll_offset={:.1} selected_rows={:?} last_drop={:?}",
                self.count,
                self.frames,
                self.started.elapsed().as_millis(),
                self.frames as f64 / self.started.elapsed().as_secs_f64().max(0.001),
                self.keyboard_events,
                self.table.focused,
                self.table.widget_focused,
                self.scroll_offset,
                self.table.selected.iter().take(8).collect::<Vec<_>>(),
                self.table.last_drop
            );
        }
        if self.started.elapsed() < Duration::from_secs(15) {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TableState, render_tracks, show_tracks, track_row};
    use eframe::egui::{self, Event, Key, PointerButton, Pos2, RawInput, Rect, Vec2};

    fn render_rows(
        ctx: &egui::Context,
        table: &mut TableState,
        events: Vec<Event>,
        rects: &mut Vec<Rect>,
    ) -> egui::FullOutput {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            events,
            ..Default::default()
        };
        ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rects.clear();
                for index in 0..2 {
                    rects.push(track_row(ui, index, table));
                }
            });
        })
    }

    #[test]
    fn click_selects_row_and_command_click_toggles_another() {
        let mut table = TableState::default();

        table.click(2, false, false);
        table.click(5, false, true);

        assert_eq!(table.focused, 5);
        assert!(table.has_focus);
        assert_eq!(table.selected.iter().copied().collect::<Vec<_>>(), [2, 5]);

        table.click(5, false, true);
        assert_eq!(table.selected.iter().copied().collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn shift_navigation_selects_a_range_and_home_end_stay_bounded() {
        let mut table = TableState::default();
        table.click(3, false, false);

        table.key(Key::ArrowDown, true, false, 8);
        table.key(Key::ArrowDown, true, false, 8);

        assert_eq!(table.focused, 5);
        assert!(table.scroll_to_focused);
        assert_eq!(
            table.selected.iter().copied().collect::<Vec<_>>(),
            [3, 4, 5]
        );

        table.key(Key::End, false, false, 8);
        table.key(Key::ArrowDown, false, false, 8);
        assert_eq!(table.focused, 7);
        assert!(table.scroll_to_focused);
        assert_eq!(table.selected.iter().copied().collect::<Vec<_>>(), [7]);
    }

    #[test]
    fn empty_table_ignores_navigation() {
        let mut table = TableState::default();

        table.key(Key::ArrowDown, false, false, 0);

        assert_eq!(table.focused, 0);
        assert!(table.selected.is_empty());
    }

    #[test]
    fn drag_source_released_over_another_row_records_drop() {
        let ctx = egui::Context::default();
        let mut table = TableState::default();
        let mut rects = Vec::new();
        let _ = render_rows(&ctx, &mut table, Vec::new(), &mut rects);

        let source = rects[0].center();
        let target = rects[1].center();
        let _ = render_rows(
            &ctx,
            &mut table,
            vec![
                Event::PointerMoved(source),
                Event::PointerButton {
                    pos: source,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut rects,
        );
        let _ = render_rows(
            &ctx,
            &mut table,
            vec![Event::PointerMoved(target)],
            &mut rects,
        );
        let _ = render_rows(
            &ctx,
            &mut table,
            vec![Event::PointerButton {
                pos: target,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut rects,
        );

        assert_eq!(table.last_drop, Some((0, 1)));
    }

    #[test]
    fn accesskit_update_contains_named_track_controls() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut table = TableState::default();
        let mut rects = Vec::new();

        let output = render_rows(&ctx, &mut table, Vec::new(), &mut rects);
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit update should be emitted after enabling it");
        let track_control = update.nodes.iter().find_map(|(_, node)| {
            node.label()
                .filter(|label| label.contains("Track 000000"))
                .map(|label| (node.role(), label.to_owned()))
        });

        assert!(track_control.is_some(), "tree nodes: {:?}", update.nodes);
        let (role, label) = track_control.unwrap();
        assert_eq!(role, egui::accesskit::Role::Button);
        assert!(label.contains("Synthetic Artist"));
    }

    #[test]
    fn offscreen_keyboard_focus_scrolls_and_focuses_the_target_row() {
        let ctx = egui::Context::default();
        let mut table = TableState {
            focused: 199,
            has_focus: true,
            scroll_to_focused: true,
            ..Default::default()
        };
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            ..Default::default()
        };
        let mut scroll_offset = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                scroll_offset = show_tracks(ui, 200, &mut table);
            });
        });

        assert!(scroll_offset > 0.0);
        assert_eq!(table.widget_focused, Some(199));
        assert!(!table.scroll_to_focused);
    }

    #[test]
    fn update_render_records_current_widget_focus_before_reporting() {
        let ctx = egui::Context::default();
        let mut table = TableState {
            focused: 199,
            has_focus: true,
            scroll_to_focused: true,
            widget_focused: Some(0),
            ..Default::default()
        };
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            ..Default::default()
        };
        table.widget_focused = None;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = render_tracks(ui, 200, &mut table);
            });
        });

        assert_eq!(table.widget_focused, Some(199));
    }

    #[test]
    fn last_row_focus_scales_to_200k_and_500k_rows() {
        for count in [200_000, 500_000] {
            let ctx = egui::Context::default();
            let mut table = TableState {
                focused: count - 1,
                has_focus: true,
                scroll_to_focused: true,
                ..Default::default()
            };
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
                ..Default::default()
            };
            let mut scroll_offset = 0.0;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    scroll_offset = render_tracks(ui, count, &mut table);
                });
            });

            assert!(scroll_offset > 0.0, "count={count}");
            assert_eq!(table.widget_focused, Some(count - 1), "count={count}");
        }
    }
}

fn main() -> eframe::Result {
    let count = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1100.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OxiDJ isolated GUI probe",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Probe {
                count,
                started: Instant::now(),
                frames: 0,
                reported: false,
                keyboard_events: 0,
                table: TableState::default(),
                scroll_offset: 0.0,
            }))
        }),
    )
}
