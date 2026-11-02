//! Guarded XML playlist workflows: sibling reorder, move, member reorder/append/
//! insert/delete, clear, delete, create and rename.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistMemberReorder {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) prepared: oxidj_application::PreparedPlaylistMemberReorderExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingSiblingPlaylistReorder {
    pub(crate) source: PathBuf,
    pub(crate) parent_index: usize,
    pub(crate) parent_name: String,
    pub(crate) expected_order: Vec<String>,
    pub(crate) new_order: Vec<String>,
    pub(crate) prepared: oxidj_application::PreparedSiblingPlaylistReorderExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistMove {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) source_parent_index: usize,
    pub(crate) source_parent_name: String,
    pub(crate) source_parent_path: Vec<String>,
    pub(crate) destination_parent_index: usize,
    pub(crate) destination_parent_name: String,
    pub(crate) destination_parent_path: Vec<String>,
    pub(crate) expected_members: Vec<String>,
    pub(crate) expected_source_order: Vec<String>,
    pub(crate) expected_destination_order: Vec<String>,
    pub(crate) old_path: Vec<String>,
    pub(crate) new_path: Vec<String>,
    pub(crate) source_count: Option<String>,
    pub(crate) destination_count: Option<String>,
    pub(crate) prepared: oxidj_application::PreparedPlaylistMoveExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistMemberAppend {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) track_id: String,
    pub(crate) prepared: oxidj_application::PreparedPlaylistMemberAppendExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistMemberInsert {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) track_id: String,
    pub(crate) position: usize,
    pub(crate) expected_order: Vec<String>,
    pub(crate) prepared: oxidj_application::PreparedPlaylistMemberInsertExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistClear {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) expected_order: Vec<String>,
    pub(crate) prepared: oxidj_application::PreparedPlaylistClearExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistDelete {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) name: String,
    pub(crate) expected_order: Vec<String>,
    pub(crate) prepared: oxidj_application::PreparedPlaylistDeleteExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistMemberDelete {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) member_index: usize,
    pub(crate) track_id: String,
    pub(crate) prepared: oxidj_application::PreparedPlaylistMemberDeleteExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistRename {
    pub(crate) source: PathBuf,
    pub(crate) playlist_index: usize,
    pub(crate) current_name: String,
    pub(crate) name: String,
    pub(crate) prepared: oxidj_application::PreparedPlaylistRenameExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPlaylistCreate {
    pub(crate) source: PathBuf,
    pub(crate) parent_index: usize,
    pub(crate) parent_name: String,
    pub(crate) name: String,
    pub(crate) prepared: oxidj_application::PreparedPlaylistCreateExport,
}

pub(crate) fn immediate_parent_folder<'a>(
    preview: &'a oxidj_application::Preview,
    playlist: &oxidj_application::Playlist,
) -> Option<&'a oxidj_application::Folder> {
    let parent_name = playlist.path.get(playlist.path.len().checked_sub(2)?)?;
    let ancestors = playlist.path.get(..playlist.path.len().checked_sub(1)?)?;
    preview.folders.iter().find(|folder| {
        folder.name == *parent_name
            && folder.path.len() == ancestors.len() + 1
            && folder.path.get(1..) == Some(ancestors)
    })
}

impl LibraryApp {
    pub(crate) fn prepare_xml_sibling_playlist_reorder(&mut self) {
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

    pub(crate) fn cancel_xml_sibling_playlist_reorder(&mut self) {
        self.xml_sibling_pending = None;
    }

    pub(crate) fn export_xml_sibling_playlist_reorder(&mut self) {
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

    pub(crate) fn prepare_xml_playlist_move(&mut self, playlist_index: usize) {
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
        let order_under = |parent: &oxidj_application::Folder| -> Vec<String> {
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

    pub(crate) fn cancel_xml_playlist_move(&mut self) {
        self.xml_pending_move = None;
    }

    pub(crate) fn export_xml_playlist_move(&mut self) {
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
        let order_under = |parent: Option<&oxidj_application::Folder>| -> Vec<String> {
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

    pub(crate) fn prepare_xml_member_reorder(
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

    pub(crate) fn export_xml_member_reorder(&mut self) {
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

    pub(crate) fn prepare_xml_member_append(&mut self, playlist_index: usize, track_id: &str) {
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

    pub(crate) fn cancel_xml_member_append(&mut self) {
        self.xml_pending_append = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_member_append(&mut self) {
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

    pub(crate) fn prepare_xml_member_insert(
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

    pub(crate) fn cancel_xml_member_insert(&mut self) {
        self.xml_pending_insert = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_member_insert(&mut self) {
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

    pub(crate) fn prepare_xml_playlist_delete(&mut self, playlist_index: usize) {
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

    pub(crate) fn cancel_xml_playlist_delete(&mut self) {
        self.xml_pending_playlist_delete = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_playlist_delete(&mut self) {
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

    pub(crate) fn prepare_xml_playlist_clear(&mut self, playlist_index: usize) {
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

    pub(crate) fn cancel_xml_playlist_clear(&mut self) {
        self.xml_pending_clear = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_playlist_clear(&mut self) {
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

    pub(crate) fn prepare_xml_playlist_create(&mut self) {
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

    // Test-only: the provisional playlist-create panel has no Cancel control yet.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn cancel_xml_playlist_create(&mut self) {
        self.xml_pending_create = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_playlist_create(&mut self) {
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

    pub(crate) fn prepare_xml_member_delete(&mut self, playlist_index: usize, member_index: usize) {
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

    pub(crate) fn invalidate_xml_member_delete(&mut self) {
        self.xml_pending_delete = None;
    }

    pub(crate) fn prepare_xml_playlist_rename(&mut self, index: usize, current_name: &str) {
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

    pub(crate) fn cancel_xml_playlist_rename(&mut self) {
        self.xml_pending_rename = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_playlist_rename(&mut self) {
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

    pub(crate) fn cancel_xml_member_delete(&mut self) {
        self.invalidate_xml_member_delete();
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_member_delete(&mut self) {
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
}
