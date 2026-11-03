//! The egui view: toolbar, catalog table, decks and the XML preview/edit window.

use super::*;

impl eframe::App for LibraryApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.theme.save(self.custom_tokens, storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}


/// Fixed track-table columns: ID, Duration, Format facts, Availability.
const ID_COLUMN: f32 = 56.0;
const DURATION_COLUMN: f32 = 76.0;
const FORMAT_COLUMN: f32 = 170.0;
const AVAILABILITY_COLUMN: f32 = 96.0;
/// Filename and path share the rest; below this the table scrolls sideways.
const MIN_FLEX_COLUMNS: f32 = 2.0 * 130.0;
/// Horizontal text inset of every table cell.
const CELL_INSET: f32 = 6.0;
/// Width of the "…" popup (label + 240 px field + action button).
const MORE_MENU_WIDTH: f32 = 500.0;
/// Panel shortcut legend shown in the status bar and the toggle tooltips.
/// Deck strip (W9): truncated track-name column and one deck's wrap width
/// (label + name + Play/Cue + spacing).
pub(crate) const DECK_NAME_WIDTH: f32 = 200.0;
const DECK_WIDTH: f32 = DECK_NAME_WIDTH + 170.0;
/// Narrowest details panel (W9).
pub(crate) const DETAILS_MIN_WIDTH: f32 = 320.0;
const PANEL_SHORTCUTS: &str = "Ctrl+B Nav · Ctrl+I Details · Ctrl+D Decks";

impl LibraryApp {
    /// One frame of the whole library UI (the production `update` body):
    /// one-row toolbar, status bar, optional nav/details/decks panels and the
    /// track table filling whatever space the visible panels leave.
    pub(crate) fn ui(&mut self, ctx: &egui::Context) {
        self.handle_theme_shortcuts(ctx);
        self.handle_panel_shortcuts(ctx);
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        self.shell = ShellLayout::default();
        let style = ctx.style();
        let bar = egui::Frame::side_top_panel(&style).inner_margin(theme::BAR_MARGIN);
        let panel = egui::Frame::side_top_panel(&style).inner_margin(theme::PANEL_MARGIN);
        let toolbar = egui::TopBottomPanel::top("toolbar")
            .frame(bar)
            .show(ctx, |ui| self.toolbar(ui, ctx, tokens));
        self.shell.toolbar = Some(toolbar.response.rect);
        self.show_control_gallery_window(ctx, tokens);
        self.show_waveform_demo_window(ctx);
        self.xml_preview_window(ctx, tokens);
        let status = egui::TopBottomPanel::bottom("status-bar")
            .frame(bar)
            .show(ctx, |ui| self.status_bar(ui, tokens));
        self.shell.status = Some(status.response.rect);
        if self.show_decks {
            let decks = egui::TopBottomPanel::bottom("demo-decks")
                .frame(panel)
                .resizable(false)
                .show(ctx, |ui| self.demo_decks(ui, tokens));
            self.shell.decks = Some(decks.response.rect);
        }
        if self.show_nav {
            let nav = egui::SidePanel::left("library-navigation")
                .frame(panel)
                .default_width(168.0)
                .width_range(140.0..=280.0)
                .resizable(true)
                .show(ctx, |ui| self.navigation(ui, tokens));
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
        }
        // No inner margin: the table runs edge to edge between the panels'
        // hairline separators.
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&style).inner_margin(0))
            .show(ctx, |ui| self.track_table(ui, tokens));
    }

    /// One slim row: title, growing search field, then (right-aligned) the
    /// panel toggles, theme picker and the "…" menu with the rarely used
    /// catalog/import/XML fields and tool windows.
    pub(crate) fn toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tokens: theme::ColorTokens) {
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
        ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Right-to-left: the first widget added is the rightmost.
            let more = ui
                .add(egui::Button::new("…").min_size(egui::vec2(theme::CONTROL_HEIGHT + 8.0, 0.0)))
                .on_hover_text("More: catalog, import, Rekordbox XML and tool windows");
            self.shell.more = Some(more.rect);
            // Placed from its fixed width, not from egui's previous-frame size
            // estimate: egui repaints only on input, so a freshly opened popup
            // positioned by pivot stayed half off-screen on the live build.
            let corner = egui::pos2((more.rect.right() - MORE_MENU_WIDTH).max(0.0), more.rect.bottom() + 2.0);
            let popup = egui::Popup::from_toggle_button_response(&more)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .anchor(corner)
                .align(egui::RectAlign::BOTTOM_START)
                .gap(0.0)
                .width(MORE_MENU_WIDTH)
                .show(|ui| {
                    // Outer width = MORE_MENU_WIDTH: subtract the frame margin and stroke.
                    let frame = ui.style().spacing.menu_margin.sum().x + 2.0 * ui.visuals().window_stroke.width;
                    ui.set_width(MORE_MENU_WIDTH - frame);
                    self.more_menu(ui, tokens)
                });
            self.shell.more_popup = popup.map(|popup| popup.response.rect);
            self.theme_selection_ui(ui);
            ui.separator();
            let mut toggles = [egui::Rect::NOTHING; 3];
            for (index, label, shortcut) in [
                (2, "Decks", "Ctrl+D"),
                (1, "Details", "Ctrl+I"),
                (0, "Nav", "Ctrl+B"),
            ] {
                let open = match index {
                    0 => &mut self.show_nav,
                    1 => &mut self.show_details,
                    _ => &mut self.show_decks,
                };
                let response = ui
                    .selectable_label(*open, label)
                    .on_hover_text(format!("Show or hide the {label} panel ({shortcut})"));
                if response.clicked() {
                    *open = !*open;
                }
                toggles[index] = response.rect;
            }
            self.shell.toggles = Some(toggles);
            ui.separator();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("OxiDJ").strong().color(tokens.text));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search tracks")
                        .desired_width(ui.available_width().max(120.0))
                        .min_size(egui::vec2(0.0, theme::CONTROL_HEIGHT))
                        .vertical_align(egui::Align::Center),
                );
            });
        });
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
                            egui::Label::new(egui::RichText::new(&full_path).monospace().color(tokens.muted_text))
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
                    egui::TextEdit::singleline(&mut self.import_root).hint_text("Folder path").vertical_align(egui::Align::Center),
                );
                if (PrimaryActionButton { label: "Import references" }).show(ui, tokens).clicked() {
                    self.import();
                    close = true;
                }
                ui.end_row();
                ui.label("Rekordbox XML");
                let source_changed = ui
                    .add_sized(
                        [field, height],
                        egui::TextEdit::singleline(&mut self.xml_path).hint_text("XML file path (read-only)").vertical_align(egui::Align::Center),
                    )
                    .changed();
                if source_changed {
                    self.on_xml_source_changed();
                }
                if (PrimaryActionButton { label: "Preview XML" }).show(ui, tokens).clicked() {
                    self.preview_xml();
                    close = true;
                }
                ui.end_row();
            });
        ui.separator();
        ui.horizontal(|ui| {
            close |= self.show_control_gallery_button(ui).clicked();
            close |= self.show_waveform_demo_button(ui).clicked();
            if self.theme == ThemeMode::Custom && ui.button("Edit palette…").clicked() {
                self.show_theme_editor = true;
                close = true;
            }
        });
        if close {
            egui::Popup::close_all(ui.ctx());
        }
    }

    /// One line: the latest notice (or "Ready"), track count, shortcut legend.
    fn status_bar(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let row = egui::vec2(ui.available_width(), theme::CONTROL_HEIGHT);
        ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let muted = |text: String| egui::RichText::new(text).small().color(tokens.muted_text);
            ui.label(muted(PANEL_SHORTCUTS.into()));
            ui.separator();
            ui.label(muted(format!("{} tracks", self.tracks.len())));
            ui.separator();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if self.notice.is_empty() {
                    ui.label(muted("Ready".into()));
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
                .show(ui, tokens);
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

    /// Collection and availability filters.
    fn navigation(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        SectionHeading {
            title: "Collection",
        }
        .show(ui, tokens);
        if ui
            .selectable_label(
                self.availability_filter.is_none(),
                format!("All tracks · {}", self.tracks.len()),
            )
            .clicked()
        {
            self.availability_filter = None;
        }
        ui.add_space(4.0);
        ui.separator();
        ui.label(egui::RichText::new("LOCAL FILTERS").small().strong().color(tokens.muted_text));
        let available = self
            .tracks
            .iter()
            .filter(|track| track.availability == TrackAvailability::Available)
            .count();
        let missing = self.tracks.len().saturating_sub(available);
        for (filter, label) in [
            (TrackAvailability::Available, format!("Available · {available}")),
            (TrackAvailability::Missing, format!("Missing · {missing}")),
        ] {
            if ui
                .selectable_label(self.availability_filter == Some(filter), label)
                .clicked()
            {
                self.availability_filter = Some(filter);
            }
        }
    }

    /// Demo deck strip (no audio engine is connected).
    fn demo_decks(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        let loaded = self
            .selected
            .and_then(|id| self.tracks.iter().find(|track| track.track.id.as_u64() == id))
            .map(|track| file_name(&track.track.path))
            .unwrap_or_else(|| "No track selected".into());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("LOCAL DEMO · NO AUDIO CONNECTED")
                    .small()
                    .strong()
                    .color(tokens.muted_text),
            );
            for deck in 0..2 {
                // One deck wraps as a unit so Play/Cue never orphan.
                if ui.max_rect().right() - ui.cursor().min.x < DECK_WIDTH {
                    ui.end_row();
                } else {
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    ui.strong(format!("DECK {}", if deck == 0 { "A" } else { "B" }));
                    // W9: a fixed, truncated name column (full name on hover):
                    // a long filename pushed Play/Cue past the window edge.
                    ui.allocate_ui_with_layout(
                        egui::vec2(DECK_NAME_WIDTH, theme::CONTROL_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add(egui::Label::new(&loaded).truncate()).on_hover_text(&loaded);
                        },
                    );
                    // W9: a fixed label (the selected fill shows the state):
                    // "Playing · demo" widened the button and shifted Cue and
                    // the other deck on every toggle.
                    if ui
                        .selectable_label(self.deck_playing[deck], "Play")
                        .on_hover_text("Demo toggle only: no audio is played")
                        .clicked()
                    {
                        self.deck_playing[deck] = !self.deck_playing[deck];
                    }
                    if ui.selectable_label(self.deck_cue[deck], "Cue").clicked() {
                        self.deck_cue[deck] = !self.deck_cue[deck];
                    }
                });
            }
        });
    }

    /// Selected-track facts as an aligned key/value grid, then the waveform.
    fn details(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        SectionHeading {
            title: "Track details",
        }
        .show(ui, tokens);
        let Some(id) = self.selected else {
            ui.label(egui::RichText::new("Select a track to inspect its details.").color(tokens.muted_text));
            return;
        };
        let Some(item) = self.tracks.iter().find(|x| x.track.id.as_u64() == id) else {
            ui.label("Selected track is no longer in the catalog.");
            return;
        };
        // The facts grid sizes rows from the previous frame; re-run the
        // frame when the selection changes so it never paints overlapped.
        let shown = egui::Id::new("track-facts-shown");
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
        let hash: String = item.track.sha256.iter().map(|b| format!("{b:02x}")).collect();
        let facts = [
            ("ID", id.to_string()),
            ("Filename", file_name(&item.track.path)),
            ("Path", item.track.path.clone()),
            ("Duration", item.duration_seconds.map(|v| format!("{v:.3} s")).unwrap_or_else(unavailable)),
            ("Sample rate", item.sample_rate.map(|v| format!("{v} Hz")).unwrap_or_else(unavailable)),
            ("Channels", item.channels.map(|v| v.to_string()).unwrap_or_else(unavailable)),
            ("Bit depth", item.bit_depth.map(|v| format!("{v} bit")).unwrap_or_else(unavailable)),
            ("Size", format!("{} bytes", item.track.byte_length)),
        ];
        egui::Grid::new("track-facts")
            .num_columns(2)
            .max_col_width(ui.available_width() - 110.0)
            .min_row_height(ui.text_style_height(&egui::TextStyle::Body))
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                for (key, value) in facts {
                    ui.label(egui::RichText::new(key).color(tokens.muted_text));
                    // Truncated, not wrapped: row heights must not depend
                    // on the (resizable) panel width or rows overlap.
                    ui.add(egui::Label::new(&value).truncate()).on_hover_text(&value);
                    ui.end_row();
                }
                ui.label(egui::RichText::new("Availability").color(tokens.muted_text));
                let (availability, kind) = availability_status(item.availability);
                StatusBadge {
                    label: availability,
                    kind,
                }
                .show(ui, tokens);
                ui.end_row();
                ui.label(egui::RichText::new("SHA-256").color(tokens.muted_text));
                // Two fixed 32-hex-digit lines: same height at every width.
                let (first, second) = hash.split_at(hash.len() / 2);
                ui.add(egui::Label::new(egui::RichText::new(format!("{first}\n{second}")).monospace().small()).extend());
                ui.end_row();
            });
        ui.add_space(4.0);
        self.track_waveform
            .show(ui, self.catalog.as_ref().ok(), item, tokens);
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
        let fixed = ID_COLUMN + DURATION_COLUMN + FORMAT_COLUMN + AVAILABILITY_COLUMN;
        let total = ui.available_width().max(fixed + MIN_FLEX_COLUMNS);
        let name = ((total - fixed) * 0.45).floor();
        let widths = [ID_COLUMN, name, total - fixed - name, DURATION_COLUMN, FORMAT_COLUMN, AVAILABILITY_COLUMN];
        // The visible table viewport (wider content scrolls inside it).
        self.shell.table = Some(ui.max_rect());
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
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
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(ui, theme::ROW_HEIGHT, visible.len(), |ui, range| {
                        for index in range {
                            let item = visible[index];
                            if track_row(ui, item, self.selected, index, total, widths, tokens) {
                                self.selected = Some(item.track.id.as_u64());
                            }
                        }
                    });
            });
    }

}

/// Column labels; ID, Filename and Duration sort on click (or Enter).
fn table_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    widths: [f32; 6],
    tokens: theme::ColorTokens,
    current: &mut Sort,
    descending: &mut bool,
) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, tokens.surface);
        painter.hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0_f32, tokens.border));
        let columns: [(&str, Option<Sort>); 6] = [
            ("ID", Some(Sort::Id)),
            ("Filename", Some(Sort::Filename)),
            ("Path", None),
            ("Duration", Some(Sort::Duration)),
            ("Format", None),
            ("Availability", None),
        ];
        let mut x = rect.left();
        for ((title, sort), width) in columns.into_iter().zip(widths) {
            let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
            x += width;
            let active = sort == Some(*current);
            // The default fonts lack ▲/▼, so the sort arrow is painted.
            let name = if active {
                format!("{title}, sorted {}", if *descending { "descending" } else { "ascending" })
            } else {
                title.to_owned()
            };
            let text = title;
            let color = if active { tokens.text } else { tokens.muted_text };
            let galley = egui::WidgetText::from(egui::RichText::new(text).small().strong()).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                width - 2.0 * CELL_INSET,
                egui::TextStyle::Small,
            );
            let pos = egui::pos2(cell.left() + CELL_INSET, cell.center().y - galley.size().y / 2.0);
            if let Some(sort) = sort {
                let response = ui.interact(cell, ui.id().with(("sort-header", title)), egui::Sense::click());
                response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, &name));
                if response.hovered() || response.has_focus() {
                    let stroke: f32 = if response.has_focus() { 2.0 } else { 1.0 };
                    ui.painter().rect_stroke(cell.shrink(1.0), 2.0, egui::Stroke::new(stroke, tokens.focus), egui::StrokeKind::Inside);
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
                let c = egui::pos2(pos.x + size.x + 8.0, cell.center().y);
                let (tip, base) = if *descending { (3.0, -3.0) } else { (-3.0, 3.0) };
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![egui::pos2(c.x, c.y + tip), egui::pos2(c.x + 4.0, c.y + base), egui::pos2(c.x - 4.0, c.y + base)],
                    color,
                    egui::Stroke::NONE,
                ));
            }
            ui.painter().galley(pos, galley, color);
        }
    }

/// One full-width selectable row (true when clicked); the selected row
/// carries a 3 px primary rail (a full-row tint would drop Day muted text
/// below 4.5:1).
fn track_row(
        ui: &mut egui::Ui,
        item: &TrackDetails,
        selected: Option<u64>,
        index: usize,
        total: f32,
        widths: [f32; 6],
        tokens: theme::ColorTokens,
    ) -> bool {
        let id = item.track.id.as_u64();
        let selected = selected == Some(id);
        let filename = file_name(&item.track.path);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(total, theme::ROW_HEIGHT), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &filename));
        if index % 2 == 1 {
            ui.painter().rect_filled(rect, 0.0, tokens.surface);
        }
        if selected {
            let rail = egui::Rect::from_x_y_ranges(rect.left()..=rect.left() + 3.0, rect.y_range());
            ui.painter().rect_filled(rail, 0.0, tokens.primary);
        }
        if response.hovered() || response.has_focus() {
            let stroke: f32 = if response.has_focus() { 2.0 } else { 1.0 };
            ui.painter().rect_stroke(rect.shrink(0.5), 0.0, egui::Stroke::new(stroke, tokens.focus), egui::StrokeKind::Inside);
        }
        let format = match (item.sample_rate, item.channels, item.bit_depth) {
            (Some(r), Some(c), Some(b)) => format!("{r} Hz · {c} ch · {b} bit"),
            _ => "Facts unavailable".into(),
        };
        let duration = item
            .duration_seconds
            .map(|v| format!("{v:.2} s"))
            .unwrap_or_else(|| "—".into());
        let (availability, kind) = availability_status(item.availability);
        let cells = [
            (id.to_string(), tokens.text, false),
            (filename, tokens.text, selected),
            (item.track.path.clone(), tokens.muted_text, false),
            (duration, tokens.text, false),
            (format, tokens.text, false),
            (availability.to_owned(), tokens.text, false),
        ];
        let mut x = rect.left();
        let mut elided_path = false;
        for (column, ((text, color, strong), width)) in cells.into_iter().zip(widths).enumerate() {
            let mut inset = CELL_INSET;
            if column == 5 {
                // Availability: a semantic dot before plain text-coloured words.
                let dot = egui::pos2(x + CELL_INSET + 4.0, rect.center().y);
                ui.painter().circle_filled(dot, 4.0, kind_color(kind, tokens));
                inset += 12.0;
            }
            let rich = egui::RichText::new(text);
            let galley = egui::WidgetText::from(if strong { rich.strong() } else { rich }).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                width - inset - CELL_INSET,
                egui::TextStyle::Body,
            );
            elided_path |= column == 2 && galley.elided;
            let pos = egui::pos2(x + inset, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(pos, galley, color);
            x += width;
        }
        let response = if elided_path {
            response.on_hover_text(&item.track.path)
        } else {
            response
        };
        response.clicked()
}

/// The semantic token behind a status kind.
fn kind_color(kind: StatusKind, tokens: theme::ColorTokens) -> egui::Color32 {
    match kind {
        StatusKind::Success => tokens.success,
        StatusKind::Warning => tokens.warning,
        StatusKind::Error => tokens.error,
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
