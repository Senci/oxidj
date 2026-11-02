//! Guarded XML folder workflows: sibling-folder reorder, subtree move, delete,
//! rename and create (prepare → preview → cancel / new-file export).

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct PendingSiblingFolderReorder {
    pub(crate) source: PathBuf,
    pub(crate) parent_index: usize,
    pub(crate) parent_name: String,
    pub(crate) parent_path: Vec<String>,
    pub(crate) expected_order: Vec<String>,
    pub(crate) new_order: Vec<String>,
    pub(crate) prepared: oxidj_application::PreparedSiblingFolderReorderExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingFolderMove {
    pub(crate) source: PathBuf,
    pub(crate) folder_index: usize,
    pub(crate) folder_name: String,
    pub(crate) old_path: Vec<String>,
    pub(crate) new_path: Vec<String>,
    pub(crate) source_parent_index: usize,
    pub(crate) source_parent_name: String,
    pub(crate) source_parent_path: Vec<String>,
    pub(crate) target_parent_index: usize,
    pub(crate) target_parent_name: String,
    pub(crate) target_parent_path: Vec<String>,
    pub(crate) expected_source_order: Vec<String>,
    pub(crate) expected_target_order: Vec<String>,
    pub(crate) source_count: Option<String>,
    pub(crate) target_count: Option<String>,
    pub(crate) prepared: oxidj_application::PreparedFolderMoveExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingFolderDelete {
    pub(crate) source: PathBuf,
    pub(crate) folder_index: usize,
    pub(crate) ordinal: usize,
    pub(crate) name: String,
    pub(crate) path: Vec<String>,
    pub(crate) count: Option<String>,
    pub(crate) parent_path: Vec<String>,
    pub(crate) parent_count: Option<String>,
    pub(crate) prepared: oxidj_application::PreparedFolderDeleteExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingFolderRename {
    pub(crate) source: PathBuf,
    pub(crate) folder_index: usize,
    pub(crate) ordinal: usize,
    pub(crate) old_name: String,
    pub(crate) path: Vec<String>,
    pub(crate) new_name: String,
    pub(crate) prepared: oxidj_application::PreparedFolderRenameExport,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingFolderCreate {
    pub(crate) source: PathBuf,
    pub(crate) parent_index: usize,
    pub(crate) parent_ordinal: usize,
    pub(crate) parent_name: String,
    pub(crate) name: String,
    pub(crate) parent_count: Option<String>,
    pub(crate) prepared: oxidj_application::PreparedFolderCreateExport,
}

impl LibraryApp {
    pub(crate) fn prepare_xml_sibling_folder_reorder(&mut self) {
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

    pub(crate) fn cancel_xml_sibling_folder_reorder(&mut self) {
        self.xml_folder_sibling_pending = None;
    }

    pub(crate) fn export_xml_sibling_folder_reorder(&mut self) {
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

    pub(crate) fn prepare_xml_folder_move(&mut self, folder_index: usize) {
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
        let order = |parent: &oxidj_application::Folder| {
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

    pub(crate) fn cancel_xml_folder_move(&mut self) {
        self.xml_pending_folder_move = None;
    }

    pub(crate) fn export_xml_folder_move(&mut self) {
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

    pub(crate) fn prepare_xml_folder_delete(&mut self) {
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

    pub(crate) fn cancel_xml_folder_delete(&mut self) {
        self.xml_pending_folder_delete = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_folder_delete(&mut self) {
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

    pub(crate) fn prepare_xml_folder_rename(&mut self) {
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

    pub(crate) fn export_xml_folder_rename(&mut self) {
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

    pub(crate) fn prepare_xml_folder_create(&mut self) {
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

    pub(crate) fn cancel_xml_folder_create(&mut self) {
        self.xml_pending_folder_create = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_folder_create(&mut self) {
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
}
