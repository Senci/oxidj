use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

use eframe::egui::{self, ScrollArea};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::{Backend, ObjectId},
    protocol::{wl_registry, wl_surface},
};
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};

#[derive(Default)]
struct FeedbackEvents {
    presented: Vec<(u64, u32, u32)>,
    discarded: u64,
    feedbacks: Vec<wp_presentation_feedback::WpPresentationFeedback>,
    manager: Option<wp_presentation::WpPresentation>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for FeedbackEvents {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "wp_presentation" && state.manager.is_none() {
                let manager = registry.bind::<wp_presentation::WpPresentation, _, _>(
                    name,
                    version.min(1),
                    qh,
                    (),
                );
                state.manager = Some(manager);
            }
        }
    }
}

impl Dispatch<wp_presentation::WpPresentation, ()> for FeedbackEvents {
    fn event(
        _: &mut Self,
        _: &wp_presentation::WpPresentation,
        _: wp_presentation::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_presentation_feedback::WpPresentationFeedback, ()> for FeedbackEvents {
    fn event(
        state: &mut Self,
        _: &wp_presentation_feedback::WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                ..
            } => {
                state.presented.push((
                    ((tv_sec_hi as u64) << 32) | tv_sec_lo as u64,
                    tv_nsec,
                    refresh,
                ));
            }
            wp_presentation_feedback::Event::Discarded => {
                state.discarded += 1;
            }
            wp_presentation_feedback::Event::SyncOutput { .. } => {}
            _ => {}
        }
    }
}

struct PresentationObserver {
    _backend: Backend,
    connection: Connection,
    queue: EventQueue<FeedbackEvents>,
    state: FeedbackEvents,
    surface: wl_surface::WlSurface,
    _registry: wl_registry::WlRegistry,
}

impl PresentationObserver {
    fn from_frame(frame: &eframe::Frame) -> Result<Self, String> {
        let display = frame.display_handle().map_err(|e| e.to_string())?;
        let window = frame.window_handle().map_err(|e| e.to_string())?;
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) =
            (display.as_raw(), window.as_raw())
        else {
            return Err("native handles are not Wayland".into());
        };
        // SAFETY: eframe/winit owns this display and keeps it alive for the App lifetime.
        // The backend is a guest and does not close the borrowed display on drop.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend.clone());
        let queue = connection.new_event_queue::<FeedbackEvents>();
        let qh = queue.handle();
        let registry = connection.display().get_registry(&qh, ());
        let id = unsafe {
            ObjectId::from_ptr(
                wl_surface::WlSurface::interface(),
                window.surface.as_ptr().cast(),
            )
        }
        .map_err(|e| format!("borrow eframe wl_surface: {e}"))?;
        let surface = wl_surface::WlSurface::from_id(&connection, id)
            .map_err(|e| format!("wrap eframe wl_surface: {e}"))?;
        let state = FeedbackEvents::default();
        // Guest-mode setup is asynchronous: never read or poll the display shared with eframe.
        connection
            .flush()
            .map_err(|e| format!("flush registry request: {e}"))?;
        Ok(Self {
            _backend: backend,
            connection,
            queue,
            state,
            surface,
            _registry: registry,
        })
    }

    fn drain(&mut self) -> Result<(), String> {
        self.queue
            .dispatch_pending(&mut self.state)
            .map(|_| ())
            .map_err(|e| format!("dispatch presentation feedback: {e}"))
    }

    fn observe(&mut self) -> Result<(), String> {
        self.drain()?;
        if let Some(manager) = &self.state.manager {
            let feedback = manager.feedback(&self.surface, &self.queue.handle(), ());
            self.state.feedbacks.push(feedback);
        }
        self.connection
            .flush()
            .map_err(|e| format!("flush presentation request: {e}"))
    }
}

#[derive(Default)]
struct TableState {
    focused: usize,
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
    has_focus: bool,
    scroll_to_focused: bool,
    widget_focused: Option<usize>,
    last_drop: Option<(usize, usize)>,
    order: Vec<usize>,
    drag_source: Option<usize>,
    drag_active: bool,
}

impl TableState {
    fn ensure_order(&mut self, count: usize) {
        if self.order.len() != count {
            self.order = (0..count).collect();
        }
    }

    fn reorder(&mut self, source: usize, target: usize) {
        if source < self.order.len() && target < self.order.len() && source != target {
            let item = self.order.remove(source);
            self.order.insert(target, item);
        }
    }

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

fn auto_scroll_row(elapsed: Duration, count: usize) -> Option<usize> {
    (count > 0).then(|| (elapsed.as_millis() / 100) as usize % count)
}

fn track_row(ui: &mut egui::Ui, index: usize, table: &mut TableState) -> egui::Rect {
    let item = table.order.get(index).copied().unwrap_or(index);
    let label = format!("Track {item:06} — Synthetic Artist — Synthetic Album");
    let response = ui.add(
        egui::Button::selectable(table.selected.contains(&index), &label)
            .sense(egui::Sense::click_and_drag()),
    );
    if response.clicked() {
        let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
        table.click(index, shift, command);
    }
    if table.has_focus && table.focused == index {
        response.request_focus();
    }
    if response.has_focus() {
        table.widget_focused = Some(index);
    }
    if response.is_pointer_button_down_on() && table.drag_source.is_none() {
        table.drag_source = Some(index);
    }
    if response.dragged() {
        table.drag_active = true;
    }
    response.rect
}

fn finish_row_drag(
    table: &mut TableState,
    rows: &[(usize, egui::Rect)],
    released: bool,
    release_pos: Option<egui::Pos2>,
) {
    if !released {
        return;
    }
    let Some(source) = table.drag_source.take() else {
        table.drag_active = false;
        return;
    };
    let active = std::mem::take(&mut table.drag_active);
    if !active {
        return;
    }
    let Some(target) = release_pos.and_then(|pos| {
        rows.iter()
            .find_map(|(index, rect)| rect.contains(pos).then_some(*index))
    }) else {
        return;
    };
    if source != target {
        table.last_drop = Some((source, target));
        table.reorder(source, target);
    }
}

fn show_tracks(ui: &mut egui::Ui, count: usize, table: &mut TableState) -> f32 {
    table.ensure_order(count);
    const ROW_HEIGHT: f32 = 24.0;
    let row_height_with_spacing = ROW_HEIGHT + ui.spacing().item_spacing.y;
    let scroll_area = ScrollArea::vertical().auto_shrink([false, false]);
    let scroll_area = if table.scroll_to_focused {
        scroll_area.vertical_scroll_offset(table.focused as f32 * row_height_with_spacing)
    } else {
        scroll_area
    };
    let mut row_rects = Vec::new();
    let output = scroll_area.show_rows(ui, ROW_HEIGHT, count, |ui, rows| {
        for index in rows {
            row_rects.push((index, track_row(ui, index, table)));
        }
    });
    let (released, release_pos) = ui.input(|input| {
        (
            input.pointer.button_released(egui::PointerButton::Primary),
            input.pointer.interact_pos(),
        )
    });
    finish_row_drag(table, &row_rects, released, release_pos);
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
    auto_scroll_enabled: bool,
    presentation_observer_enabled: bool,
    started: Instant,
    frames: u64,
    reported: bool,
    keyboard_events: u32,
    table: TableState,
    scroll_offset: f32,
    presentation: Option<PresentationObserver>,
    presentation_error: Option<String>,
}

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.presentation_observer_enabled {
            if self.presentation.is_none() && self.presentation_error.is_none() {
                match PresentationObserver::from_frame(frame) {
                    Ok(observer) => self.presentation = Some(observer),
                    Err(error) => self.presentation_error = Some(error),
                }
            }
            if let Some(observer) = &mut self.presentation {
                let result = if self.started.elapsed() < Duration::from_secs(12) {
                    observer.observe()
                } else {
                    observer.drain()
                };
                if let Err(error) = result {
                    self.presentation_error = Some(error);
                    self.presentation = None;
                }
            }
        }
        if self.auto_scroll_enabled && self.started.elapsed() < Duration::from_secs(12) {
            if let Some(row) = auto_scroll_row(self.started.elapsed(), self.count) {
                self.table.focused = row;
                self.table.has_focus = true;
                self.table.scroll_to_focused = true;
            }
        }
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
            let (
                feedback_presented,
                feedback_unique_timestamps,
                feedback_discarded,
                feedback_refresh_ns,
            ) = self
                .presentation
                .as_ref()
                .map_or((0, 0, 0, None), |observer| {
                    let timestamps = observer
                        .state
                        .presented
                        .iter()
                        .map(|(seconds, nanos, _)| seconds * 1_000_000_000 + u64::from(*nanos))
                        .collect::<BTreeSet<_>>();
                    let refresh = observer
                        .state
                        .presented
                        .last()
                        .map(|(_, _, refresh)| *refresh);
                    (
                        observer.state.presented.len(),
                        timestamps.len(),
                        observer.state.discarded,
                        refresh,
                    )
                });
            eprintln!(
                "probe_finished rows={} auto_scroll_enabled={} presentation_observer_enabled={} update_callbacks={} elapsed_ms={} callback_fps={:.1} keyboard_events={} focused_row={} widget_focused_row={:?} scroll_offset={:.1} selected_rows={:?} last_drop={:?} presentation_window_ms=12000 presentation_feedback_presented={} presentation_feedback_unique_timestamps={} presentation_fps={:.2} presentation_feedback_discarded={} last_refresh_ns={:?} presentation_error={:?}",
                self.count,
                self.auto_scroll_enabled,
                self.presentation_observer_enabled,
                self.frames,
                self.started.elapsed().as_millis(),
                self.frames as f64 / self.started.elapsed().as_secs_f64().max(0.001),
                self.keyboard_events,
                self.table.focused,
                self.table.widget_focused,
                self.scroll_offset,
                self.table.selected.iter().take(8).collect::<Vec<_>>(),
                self.table.last_drop,
                feedback_presented,
                feedback_unique_timestamps,
                feedback_unique_timestamps as f64 / 12.0,
                feedback_discarded,
                feedback_refresh_ns,
                self.presentation_error
            );
        }
        if self.started.elapsed() < Duration::from_secs(15) {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TableState, auto_scroll_row, render_tracks, show_tracks, track_row};
    use eframe::egui::{self, Event, Key, PointerButton, Pos2, RawInput, Rect, Vec2};
    use std::time::Duration;

    #[test]
    fn auto_scroll_advances_every_100ms_and_wraps() {
        assert_eq!(auto_scroll_row(Duration::from_millis(0), 3), Some(0));
        assert_eq!(auto_scroll_row(Duration::from_millis(99), 3), Some(0));
        assert_eq!(auto_scroll_row(Duration::from_millis(100), 3), Some(1));
        assert_eq!(auto_scroll_row(Duration::from_millis(299), 3), Some(2));
        assert_eq!(auto_scroll_row(Duration::from_millis(300), 3), Some(0));
        assert_eq!(auto_scroll_row(Duration::from_millis(0), 0), None);
    }

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
            table.ensure_order(2);
            egui::CentralPanel::default().show(ctx, |ui| {
                rects.clear();
                for index in 0..2 {
                    rects.push(track_row(ui, index, table));
                }
                let (released, release_pos) = ui.input(|input| {
                    (
                        input.pointer.button_released(PointerButton::Primary),
                        input.pointer.interact_pos(),
                    )
                });
                let rows = rects.iter().copied().enumerate().collect::<Vec<_>>();
                super::finish_row_drag(table, &rows, released, release_pos);
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
    fn command_space_toggles_focused_row() {
        let mut table = TableState::default();
        table.click(2, false, false);

        table.key(Key::Space, false, true, 8);
        assert!(table.selected.is_empty());

        table.key(Key::Space, false, true, 8);
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

        table.key(Key::ArrowUp, false, false, 8);
        assert_eq!(table.focused, 6);
        assert_eq!(table.selected.iter().copied().collect::<Vec<_>>(), [6]);

        table.key(Key::Home, false, false, 8);
        assert_eq!(table.focused, 0);
        assert_eq!(table.selected.iter().copied().collect::<Vec<_>>(), [0]);
    }

    #[test]
    fn empty_table_ignores_navigation() {
        let mut table = TableState::default();

        table.key(Key::ArrowDown, false, false, 0);

        assert_eq!(table.focused, 0);
        assert!(table.selected.is_empty());
    }

    #[test]
    fn widget_click_selects_without_reordering() {
        let ctx = egui::Context::default();
        let mut table = TableState::default();
        let mut rects = Vec::new();
        let _ = render_rows(&ctx, &mut table, Vec::new(), &mut rects);
        let point = rects[0].center();
        let _ = render_rows(
            &ctx,
            &mut table,
            vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
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
            vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut rects,
        );
        assert!(table.selected.contains(&0));
        assert_eq!(table.last_drop, None);
        assert_eq!(table.order, vec![0, 1]);
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
        assert_eq!(table.drag_source, Some(0));
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
        assert_eq!(table.order, vec![1, 0]);
        assert_eq!(
            table
                .order
                .iter()
                .map(|item| format!("Track {item:06}"))
                .collect::<Vec<_>>(),
            vec!["Track 000001", "Track 000000"]
        );
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
    let auto_scroll_enabled = std::env::var_os("OXIDJ_PRESENT_SCROLL").is_some();
    let presentation_observer_enabled = !auto_scroll_enabled;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1100.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OxiDJ isolated GUI probe",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.enable_accesskit();
            Ok(Box::new(Probe {
                count,
                auto_scroll_enabled,
                presentation_observer_enabled,
                started: Instant::now(),
                frames: 0,
                reported: false,
                keyboard_events: 0,
                table: TableState::default(),
                scroll_offset: 0.0,
                presentation: None,
                presentation_error: None,
            }))
        }),
    )
}
