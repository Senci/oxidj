use eframe::egui;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use std::path::PathBuf;

mod components;
mod control_gallery;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_empty_folder_create_oracle.rs"]
mod f02_empty_folder_create_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_empty_folder_delete_oracle.rs"]
mod f02_empty_folder_delete_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_empty_playlist_create_oracle.rs"]
mod f02_empty_playlist_create_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_folder_rename_oracle.rs"]
mod f02_folder_rename_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_clear_oracle.rs"]
mod f02_playlist_clear_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_delete_oracle.rs"]
mod f02_playlist_delete_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_member_delete_oracle.rs"]
mod f02_playlist_member_delete_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_rename_oracle.rs"]
mod f02_playlist_rename_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_sibling_playlist_reorder_oracle.rs"]
mod f02_sibling_playlist_reorder_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/common/f02_track_name_oracle_support.rs"]
mod f02_track_name_oracle_support;
mod theme;
use components::{PrimaryActionButton, SectionHeading, StatusBadge, StatusKind};
use theme::ThemeMode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Id,
    Filename,
    Duration,
}
#[derive(Debug, Clone)]
struct PendingTrackNameEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackNameExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistMemberReorder {
    source: PathBuf,
    playlist_index: usize,
    prepared: oxidj_application::PreparedPlaylistMemberReorderExport,
}
#[derive(Debug, Clone)]
struct PendingSiblingPlaylistReorder {
    source: PathBuf,
    parent_index: usize,
    parent_name: String,
    expected_order: Vec<String>,
    new_order: Vec<String>,
    prepared: oxidj_application::PreparedSiblingPlaylistReorderExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistMemberAppend {
    source: PathBuf,
    playlist_index: usize,
    track_id: String,
    prepared: oxidj_application::PreparedPlaylistMemberAppendExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistMemberInsert {
    source: PathBuf,
    playlist_index: usize,
    track_id: String,
    position: usize,
    expected_order: Vec<String>,
    prepared: oxidj_application::PreparedPlaylistMemberInsertExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistClear {
    source: PathBuf,
    playlist_index: usize,
    expected_order: Vec<String>,
    prepared: oxidj_application::PreparedPlaylistClearExport,
}

#[derive(Debug, Clone)]
struct PendingPlaylistDelete {
    source: PathBuf,
    playlist_index: usize,
    name: String,
    expected_order: Vec<String>,
    prepared: oxidj_application::PreparedPlaylistDeleteExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistMemberDelete {
    source: PathBuf,
    playlist_index: usize,
    member_index: usize,
    track_id: String,
    prepared: oxidj_application::PreparedPlaylistMemberDeleteExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistRename {
    source: PathBuf,
    playlist_index: usize,
    current_name: String,
    name: String,
    prepared: oxidj_application::PreparedPlaylistRenameExport,
}

#[derive(Debug, Clone)]
struct PendingFolderDelete {
    source: PathBuf,
    folder_index: usize,
    ordinal: usize,
    name: String,
    path: Vec<String>,
    count: Option<String>,
    parent_path: Vec<String>,
    parent_count: Option<String>,
    prepared: oxidj_application::PreparedFolderDeleteExport,
}

#[derive(Debug, Clone)]
struct PendingFolderRename {
    source: PathBuf,
    folder_index: usize,
    ordinal: usize,
    old_name: String,
    path: Vec<String>,
    new_name: String,
    prepared: oxidj_application::PreparedFolderRenameExport,
}

#[derive(Debug, Clone)]
struct PendingFolderCreate {
    source: PathBuf,
    parent_index: usize,
    parent_ordinal: usize,
    parent_name: String,
    name: String,
    parent_count: Option<String>,
    prepared: oxidj_application::PreparedFolderCreateExport,
}

#[derive(Debug, Clone)]
struct PendingPlaylistCreate {
    source: PathBuf,
    parent_index: usize,
    parent_name: String,
    name: String,
    prepared: oxidj_application::PreparedPlaylistCreateExport,
}

fn immediate_parent_folder<'a>(
    preview: &'a oxidj_rekordbox::Preview,
    playlist: &oxidj_rekordbox::Playlist,
) -> Option<&'a oxidj_rekordbox::Folder> {
    let parent_name = playlist.path.get(playlist.path.len().checked_sub(2)?)?;
    let ancestors = playlist.path.get(..playlist.path.len().checked_sub(1)?)?;
    preview.folders.iter().find(|folder| {
        folder.name == *parent_name
            && folder.path.len() == ancestors.len() + 1
            && folder.path.get(1..) == Some(ancestors)
    })
}

struct LibraryApp {
    db_path: PathBuf,
    catalog: Result<Catalog, String>,
    tracks: Vec<TrackDetails>,
    load_error: Option<String>,
    search: String,
    import_root: String,
    selected: Option<u64>,
    sort: Sort,
    descending: bool,
    notice: String,
    theme: ThemeMode,
    custom_tokens: theme::ColorTokens,
    show_theme_editor: bool,
    primary_hex: String,
    secondary_hex: String,
    accent_error: Option<String>,
    xml_path: String,
    xml_document_source: Option<PathBuf>,
    xml_preview: Option<Result<oxidj_application::Preview, String>>,
    show_xml_preview: bool,
    xml_selected_track: Option<String>,
    xml_edit_name: String,
    xml_pending_edit: Option<PendingTrackNameEdit>,
    xml_selected_playlist: Option<usize>,
    xml_selected_member: Option<usize>,
    xml_pending_reorder: Option<PendingPlaylistMemberReorder>,
    xml_reorder_export_path: String,
    xml_sibling_parent: Option<usize>,
    xml_sibling_order: String,
    xml_sibling_pending: Option<PendingSiblingPlaylistReorder>,
    xml_sibling_export_path: String,
    xml_pending_append: Option<PendingPlaylistMemberAppend>,
    xml_append_export_path: String,
    xml_insert_position: usize,
    xml_pending_insert: Option<PendingPlaylistMemberInsert>,
    xml_pending_clear: Option<PendingPlaylistClear>,
    xml_clear_export_path: String,
    xml_insert_export_path: String,
    xml_pending_playlist_delete: Option<PendingPlaylistDelete>,
    xml_playlist_delete_export_path: String,
    xml_pending_delete: Option<PendingPlaylistMemberDelete>,
    xml_delete_export_path: String,
    xml_selected_parent: Option<usize>,
    xml_create_name: String,
    xml_pending_create: Option<PendingPlaylistCreate>,
    xml_create_export_path: String,
    xml_folder_create_name: String,
    xml_pending_folder_create: Option<PendingFolderCreate>,
    xml_folder_create_export_path: String,
    xml_selected_folder_delete: Option<usize>,
    xml_pending_folder_delete: Option<PendingFolderDelete>,
    xml_folder_delete_export_path: String,
    xml_selected_folder_rename: Option<usize>,
    xml_folder_rename_name: String,
    xml_pending_folder_rename: Option<PendingFolderRename>,
    xml_folder_rename_export_path: String,
    xml_playlist_rename_name: String,
    xml_pending_rename: Option<PendingPlaylistRename>,
    xml_rename_export_path: String,
    xml_export_path: String,
    xml_edit_error: Option<String>,
    show_control_gallery: bool,
    control_gallery: control_gallery::GalleryState,
    availability_filter: Option<TrackAvailability>,
    deck_playing: [bool; 2],
    deck_cue: [bool; 2],
}
impl LibraryApp {
    fn new(db_path: PathBuf, theme: ThemeMode, custom_tokens: theme::ColorTokens) -> Self {
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
            xml_selected_playlist: None,
            xml_selected_member: None,
            xml_pending_reorder: None,
            xml_reorder_export_path: String::new(),
            xml_sibling_parent: None,
            xml_sibling_order: String::new(),
            xml_sibling_pending: None,
            xml_sibling_export_path: String::new(),
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
            availability_filter: None,
            deck_playing: [false; 2],
            deck_cue: [false; 2],
        };
        app.reload();
        app
    }
    fn show_control_gallery_button(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.button("Control concepts…");
        if response.clicked() {
            self.show_control_gallery = true;
        }
        response
    }

    fn show_control_gallery_window(&mut self, ctx: &egui::Context, tokens: theme::ColorTokens) {
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

    fn active_tokens(&self) -> theme::ColorTokens {
        if self.theme == ThemeMode::Custom {
            self.custom_tokens
        } else {
            self.theme.tokens()
        }
    }

    fn select_theme(&mut self, mode: ThemeMode) {
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

    fn sync_accent_fields(&mut self) {
        self.primary_hex = color_hex(self.custom_tokens.primary);
        self.secondary_hex = color_hex(self.custom_tokens.secondary);
        self.accent_error = None;
    }

    fn commit_accent(&mut self, primary: bool) {
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

    fn handle_theme_shortcuts(&mut self, ctx: &egui::Context) {
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

    fn show_palette_fields(&mut self, ui: &mut egui::Ui, error_color: egui::Color32) {
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

    fn reload(&mut self) {
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
    fn import(&mut self) {
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

    fn prepare_xml_sibling_playlist_reorder(&mut self) {
        let Some(source) = self.xml_document_source.clone() else {
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(parent_index) = self.xml_sibling_parent else {
            return;
        };
        let Some(parent) = preview
            .folders
            .iter()
            .find(|folder| folder.ordinal == parent_index)
        else {
            return;
        };
        let expected_order: Vec<String> = preview
            .playlists
            .iter()
            .filter(|playlist| {
                playlist.path.len() >= 2
                    && playlist.path[..playlist.path.len() - 1] == parent.path[1..]
            })
            .map(|playlist| playlist.name.clone())
            .collect();
        let new_order: Vec<String> = self
            .xml_sibling_order
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let adapter_parent_index = parent_index.saturating_sub(1);
        let request = oxidj_application::SiblingPlaylistReorderRequest {
            parent_index: adapter_parent_index,
            expected_parent_name: parent.name.clone(),
            expected_order: expected_order.clone(),
            new_order: new_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_sibling_playlist_reorder_export(
            &source, &request,
        ) {
            Ok(prepared) => {
                self.xml_sibling_pending = Some(PendingSiblingPlaylistReorder {
                    source,
                    parent_index: adapter_parent_index,
                    parent_name: parent.name.clone(),
                    expected_order,
                    new_order,
                    prepared,
                })
            }
            Err(error) => {
                self.xml_sibling_pending = None;
                self.xml_edit_error = Some(format!("Sibling playlist reorder refused: {error}"));
            }
        }
    }

    fn export_xml_sibling_playlist_reorder(&mut self) {
        let Some(pending) = self.xml_sibling_pending.as_ref() else {
            return;
        };
        let current_parent = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| {
                p.folders
                    .iter()
                    .find(|f| f.ordinal == pending.parent_index + 1)
            });
        let current_order: Vec<String> = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .map(|p| {
                p.playlists
                    .iter()
                    .filter(|pl| {
                        pl.path.len() >= 2
                            && current_parent
                                .is_some_and(|f| pl.path[..pl.path.len() - 1] == f.path[1..])
                    })
                    .map(|pl| pl.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || current_parent.is_none_or(|p| p.name != pending.parent_name)
            || current_order != pending.expected_order
        {
            self.xml_sibling_pending = None;
            self.xml_edit_error = Some(
                "Sibling playlist reorder invalidated by source, parent, or order change".into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_sibling_export_path);
        match oxidj_application::export_prepared_rekordbox_sibling_playlist_reorder(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                let source = pending.source.clone();
                self.xml_sibling_pending = None;
                self.xml_path = output.display().to_string();
                self.preview_xml();
                let _ = source;
            }
            Err(error) => {
                self.xml_edit_error =
                    Some(format!("Sibling playlist reorder export failed: {error}"))
            }
        }
    }

    fn prepare_xml_member_reorder(
        &mut self,
        playlist_index: usize,
        member_index: usize,
        direction: isize,
    ) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_reorder = None;
            self.xml_edit_error =
                Some("Playlist reorder refused: no XML document is loaded".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            return;
        };
        let target = member_index as isize + direction;
        if target < 0 || target >= playlist.tracks.len() as isize {
            self.xml_pending_reorder = None;
            self.xml_edit_error =
                Some("Playlist reorder refused: member is at the boundary".into());
            return;
        }
        let mut new_order = playlist.tracks.clone();
        new_order.swap(member_index, target as usize);
        if new_order
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != new_order.len()
        {
            self.xml_pending_reorder = None;
            self.xml_edit_error =
                Some("Playlist reorder refused: duplicate members are unsupported".into());
            return;
        }
        let request = oxidj_application::PlaylistReorderRequest {
            playlist_index,
            expected_order: playlist.tracks.clone(),
            new_order,
        };
        match oxidj_application::prepare_rekordbox_playlist_member_reorder_export(&source, &request)
        {
            Ok(prepared) => {
                self.xml_pending_reorder = Some(PendingPlaylistMemberReorder {
                    source,
                    playlist_index,
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_reorder = None;
                self.xml_edit_error = Some(format!("Playlist reorder refused: {error}"));
            }
        }
    }

    fn export_xml_member_reorder(&mut self) {
        let Some(pending) = self.xml_pending_reorder.as_ref() else {
            self.xml_edit_error =
                Some("Playlist reorder export refused: no pending proposal".into());
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
        {
            self.xml_pending_reorder = None;
            self.xml_edit_error =
                Some("Playlist reorder export refused: source or playlist changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_reorder_export_path);
        if self.xml_reorder_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist reorder export refused: choose a new destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_playlist_member_reorder(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_pending_reorder = None;
                self.xml_edit_error = None;
                self.notice = format!("Playlist member order exported to {}", output.display());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist reorder export failed: {error}"))
            }
        }
    }

    fn prepare_xml_member_append(&mut self, playlist_index: usize, track_id: &str) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_append = None;
            self.xml_edit_error = Some("Playlist append refused: no XML document is loaded".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            return;
        };
        let request = oxidj_application::PlaylistMemberAppendRequest {
            playlist_index,
            expected_order: playlist.tracks.clone(),
            track_id: track_id.to_owned(),
        };
        match oxidj_application::prepare_rekordbox_playlist_member_append_export(&source, &request)
        {
            Ok(prepared) => {
                self.xml_pending_append = Some(PendingPlaylistMemberAppend {
                    source,
                    playlist_index,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_append = None;
                self.xml_edit_error = Some(format!("Playlist append refused: {error}"));
            }
        }
    }

    fn cancel_xml_member_append(&mut self) {
        self.xml_pending_append = None;
        self.xml_edit_error = None;
    }

    fn export_xml_member_append(&mut self) {
        let Some(pending) = self.xml_pending_append.as_ref() else {
            self.xml_edit_error =
                Some("Playlist append export refused: no pending proposal".into());
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
        {
            self.xml_pending_append = None;
            self.xml_edit_error =
                Some("Playlist append export refused: source or playlist changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_append_export_path);
        if self.xml_append_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist append export refused: choose a new destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_playlist_member_append(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.notice = format!("Playlist member appended to {}", output.display());
                self.xml_pending_append = None;
                self.xml_edit_error = None;
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist append export failed: {error}"))
            }
        }
    }

    fn prepare_xml_member_insert(
        &mut self,
        playlist_index: usize,
        track_id: &str,
        position: usize,
    ) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_insert = None;
            self.xml_edit_error =
                Some("Playlist insertion refused: no XML document is loaded".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            self.xml_pending_insert = None;
            self.xml_edit_error =
                Some("Playlist insertion refused: selected playlist is invalid".into());
            return;
        };
        let request = oxidj_application::PlaylistMemberInsertRequest {
            playlist_index,
            expected_order: playlist.tracks.clone(),
            track_id: track_id.to_owned(),
            position,
        };
        match oxidj_application::prepare_rekordbox_playlist_member_insert_export(&source, &request)
        {
            Ok(prepared) => {
                self.xml_pending_insert = Some(PendingPlaylistMemberInsert {
                    source,
                    playlist_index,
                    track_id: track_id.to_owned(),
                    position,
                    expected_order: playlist.tracks.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_insert = None;
                self.xml_edit_error = Some(format!("Playlist insertion refused: {error}"));
            }
        }
    }

    fn cancel_xml_member_insert(&mut self) {
        self.xml_pending_insert = None;
        self.xml_edit_error = None;
    }

    fn export_xml_member_insert(&mut self) {
        let Some(pending) = self.xml_pending_insert.clone() else {
            self.xml_edit_error =
                Some("Playlist insertion export refused: no pending proposal".into());
            return;
        };
        let order_unchanged = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| preview.playlists.get(pending.playlist_index))
            .is_some_and(|playlist| playlist.tracks == pending.expected_order);
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_insert_position != pending.position
            || !order_unchanged
        {
            self.xml_pending_insert = None;
            self.xml_edit_error = Some(
                "Playlist insertion export refused: source, playlist, track, or slot changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_insert_export_path);
        if self.xml_insert_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist insertion export refused: choose a new destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_playlist_member_insert(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_pending_insert = None;
                self.xml_edit_error = None;
                self.notice = format!(
                    "Playlist member inserted at slot {} in {}",
                    pending.position,
                    output.display()
                );
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist insertion export failed: {error}"))
            }
        }
    }

    fn prepare_xml_playlist_delete(&mut self, playlist_index: usize) {
        let (Some(source), Some(Ok(preview))) =
            (self.xml_document_source.clone(), self.xml_preview.as_ref())
        else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            self.xml_pending_playlist_delete = None;
            self.xml_edit_error =
                Some("Playlist deletion refused: selected playlist is invalid".into());
            return;
        };
        let name = playlist.name.clone();
        let expected_order = playlist.tracks.clone();
        let request = oxidj_application::PlaylistDeleteRequest {
            playlist_index,
            expected_name: name.clone(),
            expected_order: expected_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_playlist_delete_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_playlist_delete = Some(PendingPlaylistDelete {
                    source,
                    playlist_index,
                    name,
                    expected_order,
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_playlist_delete = None;
                self.xml_edit_error = Some(format!("Playlist deletion refused: {error}"));
            }
        }
    }
    fn cancel_xml_playlist_delete(&mut self) {
        self.xml_pending_playlist_delete = None;
        self.xml_edit_error = None;
    }
    fn export_xml_playlist_delete(&mut self) {
        let Some(pending) = self.xml_pending_playlist_delete.clone() else {
            self.xml_edit_error =
                Some("Playlist deletion export refused: no pending proposal".into());
            return;
        };
        let valid = self.xml_document_source.as_ref() == Some(&pending.source)
            && self.xml_selected_playlist == Some(pending.playlist_index)
            && self
                .xml_preview
                .as_ref()
                .and_then(|p| p.as_ref().ok())
                .and_then(|p| p.playlists.get(pending.playlist_index))
                .is_some_and(|p| p.name == pending.name && p.tracks == pending.expected_order);
        if !valid {
            self.xml_pending_playlist_delete = None;
            self.xml_edit_error = Some(
                "Playlist deletion refused: source, playlist, name, or membership order changed"
                    .into(),
            );
            return;
        }
        if self.xml_playlist_delete_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist deletion refused: choose a new destination".into());
            return;
        }
        let output = PathBuf::from(&self.xml_playlist_delete_export_path);
        match oxidj_application::export_prepared_rekordbox_playlist_delete(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_pending_playlist_delete = None;
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output.clone());
                self.notice = format!(
                    "Playlist node deleted in {}; collection tracks and other playlists are unchanged",
                    output.display()
                );
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist deletion export failed: {error}"))
            }
        }
    }

    fn prepare_xml_playlist_clear(&mut self, playlist_index: usize) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_edit_error = Some("Playlist clear refused: no XML document is loaded".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            self.xml_pending_clear = None;
            self.xml_edit_error =
                Some("Playlist clear refused: selected playlist is invalid".into());
            return;
        };
        if playlist.tracks.is_empty() {
            self.xml_pending_clear = None;
            self.xml_edit_error = Some("Playlist clear refused: playlist is empty".into());
            return;
        }
        let expected_order = playlist.tracks.clone();
        let request = oxidj_application::PlaylistClearRequest {
            playlist_index,
            expected_order: expected_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_playlist_clear_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_clear = Some(PendingPlaylistClear {
                    source,
                    playlist_index,
                    expected_order,
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_clear = None;
                self.xml_edit_error = Some(format!("Playlist clear refused: {error}"));
            }
        }
    }

    fn cancel_xml_playlist_clear(&mut self) {
        self.xml_pending_clear = None;
        self.xml_edit_error = None;
    }

    fn export_xml_playlist_clear(&mut self) {
        let Some(pending) = self.xml_pending_clear.clone() else {
            self.xml_edit_error = Some("Playlist clear export refused: no pending proposal".into());
            return;
        };
        let order_unchanged = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.playlists.get(pending.playlist_index))
            .is_some_and(|p| p.tracks == pending.expected_order);
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
            || !order_unchanged
        {
            self.xml_pending_clear = None;
            self.xml_edit_error = Some(
                "Playlist clear export refused: source, playlist, or membership order changed"
                    .into(),
            );
            return;
        }
        if self.xml_clear_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist clear export refused: choose a new destination".into());
            return;
        }
        let output = PathBuf::from(&self.xml_clear_export_path);
        match oxidj_application::export_prepared_rekordbox_playlist_clear(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_pending_clear = None;
                self.xml_edit_error = None;
                self.notice = format!(
                    "Playlist membership cleared in {}; collection tracks and other playlists are unchanged",
                    output.display()
                );
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist clear export failed: {error}"))
            }
        }
    }

    fn prepare_xml_folder_delete(&mut self) {
        let (Some(source), Some(folder_index)) = (
            self.xml_document_source.clone(),
            self.xml_selected_folder_delete,
        ) else {
            self.xml_pending_folder_delete = None;
            self.xml_edit_error = Some("Folder deletion refused: select one folder".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(folder) = preview
            .folders
            .get(folder_index)
            .filter(|f| f.path.len() > 1 && f.count.as_deref() == Some("0"))
        else {
            self.xml_pending_folder_delete = None;
            self.xml_edit_error =
                Some("Folder deletion refused: folder must be empty and non-root".into());
            return;
        };
        let Some(parent_path) = folder
            .path
            .get(..folder.path.len() - 1)
            .map(<[String]>::to_vec)
        else {
            return;
        };
        let Some(parent) = preview.folders.iter().find(|f| f.path == parent_path) else {
            self.xml_pending_folder_delete = None;
            self.xml_edit_error =
                Some("Folder deletion refused: immediate parent not found by full path".into());
            return;
        };
        let request = oxidj_application::FolderDeleteRequest {
            folder_index: folder.ordinal.saturating_sub(1),
            expected_name: folder.name.clone(),
        };
        match oxidj_application::prepare_rekordbox_folder_delete_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_folder_delete = Some(PendingFolderDelete {
                    source,
                    folder_index,
                    ordinal: folder.ordinal,
                    name: folder.name.clone(),
                    path: folder.path.clone(),
                    count: folder.count.clone(),
                    parent_path,
                    parent_count: parent.count.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_folder_delete = None;
                self.xml_edit_error = Some(format!("Folder deletion refused: {error}"));
            }
        }
    }
    fn cancel_xml_folder_delete(&mut self) {
        self.xml_pending_folder_delete = None;
        self.xml_edit_error = None;
    }
    fn export_xml_folder_delete(&mut self) {
        let Some(pending) = self.xml_pending_folder_delete.clone() else {
            self.xml_edit_error = Some("Folder deletion refused: no pending proposal".into());
            return;
        };
        let parent_unchanged = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .is_some_and(|p| {
                p.folders
                    .iter()
                    .any(|f| f.path == pending.parent_path && f.count == pending.parent_count)
            });
        let valid = self.xml_document_source.as_ref() == Some(&pending.source)
            && self.xml_selected_folder_delete == Some(pending.folder_index)
            && parent_unchanged
            && self
                .xml_preview
                .as_ref()
                .and_then(|p| p.as_ref().ok())
                .and_then(|p| p.folders.get(pending.folder_index))
                .is_some_and(|f| {
                    f.ordinal == pending.ordinal
                        && f.name == pending.name
                        && f.path == pending.path
                        && f.count == pending.count
                        && f.count.as_deref() == Some("0")
                });
        if !valid {
            self.xml_pending_folder_delete = None;
            self.xml_edit_error =
                Some("Folder deletion refused: source or selected folder identity changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_folder_delete_export_path);
        match oxidj_application::export_prepared_rekordbox_folder_delete(&pending.prepared, &output)
        {
            Ok(()) => {
                self.xml_pending_folder_delete = None;
                self.xml_edit_error = None;
                self.notice = format!("Empty folder deletion exported to {}", output.display());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
                self.xml_selected_folder_delete = None;
            }
            Err(error) => {
                if error
                    .to_string()
                    .contains("source changed after folder deletion preparation")
                {
                    self.xml_pending_folder_delete = None;
                }
                self.xml_edit_error = Some(format!("Folder deletion export failed: {error}"))
            }
        }
    }

    fn prepare_xml_folder_rename(&mut self) {
        let (Some(source), Some(folder_index)) = (
            self.xml_document_source.clone(),
            self.xml_selected_folder_rename,
        ) else {
            self.xml_pending_folder_rename = None;
            self.xml_edit_error = Some("Folder rename refused: select one non-root folder".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(folder) = preview
            .folders
            .get(folder_index)
            .filter(|f| f.path.len() > 1)
        else {
            self.xml_pending_folder_rename = None;
            self.xml_edit_error =
                Some("Folder rename refused: root or invalid folder selection".into());
            return;
        };
        let request = oxidj_application::FolderRenameRequest {
            folder_index: folder.ordinal.saturating_sub(1),
            expected_old_name: folder.name.clone(),
            new_name: self.xml_folder_rename_name.clone(),
        };
        match oxidj_application::prepare_rekordbox_folder_rename_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_folder_rename = Some(PendingFolderRename {
                    source,
                    folder_index,
                    ordinal: folder.ordinal,
                    old_name: folder.name.clone(),
                    path: folder.path.clone(),
                    new_name: self.xml_folder_rename_name.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_folder_rename = None;
                self.xml_edit_error = Some(format!("Folder rename preparation refused: {error}"));
            }
        }
    }
    fn export_xml_folder_rename(&mut self) {
        let Some(pending) = self.xml_pending_folder_rename.clone() else {
            self.xml_edit_error = Some("Folder rename export refused: no prepared proposal".into());
            return;
        };
        let unchanged = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.folders.get(pending.folder_index))
            .is_some_and(|f| {
                f.ordinal == pending.ordinal && f.name == pending.old_name && f.path == pending.path
            });
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_folder_rename != Some(pending.folder_index)
            || self.xml_folder_rename_name != pending.new_name
            || !unchanged
        {
            self.xml_pending_folder_rename = None;
            self.xml_edit_error =
                Some("Folder rename export refused: source, selection, or name changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_folder_rename_export_path);
        match oxidj_application::export_prepared_rekordbox_folder_rename(&pending.prepared, &output)
        {
            Ok(()) => {
                self.xml_pending_folder_rename = None;
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output.clone());
                self.notice = format!("Folder renamed in {}", output.display());
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Folder rename export failed: {error}"))
            }
        }
    }

    fn prepare_xml_folder_create(&mut self) {
        let (Some(source), Some(parent_index)) =
            (self.xml_document_source.clone(), self.xml_selected_parent)
        else {
            self.xml_pending_folder_create = None;
            self.xml_edit_error =
                Some("Folder creation refused: select a paired non-root parent".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(parent) = preview
            .folders
            .get(parent_index)
            .filter(|folder| folder.path.len() > 1)
        else {
            self.xml_pending_folder_create = None;
            self.xml_edit_error =
                Some("Folder creation refused: selected parent is invalid or root".into());
            return;
        };
        let parent_name = parent.name.clone();
        let request = oxidj_application::FolderCreateRequest {
            parent_index: parent.ordinal.saturating_sub(1),
            expected_parent_name: parent_name.clone(),
            name: self.xml_folder_create_name.clone(),
        };
        match oxidj_application::prepare_rekordbox_folder_create_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_folder_create = Some(PendingFolderCreate {
                    source,
                    parent_index,
                    parent_ordinal: parent.ordinal,
                    parent_name,
                    name: self.xml_folder_create_name.clone(),
                    parent_count: parent.count.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_folder_create = None;
                self.xml_edit_error = Some(format!("Folder creation refused: {error}"));
            }
        }
    }

    fn cancel_xml_folder_create(&mut self) {
        self.xml_pending_folder_create = None;
        self.xml_edit_error = None;
    }

    fn export_xml_folder_create(&mut self) {
        let Some(pending) = self.xml_pending_folder_create.clone() else {
            self.xml_edit_error = Some("Folder creation refused: no pending proposal".into());
            return;
        };
        let parent_unchanged = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.folders.get(pending.parent_index))
            .is_some_and(|parent| {
                parent.ordinal == pending.parent_ordinal
                    && parent.name == pending.parent_name
                    && parent.count == pending.parent_count
                    && parent.path.len() > 1
            });
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_parent != Some(pending.parent_index)
            || self.xml_folder_create_name != pending.name
            || !parent_unchanged
        {
            self.xml_pending_folder_create = None;
            self.xml_edit_error =
                Some("Folder creation refused: source, parent, or name changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_folder_create_export_path);
        match oxidj_application::export_prepared_rekordbox_folder_create(&pending.prepared, &output)
        {
            Ok(()) => {
                self.xml_pending_folder_create = None;
                self.xml_edit_error = None;
                self.notice = format!("Empty folder exported to {}", output.display());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Folder creation export failed: {error}"))
            }
        }
    }

    fn prepare_xml_playlist_create(&mut self) {
        let (Some(source), Some(parent_index)) =
            (self.xml_document_source.clone(), self.xml_selected_parent)
        else {
            self.xml_pending_create = None;
            self.xml_edit_error =
                Some("Playlist creation refused: select a paired non-root parent".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(parent) = preview
            .folders
            .get(parent_index)
            .filter(|folder| folder.path.len() > 1)
        else {
            self.xml_pending_create = None;
            self.xml_edit_error =
                Some("Playlist creation refused: selected parent is invalid or root".into());
            return;
        };
        let parent_name = parent.name.clone();
        let request = oxidj_application::PlaylistCreateRequest {
            parent_index: parent.ordinal.saturating_sub(1),
            expected_parent_name: parent_name.clone(),
            name: self.xml_create_name.clone(),
        };
        match oxidj_application::prepare_rekordbox_playlist_create_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_create = Some(PendingPlaylistCreate {
                    source,
                    parent_index,
                    parent_name,
                    name: self.xml_create_name.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_create = None;
                self.xml_edit_error = Some(format!("Playlist creation refused: {error}"));
            }
        }
    }

    fn cancel_xml_playlist_create(&mut self) {
        self.xml_pending_create = None;
        self.xml_edit_error = None;
    }

    fn export_xml_playlist_create(&mut self) {
        let Some(pending) = self.xml_pending_create.clone() else {
            self.xml_edit_error = Some("Playlist creation refused: no pending proposal".into());
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_parent != Some(pending.parent_index)
            || self.xml_create_name != pending.name
            || self
                .xml_preview
                .as_ref()
                .and_then(|p| p.as_ref().ok())
                .and_then(|p| p.folders.get(pending.parent_index))
                .is_none_or(|parent| parent.name != pending.parent_name)
        {
            self.xml_pending_create = None;
            self.xml_edit_error =
                Some("Playlist creation refused: source, parent, or name changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_create_export_path);
        match oxidj_application::export_prepared_rekordbox_playlist_create(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_pending_create = None;
                self.xml_edit_error = None;
                self.notice = format!("Empty playlist exported to {}", output.display());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist creation export failed: {error}"))
            }
        }
    }

    fn prepare_xml_member_delete(&mut self, playlist_index: usize, member_index: usize) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_delete = None;
            self.xml_edit_error =
                Some("Playlist removal refused: no XML document is loaded".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            self.xml_pending_delete = None;
            self.xml_edit_error =
                Some("Playlist removal refused: selected playlist is invalid".into());
            return;
        };
        let Some(track_id) = playlist.tracks.get(member_index).cloned() else {
            self.xml_pending_delete = None;
            self.xml_edit_error =
                Some("Playlist removal refused: selected membership is invalid".into());
            return;
        };
        let request = oxidj_application::PlaylistMemberDeletePreparationRequest {
            source: source.clone(),
            removal: oxidj_application::PlaylistMemberDeleteRequest {
                playlist_index,
                expected_order: playlist.tracks.clone(),
                track_id: track_id.clone(),
            },
        };
        match oxidj_application::prepare_rekordbox_playlist_member_delete_export(&request) {
            Ok(prepared) => {
                self.xml_pending_delete = Some(PendingPlaylistMemberDelete {
                    source,
                    playlist_index,
                    member_index,
                    track_id,
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_delete = None;
                self.xml_edit_error = Some(format!("Playlist removal refused: {error}"));
            }
        }
    }

    fn invalidate_xml_member_delete(&mut self) {
        self.xml_pending_delete = None;
    }

    fn select_xml_track(&mut self, track_id: &str, name: &str) {
        self.xml_selected_track = Some(track_id.to_owned());
        self.xml_edit_name = name.to_owned();
        self.xml_pending_edit = None;
        self.xml_pending_insert = None;
        self.xml_pending_clear = None;
        self.xml_edit_error = None;
    }

    fn on_xml_source_changed(&mut self) {
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
        self.invalidate_xml_member_delete();
        self.xml_selected_playlist = None;
        self.xml_selected_member = None;
    }

    fn select_xml_playlist(&mut self, index: usize) {
        self.xml_pending_rename = None;
        self.xml_pending_playlist_delete = None;
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
        self.invalidate_xml_member_delete();
        self.xml_edit_error = None;
    }

    fn prepare_xml_playlist_rename(&mut self, index: usize, current_name: &str) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_rename = None;
            self.xml_edit_error = Some("Playlist rename refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::PlaylistRenameRequest {
            playlist_index: index,
            expected_old_name: current_name.to_owned(),
            new_name: self.xml_playlist_rename_name.clone(),
        };
        match oxidj_application::prepare_rekordbox_playlist_rename_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_rename = Some(PendingPlaylistRename {
                    source,
                    playlist_index: index,
                    current_name: current_name.to_owned(),
                    name: self.xml_playlist_rename_name.clone(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_rename = None;
                self.xml_edit_error = Some(format!("Playlist rename preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_playlist_rename(&mut self) {
        self.xml_pending_rename = None;
        self.xml_edit_error = None;
    }

    fn export_xml_playlist_rename(&mut self) {
        let Some(pending) = self.xml_pending_rename.as_ref() else {
            self.xml_edit_error =
                Some("Playlist rename export refused: no prepared proposal".into());
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
            || self.xml_playlist_rename_name != pending.name
            || self
                .xml_preview
                .as_ref()
                .and_then(|p| p.as_ref().ok())
                .and_then(|p| p.playlists.get(pending.playlist_index))
                .map(|p| p.name.as_str())
                != Some(pending.current_name.as_str())
        {
            self.xml_pending_rename = None;
            self.xml_edit_error =
                Some("Playlist rename export refused: source, selection, or name changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_rename_export_path);
        match oxidj_application::export_prepared_rekordbox_playlist_rename(
            &pending.prepared,
            &output,
        ) {
            Ok(_) => {
                self.notice = format!("Playlist renamed to {}", output.display());
                self.xml_pending_rename = None;
                self.xml_edit_error = None;
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist rename export failed: {error}"))
            }
        }
    }

    fn select_xml_member(&mut self, index: usize) {
        self.xml_selected_member = Some(index);
        self.xml_pending_reorder = None;
        self.invalidate_xml_member_delete();
    }

    fn cancel_xml_member_delete(&mut self) {
        self.invalidate_xml_member_delete();
        self.xml_edit_error = None;
    }

    fn export_xml_member_delete(&mut self) {
        let Some(pending) = self.xml_pending_delete.as_ref() else {
            self.xml_edit_error =
                Some("Playlist removal export refused: no pending proposal".into());
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
            || self.xml_selected_member != Some(pending.member_index)
            || preview
                .playlists
                .get(pending.playlist_index)
                .and_then(|playlist| playlist.tracks.get(pending.member_index))
                != Some(&pending.track_id)
        {
            self.xml_pending_delete = None;
            self.xml_edit_error =
                Some("Playlist removal export refused: source or membership changed".into());
            return;
        }
        let output = PathBuf::from(&self.xml_delete_export_path);
        if self.xml_delete_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Playlist removal export refused: choose a new destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_playlist_member_delete(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.notice = format!("Playlist member removed to {}", output.display());
                self.xml_pending_delete = None;
                self.xml_edit_error = None;
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_document_source = Some(output);
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Playlist removal export failed: {error}"))
            }
        }
    }

    fn preview_xml(&mut self) {
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
        self.xml_edit_error = None;
        let source = PathBuf::from(&self.xml_path);
        self.xml_document_source = Some(source.clone());
        self.xml_preview = Some(
            oxidj_application::preview_rekordbox_xml(&self.xml_path).map_err(|e| e.to_string()),
        );
        self.show_xml_preview = true;
    }

    fn prepare_xml_name_edit(&mut self, track_id: &str, old_name: &str) {
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_edit = None;
            self.xml_edit_error =
                Some("Track Name preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackNameEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_name: old_name.to_owned(),
            new_name: self.xml_edit_name.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_name_export(&request) {
            Ok(prepared) => {
                self.xml_pending_edit = Some(PendingTrackNameEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_edit = None;
                self.xml_edit_error = Some(format!("Track Name preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_name_edit(&mut self) {
        self.xml_pending_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_name_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_edit.as_ref() else {
            self.xml_edit_error =
                Some("Track Name export refused: no prepared pending edit".into());
            return;
        };
        if self.xml_document_source.as_ref() != Some(&pending.source) {
            self.xml_edit_error =
                Some("Track Name export refused: pending edit is not for the loaded source".into());
            return;
        }
        let output = PathBuf::from(&self.xml_export_path);
        if self.xml_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Name export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_name_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Name exported to {}", output.display());
                self.xml_edit_name = change.new_name;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track Name export failed: {error}"));
            }
        }
    }

    fn theme_selection_ui(&mut self, ui: &mut egui::Ui) -> (egui::Rect, Option<[egui::Rect; 3]>) {
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

impl eframe::App for LibraryApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.theme.save(self.custom_tokens, storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_theme_shortcuts(ctx);
        self.theme.apply(self.custom_tokens, ctx);
        let tokens = self.active_tokens();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.heading("OxiDJ · Local Library");
            ui.horizontal(|ui| {
                self.theme_selection_ui(ui);
                self.show_control_gallery_button(ui);
                if self.theme == ThemeMode::Custom {
                    if ui.button("Edit palette…").clicked() {
                        self.show_theme_editor = true;
                    }
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
            ui.horizontal_wrapped(|ui| {
                ui.label("Catalog:");
                let full_path = self.db_path.display().to_string();
                let path_width = (ui.available_width() - 68.0).clamp(80.0, 360.0);
                ui.add_sized(
                    [path_width, 20.0],
                    egui::Label::new(egui::RichText::new(&full_path).monospace()).truncate(),
                )
                .on_hover_text(&full_path);
                if ui.button("Reload").clicked() {
                    self.reload();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Search");
                ui.add_sized(
                    [ui.available_width().clamp(100.0, 160.0), 24.0],
                    egui::TextEdit::singleline(&mut self.search).hint_text("Filter tracks"),
                );
                ui.label("Import folder");
                ui.add_sized(
                    [ui.available_width().clamp(120.0, 200.0), 24.0],
                    egui::TextEdit::singleline(&mut self.import_root).hint_text("Folder path"),
                );
                if (PrimaryActionButton {
                    label: "Import references",
                })
                .show(ui, tokens)
                .clicked()
                {
                    self.import();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Rekordbox XML (read-only)");
                let source_changed = ui
                    .add_sized(
                        [ui.available_width().clamp(140.0, 360.0), 24.0],
                        egui::TextEdit::singleline(&mut self.xml_path).hint_text("XML file path"),
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
            });
            if !self.notice.is_empty() {
                let kind = if self.notice.starts_with("Import failed:")
                    || self.notice.starts_with("Track Name export failed:")
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
        });
        self.show_control_gallery_window(ctx, tokens);
        if self.show_xml_preview {
            let mut open = self.show_xml_preview;
            egui::Window::new("Rekordbox XML preview · guarded Name export")
                .open(&mut open)
                .default_width(720.0)
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
                            if ui.button("Cancel sibling reorder").clicked() { self.xml_sibling_pending = None; }
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
        egui::SidePanel::left("library-navigation")
            .default_width(176.0)
            .resizable(true)
            .show(ctx, |ui| {
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
                ui.separator();
                ui.small("LOCAL FILTERS");
                let available = self
                    .tracks
                    .iter()
                    .filter(|track| track.availability == TrackAvailability::Available)
                    .count();
                let missing = self.tracks.len().saturating_sub(available);
                if ui
                    .selectable_label(
                        self.availability_filter == Some(TrackAvailability::Available),
                        format!("Available · {available}"),
                    )
                    .clicked()
                {
                    self.availability_filter = Some(TrackAvailability::Available);
                }
                if ui
                    .selectable_label(
                        self.availability_filter == Some(TrackAvailability::Missing),
                        format!("Missing · {missing}"),
                    )
                    .clicked()
                {
                    self.availability_filter = Some(TrackAvailability::Missing);
                }
            });
        egui::TopBottomPanel::bottom("demo-decks")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.small("LOCAL DEMO · NO AUDIO CONNECTED");
                    for deck in 0..2 {
                        ui.separator();
                        ui.strong(format!("DECK {}", if deck == 0 { "A" } else { "B" }));
                        ui.label(
                            self.selected
                                .and_then(|id| {
                                    self.tracks
                                        .iter()
                                        .find(|track| track.track.id.as_u64() == id)
                                })
                                .map(|track| {
                                    PathBuf::from(&track.track.path)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                })
                                .unwrap_or_else(|| "No track selected".into()),
                        );
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
            });
        egui::SidePanel::right("details")
            .default_width(300.0)
            .show(ctx, |ui| {
                SectionHeading {
                    title: "Track details",
                }
                .show(ui, tokens);
                if let Some(id) = self.selected {
                    if let Some(item) = self.tracks.iter().find(|x| x.track.id.as_u64() == id) {
                        ui.label(format!("ID: {id}"));
                        ui.label(format!(
                            "Filename: {}",
                            PathBuf::from(&item.track.path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                        ui.label(format!("Path: {}", item.track.path));
                        ui.label(format!(
                            "Duration: {}",
                            item.duration_seconds
                                .map(|v| format!("{v:.3} s"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Sample rate: {}",
                            item.sample_rate
                                .map(|v| format!("{v} Hz"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Channels: {}",
                            item.channels
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!(
                            "Bit depth: {}",
                            item.bit_depth
                                .map(|v| format!("{v} bit"))
                                .unwrap_or_else(|| "Unavailable".into())
                        ));
                        ui.label(format!("Size: {} bytes", item.track.byte_length));
                        let (availability, kind) = match item.availability {
                            TrackAvailability::Available => ("Available", StatusKind::Success),
                            TrackAvailability::Missing => ("Missing", StatusKind::Warning),
                        };
                        StatusBadge {
                            label: &format!("Availability: {availability}"),
                            kind,
                        }
                        .show(ui, tokens);
                        ui.monospace(format!(
                            "SHA-256: {}",
                            item.track
                                .sha256
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ));
                    } else {
                        ui.label("Selected track is no longer in the catalog.");
                    }
                } else {
                    ui.label("Select a track to inspect its details.");
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
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
                ui.label(format!("{} tracks", self.tracks.len()));
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
                    ui.label(if self.tracks.is_empty() {
                        "Library is empty. Import a folder of supported audio references."
                    } else {
                        "No tracks match this search."
                    })
                });
                return;
            }
            SectionHeading { title: "Tracks" }.show(ui, tokens);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("tracks").striped(true).show(ui, |ui| {
                    ui.strong("ID");
                    ui.strong("Filename / path");
                    ui.strong("Duration");
                    ui.strong("Format facts");
                    ui.strong("Availability");
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
                            let name = PathBuf::from(&item.track.path)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if ui
                                .selectable_label(self.selected == Some(id), name)
                                .clicked()
                            {
                                self.selected = Some(id);
                            }
                            ui.small(&item.track.path);
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
                        let (availability, kind) = match item.availability {
                            TrackAvailability::Available => ("Available", StatusKind::Success),
                            TrackAvailability::Missing => ("Missing", StatusKind::Warning),
                        };
                        StatusBadge {
                            label: availability,
                            kind,
                        }
                        .show(ui, tokens);
                        ui.end_row();
                    }
                });
            });
        });
    }
}
fn edit_color(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_srgba(color);
    });
}

fn color_hex(color: egui::Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn parse_hex_color(value: &str) -> Option<egui::Color32> {
    let digits = value.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    Some(egui::Color32::from_rgb(
        u8::from_str_radix(&digits[0..2], 16).ok()?,
        u8::from_str_radix(&digits[2..4], 16).ok()?,
        u8::from_str_radix(&digits[4..6], 16).ok()?,
    ))
}

#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_member_insert_oracle.rs"]
mod f02_playlist_member_insert_oracle;

mod keyboard_tests {
    use super::*;

    fn app() -> LibraryApp {
        LibraryApp::new(
            PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        )
    }

    #[test]
    fn sibling_playlist_reorder_ui_export_bytes_pass_existing_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-sibling-order-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_sibling_parent = Some(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .folders
                .iter()
                .find(|folder| folder.name == "Sets")
                .unwrap()
                .ordinal,
        );
        app.xml_sibling_order = "Techno, House".into();
        app.prepare_xml_sibling_playlist_reorder();
        assert_eq!(
            app.xml_sibling_pending.as_ref().unwrap().expected_order,
            ["House", "Techno"]
        );
        app.xml_sibling_export_path = root.join("reordered.xml").display().to_string();
        app.export_xml_sibling_playlist_reorder();
        let bytes = std::fs::read(root.join("reordered.xml")).unwrap();
        f02_sibling_playlist_reorder_oracle::verify(&bytes).unwrap();
        let mutated = String::from_utf8(bytes)
            .unwrap()
            .replace("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"");
        assert!(f02_sibling_playlist_reorder_oracle::verify(mutated.as_bytes()).is_err());
        assert_eq!(
            std::fs::read(source).unwrap(),
            include_bytes!("../../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn control_concepts_toolbar_button_opens_gallery_on_next_frame() {
        let mut app = app();
        let ctx = egui::Context::default();
        let render = |ctx: &egui::Context, app: &mut LibraryApp, input| {
            let mut button_rect = None;
            let output = ctx.run(input, |ctx| {
                egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                    button_rect = Some(app.show_control_gallery_button(ui).rect);
                });
                app.show_control_gallery_window(ctx, ThemeMode::TokyoNight.tokens());
            });
            (button_rect, output)
        };
        let (button_rect, before_click) = render(&ctx, &mut app, egui::RawInput::default());
        let gallery_text = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(
            !gallery_text(&before_click)
                .iter()
                .any(|text| text == "OxiDJ · Control concept gallery")
        );
        let rect = button_rect.expect("toolbar action rendered");
        let pointer = |pressed| egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let _ = render(&ctx, &mut app, pointer(true));
        let _ = render(&ctx, &mut app, pointer(false));
        assert!(app.show_control_gallery);
        let (_, output) = render(&ctx, &mut app, egui::RawInput::default());
        assert!(app.show_control_gallery);
        let rendered_text = gallery_text(&output);
        assert!(
            rendered_text
                .iter()
                .any(|text| text == "OxiDJ · Control concept gallery"),
            "gallery window title was not rendered: {rendered_text:?}"
        );
        assert!(
            rendered_text.iter().any(|text| text == "Studio rack"),
            "gallery content was not rendered: {rendered_text:?}"
        );
    }

    fn temporary_xml() -> (PathBuf, PathBuf) {
        use std::time::SystemTime;
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-name-edit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("library.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin"),
        )
        .unwrap();
        (root, source)
    }

    #[test]
    fn playlist_rename_ui_exports_prepared_bytes_and_reopens_with_memberships_unchanged() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-playlist-rename/source.xml"),
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let output = root.join("renamed.xml");
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_playlist(0);
        let current = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .playlists[0]
            .name
            .clone();
        app.xml_playlist_rename_name = "Summer & Sun".into();
        app.prepare_xml_playlist_rename(0, &current);
        let expected = app
            .xml_pending_rename
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        app.cancel_xml_playlist_rename();
        assert!(app.xml_pending_rename.is_none());
        app.prepare_xml_playlist_rename(0, &current);
        app.xml_rename_export_path = output.display().to_string();
        app.export_xml_playlist_rename();
        let actual = std::fs::read(&output).unwrap();
        assert_eq!(actual, expected);
        f02_playlist_rename_oracle::check(&actual)
            .expect("actual UI bytes satisfy independent F-02 oracle");
        let text = String::from_utf8(actual.clone()).unwrap();
        let mutated = text.replacen("Summer &amp; Sun", "Tampered", 1);
        assert!(
            f02_playlist_rename_oracle::check(mutated.as_bytes()).is_err(),
            "oracle rejects mutation of actual UI output"
        );
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[0].name, "Summer & Sun");
        assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_export_writes_exact_prepared_bytes_and_preserves_source_and_catalog() {
        let (root, source) = temporary_xml();
        let output = root.join("edited.xml");
        let original = std::fs::read(&source).unwrap();
        let catalog_path = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog_path).unwrap());
        let catalog_before = std::fs::read(&catalog_path).unwrap();
        let mut app = LibraryApp::new(
            catalog_path.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_edit_name = "New < title".into();
        app.prepare_xml_name_edit("100", "A");
        let previewed_bytes = app
            .xml_pending_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.xml_export_path = output.display().to_string();

        app.export_xml_name_edit();

        let exported_bytes = std::fs::read(&output).unwrap();
        assert_eq!(exported_bytes, previewed_bytes);
        f02_track_name_oracle_support::check(&exported_bytes)
            .expect("independent F-02 oracle accepts only the selected Name change");
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .name,
            "New < title",
            "reopened output exposes the exported Name"
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(
            std::fs::read(&catalog_path).unwrap(),
            catalog_before,
            "XML export changed catalog bytes"
        );
        assert!(app.xml_pending_edit.is_none());
        assert!(app.notice.contains(&output.display().to_string()));
        assert!(app.xml_edit_error.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_clear_preview_cancel_export_reopen_and_oracle_are_source_bound() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-clear-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-clear/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_playlist(0);
        app.prepare_xml_playlist_clear(0);
        let prepared = app
            .xml_pending_clear
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "preparation is read-only"
        );
        let cancelled = root.join("cancelled.xml");
        app.xml_clear_export_path = cancelled.display().to_string();
        app.cancel_xml_playlist_clear();
        assert!(app.xml_pending_clear.is_none(), "cancel clears proposal");
        assert!(!cancelled.exists(), "cancel writes nothing");
        app.prepare_xml_playlist_clear(0);
        let output = root.join("cleared.xml");
        app.xml_clear_export_path = output.display().to_string();
        app.select_xml_track("200", "B");
        assert!(
            app.xml_pending_clear.is_none(),
            "track selection invalidates pending clear"
        );
        app.export_xml_playlist_clear();
        assert!(!output.exists(), "invalidated proposal cannot export");
        app.select_xml_playlist(0);
        app.prepare_xml_playlist_clear(0);
        app.xml_clear_export_path = output.display().to_string();
        app.export_xml_playlist_clear();
        let exported = std::fs::read(&output).unwrap();
        assert_eq!(exported, prepared);
        f02_playlist_clear_oracle::check(&exported)
            .expect("UI-exported bytes satisfy independent F-02 oracle");
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[0].name, "House");
        assert!(reopened.playlists[0].tracks.is_empty());
        assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
        assert_eq!(
            reopened
                .tracks
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["100", "200", "300", "400"]
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        assert!(
            app.notice
                .contains("collection tracks and other playlists are unchanged")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_clear_refuses_changed_source_order_invalid_selection_and_existing_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-clear-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-clear/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.prepare_xml_playlist_clear(99);
        assert!(app.xml_pending_clear.is_none());
        app.prepare_xml_playlist_clear(2);
        assert!(
            app.xml_pending_clear.is_none(),
            "empty playlist is rejected"
        );
        app.select_xml_playlist(0);
        app.prepare_xml_playlist_clear(0);
        if let Some(Ok(preview)) = app.xml_preview.as_mut() {
            preview.playlists[0].tracks.swap(0, 1);
        }
        app.xml_clear_export_path = root.join("changed-order.xml").display().to_string();
        app.export_xml_playlist_clear();
        assert!(!PathBuf::from(&app.xml_clear_export_path).exists());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("membership order changed")
        );
        app.preview_xml();
        app.select_xml_playlist(0);
        app.prepare_xml_playlist_clear(0);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_clear_export_path = existing.display().to_string();
        app.export_xml_playlist_clear();
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        app.prepare_xml_playlist_clear(0);
        std::fs::write(&source, b"changed source").unwrap();
        let stale_output = root.join("stale.xml");
        app.xml_clear_export_path = stale_output.display().to_string();
        app.export_xml_playlist_clear();
        assert!(!stale_output.exists());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("source changed")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn whole_playlist_delete_preview_cancel_export_and_oracle_are_source_bound() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-delete-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-delete/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let mut app = LibraryApp::new(
            catalog,
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_playlist(1);
        app.prepare_xml_playlist_delete(1);
        let prepared = app
            .xml_pending_playlist_delete
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        let prepared_export = app
            .xml_pending_playlist_delete
            .as_ref()
            .unwrap()
            .prepared
            .clone();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let stale_output = root.join("stale-source.xml");
        std::fs::write(&source, b"changed source").unwrap();
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_delete(
                &prepared_export,
                &stale_output
            )
            .is_err()
        );
        assert!(!stale_output.exists());
        std::fs::write(&source, &original).unwrap();
        app.select_xml_playlist(0);
        assert!(
            app.xml_pending_playlist_delete.is_none(),
            "selection change invalidates proposal"
        );
        app.select_xml_playlist(1);
        app.prepare_xml_playlist_delete(1);
        let cancelled = root.join("cancelled.xml");
        app.xml_playlist_delete_export_path = cancelled.display().to_string();
        app.cancel_xml_playlist_delete();
        assert!(!cancelled.exists());
        app.prepare_xml_playlist_delete(1);
        let preview_before = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let removed = &preview_before.playlists[1];
        let mut collision_preview = preview_before.clone();
        collision_preview.folders.push(oxidj_rekordbox::Folder {
            ordinal: 99,
            path: vec!["ROOT".into(), "Other".into(), "Sets".into()],
            name: "Sets".into(),
            count: Some("99".into()),
        });
        let parent = immediate_parent_folder(&collision_preview, removed).unwrap_or_else(|| {
            panic!(
                "playlist path {:?}, folders {:?}",
                removed.path, collision_preview.folders
            )
        });
        assert_eq!(parent.path, ["ROOT", "Sets"]);
        assert_eq!(parent.name, "Sets");
        assert_eq!(parent.count.as_deref(), Some("3"));
        assert_eq!(
            parent.count.as_deref().unwrap().parse::<usize>().unwrap() - 1,
            2
        );
        let output = root.join("deleted.xml");
        app.xml_playlist_delete_export_path = output.display().to_string();
        app.export_xml_playlist_delete();
        let exported = std::fs::read(&output).unwrap();
        assert_eq!(exported, prepared);
        f02_playlist_delete_oracle::check(&exported)
            .expect("actual UI-exported bytes pass independent F-02 oracle");
        assert_eq!(app.xml_document_source.as_ref(), Some(&output));
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(
            reopened
                .playlists
                .iter()
                .map(|playlist| playlist.name.as_str())
                .collect::<Vec<_>>(),
            ["House"]
        );
        assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
        let reopened_parent = reopened
            .folders
            .iter()
            .find(|folder| folder.name == "Sets")
            .unwrap();
        assert_eq!(reopened_parent.count.as_deref(), Some("2"));
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_delete(
                &prepared_export,
                &existing
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_removal_preview_cancel_export_and_oracle_are_source_bound() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-delete-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        app.prepare_xml_member_delete(0, 1);
        let prepared = app
            .xml_pending_delete
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let cancelled = root.join("cancelled.xml");
        app.xml_delete_export_path = cancelled.display().to_string();
        app.cancel_xml_member_delete();
        assert!(!cancelled.exists());
        app.prepare_xml_member_delete(0, 1);
        assert_eq!(
            app.xml_pending_delete.as_ref().unwrap().prepared.bytes(),
            prepared
        );
        let output = root.join("removed.xml");
        app.xml_delete_export_path = output.display().to_string();
        app.export_xml_member_delete();
        let exported = std::fs::read(&output).unwrap();
        assert_eq!(exported, prepared);
        f02_playlist_member_delete_oracle::check(&exported)
            .expect("actual UI-exported bytes pass independent F-02 oracle");
        let mut mutation = exported.clone();
        let index = mutation
            .windows(9)
            .position(|w| w == b"Key=\"100\"")
            .unwrap();
        mutation[index + 5] = b'9';
        assert!(
            f02_playlist_member_delete_oracle::check(&mutation).is_err(),
            "oracle rejects semantic mutation"
        );
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[0].tracks, ["100", "300"]);
        assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_removal_proposal_invalidation_clears_stale_preview() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-delete-invalidate-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();

        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        app.prepare_xml_member_delete(0, 1);
        assert!(app.xml_pending_delete.is_some());
        app.on_xml_source_changed();
        assert!(
            app.xml_pending_delete.is_none(),
            "source change clears proposal"
        );
        assert_eq!(app.xml_selected_playlist, None);
        assert_eq!(app.xml_selected_member, None);

        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        app.prepare_xml_member_delete(0, 1);
        assert!(app.xml_pending_delete.is_some());
        app.select_xml_playlist(1);
        assert!(
            app.xml_pending_delete.is_none(),
            "playlist selection clears proposal"
        );
        assert_eq!(app.xml_selected_playlist, Some(1));
        assert_eq!(app.xml_selected_member, None);

        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        app.prepare_xml_member_delete(0, 1);
        assert!(app.xml_pending_delete.is_some());
        app.select_xml_member(0);
        assert!(
            app.xml_pending_delete.is_none(),
            "member selection clears proposal"
        );
        assert_eq!(app.xml_selected_member, Some(0));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_removal_refuses_stale_source_membership_and_existing_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-delete-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-member-delete/source.xml"
        );
        std::fs::copy(fixture, &source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        if let Some(Ok(preview)) = app.xml_preview.as_mut() {
            preview.playlists[0].tracks[1] = "999".into();
        }
        app.prepare_xml_member_delete(0, 1);
        assert!(
            app.xml_pending_delete.is_none(),
            "stale membership accepted"
        );
        app.xml_preview =
            Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
        app.prepare_xml_member_delete(0, 99);
        assert!(app.xml_pending_delete.is_none());
        app.prepare_xml_member_delete(0, 1);
        let pending = app.xml_pending_delete.take().unwrap();
        let stale = root.join("stale.xml");
        std::fs::write(&source, b"stale source").unwrap();
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_member_delete(
                &pending.prepared,
                &stale
            )
            .is_err()
        );
        assert!(!stale.exists());
        std::fs::copy(fixture, &source).unwrap();
        app.xml_preview =
            Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
        app.prepare_xml_member_delete(0, 1);
        let pending = app.xml_pending_delete.take().unwrap();
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_member_delete(
                &pending.prepared,
                &existing
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_insertion_preview_cancel_export_and_oracle_are_source_bound() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-insert-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        let output = root.join("inserted.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(1);
        app.xml_selected_track = Some("100".into());
        app.xml_insert_position = 1;
        app.prepare_xml_member_insert(1, "100", 1);
        let expected = app
            .xml_pending_insert
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.cancel_xml_member_insert();
        assert!(app.xml_pending_insert.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(!output.exists(), "cancel wrote a destination");
        app.prepare_xml_member_insert(1, "100", 1);
        app.xml_insert_export_path = output.display().to_string();
        app.export_xml_member_insert();
        let exported = std::fs::read(&output).unwrap();
        assert_eq!(exported, expected);
        assert_eq!(
            super::f02_playlist_member_insert_oracle::SOURCE,
            original.as_slice()
        );
        super::f02_playlist_member_insert_oracle::check(&exported).unwrap();
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[1].tracks, ["200", "100", "400"]);
        assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_insertion_invalidates_changed_source_playlist_track_slot_and_order() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-insert-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(1);
        app.xml_selected_track = Some("100".into());
        app.prepare_xml_member_insert(1, "100", 1);
        assert!(app.xml_pending_insert.is_some());
        app.select_xml_playlist(0);
        assert!(
            app.xml_pending_insert.is_none(),
            "playlist change retained proposal"
        );
        app.select_xml_playlist(1);
        app.prepare_xml_member_insert(1, "100", 1);
        app.xml_selected_track = Some("300".into());
        app.xml_insert_export_path = root.join("track-change.xml").display().to_string();
        app.export_xml_member_insert();
        assert!(
            app.xml_pending_insert.is_none(),
            "track change retained proposal"
        );
        assert!(!root.join("track-change.xml").exists());
        app.xml_selected_track = Some("100".into());
        app.prepare_xml_member_insert(1, "100", 1);
        app.xml_insert_position = 0;
        app.xml_insert_export_path = root.join("slot-change.xml").display().to_string();
        app.export_xml_member_insert();
        assert!(
            app.xml_pending_insert.is_none(),
            "slot change retained proposal"
        );
        assert!(!root.join("slot-change.xml").exists());
        app.xml_insert_position = 1;
        app.prepare_xml_member_insert(1, "100", 1);
        if let Some(Ok(preview)) = app.xml_preview.as_mut() {
            preview.playlists[1].tracks.swap(0, 1);
        }
        app.xml_insert_export_path = root.join("order-change.xml").display().to_string();
        app.export_xml_member_insert();
        assert!(
            app.xml_pending_insert.is_none(),
            "order change retained proposal"
        );
        assert!(!root.join("order-change.xml").exists());
        app.prepare_xml_member_insert(1, "100", 1);
        app.on_xml_source_changed();
        assert!(
            app.xml_pending_insert.is_none(),
            "source change retained proposal"
        );
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
            ),
            &source,
        )
        .unwrap();
        app.preview_xml();
        app.xml_selected_playlist = Some(1);
        app.xml_selected_track = Some("100".into());
        app.prepare_xml_member_insert(1, "100", 1);
        let stale_output = root.join("stale.xml");
        std::fs::write(&source, b"changed after prepare").unwrap();
        app.xml_insert_export_path = stale_output.display().to_string();
        app.export_xml_member_insert();
        assert!(
            app.xml_pending_insert.is_some(),
            "failed revalidation should retain preview"
        );
        assert!(!stale_output.exists(), "stale source published output");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-insert/source.xml"
            ),
            &source,
        )
        .unwrap();
        app.preview_xml();
        app.xml_selected_playlist = Some(1);
        app.xml_selected_track = Some("100".into());
        app.prepare_xml_member_insert(1, "100", 1);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_insert_export_path = existing.display().to_string();
        app.export_xml_member_insert();
        assert_eq!(
            std::fs::read(&existing).unwrap(),
            b"keep",
            "existing output was overwritten"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_append_preview_cancel_and_export_are_source_bound() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-append-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_track = Some("100".into());
        app.xml_selected_playlist = Some(1);
        app.prepare_xml_member_append(1, "100");
        let prepared = app
            .xml_pending_append
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert!(app.xml_pending_append.is_some());
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "prepare must not write source"
        );
        let cancelled = root.join("cancelled.xml");
        app.xml_append_export_path = cancelled.display().to_string();
        app.cancel_xml_member_append();
        assert!(!cancelled.exists(), "cancel must not write output");
        app.prepare_xml_member_append(1, "100");
        assert_eq!(
            app.xml_pending_append.as_ref().unwrap().prepared.bytes(),
            prepared
        );
        let output = root.join("appended.xml");
        app.xml_append_export_path = output.display().to_string();
        app.export_xml_member_append();
        mod f02_oracle {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/common/f02_playlist_member_append_oracle_shared.rs"
            ));
        }
        assert_eq!(f02_oracle::SOURCE, original.as_slice());
        f02_oracle::check(&std::fs::read(&output).unwrap()).unwrap();
        let mutated = std::fs::read(&output)
            .unwrap()
            .windows(9)
            .position(|w| w == b"Key=\"100\"");
        assert!(mutated.is_some());
        let mut bad = std::fs::read(&output).unwrap();
        let at = mutated.unwrap();
        bad[at + 5] = b'9';
        assert!(
            f02_oracle::check(&bad).is_err(),
            "oracle must reject semantic mutation"
        );
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[1].tracks, ["200", "400", "100"]);
        assert_eq!(reopened.playlists[0].tracks, ["100", "200", "300"]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_append_refuses_stale_source_membership_duplicate_unknown_and_existing_output()
     {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-append-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(1);
        app.prepare_xml_member_append(1, "200");
        assert!(
            app.xml_pending_append.is_none(),
            "duplicate membership accepted"
        );
        app.prepare_xml_member_append(1, "999");
        assert!(app.xml_pending_append.is_none(), "unknown track accepted");
        if let Some(Ok(preview)) = app.xml_preview.as_mut() {
            preview.playlists[1].tracks = vec!["999".into()];
        }
        app.prepare_xml_member_append(1, "100");
        assert!(
            app.xml_pending_append.is_none(),
            "stale membership accepted"
        );
        app.xml_preview =
            Some(oxidj_application::preview_rekordbox_xml(&source).map_err(|e| e.to_string()));
        app.prepare_xml_member_append(1, "100");
        let pending = app.xml_pending_append.take().unwrap();
        std::fs::write(&source, b"stale source").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_member_append(
                &pending.prepared,
                &stale_output
            )
            .is_err()
        );
        assert!(!stale_output.exists());
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-append/source.xml"
            ),
            &source,
        )
        .unwrap();
        app.prepare_xml_member_append(1, "100");
        let pending = app.xml_pending_append.take().unwrap();
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(
            oxidj_application::export_prepared_rekordbox_playlist_member_append(
                &pending.prepared,
                &existing
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_reorder_preview_exports_exact_prepared_bytes_and_reopens_order() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-member-reorder-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-playlist-member-reorder/source.xml"
            ),
            &source,
        )
        .unwrap();
        let intermediate = root.join("intermediate.xml");
        let output = root.join("reordered.xml");
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(2);
        app.prepare_xml_member_reorder(0, 2, -1);
        let first_bytes = app
            .xml_pending_reorder
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(
            app.xml_pending_reorder.as_ref().unwrap().prepared.source(),
            source
        );
        app.xml_reorder_export_path = intermediate.display().to_string();
        app.export_xml_member_reorder();
        assert_eq!(std::fs::read(&intermediate).unwrap(), first_bytes);
        app.xml_document_source = Some(intermediate.clone());
        app.xml_preview = Some(
            oxidj_application::preview_rekordbox_xml(&intermediate).map_err(|e| e.to_string()),
        );
        app.xml_selected_playlist = Some(0);
        app.xml_selected_member = Some(1);
        app.prepare_xml_member_reorder(0, 1, -1);
        let expected = app
            .xml_pending_reorder
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.xml_reorder_export_path = output.display().to_string();
        app.export_xml_member_reorder();
        let exported = std::fs::read(&output).unwrap();
        assert_eq!(exported, expected);
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert_eq!(reopened.playlists[0].tracks, ["300", "100", "200"]);
        assert_eq!(reopened.playlists[1].tracks, ["200", "400"]);
        assert_eq!(
            std::fs::read(&source).unwrap(),
            include_bytes!("../../../xtask/tests/fixtures/f02-playlist-member-reorder/source.xml")
        );
        assert!(app.xml_pending_reorder.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_member_reorder_refuses_boundary_and_clears_pending_proposal() {
        let (root, source) = temporary_xml();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_selected_playlist = Some(0);
        app.prepare_xml_member_reorder(0, 1, 1);
        assert!(app.xml_pending_reorder.is_some());
        app.prepare_xml_member_reorder(0, 0, -1);
        assert!(app.xml_pending_reorder.is_none());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("boundary"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_refuses_stale_source_alias_existing_output_and_surfaces_write_failure() {
        let (root, source) = temporary_xml();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_edit_name = "New < title".into();
        app.prepare_xml_name_edit("100", "A");
        let prepared_bytes = app
            .xml_pending_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();

        app.xml_export_path = source.display().to_string();
        app.export_xml_name_edit();
        assert!(app.xml_pending_edit.is_some());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("same file"));
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep me").unwrap();
        app.xml_export_path = existing.display().to_string();
        app.export_xml_name_edit();
        assert!(app.xml_pending_edit.is_some());
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");

        std::fs::write(&source, b"stale source").unwrap();
        let stale_out = root.join("stale.xml");
        app.xml_export_path = stale_out.display().to_string();
        app.export_xml_name_edit();
        assert!(app.xml_pending_edit.is_some());
        assert!(!stale_out.exists());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("source changed")
        );
        assert_ne!(prepared_bytes, std::fs::read(&source).unwrap());

        std::fs::write(&source, &original).unwrap();
        let failed_out = root.join("missing-parent").join("failed.xml");
        app.xml_export_path = failed_out.display().to_string();
        app.export_xml_name_edit();
        assert!(app.xml_pending_edit.is_some());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .starts_with("Track Name export failed:")
        );
        assert!(
            app.notice.is_empty(),
            "failure must not acknowledge success"
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_track_name_edit_is_source_and_track_bound_and_cancelable() {
        let (root, source) = temporary_xml();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_path = root.join("typed-but-not-loaded.xml").display().to_string();
        app.xml_selected_track = Some("100".into());
        app.xml_edit_name = "Renamed".into();
        app.prepare_xml_name_edit("100", "A");
        let pending = app
            .xml_pending_edit
            .as_ref()
            .expect("prepared pending edit");
        assert_eq!(pending.source, source);
        assert_eq!(pending.track_id, "100");
        assert_eq!(pending.prepared.change().track_id, "100");
        assert_eq!(pending.prepared.change().old_name, "A");
        assert_eq!(pending.prepared.change().new_name, "Renamed");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        app.cancel_xml_name_edit();
        assert!(app.xml_pending_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        assert!(app.notice.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn previewing_another_document_clears_pending_edit() {
        let (root, source) = temporary_xml();
        let second = root.join("other.xml");
        std::fs::write(&second, std::fs::read(&source).unwrap()).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.xml_edit_name = "New".into();
        app.prepare_xml_name_edit("100", "A");
        assert!(app.xml_pending_edit.is_some());
        app.xml_path = second.display().to_string();
        app.preview_xml();
        assert!(app.xml_pending_edit.is_none());
        assert_eq!(app.xml_selected_track, None);
        assert!(app.xml_edit_error.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_source_guard_is_shown_as_preparation_error() {
        let (root, source) = temporary_xml();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        std::fs::write(&source, br#"<DJ_PLAYLISTS Version="1.0.0"><COLLECTION Entries="1"><TRACK TrackID="7" Name="Changed" Location="x"/></COLLECTION></DJ_PLAYLISTS>"#).unwrap();
        app.xml_edit_name = "New".into();
        app.prepare_xml_name_edit("7", "Old");
        let error = app.xml_edit_error.as_ref().expect("guard refusal surfaced");
        assert!(
            error.starts_with("Track Name preparation refused:"),
            "{error}"
        );
        assert!(app.xml_pending_edit.is_none());
        let ctx = egui::Context::default();
        let tokens = ThemeMode::TokyoNight.tokens();
        let error = app.xml_edit_error.clone().unwrap();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                StatusBadge {
                    label: &error,
                    kind: StatusKind::Error,
                }
                .show(ui, tokens);
            });
        });
        let labels = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            labels
                .iter()
                .any(|label| label.contains("Track Name preparation refused")),
            "{labels:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn toolbar_controls_fit_narrow_and_wide_viewports() {
        for width in [800.0, 1440.0] {
            let ctx = egui::Context::default();
            let mut app = app();
            let mut rects = Vec::new();
            let _ = ctx.run(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0))),
                ..Default::default()
            }, |ctx| {
                egui::TopBottomPanel::top("test-toolbar").show(ctx, |ui| {
                    let (theme_rect, _) = app.theme_selection_ui(ui);
                    rects.push(theme_rect);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Catalog:");
                        let path = "/a/very/long/catalog/path/that/must/not/push/actions/offscreen/catalog.sqlite";
                        let path_width = (ui.available_width() - 68.0).clamp(80.0, 360.0);
                        rects.push(ui.add_sized([path_width, 20.0], egui::Label::new(egui::RichText::new(path).monospace()).truncate()).on_hover_text(path).rect);
                        rects.push(ui.button("Reload").rect);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Search");
                        rects.push(ui.add_sized([ui.available_width().clamp(100.0, 160.0), 24.0], egui::TextEdit::singleline(&mut String::new()).hint_text("Filter tracks")).rect);
                        ui.label("Import folder");
                        rects.push(ui.add_sized([ui.available_width().clamp(120.0, 200.0), 24.0], egui::TextEdit::singleline(&mut String::new()).hint_text("Folder path")).rect);
                        rects.push(ui.button("Import references").rect);
                    });
                });
            });
            let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
            assert!(
                rects.iter().all(|rect| content.contains_rect(*rect)),
                "toolbar control clipped at {width}px: {rects:?}"
            );
            assert!(
                rects.windows(2).all(|pair| !pair[0].intersects(pair[1])),
                "toolbar controls overlap at {width}px: {rects:?}"
            );
        }
    }

    #[test]
    fn day_to_custom_combo_selection_initializes_tokens_and_opens_editor() {
        let mut app = app();
        app.theme = ThemeMode::TokyoDay;
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.custom_tokens.secondary = egui::Color32::from_rgb(4, 5, 6);
        let ctx = egui::Context::default();
        let render = |ctx: &egui::Context, app: &mut LibraryApp, input| {
            let mut result = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    result = Some(app.theme_selection_ui(ui));
                });
            });
            result.unwrap()
        };
        let (combo_rect, _) = render(&ctx, &mut app, egui::RawInput::default());
        let click = |pos| egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..Default::default()
        };
        let _ = render(&ctx, &mut app, click(combo_rect.center()));
        let (_, option_rects) = render(&ctx, &mut app, egui::RawInput::default());
        let custom_pos = option_rects.unwrap()[2].center();
        let _ = render(&ctx, &mut app, click(custom_pos));
        assert_eq!(app.theme, ThemeMode::Custom);
        assert!(app.show_theme_editor);
        assert_eq!(app.custom_tokens, ThemeMode::TokyoDay.tokens());
    }

    #[test]
    fn theme_controls_have_accesskit_names_and_control_roles() {
        let mut app = app();
        app.theme = ThemeMode::Custom;
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.theme_selection_ui(ui);
                app.show_palette_fields(ui, egui::Color32::RED);
            });
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit update");
        let nodes = update
            .nodes
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let assert_named = |name, role| {
            assert!(
                nodes.values().any(|control| {
                    control.role() == role
                        && control.labelled_by().iter().any(|label_id| {
                            nodes
                                .get(label_id)
                                .is_some_and(|label| label.value() == Some(name))
                        })
                }),
                "missing {role:?} labelled by {name:?}"
            );
        };
        assert_named("Theme (Ctrl+Alt+1/2/3)", egui::accesskit::Role::ComboBox);
        assert_named("Primary", egui::accesskit::Role::TextInput);
        assert_named("Secondary", egui::accesskit::Role::TextInput);
    }

    #[test]
    fn shortcut_raw_input_selects_custom_and_preserves_existing_custom_tokens() {
        let mut app = app();
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
        assert_eq!(app.theme, ThemeMode::Custom);
        assert!(app.show_theme_editor);
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.show_theme_editor = false;
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = ctx.run(input, |ctx| app.handle_theme_shortcuts(ctx));
        assert!(app.show_theme_editor);
        assert_eq!(app.custom_tokens.primary, egui::Color32::from_rgb(1, 2, 3));
    }

    #[test]
    fn raw_text_events_commit_field_and_storage_round_trip() {
        use std::collections::HashMap;
        #[derive(Default)]
        struct Store(HashMap<String, String>);
        impl eframe::Storage for Store {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.into(), value);
            }
            fn flush(&mut self) {}
        }
        let mut app = app();
        app.select_theme(ThemeMode::Custom);
        let ctx = egui::Context::default();
        let run_editor = |ctx: &egui::Context, app: &mut LibraryApp, input| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| app.show_palette_fields(ui, egui::Color32::RED));
            })
        };
        app.primary_hex.clear();
        let _ = run_editor(&ctx, &mut app, egui::RawInput::default());
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        input.events.push(egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: input.modifiers,
        });
        let _ = run_editor(&ctx, &mut app, input);
        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Text("#123456".into()));
        let _ = run_editor(&ctx, &mut app, input);
        assert_eq!(app.primary_hex, "#123456");
        let mut enter = egui::RawInput::default();
        enter.events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });
        let _ = run_editor(&ctx, &mut app, enter);
        assert_eq!(
            app.custom_tokens.primary,
            egui::Color32::from_rgb(0x12, 0x34, 0x56)
        );
        assert_eq!(app.theme, ThemeMode::Custom);
        let mut store = Store::default();
        eframe::App::save(&mut app, &mut store);
        assert_eq!(
            ThemeMode::load(Some(&store)),
            (app.theme, app.custom_tokens)
        );

        // Invalid focused input is rejected on Enter and leaves the committed token intact.
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("primary-hex")));
        let mut invalid = egui::RawInput::default();
        invalid.events.extend([
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            },
            egui::Event::Text("#12".into()),
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            },
        ]);
        let _ = run_editor(&ctx, &mut app, invalid);
        assert_eq!(
            app.custom_tokens.primary,
            egui::Color32::from_rgb(0x12, 0x34, 0x56)
        );
        assert!(app.accent_error.is_some());
    }

    #[test]
    fn palette_swatches_and_reset_keep_text_synchronized() {
        let mut app = app();
        app.custom_tokens.primary = egui::Color32::from_rgb(1, 2, 3);
        app.sync_accent_fields();
        assert_eq!(app.primary_hex, "#010203");
        app.custom_tokens = ThemeMode::reset_custom_tokens(ThemeMode::TokyoDay);
        app.sync_accent_fields();
        assert_eq!(
            app.primary_hex,
            color_hex(ThemeMode::TokyoDay.tokens().primary)
        );
        assert_eq!(
            app.secondary_hex,
            color_hex(ThemeMode::TokyoDay.tokens().secondary)
        );
    }
    #[test]
    fn empty_folder_delete_ui_exports_prepared_bytes_and_reopens_without_stale_selection() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-delete-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-empty-folder-delete/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let mut index = preview
            .folders
            .iter()
            .position(|f| f.name == "Empty" && f.path == ["ROOT", "Sets", "Empty"])
            .unwrap();
        let mut same_named_other_parent = preview
            .folders
            .iter()
            .find(|f| f.path == ["ROOT", "Sets"])
            .unwrap()
            .clone();
        same_named_other_parent.path = vec!["ROOT".into(), "Other".into(), "Sets".into()];
        same_named_other_parent.count = Some("99".into());
        app.xml_preview
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .folders
            .insert(0, same_named_other_parent);
        index += 1;
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        assert!(
            app.xml_pending_folder_delete.is_some(),
            "prepare error: {:?}",
            app.xml_edit_error
        );
        let bytes = app
            .xml_pending_folder_delete
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.cancel_xml_folder_delete();
        assert!(app.xml_pending_folder_delete.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        app.xml_folder_delete_export_path = source.display().to_string();
        app.export_xml_folder_delete();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "source alias must not overwrite input"
        );
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        let changed_source = root.join("changed.xml");
        app.xml_folder_delete_export_path = changed_source.display().to_string();
        let mut changed = original.clone();
        changed.push(b' ');
        std::fs::write(&source, &changed).unwrap();
        app.export_xml_folder_delete();
        assert!(
            !changed_source.exists(),
            "source revalidation rejects stale preparation"
        );
        assert!(
            app.xml_pending_folder_delete.is_none(),
            "stale source invalidates the pending proposal"
        );
        assert_eq!(std::fs::read(&source).unwrap(), changed);
        std::fs::write(&source, &original).unwrap();
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        app.xml_selected_folder_delete = Some(0);
        app.export_xml_folder_delete();
        assert!(
            app.xml_pending_folder_delete.is_none(),
            "changed selection invalidates"
        );
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        app.xml_folder_delete_export_path = existing.display().to_string();
        app.export_xml_folder_delete();
        assert_eq!(
            std::fs::read(&existing).unwrap(),
            b"keep",
            "existing destination is preserved"
        );
        app.xml_selected_folder_delete = Some(index);
        app.prepare_xml_folder_delete();
        let output = root.join("deleted.xml");
        app.xml_folder_delete_export_path = output.display().to_string();
        app.export_xml_folder_delete();
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
        f02_empty_folder_delete_oracle::check(&bytes)
            .expect("actual UI bytes satisfy independent F-02 oracle");
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert!(
            !reopened
                .folders
                .iter()
                .any(|f| f.path == ["ROOT", "Sets", "Empty"])
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.path == ["ROOT", "Sets"])
                .unwrap()
                .count
                .as_deref(),
            Some("2")
        );
        assert_eq!(app.xml_selected_folder_delete, None);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_document_source = Some(output.clone());
        app.xml_selected_folder_delete = Some(0);
        app.prepare_xml_folder_delete();
        app.xml_folder_delete_export_path = output.display().to_string();
        app.export_xml_folder_delete();
        assert_eq!(
            std::fs::read(&output).unwrap(),
            bytes,
            "source alias cannot overwrite"
        );
        assert_eq!(std::fs::read(existing).unwrap(), b"keep");
        let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
            "Name=\"Sets\" Count=\"2\"",
            "Name=\"Sets\" Count=\"3\"",
            1,
        );
        assert!(f02_empty_folder_delete_oracle::check(mutated.as_bytes()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_rename_ui_refuses_stale_and_invalid_exports_without_overwriting() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-rename-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-rename/source.xml"
        ))
        .unwrap();
        let with_sibling = String::from_utf8(fixture).unwrap().replace(
            "    </NODE>\n  </PLAYLISTS>",
            "      <NODE Type=\"0\" Name=\"Other\" Count=\"0\"/>\n    </NODE>\n  </PLAYLISTS>",
        );
        std::fs::write(&source, with_sibling).unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = LibraryApp::new(
            root.join("catalog.sqlite"),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let folders = &app.xml_preview.as_ref().unwrap().as_ref().unwrap().folders;
        let index = folders
            .iter()
            .position(|f| f.name == "Sets" && f.path.len() > 1)
            .unwrap();
        app.xml_selected_folder_rename = Some(index);
        app.xml_folder_rename_name = "Renamed".into();
        app.prepare_xml_folder_rename();
        app.xml_selected_folder_rename = Some(index + 1);
        app.xml_folder_rename_export_path = root.join("selection.xml").display().to_string();
        app.export_xml_folder_rename();
        assert!(app.xml_pending_folder_rename.is_none());
        assert!(!root.join("selection.xml").exists());

        for (selection, name) in [
            (Some(0), "Renamed"),
            (Some(usize::MAX), "Renamed"),
            (Some(index), ""),
            (Some(index), "bad\0name"),
            (Some(index), "Other"),
        ] {
            app.xml_selected_folder_rename = selection;
            app.xml_folder_rename_name = name.into();
            app.prepare_xml_folder_rename();
            assert!(
                app.xml_pending_folder_rename.is_none(),
                "selection={selection:?}, name={name:?}"
            );
        }
        app.xml_selected_folder_rename = Some(index);
        app.xml_folder_rename_name = "Renamed".into();
        app.prepare_xml_folder_rename();
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve me").unwrap();
        app.xml_folder_rename_export_path = existing.display().to_string();
        app.export_xml_folder_rename();
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve me");

        app.xml_selected_folder_rename = Some(index);
        app.prepare_xml_folder_rename();
        app.xml_folder_rename_export_path = source.display().to_string();
        app.export_xml_folder_rename();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "source alias preserved"
        );

        app.xml_folder_rename_export_path = root.join("stale.xml").display().to_string();
        app.prepare_xml_folder_rename();
        let mut changed = original.clone();
        changed.push(b' ');
        std::fs::write(&source, &changed).unwrap();
        app.export_xml_folder_rename();
        assert!(!root.join("stale.xml").exists());
        assert_eq!(std::fs::read(&source).unwrap(), changed);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_rename_ui_prepares_exports_reopens_and_runs_independent_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-rename-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-folder-rename/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|f| f.name == "Sets" && f.path.len() > 1)
            .unwrap();
        app.xml_selected_folder_rename = Some(index);
        app.xml_folder_rename_name = "Renamed".into();
        app.prepare_xml_folder_rename();
        let prepared = app
            .xml_pending_folder_rename
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.xml_pending_folder_rename = None;
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "preview/cancel are write-free"
        );
        app.prepare_xml_folder_rename();
        app.xml_folder_rename_name.push('x');
        app.export_xml_folder_rename();
        assert!(
            app.xml_pending_folder_rename.is_none(),
            "name changes invalidate proposal"
        );
        app.xml_folder_rename_name = "Renamed".into();
        app.prepare_xml_folder_rename();
        let output = root.join("renamed.xml");
        app.xml_folder_rename_export_path = output.display().to_string();
        app.export_xml_folder_rename();
        let exported_bytes = std::fs::read(&output).unwrap();
        assert_eq!(exported_bytes, prepared);
        f02_folder_rename_oracle::check_ui_export_mutation_rejected(&exported_bytes, "Renamed");
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert!(reopened.folders.iter().any(|f| f.name == "Renamed"));
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(catalog).unwrap(), catalog_before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_folder_create_ui_exports_prepared_bytes_and_reopens_folder() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-create-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-empty-folder-create/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        drop(Catalog::open(&catalog).unwrap());
        let catalog_before = std::fs::read(&catalog).unwrap();
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let parent_index = preview
            .folders
            .iter()
            .position(|f| f.name == "Sets" && f.path.len() > 1)
            .unwrap();
        app.xml_selected_parent = Some(parent_index);
        app.xml_folder_create_name = "Practice".into();
        app.prepare_xml_folder_create();
        assert!(
            app.xml_pending_folder_create.is_some(),
            "preparation: {:?}",
            app.xml_edit_error
        );
        let prepared = app
            .xml_pending_folder_create
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.cancel_xml_folder_create();
        assert!(app.xml_pending_folder_create.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);

        app.xml_selected_parent = Some(parent_index);
        app.prepare_xml_folder_create();
        app.xml_folder_create_name.push('x');
        app.export_xml_folder_create();
        assert!(
            app.xml_pending_folder_create.is_none(),
            "name change invalidates proposal"
        );
        app.xml_folder_create_name = "Practice".into();
        app.prepare_xml_folder_create();
        let output = root.join("created.xml");
        app.xml_folder_create_export_path = output.display().to_string();
        app.export_xml_folder_create();
        assert_eq!(std::fs::read(&output).unwrap(), prepared);
        f02_empty_folder_create_oracle::check(&prepared)
            .expect("actual UI bytes satisfy independent F-02 oracle");
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        let folder = reopened
            .folders
            .iter()
            .find(|f| f.name == "Practice")
            .unwrap();
        assert_eq!(folder.path, vec!["ROOT", "Sets", "Practice"]);
        assert_eq!(folder.count.as_deref(), Some("0"));
        let parent = reopened
            .folders
            .iter()
            .find(|f| f.path == vec!["ROOT", "Sets"])
            .unwrap();
        assert_eq!(parent.count.as_deref(), Some("3"));
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let mut mutated = String::from_utf8(prepared.clone()).unwrap();
        mutated = mutated.replacen("Name=\"Practice\"", "Name=\"Other\"", 1);
        assert!(f02_empty_folder_create_oracle::check(mutated.as_bytes()).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_folder_create_name = "Another".into();
        app.xml_selected_parent = Some(parent_index);
        app.prepare_xml_folder_create();
        app.xml_folder_create_export_path = existing.display().to_string();
        app.export_xml_folder_create();
        assert_eq!(std::fs::read(existing).unwrap(), b"keep");
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_playlist_create_ui_exports_prepared_bytes_and_reopens_selectable_playlist() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-create-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-empty-playlist-create/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let parent_index = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .position(|folder| folder.name == "Sets" && folder.path.len() > 1)
            .unwrap();
        app.xml_selected_parent = Some(parent_index);
        app.xml_create_name = "Practice".into();
        app.prepare_xml_playlist_create();
        assert!(
            app.xml_pending_create.is_some(),
            "preparation failed: {:?}",
            app.xml_edit_error
        );
        let prepared = app
            .xml_pending_create
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        app.cancel_xml_playlist_create();
        assert!(
            std::fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .all(|e| e.file_name() != "cancelled.xml")
        );
        app.xml_selected_parent = Some(parent_index);
        app.prepare_xml_playlist_create();
        let output = root.join("created.xml");
        app.xml_create_export_path = output.display().to_string();
        app.export_xml_playlist_create();
        let actual = std::fs::read(&output).unwrap();
        assert_eq!(actual, prepared);
        f02_empty_playlist_create_oracle::check(&actual)
            .expect("UI bytes satisfy independent F-02 oracle");
        let mutated = String::from_utf8(actual.clone()).unwrap().replacen(
            "Name=\"Practice\"",
            "Name=\"Changed\"",
            1,
        );
        assert!(
            f02_empty_playlist_create_oracle::check(mutated.as_bytes()).is_err(),
            "oracle rejects mutation of actual UI output"
        );
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert!(
            reopened
                .playlists
                .iter()
                .any(|p| p.name == "Practice" && p.tracks.is_empty())
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        app.xml_selected_parent = Some(parent_index);
        app.xml_create_name = "Another".into();
        app.prepare_xml_playlist_create();
        app.xml_create_export_path = existing.display().to_string();
        app.export_xml_playlist_create();
        assert_eq!(std::fs::read(existing).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let db = match (args.next().as_deref(), args.next()) {
        (Some("--db"), Some(path)) => PathBuf::from(path),
        _ => PathBuf::from("oxidj-library.sqlite"),
    };
    eframe::run_native(
        "OxiDJ Local Library",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            cc.egui_ctx.enable_accesskit();
            let (theme, custom_tokens) = ThemeMode::load(cc.storage);
            Ok(Box::new(LibraryApp::new(db, theme, custom_tokens)))
        }),
    )
}
