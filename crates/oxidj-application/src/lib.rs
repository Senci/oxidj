//! Shared application handlers for CLI and future UI callers.

use std::path::Path;

pub use oxidj_rekordbox::{
    Error as RekordboxError, ExportDecision, ExportPreflight, PlaylistRenameChange,
    PlaylistRenameRequest, PositionMarkEditChange, PositionMarkEditRequest,
    PositionMarkEndEditChange, PositionMarkEndEditRequest, PositionMarkNameEditChange,
    PositionMarkNameEditRequest, PreparedPlaylistRename, PreparedPositionMarkEdit,
    PreparedPositionMarkEndEdit, PreparedPositionMarkNameEdit, PreparedTempoEdit,
    PreparedTempoStartEdit, Preview, TempoEditChange, TempoEditRequest, TempoStartEditChange,
    TempoStartEditRequest,
};

/// Read-only preview handler; parsing and vendor-specific semantics stay in the adapter.
pub fn preview_rekordbox_xml(path: impl AsRef<Path>) -> Result<Preview, RekordboxError> {
    oxidj_rekordbox::parse_file(path)
}

/// Read-only eligibility plan for the bounded no-edit Rekordbox XML exporter.
pub fn preflight_rekordbox_xml(path: impl AsRef<Path>) -> Result<ExportPreflight, RekordboxError> {
    let bytes = std::fs::read(path).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::preflight_export(&bytes)
}

/// Export one guarded playlist rename to a new, non-overwriting file.
pub fn export_rekordbox_playlist_rename(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistRenameRequest,
) -> Result<PlaylistRenameChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_rename(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Prepare a validated tempo edit without writing any file.
pub fn prepare_rekordbox_tempo_edit(
    input: impl AsRef<Path>,
    request: &TempoEditRequest,
) -> Result<PreparedTempoEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_tempo_edit(&bytes, request)
}

/// Prepare one guarded position mark End edit without writing any file.
pub fn prepare_rekordbox_position_mark_end_edit(
    input: impl AsRef<Path>,
    request: &PositionMarkEndEditRequest,
) -> Result<PreparedPositionMarkEndEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_position_mark_end_edit(&bytes, request)
}

/// Prepare one validated position mark Start edit without writing any file.
pub fn prepare_rekordbox_position_mark_edit(
    input: impl AsRef<Path>,
    request: &PositionMarkEditRequest,
) -> Result<PreparedPositionMarkEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_position_mark_edit(&bytes, request)
}

/// Export one guarded position mark End edit to a new file.
pub fn export_rekordbox_position_mark_end_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PositionMarkEndEditRequest,
) -> Result<PositionMarkEndEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_position_mark_end_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Safely export the bounded no-edit XML subset without replacing any file.
pub fn export_rekordbox_xml(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (source_bytes, output) = read_new_export_source(input, output)?;
    let bytes = oxidj_rekordbox::export_xml(&source_bytes)?;
    publish_new_file(&output, &bytes)
}

/// Prepare again against current source bytes, then publish the edit as a new file.
pub fn export_rekordbox_tempo_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TempoEditRequest,
) -> Result<TempoEditChange, RekordboxError> {
    let (source_bytes, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_tempo_edit(&source_bytes, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Prepare a validated tempo-point Inizio edit without writing any file.
pub fn prepare_rekordbox_tempo_start_edit(
    input: impl AsRef<Path>,
    request: &TempoStartEditRequest,
) -> Result<PreparedTempoStartEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_tempo_start_edit(&bytes, request)
}

/// Export one guarded tempo-point Inizio edit to a new file.
pub fn export_rekordbox_tempo_start_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TempoStartEditRequest,
) -> Result<TempoStartEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_tempo_start_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Prepare a guarded position-mark Name edit without writing any file.
pub fn prepare_rekordbox_position_mark_name_edit(
    input: impl AsRef<Path>,
    request: &PositionMarkNameEditRequest,
) -> Result<PreparedPositionMarkNameEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|e| RekordboxError(e.to_string()))?;
    oxidj_rekordbox::prepare_position_mark_name_edit(&bytes, request)
}

/// Export one guarded position-mark Name edit to a new file.
pub fn export_rekordbox_position_mark_name_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PositionMarkNameEditRequest,
) -> Result<PositionMarkNameEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_position_mark_name_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Re-validate one position mark Start edit, then publish it as a new file.
pub fn export_rekordbox_position_mark_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PositionMarkEditRequest,
) -> Result<PositionMarkEditChange, RekordboxError> {
    let (source_bytes, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_position_mark_edit(&source_bytes, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

fn read_new_export_source(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<(Vec<u8>, std::path::PathBuf), RekordboxError> {
    use std::fs;
    let input = input.as_ref();
    let output = output.as_ref();
    let source = fs::canonicalize(input).map_err(|e| RekordboxError(e.to_string()))?;
    if output.exists() {
        let dest = fs::canonicalize(output).map_err(|e| RekordboxError(e.to_string()))?;
        if source == dest {
            return Err(RekordboxError(
                "source and destination are the same file".into(),
            ));
        }
        return Err(RekordboxError(
            "refusing to overwrite existing output".into(),
        ));
    }
    let bytes = fs::read(source).map_err(|e| RekordboxError(e.to_string()))?;
    Ok((bytes, output.to_path_buf()))
}

fn publish_new_file(output: &Path, bytes: &[u8]) -> Result<(), RekordboxError> {
    use std::{fs, io::Write};
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .ok_or_else(|| RekordboxError("output must name a file".into()))?
        .to_string_lossy();
    let temp = parent.join(format!(".{name}.oxidj-tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| RekordboxError(e.to_string()))?;
        file.write_all(bytes)
            .map_err(|e| RekordboxError(e.to_string()))?;
        file.sync_all().map_err(|e| RekordboxError(e.to_string()))?;
        fs::hard_link(&temp, output).map_err(|e| RekordboxError(e.to_string()))?;
        fs::remove_file(&temp).map_err(|e| RekordboxError(e.to_string()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
