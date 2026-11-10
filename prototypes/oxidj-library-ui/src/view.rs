//! The egui view: toolbar, catalog table, decks and the XML preview/edit window.

use super::*;

impl eframe::App for LibraryApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.theme.save(self.custom_tokens, storage);
        self.settings.save(storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.deck_input_hook(ctx, raw_input);
    }
}

/// Fixed track-table columns (W11): availability dot, ID, Duration, Format.
const STATUS_COLUMN: f32 = 24.0;
const ID_COLUMN: f32 = 52.0;
const DURATION_COLUMN: f32 = 72.0;
const FORMAT_COLUMN: f32 = 172.0;
/// Filename and folder share the rest; below this the table scrolls sideways.
const MIN_FLEX_COLUMNS: f32 = 2.0 * 130.0;
/// Track-table columns, left to right.
const COLUMNS: usize = 6;
/// Columns whose cells and header are right-aligned (numbers).
const RIGHT_ALIGNED: [bool; COLUMNS] = [false, true, false, false, true, false];
/// Details panel: fixed key column (W11; 104 px fits "BPM (computed)", B2c).
const DETAILS_LABEL_WIDTH: f32 = 104.0;
/// B2c details facts key for the OxiDJ-computed tempo.
pub(crate) const COMPUTED_BPM_LABEL: &str = "BPM (computed)";
/// Hex digits kept at each end of the middle-truncated SHA-256.
const SHA_EDGE: usize = 12;
/// Horizontal text inset of every table cell.
const CELL_INSET: f32 = 6.0;
/// Width of the "…" popup (label + 240 px field + action button).
const MORE_MENU_WIDTH: f32 = 500.0;
/// Panel shortcut legend shown in the status bar and the toggle tooltips.
/// Narrowest details panel (W9).
pub(crate) const DETAILS_MIN_WIDTH: f32 = 320.0;
/// Temp-data key: the track whose facts grid was laid out last frame.
const TRACK_FACTS_SHOWN: &str = "track-facts-shown";
const PANEL_SHORTCUTS: &str = "Ctrl+B Nav · Ctrl+I Details · Ctrl+D Decks";

impl LibraryApp {
    /// One frame of the whole library UI (the production `update` body):
    /// one-row toolbar, status bar, optional nav/details/decks panels and the
    /// track table filling whatever space the visible panels leave.
    pub(crate) fn ui(&mut self, ctx: &egui::Context) {
        self.handle_settings_shortcuts(ctx);
        self.handle_theme_shortcuts(ctx);
        self.handle_panel_shortcuts(ctx);
        self.handle_deck_keys(ctx);
        self.decks.pump();
        self.sync_artwork_settings();
        self.decks.pump_artwork(ctx);
        self.pump_waveform_job(ctx);
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        self.shell = ShellLayout::default();
        let style = ctx.style();
        let bar = egui::Frame::side_top_panel(&style).inner_margin(theme::BAR_MARGIN);
        let status_frame = egui::Frame::side_top_panel(&style).inner_margin(theme::STATUS_MARGIN);
        let panel = egui::Frame::side_top_panel(&style).inner_margin(theme::PANEL_MARGIN);
        let toolbar = egui::TopBottomPanel::top("toolbar")
            .frame(bar)
            .show(ctx, |ui| self.toolbar(ui, ctx, tokens));
        self.shell.toolbar = Some(toolbar.response.rect);
        // D1: the deck area sits under the toolbar, the library below it.
        if self.show_decks {
            let decks = egui::TopBottomPanel::top("decks")
                .frame(panel)
                .resizable(false)
                .show(ctx, |ui| {
                    // Tall mixer faders must remain reachable on short displays.
                    // The library table keeps `LIBRARY_MIN_H` below the decks.
                    let chrome = self.shell.toolbar.map_or(40.0, |t| t.height()) + 30.0 + 16.0;
                    let max_height =
                        (ctx.screen_rect().height() - chrome - decks::LIBRARY_MIN_H).max(100.0);
                    if decks::area_height(
                        self.settings.arrangement,
                        self.settings.sections,
                        crate::mixer_view::mixer_height(self.settings.mixer),
                        ctx.screen_rect(),
                        ui.available_width(),
                    ) > max_height
                    {
                        egui::ScrollArea::vertical()
                            .id_salt("deck-area-scroll")
                            .scroll_source(egui::scroll_area::ScrollSource {
                                drag: false,
                                ..Default::default()
                            })
                            // Content is taller than `max_height` here, so take
                            // exactly that height from the first frame (an
                            // auto-shrunk panel can stay stuck small).
                            .auto_shrink([false, false])
                            .min_scrolled_height(max_height)
                            .max_height(max_height)
                            .show(ui, |ui| self.deck_area(ui, tokens));
                    } else {
                        self.deck_area(ui, tokens);
                    }
                });
            self.shell.decks = Some(decks.response.rect);
        } else {
            self.decks.layout = Default::default();
        }
        // Settings first, so an option changed this frame is what the
        // panels below render (theme changes apply from the next frame).
        self.settings_window(ctx, tokens);
        waveform_view::publish_smooth(ctx, self.settings.smooth);
        self.track_waveform.style = self.settings.waveform_style;
        self.track_waveform.prefs = track_waveform::WaveformPrefs {
            beat_lines: self.settings.beat_lines,
            zoom_bars: self.settings.zoom_bars,
        };
        self.show_control_gallery_window(ctx, tokens);
        self.show_waveform_demo_window(ctx);
        self.xml_preview_window(ctx, tokens);
        let status = egui::TopBottomPanel::bottom("status-bar")
            .frame(status_frame)
            .show(ctx, |ui| self.status_bar(ui, tokens));
        self.shell.status = Some(status.response.rect);
        if self.show_nav {
            let nav = egui::SidePanel::left("library-navigation")
                .frame(panel)
                .default_width(168.0)
                .width_range(140.0..=280.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("navigation-scroll")
                        .show(ui, |ui| self.navigation(ui, tokens));
                });
            self.shell.nav = Some(nav.response.rect);
        }
        if self.show_details {
            // A bounded width keeps the panel from jumping with its content
            // (long hashes/paths) and leaves the waveform a usable width;
            // at most 40% of a narrow window so the track table keeps room.
            // W9: the minimum is the content's real minimum (the two-line
            // SHA-256 row is ~296 px), so the panel never silently widens.
            let details = egui::SidePanel::right("details")
                .frame(panel)
                .default_width(380.0)
                .min_width(DETAILS_MIN_WIDTH)
                .max_width((ctx.screen_rect().width() * 0.4).clamp(DETAILS_MIN_WIDTH, 480.0))
                .resizable(true)
                .show(ctx, |ui| {
                    // No auto-shrink: the panel keeps its width with or without
                    // a selection instead of jumping to the content width.
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.details(ui, tokens));
                });
            self.shell.details = Some(details.response.rect);
            // The panel's own style picker is the same persisted setting.
            self.settings.waveform_style = self.track_waveform.style;
        } else {
            // Re-opening the panel re-lays the facts grid like a new
            // selection does (W11: the first opened frame showed rows sized
            // from a stale grid until the next input).
            ctx.data_mut(|d| d.remove::<u64>(egui::Id::new(TRACK_FACTS_SHOWN)));
        }
        // No inner margin: the table runs edge to edge between the panels'
        // hairline separators.
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&style).inner_margin(0))
            .show(ctx, |ui| self.track_table(ui, tokens));
        self.finish_track_drop(ctx, tokens);
        // D2: this frame's deck controls go to the audio engine; D4: the
        // mixer is in the path only while visible (else exact unity bypass).
        self.decks.range = crate::decks::tempo_range(self.settings.tempo_range);
        self.decks.crossfader_on = self.settings.crossfader;
        self.decks.curve = self.settings.xfader_curve;
        self.decks
            .flush(self.show_decks && self.settings.sections.mixer);
    }

    /// Resolve the release after every region has been laid out. The source
    /// survives leaving its virtual row; only primary release over a deck loads.
    fn finish_track_drop(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
        use crate::app::TrackSource;
        if self.dragging_track.is_none() {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.dragging_track = None;
            return;
        }
        let layout = self.decks.layout;
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let mut target = None;
        if self.show_decks && !self.settings_ui.open {
            for d in 0..2 {
                let strip = layout.strip.map(|r| {
                    let half = r.width() / 2.0;
                    egui::Rect::from_min_size(
                        egui::pos2(r.left() + d as f32 * half, r.top()),
                        egui::vec2(half, r.height()),
                    )
                });
                for rect in [
                    layout.halves.map(|r| r[d]),
                    layout.overviews.map(|r| r[d]),
                    strip,
                ]
                .into_iter()
                .flatten()
                {
                    if pointer.is_some_and(|p| rect.contains(p)) {
                        target = Some(d);
                        ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Foreground,
                            egui::Id::new("track-drop-highlight"),
                        ))
                        .rect_stroke(
                            rect.shrink(1.0),
                            0.0,
                            egui::Stroke::new(2.0_f32, tokens.focus),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
        }
        ctx.set_cursor_icon(if target.is_some() {
            egui::CursorIcon::Copy
        } else {
            egui::CursorIcon::Grabbing
        });
        if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            let source = self.dragging_track.take();
            if let (Some(source), Some(deck)) = (source, target) {
                match source {
                    TrackSource::Catalog(id) => self.load_deck(deck, id),
                    TrackSource::Imported(member) => self.load_imported_member(member, deck),
                }
            }
        } else if !ctx.input(|i| i.pointer.primary_down()) {
            self.dragging_track = None;
        }
    }

    /// One slim row: title, growing search field, then (right-aligned) the
    /// panel toggles, Settings and the "…" menu with the rarely used
    /// catalog/import/XML fields and tool windows.
    pub(crate) fn toolbar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        tokens: theme::ColorTokens,
    ) {
        if self.show_theme_editor {
            let mut editor_open = self.show_theme_editor;
            egui::Window::new("Custom palette")
                .open(&mut editor_open)
                .show(ctx, |ui| {
                    ui.label("Semantic colors");
                    edit_color(ui, "Background", &mut self.custom_tokens.background);
                    edit_color(ui, "Surface", &mut self.custom_tokens.surface);
                    edit_color(ui, "Text", &mut self.custom_tokens.text);
                    edit_color(ui, "Muted text", &mut self.custom_tokens.muted_text);
                    self.show_palette_fields(ui, tokens.error);
                });
            self.show_theme_editor = editor_open;
        }
        // A fixed one-control-high row: separators and right-to-left layouts
        // fill the available height, which would otherwise grow the panel.
        let row = egui::vec2(ui.available_width(), theme::CONTROL_HEIGHT);
        ui.allocate_ui_with_layout(
            row,
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                // Right-to-left: the first widget added is the rightmost.
                let more = ui
                    .add(
                        egui::Button::new("…")
                            .min_size(egui::vec2(theme::CONTROL_HEIGHT + 8.0, 0.0)),
                    )
                    .on_hover_text("More: catalog, import, Rekordbox XML and tool windows");
                self.shell.more = Some(more.rect);
                // Placed from its fixed width, not from egui's previous-frame size
                // estimate: egui repaints only on input, so a freshly opened popup
                // positioned by pivot stayed half off-screen on the live build.
                let corner = egui::pos2(
                    (more.rect.right() - MORE_MENU_WIDTH).max(0.0),
                    more.rect.bottom() + 2.0,
                );
                let popup = egui::Popup::from_toggle_button_response(&more)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .anchor(corner)
                    .align(egui::RectAlign::BOTTOM_START)
                    .gap(0.0)
                    .width(MORE_MENU_WIDTH)
                    .show(|ui| {
                        // Outer width = MORE_MENU_WIDTH: subtract the frame margin and stroke.
                        let frame = ui.style().spacing.menu_margin.sum().x
                            + 2.0 * ui.visuals().window_stroke.width;
                        ui.set_width(MORE_MENU_WIDTH - frame);
                        self.more_menu(ui, tokens)
                    });
                self.shell.more_popup = popup.map(|popup| popup.response.rect);
                let gear = ui
                    .add(
                        egui::Button::new("⚙")
                            .min_size(egui::vec2(theme::CONTROL_HEIGHT + 8.0, 0.0)),
                    )
                    .on_hover_text("Settings (Ctrl+,)");
                gear.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Settings")
                });
                if gear.clicked() {
                    let open = !self.settings_ui.open;
                    self.set_settings_open(ctx, open);
                }
                self.shell.settings = Some(gear.rect);
                ui.separator();
                // W11: one segmented Nav | Details | Decks group (left to right
                // inside the right-to-left toolbar row).
                let cells = [
                    ("Nav", self.show_nav),
                    ("Details", self.show_details),
                    ("Decks", self.show_decks),
                ];
                let group = crate::components::segmented_toggles(ui, tokens, &cells);
                let mut toggles = [egui::Rect::NOTHING; 3];
                for (index, (response, shortcut)) in group
                    .into_iter()
                    .zip(["Ctrl+B", "Ctrl+I", "Ctrl+D"])
                    .enumerate()
                {
                    let label = cells[index].0;
                    let response = response
                        .on_hover_text(format!("Show or hide the {label} panel ({shortcut})"));
                    if response.clicked() {
                        let open = match index {
                            0 => &mut self.show_nav,
                            1 => &mut self.show_details,
                            _ => &mut self.show_decks,
                        };
                        *open = !*open;
                    }
                    toggles[index] = response.rect;
                }
                self.shell.toggles = Some(toggles);
                ui.separator();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("OxiDJ").strong().color(tokens.text));
                    ui.add_space(4.0);
                    self.search_field(ui, tokens);
                });
            },
        );
    }

    /// The "…" popup: catalog path + reload, import folder, Rekordbox XML
    /// preview, then the tool windows. Actions close the popup.
    fn more_menu(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        let field = 240.0;
        let height = theme::CONTROL_HEIGHT;
        let mut close = false;
        egui::Grid::new("more-menu-fields")
            .num_columns(3)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Catalog");
                let full_path = self.db_path.display().to_string();
                // Left-aligned like the fields (add_sized would centre it).
                ui.allocate_ui_with_layout(
                    egui::vec2(field, height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(field);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&full_path)
                                    .monospace()
                                    .color(tokens.muted_text),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&full_path);
                    },
                );
                if ui.button("Reload").clicked() {
                    self.reload();
                    close = true;
                }
                ui.end_row();
                ui.label("Import folder");
                ui.add_sized(
                    [field, height],
                    egui::TextEdit::singleline(&mut self.import_root)
                        .hint_text("Folder path")
                        .vertical_align(egui::Align::Center),
                );
                let import = ui
                    .add_enabled_ui(self.waveform_job.is_none(), |ui| {
                        (PrimaryActionButton {
                            label: "Import references",
                        })
                        .show(ui, tokens)
                    })
                    .inner;
                if import.clicked() {
                    self.import();
                    ui.ctx().request_repaint();
                    close = true;
                }
                ui.end_row();
                ui.label("Rekordbox XML");
                let source_changed = ui
                    .add_sized(
                        [field, height],
                        egui::TextEdit::singleline(&mut self.xml_path)
                            .hint_text("XML file path (read-only)")
                            .vertical_align(egui::Align::Center),
                    )
                    .changed();
                if source_changed {
                    self.on_xml_source_changed();
                }
                if (PrimaryActionButton {
                    label: "Preview XML",
                })
                .show(ui, tokens)
                .clicked()
                {
                    self.preview_xml();
                    close = true;
                }
                ui.end_row();
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Settings… (Ctrl+,)").clicked() {
                self.set_settings_open(ui.ctx(), true);
                close = true;
            }
            close |= self.show_control_gallery_button(ui).clicked();
            close |= self.show_waveform_demo_button(ui).clicked();
        });
        if close {
            egui::Popup::close_all(ui.ctx());
        }
    }

    /// W11 search: one framed control holding a painted magnifier, the
    /// frameless text field and (when there is text) a clear (x) button.
    /// The field and button are 1 px apart, so their hit rects never touch.
    fn search_field(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        let h = theme::CONTROL_HEIGHT;
        let width = ui.available_width().max(160.0);
        let (outer, _) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::hover());
        let glyph = egui::Rect::from_min_size(outer.min, egui::vec2(h, h));
        let clear =
            egui::Rect::from_min_size(egui::pos2(outer.right() - h, outer.top()), egui::vec2(h, h));
        let field = egui::Rect::from_min_max(
            egui::pos2(glyph.right(), outer.top()),
            egui::pos2(clear.left() - 1.0, outer.bottom()),
        );
        ui.painter()
            .rect_filled(outer, crate::components::CONTROL_RADIUS, tokens.background);
        let edit = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.search)
                .id_salt("track-search")
                .hint_text("Search tracks")
                .frame(false)
                .margin(egui::Margin::symmetric(0, 2))
                .desired_width(field.width())
                .min_size(egui::vec2(0.0, h))
                .vertical_align(egui::Align::Center),
        );
        let stroke = if edit.has_focus() {
            egui::Stroke::new(2.0_f32, tokens.focus)
        } else if edit.hovered() {
            egui::Stroke::new(1.0_f32, tokens.focus)
        } else {
            egui::Stroke::new(1.0_f32, tokens.border)
        };
        let painter = ui.painter();
        painter.rect_stroke(
            outer,
            crate::components::CONTROL_RADIUS,
            stroke,
            egui::StrokeKind::Inside,
        );
        // Magnifier: a 5 px lens and a short handle, painted (no font glyph).
        let lens = glyph.center() - egui::vec2(1.5, 1.5);
        let ink = egui::Stroke::new(1.5_f32, tokens.muted_text);
        painter.circle_stroke(lens, 4.5, ink);
        let d = 4.5 / std::f32::consts::SQRT_2;
        painter.line_segment(
            [lens + egui::vec2(d, d), lens + egui::vec2(d + 3.5, d + 3.5)],
            ink,
        );
        self.shell.search = Some(edit.rect);
        self.shell.search_clear = None;
        if !self.search.is_empty() {
            let response = ui.interact(clear, ui.id().with("clear-search"), egui::Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Clear search")
            });
            let color = if response.hovered() || response.has_focus() {
                tokens.text
            } else {
                tokens.muted_text
            };
            if response.has_focus() {
                ui.painter().rect_stroke(
                    clear.shrink(2.0),
                    crate::components::CONTROL_RADIUS,
                    egui::Stroke::new(2.0_f32, tokens.focus),
                    egui::StrokeKind::Inside,
                );
            }
            let c = clear.center();
            let x = egui::Stroke::new(1.5_f32, color);
            ui.painter()
                .line_segment([c - egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 4.0)], x);
            ui.painter()
                .line_segment([c + egui::vec2(-4.0, 4.0), c + egui::vec2(4.0, -4.0)], x);
            self.shell.search_clear = Some(clear);
            if response.on_hover_text("Clear search").clicked() {
                self.search.clear();
                edit.request_focus();
            }
        }
    }

    /// One 24 px line: the latest notice (or "Ready") in body text on the
    /// left; track count and shortcut legend as small muted hints (W11).
    fn status_bar(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let row = egui::vec2(ui.available_width(), theme::CONTROL_HEIGHT);
        ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let muted = |text: String| egui::RichText::new(text).small().color(tokens.muted_text);
            ui.label(muted(PANEL_SHORTCUTS.into()));
            ui.separator();
            ui.label(muted(format!("{} tracks", self.tracks.len())));
            ui.separator();
            if self.show_decks {
                let (status, ok) = self.decks.status();
                let colour = if ok { tokens.muted_text } else { tokens.warning };
                ui.label(egui::RichText::new(status).small().strong().color(colour))
                    .on_hover_text("Deck audio output (D2): both decks mixed at unity; without a device the decks keep running silently");
                ui.separator();
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if self.notice.is_empty() {
                    ui.label(egui::RichText::new("Ready").color(tokens.text));
                    return;
                }
                let kind = if self.notice.starts_with("Import failed:")
                    || self.notice.starts_with("Track Name export failed:")
                    || self.notice.starts_with("Track Artist export failed:")
                    || self.notice.starts_with("Catalog unavailable:")
                {
                    StatusKind::Error
                } else if self.notice.contains("incomplete") {
                    StatusKind::Warning
                } else {
                    StatusKind::Success
                };
                StatusBadge {
                    label: &self.notice,
                    kind,
                }
                .show_inline(ui, tokens);
            });
        });
    }

    /// The guarded Rekordbox XML preview/edit window (one panel per workflow).
    fn xml_preview_window(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
        if self.show_xml_preview {
            let mut open = self.show_xml_preview;
            egui::Window::new("Rekordbox XML preview · guarded edits")
                .open(&mut open)
                .default_width(720.0)
                .vscroll(true)
                .max_height(ctx.screen_rect().height() - 96.0)
                .show(ctx, |ui| match self.xml_preview.clone() {
                    Some(Err(error)) => {
                        StatusBadge {
                            label: &format!("Preview failed: {error}"),
                            kind: StatusKind::Error,
                        }
                        .show(ui, tokens);
                    }
                    Some(Ok(preview)) => {
                        ui.label(format!(
                            "{} tracks · {} folders · {} playlists",
                            preview.tracks.len(),
                            preview.folders.len(),
                            preview.playlists.len()
                        ));
                        if preview.tracks.is_empty() {
                            ui.label("No tracks in this XML file.");
                        }
                        egui::ScrollArea::vertical()
                            .max_height(260.0)
                            .show(ui, |ui| {
                                egui::Grid::new("rekordbox-preview-tracks")
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.strong("Select");
                                        ui.strong("Track");
                                        ui.strong("Artist");
                                        ui.strong("Raw location");
                                        ui.end_row();
                                        for track in &preview.tracks {
                                            let selected = self.xml_selected_track.as_deref() == Some(&track.id);
                                            if ui.selectable_label(selected, "Select").clicked() {
                                                self.select_xml_track(&track.id, &track.name);
                                            }
                                            ui.label(&track.name);
                                            ui.label(track.artist.as_deref().unwrap_or("—"));
                                            ui.monospace(&track.location);
                                            ui.end_row();
                                        }
                                    });
                            });
                        SectionHeading { title: "Track Name edit" }.show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            ui.label(format!("Selected TrackID: {track_id}"));
                            ui.horizontal(|ui| {
                                ui.label("Proposed Name");
                                text_field(ui, &mut self.xml_edit_name);
                            });
                            if ui.button("Prepare pending edit").clicked() {
                                if let Some(track) = preview.tracks.iter().find(|track| track.id == track_id) {
                                    self.prepare_xml_name_edit(&track.id, &track.name);
                                } else {
                                    self.xml_edit_error = Some("Track Name preparation refused: selected TrackID is no longer in the loaded preview".into());
                                    self.xml_pending_edit = None;
                                }
                            }
                            if let Some(error) = &self.xml_edit_error {
                                StatusBadge { label: error, kind: StatusKind::Error }.show(ui, tokens);
                            }
                            if let Some(pending) = &self.xml_pending_edit {
                                ui.separator();
                                ui.strong("Pending edit · not exported or saved");
                                ui.label(format!("Source: {}", pending.source.display()));
                                ui.label(format!("TrackID: {}", pending.track_id));
                                ui.label(format!("Original Name: {:?}", pending.prepared.change().old_name));
                                ui.label(format!("Proposed Name: {:?}", pending.prepared.change().new_name));
                                ui.horizontal(|ui| {
                                    ui.label("New XML destination");
                                    text_field(ui, &mut self.xml_export_path);
                                });
                                if ui.button("Export prepared edit").clicked() {
                                    self.export_xml_name_edit();
                                }
                                if ui.button("Cancel pending edit").clicked() {
                                    self.cancel_xml_name_edit();
                                }
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Name edit.");
                        }
                        self.track_attribute_panels(ui, tokens, &preview);
                        SectionHeading { title: "Sibling playlist order (provisional)" }.show(ui, tokens);
                        for folder in preview.folders.iter().filter(|folder| folder.path.len() > 1) {
                            let label = format!("Parent #{} · {}", folder.ordinal, folder.path.join(" / "));
                            if ui.selectable_label(self.xml_sibling_parent == Some(folder.ordinal), label).clicked() {
                                self.xml_sibling_parent = Some(folder.ordinal);
                                self.xml_sibling_pending = None;
                                self.xml_sibling_order = preview.playlists.iter().filter(|pl| pl.path.len() >= 2 && pl.path[..pl.path.len()-1] == folder.path[1..]).map(|pl| pl.name.clone()).collect::<Vec<_>>().join(", ");
                            }
                        }
                        ui.horizontal(|ui| { ui.label("Requested direct playlist order (comma-separated names)"); if text_field(ui, &mut self.xml_sibling_order).changed() { self.xml_sibling_pending = None; } });
                        if ui.button("Prepare sibling playlist reorder preview").clicked() { self.prepare_xml_sibling_playlist_reorder(); }
                        if let Some(pending) = &self.xml_sibling_pending {
                            ui.label(format!("Preview · parent #{} {} · {:?} → {:?}", pending.parent_index, pending.parent_name, pending.expected_order, pending.new_order));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_sibling_export_path); });
                            if ui.button("Export prepared sibling order").clicked() { self.export_xml_sibling_playlist_reorder(); }
                            if ui.button("Cancel sibling reorder").clicked() { self.cancel_xml_sibling_playlist_reorder(); }
                        }
                        SectionHeading { title: "Sibling folder order (provisional)" }.show(ui, tokens);
                        for folder in preview.folders.iter().filter(|f| f.path.len() > 1) {
                            let label = format!("Parent #{} · {}", folder.ordinal, folder.path.join(" / "));
                            if ui.selectable_label(self.xml_folder_sibling_parent == Some(folder.ordinal), label).clicked() {
                                self.xml_folder_sibling_parent = Some(folder.ordinal);
                                self.xml_folder_sibling_pending = None;
                                self.xml_folder_sibling_order = preview.folders.iter().filter(|child| child.path.len() > 2 && child.path[..child.path.len()-1] == folder.path).map(|child| child.name.clone()).collect::<Vec<_>>().join(", ");
                            }
                        }
                        ui.horizontal(|ui| { ui.label("Requested direct folder order (comma-separated names)"); if text_field(ui, &mut self.xml_folder_sibling_order).changed() { self.xml_folder_sibling_pending = None; } });
                        if ui.button("Prepare sibling folder reorder preview").clicked() { self.prepare_xml_sibling_folder_reorder(); }
                        if let Some(pending) = &self.xml_folder_sibling_pending {
                            ui.label(format!("Preview · parent #{} {} · {:?} → {:?} (playlist slots retained)", pending.parent_index, pending.parent_name, pending.expected_order, pending.new_order));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_folder_sibling_export_path); });
                            if ui.button("Export prepared sibling folders").clicked() { self.export_xml_sibling_folder_reorder(); }
                            if ui.button("Cancel sibling folder reorder").clicked() { self.cancel_xml_sibling_folder_reorder(); }
                        }
                        SectionHeading { title: "Guarded cross-parent folder-subtree move (provisional)" }.show(ui, tokens);
                        for (index, folder) in preview.folders.iter().enumerate().filter(|(_, f)| f.path.len() > 2) {
                            if ui.selectable_label(self.xml_folder_move_selection == Some(index), format!("Source #{} · {}", folder.ordinal, folder.path.join(" / "))).clicked() {
                                self.xml_folder_move_selection = Some(index);
                                self.xml_pending_folder_move = None;
                            }
                        }
                        for folder in preview.folders.iter().filter(|f| f.path.len() > 1) {
                            if ui.selectable_label(self.xml_folder_move_destination == Some(folder.ordinal), format!("Destination #{} · {}", folder.ordinal, folder.path.join(" / "))).clicked() {
                                self.xml_folder_move_destination = Some(folder.ordinal);
                                self.xml_pending_folder_move = None;
                            }
                        }
                        if let Some(index) = self.xml_folder_move_selection {
                            if ui.button("Prepare folder-subtree move preview").clicked() { self.prepare_xml_folder_move(index); }
                        }
                        if let Some(pending) = &self.xml_pending_folder_move {
                            let count = |value: &Option<String>, delta: isize| value.as_deref().and_then(|v| v.parse::<usize>().ok()).map(|n| if delta < 0 { n.saturating_sub(delta.unsigned_abs()) } else { n + delta as usize }.to_string()).unwrap_or_else(|| if delta < 0 { "N-1".into() } else { "N+1".into() });
                            ui.label(format!("Preview · {} → {} · source Count {} → {}; destination Count {} → {}", pending.old_path.join(" / "), pending.new_path.join(" / "), pending.source_count.as_deref().unwrap_or("?"), count(&pending.source_count, -1), pending.target_count.as_deref().unwrap_or("?"), count(&pending.target_count, 1)));
                            ui.label("Subtree, nested playlists, memberships and unrelated Counts remain unchanged.");
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_folder_move_export_path); });
                            if ui.button("Export prepared folder move").clicked() { self.export_xml_folder_move(); }
                            if ui.button("Cancel folder move").clicked() { self.cancel_xml_folder_move(); }
                        }
                        SectionHeading { title: "Guarded cross-folder playlist move (provisional)" }.show(ui, tokens);
                        for folder in preview.folders.iter().filter(|f| f.path.len() > 1) {
                            if ui.selectable_label(self.xml_move_destination == Some(folder.ordinal), format!("Destination #{} · {}", folder.ordinal, folder.path.join(" / "))).clicked() {
                                self.xml_move_destination = Some(folder.ordinal);
                                self.xml_pending_move = None;
                            }
                        }
                        if let Some(index) = self.xml_selected_playlist {
                            if ui.button("Prepare cross-folder move").clicked() { self.prepare_xml_playlist_move(index); }
                        }
                        if let Some(pending) = &self.xml_pending_move {
                            let old_count = pending.source_count.as_deref().unwrap_or("?");
                            let new_source_count = pending.source_count.as_deref().and_then(|v| v.parse::<usize>().ok()).map(|v| v.saturating_sub(1).to_string()).unwrap_or_else(|| "N-1".into());
                            let new_destination_count = pending.destination_count.as_deref().and_then(|v| v.parse::<usize>().ok()).map(|v| (v+1).to_string()).unwrap_or_else(|| "N+1".into());
                            ui.label(format!("Preview #{} · {} → {} · source Count {old_count} → {new_source_count}; destination Count {} → {new_destination_count}", pending.playlist_index, pending.old_path.join(" / "), pending.new_path.join(" / "), pending.destination_count.as_deref().unwrap_or("?")));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_move_export_path); });
                            if ui.button("Export prepared playlist move").clicked() { self.export_xml_playlist_move(); }
                            if ui.button("Cancel playlist move").clicked() { self.cancel_xml_playlist_move(); }
                        }
                        SectionHeading { title: "Playlist-member reorder (provisional)" }.show(ui, tokens);
                        for (index, playlist) in preview.playlists.iter().enumerate() {
                            let label = format!("#{index} · {} · {}", playlist.path.join(" / "), playlist.name);
                            if ui.selectable_label(self.xml_selected_playlist == Some(index), label).clicked() {
                                self.select_xml_playlist(index);
                            }
                        }
                        if let Some(index) = self.xml_selected_playlist {
                            if let Some(playlist) = preview.playlists.get(index) {
                                ui.label(format!("Selected playlist #{index}: {} / {}", playlist.path.join(" / "), playlist.name));
                                let changed = text_field(ui, &mut self.xml_playlist_rename_name).changed();
                                if changed { self.xml_pending_rename = None; }
                                if ui.button("Prepare playlist rename").clicked() { self.prepare_xml_playlist_rename(index, &playlist.name); }
                                if ui.button("Prepare delete playlist node").clicked() { self.prepare_xml_playlist_delete(index); }
                                for (member_index, track_id) in playlist.tracks.iter().enumerate() {
                                    if ui.selectable_label(self.xml_selected_member == Some(member_index), format!("{member_index}: TrackID {track_id}")).clicked() {
                                        self.select_xml_member(member_index);
                                    }
                                }
                                if let Some(member_index) = self.xml_selected_member {
                                    ui.horizontal(|ui| {
                                        if ui.add_enabled(member_index > 0, egui::Button::new("Move up")).clicked() {
                                            self.prepare_xml_member_reorder(index, member_index, -1);
                                        }
                                        if ui.add_enabled(member_index + 1 < playlist.tracks.len(), egui::Button::new("Move down")).clicked() {
                                            self.prepare_xml_member_reorder(index, member_index, 1);
                                        }
                                        if ui.button("Prepare removal").clicked() {
                                            self.prepare_xml_member_delete(index, member_index);
                                        }
                                    });
                                }
                            }
                        }
                        if let Some(index) = self.xml_selected_playlist {
                            if let Some(playlist) = preview.playlists.get(index) {
                                if ui.add_enabled(!playlist.tracks.is_empty(), egui::Button::new("Prepare clear membership")).clicked() {
                                    self.prepare_xml_playlist_clear(index);
                                }
                            }
                        }
                        if let Some(pending) = &self.xml_pending_playlist_delete {
                            ui.separator();
                            ui.strong("Pending whole-playlist deletion · not exported or saved");
                            ui.label(format!("Source: {} · playlist #{} · {:?}", pending.source.display(), pending.playlist_index, pending.name));
                            ui.label(format!("Ordered members: {:?}", pending.expected_order));
                            ui.label(format!("Proposed result: remove playlist node {:?} and its memberships", pending.name));
                            if let Some(parent) = self.xml_preview.as_ref()
                                .and_then(|p| p.as_ref().ok())
                                .and_then(|p| p.playlists.get(pending.playlist_index)
                                    .and_then(|playlist| immediate_parent_folder(p, playlist))) {
                                if let Some(count) = parent.count.as_deref().and_then(|value| value.parse::<usize>().ok()) {
                                    ui.label(format!("Immediate parent {:?} Count: {count} → {}", parent.name, count.saturating_sub(1)));
                                }
                            }
                            ui.label("Collection tracks and shared memberships in other playlists stay intact.");
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_playlist_delete_export_path); });
                            if ui.button("Export prepared playlist deletion").clicked() { self.export_xml_playlist_delete(); }
                            if ui.button("Cancel playlist deletion").clicked() { self.cancel_xml_playlist_delete(); }
                        }
                        if let Some(pending) = &self.xml_pending_clear {
                            ui.separator();
                            ui.strong("Pending clear · not exported or saved");
                            ui.label(format!("Source: {} · playlist #{}", pending.source.display(), pending.playlist_index));
                            ui.label(format!("Ordered membership: {:?} → empty", pending.expected_order));
                            ui.label("Collection tracks and other playlists stay unchanged.");
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                text_field(ui, &mut self.xml_clear_export_path);
                            });
                            if ui.button("Export prepared clear").clicked() { self.export_xml_playlist_clear(); }
                            if ui.button("Cancel clear").clicked() { self.cancel_xml_playlist_clear(); }
                        }
                        if let Some(pending) = &self.xml_pending_reorder {
                            ui.separator();
                            ui.label(format!("Pending reorder · playlist #{} · source {}", pending.playlist_index, pending.source.display()));
                            ui.label("Prepared XML bytes are not written until export.");
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                text_field(ui, &mut self.xml_reorder_export_path);
                            });
                            if ui.button("Export prepared reorder").clicked() { self.export_xml_member_reorder(); }
                            if ui.button("Cancel reorder").clicked() { self.xml_pending_reorder = None; }
                        }
                        if let Some(pending) = &self.xml_pending_delete {
                            ui.separator();
                            ui.strong("Pending removal · not exported or saved");
                            ui.label(format!("Source: {}", pending.source.display()));
                            ui.label(format!("Playlist #{} · remove membership {} (TrackID {})", pending.playlist_index, pending.member_index, pending.track_id));
                            ui.label(format!("Prepared output: {} bytes", pending.prepared.bytes().len()));
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                text_field(ui, &mut self.xml_delete_export_path);
                            });
                            if ui.button("Export prepared removal").clicked() { self.export_xml_member_delete(); }
                            if ui.button("Cancel removal").clicked() { self.cancel_xml_member_delete(); }
                        }
                        if let Some(pending) = &self.xml_pending_rename {
                            ui.separator();
                            ui.strong("Pending playlist rename · not exported or saved");
                            ui.label(format!("Source: {} · playlist #{} · {} → {}", pending.source.display(), pending.playlist_index, pending.current_name, pending.name));
                            ui.label(format!("Prepared XML bytes: {}", pending.prepared.bytes().len()));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_rename_export_path); });
                            if ui.button("Export prepared rename").clicked() { self.export_xml_playlist_rename(); }
                            if ui.button("Cancel playlist rename").clicked() { self.cancel_xml_playlist_rename(); }
                        }
                        SectionHeading { title: "Guarded empty-playlist creation (provisional)" }.show(ui, tokens);
                        for (parent_index, parent) in preview.folders.iter().enumerate().filter(|(_, parent)| parent.path.len() > 1) {
                            let label = format!("Parent #{} · {}", parent.ordinal.saturating_sub(1), parent.path.join(" / "));
                            if ui.selectable_label(self.xml_selected_parent == Some(parent_index), label).clicked() {
                                self.xml_selected_parent = Some(parent_index);
                                self.xml_pending_create = None;
                                self.xml_pending_folder_create = None;
                            }
                        }
                        if let Some(parent_index) = self.xml_selected_parent {
                            ui.label(format!("Selected parent #{parent_index}"));
                            let name_changed = text_field(ui, &mut self.xml_create_name).changed();
                            if name_changed { self.xml_pending_create = None; }
                            if ui.button("Prepare empty playlist").clicked() { self.prepare_xml_playlist_create(); }
                        }
                        if let Some(pending) = &self.xml_pending_create {
                            ui.label(format!("Preview: create {:?} under paired parent #{} ({:?}) · {} bytes prepared", pending.name, pending.parent_index, pending.parent_name, pending.prepared.bytes().len()));
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                text_field(ui, &mut self.xml_create_export_path);
                            });
                            if ui.button("Export prepared playlist").clicked() { self.export_xml_playlist_create(); }
                            if ui.button("Cancel playlist creation").clicked() { self.xml_pending_create = None; }
                        }
                        SectionHeading { title: "Guarded empty-folder creation (provisional)" }.show(ui, tokens);
                        if let Some(parent_index) = self.xml_selected_parent {
                            let parent = preview.folders.get(parent_index).filter(|folder| folder.path.len() > 1);
                            if let Some(parent) = parent {
                                let label = format!("Selected paired parent #{} · {}", parent.ordinal.saturating_sub(1), parent.path.join(" / "));
                                ui.label(label);
                                let changed = text_field(ui, &mut self.xml_folder_create_name).changed();
                                if changed { self.xml_pending_folder_create = None; }
                                if ui.button("Prepare empty folder").clicked() { self.prepare_xml_folder_create(); }
                            }
                        } else {
                            ui.label("Select a paired non-root parent above to create one empty child folder.");
                        }
                        if let Some(pending) = &self.xml_pending_folder_create {
                            let new_count = pending.parent_count.as_deref().and_then(|v| v.parse::<u64>().ok()).map(|v| v + 1).map(|v| v.to_string()).unwrap_or_else(|| "N+1".into());
                            let parent_path = preview.folders.get(pending.parent_index).map(|f| f.path.join(" / ")).unwrap_or_default();
                            ui.strong("Pending folder creation · not exported or saved");
                            ui.label(format!("Preview: {} / {} · child Count=0 · parent Count {:?} → {} · {} bytes prepared", parent_path, pending.name, pending.parent_count, new_count, pending.prepared.bytes().len()));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_folder_create_export_path); });
                            if ui.button("Export prepared folder").clicked() { self.export_xml_folder_create(); }
                            if ui.button("Cancel folder creation").clicked() { self.cancel_xml_folder_create(); }
                        }
                        SectionHeading { title: "Guarded folder rename (provisional)" }.show(ui, tokens);
                        for (index, folder) in preview.folders.iter().enumerate() {
                            if ui.selectable_label(self.xml_selected_folder_rename == Some(index), format!("Folder #{} · {}", folder.ordinal, folder.path.join(" / "))).clicked() {
                                self.xml_selected_folder_rename = Some(index);
                                self.xml_folder_rename_name = folder.name.clone();
                                self.xml_pending_folder_rename = None;
                            }
                        }
                        if let Some(index) = self.xml_selected_folder_rename {
                            if let Some(folder) = preview.folders.get(index).filter(|f| f.path.len() > 1) {
                                ui.label(format!("Selected #{} · {} · {:?}", folder.ordinal, folder.path.join(" / "), folder.name));
                                if text_field(ui, &mut self.xml_folder_rename_name).changed() { self.xml_pending_folder_rename = None; }
                                if ui.button("Prepare folder rename").clicked() { self.prepare_xml_folder_rename(); }
                            } else { ui.label("Root cannot be renamed."); }
                        }
                        if let Some(pending) = &self.xml_pending_folder_rename {
                            ui.strong("Pending folder rename · not exported or saved");
                            ui.label(format!("{} · {:?} → {:?}", pending.path.join(" / "), pending.old_name, pending.new_name));
                            for playlist in preview.playlists.iter().filter(|p| p.path.starts_with(&pending.path)) {
                                ui.label(format!("Affected playlist: {}", playlist.path.join(" / ")));
                            }
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_folder_rename_export_path); });
                            if ui.button("Export prepared folder rename").clicked() { self.export_xml_folder_rename(); }
                            if ui.button("Cancel folder rename").clicked() { self.xml_pending_folder_rename = None; }
                        }
                        SectionHeading { title: "Guarded empty-folder deletion (provisional)" }.show(ui, tokens);
                        for (index, folder) in preview.folders.iter().enumerate() {
                            let label = format!("Folder #{} · {} · Count={}", folder.ordinal, folder.path.join(" / "), folder.count.as_deref().unwrap_or("missing"));
                            if ui.selectable_label(self.xml_selected_folder_delete == Some(index), label).clicked() {
                                self.xml_selected_folder_delete = Some(index);
                                self.xml_pending_folder_delete = None;
                            }
                        }
                        if let Some(index) = self.xml_selected_folder_delete {
                            if let Some(folder) = preview.folders.get(index) {
                                ui.label(format!("Selected #{} {:?} · {} · Count={}", folder.ordinal, folder.name, folder.path.join(" / "), folder.count.as_deref().unwrap_or("missing")));
                                if ui.button("Prepare empty-folder deletion").clicked() { self.prepare_xml_folder_delete(); }
                            }
                        }
                        if let Some(pending) = &self.xml_pending_folder_delete {
                            let next_count = pending.parent_count.as_deref().and_then(|n| n.parse::<usize>().ok()).map(|n| n.saturating_sub(1).to_string()).unwrap_or_else(|| "N-1".into());
                            ui.strong("Pending folder deletion · not exported or saved");
                            ui.label(format!("Remove {} · Count=0 · immediate parent {} Count {:?} → {} · {} bytes prepared", pending.path.join(" / "), pending.parent_path.join(" / "), pending.parent_count, next_count, pending.prepared.bytes().len()));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_folder_delete_export_path); });
                            if ui.button("Export prepared folder deletion").clicked() { self.export_xml_folder_delete(); }
                            if ui.button("Cancel folder deletion").clicked() { self.cancel_xml_folder_delete(); }
                        }
                        SectionHeading { title: "Playlist-member append (provisional)" }.show(ui, tokens);
                        if let (Some(playlist_index), Some(track_id)) = (self.xml_selected_playlist, self.xml_selected_track.clone()) {
                            ui.label(format!("Append existing collection TrackID {track_id} to playlist #{playlist_index}"));
                            if ui.button("Prepare guarded append").clicked() {
                                self.prepare_xml_member_append(playlist_index, &track_id);
                            }
                        } else {
                            ui.label("Select one collection track and one playlist to prepare an append.");
                        }
                        if let Some(pending) = &self.xml_pending_append {
                            ui.separator();
                            ui.strong("Pending append · not exported or saved");
                            ui.label(format!("Source: {}", pending.source.display()));
                            ui.label(format!("Playlist #{} · append TrackID {}", pending.playlist_index, pending.track_id));
                            ui.label(format!("Prepared output: {} bytes", pending.prepared.bytes().len()));
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                text_field(ui, &mut self.xml_append_export_path);
                            });
                            if ui.button("Export prepared append").clicked() { self.export_xml_member_append(); }
                            if ui.button("Cancel append").clicked() {
                                self.cancel_xml_member_append();
                            }
                        }
                        SectionHeading { title: "Playlist-member insertion (provisional)" }.show(ui, tokens);
                        if let (Some(playlist_index), Some(track_id)) = (self.xml_selected_playlist, self.xml_selected_track.clone()) {
                            if let Some(playlist) = preview.playlists.get(playlist_index) {
                                ui.label(format!("Insert existing collection TrackID {track_id} into playlist #{playlist_index}"));
                                let max_slot = playlist.tracks.len();
                                let old_slot = self.xml_insert_position;
                                ui.add(egui::DragValue::new(&mut self.xml_insert_position).range(0..=max_slot).prefix("Zero-based slot "));
                                self.xml_insert_position = self.xml_insert_position.min(max_slot);
                                if self.xml_insert_position != old_slot { self.xml_pending_insert = None; }
                                if ui.button("Prepare guarded insertion").clicked() {
                                    self.prepare_xml_member_insert(playlist_index, &track_id, self.xml_insert_position);
                                }
                            }
                        } else {
                            ui.label("Select one collection track and one playlist to prepare an insertion.");
                        }
                        if let Some(pending) = &self.xml_pending_insert {
                            ui.separator();
                            ui.strong("Pending insertion · not exported or saved");
                            ui.label(format!("Source: {}", pending.source.display()));
                            ui.label(format!("Playlist #{} · insert TrackID {} at slot {}", pending.playlist_index, pending.track_id, pending.position));
                            ui.label(format!("Prepared output: {} bytes", pending.prepared.bytes().len()));
                            ui.horizontal(|ui| { ui.label("New XML destination"); text_field(ui, &mut self.xml_insert_export_path); });
                            if ui.button("Export prepared insertion").clicked() { self.export_xml_member_insert(); }
                            if ui.button("Cancel insertion").clicked() { self.cancel_xml_member_insert(); }
                        }
                        SectionHeading { title: "Folders" }.show(ui, tokens);
                        if preview.folders.is_empty() {
                            ui.label("No folders.");
                        }
                        for folder in &preview.folders {
                            ui.label(format!("{} · {}", folder.path.join(" / "), folder.name));
                        }
                        SectionHeading { title: "Playlists" }.show(ui, tokens);
                        if preview.playlists.is_empty() {
                            ui.label("No playlists.");
                        }
                        for playlist in &preview.playlists {
                            ui.label(format!(
                                "{} · {} ({} tracks)",
                                playlist.path.join(" / "),
                                playlist.name,
                                playlist.tracks.len()
                            ));
                        }
                        if !preview.unsupported.is_empty() {
                            StatusBadge {
                                label: "Unsupported content detected:",
                                kind: StatusKind::Warning,
                            }
                            .show(ui, tokens);
                            for warning in &preview.unsupported {
                                StatusBadge {
                                    label: warning,
                                    kind: StatusKind::Warning,
                                }
                                .show(ui, tokens);
                            }
                        }
                    }
                    None => {
                        ui.label("Enter an XML path and select Preview XML.");
                    }
                });
            self.show_xml_preview = open;
        }
    }

    /// Collection and availability filters (W11): one heading style, full
    /// width 24 px rows with the count right-aligned in muted text, and the
    /// selected filter marked like the selected track row (tint + rail).
    fn navigation(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        ui.spacing_mut().item_spacing.y = 0.0;
        SectionHeading {
            title: "Collection",
        }
        .show(ui, tokens);
        ui.add_space(4.0);
        let all = nav_row(
            ui,
            self.availability_filter.is_none() && self.imported_playlist.is_none(),
            "All tracks",
            self.tracks.len(),
            tokens,
        );
        if all.clicked() {
            self.availability_filter = None;
            self.imported_playlist = None;
            self.imported_member = None;
            self.selected_imported = None;
        }
        let available = self
            .tracks
            .iter()
            .filter(|track| track.availability == TrackAvailability::Available)
            .count();
        let missing = self.tracks.len().saturating_sub(available);
        ui.add_space(8.0);
        SectionHeading { title: "Filters" }.show(ui, tokens);
        ui.add_space(4.0);
        for (filter, label, count) in [
            (TrackAvailability::Available, "Available", available),
            (TrackAvailability::Missing, "Missing", missing),
        ] {
            if nav_row(
                ui,
                self.availability_filter == Some(filter),
                label,
                count,
                tokens,
            )
            .clicked()
            {
                self.availability_filter = Some(filter);
                self.imported_playlist = None;
                self.imported_member = None;
                self.selected_imported = None;
            }
        }
        ui.add_space(8.0);
        SectionHeading {
            title: "Imported playlists",
        }
        .show(ui, tokens);
        let snapshot = self.imported_navigation.clone();
        if let Ok(snapshot) = snapshot {
            if snapshot.roots.is_empty() {
                ui.label("No imported playlists");
            }
            fn show_nodes(
                ui: &mut egui::Ui,
                nodes: &[oxidj_library::LibraryNode],
                path: &mut Vec<usize>,
                app: &mut LibraryApp,
            ) {
                for (index, node) in nodes.iter().enumerate() {
                    path.push(index);
                    match node {
                        oxidj_library::LibraryNode::Folder { name, children } => {
                            egui::CollapsingHeader::new(format!("📁 {name}"))
                                .default_open(true)
                                .id_salt(("imported-folder", &*path))
                                .show(ui, |ui| show_nodes(ui, children, path, app));
                        }
                        oxidj_library::LibraryNode::Playlist { name, .. } => {
                            let label = format!("♫ {name}");
                            if ui
                                .selectable_label(
                                    app.imported_playlist.as_ref() == Some(path),
                                    label,
                                )
                                .clicked()
                            {
                                app.imported_playlist = Some(path.clone());
                                app.imported_member = None;
                                app.selected_imported = None;
                                app.selected = None;
                            }
                        }
                    }
                    path.pop();
                }
            }
            show_nodes(ui, &snapshot.roots, &mut Vec::new(), self);
        } else if let Err(error) = snapshot {
            ui.label(format!("Imported playlists unavailable: {error}"));
        }
    }

    /// Selected-track facts as an aligned key/value grid, then the waveform.
    fn details(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        SectionHeading {
            title: "Track details",
        }
        .show(ui, tokens);
        if let Some(member) = self.selected_imported {
            if let Ok(snapshot) = &self.imported_navigation {
                if let Some(track) = snapshot.tracks.get(member) {
                    egui::Grid::new("playlist-track-details")
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("Title", track.title.clone()),
                                ("Artist", track.artist.clone().unwrap_or_else(|| "—".into())),
                                ("Album", track.album.clone().unwrap_or_else(|| "—".into())),
                                (
                                    "BPM",
                                    format!("{:.2}", track.tempo_hundredths_bpm as f64 / 100.0),
                                ),
                                (
                                    "Imported key",
                                    track.key.clone().unwrap_or_else(|| "—".into()),
                                ),
                                ("Duration", format_duration(track.duration_seconds as f64)),
                                ("Location", track.location.clone()),
                            ] {
                                ui.label(egui::RichText::new(label).color(tokens.muted_text));
                                ui.add(egui::Label::new(&value).truncate())
                                    .on_hover_text(value);
                                ui.end_row();
                            }
                        });
                    return;
                }
            }
        }
        let Some(id) = self.selected else {
            ui.label(
                egui::RichText::new("Select a track to inspect its details.")
                    .color(tokens.muted_text),
            );
            return;
        };
        let Some(item) = self.tracks.iter().find(|x| x.track.id.as_u64() == id) else {
            ui.label("Selected track is no longer in the catalog.");
            return;
        };
        // The facts grid sizes rows from the previous frame; re-run the
        // frame when the selection changes so it never paints overlapped.
        let shown = egui::Id::new(TRACK_FACTS_SHOWN);
        let previous = ui.ctx().data_mut(|d| {
            let previous = d.get_temp::<u64>(shown);
            d.insert_temp(shown, id);
            previous
        });
        if previous != Some(id) {
            ui.ctx().request_discard("track facts grid layout");
            ui.ctx().request_repaint();
        }
        let unavailable = || "Unavailable".to_owned();
        let hash: String = item
            .track
            .sha256
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let imported_key = match self.catalog.as_ref() {
            Ok(catalog) => {
                let (uri, _) = oxidj_application::location_uri(&item.track.path);
                match catalog.key_for_locations(&[&item.track.path, &uri]) {
                    Ok(key) => key,
                    Err(error) => Some(format!("Unavailable: {error}")),
                }
            }
            Err(_) => Some("Unavailable".to_owned()),
        };
        let facts = [
            ("ID", id.to_string()),
            ("Filename", file_name(&item.track.path)),
            ("Path", item.track.path.clone()),
            (
                "Duration",
                item.duration_seconds
                    .map(|v| format!("{v:.3} s"))
                    .unwrap_or_else(unavailable),
            ),
            (
                "Sample rate",
                item.sample_rate
                    .map(|v| format!("{v} Hz"))
                    .unwrap_or_else(unavailable),
            ),
            (
                "Channels",
                item.channels
                    .map(|v| v.to_string())
                    .unwrap_or_else(unavailable),
            ),
            (
                "Bit depth",
                item.bit_depth
                    .map(|v| format!("{v} bit"))
                    .unwrap_or_else(unavailable),
            ),
            ("Size", format!("{} bytes", item.track.byte_length)),
        ];
        // B2c: computed BPM (OxiDJ analysis), only when a current grid is stored.
        let tempo = self
            .track_waveform
            .computed_tempo(self.catalog.as_ref().ok(), item);
        ui.add_space(4.0);
        egui::Grid::new("track-facts")
            .num_columns(2)
            // W11: a fixed label column, so every value starts at one x.
            .min_col_width(DETAILS_LABEL_WIDTH)
            .max_col_width(ui.available_width() - DETAILS_LABEL_WIDTH - 12.0)
            .min_row_height(ui.text_style_height(&egui::TextStyle::Body))
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                for (key, value) in facts {
                    ui.label(egui::RichText::new(key).color(tokens.muted_text));
                    // Truncated, not wrapped: row heights must not depend
                    // on the (resizable) panel width or rows overlap.
                    ui.add(egui::Label::new(&value).truncate()).on_hover_text(&value);
                    ui.end_row();
                }
                ui.label(egui::RichText::new("Imported key").color(tokens.muted_text));
                let imported_key_value = imported_key.as_deref().unwrap_or("—");
                ui.add(egui::Label::new(imported_key_value).truncate())
                    .on_hover_text(imported_key_value);
                ui.end_row();
                if let Some((bpm, confidence)) = tempo {
                    ui.label(egui::RichText::new(COMPUTED_BPM_LABEL).color(tokens.muted_text));
                    // One body-height row (like Availability), value then badge.
                    // Zero desired width: the row takes only its content's
                    // width, so it never widens the panel past its minimum.
                    let size = egui::vec2(0.0, ui.text_style_height(&egui::TextStyle::Body));
                    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(format!("{bpm:.2}"));
                        if confidence < crate::track_waveform::LOW_CONFIDENCE {
                            StatusBadge {
                                label: "low",
                                kind: StatusKind::Muted,
                            }
                            .show_inline(ui, tokens)
                            .on_hover_text(format!(
                                "Analyser confidence {confidence:.2}: treat this BPM as a rough suggestion"
                            ));
                        }
                    });
                    ui.end_row();
                }
                ui.label(egui::RichText::new("Availability").color(tokens.muted_text));
                let (availability, kind) = availability_status(item.availability);
                StatusBadge {
                    label: availability,
                    kind,
                }
                .show_inline(ui, tokens);
                ui.end_row();
                ui.label(egui::RichText::new("SHA-256").color(tokens.muted_text));
                // W11: one middle-truncated line; full digest on hover,
                // click copies it.
                let response = ui
                    .add(
                        egui::Label::new(egui::RichText::new(truncate_middle(&hash, SHA_EDGE)).monospace())
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text(format!("{hash}\nClick to copy"));
                if response.clicked() {
                    ui.ctx().copy_text(hash.clone());
                    self.notice = "SHA-256 copied to the clipboard".into();
                }
                ui.end_row();
            });
        ui.add_space(8.0);
        self.show_imported_cues(ui, item, tokens);
        ui.add_space(8.0);
        self.track_waveform
            .show(ui, self.catalog.as_ref().ok(), item, tokens);
    }

    /// Display imported cue metadata verbatim from the catalog. Lookup follows
    /// the same path/URI matching used by decks and is read-only.
    fn show_imported_cues(
        &self,
        ui: &mut egui::Ui,
        item: &TrackDetails,
        tokens: theme::ColorTokens,
    ) {
        SectionHeading {
            title: "Imported cues & loops",
        }
        .show(ui, tokens);
        let cues = match self.catalog.as_ref() {
            Ok(catalog) => {
                let (uri, _) = oxidj_application::location_uri(&item.track.path);
                match catalog.cues_for_locations(&[&item.track.path, &uri]) {
                    Ok(cues) => cues,
                    Err(error) => {
                        ui.label(format!("Cue metadata unavailable: {error}"));
                        return;
                    }
                }
            }
            Err(_) => {
                ui.label("Cue metadata unavailable.");
                return;
            }
        };
        if cues.is_empty() {
            ui.label(egui::RichText::new("No imported cues or loops.").color(tokens.muted_text));
            return;
        }
        egui::Grid::new("imported-track-cues")
            .num_columns(4)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Cue");
                ui.strong("Start");
                ui.strong("Loop end");
                ui.strong("Name");
                ui.end_row();
                for cue in cues {
                    ui.label(
                        cue.hot_cue
                            .map(|index| format!("Hot cue {index}"))
                            .unwrap_or_else(|| "Memory cue".into()),
                    );
                    ui.label(format_cue_time(cue.start_ms));
                    ui.label(
                        cue.end_ms
                            .map(format_cue_time)
                            .unwrap_or_else(|| "—".into()),
                    );
                    ui.add(
                        egui::Label::new(if cue.name.is_empty() {
                            "—"
                        } else {
                            &cue.name
                        })
                        .truncate(),
                    );
                    ui.end_row();
                }
            });
    }

    /// The track table: a sortable header (click a column to sort, again to
    /// reverse) above virtualised, full-width 24 px rows, edge to edge.
    fn track_table(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        if let Some(error) = &self.load_error {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(CELL_INSET);
                StatusBadge {
                    label: &format!("Catalog error: {error}"),
                    kind: StatusKind::Error,
                }
                .show(ui, tokens);
            });
            return;
        }
        if let (Some(path), Ok(snapshot)) = (&self.imported_playlist, &self.imported_navigation) {
            let mut nodes = snapshot.roots.as_slice();
            let mut playlist = None;
            for &index in path {
                match nodes.get(index) {
                    Some(oxidj_library::LibraryNode::Folder { children, .. }) => nodes = children,
                    Some(oxidj_library::LibraryNode::Playlist { members, .. }) => {
                        playlist = Some(members.clone());
                        break;
                    }
                    None => break,
                }
            }
            if let Some(members) = playlist {
                let snapshot = snapshot.clone();
                self.shell.table = Some(ui.max_rect());
                let mut load = None;
                let total = ui.available_width().max(720.0);
                let widths = [
                    total * 0.34,
                    total * 0.23,
                    total * 0.13,
                    total * 0.10,
                    total * 0.10,
                    total * 0.10,
                ];
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                egui::ScrollArea::horizontal()
                    .id_salt("playlist-columns")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (header, _) = ui.allocate_exact_size(
                            egui::vec2(total, theme::HEADER_HEIGHT),
                            egui::Sense::hover(),
                        );
                        playlist_cells(
                            ui,
                            header,
                            widths,
                            ["Title", "Artist", "Album", "BPM", "Key", "Duration"]
                                .map(str::to_owned),
                            tokens,
                        );
                        if members.is_empty() {
                            ui.label("This playlist is empty.");
                        }
                        let query = self.search.to_lowercase();
                        let visible: Vec<_> = members
                            .iter()
                            .enumerate()
                            .filter(|(_, member)| {
                                snapshot.tracks.get(**member).is_some_and(|t| {
                                    format!(
                                        "{} {} {}",
                                        t.title,
                                        t.artist.as_deref().unwrap_or(""),
                                        t.album.as_deref().unwrap_or("")
                                    )
                                    .to_lowercase()
                                    .contains(&query)
                                })
                            })
                            .collect();
                        egui::ScrollArea::vertical()
                            .id_salt(("playlist-rows", &self.imported_playlist))
                            .auto_shrink([false, false])
                            .show_rows(ui, theme::ROW_HEIGHT, visible.len(), |ui, range| {
                                for index in range {
                                    let (position, &member) = visible[index];
                                    let track = &snapshot.tracks[member];
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(total, theme::ROW_HEIGHT),
                                        egui::Sense::click_and_drag(),
                                    );
                                    let selected = self.imported_member == Some(position);
                                    response.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::WidgetType::SelectableLabel,
                                            true,
                                            selected,
                                            &track.title,
                                        )
                                    });
                                    if selected || response.hovered() || index % 2 == 1 {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            if selected {
                                                theme::selection_fill(tokens)
                                            } else {
                                                theme::zebra(tokens)
                                            },
                                        );
                                    }
                                    playlist_cells(
                                        ui,
                                        rect,
                                        widths,
                                        [
                                            format!("{} · {}", position + 1, track.title),
                                            track.artist.clone().unwrap_or_else(|| "—".into()),
                                            track.album.clone().unwrap_or_else(|| "—".into()),
                                            if track.tempo_hundredths_bpm == 0 {
                                                "—".into()
                                            } else {
                                                format!(
                                                    "{:.2}",
                                                    track.tempo_hundredths_bpm as f64 / 100.0
                                                )
                                            },
                                            track.key.clone().unwrap_or_else(|| "—".into()),
                                            format_duration(track.duration_seconds as f64),
                                        ],
                                        tokens,
                                    );
                                    if response.clicked() || response.drag_started() {
                                        self.imported_member = Some(position);
                                        self.selected_imported = Some(member);
                                        self.selected = None;
                                    }
                                    if response.drag_started() {
                                        self.dragging_track =
                                            Some(crate::app::TrackSource::Imported(member));
                                    }
                                    response.context_menu(|ui| {
                                        for (deck, letter) in crate::decks::DECKS.iter().enumerate()
                                        {
                                            if ui.button(format!("Load to deck {letter}")).clicked()
                                            {
                                                load = Some((member, deck));
                                                ui.close();
                                            }
                                        }
                                    });
                                }
                            });
                    });
                if let Some((member, deck)) = load {
                    self.load_imported_member(member, deck);
                }
                return;
            }
        }
        let query = self.search.to_lowercase();
        let mut visible: Vec<_> = self
            .tracks
            .iter()
            .filter(|t| {
                query.is_empty()
                    || t.track.path.to_lowercase().contains(&query)
                    || t.track.id.as_u64().to_string().contains(&query)
            })
            .filter(|track| {
                self.availability_filter
                    .is_none_or(|filter| track.availability == filter)
            })
            .collect();
        visible.sort_by(|a, b| {
            let order = match self.sort {
                Sort::Id => a.track.id.as_u64().cmp(&b.track.id.as_u64()),
                Sort::Filename => PathBuf::from(&a.track.path)
                    .file_name()
                    .cmp(&PathBuf::from(&b.track.path).file_name())
                    .then(a.track.id.as_u64().cmp(&b.track.id.as_u64())),
                Sort::Duration => a
                    .duration_seconds
                    .partial_cmp(&b.duration_seconds)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.track.id.as_u64().cmp(&b.track.id.as_u64())),
            };
            if self.descending {
                order.reverse()
            } else {
                order
            }
        });
        let fixed = STATUS_COLUMN + ID_COLUMN + DURATION_COLUMN + FORMAT_COLUMN;
        let total = ui.available_width().max(fixed + MIN_FLEX_COLUMNS);
        let name = ((total - fixed) * 0.6).floor();
        let widths = [
            STATUS_COLUMN,
            ID_COLUMN,
            name,
            total - fixed - name,
            DURATION_COLUMN,
            FORMAT_COLUMN,
        ];
        // The visible table viewport (wider content scrolls inside it).
        self.shell.table = Some(ui.max_rect());
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        // D1: a "Load to deck" choice from a row's context menu.
        let mut load = None;
        // Scrolls sideways only when the window is narrower than the columns.
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (header, _) = ui.allocate_exact_size(egui::vec2(total, theme::HEADER_HEIGHT), egui::Sense::hover());
                table_header(ui, header, widths, tokens, &mut self.sort, &mut self.descending);
                if visible.is_empty() {
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(if self.tracks.is_empty() {
                                "Library is empty. Import a folder of supported audio references (… menu)."
                            } else {
                                "No tracks match this search."
                            })
                            .color(tokens.muted_text),
                        )
                    });
                    return;
                }
                // W11: with few rows the zebra continues faintly below the
                // last row instead of leaving a dead block (no scrolling
                // then, so the stripes line up with the rows).
                let body = ui.available_rect_before_wrap();
                let rows = (body.height() / theme::ROW_HEIGHT).ceil() as usize;
                for index in (visible.len()..rows).filter(|i| i % 2 == 1) {
                    let top = body.top() + index as f32 * theme::ROW_HEIGHT;
                    let stripe = egui::Rect::from_min_size(egui::pos2(body.left(), top), egui::vec2(total, theme::ROW_HEIGHT));
                    ui.painter().rect_filled(stripe.intersect(body), 0.0, theme::zebra(tokens));
                }
                let loaded = self.decks.loaded();
                let badge_colours = [0, 1].map(|d| crate::decks::deck_colours(self.settings.deck_palette, d, tokens));
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(ui, theme::ROW_HEIGHT, visible.len(), |ui, range| {
                        for index in range {
                            let item = visible[index];
                            let id = item.track.id.as_u64();
                            let on = crate::decks::badges(loaded, id);
                            let response = track_row(ui, item, self.selected, index, total, widths, tokens, on, badge_colours);
                            if response.clicked() || response.drag_started() {
                                self.selected = Some(id);
                                self.selected_imported = None;
                            }
                            if response.drag_started() { self.dragging_track = Some(crate::app::TrackSource::Catalog(id)); }
                            response.context_menu(|ui| {
                                for (d, letter) in crate::decks::DECKS.iter().enumerate() {
                                    if ui.button(format!("Load to deck {letter}")).clicked() {
                                        load = Some((d, id));
                                        ui.close();
                                    }
                                }
                            });
                        }
                    });
            });
        if let Some((deck, id)) = load {
            self.selected = Some(id);
            self.load_deck(deck, id);
        }
    }
}

/// Imported metadata uses the same full-width virtual table, without inventing
/// scanned technical facts or changing the source's member order.
fn playlist_cells(
    ui: &egui::Ui,
    rect: egui::Rect,
    widths: [f32; 6],
    cells: [String; 6],
    tokens: theme::ColorTokens,
) {
    let mut x = rect.left();
    for (text, width) in cells.into_iter().zip(widths) {
        let galley = egui::WidgetText::from(egui::RichText::new(text).color(tokens.text))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                width - 2.0 * CELL_INSET,
                egui::TextStyle::Body,
            );
        ui.painter()
            .with_clip_rect(rect.intersect(ui.clip_rect()))
            .galley(
                egui::pos2(x + CELL_INSET, rect.center().y - galley.size().y / 2.0),
                galley,
                tokens.text,
            );
        x += width;
    }
}

/// Column labels; ID, Filename and Duration sort on click (or Enter).
fn table_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    widths: [f32; COLUMNS],
    tokens: theme::ColorTokens,
    current: &mut Sort,
    descending: &mut bool,
) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tokens.surface);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0_f32, theme::hairline(tokens)),
    );
    // W11: the availability column is a dot only (state on row hover);
    // "Folder" replaces "Path", which repeated the filename.
    let columns: [(&str, Option<Sort>); COLUMNS] = [
        ("", None),
        ("ID", Some(Sort::Id)),
        ("Filename", Some(Sort::Filename)),
        ("Folder", None),
        ("Duration", Some(Sort::Duration)),
        ("Format", None),
    ];
    let mut x = rect.left();
    for (column, ((title, sort), width)) in columns.into_iter().zip(widths).enumerate() {
        let right = RIGHT_ALIGNED[column];
        let cell =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
        x += width;
        let active = sort == Some(*current);
        // The default fonts lack ▲/▼, so the sort arrow is painted.
        let name = if active {
            format!(
                "{title}, sorted {}",
                if *descending {
                    "descending"
                } else {
                    "ascending"
                }
            )
        } else {
            title.to_owned()
        };
        let text = title;
        let color = if active {
            tokens.text
        } else {
            tokens.muted_text
        };
        let galley =
            egui::WidgetText::from(egui::RichText::new(text).small().strong().color(color))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    width - 2.0 * CELL_INSET,
                    egui::TextStyle::Small,
                );
        let tx = if right {
            cell.right() - CELL_INSET - galley.size().x
        } else {
            cell.left() + CELL_INSET
        };
        let pos = egui::pos2(tx, cell.center().y - galley.size().y / 2.0);
        if let Some(sort) = sort {
            let response = ui.interact(
                cell,
                ui.id().with(("sort-header", title)),
                egui::Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, &name)
            });
            if response.hovered() || response.has_focus() {
                let stroke: f32 = if response.has_focus() { 2.0 } else { 1.0 };
                ui.painter().rect_stroke(
                    cell.shrink(1.0),
                    2.0,
                    egui::Stroke::new(stroke, tokens.focus),
                    egui::StrokeKind::Inside,
                );
            }
            if response.on_hover_text(format!("Sort by {title}")).clicked() {
                if active {
                    *descending = !*descending;
                } else {
                    *current = sort;
                    *descending = false;
                }
            }
        }
        if active {
            let size = galley.size();
            // Right-aligned titles carry the arrow on their left.
            let ax = if right {
                pos.x - 8.0
            } else {
                pos.x + size.x + 8.0
            };
            let c = egui::pos2(ax, cell.center().y);
            let (tip, base) = if *descending {
                (3.0, -3.0)
            } else {
                (-3.0, 3.0)
            };
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x, c.y + tip),
                    egui::pos2(c.x + 4.0, c.y + base),
                    egui::pos2(c.x - 4.0, c.y + base),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        ui.painter().galley(pos, galley, color);
    }
}

/// One full-width selectable row (true when clicked), W11 states:
/// odd rows a faint zebra, hover the full surface, selected the selection
/// tint plus a 3 px primary rail with every cell in text colour (muted text
/// on the tint would be below 4.5:1 in Tokyo Day), keyboard focus a 2 px
/// focus ring. Missing tracks are dimmed (muted text). Numbers are
/// right-aligned; the availability column is a dot, its word on hover.
fn track_row(
    ui: &mut egui::Ui,
    item: &TrackDetails,
    selected: Option<u64>,
    index: usize,
    total: f32,
    widths: [f32; COLUMNS],
    tokens: theme::ColorTokens,
    on_decks: [bool; 2],
    deck_colours: [crate::decks::DeckColours; 2],
) -> egui::Response {
    let id = item.track.id.as_u64();
    let selected = selected == Some(id);
    let filename = file_name(&item.track.path);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(total, theme::ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &filename)
    });
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, theme::selection_fill(tokens));
        let rail = egui::Rect::from_x_y_ranges(rect.left()..=rect.left() + 3.0, rect.y_range());
        painter.rect_filled(rail, 0.0, tokens.primary);
    } else if response.hovered() {
        painter.rect_filled(rect, 0.0, tokens.surface);
    } else if index % 2 == 1 {
        painter.rect_filled(rect, 0.0, theme::zebra(tokens));
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect.shrink(0.5),
            0.0,
            egui::Stroke::new(2.0_f32, tokens.focus),
            egui::StrokeKind::Inside,
        );
    }
    let (availability, kind) = availability_status(item.availability);
    let missing = item.availability == TrackAvailability::Missing;
    let (primary, secondary) = if selected {
        (tokens.text, tokens.text)
    } else if missing {
        (tokens.muted_text, tokens.muted_text)
    } else {
        (tokens.text, tokens.muted_text)
    };
    let cells = [
        (String::new(), primary, false),
        (id.to_string(), secondary, false),
        (filename, primary, selected),
        (folder_name(&item.track.path), secondary, false),
        (
            item.duration_seconds
                .map(format_duration)
                .unwrap_or_else(|| "—".into()),
            primary,
            false,
        ),
        (
            format_facts(item.sample_rate, item.channels, item.bit_depth),
            secondary,
            false,
        ),
    ];
    let dot = egui::pos2(rect.left() + STATUS_COLUMN / 2.0 + 1.0, rect.center().y);
    ui.painter()
        .circle_filled(dot, 3.5, kind_color(kind, tokens));
    // D1 deck badges at the right end of the Filename cell.
    let badge_w = 18.0;
    let badges = on_decks.iter().filter(|b| **b).count() as f32;
    let mut x = rect.left();
    for (column, ((text, color, strong), width)) in cells.into_iter().zip(widths).enumerate() {
        let width = if column == 2 {
            width - badges * (badge_w + 2.0)
        } else {
            width
        };
        if column == 3 {
            let mut bx = x;
            for (d, on) in on_decks.into_iter().enumerate() {
                if on {
                    let r = egui::Rect::from_min_size(
                        egui::pos2(bx, rect.center().y - 8.0),
                        egui::vec2(badge_w, 16.0),
                    );
                    ui.painter().rect_filled(r, 3.0, deck_colours[d].outer);
                    let g = ui.painter().layout_no_wrap(
                        crate::decks::DECKS[d].into(),
                        egui::FontId::proportional(11.0),
                        deck_colours[d].chip_text,
                    );
                    ui.painter()
                        .galley(r.center() - g.size() / 2.0, g, deck_colours[d].chip_text);
                    bx += badge_w + 2.0;
                }
            }
            x += badges * (badge_w + 2.0);
        }
        if !text.is_empty() {
            let rich = egui::RichText::new(text).color(color);
            let galley = egui::WidgetText::from(if strong { rich.strong() } else { rich })
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    width - 2.0 * CELL_INSET,
                    egui::TextStyle::Body,
                );
            let left = if RIGHT_ALIGNED[column] {
                x + width - CELL_INSET - galley.size().x
            } else {
                x + CELL_INSET
            };
            let pos = egui::pos2(left, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(pos, galley, color);
        }
        x += width;
    }
    let decks: Vec<&str> = on_decks
        .iter()
        .zip(crate::decks::DECKS)
        .filter(|(on, _)| **on)
        .map(|(_, l)| l)
        .collect();
    let on = if decks.is_empty() {
        String::new()
    } else {
        format!(" · on deck {}", decks.join(" and "))
    };
    response.on_hover_text(format!("{availability}{on} · {}", item.track.path))
}

/// Cue positions are source milliseconds; show all stored precision.
fn format_cue_time(milliseconds: u32) -> String {
    format!(
        "{}:{:02}.{:03}",
        milliseconds / 60_000,
        (milliseconds / 1_000) % 60,
        milliseconds % 1_000
    )
}

/// Track length as `m:ss` (rounded to the second), like DJ library tables.
fn format_duration(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// Compact audio facts for the table: "44.1 kHz · 16-bit · mono".
fn format_facts(rate: Option<u32>, channels: Option<u16>, bits: Option<u16>) -> String {
    let (Some(rate), Some(channels), Some(bits)) = (rate, channels, bits) else {
        return "—".into();
    };
    let khz = f64::from(rate) / 1000.0;
    let khz = if rate % 1000 == 0 {
        format!("{khz:.0}")
    } else {
        format!("{khz:.1}")
    };
    let layout = match channels {
        1 => "mono".to_owned(),
        2 => "stereo".to_owned(),
        n => format!("{n} ch"),
    };
    format!("{khz} kHz · {bits}-bit · {layout}")
}

/// The containing folder's name (the table's Folder column).
fn folder_name(path: &str) -> String {
    PathBuf::from(path)
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `first…last` with `edge` characters kept at each end.
pub(crate) fn truncate_middle(text: &str, edge: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 2 * edge + 1 {
        return text.to_owned();
    }
    let head: String = chars[..edge].iter().collect();
    let tail: String = chars[chars.len() - edge..].iter().collect();
    format!("{head}…{tail}")
}

/// A navigation row (W11): label left, count right-aligned in muted text,
/// selected = selection tint + 3 px rail; native selectable widget.
fn nav_row(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    count: usize,
    tokens: theme::ColorTokens,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::ROW_HEIGHT),
        egui::Sense::click(),
    );
    let name = format!("{label} · {count}");
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &name)
    });
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 2.0, theme::selection_fill(tokens));
        let rail = egui::Rect::from_x_y_ranges(rect.left()..=rect.left() + 3.0, rect.y_range());
        painter.rect_filled(rail, 0.0, tokens.primary);
    } else if response.hovered() {
        painter.rect_filled(rect, 2.0, tokens.surface);
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0_f32, tokens.focus),
            egui::StrokeKind::Inside,
        );
    }
    let text = |t: String, color| {
        egui::WidgetText::from(egui::RichText::new(t).color(color)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            rect.width() - 2.0 * NAV_INSET,
            egui::TextStyle::Body,
        )
    };
    let count = text(
        count.to_string(),
        if selected {
            tokens.text
        } else {
            tokens.muted_text
        },
    );
    let label = text(label.to_owned(), tokens.text);
    let y = |g: &std::sync::Arc<egui::Galley>| rect.center().y - g.size().y / 2.0;
    let count_pos = egui::pos2(rect.right() - NAV_INSET - count.size().x, y(&count));
    let label_pos = egui::pos2(rect.left() + NAV_INSET, y(&label));
    ui.painter().galley(label_pos, label, tokens.text);
    ui.painter().galley(count_pos, count, tokens.muted_text);
    response
}

/// Horizontal text inset of navigation rows (8 px grid).
const NAV_INSET: f32 = 8.0;

/// The semantic token behind a status kind.
fn kind_color(kind: StatusKind, tokens: theme::ColorTokens) -> egui::Color32 {
    match kind {
        StatusKind::Success => tokens.success,
        StatusKind::Warning => tokens.warning,
        StatusKind::Error => tokens.error,
        StatusKind::Muted => tokens.muted_text,
    }
}

/// The last path component, for display.
fn file_name(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Badge label and semantic kind for a track's availability.
fn availability_status(availability: TrackAvailability) -> (&'static str, StatusKind) {
    match availability {
        TrackAvailability::Available => ("Available", StatusKind::Success),
        TrackAvailability::Missing => ("Missing", StatusKind::Warning),
    }
}
