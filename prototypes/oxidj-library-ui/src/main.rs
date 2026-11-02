use eframe::egui;
use oxidj_library::{Catalog, TrackAvailability, TrackDetails};
use std::path::PathBuf;

fn direct_xml_typed_child_order(
    source: &std::path::Path,
    wanted: &[String],
) -> Result<Vec<String>, String> {
    use quick_xml::{Reader, events::Event};
    let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    let mut buf = Vec::new();
    let mut folders = Vec::<String>::new();
    let mut frames = Vec::<bool>::new();
    let mut result = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let (mut kind, mut name) = (String::new(), String::new());
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = value,
                        b"Name" => name = value,
                        _ => {}
                    }
                }
                if folders == wanted {
                    result.push(format!("{kind}:{name}"));
                }
                let nested = kind == "0" && !frames.is_empty();
                if nested {
                    folders.push(name);
                }
                frames.push(nested);
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let (mut kind, mut name) = (String::new(), String::new());
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = value,
                        b"Name" => name = value,
                        _ => {}
                    }
                }
                if folders == wanted {
                    result.push(format!("{kind}:{name}"));
                }
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if frames.pop().unwrap_or(false) {
                    folders.pop();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

fn direct_xml_node_order(
    source: &std::path::Path,
    wanted: &[String],
) -> Result<Vec<String>, String> {
    use quick_xml::{Reader, events::Event};
    let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    let mut buf = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    let mut folder_frames: Vec<bool> = Vec::new();
    let mut result = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) if e.name().as_ref() == b"NODE" => {
                let mut kind = None;
                let mut name = None;
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = Some(value),
                        b"Name" => name = Some(value),
                        _ => {}
                    }
                }
                let kind = kind.ok_or("NODE missing Type")?;
                let name = name.ok_or("NODE missing Name")?;
                if folders == wanted {
                    result.push(name.clone());
                }
                let is_folder = kind == "0";
                if is_folder && !folder_frames.is_empty() {
                    folders.push(name);
                }
                folder_frames.push(is_folder && !folder_frames.is_empty());
            }
            Event::Empty(e) if e.name().as_ref() == b"NODE" => {
                let mut kind = None;
                let mut name = None;
                for attr in e.attributes().with_checks(false) {
                    let attr = attr.map_err(|e| e.to_string())?;
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"Type" => kind = Some(value),
                        b"Name" => name = Some(value),
                        _ => {}
                    }
                }
                let name = name.ok_or("NODE missing Name")?;
                if folders == wanted {
                    result.push(name);
                }
                let _ = kind;
            }
            Event::End(e) if e.name().as_ref() == b"NODE" => {
                if folder_frames.pop().unwrap_or(false) {
                    folders.pop();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

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
#[path = "../../../xtask/tests/f02_folder_move_oracle.rs"]
mod f02_folder_move_oracle;
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
#[path = "../../../xtask/tests/f02_playlist_move_oracle.rs"]
mod f02_playlist_move_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_playlist_rename_oracle.rs"]
mod f02_playlist_rename_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_sibling_folder_reorder_oracle.rs"]
mod f02_sibling_folder_reorder_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_sibling_playlist_reorder_oracle.rs"]
mod f02_sibling_playlist_reorder_oracle;
#[path = "../../../xtask/tests/f02_track_album_edit_oracle.rs"]
mod f02_track_album_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_artist_edit_oracle.rs"]
mod f02_track_artist_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_comments_edit_oracle.rs"]
mod f02_track_comments_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_label_edit_oracle.rs"]
mod f02_track_label_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_composer_edit_oracle.rs"]
mod f02_track_composer_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_grouping_edit_oracle.rs"]
mod f02_track_grouping_edit_oracle;
#[cfg(test)]
#[path = "../../../xtask/tests/f02_track_genre_edit_oracle.rs"]
mod f02_track_genre_edit_oracle;
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
struct PendingTrackArtistEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackArtistExport,
}
#[derive(Debug, Clone)]
struct PendingTrackCommentsEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackCommentsExport,
}
#[derive(Debug, Clone)]
struct PendingTrackAlbumEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackAlbumExport,
}
#[derive(Debug, Clone)]
struct PendingTrackComposerEdit { source: PathBuf, track_id: String, prepared: oxidj_application::PreparedTrackComposerExport }
#[derive(Debug, Clone)]
struct PendingTrackRemixerEdit { source: PathBuf, track_id: String, prepared: oxidj_application::PreparedTrackRemixerExport }
#[derive(Debug, Clone)]
struct PendingTrackGroupingEdit { source: PathBuf, track_id: String, prepared: oxidj_application::PreparedTrackGroupingExport }
#[derive(Debug, Clone)]
struct PendingTrackLabelEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackLabelExport,
}

#[derive(Debug, Clone)]
struct PendingTrackGenreEdit {
    source: PathBuf,
    track_id: String,
    prepared: oxidj_application::PreparedTrackGenreExport,
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
struct PendingSiblingFolderReorder {
    source: PathBuf,
    parent_index: usize,
    parent_name: String,
    parent_path: Vec<String>,
    expected_order: Vec<String>,
    new_order: Vec<String>,
    prepared: oxidj_application::PreparedSiblingFolderReorderExport,
}
#[derive(Debug, Clone)]
struct PendingFolderMove {
    source: PathBuf,
    folder_index: usize,
    folder_name: String,
    old_path: Vec<String>,
    new_path: Vec<String>,
    source_parent_index: usize,
    source_parent_name: String,
    source_parent_path: Vec<String>,
    target_parent_index: usize,
    target_parent_name: String,
    target_parent_path: Vec<String>,
    expected_source_order: Vec<String>,
    expected_target_order: Vec<String>,
    source_count: Option<String>,
    target_count: Option<String>,
    prepared: oxidj_application::PreparedFolderMoveExport,
}
#[derive(Debug, Clone)]
struct PendingPlaylistMove {
    source: PathBuf,
    playlist_index: usize,
    source_parent_index: usize,
    source_parent_name: String,
    source_parent_path: Vec<String>,
    destination_parent_index: usize,
    destination_parent_name: String,
    destination_parent_path: Vec<String>,
    expected_members: Vec<String>,
    expected_source_order: Vec<String>,
    expected_destination_order: Vec<String>,
    old_path: Vec<String>,
    new_path: Vec<String>,
    source_count: Option<String>,
    destination_count: Option<String>,
    prepared: oxidj_application::PreparedPlaylistMoveExport,
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
    xml_edit_artist: String,
    xml_pending_artist_edit: Option<PendingTrackArtistEdit>,
    xml_artist_export_path: String,
    xml_edit_comments: String,
    xml_pending_comments_edit: Option<PendingTrackCommentsEdit>,
    xml_comments_export_path: String,
    xml_edit_album: String,
    xml_pending_album_edit: Option<PendingTrackAlbumEdit>,
    xml_album_export_path: String,
    xml_edit_genre: String,
    xml_pending_genre_edit: Option<PendingTrackGenreEdit>,
    xml_edit_composer: String,
    xml_composer_export_path: String,
    xml_pending_composer_edit: Option<PendingTrackComposerEdit>,
    xml_edit_grouping: String,
    xml_grouping_export_path: String,
    xml_pending_grouping_edit: Option<PendingTrackGroupingEdit>,
    xml_edit_label: String,
    xml_label_export_path: String,
    xml_pending_label_edit: Option<PendingTrackLabelEdit>,
    xml_edit_remixer: String,
    xml_remixer_export_path: String,
    xml_pending_remixer_edit: Option<PendingTrackRemixerEdit>,
    xml_genre_export_path: String,
    xml_selected_playlist: Option<usize>,
    xml_selected_member: Option<usize>,
    xml_pending_reorder: Option<PendingPlaylistMemberReorder>,
    xml_reorder_export_path: String,
    xml_sibling_parent: Option<usize>,
    xml_sibling_order: String,
    xml_sibling_pending: Option<PendingSiblingPlaylistReorder>,
    xml_sibling_export_path: String,
    xml_folder_sibling_parent: Option<usize>,
    xml_folder_sibling_order: String,
    xml_folder_sibling_pending: Option<PendingSiblingFolderReorder>,
    xml_folder_sibling_export_path: String,
    xml_move_destination: Option<usize>,
    xml_pending_folder_move: Option<PendingFolderMove>,
    xml_folder_move_selection: Option<usize>,
    xml_folder_move_destination: Option<usize>,
    xml_folder_move_export_path: String,
    xml_pending_move: Option<PendingPlaylistMove>,
    xml_move_export_path: String,
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
        self.xml_sibling_pending = None;
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

    fn cancel_xml_sibling_playlist_reorder(&mut self) {
        self.xml_sibling_pending = None;
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
                self.xml_sibling_pending = None;
                self.xml_path = output.display().to_string();
                self.preview_xml();
            }
            Err(error) => {
                self.xml_edit_error =
                    Some(format!("Sibling playlist reorder export failed: {error}"))
            }
        }
    }

    fn prepare_xml_sibling_folder_reorder(&mut self) {
        self.xml_folder_sibling_pending = None;
        let Some(source) = self.xml_document_source.clone() else {
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            return;
        };
        let Some(parent_ordinal) = self.xml_folder_sibling_parent else {
            return;
        };
        let Some(parent) = preview
            .folders
            .iter()
            .find(|f| f.ordinal == parent_ordinal && f.path.len() > 1)
        else {
            return;
        };
        let expected_order: Vec<String> = preview
            .folders
            .iter()
            .filter(|f| f.path.len() > 2 && f.path[..f.path.len() - 1] == parent.path)
            .map(|f| f.name.clone())
            .collect();
        let new_order: Vec<String> = self
            .xml_folder_sibling_order
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let request = oxidj_application::SiblingFolderReorderRequest {
            parent_index: parent_ordinal.saturating_sub(1),
            expected_parent_name: parent.name.clone(),
            expected_order: expected_order.clone(),
            new_order: new_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_sibling_folder_reorder_export(&source, &request)
        {
            Ok(prepared) => {
                self.xml_folder_sibling_pending = Some(PendingSiblingFolderReorder {
                    source,
                    parent_index: parent_ordinal.saturating_sub(1),
                    parent_name: parent.name.clone(),
                    parent_path: parent.path.clone(),
                    expected_order,
                    new_order,
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Sibling folder reorder refused: {error}"));
            }
        }
    }

    fn cancel_xml_sibling_folder_reorder(&mut self) {
        self.xml_folder_sibling_pending = None;
    }

    fn export_xml_sibling_folder_reorder(&mut self) {
        let Some(pending) = self.xml_folder_sibling_pending.as_ref() else {
            return;
        };
        let parent = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| {
                p.folders
                    .iter()
                    .find(|f| f.ordinal == pending.parent_index + 1)
            });
        let order: Vec<String> = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .map(|p| {
                p.folders
                    .iter()
                    .filter(|f| {
                        f.path.len() > 2
                            && parent.is_some_and(|par| f.path[..f.path.len() - 1] == par.path)
                    })
                    .map(|f| f.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_folder_sibling_parent != Some(pending.parent_index + 1)
            || parent.is_none_or(|p| p.name != pending.parent_name || p.path != pending.parent_path)
            || order != pending.expected_order
        {
            self.xml_folder_sibling_pending = None;
            self.xml_edit_error = Some(
                "Sibling folder reorder invalidated by source, parent, or order change".into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_folder_sibling_export_path);
        match oxidj_application::export_prepared_rekordbox_sibling_folder_reorder(
            &pending.prepared,
            &output,
        ) {
            Ok(()) => {
                self.xml_folder_sibling_pending = None;
                self.xml_path = output.display().to_string();
                self.preview_xml();
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Sibling folder reorder export failed: {error}"))
            }
        }
    }

    fn prepare_xml_folder_move(&mut self, folder_index: usize) {
        self.xml_pending_folder_move = None;
        let (Some(source), Some(Ok(preview)), Some(target_ordinal)) = (
            self.xml_document_source.clone(),
            self.xml_preview.as_ref(),
            self.xml_folder_move_destination,
        ) else {
            return;
        };
        let Some(folder) = preview
            .folders
            .get(folder_index)
            .filter(|f| f.path.len() > 2)
        else {
            self.xml_edit_error = Some("Folder move refused: select a non-root folder".into());
            return;
        };
        let Some(source_parent) = preview
            .folders
            .iter()
            .find(|p| p.path == folder.path[..folder.path.len() - 1])
        else {
            self.xml_edit_error = Some("Folder move refused: source parent unavailable".into());
            return;
        };
        let Some(target) = preview
            .folders
            .iter()
            .find(|p| p.ordinal == target_ordinal && p.path.len() > 1)
        else {
            self.xml_edit_error = Some("Folder move refused: destination must be non-root".into());
            return;
        };
        let order = |parent: &oxidj_rekordbox::Folder| {
            direct_xml_typed_child_order(&source, &parent.path[1..])
        };
        let source_order = match order(source_parent) {
            Ok(order) => order,
            Err(error) => {
                self.xml_edit_error = Some(format!(
                    "Folder move refused: cannot read source child order: {error}"
                ));
                return;
            }
        };
        let target_order = match order(target) {
            Ok(order) => order,
            Err(error) => {
                self.xml_edit_error = Some(format!(
                    "Folder move refused: cannot read target child order: {error}"
                ));
                return;
            }
        };
        let request = oxidj_application::FolderMoveRequest {
            folder_index: folder.ordinal.saturating_sub(1),
            expected_folder_name: folder.name.clone(),
            source_parent_index: source_parent.ordinal.saturating_sub(1),
            expected_source_parent_name: source_parent.name.clone(),
            expected_source_order: source_order.clone(),
            target_parent_index: target.ordinal.saturating_sub(1),
            expected_target_parent_name: target.name.clone(),
            expected_target_order: target_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_folder_move_export(&source, &request) {
            Ok(prepared) => {
                let mut new_path = target.path.iter().skip(1).cloned().collect::<Vec<_>>();
                new_path.push(folder.name.clone());
                self.xml_pending_folder_move = Some(PendingFolderMove {
                    source,
                    folder_index,
                    folder_name: folder.name.clone(),
                    old_path: folder.path.clone(),
                    new_path,
                    source_parent_index: source_parent.ordinal.saturating_sub(1),
                    source_parent_name: source_parent.name.clone(),
                    source_parent_path: source_parent.path.clone(),
                    target_parent_index: target.ordinal.saturating_sub(1),
                    target_parent_name: target.name.clone(),
                    target_parent_path: target.path.clone(),
                    expected_source_order: source_order,
                    expected_target_order: target_order,
                    source_count: source_parent.count.clone(),
                    target_count: target.count.clone(),
                    prepared,
                });
            }
            Err(error) => self.xml_edit_error = Some(format!("Folder move refused: {error}")),
        }
    }

    fn cancel_xml_folder_move(&mut self) {
        self.xml_pending_folder_move = None;
    }

    fn export_xml_folder_move(&mut self) {
        let Some(pending) = self.xml_pending_folder_move.as_ref() else {
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            self.xml_pending_folder_move = None;
            return;
        };
        let source_parent = preview
            .folders
            .iter()
            .find(|f| f.ordinal == pending.source_parent_index + 1);
        let target = preview
            .folders
            .iter()
            .find(|f| f.ordinal == pending.target_parent_index + 1);
        let folder = preview.folders.get(pending.folder_index);
        let live_source_order = match source_parent {
            Some(parent) => {
                match direct_xml_typed_child_order(&pending.source, &parent.path[1..]) {
                    Ok(order) => order,
                    Err(error) => {
                        self.xml_pending_folder_move = None;
                        self.xml_edit_error = Some(format!(
                            "Folder move export refused: cannot read source child order: {error}"
                        ));
                        return;
                    }
                }
            }
            None => Vec::new(),
        };
        let live_target_order = match target {
            Some(parent) => {
                match direct_xml_typed_child_order(&pending.source, &parent.path[1..]) {
                    Ok(order) => order,
                    Err(error) => {
                        self.xml_pending_folder_move = None;
                        self.xml_edit_error = Some(format!(
                            "Folder move export refused: cannot read target child order: {error}"
                        ));
                        return;
                    }
                }
            }
            None => Vec::new(),
        };
        let stale = self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_folder_move_selection != Some(pending.folder_index)
            || self.xml_folder_move_destination != Some(pending.target_parent_index + 1)
            || folder.is_none_or(|f| f.name != pending.folder_name || f.path != pending.old_path)
            || source_parent.is_none_or(|p| {
                p.name != pending.source_parent_name
                    || p.path != pending.source_parent_path
                    || p.count != pending.source_count
            })
            || target.is_none_or(|p| {
                p.name != pending.target_parent_name
                    || p.path != pending.target_parent_path
                    || p.count != pending.target_count
            })
            || live_source_order != pending.expected_source_order
            || live_target_order != pending.expected_target_order;
        if stale {
            self.xml_pending_folder_move = None;
            self.xml_edit_error = Some(
                "Folder move invalidated by source, selection, parent or typed child order change"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_folder_move_export_path);
        match oxidj_application::export_prepared_rekordbox_folder_move(&pending.prepared, &output) {
            Ok(()) => {
                self.xml_pending_folder_move = None;
                self.xml_path = output.display().to_string();
                self.preview_xml();
            }
            Err(error) => self.xml_edit_error = Some(format!("Folder move export failed: {error}")),
        }
    }

    fn prepare_xml_playlist_move(&mut self, playlist_index: usize) {
        self.xml_pending_move = None;
        let (Some(source), Some(Ok(preview)), Some(destination_ordinal)) = (
            self.xml_document_source.clone(),
            self.xml_preview.as_ref(),
            self.xml_move_destination,
        ) else {
            return;
        };
        let Some(playlist) = preview.playlists.get(playlist_index) else {
            return;
        };
        let Some(source_parent) = immediate_parent_folder(preview, playlist) else {
            self.xml_edit_error = Some("Playlist move refused: source parent unavailable".into());
            return;
        };
        let Some(destination) = preview
            .folders
            .iter()
            .find(|f| f.ordinal == destination_ordinal && f.path.len() > 1)
        else {
            self.xml_edit_error =
                Some("Playlist move refused: destination must be a non-root folder".into());
            return;
        };
        let old_path = playlist.path.clone();
        let new_path = [
            destination.path.iter().skip(1).cloned().collect::<Vec<_>>(),
            vec![playlist.name.clone()],
        ]
        .concat();
        let order_under = |parent: &oxidj_rekordbox::Folder| -> Vec<String> {
            direct_xml_node_order(&source, &parent.path[1..]).unwrap_or_default()
        };
        let source_order = order_under(source_parent);
        let destination_order = order_under(destination);
        let request = oxidj_application::PlaylistMoveRequest {
            playlist_index,
            expected_playlist_name: playlist.name.clone(),
            expected_members: playlist.tracks.clone(),
            source_parent_index: source_parent.ordinal.saturating_sub(1),
            expected_source_parent_name: source_parent.name.clone(),
            target_parent_index: destination.ordinal.saturating_sub(1),
            expected_target_parent_name: destination.name.clone(),
            expected_target_order: destination_order.clone(),
        };
        match oxidj_application::prepare_rekordbox_playlist_move_export(&source, &request) {
            Ok(prepared) => {
                self.xml_pending_move = Some(PendingPlaylistMove {
                    source,
                    playlist_index,
                    source_parent_index: source_parent.ordinal.saturating_sub(1),
                    source_parent_name: source_parent.name.clone(),
                    source_parent_path: source_parent.path.clone(),
                    destination_parent_index: destination.ordinal.saturating_sub(1),
                    destination_parent_name: destination.name.clone(),
                    destination_parent_path: destination.path.clone(),
                    expected_members: playlist.tracks.clone(),
                    expected_source_order: source_order,
                    expected_destination_order: destination_order,
                    old_path,
                    new_path,
                    source_count: source_parent.count.clone(),
                    destination_count: destination.count.clone(),
                    prepared,
                })
            }
            Err(error) => self.xml_edit_error = Some(format!("Playlist move refused: {error}")),
        }
    }

    fn cancel_xml_playlist_move(&mut self) {
        self.xml_pending_move = None;
    }

    fn export_xml_playlist_move(&mut self) {
        let Some(pending) = self.xml_pending_move.as_ref() else {
            return;
        };
        let Some(Ok(preview)) = self.xml_preview.as_ref() else {
            self.xml_pending_move = None;
            return;
        };
        let source_parent = preview
            .folders
            .iter()
            .find(|f| f.ordinal == pending.source_parent_index + 1);
        let destination = preview
            .folders
            .iter()
            .find(|f| f.ordinal == pending.destination_parent_index + 1);
        let playlist = preview.playlists.get(pending.playlist_index);
        let order_under = |parent: Option<&oxidj_rekordbox::Folder>| -> Vec<String> {
            parent
                .and_then(|parent| direct_xml_node_order(&pending.source, &parent.path[1..]).ok())
                .unwrap_or_default()
        };
        let live_source_order = source_parent
            .and_then(|p| direct_xml_node_order(&pending.source, &p.path[1..]).ok())
            .unwrap_or_default();
        let live_destination_order = destination
            .and_then(|p| direct_xml_node_order(&pending.source, &p.path[1..]).ok())
            .unwrap_or_default();
        let stale = self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_playlist != Some(pending.playlist_index)
            || self.xml_move_destination != Some(pending.destination_parent_index + 1)
            || playlist.is_none_or(|p| {
                p.name != *pending.old_path.last().unwrap_or(&String::new())
                    || p.tracks != pending.expected_members
            })
            || source_parent.is_none_or(|p| {
                p.name != pending.source_parent_name
                    || p.path != pending.source_parent_path
                    || p.count != pending.source_count
            })
            || destination.is_none_or(|p| {
                p.name != pending.destination_parent_name
                    || p.path != pending.destination_parent_path
                    || p.count != pending.destination_count
            })
            || order_under(source_parent) != pending.expected_source_order
            || order_under(destination) != pending.expected_destination_order
            || live_source_order != pending.expected_source_order
            || live_destination_order != pending.expected_destination_order;
        if stale {
            self.xml_pending_move = None;
            self.xml_edit_error = Some(
                "Playlist move invalidated by source, selection, parent or order change".into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_move_export_path);
        match oxidj_application::export_prepared_rekordbox_playlist_move(&pending.prepared, &output)
        {
            Ok(()) => {
                self.xml_pending_move = None;
                self.xml_path = output.display().to_string();
                self.preview_xml();
            }
            Err(error) => {
                self.xml_pending_move = None;
                self.xml_edit_error = Some(format!("Playlist move export failed: {error}"));
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
        self.xml_edit_artist = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| preview.tracks.iter().find(|track| track.id == track_id))
            .and_then(|track| track.artist.clone())
            .unwrap_or_default();
        self.xml_edit_comments = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| preview.tracks.iter().find(|track| track.id == track_id))
            .and_then(|track| track.comments.clone())
            .unwrap_or_default();
        self.xml_edit_album = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| preview.tracks.iter().find(|track| track.id == track_id))
            .and_then(|track| track.album.clone())
            .unwrap_or_default();
        self.xml_edit_genre = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == track_id))
            .and_then(|t| t.genre.clone())
            .unwrap_or_default();
        self.xml_pending_genre_edit = None;
        self.xml_edit_label = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == track_id))
            .and_then(|t| t.label.clone())
            .unwrap_or_default();
        self.xml_edit_composer = self.xml_preview.as_ref().and_then(|p| p.as_ref().ok()).and_then(|p| p.tracks.iter().find(|t| t.id == track_id)).and_then(|t| t.composer.clone()).unwrap_or_default();
        self.xml_pending_composer_edit = None;
        self.xml_edit_grouping = self.xml_preview.as_ref().and_then(|p| p.as_ref().ok()).and_then(|p| p.tracks.iter().find(|t| t.id == track_id)).and_then(|t| t.grouping.clone()).unwrap_or_default();
        self.xml_pending_grouping_edit = None;
        self.xml_pending_label_edit = None;
        self.xml_pending_edit = None;
        self.xml_pending_artist_edit = None;
        self.xml_pending_comments_edit = None;
        self.xml_pending_album_edit = None;
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
        self.xml_pending_artist_edit = None;
        self.xml_pending_comments_edit = None;
        self.xml_pending_album_edit = None;
        self.xml_pending_genre_edit = None;
        self.xml_pending_composer_edit = None;
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
        self.xml_pending_artist_edit = None;
        self.xml_pending_comments_edit = None;
        self.xml_pending_album_edit = None;
        self.xml_pending_genre_edit = None;
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

    fn prepare_xml_artist_edit(&mut self, track_id: &str, old_artist: Option<&str>) {
        let Some(old_artist) = old_artist else {
            self.xml_pending_artist_edit = None;
            self.xml_edit_error = Some(
                "Track Artist preparation refused: selected track has no Artist attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_artist_edit = None;
            self.xml_edit_error =
                Some("Track Artist preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackArtistEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_artist: old_artist.to_owned(),
            new_artist: self.xml_edit_artist.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_artist_export(&request) {
            Ok(prepared) => {
                self.xml_pending_artist_edit = Some(PendingTrackArtistEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_artist_edit = None;
                self.xml_edit_error = Some(format!("Track Artist preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_artist_edit(&mut self) {
        self.xml_pending_artist_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_artist_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_artist_edit.clone() else {
            self.xml_edit_error =
                Some("Track Artist export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_artist = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| {
                preview
                    .tracks
                    .iter()
                    .find(|track| track.id == pending.track_id)
            })
            .and_then(|track| track.artist.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_artist != proposal.new_artist
            || selected_artist != Some(proposal.old_artist.as_str())
        {
            self.xml_pending_artist_edit = None;
            self.xml_edit_error = Some(
                "Track Artist export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_artist_export_path);
        if self.xml_artist_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Artist export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_artist_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_artist_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Artist exported to {}", output.display());
                self.xml_artist_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output)
                        .map_err(|error| error.to_string()),
                );
                self.xml_edit_artist = change.new_artist;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track Artist export failed: {error}"));
            }
        }
    }

    fn prepare_xml_comments_edit(&mut self, track_id: &str, old_comments: Option<&str>) {
        let Some(old_comments) = old_comments else {
            self.xml_pending_comments_edit = None;
            self.xml_edit_error = Some(
                "Track Comments preparation refused: selected track has no Comments attribute"
                    .into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_comments_edit = None;
            self.xml_edit_error =
                Some("Track Comments preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackCommentsEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_comments: old_comments.to_owned(),
            new_comments: self.xml_edit_comments.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_comments_export(&request) {
            Ok(prepared) => {
                self.xml_pending_comments_edit = Some(PendingTrackCommentsEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_comments_edit = None;
                self.xml_edit_error = Some(format!("Track Comments preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_comments_edit(&mut self) {
        self.xml_pending_comments_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_comments_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_comments_edit.clone() else {
            self.xml_edit_error =
                Some("Track Comments export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_comments = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| {
                preview
                    .tracks
                    .iter()
                    .find(|track| track.id == pending.track_id)
            })
            .and_then(|track| track.comments.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_comments != proposal.new_comments
            || selected_comments != Some(proposal.old_comments.as_str())
        {
            self.xml_pending_comments_edit = None;
            self.xml_edit_error = Some(
                "Track Comments export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_comments_export_path);
        if self.xml_comments_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Comments export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_comments_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_comments_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Comments exported to {}", output.display());
                self.xml_comments_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output)
                        .map_err(|error| error.to_string()),
                );
                self.xml_edit_comments = change.new_comments;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track Comments export failed: {error}"));
            }
        }
    }

    fn prepare_xml_album_edit(&mut self, track_id: &str, old_album: Option<&str>) {
        let Some(old_album) = old_album else {
            self.xml_pending_album_edit = None;
            self.xml_edit_error = Some(
                "Track Album preparation refused: selected track has no Album attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_album_edit = None;
            self.xml_edit_error =
                Some("Track Album preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackAlbumEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_album: old_album.to_owned(),
            new_album: self.xml_edit_album.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_album_export(&request) {
            Ok(prepared) => {
                self.xml_pending_album_edit = Some(PendingTrackAlbumEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_album_edit = None;
                self.xml_edit_error = Some(format!("Track Album preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_album_edit(&mut self) {
        self.xml_pending_album_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_album_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_album_edit.clone() else {
            self.xml_edit_error =
                Some("Track Album export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_album = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| {
                preview
                    .tracks
                    .iter()
                    .find(|track| track.id == pending.track_id)
            })
            .and_then(|track| track.album.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_album != proposal.new_album
            || selected_album != Some(proposal.old_album.as_str())
        {
            self.xml_pending_album_edit = None;
            self.xml_edit_error = Some(
                "Track Album export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_album_export_path);
        if self.xml_album_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Album export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_album_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_album_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Album exported to {}", output.display());
                self.xml_album_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output)
                        .map_err(|error| error.to_string()),
                );
                self.xml_edit_album = change.new_album;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track Album export failed: {error}"));
            }
        }
    }

    fn prepare_xml_genre_edit(&mut self, track_id: &str, old_genre: Option<&str>) {
        let Some(old_genre) = old_genre else {
            self.xml_pending_genre_edit = None;
            self.xml_edit_error = Some(
                "Track Genre preparation refused: selected track has no Genre attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_genre_edit = None;
            self.xml_edit_error =
                Some("Track Genre preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackGenreEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_genre: old_genre.to_owned(),
            new_genre: self.xml_edit_genre.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_genre_export(&request) {
            Ok(prepared) => {
                self.xml_pending_genre_edit = Some(PendingTrackGenreEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_genre_edit = None;
                self.xml_edit_error = Some(format!("Track Genre preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_genre_edit(&mut self) {
        self.xml_pending_genre_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_genre_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_genre_edit.clone() else {
            self.xml_edit_error =
                Some("Track Genre export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_genre = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(|t| t.genre.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_genre != proposal.new_genre
            || selected_genre != Some(proposal.old_genre.as_str())
        {
            self.xml_pending_genre_edit = None;
            self.xml_edit_error = Some(
                "Track Genre export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_genre_export_path);
        if self.xml_genre_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Genre export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_genre_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_genre_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Genre exported to {}", output.display());
                self.xml_genre_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_edit_genre = change.new_genre;
            }
            Err(error) => self.xml_edit_error = Some(format!("Track Genre export failed: {error}")),
        }
    }

    fn prepare_xml_composer_edit(&mut self, track_id: &str, old_composer: Option<&str>) {
        let Some(old_composer) = old_composer else {
            self.xml_pending_composer_edit = None;
            self.xml_edit_error = Some(
                "Track Composer preparation refused: selected track has no Composer attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_composer_edit = None;
            self.xml_edit_error =
                Some("Track Composer preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackComposerEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_composer: old_composer.to_owned(),
            new_composer: self.xml_edit_composer.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_composer_export(&request) {
            Ok(prepared) => {
                self.xml_pending_composer_edit = Some(PendingTrackComposerEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_composer_edit = None;
                self.xml_edit_error = Some(format!("Track Composer preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_composer_edit(&mut self) {
        self.xml_pending_composer_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_composer_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_composer_edit.clone() else {
            self.xml_edit_error =
                Some("Track Composer export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_composer = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(|t| t.composer.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || PathBuf::from(&self.xml_path) != pending.source
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_composer != proposal.new_composer
            || selected_composer != Some(proposal.old_composer.as_str())
        {
            self.xml_pending_composer_edit = None;
            self.xml_edit_error = Some(
                "Track Composer export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_composer_export_path);
        if self.xml_composer_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Composer export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_composer_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_composer_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Composer exported to {}", output.display());
                self.xml_composer_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_edit_composer = change.new_composer;
            }
            Err(error) => self.xml_edit_error = Some(format!("Track Composer export failed: {error}")),
        }
    }

    fn prepare_xml_remixer_edit(&mut self, track_id: &str, old_remixer: Option<&str>) {
        let Some(old_remixer) = old_remixer else {
            self.xml_pending_remixer_edit = None;
            self.xml_edit_error = Some(
                "Track Remixer preparation refused: selected track has no Remixer attribute"
                    .into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_remixer_edit = None;
            self.xml_edit_error =
                Some("Track Remixer preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackRemixerEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_remixer: old_remixer.to_owned(),
            new_remixer: self.xml_edit_remixer.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_remixer_export(&request) {
            Ok(prepared) => {
                self.xml_pending_remixer_edit = Some(PendingTrackRemixerEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_remixer_edit = None;
                self.xml_edit_error = Some(format!("Track Remixer preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_remixer_edit(&mut self) {
        self.xml_pending_remixer_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_remixer_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_remixer_edit.clone() else {
            self.xml_edit_error = Some("Track Remixer export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_remixer = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(|t| t.remixer.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || PathBuf::from(&self.xml_path) != pending.source
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_remixer != proposal.new_remixer
            || selected_remixer != Some(proposal.old_remixer.as_str())
        {
            self.xml_pending_remixer_edit = None;
            self.xml_edit_error = Some(
                "Track Remixer export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_remixer_export_path);
        if self.xml_remixer_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Remixer export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_remixer_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_remixer_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Remixer exported to {}", output.display());
                self.xml_remixer_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_edit_remixer = change.new_remixer;
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track Remixer export failed: {error}"));
            }
        }
    }

    fn prepare_xml_grouping_edit(&mut self, track_id: &str, old_grouping: Option<&str>) {
        let Some(old_grouping) = old_grouping else {
            self.xml_pending_grouping_edit = None;
            self.xml_edit_error = Some(
                "Track Grouping preparation refused: selected track has no Grouping attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_grouping_edit = None;
            self.xml_edit_error =
                Some("Track Grouping preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackGroupingEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_grouping: old_grouping.to_owned(),
            new_grouping: self.xml_edit_grouping.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_grouping_export(&request) {
            Ok(prepared) => {
                self.xml_pending_grouping_edit = Some(PendingTrackGroupingEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_grouping_edit = None;
                self.xml_edit_error = Some(format!("Track Grouping preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_grouping_edit(&mut self) {
        self.xml_pending_grouping_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_grouping_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_grouping_edit.clone() else {
            self.xml_edit_error =
                Some("Track Grouping export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_grouping = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(|t| t.grouping.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || PathBuf::from(&self.xml_path) != pending.source
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_grouping != proposal.new_grouping
            || selected_grouping != Some(proposal.old_grouping.as_str())
        {
            self.xml_pending_grouping_edit = None;
            self.xml_edit_error = Some(
                "Track Grouping export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_grouping_export_path);
        if self.xml_grouping_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Grouping export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_grouping_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_grouping_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Grouping exported to {}", output.display());
                self.xml_grouping_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_edit_grouping = change.new_grouping;
            }
            Err(error) => self.xml_edit_error = Some(format!("Track Grouping export failed: {error}")),
        }
    }

    fn prepare_xml_label_edit(&mut self, track_id: &str, old_label: Option<&str>) {
        let Some(old_label) = old_label else {
            self.xml_pending_label_edit = None;
            self.xml_edit_error = Some(
                "Track Label preparation refused: selected track has no Label attribute".into(),
            );
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            self.xml_pending_label_edit = None;
            self.xml_edit_error =
                Some("Track Label preparation refused: no XML document is loaded".into());
            return;
        };
        let request = oxidj_application::TrackLabelEditPreparationRequest {
            source: source.clone(),
            track_id: track_id.to_owned(),
            expected_old_label: old_label.to_owned(),
            new_label: self.xml_edit_label.clone(),
        };
        match oxidj_application::prepare_rekordbox_track_label_export(&request) {
            Ok(prepared) => {
                self.xml_pending_label_edit = Some(PendingTrackLabelEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                self.xml_pending_label_edit = None;
                self.xml_edit_error = Some(format!("Track Label preparation refused: {error}"));
            }
        }
    }

    fn cancel_xml_label_edit(&mut self) {
        self.xml_pending_label_edit = None;
        self.xml_edit_error = None;
    }

    fn export_xml_label_edit(&mut self) {
        self.notice.clear();
        let Some(pending) = self.xml_pending_label_edit.clone() else {
            self.xml_edit_error =
                Some("Track Label export refused: no prepared pending edit".into());
            return;
        };
        let proposal = pending.prepared.change();
        let selected_label = self
            .xml_preview
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .and_then(|p| p.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(|t| t.label.as_deref());
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || self.xml_edit_label != proposal.new_label
            || selected_label != Some(proposal.old_label.as_str())
        {
            self.xml_pending_label_edit = None;
            self.xml_edit_error = Some(
                "Track Label export refused: source, selected track, or proposed value changed"
                    .into(),
            );
            return;
        }
        let output = PathBuf::from(&self.xml_label_export_path);
        if self.xml_label_export_path.trim().is_empty() {
            self.xml_edit_error =
                Some("Track Label export refused: choose a new XML destination".into());
            return;
        }
        match oxidj_application::export_prepared_rekordbox_track_label_edit(
            &pending.prepared,
            &output,
        ) {
            Ok(change) => {
                self.xml_pending_label_edit = None;
                self.xml_edit_error = None;
                self.notice = format!("Track Label exported to {}", output.display());
                self.xml_label_export_path = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output).map_err(|e| e.to_string()),
                );
                self.xml_edit_label = change.new_label;
            }
            Err(error) => self.xml_edit_error = Some(format!("Track Label export failed: {error}")),
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
                        ui.strong("Prepare one guarded Artist edit");
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|track| track.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if track.artist.is_none() {
                                    ui.label("Selected track has no Artist attribute; insertion is not supported.");
                                }
                                ui.horizontal(|ui| {
                                    ui.label("Proposed Artist");
                                    if ui.text_edit_singleline(&mut self.xml_edit_artist).changed() {
                                        self.xml_pending_artist_edit = None;
                                    }
                                });
                                if ui.button("Prepare pending Artist edit").clicked() {
                                    self.prepare_xml_artist_edit(&track.id, track.artist.as_deref());
                                }
                                if let Some(pending) = &self.xml_pending_artist_edit {
                                    ui.separator();
                                    ui.strong("Pending Artist edit · not exported or saved");
                                    ui.label(format!("Source: {}", pending.source.display()));
                                    ui.label(format!("TrackID: {}", pending.track_id));
                                    ui.label(format!("Original Artist: {:?}", pending.prepared.change().old_artist));
                                    ui.label(format!("Proposed Artist: {:?}", pending.prepared.change().new_artist));
                                    ui.horizontal(|ui| {
                                        ui.label("New XML destination");
                                        ui.text_edit_singleline(&mut self.xml_artist_export_path);
                                    });
                                    if ui.button("Export prepared Artist edit").clicked() {
                                        self.export_xml_artist_edit();
                                    }
                                    if ui.button("Cancel pending Artist edit").clicked() {
                                        self.cancel_xml_artist_edit();
                                    }
                                }
                            } else {
                                self.xml_pending_artist_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Artist edit.");
                        }
                        ui.separator();
                        ui.strong("Prepare one guarded Comments edit");
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|track| track.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if track.comments.is_none() {
                                    ui.label("Selected track has no Comments attribute; insertion is not supported.");
                                } else {
                                    ui.label(if track.comments.as_deref() == Some("") {
                                        "Current Comments: present but empty"
                                    } else {
                                        "Current Comments: present"
                                    });
                                    if ui.add(
                                        egui::TextEdit::multiline(&mut self.xml_edit_comments)
                                            .desired_rows(4)
                                            .hint_text("Comments"),
                                    ).changed() {
                                        self.xml_pending_comments_edit = None;
                                    }
                                    if ui.button("Prepare pending Comments edit").clicked() {
                                        self.prepare_xml_comments_edit(
                                            &track.id,
                                            track.comments.as_deref(),
                                        );
                                    }
                                    if let Some(pending) = &self.xml_pending_comments_edit {
                                        ui.separator();
                                        ui.strong("Pending Comments edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display()));
                                        ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!("Original Comments: {:?}", pending.prepared.change().old_comments));
                                        ui.label(format!("Proposed Comments: {:?}", pending.prepared.change().new_comments));
                                        ui.horizontal(|ui| {
                                            ui.label("New XML destination");
                                            ui.text_edit_singleline(&mut self.xml_comments_export_path);
                                        });
                                        if ui.button("Export prepared Comments edit").clicked() {
                                            self.export_xml_comments_edit();
                                        }
                                        if ui.button("Cancel pending Comments edit").clicked() {
                                            self.cancel_xml_comments_edit();
                                        }
                                    }
                                }
                            } else {
                                self.xml_pending_comments_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Comments edit.");
                        }
                        ui.separator();
                        ui.strong("Prepare one guarded Album edit");
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|track| track.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if track.album.is_none() {
                                    ui.label("Selected track has no Album attribute; insertion is not supported.");
                                } else {
                                    ui.label(if track.album.as_deref() == Some("") {
                                        "Current Album: present but empty"
                                    } else {
                                        "Current Album: present"
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Proposed Album");
                                        if ui.text_edit_singleline(&mut self.xml_edit_album).changed() {
                                            self.xml_pending_album_edit = None;
                                        }
                                    });
                                    if ui.button("Prepare pending Album edit").clicked() {
                                        self.prepare_xml_album_edit(&track.id, track.album.as_deref());
                                    }
                                    if let Some(pending) = &self.xml_pending_album_edit {
                                        ui.separator();
                                        ui.strong("Pending Album edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display()));
                                        ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!("Original Album: {:?}", pending.prepared.change().old_album));
                                        ui.label(format!("Proposed Album: {:?}", pending.prepared.change().new_album));
                                        ui.horizontal(|ui| {
                                            ui.label("New XML destination");
                                            ui.text_edit_singleline(&mut self.xml_album_export_path);
                                        });
                                        if ui.button("Export prepared Album edit").clicked() {
                                            self.export_xml_album_edit();
                                        }
                                        if ui.button("Cancel pending Album edit").clicked() {
                                            self.cancel_xml_album_edit();
                                        }
                                    }
                                }
                            } else {
                                self.xml_pending_album_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Album edit.");
                        }
                        SectionHeading {
                            title: "Track Genre edit (provisional)",
                        }
                        .show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if let Some(genre) = track.genre.as_deref() {
                                    ui.label(if genre.is_empty() {
                                        "Current Genre: present but empty"
                                    } else {
                                        "Current Genre: present"
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Proposed Genre");
                                        if ui
                                            .text_edit_singleline(&mut self.xml_edit_genre)
                                            .changed()
                                        {
                                            self.xml_pending_genre_edit = None;
                                        }
                                    });
                                    if ui.button("Prepare pending Genre edit").clicked() {
                                        self.prepare_xml_genre_edit(&track.id, Some(genre));
                                    }
                                    if let Some(pending) = &self.xml_pending_genre_edit {
                                        ui.separator();
                                        ui.strong("Pending Genre edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display()));
                                        ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!(
                                            "Original Genre: {:?}",
                                            pending.prepared.change().old_genre
                                        ));
                                        ui.label(format!(
                                            "Proposed Genre: {:?}",
                                            pending.prepared.change().new_genre
                                        ));
                                        ui.horizontal(|ui| {
                                            ui.label("New XML destination");
                                            ui.text_edit_singleline(&mut self.xml_genre_export_path);
                                        });
                                        if ui.button("Export prepared Genre edit").clicked() {
                                            self.export_xml_genre_edit();
                                        }
                                        if ui.button("Cancel pending Genre edit").clicked() {
                                            self.cancel_xml_genre_edit();
                                        }
                                    }
                                } else {
                                    self.xml_pending_genre_edit = None;
                                    ui.label("Selected track has no Genre attribute; insertion is not supported.");
                                }
                            } else {
                                self.xml_pending_genre_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Genre edit.");
                        }
                        SectionHeading { title: "Track Remixer edit (provisional)" }.show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if let Some(remixer) = track.remixer.as_deref() {
                                    ui.label(if remixer.is_empty() {
                                        "Current Remixer: present but empty"
                                    } else {
                                        "Current Remixer: present"
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Proposed Remixer");
                                        if ui
                                            .text_edit_singleline(&mut self.xml_edit_remixer)
                                            .changed()
                                        {
                                            self.xml_pending_remixer_edit = None;
                                        }
                                    });
                                    if ui.button("Prepare pending Remixer edit").clicked() {
                                        self.prepare_xml_remixer_edit(&track.id, Some(remixer));
                                    }
                                    if let Some(pending) = &self.xml_pending_remixer_edit {
                                        ui.separator();
                                        ui.strong("Pending Remixer edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display()));
                                        ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!(
                                            "Original Remixer: {:?}",
                                            pending.prepared.change().old_remixer
                                        ));
                                        ui.label(format!(
                                            "Proposed Remixer: {:?}",
                                            pending.prepared.change().new_remixer
                                        ));
                                        ui.horizontal(|ui| {
                                            ui.label("New XML destination");
                                            ui.text_edit_singleline(&mut self.xml_remixer_export_path);
                                        });
                                        if ui.button("Export prepared Remixer edit").clicked() {
                                            self.export_xml_remixer_edit();
                                        }
                                        if ui.button("Cancel pending Remixer edit").clicked() {
                                            self.cancel_xml_remixer_edit();
                                        }
                                    }
                                } else {
                                    self.xml_pending_remixer_edit = None;
                                    ui.label("Selected track has no Remixer attribute; insertion is not supported.");
                                }
                            } else {
                                self.xml_pending_remixer_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Remixer edit.");
                        }
                        SectionHeading { title: "Track Composer edit (provisional)" }.show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if let Some(composer) = track.composer.as_deref() {
                                    ui.label(if composer.is_empty() { "Current Composer: present but empty" } else { "Current Composer: present" });
                                    ui.horizontal(|ui| { ui.label("Proposed Composer"); if ui.text_edit_singleline(&mut self.xml_edit_composer).changed() { self.xml_pending_composer_edit = None; } });
                                    if ui.button("Prepare pending Composer edit").clicked() { self.prepare_xml_composer_edit(&track.id, Some(composer)); }
                                    if let Some(pending) = &self.xml_pending_composer_edit {
                                        ui.separator(); ui.strong("Pending Composer edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display())); ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!("Original Composer: {:?}", pending.prepared.change().old_composer));
                                        ui.label(format!("Proposed Composer: {:?}", pending.prepared.change().new_composer));
                                        ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_composer_export_path); });
                                        if ui.button("Export prepared Composer edit").clicked() { self.export_xml_composer_edit(); }
                                        if ui.button("Cancel pending Composer edit").clicked() { self.cancel_xml_composer_edit(); }
                                    }
                                } else { self.xml_pending_composer_edit = None; ui.label("Selected track has no Composer attribute; insertion is not supported."); }
                            } else { self.xml_pending_composer_edit = None; ui.label("Selected TrackID is no longer in the loaded preview."); }
                        } else { ui.label("Select one track above to prepare a pending Composer edit."); }
                        SectionHeading { title: "Track Grouping edit (provisional)" }.show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if let Some(grouping) = track.grouping.as_deref() {
                                    ui.label(if grouping.is_empty() { "Current Grouping: present but empty" } else { "Current Grouping: present" });
                                    ui.horizontal(|ui| { ui.label("Proposed Grouping"); if ui.text_edit_singleline(&mut self.xml_edit_grouping).changed() { self.xml_pending_grouping_edit = None; } });
                                    if ui.button("Prepare pending Grouping edit").clicked() { self.prepare_xml_grouping_edit(&track.id, Some(grouping)); }
                                    if let Some(pending) = &self.xml_pending_grouping_edit {
                                        ui.separator(); ui.strong("Pending Grouping edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display())); ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!("Original Grouping: {:?}", pending.prepared.change().old_grouping));
                                        ui.label(format!("Proposed Grouping: {:?}", pending.prepared.change().new_grouping));
                                        ui.horizontal(|ui| { ui.label("New XML destination"); ui.text_edit_singleline(&mut self.xml_grouping_export_path); });
                                        if ui.button("Export prepared Grouping edit").clicked() { self.export_xml_grouping_edit(); }
                                        if ui.button("Cancel pending Grouping edit").clicked() { self.cancel_xml_grouping_edit(); }
                                    }
                                } else { self.xml_pending_grouping_edit = None; ui.label("Selected track has no Grouping attribute; insertion is not supported."); }
                            } else { self.xml_pending_grouping_edit = None; ui.label("Selected TrackID is no longer in the loaded preview."); }
                        } else { ui.label("Select one track above to prepare a pending Grouping edit."); }
                        SectionHeading {
                            title: "Track Label edit (provisional)",
                        }
                        .show(ui, tokens);
                        if let Some(track_id) = self.xml_selected_track.clone() {
                            if let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) {
                                ui.label(format!("Selected TrackID: {track_id}"));
                                if let Some(label) = track.label.as_deref() {
                                    ui.label(if label.is_empty() {
                                        "Current Label: present but empty"
                                    } else {
                                        "Current Label: present"
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Proposed Label");
                                        if ui
                                            .text_edit_singleline(&mut self.xml_edit_label)
                                            .changed()
                                        {
                                            self.xml_pending_label_edit = None;
                                        }
                                    });
                                    if ui.button("Prepare pending Label edit").clicked() {
                                        self.prepare_xml_label_edit(&track.id, Some(label));
                                    }
                                    if let Some(pending) = &self.xml_pending_label_edit {
                                        ui.separator();
                                        ui.strong("Pending Label edit · not exported or saved");
                                        ui.label(format!("Source: {}", pending.source.display()));
                                        ui.label(format!("TrackID: {}", pending.track_id));
                                        ui.label(format!(
                                            "Original Label: {:?}",
                                            pending.prepared.change().old_label
                                        ));
                                        ui.label(format!(
                                            "Proposed Label: {:?}",
                                            pending.prepared.change().new_label
                                        ));
                                        ui.horizontal(|ui| {
                                            ui.label("New XML destination");
                                            ui.text_edit_singleline(&mut self.xml_label_export_path);
                                        });
                                        if ui.button("Export prepared Label edit").clicked() {
                                            self.export_xml_label_edit();
                                        }
                                        if ui.button("Cancel pending Label edit").clicked() {
                                            self.cancel_xml_label_edit();
                                        }
                                    }
                                } else {
                                    self.xml_pending_label_edit = None;
                                    ui.label("Selected track has no Label attribute; insertion is not supported.");
                                }
                            } else {
                                self.xml_pending_label_edit = None;
                                ui.label("Selected TrackID is no longer in the loaded preview.");
                            }
                        } else {
                            ui.label("Select one track above to prepare a pending Label edit.");
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
    fn sibling_folder_reorder_ui_exports_verified_bytes_and_guards_refusals() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-sibling-folder-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-sibling-folder-reorder/source.xml"
        );
        let source = root.join("source.xml");
        std::fs::copy(fixture, &source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let parent = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Sets")
            .unwrap();
        app.xml_folder_sibling_parent = Some(parent.ordinal);
        app.xml_folder_sibling_order = "Beta, Alpha".into();
        app.prepare_xml_sibling_folder_reorder();
        assert!(
            app.xml_folder_sibling_pending.is_some(),
            "{:?}",
            app.xml_edit_error
        );
        let pending = app.xml_folder_sibling_pending.as_ref().unwrap();
        assert_eq!(pending.expected_order, ["Alpha", "Beta"]);
        assert_eq!(pending.new_order, ["Beta", "Alpha"]);
        let dest = root.join("output.xml");
        app.xml_folder_sibling_export_path = dest.display().to_string();
        app.export_xml_sibling_folder_reorder();
        let bytes = std::fs::read(&dest).unwrap();
        f02_sibling_folder_reorder_oracle::check(&bytes).unwrap();
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let set = reopened.folders.iter().find(|f| f.name == "Sets").unwrap();
        let children: Vec<_> = reopened
            .folders
            .iter()
            .filter(|f| f.path.len() > 2 && f.path[..f.path.len() - 1] == set.path)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(children, ["Beta", "Alpha"]);
        assert!(
            reopened
                .playlists
                .iter()
                .any(|p| p.name == "Interleaved" && p.tracks == ["200", "400"])
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
        let mutated = String::from_utf8(bytes.clone())
            .unwrap()
            .replace("Name=\"Sets\" Count=\"3\"", "Name=\"Sets\" Count=\"4\"");
        assert!(f02_sibling_folder_reorder_oracle::check(mutated.as_bytes()).is_err());
        app.export_xml_sibling_folder_reorder();
        assert_eq!(std::fs::read(&dest).unwrap(), bytes);

        let source2 = root.join("stale.xml");
        std::fs::copy(fixture, &source2).unwrap();
        let mut stale = LibraryApp::new(
            PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        stale.xml_path = source2.display().to_string();
        stale.preview_xml();
        stale.xml_folder_sibling_parent = Some(
            stale
                .xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .folders
                .iter()
                .find(|f| f.name == "Sets")
                .unwrap()
                .ordinal,
        );
        stale.xml_folder_sibling_order = "Beta, Alpha".into();
        stale.prepare_xml_sibling_folder_reorder();
        stale.xml_document_source = Some(root.join("other.xml"));
        stale.xml_folder_sibling_export_path = root.join("stale-output.xml").display().to_string();
        stale.export_xml_sibling_folder_reorder();
        assert!(stale.xml_folder_sibling_pending.is_none());
        assert!(!root.join("stale-output.xml").exists());
        let setup = |label: &str, order: &str| {
            let path = root.join(format!("{label}.xml"));
            std::fs::copy(fixture, &path).unwrap();
            let mut candidate = LibraryApp::new(
                PathBuf::from("/definitely/missing/oxidj-test.sqlite"),
                ThemeMode::TokyoNight,
                ThemeMode::TokyoNight.tokens(),
            );
            candidate.xml_path = path.display().to_string();
            candidate.preview_xml();
            candidate.xml_folder_sibling_parent = Some(
                candidate
                    .xml_preview
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .folders
                    .iter()
                    .find(|f| f.name == "Sets")
                    .unwrap()
                    .ordinal,
            );
            candidate.xml_folder_sibling_order = order.into();
            candidate.prepare_xml_sibling_folder_reorder();
            (candidate, path)
        };
        for stale_kind in ["parent", "order"] {
            let (mut candidate, source) = setup("changed", "Beta, Alpha");
            if stale_kind == "parent" {
                candidate
                    .xml_preview
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .folders
                    .iter_mut()
                    .find(|f| f.name == "Sets")
                    .unwrap()
                    .name = "Changed".into();
            } else {
                let folders = &mut candidate
                    .xml_preview
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .folders;
                let alpha = folders.iter().position(|f| f.name == "Alpha").unwrap();
                let beta = folders.iter().position(|f| f.name == "Beta").unwrap();
                folders.swap(alpha, beta);
            }
            let output = root.join(format!("{stale_kind}-output.xml"));
            candidate.xml_folder_sibling_export_path = output.display().to_string();
            candidate.export_xml_sibling_folder_reorder();
            assert!(candidate.xml_folder_sibling_pending.is_none());
            assert!(!output.exists());
            assert_eq!(
                std::fs::read(source).unwrap(),
                std::fs::read(fixture).unwrap()
            );
        }
        let (mut root_parent, root_source) = setup("root-parent", "Beta, Alpha");
        root_parent.xml_folder_sibling_parent = Some(0);
        root_parent.prepare_xml_sibling_folder_reorder();
        assert!(root_parent.xml_folder_sibling_pending.is_none());
        assert_eq!(
            std::fs::read(root_source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
        for invalid in ["Beta, Beta", "Beta, Missing", "Beta"] {
            let (candidate, path) = setup("invalid", invalid);
            assert!(
                candidate.xml_folder_sibling_pending.is_none(),
                "accepted {invalid}"
            );
            assert_eq!(
                std::fs::read(path).unwrap(),
                std::fs::read(fixture).unwrap()
            );
        }
        let (mut cancelled, cancel_source) = setup("cancel", "Beta, Alpha");
        cancelled.cancel_xml_sibling_folder_reorder();
        let cancelled_output = root.join("cancel-output.xml");
        cancelled.xml_folder_sibling_export_path = cancelled_output.display().to_string();
        cancelled.export_xml_sibling_folder_reorder();
        assert!(!cancelled_output.exists());
        assert_eq!(
            std::fs::read(cancel_source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
        let (mut alias, alias_source) = setup("alias", "Beta, Alpha");
        alias.xml_folder_sibling_export_path = alias_source.display().to_string();
        alias.export_xml_sibling_folder_reorder();
        assert_eq!(
            std::fs::read(alias_source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
        let (mut existing, existing_source) = setup("existing", "Beta, Alpha");
        let protected = root.join("protected.xml");
        std::fs::write(&protected, b"keep").unwrap();
        existing.xml_folder_sibling_export_path = protected.display().to_string();
        existing.export_xml_sibling_folder_reorder();
        assert_eq!(std::fs::read(protected).unwrap(), b"keep");
        assert_eq!(
            std::fs::read(existing_source).unwrap(),
            std::fs::read(fixture).unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_subtree_move_ui_prepares_cancels_exports_and_reopens_oracle_checked_bytes() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-move-{}-{}",
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
                "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|f| f.name == "ToMove")
            .unwrap();
        let target = preview
            .folders
            .iter()
            .find(|f| f.name == "Dest")
            .unwrap()
            .ordinal;
        app.xml_folder_move_selection = Some(index);
        app.xml_folder_move_destination = Some(target);
        app.prepare_xml_folder_move(index);
        let pending = app.xml_pending_folder_move.as_ref().unwrap();
        assert_eq!(pending.old_path, ["ROOT", "From", "ToMove"]);
        assert_eq!(pending.new_path, ["Dest", "ToMove"]);
        assert_eq!(
            app.xml_pending_folder_move
                .as_ref()
                .unwrap()
                .expected_source_order,
            ["0:ToMove", "1:Remain"]
        );
        assert_eq!(
            app.xml_pending_folder_move
                .as_ref()
                .unwrap()
                .expected_target_order,
            ["1:Existing"]
        );
        let prepared = pending.prepared.bytes().to_vec();
        app.cancel_xml_folder_move();
        assert!(app.xml_pending_folder_move.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        app.prepare_xml_folder_move(index);
        let output = root.join("moved.xml");
        app.xml_folder_move_export_path = output.display().to_string();
        app.export_xml_folder_move();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared);
        f02_folder_move_oracle::check(&bytes).unwrap();
        let moved_subtree = b"<NODE Type=\"0\" Name=\"ToMove\" Count=\"1\"><NODE Type=\"0\" Name=\"Nested\" Count=\"1\"><NODE Type=\"1\" Name=\"Inside\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"1\"/><TRACK Key=\"3\"/></NODE></NODE></NODE>";
        assert!(
            bytes
                .windows(moved_subtree.len())
                .any(|window| window == moved_subtree)
        );
        let collection = b"<COLLECTION Entries=\"3\"><TRACK TrackID=\"1\" Name=\"one\" Location=\"file://localhost/one\"/><TRACK TrackID=\"2\" Name=\"two\" Location=\"file://localhost/two\"/><TRACK TrackID=\"3\" Name=\"three\" Location=\"file://localhost/three\"/></COLLECTION>";
        assert!(
            bytes
                .windows(collection.len())
                .any(|window| window == collection)
        );
        assert_eq!(
            direct_xml_typed_child_order(&output, &["From".into()]).unwrap(),
            ["1:Remain"]
        );
        assert_eq!(
            direct_xml_typed_child_order(&output, &["Dest".into()]).unwrap(),
            ["1:Existing", "0:ToMove"]
        );
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert!(
            reopened
                .folders
                .iter()
                .any(|f| f.path == ["ROOT", "Dest", "ToMove", "Nested"])
        );
        assert!(
            reopened
                .playlists
                .iter()
                .any(|p| p.path == ["Dest", "ToMove", "Nested", "Inside"]
                    && p.name == "Inside"
                    && p.tracks == ["1", "3"])
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.name == "From")
                .unwrap()
                .count
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.name == "Dest")
                .unwrap()
                .count
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.name == "Nested")
                .unwrap()
                .count
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.path == ["ROOT"])
                .unwrap()
                .count
                .as_deref(),
            Some("2")
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_subtree_move_ui_rejects_stale_typed_child_order_and_preserves_aliases() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-move-stale-{}-{}",
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
            "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
        );
        std::fs::copy(fixture, &source).unwrap();
        let app_factory = || app();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|f| f.name == "ToMove")
            .unwrap();
        let target = preview
            .folders
            .iter()
            .find(|f| f.name == "Dest")
            .unwrap()
            .ordinal;
        app.xml_folder_move_selection = Some(index);
        app.xml_folder_move_destination = Some(target);
        app.prepare_xml_folder_move(index);
        let original = std::fs::read_to_string(&source).unwrap();
        let left = "<NODE Type=\"0\" Name=\"ToMove\" Count=\"1\"><NODE Type=\"0\" Name=\"Nested\" Count=\"1\"><NODE Type=\"1\" Name=\"Inside\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"1\"/><TRACK Key=\"3\"/></NODE></NODE></NODE>";
        let right = "<NODE Type=\"1\" Name=\"Remain\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"2\"/></NODE>";
        assert!(original.contains(&format!("{left}{right}")));
        std::fs::write(
            &source,
            original.replacen(&format!("{left}{right}"), &format!("{right}{left}"), 1),
        )
        .unwrap();
        let changed = std::fs::read(&source).unwrap();
        app.xml_folder_move_export_path = root.join("stale.xml").display().to_string();
        app.export_xml_folder_move();
        assert!(app.xml_pending_folder_move.is_none());
        assert!(!root.join("stale.xml").exists());
        assert_eq!(std::fs::read(&source).unwrap(), changed);
        app.xml_document_source = Some(source.clone());
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let alias_index = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .position(|f| f.name == "ToMove")
            .unwrap();
        let dest = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Dest")
            .unwrap()
            .ordinal;
        app.xml_folder_move_selection = Some(alias_index);
        app.xml_folder_move_destination = Some(dest);
        app.prepare_xml_folder_move(alias_index);
        app.xml_folder_move_export_path = source.display().to_string();
        app.export_xml_folder_move();
        assert_eq!(std::fs::read(&source).unwrap(), changed);

        for stale_kind in ["source", "selection", "source-parent", "target-parent"] {
            let stale_source = root.join(format!("{stale_kind}-source.xml"));
            std::fs::copy(fixture, &stale_source).unwrap();
            let mut candidate = app_factory();
            candidate.xml_path = stale_source.display().to_string();
            candidate.preview_xml();
            let preview = candidate.xml_preview.as_ref().unwrap().as_ref().unwrap();
            let stale_index = preview
                .folders
                .iter()
                .position(|folder| folder.name == "ToMove")
                .unwrap();
            let stale_target = preview
                .folders
                .iter()
                .find(|folder| folder.name == "Dest")
                .unwrap()
                .ordinal;
            candidate.xml_folder_move_selection = Some(stale_index);
            candidate.xml_folder_move_destination = Some(stale_target);
            candidate.prepare_xml_folder_move(stale_index);
            assert!(candidate.xml_pending_folder_move.is_some());
            match stale_kind {
                "source" => candidate.xml_document_source = Some(root.join("other.xml")),
                "selection" => candidate.xml_folder_move_selection = Some(stale_index + 1),
                "source-parent" | "target-parent" => {
                    let parent_name = if stale_kind == "source-parent" {
                        "From"
                    } else {
                        "Dest"
                    };
                    candidate
                        .xml_preview
                        .as_mut()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .folders
                        .iter_mut()
                        .find(|folder| folder.name == parent_name)
                        .unwrap()
                        .name = "Changed".into();
                }
                _ => unreachable!(),
            }
            let output = root.join(format!("{stale_kind}-output.xml"));
            candidate.xml_folder_move_export_path = output.display().to_string();
            candidate.export_xml_folder_move();
            assert!(candidate.xml_pending_folder_move.is_none(), "{stale_kind}");
            assert!(!output.exists(), "{stale_kind}");
            assert_eq!(
                std::fs::read(stale_source).unwrap(),
                std::fs::read(fixture).unwrap()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_subtree_move_ui_refuses_typed_child_order_read_errors() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-move-read-error-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
        );

        for fail_during_export in [false, true] {
            let source = root.join(if fail_during_export {
                "export-source.xml"
            } else {
                "prepare-source.xml"
            });
            std::fs::copy(fixture, &source).unwrap();
            let mut app = app();
            app.xml_path = source.display().to_string();
            app.preview_xml();
            let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
            let index = preview
                .folders
                .iter()
                .position(|folder| folder.name == "ToMove")
                .unwrap();
            let target = preview
                .folders
                .iter()
                .find(|folder| folder.name == "Dest")
                .unwrap()
                .ordinal;
            app.xml_folder_move_selection = Some(index);
            app.xml_folder_move_destination = Some(target);

            if fail_during_export {
                app.prepare_xml_folder_move(index);
                assert!(app.xml_pending_folder_move.is_some());
            }
            std::fs::write(&source, b"<NODE Type='0' Name=\"broken>").unwrap();
            let output = root.join(format!("output-{fail_during_export}.xml"));
            app.xml_folder_move_export_path = output.display().to_string();
            if fail_during_export {
                app.export_xml_folder_move();
                assert!(app.xml_pending_folder_move.is_none());
            } else {
                app.prepare_xml_folder_move(index);
                assert!(app.xml_pending_folder_move.is_none());
            }
            assert!(app.xml_edit_error.as_deref().is_some_and(
                |error| error.contains("cannot read") && error.contains("child order")
            ));
            assert!(!output.exists());
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_subtree_move_ui_refuses_invalid_sources_parents_and_duplicate_children() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-move-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
        );
        let fixture_xml = std::fs::read_to_string(fixture).unwrap();
        let setup = |name: &str, xml: &str| {
            let source = root.join(format!("{name}.xml"));
            std::fs::write(&source, xml).unwrap();
            let mut app = app();
            app.xml_path = source.display().to_string();
            app.preview_xml();
            let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
            let index = preview
                .folders
                .iter()
                .position(|folder| folder.name == "ToMove")
                .unwrap();
            let source_parent = preview
                .folders
                .iter()
                .find(|folder| folder.name == "From")
                .unwrap()
                .ordinal;
            let target = preview
                .folders
                .iter()
                .find(|folder| folder.name == "Dest")
                .unwrap()
                .ordinal;
            let nested = preview
                .folders
                .iter()
                .find(|folder| folder.name == "Nested")
                .unwrap()
                .ordinal;
            app.xml_folder_move_selection = Some(index);
            app.xml_folder_move_destination = Some(target);
            (app, source, index, source_parent, target, nested)
        };

        for (name, source_index, destination) in [
            ("root-source", Some(0), None),
            ("out-of-range", Some(usize::MAX), None),
            ("same-parent", None, Some("source")),
            ("cycle", None, Some("descendant")),
            ("root-target", None, Some("root")),
        ] {
            let (mut candidate, source, index, source_parent, _target, nested) =
                setup(name, &fixture_xml);
            if let Some(destination) = destination {
                candidate.xml_folder_move_destination = Some(match destination {
                    "source" => source_parent,
                    "descendant" => nested,
                    "root" => {
                        candidate
                            .xml_preview
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .folders
                            .iter()
                            .find(|folder| folder.path.len() == 1)
                            .unwrap()
                            .ordinal
                    }
                    _ => unreachable!(),
                });
            }
            let selected = source_index.unwrap_or(index);
            candidate.prepare_xml_folder_move(selected);
            assert!(candidate.xml_pending_folder_move.is_none(), "{name}");
            assert_eq!(std::fs::read(source).unwrap(), fixture_xml.as_bytes());
        }

        let duplicate_xml = fixture_xml.replacen(
            "</NODE><NODE Type=\"1\" Name=\"Remain\"",
            "</NODE><NODE Type=\"0\" Name=\"ToMove\" Count=\"0\"/><NODE Type=\"1\" Name=\"Remain\"",
            1,
        );
        assert_ne!(duplicate_xml, fixture_xml);
        let (mut duplicate, duplicate_source, index, _, _, _) =
            setup("duplicate-child", &duplicate_xml);
        duplicate.prepare_xml_folder_move(index);
        assert!(duplicate.xml_pending_folder_move.is_none());
        assert_eq!(
            std::fs::read(duplicate_source).unwrap(),
            duplicate_xml.as_bytes()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_subtree_move_ui_supports_self_closing_destination_and_protects_existing_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-folder-move-self-closing-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-folder-move/source.xml"
        );
        let fixture_xml = std::fs::read_to_string(fixture).unwrap();
        let xml = fixture_xml.replace(
            "<NODE Type=\"0\" Name=\"Dest\" Count=\"1\"><NODE Type=\"1\" Name=\"Existing\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"3\"/></NODE></NODE>",
            "<NODE Type=\"0\" Name=\"Dest\" Count=\"0\"/>",
        );
        assert_ne!(xml, fixture_xml);
        let source = root.join("source.xml");
        std::fs::write(&source, xml.as_bytes()).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let preview = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let index = preview
            .folders
            .iter()
            .position(|folder| folder.name == "ToMove")
            .unwrap();
        let target = preview
            .folders
            .iter()
            .find(|folder| folder.name == "Dest")
            .unwrap();
        let target_ordinal = target.ordinal;
        assert_eq!(target.count.as_deref(), Some("0"));
        app.xml_folder_move_selection = Some(index);
        app.xml_folder_move_destination = Some(target_ordinal);
        app.prepare_xml_folder_move(index);
        let pending = app.xml_pending_folder_move.as_ref().unwrap();
        assert_eq!(pending.expected_target_order, Vec::<String>::new());
        assert_eq!(pending.target_count.as_deref(), Some("0"));

        let protected = root.join("protected.xml");
        std::fs::write(&protected, b"keep").unwrap();
        app.xml_folder_move_export_path = protected.display().to_string();
        app.export_xml_folder_move();
        assert_eq!(std::fs::read(&protected).unwrap(), b"keep");
        assert_eq!(std::fs::read(&source).unwrap(), xml.as_bytes());

        let output = root.join("moved.xml");
        app.xml_folder_move_export_path = output.display().to_string();
        app.export_xml_folder_move();
        let reopened = oxidj_application::preview_rekordbox_xml(&output).unwrap();
        assert!(
            reopened
                .folders
                .iter()
                .any(|folder| folder.path == ["ROOT", "Dest", "ToMove"])
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|folder| folder.name == "Dest")
                .unwrap()
                .count
                .as_deref(),
            Some("1")
        );
        assert_eq!(std::fs::read(&source).unwrap(), xml.as_bytes());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_move_ui_exports_prepared_bytes_and_runs_independent_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-move-{}-{}",
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
                "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
            ),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let catalog = root.join("catalog.sqlite");
        let mut app = LibraryApp::new(
            catalog.clone(),
            ThemeMode::TokyoNight,
            ThemeMode::TokyoNight.tokens(),
        );
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let source_tracks: Vec<_> = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .map(|track| (track.id.clone(), track.name.clone(), track.location.clone()))
            .collect();
        let target = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .ordinal;
        app.xml_selected_playlist = Some(0);
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        let prepared = app
            .xml_pending_move
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(
            app.xml_pending_move.as_ref().unwrap().old_path,
            ["Sets", "House"]
        );
        assert_eq!(
            app.xml_pending_move.as_ref().unwrap().new_path,
            ["Target", "House"]
        );
        app.cancel_xml_playlist_move();
        assert!(app.xml_pending_move.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        app.prepare_xml_playlist_move(0);
        app.xml_move_export_path = source.display().to_string();
        app.export_xml_playlist_move();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "source alias cannot overwrite"
        );
        assert!(app.xml_pending_move.is_none());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.prepare_xml_playlist_move(0);
        app.xml_move_export_path = existing.display().to_string();
        app.export_xml_playlist_move();
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        assert!(app.xml_pending_move.is_none());
        app.prepare_xml_playlist_move(0);
        let out = root.join("moved.xml");
        app.xml_move_export_path = out.display().to_string();
        app.export_xml_playlist_move();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(bytes, prepared);
        f02_playlist_move_oracle::check(&bytes).unwrap();
        let reopened = oxidj_application::preview_rekordbox_xml(&out).unwrap();
        assert!(
            reopened
                .playlists
                .iter()
                .any(|p| p.path == ["Target", "House"]
                    && p.name == "House"
                    && p.tracks == ["100", "200", "300"]),
            "reopened: {:?}",
            reopened.playlists
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.name == "Sets")
                .unwrap()
                .count
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            reopened
                .folders
                .iter()
                .find(|f| f.name == "Target")
                .unwrap()
                .count
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .map(|track| (track.id.clone(), track.name.clone(), track.location.clone()))
                .collect::<Vec<_>>(),
            source_tracks,
            "collection tracks must be unchanged"
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original,
            "source bytes must remain unchanged"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_move_ui_invalidates_source_selection_and_destination_changes() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-move-stale-{}-{}",
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
                "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
            ),
            &source,
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let target = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .ordinal;
        let sets = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Sets")
            .unwrap()
            .ordinal;
        app.xml_selected_playlist = Some(0);
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        app.select_xml_playlist(1);
        assert!(
            app.xml_pending_move.is_none(),
            "selection change cancels pending move"
        );
        app.xml_selected_playlist = Some(0);
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        app.xml_move_destination = Some(sets);
        app.xml_move_export_path = root.join("destination-stale.xml").display().to_string();
        app.export_xml_playlist_move();
        assert!(app.xml_pending_move.is_none());
        assert!(!root.join("destination-stale.xml").exists());
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        let mut changed = std::fs::read(&source).unwrap();
        changed.push(b' ');
        std::fs::write(&source, changed.clone()).unwrap();
        app.xml_move_export_path = root.join("source-stale.xml").display().to_string();
        app.export_xml_playlist_move();
        assert!(app.xml_pending_move.is_none());
        assert!(!root.join("source-stale.xml").exists());
        assert_eq!(std::fs::read(&source).unwrap(), changed);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_move_ui_refuses_invalid_destinations_without_mutation() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-move-refuse-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
        );
        let source = root.join("source.xml");
        std::fs::copy(fixture, &source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut candidate = app();
        candidate.xml_path = source.display().to_string();
        candidate.preview_xml();
        let folders = &candidate
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders;
        let sets = folders.iter().find(|f| f.name == "Sets").unwrap().ordinal;
        let root_ordinal = folders.iter().find(|f| f.path.len() == 1).unwrap().ordinal;
        candidate.xml_move_destination = Some(sets);
        candidate.prepare_xml_playlist_move(0);
        assert!(candidate.xml_pending_move.is_none(), "same-parent refused");
        candidate.xml_move_destination = Some(root_ordinal);
        candidate.prepare_xml_playlist_move(0);
        assert!(candidate.xml_pending_move.is_none(), "root refused");
        candidate.xml_move_destination = Some(usize::MAX);
        candidate.prepare_xml_playlist_move(0);
        assert!(candidate.xml_pending_move.is_none(), "out-of-range refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(!root.join("output.xml").exists());

        let duplicate = root.join("duplicate.xml");
        let mut bytes = original.clone();
        let marker = b"<NODE Type=\"1\" Name=\"Other\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE>";
        let marker_at = bytes
            .windows(marker.len())
            .position(|w| w == marker)
            .unwrap();
        bytes.splice(marker_at..marker_at, marker.iter().copied());
        std::fs::write(&duplicate, &bytes).unwrap();
        let mut ambiguous = app();
        ambiguous.xml_path = duplicate.display().to_string();
        ambiguous.preview_xml();
        let target = ambiguous
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .ordinal;
        ambiguous.xml_move_destination = Some(target);
        ambiguous.prepare_xml_playlist_move(0);
        assert!(
            ambiguous.xml_pending_move.is_none(),
            "ambiguous duplicate child path refused"
        );
        assert_eq!(std::fs::read(&duplicate).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_move_ui_refuses_source_and_destination_order_changes() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-move-order-{}-{}",
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
            "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
        ))
        .unwrap();
        let house = "<NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\"><TRACK Key=\"100\"/><TRACK Key=\"200\"/><TRACK Key=\"300\"/></NODE>";
        let techno = "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\"><TRACK Key=\"200\"/><TRACK Key=\"400\"/></NODE>";
        let other = "<NODE Type=\"1\" Name=\"Other\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE>";
        let extra = "<NODE Type=\"1\" Name=\"Extra\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"300\"/></NODE>";
        for (label, initial, before, after) in [
            (
                "source",
                String::from_utf8(fixture.clone()).unwrap(),
                format!("{house}{techno}"),
                format!("{techno}{house}"),
            ),
            (
                "destination",
                String::from_utf8(fixture.clone()).unwrap().replace(
                    "Name=\"Target\" Count=\"1\"><NODE",
                    &format!("Name=\"Target\" Count=\"2\">{extra}<NODE"),
                ),
                format!("{extra}{other}"),
                format!("{other}{extra}"),
            ),
        ] {
            std::fs::write(&source, initial).unwrap();
            let mut app = app();
            app.xml_path = source.display().to_string();
            app.preview_xml();
            let target = app
                .xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .folders
                .iter()
                .find(|f| f.name == "Target")
                .unwrap()
                .ordinal;
            app.xml_move_destination = Some(target);
            app.prepare_xml_playlist_move(0);
            assert!(
                app.xml_pending_move.is_some(),
                "valid initial preparation for {label}"
            );
            let mut xml = std::fs::read_to_string(&source).unwrap();
            assert!(
                xml.contains(&before),
                "expected original direct-child block order for {label}"
            );
            xml = xml.replacen(&before, &after, 1);
            std::fs::write(&source, xml.as_bytes()).unwrap();
            let changed = std::fs::read(&source).unwrap();
            app.xml_move_export_path = root
                .join(format!("{label}-order.xml"))
                .display()
                .to_string();
            app.export_xml_playlist_move();
            assert!(app.xml_pending_move.is_none());
            assert!(!root.join(format!("{label}-order.xml")).exists());
            assert_eq!(std::fs::read(&source).unwrap(), changed);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn playlist_move_ui_accepts_mixed_folder_and_playlist_children() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-playlist-move-mixed-{}-{}",
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
            "/../../xtask/tests/fixtures/f02-playlist-move/source.xml"
        ))
        .unwrap();
        let xml = String::from_utf8(fixture).unwrap().replace(
            "Name=\"Target\" Count=\"1\"><NODE",
            "Name=\"Target\" Count=\"2\"><NODE Type=\"0\" Name=\"Sub\" Count=\"0\"/><NODE",
        );
        std::fs::write(&source, xml).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        let target = app
            .xml_preview
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.name == "Target")
            .unwrap()
            .ordinal;
        assert_eq!(
            direct_xml_node_order(&source, &["Target".into()]).unwrap(),
            ["Sub", "Other"]
        );
        app.xml_selected_playlist = Some(0);
        app.xml_move_destination = Some(target);
        app.prepare_xml_playlist_move(0);
        assert!(
            app.xml_pending_move.is_some(),
            "mixed direct child order should prepare successfully"
        );
        std::fs::remove_dir_all(root).unwrap();
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
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        let reopened_order: Vec<_> = reopened
            .playlists
            .iter()
            .filter(|playlist| playlist.path.len() == 2 && playlist.path[0] == "Sets")
            .map(|playlist| playlist.name.as_str())
            .collect();
        assert_eq!(reopened_order, ["Techno", "House"]);
        assert_eq!(
            app.xml_document_source.as_ref().unwrap(),
            &root.join("reordered.xml")
        );
        // A second export to the same path is refused and cannot overwrite the valid export.
        let exported = std::fs::read(root.join("reordered.xml")).unwrap();
        app.export_xml_sibling_playlist_reorder();
        assert_eq!(std::fs::read(root.join("reordered.xml")).unwrap(), exported);
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
    fn sibling_playlist_reorder_ui_guards_reject_stale_invalid_cancel_and_existing_destination() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-sibling-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml"
        );
        let setup = |label: &str| {
            let source = root.join(format!("{label}.xml"));
            std::fs::copy(fixture, &source).unwrap();
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
            (app, source)
        };

        for invalid in ["House, House", "House, Missing", "House"] {
            let (mut app, source) = setup("invalid");
            app.xml_sibling_order = invalid.into();
            app.prepare_xml_sibling_playlist_reorder();
            assert!(app.xml_sibling_pending.is_none(), "accepted {invalid}");
            let destination = root.join("invalid-output.xml");
            app.xml_sibling_export_path = destination.display().to_string();
            app.export_xml_sibling_playlist_reorder();
            assert!(!destination.exists());
            assert_eq!(
                std::fs::read(source).unwrap(),
                std::fs::read(fixture).unwrap()
            );
        }

        for stale in ["source", "parent", "order"] {
            let (mut app, source) = setup(stale);
            match stale {
                "source" => app.xml_document_source = Some(root.join("different.xml")),
                "parent" => {
                    app.xml_preview
                        .as_mut()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .folders
                        .iter_mut()
                        .find(|folder| folder.name == "Sets")
                        .unwrap()
                        .name = "Renamed".into()
                }
                "order" => {
                    let preview = app.xml_preview.as_mut().unwrap().as_mut().unwrap();
                    let house = preview
                        .playlists
                        .iter()
                        .position(|p| p.name == "House")
                        .unwrap();
                    let techno = preview
                        .playlists
                        .iter()
                        .position(|p| p.name == "Techno")
                        .unwrap();
                    preview.playlists.swap(house, techno);
                }
                _ => unreachable!(),
            }
            let destination = root.join(format!("{stale}-output.xml"));
            app.xml_sibling_export_path = destination.display().to_string();
            app.export_xml_sibling_playlist_reorder();
            assert!(app.xml_sibling_pending.is_none());
            assert!(!destination.exists(), "stale {stale} created output");
            assert_eq!(
                std::fs::read(source).unwrap(),
                std::fs::read(fixture).unwrap()
            );
        }

        let (mut boundary, source) = setup("boundary");
        let original = std::fs::read(&source).unwrap();
        boundary.xml_sibling_parent = Some(usize::MAX);
        boundary.prepare_xml_sibling_playlist_reorder();
        assert!(boundary.xml_sibling_pending.is_none());
        let destination = root.join("boundary-output.xml");
        boundary.xml_sibling_export_path = destination.display().to_string();
        boundary.export_xml_sibling_playlist_reorder();
        assert!(!destination.exists());
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let (mut cancelled, source) = setup("cancel");
        let original = std::fs::read(&source).unwrap();
        cancelled.cancel_xml_sibling_playlist_reorder();
        assert!(cancelled.xml_sibling_pending.is_none());
        let destination = root.join("cancel-output.xml");
        cancelled.xml_sibling_export_path = destination.display().to_string();
        cancelled.export_xml_sibling_playlist_reorder();
        assert!(!destination.exists());
        assert_eq!(std::fs::read(source).unwrap(), original);

        let (mut refused, source) = setup("destination");
        let destination = root.join("already-exists.xml");
        std::fs::write(&destination, b"do not overwrite").unwrap();
        refused.xml_sibling_export_path = destination.display().to_string();
        refused.export_xml_sibling_playlist_reorder();
        assert_eq!(std::fs::read(destination).unwrap(), b"do not overwrite");
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(fixture).unwrap()
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
    fn track_artist_edit_ui_exports_prepared_bytes_and_checks_independent_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-artist-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_artist, "Old & exact");
        app.xml_edit_artist = "New Artist".into();
        app.prepare_xml_artist_edit("100", Some("Old & exact"));
        let prepared_bytes = app
            .xml_pending_artist_edit
            .as_ref()
            .expect("prepared Artist edit")
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

        let cancelled = root.join("cancelled.xml");
        app.xml_artist_export_path = cancelled.display().to_string();
        app.cancel_xml_artist_edit();
        assert!(app.xml_pending_artist_edit.is_none());
        assert!(!cancelled.exists(), "cancel must not write output");

        let output = root.join("edited.xml");
        app.prepare_xml_artist_edit("100", Some("Old & exact"));
        app.xml_artist_export_path = output.display().to_string();
        app.export_xml_artist_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared_bytes);
        f02_track_artist_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts only the selected Artist change");
        let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
            "Artist=\"New Artist\"",
            "Artist=\"Altered\"",
            1,
        );
        assert_ne!(mutated.as_bytes(), bytes.as_slice());
        assert!(f02_track_artist_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .artist
                .as_deref(),
            Some("New Artist")
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "200")
                .unwrap()
                .artist,
            None,
            "absent Artist remains absent on other tracks"
        );
        assert!(app.xml_pending_artist_edit.is_none());
        assert!(app.notice.contains(&output.display().to_string()));
        assert!(app.xml_edit_error.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_artist_ui_escapes_xml_and_refuses_changed_proposals_and_absent_values() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-artist-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();

        app.select_xml_track("200", "B");
        app.prepare_xml_artist_edit("200", None);
        assert!(app.xml_pending_artist_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("no Artist attribute")
        );

        app.select_xml_track("100", "A");
        app.xml_edit_artist = "New < Artist & \"guest\"".into();
        app.prepare_xml_artist_edit("100", Some("Old & exact"));
        let pending = app.xml_pending_artist_edit.as_ref().unwrap();
        let expected = pending.prepared.change().new_artist.clone();
        let output = root.join("escaped.xml");
        app.xml_artist_export_path = output.display().to_string();
        app.xml_edit_artist.push('!');
        app.export_xml_artist_edit();
        assert!(app.xml_pending_artist_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("proposed value changed")
        );
        assert!(!output.exists());

        app.xml_edit_artist = expected;
        app.prepare_xml_artist_edit("100", Some("Old & exact"));
        app.export_xml_artist_edit();
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("New &lt; Artist &amp; &quot;guest&quot;"));
        f02_track_artist_edit_oracle::check_with_artist(&bytes, "New < Artist & \"guest\"")
            .expect("independent whole-tree oracle accepts escaped UI output");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_artist_ui_rejects_stale_source_without_creating_output() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"),
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_artist = "Changed".into();
        app.prepare_xml_artist_edit("100", Some("Old & exact"));
        std::fs::write(&source, b"<changed/>").unwrap();
        let output = root.join("stale.xml");
        app.xml_artist_export_path = output.display().to_string();
        app.export_xml_artist_edit();
        assert!(app.xml_pending_artist_edit.is_some());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("source changed")
        );
        assert!(!output.exists());
        std::fs::write(&source, original).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_comments_edit_ui_exports_previewed_bytes_through_independent_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-comments-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_comments, "old & note");
        app.xml_edit_comments = "new < note".into();
        app.prepare_xml_comments_edit("100", Some("old & note"));
        let prepared_bytes = app
            .xml_pending_comments_edit
            .as_ref()
            .expect("prepared Comments edit")
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

        let cancelled = root.join("cancelled.xml");
        app.xml_comments_export_path = cancelled.display().to_string();
        app.cancel_xml_comments_edit();
        assert!(app.xml_pending_comments_edit.is_none());
        assert!(!cancelled.exists(), "cancel must not write output");

        app.prepare_xml_comments_edit("100", Some("old & note"));
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_comments_export_path = existing.display().to_string();
        app.export_xml_comments_edit();
        assert!(
            app.xml_edit_error.is_some(),
            "export errors must be surfaced"
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        assert_eq!(std::fs::read(&source).unwrap(), original);

        app.xml_comments_export_path = root.join("edited.xml").display().to_string();
        app.export_xml_comments_edit();
        let output = PathBuf::from(&app.xml_comments_export_path);
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared_bytes);
        f02_track_comments_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts only selected Comments edit");
        let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
            "Comments=\"new &lt; note\"",
            "Comments=\"altered\"",
            1,
        );
        assert_ne!(mutated.as_bytes(), bytes.as_slice());
        assert!(f02_track_comments_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .comments
                .as_deref(),
            Some("new < note")
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "200")
                .unwrap()
                .comments,
            None,
            "absent Comments remains absent on other tracks"
        );
        assert!(app.xml_pending_comments_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_comments_edit_ui_refuses_absent_and_changed_proposal() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-comments-guards-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();

        app.select_xml_track("200", "B");
        app.prepare_xml_comments_edit("200", None);
        assert!(app.xml_pending_comments_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("no Comments attribute")
        );

        app.select_xml_track("100", "A");
        app.xml_edit_comments = "new < note".into();
        app.prepare_xml_comments_edit("100", Some("old & note"));
        app.xml_edit_comments.push('!');
        app.xml_comments_export_path = root.join("changed.xml").display().to_string();
        app.export_xml_comments_edit();
        assert!(app.xml_pending_comments_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("proposed value changed")
        );
        assert!(!root.join("changed.xml").exists());

        app.xml_edit_comments = "new < note".into();
        app.prepare_xml_comments_edit("100", Some("old & note"));
        assert!(app.xml_pending_comments_edit.is_some());
        app.select_xml_track("200", "B");
        assert!(app.xml_pending_comments_edit.is_none());

        app.select_xml_track("100", "A");
        app.prepare_xml_comments_edit("100", Some("old & note"));
        assert!(app.xml_pending_comments_edit.is_some());
        app.preview_xml();
        assert!(app.xml_pending_comments_edit.is_none());

        let empty_comments = std::fs::read(&source)
            .unwrap()
            .windows(b"Comments=\"old &amp; note\"".len())
            .position(|window| window == b"Comments=\"old &amp; note\"")
            .expect("fixture Comments attribute") as usize;
        let mut empty_fixture = std::fs::read(&source).unwrap();
        empty_fixture.splice(
            empty_comments..empty_comments + b"Comments=\"old &amp; note\"".len(),
            b"Comments=\"\"".iter().copied(),
        );
        std::fs::write(&source, empty_fixture).unwrap();
        app.preview_xml();
        app.select_xml_track("100", "A");
        assert!(app.xml_edit_comments.is_empty());
        app.prepare_xml_comments_edit("100", Some(""));
        assert!(app.xml_pending_comments_edit.is_some());
        app.cancel_xml_comments_edit();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_comments_edit_ui_rejects_stale_source_without_output() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"),
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_comments = "new < note".into();
        app.prepare_xml_comments_edit("100", Some("old & note"));
        std::fs::write(&source, b"<changed/>").unwrap();
        let output = root.join("stale.xml");
        app.xml_comments_export_path = output.display().to_string();
        app.export_xml_comments_edit();
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("source changed")
        );
        assert!(!output.exists());
        std::fs::write(&source, original).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_genre_edit_ui_exports_previewed_bytes_and_reopens_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-genre-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_genre, "old & style");
        app.xml_edit_genre = "new < style".into();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        let prepared = app
            .xml_pending_genre_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let cancelled = root.join("cancelled.xml");
        app.xml_genre_export_path = cancelled.display().to_string();
        app.cancel_xml_genre_edit();
        assert!(app.xml_pending_genre_edit.is_none());
        assert!(!cancelled.exists());
        app.prepare_xml_genre_edit("100", Some("old & style"));
        app.xml_genre_export_path.clear();
        app.export_xml_genre_edit();
        assert!(app.xml_pending_genre_edit.is_some());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("choose a new XML destination"));
        app.xml_genre_export_path = source.display().to_string();
        app.export_xml_genre_edit();
        assert!(app.xml_edit_error.is_some(), "source alias must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_genre_export_path = existing.display().to_string();
        app.export_xml_genre_edit();
        assert!(app.xml_edit_error.is_some(), "existing output must be refused");
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        app.xml_genre_export_path = root.join("missing/output.xml").display().to_string();
        app.export_xml_genre_edit();
        assert!(app.xml_edit_error.is_some(), "write failure must be surfaced");
        assert!(app.notice.is_empty(), "failure must not claim success");
        app.xml_genre_export_path = root.join("stale-source.xml").display().to_string();
        std::fs::write(&source, b"<changed/>").unwrap();
        app.export_xml_genre_edit();
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
        assert!(!root.join("stale-source.xml").exists());
        std::fs::write(&source, &original).unwrap();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        app.xml_edit_genre = "changed after preview".into();
        let stale_proposal = root.join("stale-proposal.xml");
        app.xml_genre_export_path = stale_proposal.display().to_string();
        app.export_xml_genre_edit();
        assert!(!stale_proposal.exists());
        assert!(app.xml_pending_genre_edit.is_none());
        app.xml_edit_genre = "new < style".into();
        app.prepare_xml_genre_edit("100", Some("stale old value"));
        assert!(app.xml_pending_genre_edit.is_none());
        app.prepare_xml_genre_edit("200", Some("old & style"));
        assert!(
            app.xml_pending_genre_edit.is_none(),
            "absent Genre insertion is refused"
        );
        app.prepare_xml_genre_edit("100", Some("old & style"));
        let changed_selection = root.join("changed-selection.xml");
        app.xml_genre_export_path = changed_selection.display().to_string();
        app.select_xml_track("200", "B");
        app.export_xml_genre_edit();
        assert!(!changed_selection.exists());
        app.prepare_xml_genre_edit("100", Some("old & style"));
        let changed_source = root.join("different.xml");
        app.xml_document_source = Some(changed_source);
        let stale_source = root.join("stale-source.xml");
        app.xml_genre_export_path = stale_source.display().to_string();
        app.export_xml_genre_edit();
        assert!(!stale_source.exists());
        app.xml_document_source = Some(source.clone());
        app.select_xml_track("100", "A");
        app.xml_edit_genre = "new < style".into();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        let output = root.join("edited.xml");
        app.xml_genre_export_path = output.display().to_string();
        app.export_xml_genre_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared);
        f02_track_genre_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts UI-exported Genre bytes");
        assert!(String::from_utf8_lossy(&bytes).contains("Genre=\"new &lt; style\""));
        let mutated = String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("Genre=\"new &lt; style\"", "Genre=\"altered\"", 1);
        assert!(f02_track_genre_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "100")
                .unwrap()
                .genre
                .as_deref(),
            Some("new < style")
        );
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "200")
                .unwrap()
                .genre,
            None,
            "absent Genre remains absent on other tracks"
        );
        assert!(app.xml_pending_genre_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn track_label_edit_ui_exports_previewed_bytes_and_reopens_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-label-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_label, "old & imprint");
        app.xml_edit_label = "new < imprint".into();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        let prepared = app
            .xml_pending_label_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let cancelled = root.join("cancelled.xml");
        app.xml_label_export_path = cancelled.display().to_string();
        app.cancel_xml_label_edit();
        assert!(app.xml_pending_label_edit.is_none());
        assert!(!cancelled.exists());
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        app.xml_label_export_path.clear();
        app.export_xml_label_edit();
        assert!(app.xml_pending_label_edit.is_some());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("choose a new XML destination"));
        app.xml_label_export_path = source.display().to_string();
        app.export_xml_label_edit();
        assert!(app.xml_edit_error.is_some(), "source alias must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_label_export_path = existing.display().to_string();
        app.export_xml_label_edit();
        assert!(app.xml_edit_error.is_some(), "existing output must be refused");
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        app.xml_label_export_path = root.join("missing/output.xml").display().to_string();
        app.export_xml_label_edit();
        assert!(app.xml_edit_error.is_some(), "write failure must be surfaced");
        assert!(app.notice.is_empty(), "failure must not claim success");
        app.xml_label_export_path = root.join("stale-source.xml").display().to_string();
        std::fs::write(&source, b"<changed/>").unwrap();
        app.export_xml_label_edit();
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
        assert!(!root.join("stale-source.xml").exists());
        std::fs::write(&source, &original).unwrap();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        app.xml_edit_label = "changed after preview".into();
        let stale_proposal = root.join("stale-proposal.xml");
        app.xml_label_export_path = stale_proposal.display().to_string();
        app.export_xml_label_edit();
        assert!(!stale_proposal.exists());
        assert!(app.xml_pending_label_edit.is_none());
        app.xml_edit_label = "new < imprint".into();
        app.prepare_xml_label_edit("100", Some("stale old value"));
        assert!(app.xml_pending_label_edit.is_none());
        app.prepare_xml_label_edit("200", Some("old & imprint"));
        assert!(
            app.xml_pending_label_edit.is_none(),
            "absent Label insertion is refused"
        );
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        let changed_selection = root.join("changed-selection.xml");
        app.xml_label_export_path = changed_selection.display().to_string();
        app.select_xml_track("200", "B");
        app.export_xml_label_edit();
        assert!(!changed_selection.exists());
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        let changed_source = root.join("different.xml");
        app.xml_document_source = Some(changed_source);
        let stale_source = root.join("stale-source.xml");
        app.xml_label_export_path = stale_source.display().to_string();
        app.export_xml_label_edit();
        assert!(!stale_source.exists());
        app.xml_document_source = Some(source.clone());
        app.select_xml_track("100", "A");
        app.xml_edit_label = "new < imprint".into();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        let output = root.join("edited.xml");
        app.xml_label_export_path = output.display().to_string();
        app.export_xml_label_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared);
        f02_track_label_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts UI-exported Label bytes");
        assert!(String::from_utf8_lossy(&bytes).contains("Label=\"new &lt; imprint\""));
        let mutated = String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("Label=\"new &lt; imprint\"", "Label=\"altered\"", 1);
        assert!(f02_track_label_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "100")
                .unwrap()
                .label
                .as_deref(),
            Some("new < imprint")
        );
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "200")
                .unwrap()
                .label,
            None,
            "absent Label remains absent on other tracks"
        );
        assert!(app.xml_pending_label_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn track_genre_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("999", "Unknown");
        app.xml_edit_genre = "Changed".into();
        app.prepare_xml_genre_edit("999", Some("old & style"));
        assert!(app.xml_pending_genre_edit.is_none());

        app.select_xml_track("100", "A");
        app.xml_edit_genre = "bad\u{1}".into();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        assert!(app.xml_pending_genre_edit.is_none());

        app.xml_edit_genre = "Changed".into();
        app.prepare_xml_genre_edit("100", Some("stale"));
        assert!(app.xml_pending_genre_edit.is_none());

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("TrackID=\"200\"", "TrackID=\"100\""),
        )
        .unwrap();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_genre = "Changed".into();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        assert!(app.xml_pending_genre_edit.is_none(), "duplicate target must refuse");

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("Genre=\"old &amp; style\"", "Genre=\"\""),
        )
        .unwrap();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_genre, "", "present-empty Genre must stay editable");
        app.xml_edit_genre = "Restored".into();
        app.prepare_xml_genre_edit("100", Some(""));
        assert!(app.xml_pending_genre_edit.is_some(), "present-empty Genre is editable");
        app.cancel_xml_genre_edit();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_composer_edit_ui_exports_previewed_bytes_and_reopens_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-composer-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_composer, "old & composer");
        app.xml_edit_composer = "new < composer".into();
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        let prepared = app
            .xml_pending_composer_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let cancelled = root.join("cancelled.xml");
        app.xml_composer_export_path = cancelled.display().to_string();
        app.cancel_xml_composer_edit();
        assert!(app.xml_pending_composer_edit.is_none());
        assert!(!cancelled.exists());
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        app.xml_composer_export_path.clear();
        app.export_xml_composer_edit();
        assert!(app.xml_pending_composer_edit.is_some());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("choose a new XML destination"));
        app.xml_composer_export_path = source.display().to_string();
        app.export_xml_composer_edit();
        assert!(app.xml_edit_error.is_some(), "source alias must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_composer_export_path = existing.display().to_string();
        app.export_xml_composer_edit();
        assert!(app.xml_edit_error.is_some(), "existing output must be refused");
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        app.xml_composer_export_path = root.join("missing/output.xml").display().to_string();
        app.export_xml_composer_edit();
        assert!(app.xml_edit_error.is_some(), "write failure must be surfaced");
        assert!(app.notice.is_empty(), "failure must not claim success");
        app.xml_composer_export_path = root.join("stale-source.xml").display().to_string();
        std::fs::write(&source, b"<changed/>").unwrap();
        app.export_xml_composer_edit();
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
        assert!(!root.join("stale-source.xml").exists());
        std::fs::write(&source, &original).unwrap();
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        app.xml_edit_composer = "changed after preview".into();
        let stale_proposal = root.join("stale-proposal.xml");
        app.xml_composer_export_path = stale_proposal.display().to_string();
        app.export_xml_composer_edit();
        assert!(!stale_proposal.exists());
        assert!(app.xml_pending_composer_edit.is_none());
        app.xml_edit_composer = "new < composer".into();
        app.prepare_xml_composer_edit("100", Some("stale old value"));
        assert!(app.xml_pending_composer_edit.is_none());
        app.prepare_xml_composer_edit("200", Some("old & composer"));
        assert!(
            app.xml_pending_composer_edit.is_none(),
            "absent Composer insertion is refused"
        );
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        let changed_selection = root.join("changed-selection.xml");
        app.xml_composer_export_path = changed_selection.display().to_string();
        app.select_xml_track("200", "B");
        app.export_xml_composer_edit();
        assert!(!changed_selection.exists());
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        let changed_source = root.join("different.xml");
        app.xml_document_source = Some(changed_source);
        let stale_source = root.join("stale-source.xml");
        app.xml_composer_export_path = stale_source.display().to_string();
        app.export_xml_composer_edit();
        assert!(!stale_source.exists());
        app.xml_document_source = Some(source.clone());
        app.select_xml_track("100", "A");
        app.xml_edit_composer = "new < composer".into();
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        let output = root.join("edited.xml");
        app.xml_composer_export_path = output.display().to_string();
        app.export_xml_composer_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared);
        f02_track_composer_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts UI-exported Composer bytes");
        assert!(String::from_utf8_lossy(&bytes).contains("Composer=\"new &lt; composer\""));
        let mutated = String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("Composer=\"new &lt; composer\"", "Composer=\"altered\"", 1);
        assert!(f02_track_composer_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "100")
                .unwrap()
                .composer
                .as_deref(),
            Some("new < composer")
        );
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "200")
                .unwrap()
                .composer,
            None,
            "absent Composer remains absent on other tracks"
        );
        assert!(app.xml_pending_composer_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn track_grouping_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("999", "Unknown");
        app.xml_edit_grouping = "Changed".into();
        app.prepare_xml_grouping_edit("999", Some("old & crate"));
        assert!(app.xml_pending_grouping_edit.is_none());

        app.select_xml_track("100", "A");
        app.xml_edit_grouping = "bad\u{1}".into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        assert!(app.xml_pending_grouping_edit.is_none());

        app.xml_edit_grouping = "Changed".into();
        app.prepare_xml_grouping_edit("100", Some("stale"));
        assert!(app.xml_pending_grouping_edit.is_none());

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("TrackID=\"200\"", "TrackID=\"100\""),
        )
        .unwrap();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_grouping = "Changed".into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        assert!(app.xml_pending_grouping_edit.is_none(), "duplicate target must refuse");

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("Grouping=\"old &amp; crate\"", "Grouping=\"\""),
        )
        .unwrap();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_grouping, "", "present-empty Grouping must stay editable");
        app.xml_edit_grouping = "Restored".into();
        app.prepare_xml_grouping_edit("100", Some(""));
        assert!(app.xml_pending_grouping_edit.is_some(), "present-empty Grouping is editable");
        app.cancel_xml_grouping_edit();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_grouping_edit_ui_exports_previewed_bytes_and_reopens_output() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-grouping-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_grouping, "old & crate");
        app.xml_edit_grouping = "new < crate".into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        let prepared = app
            .xml_pending_grouping_edit
            .as_ref()
            .unwrap()
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let cancelled = root.join("cancelled.xml");
        app.xml_grouping_export_path = cancelled.display().to_string();
        app.cancel_xml_grouping_edit();
        assert!(app.xml_pending_grouping_edit.is_none());
        assert!(!cancelled.exists());
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        app.xml_grouping_export_path.clear();
        app.export_xml_grouping_edit();
        assert!(app.xml_pending_grouping_edit.is_some());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("choose a new XML destination"));
        app.xml_grouping_export_path = source.display().to_string();
        app.export_xml_grouping_edit();
        assert!(app.xml_edit_error.is_some(), "source alias must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_grouping_export_path = existing.display().to_string();
        app.export_xml_grouping_edit();
        assert!(app.xml_edit_error.is_some(), "existing output must be refused");
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        app.xml_grouping_export_path = root.join("missing/output.xml").display().to_string();
        app.export_xml_grouping_edit();
        assert!(app.xml_edit_error.is_some(), "write failure must be surfaced");
        assert!(app.notice.is_empty(), "failure must not claim success");
        app.xml_grouping_export_path = root.join("stale-source.xml").display().to_string();
        std::fs::write(&source, b"<changed/>").unwrap();
        app.export_xml_grouping_edit();
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source changed"));
        assert!(!root.join("stale-source.xml").exists());
        std::fs::write(&source, &original).unwrap();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        app.xml_edit_grouping = "changed after preview".into();
        let stale_proposal = root.join("stale-proposal.xml");
        app.xml_grouping_export_path = stale_proposal.display().to_string();
        app.export_xml_grouping_edit();
        assert!(!stale_proposal.exists());
        assert!(app.xml_pending_grouping_edit.is_none());
        app.xml_edit_grouping = "new < crate".into();
        app.prepare_xml_grouping_edit("100", Some("stale old value"));
        assert!(app.xml_pending_grouping_edit.is_none());
        app.prepare_xml_grouping_edit("200", Some("old & crate"));
        assert!(
            app.xml_pending_grouping_edit.is_none(),
            "absent Grouping insertion is refused"
        );
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        let changed_selection = root.join("changed-selection.xml");
        app.xml_grouping_export_path = changed_selection.display().to_string();
        app.select_xml_track("200", "B");
        app.export_xml_grouping_edit();
        assert!(!changed_selection.exists());
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        let changed_source = root.join("different.xml");
        app.xml_document_source = Some(changed_source);
        let stale_source = root.join("stale-source.xml");
        app.xml_grouping_export_path = stale_source.display().to_string();
        app.export_xml_grouping_edit();
        assert!(!stale_source.exists());
        app.xml_document_source = Some(source.clone());
        app.select_xml_track("100", "A");
        app.xml_edit_grouping = "new < crate".into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        let output = root.join("edited.xml");
        app.xml_grouping_export_path = output.display().to_string();
        app.export_xml_grouping_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared);
        f02_track_grouping_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts UI-exported Grouping bytes");
        assert!(String::from_utf8_lossy(&bytes).contains("Grouping=\"new &lt; crate\""));
        let mutated = String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("Grouping=\"new &lt; crate\"", "Grouping=\"altered\"", 1);
        assert!(f02_track_grouping_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "100")
                .unwrap()
                .grouping
                .as_deref(),
            Some("new < crate")
        );
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|t| t.id == "200")
                .unwrap()
                .grouping,
            None,
            "absent Grouping remains absent on other tracks"
        );
        assert!(app.xml_pending_grouping_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn track_label_edit_ui_refuses_invalid_xml_unknown_and_stale_old_value() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("999", "Unknown");
        app.xml_edit_label = "Changed".into();
        app.prepare_xml_label_edit("999", Some("old & imprint"));
        assert!(app.xml_pending_label_edit.is_none());

        app.select_xml_track("100", "A");
        app.xml_edit_label = "bad\u{1}".into();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        assert!(app.xml_pending_label_edit.is_none());

        app.xml_edit_label = "Changed".into();
        app.prepare_xml_label_edit("100", Some("stale"));
        assert!(app.xml_pending_label_edit.is_none());

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("TrackID=\"200\"", "TrackID=\"100\""),
        )
        .unwrap();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_label = "Changed".into();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        assert!(app.xml_pending_label_edit.is_none(), "duplicate target must refuse");

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("Label=\"old &amp; imprint\"", "Label=\"\""),
        )
        .unwrap();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_label, "", "present-empty Label must stay editable");
        app.xml_edit_label = "Restored".into();
        app.prepare_xml_label_edit("100", Some(""));
        assert!(app.xml_pending_label_edit.is_some(), "present-empty Label is editable");
        app.cancel_xml_label_edit();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_genre_edit_ui_round_trips_entities_and_whitespace() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        let genre = "new < style & mix\tfirst\nsecond\r";
        app.xml_edit_genre = genre.into();
        app.prepare_xml_genre_edit("100", Some("old & style"));
        assert!(app.xml_pending_genre_edit.is_some());
        app.xml_genre_export_path = root.join("escaped.xml").display().to_string();
        app.export_xml_genre_edit();
        let output = PathBuf::from(&app.xml_genre_export_path);
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Genre=\"new &lt; style &amp; mix&#9;first&#10;second&#13;\""));
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .genre
                .as_deref(),
            Some(genre)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_label_edit_ui_round_trips_entities_and_whitespace() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-label-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        let label = "new < imprint & mix\tfirst\nsecond\r";
        app.xml_edit_label = label.into();
        app.prepare_xml_label_edit("100", Some("old & imprint"));
        assert!(app.xml_pending_label_edit.is_some());
        app.xml_label_export_path = root.join("escaped.xml").display().to_string();
        app.export_xml_label_edit();
        let output = PathBuf::from(&app.xml_label_export_path);
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Label=\"new &lt; imprint &amp; mix&#9;first&#10;second&#13;\""));
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .label
                .as_deref(),
            Some(label)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_album_edit_ui_exports_previewed_bytes_through_independent_oracle() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-ui-track-album-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
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
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_album, "old & record");
        app.xml_edit_album = "new < record".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        let prepared_bytes = app
            .xml_pending_album_edit
            .as_ref()
            .expect("prepared Album edit")
            .prepared
            .bytes()
            .to_vec();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);

        app.xml_album_export_path.clear();
        app.export_xml_album_edit();
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("choose a new XML destination")
        );
        assert!(app.xml_pending_album_edit.is_some());

        let cancelled = root.join("cancelled.xml");
        app.xml_album_export_path = cancelled.display().to_string();
        app.cancel_xml_album_edit();
        assert!(app.xml_pending_album_edit.is_none());
        assert!(!cancelled.exists(), "cancel must not write output");

        app.prepare_xml_album_edit("100", Some("old & record"));
        app.xml_album_export_path = source.display().to_string();
        app.export_xml_album_edit();
        assert!(app.xml_edit_error.is_some(), "source alias must be refused");
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        app.xml_album_export_path = existing.display().to_string();
        app.export_xml_album_edit();
        assert!(
            app.xml_edit_error.is_some(),
            "output refusal must be surfaced"
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        assert_eq!(std::fs::read(&source).unwrap(), original);

        app.xml_album_export_path = root.join("missing/output.xml").display().to_string();
        app.export_xml_album_edit();
        assert!(
            app.xml_edit_error.is_some(),
            "write failure must be surfaced"
        );
        assert!(app.notice.is_empty(), "failure must not claim success");

        std::fs::write(&source, b"<changed/>").unwrap();
        app.xml_album_export_path = root.join("stale-source.xml").display().to_string();
        app.export_xml_album_edit();
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("source changed")
        );
        assert!(!root.join("stale-source.xml").exists());
        std::fs::write(&source, &original).unwrap();
        app.prepare_xml_album_edit("100", Some("old & record"));

        let output = root.join("edited.xml");
        app.xml_album_export_path = output.display().to_string();
        app.export_xml_album_edit();
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared_bytes);
        f02_track_album_edit_oracle::check(&bytes)
            .expect("independent F-02 oracle accepts only selected Album edit");
        let mutated = String::from_utf8(bytes.clone()).unwrap().replacen(
            "Album=\"new &lt; record\"",
            "Album=\"altered\"",
            1,
        );
        assert_ne!(mutated.as_bytes(), bytes.as_slice());
        assert!(f02_track_album_edit_oracle::check(mutated.as_bytes()).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&catalog).unwrap(), catalog_before);
        let reopened = app.xml_preview.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .album
                .as_deref(),
            Some("new < record")
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "200")
                .unwrap()
                .album,
            None,
            "absent Album remains absent on other tracks"
        );
        assert!(app.xml_pending_album_edit.is_none());
        assert!(app.xml_edit_error.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_composer_edit_ui_refuses_changed_editable_path_without_output() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_composer = "new < composer".into();
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        assert!(app.xml_pending_composer_edit.is_some());
        let output = root.join("stale-path.xml");
        app.xml_composer_export_path = output.display().to_string();
        app.xml_path = root.join("different.xml").display().to_string();
        app.export_xml_composer_edit();
        assert!(app.xml_pending_composer_edit.is_none());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source"));
        assert!(!output.exists(), "changed editable XML path must refuse export");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_grouping_edit_ui_refuses_changed_editable_path_without_output() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_grouping = "new < crate".into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        assert!(app.xml_pending_grouping_edit.is_some());
        let output = root.join("stale-path.xml");
        app.xml_grouping_export_path = output.display().to_string();
        app.xml_path = root.join("different.xml").display().to_string();
        app.export_xml_grouping_edit();
        assert!(app.xml_pending_grouping_edit.is_none());
        assert!(app.xml_edit_error.as_deref().unwrap().contains("source"));
        assert!(!output.exists(), "changed editable XML path must refuse export");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_composer_edit_ui_allows_present_empty_but_not_absent_insertion() {
        let (root, source) = temporary_xml();
        let fixture = include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml");
        let xml = String::from_utf8(fixture.to_vec()).unwrap().replace("Composer=\"old &amp; composer\"", "Composer=\"\"");
        std::fs::write(&source, xml).unwrap();
        let mut app = app(); app.xml_path = source.display().to_string(); app.preview_xml(); app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_composer, ""); app.xml_edit_composer = "Composed".into();
        app.prepare_xml_composer_edit("100", Some("")); assert!(app.xml_pending_composer_edit.is_some());
        app.xml_composer_export_path = root.join("empty-edited.xml").display().to_string(); app.export_xml_composer_edit();
        assert_eq!(app.xml_preview.as_ref().unwrap().as_ref().unwrap().tracks.iter().find(|t| t.id == "100").unwrap().composer.as_deref(), Some("Composed"));
        app.prepare_xml_composer_edit("200", None); assert!(app.xml_pending_composer_edit.is_none(), "absent Composer insertion is refused");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_grouping_edit_ui_allows_present_empty_but_not_absent_insertion() {
        let (root, source) = temporary_xml();
        let fixture = include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml");
        let xml = String::from_utf8(fixture.to_vec()).unwrap().replace("Grouping=\"old &amp; crate\"", "Grouping=\"\"");
        std::fs::write(&source, xml).unwrap();
        let mut app = app(); app.xml_path = source.display().to_string(); app.preview_xml(); app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_grouping, ""); app.xml_edit_grouping = "Composed".into();
        app.prepare_xml_grouping_edit("100", Some("")); assert!(app.xml_pending_grouping_edit.is_some());
        app.xml_grouping_export_path = root.join("empty-edited.xml").display().to_string(); app.export_xml_grouping_edit();
        assert_eq!(app.xml_preview.as_ref().unwrap().as_ref().unwrap().tracks.iter().find(|t| t.id == "100").unwrap().grouping.as_deref(), Some("Composed"));
        app.prepare_xml_grouping_edit("200", None); assert!(app.xml_pending_grouping_edit.is_none(), "absent Grouping insertion is refused");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_composer_edit_ui_round_trips_entities_and_whitespace() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-composer-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        let composer = "new < composer & mix\tfirst\nsecond\r";
        app.xml_edit_composer = composer.into();
        app.prepare_xml_composer_edit("100", Some("old & composer"));
        assert!(app.xml_pending_composer_edit.is_some());
        app.xml_composer_export_path = root.join("escaped.xml").display().to_string();
        app.export_xml_composer_edit();
        let output = PathBuf::from(&app.xml_composer_export_path);
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Composer=\"new &lt; composer &amp; mix&#9;first&#10;second&#13;\""));
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .composer
                .as_deref(),
            Some(composer)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_grouping_edit_ui_round_trips_entities_and_whitespace() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-grouping-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        let grouping = "new < crate & mix\tfirst\nsecond\r";
        app.xml_edit_grouping = grouping.into();
        app.prepare_xml_grouping_edit("100", Some("old & crate"));
        assert!(app.xml_pending_grouping_edit.is_some());
        app.xml_grouping_export_path = root.join("escaped.xml").display().to_string();
        app.export_xml_grouping_edit();
        let output = PathBuf::from(&app.xml_grouping_export_path);
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Grouping=\"new &lt; crate &amp; mix&#9;first&#10;second&#13;\""));
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .grouping
                .as_deref(),
            Some(grouping)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_album_edit_ui_guards_absent_stale_proposal_and_invalid_xml() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();

        app.select_xml_track("200", "B");
        app.prepare_xml_album_edit("200", None);
        assert!(app.xml_pending_album_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("no Album attribute")
        );

        app.select_xml_track("999", "Unknown");
        app.prepare_xml_album_edit("999", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_none());
        app.select_xml_track("100", "A");
        app.prepare_xml_album_edit("100", Some("stale"));
        assert!(app.xml_pending_album_edit.is_none());
        app.xml_edit_album = "bad\u{1}".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_none());

        app.xml_edit_album = "Changed".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_some());
        app.xml_edit_album.push('!');
        app.xml_album_export_path = root.join("changed-proposal.xml").display().to_string();
        app.export_xml_album_edit();
        assert!(app.xml_pending_album_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("proposed value changed")
        );
        assert!(!root.join("changed-proposal.xml").exists());

        app.xml_edit_album = "Changed".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        let preview = app.xml_preview.as_mut().unwrap().as_mut().unwrap();
        preview
            .tracks
            .iter_mut()
            .find(|track| track.id == "100")
            .unwrap()
            .album = Some("newer current value".into());
        app.xml_album_export_path = root.join("changed-old-value.xml").display().to_string();
        app.export_xml_album_edit();
        assert!(app.xml_pending_album_edit.is_none());
        assert!(
            app.xml_edit_error
                .as_deref()
                .unwrap()
                .contains("proposed value changed")
        );
        assert!(!root.join("changed-old-value.xml").exists());

        app.xml_edit_album = "Changed".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_some());
        app.select_xml_track("200", "B");
        assert!(
            app.xml_pending_album_edit.is_none(),
            "track change invalidates pending edit"
        );

        app.select_xml_track("100", "A");
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_some());
        app.xml_path = root.join("other.xml").display().to_string();
        app.on_xml_source_changed();
        assert!(
            app.xml_pending_album_edit.is_none(),
            "source path change invalidates pending edit"
        );

        std::fs::write(
            &source,
            String::from_utf8(
                include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml")
                    .to_vec(),
            )
            .unwrap()
            .replace("TrackID=\"200\"", "TrackID=\"100\""),
        )
        .unwrap();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        app.xml_edit_album = "Changed".into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(
            app.xml_pending_album_edit.is_none(),
            "duplicate target ID must refuse preparation"
        );

        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
        )
        .unwrap();
        let empty_fixture = std::fs::read(&source)
            .unwrap()
            .windows(b"Album=\"old &amp; record\"".len())
            .position(|window| window == b"Album=\"old &amp; record\"")
            .expect("fixture Album attribute");
        let mut bytes = std::fs::read(&source).unwrap();
        bytes.splice(
            empty_fixture..empty_fixture + b"Album=\"old &amp; record\"".len(),
            b"Album=\"\"".iter().copied(),
        );
        std::fs::write(&source, bytes).unwrap();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        assert_eq!(app.xml_edit_album, "");
        app.xml_edit_album = "restored".into();
        app.prepare_xml_album_edit("100", Some(""));
        assert!(
            app.xml_pending_album_edit.is_some(),
            "present-empty Album is editable"
        );
        app.cancel_xml_album_edit();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn track_album_edit_ui_round_trips_xml_entities_and_whitespace() {
        let (root, source) = temporary_xml();
        std::fs::write(
            &source,
            include_bytes!("../../../xtask/tests/fixtures/f02-track-album-edit/source.xml"),
        )
        .unwrap();
        let mut app = app();
        app.xml_path = source.display().to_string();
        app.preview_xml();
        app.select_xml_track("100", "A");
        let album = "new < record & mix\tfirst\nsecond\r";
        app.xml_edit_album = album.into();
        app.prepare_xml_album_edit("100", Some("old & record"));
        assert!(app.xml_pending_album_edit.is_some());
        app.xml_album_export_path = root.join("escaped.xml").display().to_string();
        app.export_xml_album_edit();
        let output = PathBuf::from(&app.xml_album_export_path);
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("Album=\"new &lt; record &amp; mix&#9;first&#10;second&#13;\""));
        f02_track_album_edit_oracle::check_with_album(&bytes, album)
            .expect("independent F-02 oracle preserves escaped Album text");
        assert_eq!(
            app.xml_preview
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .album
                .as_deref(),
            Some(album)
        );
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
