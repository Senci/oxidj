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
    pub(crate) pdb_import_open: bool,
    pub(crate) pdb_source_path: String,
    pub(crate) pdb_destination_path: String,
    pub(crate) pdb_pending: Option<(PathBuf, PathBuf)>,
    pub(crate) pdb_job: Option<
        std::sync::mpsc::Receiver<(
            PathBuf,
            Result<oxidj_application::LibraryImportReport, String>,
        )>,
    >,
    #[cfg(test)]
    pub(crate) pdb_gate: Option<std::sync::Arc<std::sync::Barrier>>,
    pub(crate) catalog_export_open: bool,
    pub(crate) catalog_export_path: String,
    pub(crate) catalog_export_drive: String,
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
    pub(crate) track_waveform: track_waveform::TrackWaveformPanel,
    pub(crate) waveform_job: Option<waveform_analysis::Job>,
    #[cfg(test)]
    pub(crate) waveform_gate: Option<std::sync::Arc<std::sync::Barrier>>,
    #[cfg(test)]
    pub(crate) artwork_gate: Option<std::sync::Arc<std::sync::Barrier>>,
    pub(crate) availability_filter: Option<TrackAvailability>,
    /// Structural node path (not membership, names, or local audio IDs).
    pub(crate) imported_playlist: Option<Vec<usize>>,
    pub(crate) imported_root_guard: Option<oxidj_library::RootPlaylistGuard>,
    pub(crate) catalog_rename_open: bool,
    pub(crate) catalog_rename_name: String,
    pub(crate) catalog_remove_pending:
        Option<(oxidj_library::RootPlaylistGuard, Vec<usize>, usize, usize)>,
    pub(crate) selected_imported_guard: Option<oxidj_library::ImportedTrackGuard>,
    pub(crate) catalog_append_pending: Option<(
        usize,
        oxidj_library::ImportedTrackGuard,
        Option<(usize, oxidj_library::RootPlaylistGuard)>,
    )>,
    pub(crate) catalog_create_pending: Option<(usize, oxidj_library::ImportedTrackGuard)>,
    pub(crate) catalog_create_name: String,
    pub(crate) imported_member: Option<usize>,
    pub(crate) selected_imported: Option<usize>,
    pub(crate) dragging_track: Option<TrackSource>,
    pub(crate) imported_navigation:
        Result<std::sync::Arc<oxidj_library::LibraryNavigation>, String>,
    /// M8/D1 deck area (silent transport until D2).
    pub(crate) decks: crate::decks::DeckArea,
    pub(crate) artwork_cache: crate::artwork::Cache,
    /// W8 shell panel toggles (app state only, not persisted).
    pub(crate) show_nav: bool,
    pub(crate) show_details: bool,
    pub(crate) show_decks: bool,
    /// Rects of the shell regions painted in the last frame (for tests).
    pub(crate) shell: ShellLayout,
    /// M8/D0 persisted settings and the Settings window state.
    pub(crate) settings: crate::settings::Settings,
    pub(crate) settings_ui: crate::settings::SettingsUi,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TrackSource {
    Catalog(u64),
    Imported(usize),
}

/// Where the last frame placed each shell region; `None` when hidden.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ShellLayout {
    pub(crate) toolbar: Option<egui::Rect>,
    pub(crate) status: Option<egui::Rect>,
    pub(crate) nav: Option<egui::Rect>,
    pub(crate) details: Option<egui::Rect>,
    pub(crate) decks: Option<egui::Rect>,
    pub(crate) table: Option<egui::Rect>,
    /// Toolbar toggles: navigation, details, decks.
    pub(crate) toggles: Option<[egui::Rect; 3]>,
    pub(crate) more: Option<egui::Rect>,
    pub(crate) more_popup: Option<egui::Rect>,
    /// Toolbar gear that opens Settings.
    pub(crate) settings: Option<egui::Rect>,
    /// Search text field and, when there is text, its clear (x) button.
    pub(crate) search: Option<egui::Rect>,
    pub(crate) search_clear: Option<egui::Rect>,
    /// Top-bar master and cue controls: master, cue mix, cue volume, then the
    /// master and cue meters (`Rect::NOTHING` when dropped for width).
    pub(crate) monitor: Option<[egui::Rect; 5]>,
}

impl LibraryApp {
    pub(crate) fn new(
        db_path: PathBuf,
        theme: ThemeMode,
        custom_tokens: theme::ColorTokens,
    ) -> Self {
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
            pdb_import_open: false,
            pdb_source_path: String::new(),
            pdb_destination_path: String::new(),
            pdb_pending: None,
            pdb_job: None,
            #[cfg(test)]
            pdb_gate: None,
            catalog_export_open: false,
            catalog_export_path: String::new(),
            catalog_export_drive: String::new(),
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
            track_waveform: Default::default(),
            waveform_job: None,
            #[cfg(test)]
            waveform_gate: None,
            #[cfg(test)]
            artwork_gate: None,
            availability_filter: None,
            imported_playlist: None,
            imported_root_guard: None,
            catalog_rename_open: false,
            catalog_rename_name: String::new(),
            catalog_remove_pending: None,
            selected_imported_guard: None,
            catalog_append_pending: None,
            catalog_create_pending: None,
            catalog_create_name: String::new(),
            imported_member: None,
            selected_imported: None,
            dragging_track: None,
            imported_navigation: Err("Imported playlists have not been loaded".into()),
            decks: Default::default(),
            artwork_cache: Default::default(),
            show_nav: true,
            show_details: true,
            show_decks: true,
            shell: ShellLayout::default(),
            settings: Default::default(),
            settings_ui: Default::default(),
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

    pub(crate) fn show_control_gallery_window(
        &mut self,
        ctx: &egui::Context,
        tokens: theme::ColorTokens,
    ) {
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
        if mode == ThemeMode::Custom {
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

    /// Ctrl+B navigation, Ctrl+I details, Ctrl+D decks (Ctrl+Alt is the
    /// theme chord, so it is left to [`Self::handle_theme_shortcuts`]).
    pub(crate) fn handle_panel_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input_mut(|input| {
            if input.modifiers.alt {
                return;
            }
            for (key, panel) in [
                (egui::Key::B, &mut self.show_nav),
                (egui::Key::I, &mut self.show_details),
                (egui::Key::D, &mut self.show_decks),
            ] {
                if input.consume_key(egui::Modifiers::COMMAND, key) {
                    *panel = !*panel;
                }
            }
        });
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
        self.track_waveform.invalidate();
        match &self.catalog {
            Ok(db) => {
                self.imported_navigation = db
                    .library_navigation()
                    .map(std::sync::Arc::new)
                    .map_err(|error| error.to_string());
                match db.list_track_details() {
                    Ok(rows) => self.tracks = rows,
                    Err(e) => self.load_error = Some(e.to_string()),
                }
            }
            Err(e) => {
                self.imported_navigation = Err(e.clone());
                self.load_error = Some(e.clone());
            }
        }
        self.refresh_decks();
    }

    pub(crate) fn load_imported_member(&mut self, member: usize, deck: usize) {
        let result = (|| {
            let imported = self.imported_navigation.as_ref().map_err(Clone::clone)?;
            let imported = imported
                .tracks
                .get(member)
                .ok_or_else(|| "playlist member is unavailable".to_owned())?;
            let id = self
                .catalog
                .as_mut()
                .map_err(|e| e.clone())?
                .reference_id_for_library_location(&imported.location)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "playlist audio is unavailable".to_owned())?;
            self.reload();
            self.tracks
                .iter()
                .find(|item| item.track.id == id)
                .map(|item| item.track.id.as_u64())
                .ok_or_else(|| "resolved audio reference is unavailable".to_owned())
        })();
        match result {
            Ok(id) => self.load_deck(deck, id),
            Err(error) => self.notice = format!("Playlist load refused: {error}"),
        }
    }

    pub(crate) fn import(&mut self) {
        // Keep the active job's completion receiver and avoid overlapping writers.
        if self.waveform_job.is_some() {
            return;
        }
        match &mut self.catalog {
            Ok(db) => match db.import_audio_directory(&self.import_root) {
                Ok(report) => {
                    let imported = waveform_analysis::imported_paths(&report);
                    let ids = db.list_track_details().map(|tracks| {
                        tracks
                            .into_iter()
                            .filter(|track| imported.contains(track.track.path.as_str()))
                            .map(|track| track.track.id.as_u64())
                            .collect::<Vec<_>>()
                    });
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
                    match ids {
                        Ok(ids) if !ids.is_empty() => {
                            let count = ids.len();
                            let start_result = {
                                #[cfg(test)]
                                {
                                    waveform_analysis::Job::start_after_gate(
                                        self.db_path.clone(),
                                        ids,
                                        self.waveform_gate.take(),
                                    )
                                }
                                #[cfg(not(test))]
                                {
                                    waveform_analysis::Job::start(self.db_path.clone(), ids)
                                }
                            };
                            self.waveform_job = match start_result {
                                Ok(job) => {
                                    self.notice = format!("{count} imported; analysing waveforms");
                                    Some(job)
                                }
                                Err(error) => {
                                    self.notice = format!(
                                        "{count} imported; waveform analysis could not start: {error}"
                                    );
                                    None
                                }
                            };
                        }
                        Err(error) => {
                            self.notice = format!(
                                "Import complete; could not queue waveform analysis: {error}"
                            )
                        }
                        _ => self.waveform_job = None,
                    }
                }
                Err(e) => self.notice = format!("Import failed: {e}"),
            },
            Err(e) => self.notice = format!("Catalog unavailable: {e}"),
        }
    }

    pub(crate) fn pump_waveform_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.waveform_job else {
            return;
        };
        if let Some(result) = job.poll() {
            let total = job.total();
            self.waveform_job = None;
            match result {
                Ok(results) => {
                    let succeeded = results.iter().filter(|(_, result)| result.is_ok()).count();
                    let failed = results.len() - succeeded;
                    self.notice = format!(
                        "Waveform analysis complete: {succeeded} succeeded, {failed} failed (of {total})"
                    );
                    self.reload();
                }
                Err(()) => {
                    self.notice = "Waveform analysis worker stopped; results unavailable".into()
                }
            }
        } else {
            let (done, total) = (job.completed(), job.total());
            self.notice = format!("Waveform analysis: {done}/{total} tracks");
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
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

    #[cfg(test)]
    pub(crate) fn theme_selection_ui(
        &mut self,
        ui: &mut egui::Ui,
    ) -> (egui::Rect, Option<[egui::Rect; 3]>) {
        let mut selected = self.theme;
        let mut option_rects = None;
        // Combo first, then its label: the W8 toolbar lays this group out
        // right-to-left, so the label still reads before the picker.
        let combo = egui::ComboBox::from_id_salt("theme-selection")
            .selected_text(self.theme.label())
            .show_ui(ui, |ui| {
                let night =
                    ui.selectable_value(&mut selected, ThemeMode::TokyoNight, "Tokyo Night");
                let day = ui.selectable_value(&mut selected, ThemeMode::TokyoDay, "Tokyo Day");
                let custom = ui.selectable_value(&mut selected, ThemeMode::Custom, "Custom");
                option_rects = Some([night.rect, day.rect, custom.rect]);
            });
        // W11: a short muted caption; the shortcuts moved to its tooltip.
        let tokens = self.active_tokens();
        let label = ui
            .label(
                egui::RichText::new("Theme")
                    .small()
                    .color(tokens.muted_text),
            )
            .on_hover_text("Theme: Ctrl+Alt+1 Night · Ctrl+Alt+2 Day · Ctrl+Alt+3 Custom");
        if selected != self.theme {
            self.select_theme(selected);
        }
        (combo.response.labelled_by(label.id).rect, option_rects)
    }
}
