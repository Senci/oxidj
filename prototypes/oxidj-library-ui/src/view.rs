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


/// Width at which the toolbar fields pair up two per row.
const WIDE_TOOLBAR: f32 = 1200.0;

impl LibraryApp {
    /// One frame of the whole library UI (the production `update` body).
    pub(crate) fn ui(&mut self, ctx: &egui::Context) {
        self.handle_theme_shortcuts(ctx);
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui, ctx, tokens));
        self.show_control_gallery_window(ctx, tokens);
        self.show_waveform_demo_window(ctx);
        self.xml_preview_window(ctx, tokens);
        egui::SidePanel::left("library-navigation")
            .default_width(184.0)
            .min_width(160.0)
            .resizable(true)
            .show(ctx, |ui| self.navigation(ui, tokens));
        egui::TopBottomPanel::bottom("demo-decks")
            .resizable(false)
            .show(ctx, |ui| self.demo_decks(ui, tokens));
        // A bounded width keeps the panel from jumping with its content
        // (long hashes/paths) and leaves the waveform a usable width.
        egui::SidePanel::right("details")
            .default_width(420.0)
            .min_width(340.0)
            .max_width(520.0)
            .resizable(true)
            .show(ctx, |ui| self.details(ui, tokens));
        egui::CentralPanel::default().show(ctx, |ui| self.track_table(ui, tokens));
    }

    /// Title row, theme/tool buttons, then an aligned grid of catalog,
    /// search, import and XML fields with their actions; status notice last.
    fn toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tokens: theme::ColorTokens) {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.heading("OxiDJ · Local Library");
            ui.add_space(16.0);
            self.theme_selection_ui(ui);
            self.show_control_gallery_button(ui);
            self.show_waveform_demo_button(ui);
            if self.theme == ThemeMode::Custom && ui.button("Edit palette…").clicked() {
                self.show_theme_editor = true;
            }
        });
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
        let wide = ui.available_width() >= WIDE_TOOLBAR;
        // label + field + action per group; two groups per row when wide.
        let field = if wide {
            ((ui.available_width() - 2.0 * 330.0) / 2.0).clamp(200.0, 420.0)
        } else {
            (ui.available_width() - 350.0).clamp(160.0, 480.0)
        };
        let height = theme::CONTROL_HEIGHT;
        let row_break = |ui: &mut egui::Ui| {
            if !wide {
                ui.end_row();
            }
        };
        ui.add_space(2.0);
        egui::Grid::new("toolbar-fields")
            .num_columns(if wide { 6 } else { 3 })
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Search");
                ui.add_sized(
                    [field, height],
                    egui::TextEdit::singleline(&mut self.search).hint_text("Filter tracks").vertical_align(egui::Align::Center),
                );
                ui.label("");
                row_break(ui);
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
                }
                ui.end_row();
                ui.label("Import folder");
                ui.add_sized(
                    [field, height],
                    egui::TextEdit::singleline(&mut self.import_root).hint_text("Folder path").vertical_align(egui::Align::Center),
                );
                if (PrimaryActionButton {
                    label: "Import references",
                })
                .show(ui, tokens)
                .clicked()
                {
                    self.import();
                }
                row_break(ui);
                ui.label("Rekordbox XML (read-only)");
                let source_changed = ui
                    .add_sized(
                        [field, height],
                        egui::TextEdit::singleline(&mut self.xml_path).hint_text("XML file path").vertical_align(egui::Align::Center),
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
                }
                ui.end_row();
            });
        if !self.notice.is_empty() {
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
        }
        ui.add_space(4.0);
    }

    /// The guarded Rekordbox XML preview/edit window (one panel per workflow).
    fn xml_preview_window(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
        if self.show_xml_preview {
            let mut open = self.show_xml_preview;
            egui::Window::new("Rekordbox XML preview · guarded Name export")
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
                        ui.separator();
                        ui.strong("Prepare one guarded Name edit");
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            ui.label(format!("Selected TrackID: {track_id}"));
                            ui.horizontal(|ui| {
                                ui.label("Proposed Name");
                                ui.text_edit_singleline(&mut self.xml_edit_name);
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
                                    ui.text_edit_singleline(&mut self.xml_export_path);
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
                        ui.separator();
                        SectionHeading { title: "Sibling playlist order (provisional)" }.show(ui, tokens);
                        for folder in preview.folders.iter().filter(|folder| folder.path.len() > 1) {
                            let label = format!("Parent #{} · {}", folder.ordinal, folder.path.join(" / "));
                            if ui.selectable_label(self.xml_sibling_parent == Some(folder.ordinal), label).clicked() {
                                self.xml_sibling_parent = Some(folder.ordinal);
                                self.xml_sibling_pending = None;
                                self.xml_sibling_order = preview.playlists.iter().filter(|pl| pl.path.len() >= 2 && pl.path[..pl.path.len()-1] == folder.path[1..]).map(|pl| pl.name.clone()).collect::<Vec<_>>().join(", ");
                            }
                        }
                        ui.horizontal(|ui| { ui.label("Requested direct playlist order (comma-separated names)"); if ui.text_edit_singleline(&mut self.xml_sibling_order).changed() { self.xml_sibling_pending = None; } });
                        if ui.button("Prepare sibling playlist reorder preview").clicked() { self.prepare_xml_sibling_playlist_reorder(); }
                        if let Some(pending) = &self.xml_sibling_pending {
                            ui.label(format!("Preview · parent #{} {} · {:?} → {:?}", pending.parent_index, pending.parent_name, pending.expected_order, pending.new_order));
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_sibling_export_path); });
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
                        ui.horizontal(|ui| { ui.label("Requested direct folder order (comma-separated names)"); if ui.text_edit_singleline(&mut self.xml_folder_sibling_order).changed() { self.xml_folder_sibling_pending = None; } });
                        if ui.button("Prepare sibling folder reorder preview").clicked() { self.prepare_xml_sibling_folder_reorder(); }
                        if let Some(pending) = &self.xml_folder_sibling_pending {
                            ui.label(format!("Preview · parent #{} {} · {:?} → {:?} (playlist slots retained)", pending.parent_index, pending.parent_name, pending.expected_order, pending.new_order));
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_folder_sibling_export_path); });
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_folder_move_export_path); });
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_move_export_path); });
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
                                let changed = ui.text_edit_singleline(&mut self.xml_playlist_rename_name).changed();
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_playlist_delete_export_path); });
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
                                ui.text_edit_singleline(&mut self.xml_clear_export_path);
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
                                ui.text_edit_singleline(&mut self.xml_reorder_export_path);
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
                                ui.text_edit_singleline(&mut self.xml_delete_export_path);
                            });
                            if ui.button("Export prepared removal").clicked() { self.export_xml_member_delete(); }
                            if ui.button("Cancel removal").clicked() { self.cancel_xml_member_delete(); }
                        }
                        if let Some(pending) = &self.xml_pending_rename {
                            ui.separator();
                            ui.strong("Pending playlist rename · not exported or saved");
                            ui.label(format!("Source: {} · playlist #{} · {} → {}", pending.source.display(), pending.playlist_index, pending.current_name, pending.name));
                            ui.label(format!("Prepared XML bytes: {}", pending.prepared.bytes().len()));
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_rename_export_path); });
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
                            let name_changed = ui.text_edit_singleline(&mut self.xml_create_name).changed();
                            if name_changed { self.xml_pending_create = None; }
                            if ui.button("Prepare empty playlist").clicked() { self.prepare_xml_playlist_create(); }
                        }
                        if let Some(pending) = &self.xml_pending_create {
                            ui.label(format!("Preview: create {:?} under paired parent #{} ({:?}) · {} bytes prepared", pending.name, pending.parent_index, pending.parent_name, pending.prepared.bytes().len()));
                            ui.horizontal(|ui| {
                                ui.label("New XML destination");
                                ui.text_edit_singleline(&mut self.xml_create_export_path);
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
                                let changed = ui.text_edit_singleline(&mut self.xml_folder_create_name).changed();
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_folder_create_export_path); });
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
                                if ui.text_edit_singleline(&mut self.xml_folder_rename_name).changed() { self.xml_pending_folder_rename = None; }
                                if ui.button("Prepare folder rename").clicked() { self.prepare_xml_folder_rename(); }
                            } else { ui.label("Root cannot be renamed."); }
                        }
                        if let Some(pending) = &self.xml_pending_folder_rename {
                            ui.strong("Pending folder rename · not exported or saved");
                            ui.label(format!("{} · {:?} → {:?}", pending.path.join(" / "), pending.old_name, pending.new_name));
                            for playlist in preview.playlists.iter().filter(|p| p.path.starts_with(&pending.path)) {
                                ui.label(format!("Affected playlist: {}", playlist.path.join(" / ")));
                            }
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_folder_rename_export_path); });
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_folder_delete_export_path); });
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
                                ui.text_edit_singleline(&mut self.xml_append_export_path);
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
                            ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_insert_export_path); });
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
                ui.separator();
                ui.strong(format!("DECK {}", if deck == 0 { "A" } else { "B" }));
                ui.label(&loaded);
                if ui
                    .selectable_label(
                        self.deck_playing[deck],
                        if self.deck_playing[deck] {
                            "Playing · demo"
                        } else {
                            "Play"
                        },
                    )
                    .clicked()
                {
                    self.deck_playing[deck] = !self.deck_playing[deck];
                }
                if ui.selectable_label(self.deck_cue[deck], "Cue").clicked() {
                    self.deck_cue[deck] = !self.deck_cue[deck];
                }
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
                    ui.add(egui::Label::new(value).wrap());
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
                ui.add(egui::Label::new(egui::RichText::new(hash).monospace().small()).wrap());
                ui.end_row();
            });
        ui.add_space(4.0);
        self.track_waveform
            .show(ui, self.catalog.as_ref().ok(), item, tokens);
    }

    /// Sort controls and the striped track table (or its empty state).
    fn track_table(&mut self, ui: &mut egui::Ui, tokens: theme::ColorTokens) {
        if let Some(error) = &self.load_error {
            StatusBadge {
                label: &format!("Catalog error: {error}"),
                kind: StatusKind::Error,
            }
            .show(ui, tokens);
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Sort by");
            egui::ComboBox::from_id_salt("sort")
                .selected_text(match self.sort {
                    Sort::Id => "Stable ID",
                    Sort::Filename => "Filename",
                    Sort::Duration => "Duration",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort, Sort::Id, "Stable ID");
                    ui.selectable_value(&mut self.sort, Sort::Filename, "Filename");
                    ui.selectable_value(&mut self.sort, Sort::Duration, "Duration");
                });
            if ui
                .button(if self.descending {
                    "Descending"
                } else {
                    "Ascending"
                })
                .clicked()
            {
                self.descending = !self.descending;
            }
            ui.label(egui::RichText::new(format!("{} tracks", self.tracks.len())).color(tokens.muted_text));
        });
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
        if visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(if self.tracks.is_empty() {
                        "Library is empty. Import a folder of supported audio references."
                    } else {
                        "No tracks match this search."
                    })
                    .color(tokens.muted_text),
                )
            });
            return;
        }
        SectionHeading { title: "Tracks" }.show(ui, tokens);
        // Fixed columns; the name column takes the remaining width and
        // truncates (full text on hover) instead of wrapping per character.
        let name_width = (ui.available_width() - 470.0).clamp(160.0, 640.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tracks")
                .striped(true)
                .min_col_width(48.0)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    let header = |ui: &mut egui::Ui, text: &str| {
                        ui.label(egui::RichText::new(text).small().strong().color(tokens.muted_text));
                    };
                    for text in ["ID", "Filename / path", "Duration", "Format facts", "Availability"] {
                        header(ui, text);
                    }
                    ui.end_row();
                    for item in visible {
                        let id = item.track.id.as_u64();
                        if ui
                            .selectable_label(self.selected == Some(id), id.to_string())
                            .clicked()
                        {
                            self.selected = Some(id);
                        }
                        ui.vertical(|ui| {
                            ui.set_max_width(name_width);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            if ui
                                .selectable_label(self.selected == Some(id), file_name(&item.track.path))
                                .clicked()
                            {
                                self.selected = Some(id);
                            }
                            ui.label(egui::RichText::new(&item.track.path).small().color(tokens.muted_text))
                                .on_hover_text(&item.track.path);
                        });
                        ui.label(
                            item.duration_seconds
                                .map(|v| format!("{v:.2} s"))
                                .unwrap_or_else(|| "—".into()),
                        );
                        ui.label(match (item.sample_rate, item.channels, item.bit_depth) {
                            (Some(r), Some(c), Some(b)) => format!("{r} Hz · {c} ch · {b} bit"),
                            _ => "Facts unavailable".into(),
                        });
                        let (availability, kind) = availability_status(item.availability);
                        StatusBadge {
                            label: availability,
                            kind,
                        }
                        .show(ui, tokens);
                        ui.end_row();
                    }
                });
        });
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
