//! Application state: the `LibraryApp` model, catalog/theme actions, and the
//! XML document selection/source lifecycle that invalidates pending edits.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sort {
    Id,
    Filename,
    Duration,
}

pub(crate) struct LibraryApp {
    pub(crate) db_path: PathBuf,
    pub(crate) catalog: Result<Catalog, String>,
    pub(crate) tracks: Vec<TrackDetails>,
    pub(crate) load_error: Option<String>,
    pub(crate) search: String,
    pub(crate) import_root: String,
    pub(crate) selected: Option<u64>,
    pub(crate) sort: Sort,
    pub(crate) descending: bool,
    pub(crate) notice: String,
    pub(crate) theme: ThemeMode,
    pub(crate) custom_tokens: theme::ColorTokens,
    pub(crate) show_theme_editor: bool,
    pub(crate) primary_hex: String,
    pub(crate) secondary_hex: String,
    pub(crate) accent_error: Option<String>,
    pub(crate) xml_path: String,
    pub(crate) xml_document_source: Option<PathBuf>,
    pub(crate) xml_preview: Option<Result<oxidj_application::Preview, String>>,
    pub(crate) show_xml_preview: bool,
    pub(crate) xml_selected_track: Option<String>,
    pub(crate) xml_edit_name: String,
    pub(crate) xml_pending_edit: Option<PendingTrackNameEdit>,
    pub(crate) xml_edit_artist: String,
    pub(crate) xml_pending_artist_edit: Option<PendingTrackArtistEdit>,
    pub(crate) xml_artist_export_path: String,
    pub(crate) xml_edit_comments: String,
    pub(crate) xml_pending_comments_edit: Option<PendingTrackCommentsEdit>,
    pub(crate) xml_comments_export_path: String,
    pub(crate) xml_edit_album: String,
    pub(crate) xml_pending_album_edit: Option<PendingTrackAlbumEdit>,
    pub(crate) xml_album_export_path: String,
    pub(crate) xml_edit_genre: String,
    pub(crate) xml_pending_genre_edit: Option<PendingTrackGenreEdit>,
    pub(crate) xml_edit_composer: String,
    pub(crate) xml_composer_export_path: String,
    pub(crate) xml_pending_composer_edit: Option<PendingTrackComposerEdit>,
    pub(crate) xml_edit_grouping: String,
    pub(crate) xml_grouping_export_path: String,
    pub(crate) xml_pending_grouping_edit: Option<PendingTrackGroupingEdit>,
    pub(crate) xml_edit_label: String,
    pub(crate) xml_label_export_path: String,
    pub(crate) xml_pending_label_edit: Option<PendingTrackLabelEdit>,
    pub(crate) xml_edit_remixer: String,
    pub(crate) xml_remixer_export_path: String,
    pub(crate) xml_pending_remixer_edit: Option<PendingTrackRemixerEdit>,
    pub(crate) xml_edit_mix: String,
    pub(crate) xml_mix_export_path: String,
    pub(crate) xml_edit_tonality: String,
    pub(crate) xml_tonality_export_path: String,
    pub(crate) xml_pending_tonality_edit: Option<PendingTrackTonalityEdit>,
    pub(crate) xml_edit_kind: String,
    pub(crate) xml_kind_export_path: String,
    pub(crate) xml_pending_kind_edit: Option<PendingTrackKindEdit>,
    pub(crate) xml_pending_mix_edit: Option<PendingTrackMixEdit>,
    pub(crate) xml_genre_export_path: String,
    pub(crate) xml_selected_playlist: Option<usize>,
    pub(crate) xml_selected_member: Option<usize>,
    pub(crate) xml_pending_reorder: Option<PendingPlaylistMemberReorder>,
    pub(crate) xml_reorder_export_path: String,
    pub(crate) xml_sibling_parent: Option<usize>,
    pub(crate) xml_sibling_order: String,
    pub(crate) xml_sibling_pending: Option<PendingSiblingPlaylistReorder>,
    pub(crate) xml_sibling_export_path: String,
    pub(crate) xml_folder_sibling_parent: Option<usize>,
    pub(crate) xml_folder_sibling_order: String,
    pub(crate) xml_folder_sibling_pending: Option<PendingSiblingFolderReorder>,
    pub(crate) xml_folder_sibling_export_path: String,
    pub(crate) xml_move_destination: Option<usize>,
    pub(crate) xml_pending_folder_move: Option<PendingFolderMove>,
    pub(crate) xml_folder_move_selection: Option<usize>,
    pub(crate) xml_folder_move_destination: Option<usize>,
    pub(crate) xml_folder_move_export_path: String,
    pub(crate) xml_pending_move: Option<PendingPlaylistMove>,
    pub(crate) xml_move_export_path: String,
    pub(crate) xml_pending_append: Option<PendingPlaylistMemberAppend>,
    pub(crate) xml_append_export_path: String,
    pub(crate) xml_insert_position: usize,
    pub(crate) xml_pending_insert: Option<PendingPlaylistMemberInsert>,
    pub(crate) xml_pending_clear: Option<PendingPlaylistClear>,
    pub(crate) xml_clear_export_path: String,
    pub(crate) xml_insert_export_path: String,
    pub(crate) xml_pending_playlist_delete: Option<PendingPlaylistDelete>,
    pub(crate) xml_playlist_delete_export_path: String,
    pub(crate) xml_pending_delete: Option<PendingPlaylistMemberDelete>,
    pub(crate) xml_delete_export_path: String,
    pub(crate) xml_selected_parent: Option<usize>,
    pub(crate) xml_create_name: String,
    pub(crate) xml_pending_create: Option<PendingPlaylistCreate>,
    pub(crate) xml_create_export_path: String,
    pub(crate) xml_folder_create_name: String,
    pub(crate) xml_pending_folder_create: Option<PendingFolderCreate>,
    pub(crate) xml_folder_create_export_path: String,
    pub(crate) xml_selected_folder_delete: Option<usize>,
    pub(crate) xml_pending_folder_delete: Option<PendingFolderDelete>,
    pub(crate) xml_folder_delete_export_path: String,
    pub(crate) xml_selected_folder_rename: Option<usize>,
    pub(crate) xml_folder_rename_name: String,
    pub(crate) xml_pending_folder_rename: Option<PendingFolderRename>,
    pub(crate) xml_folder_rename_export_path: String,
    pub(crate) xml_playlist_rename_name: String,
    pub(crate) xml_pending_rename: Option<PendingPlaylistRename>,
    pub(crate) xml_rename_export_path: String,
    pub(crate) xml_export_path: String,
    pub(crate) xml_edit_error: Option<String>,
    pub(crate) show_control_gallery: bool,
    pub(crate) control_gallery: control_gallery::GalleryState,
    pub(crate) show_waveform_demo: bool,
    pub(crate) waveform_demo: Option<waveform_view::DemoState>,
    pub(crate) availability_filter: Option<TrackAvailability>,
    pub(crate) deck_playing: [bool; 2],
    pub(crate) deck_cue: [bool; 2],
}

impl LibraryApp {
    pub(crate) fn new(db_path: PathBuf, theme: ThemeMode, custom_tokens: theme::ColorTokens) -> Self {
        let catalog = Catalog::open(&db_path).map_err(|e| e.to_string());
        let mut app = Self {
            db_path,
            catalog,
            tracks: vec![],
            load_error: None,
            search: String::new(),
            import_root: String::new(),
            selected: None,
            sort: Sort::Id,
            descending: false,
            notice: String::new(),
            theme,
            custom_tokens,
            show_theme_editor: false,
            primary_hex: format!(
                "#{:02X}{:02X}{:02X}",
                custom_tokens.primary.r(),
                custom_tokens.primary.g(),
                custom_tokens.primary.b()
            ),
            secondary_hex: format!(
                "#{:02X}{:02X}{:02X}",
                custom_tokens.secondary.r(),
                custom_tokens.secondary.g(),
                custom_tokens.secondary.b()
            ),
            accent_error: None,
            xml_path: String::new(),
            xml_document_source: None,
            xml_preview: None,
            show_xml_preview: false,
            xml_selected_track: None,
            xml_edit_name: String::new(),
            xml_pending_edit: None,
            xml_edit_artist: String::new(),
            xml_pending_artist_edit: None,
            xml_artist_export_path: String::new(),
            xml_edit_comments: String::new(),
            xml_pending_comments_edit: None,
            xml_comments_export_path: String::new(),
            xml_edit_album: String::new(),
            xml_pending_album_edit: None,
            xml_album_export_path: String::new(),
            xml_edit_genre: String::new(),
            xml_pending_genre_edit: None,
            xml_edit_composer: String::new(),
            xml_composer_export_path: String::new(),
            xml_pending_composer_edit: None,
            xml_edit_grouping: String::new(),
            xml_grouping_export_path: String::new(),
            xml_pending_grouping_edit: None,
            xml_edit_remixer: String::new(),
            xml_remixer_export_path: String::new(),
            xml_pending_remixer_edit: None,
            xml_edit_mix: String::new(),
            xml_mix_export_path: String::new(),
            xml_edit_tonality: String::new(),
            xml_tonality_export_path: String::new(),
            xml_pending_tonality_edit: None,
            xml_edit_kind: String::new(),
            xml_kind_export_path: String::new(),
            xml_pending_kind_edit: None,
            xml_pending_mix_edit: None,
            xml_edit_label: String::new(),
            xml_label_export_path: String::new(),
            xml_pending_label_edit: None,
            xml_genre_export_path: String::new(),
            xml_selected_playlist: None,
            xml_selected_member: None,
            xml_pending_reorder: None,
            xml_reorder_export_path: String::new(),
            xml_sibling_parent: None,
            xml_sibling_order: String::new(),
            xml_sibling_pending: None,
            xml_sibling_export_path: String::new(),
            xml_folder_sibling_parent: None,
            xml_folder_sibling_order: String::new(),
            xml_folder_sibling_pending: None,
            xml_folder_sibling_export_path: String::new(),
            xml_move_destination: None,
            xml_pending_folder_move: None,
            xml_folder_move_selection: None,
            xml_folder_move_destination: None,
            xml_folder_move_export_path: String::new(),
            xml_pending_move: None,
            xml_move_export_path: String::new(),
            xml_pending_append: None,
            xml_append_export_path: String::new(),
            xml_insert_position: 0,
            xml_pending_insert: None,
            xml_pending_clear: None,
            xml_clear_export_path: String::new(),
            xml_insert_export_path: String::new(),
            xml_pending_playlist_delete: None,
            xml_playlist_delete_export_path: String::new(),
            xml_pending_delete: None,
            xml_delete_export_path: String::new(),
            xml_selected_parent: None,
            xml_create_name: String::new(),
            xml_pending_create: None,
            xml_create_export_path: String::new(),
            xml_folder_create_name: String::new(),
            xml_pending_folder_create: None,
            xml_folder_create_export_path: String::new(),
            xml_selected_folder_delete: None,
            xml_pending_folder_delete: None,
            xml_folder_delete_export_path: String::new(),
            xml_selected_folder_rename: None,
            xml_folder_rename_name: String::new(),
            xml_pending_folder_rename: None,
            xml_folder_rename_export_path: String::new(),
            xml_playlist_rename_name: String::new(),
            xml_pending_rename: None,
            xml_rename_export_path: String::new(),
            xml_export_path: String::new(),
            xml_edit_error: None,
            show_control_gallery: false,
            control_gallery: control_gallery::GalleryState::default(),
            show_waveform_demo: false,
            waveform_demo: None,
            availability_filter: None,
            deck_playing: [false; 2],
            deck_cue: [false; 2],
        };
        app.reload();
        app
    }

    pub(crate) fn show_control_gallery_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.button("Control concepts…");
        if response.clicked() {
            self.show_control_gallery = true;
        }
        response
    }

    pub(crate) fn show_control_gallery_window(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
        if self.show_control_gallery {
            let mut open = self.show_control_gallery;
            egui::Window::new("OxiDJ · Control concept gallery")
                .open(&mut open)
                .default_width(900.0)
                .min_width(360.0)
                .show(ctx, |ui| {
                    control_gallery::show(ui, &mut self.control_gallery, tokens)
                });
            self.show_control_gallery = open;
        }
    }

    pub(crate) fn show_waveform_demo_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.button("Waveform view…");
        if response.clicked() {
            self.show_waveform_demo = true;
        }
        response
    }

    /// Provisional waveform renderer demo on synthetic peaks (built on first open).
    pub(crate) fn show_waveform_demo_window(&mut self, ctx: &egui::Context) {
        if self.show_waveform_demo {
            let state = self.waveform_demo.get_or_insert_with(Default::default);
            let mut open = self.show_waveform_demo;
            egui::Window::new("OxiDJ · Waveform view (provisional)")
                .open(&mut open)
                .default_width(900.0)
                .min_width(360.0)
                .show(ctx, |ui| waveform_view::show_demo(ui, state));
            self.show_waveform_demo = open;
        }
    }

    pub(crate) fn active_tokens(&self) -> theme::ColorTokens {
        if self.theme == ThemeMode::Custom {
            self.custom_tokens
        } else {
            self.theme.tokens()
        }
    }

    pub(crate) fn select_theme(&mut self, mode: ThemeMode) {
        let previous = self.theme;
        if mode == ThemeMode::Custom {
            if previous != ThemeMode::Custom {
                self.custom_tokens = previous.tokens();
            }
            self.show_theme_editor = true;
        }
        self.theme = mode;
        self.sync_accent_fields();
    }

    pub(crate) fn sync_accent_fields(&mut self) {
        self.primary_hex = color_hex(self.custom_tokens.primary);
        self.secondary_hex = color_hex(self.custom_tokens.secondary);
        self.accent_error = None;
    }

    pub(crate) fn commit_accent(&mut self, primary: bool) {
        let text = if primary {
            &self.primary_hex
        } else {
            &self.secondary_hex
        };
        match parse_hex_color(text) {
            Some(color) => {
                self.theme = ThemeMode::Custom;
                if primary {
                    self.custom_tokens.primary = color;
                } else {
                    self.custom_tokens.secondary = color;
                }
                self.accent_error = None;
            }
            None => self.accent_error = Some("Enter a complete #RRGGBB color".into()),
        }
    }

    pub(crate) fn handle_theme_shortcuts(&mut self, ctx: &egui::Context) {
        let mode = ctx.input(|input| {
            if !input.modifiers.ctrl || !input.modifiers.alt {
                return None;
            }
            if input.key_pressed(egui::Key::Num1) {
                Some(ThemeMode::TokyoNight)
            } else if input.key_pressed(egui::Key::Num2) {
                Some(ThemeMode::TokyoDay)
            } else if input.key_pressed(egui::Key::Num3) {
                Some(ThemeMode::Custom)
            } else {
                None
            }
        });
        if let Some(mode) = mode {
            self.select_theme(mode);
        }
    }

    pub(crate) fn show_palette_fields(&mut self, ui: &mut egui::Ui, error_color: egui::Color32) {
        ui.horizontal(|ui| {
            let label = ui.label("Primary");
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut self.primary_hex)
                        .id(egui::Id::new("primary-hex"))
                        .desired_width(90.0)
                        .hint_text("#RRGGBB"),
                )
                .labelled_by(label.id);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.commit_accent(true);
            }
            if ui.button("Apply").clicked() {
                self.commit_accent(true);
            }
        });
        let primary_before = self.custom_tokens.primary;
        edit_color(ui, "Primary swatch", &mut self.custom_tokens.primary);
        if self.custom_tokens.primary != primary_before {
            self.primary_hex = color_hex(self.custom_tokens.primary);
            self.accent_error = None;
        }
        ui.horizontal(|ui| {
            let label = ui.label("Secondary");
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut self.secondary_hex)
                        .id(egui::Id::new("secondary-hex"))
                        .desired_width(90.0)
                        .hint_text("#RRGGBB"),
                )
                .labelled_by(label.id);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.commit_accent(false);
            }
            if ui.button("Apply").clicked() {
                self.commit_accent(false);
            }
        });
        let secondary_before = self.custom_tokens.secondary;
        edit_color(ui, "Secondary swatch", &mut self.custom_tokens.secondary);
        if self.custom_tokens.secondary != secondary_before {
            self.secondary_hex = color_hex(self.custom_tokens.secondary);
            self.accent_error = None;
        }
        if let Some(error) = &self.accent_error {
            ui.colored_label(error_color, error);
        }
        edit_color(ui, "Border", &mut self.custom_tokens.border);
        edit_color(ui, "Focus", &mut self.custom_tokens.focus);
        edit_color(ui, "Success", &mut self.custom_tokens.success);
        edit_color(ui, "Warning", &mut self.custom_tokens.warning);
        edit_color(ui, "Error", &mut self.custom_tokens.error);
        let mut reset = false;
        ui.horizontal(|ui| {
            if ui.button("Reset to Tokyo Night").clicked() {
                self.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoNight);
                reset = true;
            }
            if ui.button("Reset to Tokyo Day").clicked() {
                self.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
                reset = true;
            }
        });
        if reset {
            self.sync_accent_fields();
        }
    }

    pub(crate) fn reload(&mut self) {
        self.tracks.clear();
        self.load_error = None;
        match &self.catalog {
            Ok(db) => match db.list_track_details() {
                Ok(rows) => self.tracks = rows,
                Err(e) => self.load_error = Some(e.to_string()),
            },
            Err(e) => self.load_error = Some(e.clone()),
        }
    }

    pub(crate) fn import(&mut self) {
        match &mut self.catalog {
            Ok(db) => match db.import_audio_directory(&self.import_root) {
                Ok(report) => {
                    self.notice = format!(
                        "Import: {} outcomes{}",
                        report.outcomes.len(),
                        if report.complete {
                            ""
                        } else {
                            " (incomplete; missing states unchanged)"
                        }
                    );
                    self.reload();
                }
                Err(e) => self.notice = format!("Import failed: {e}"),
            },
            Err(e) => self.notice = format!("Catalog unavailable: {e}"),
        }
    }

    pub(crate) fn select_xml_track(&mut self, track_id: &str, name: &str) {
        self.xml_selected_track = Some(track_id.to_owned());
        self.xml_edit_name = name.to_owned();
        self.load_track_attribute_inputs(track_id);
        self.xml_pending_edit = None;
        self.xml_pending_insert = None;
        self.xml_pending_clear = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn on_xml_source_changed(&mut self) {
        self.discard_track_attribute_edits();
        self.xml_pending_create = None;
        self.xml_pending_folder_rename = None;
        self.xml_selected_folder_rename = None;
        self.xml_pending_folder_create = None;
        self.xml_pending_folder_delete = None;
        self.xml_selected_folder_delete = None;
        self.xml_pending_rename = None;
        self.xml_pending_playlist_delete = None;
        self.xml_selected_parent = None;
        self.xml_pending_clear = None;
        self.xml_pending_reorder = None;
        self.xml_pending_append = None;
        self.xml_pending_insert = None;
        self.xml_pending_edit = None;
        self.xml_pending_move = None;
        self.xml_pending_folder_move = None;
        self.invalidate_xml_member_delete();
        self.xml_selected_playlist = None;
        self.xml_selected_member = None;
    }

    pub(crate) fn select_xml_playlist(&mut self, index: usize) {
        self.xml_pending_rename = None;
        self.xml_pending_playlist_delete = None;
        self.xml_pending_move = None;
        self.xml_playlist_rename_name = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.playlists.get(index))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.xml_pending_clear = None;
        self.xml_selected_playlist = Some(index);
        self.xml_selected_member = None;
        self.xml_pending_reorder = None;
        self.xml_pending_append = None;
        self.xml_pending_insert = None;
        self.xml_pending_remixer_edit = None;
        self.invalidate_xml_member_delete();
        self.xml_edit_error = None;
    }

    pub(crate) fn select_xml_member(&mut self, index: usize) {
        self.xml_selected_member = Some(index);
        self.xml_pending_reorder = None;
        self.invalidate_xml_member_delete();
    }

    pub(crate) fn preview_xml(&mut self) {
        self.discard_track_attribute_edits();
        self.xml_pending_folder_rename = None;
        self.xml_selected_folder_rename = None;
        self.xml_pending_folder_delete = None;
        self.xml_selected_folder_delete = None;
        self.xml_pending_folder_create = None;
        self.xml_pending_clear = None;
        self.xml_pending_edit = None;
        self.xml_pending_reorder = None;
        self.xml_pending_append = None;
        self.xml_pending_insert = None;
        self.xml_pending_delete = None;
        self.xml_pending_playlist_delete = None;
        self.xml_selected_playlist = None;
        self.xml_selected_member = None;
        self.xml_selected_track = None;
        self.xml_pending_create = None;
        self.xml_pending_rename = None;
        self.xml_pending_move = None;
        self.xml_pending_folder_move = None;
        self.xml_edit_error = None;
        let source = PathBuf::from(&self.xml_path);
        self.xml_document_source = Some(source.clone());
        self.xml_preview = Some(
            oxidj_application::preview_rekordbox_xml(&self.xml_path).map_err(|e| e.to_string()),
        );
        self.show_xml_preview = true;
    }

    pub(crate) fn theme_selection_ui(&mut self, ui: &mut egui::Ui) -> (egui::Rect, Option<[egui::Rect; 3]>) {
        let mut selected = self.theme;
        let mut option_rects = None;
        let label = ui.label("Theme (Ctrl+Alt+1/2/3)");
        let combo = egui::ComboBox::from_id_salt("theme-selection")
            .selected_text(self.theme.label())
            .show_ui(ui, |ui| {
                let night =
                    ui.selectable_value(&mut selected, ThemeMode::TokyoNight, "Tokyo Night");
                let day = ui.selectable_value(&mut selected, ThemeMode::TokyoDay, "Tokyo Day");
                let custom = ui.selectable_value(&mut selected, ThemeMode::Custom, "Custom");
                option_rects = Some([night.rect, day.rect, custom.rect]);
            });
        if selected != self.theme {
            self.select_theme(selected);
        }
        (combo.response.labelled_by(label.id).rect, option_rects)
    }
}
