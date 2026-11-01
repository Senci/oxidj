//! Shared application handlers for CLI and future UI callers.

use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
    process::{Command, Stdio},
};
use zeroize::{Zeroize, Zeroizing};

pub use oxidj_rekordbox::{
    Error as RekordboxError, ExportDecision, ExportPreflight, FolderCreateRequest,
    FolderDeleteRequest, FolderMoveRequest, FolderRenameChange, FolderRenameRequest, PdbPreview,
    PdbPreviewError, PlaylistClearRequest, PlaylistCreateRequest, PlaylistDeleteRequest,
    PlaylistMemberAppendRequest, PlaylistMemberDeleteRequest, PlaylistMemberInsertRequest,
    PlaylistMoveRequest, PlaylistRenameChange, PlaylistRenameRequest, PlaylistReorderRequest,
    PositionMarkDeleteChange, PositionMarkDeleteRequest, PositionMarkEditChange,
    PositionMarkEditRequest, PositionMarkEndEditChange, PositionMarkEndEditRequest,
    PositionMarkNameEditChange, PositionMarkNameEditRequest, PreparedFolderDelete,
    PreparedFolderMove, PreparedFolderRename, PreparedPlaylistDelete, PreparedPlaylistMove,
    PreparedPlaylistRename, PreparedPlaylistReorder, PreparedPositionMarkDelete,
    PreparedPositionMarkEdit, PreparedPositionMarkEndEdit, PreparedPositionMarkNameEdit,
    PreparedSiblingFolderReorder, PreparedSiblingPlaylistReorder, PreparedTempoBeatEdit,
    PreparedTempoEdit, PreparedTempoMeterEdit, PreparedTempoStartEdit, PreparedTrackAlbumEdit,
    PreparedTrackArtistEdit, PreparedTrackAverageBpmEdit, PreparedTrackBitRateEdit,
    PreparedTrackColourEdit, PreparedTrackCommentsEdit, PreparedTrackComposerEdit,
    PreparedTrackDateAddedEdit, PreparedTrackDateModifiedEdit, PreparedTrackDelete,
    PreparedTrackDiscNumberEdit, PreparedTrackGenreEdit, PreparedTrackGroupingEdit,
    PreparedTrackKindEdit, PreparedTrackLabelEdit, PreparedTrackLastPlayedEdit,
    PreparedTrackMixEdit, PreparedTrackNameEdit, PreparedTrackNumberEdit,
    PreparedTrackPlayCountEdit, PreparedTrackRatingEdit, PreparedTrackRemixerEdit,
    PreparedTrackSampleRateEdit, PreparedTrackSizeEdit, PreparedTrackTotalTimeEdit,
    PreparedTrackYearEdit, Preview, SiblingFolderReorderRequest, SiblingPlaylistReorderRequest,
    TempoBeatEditChange, TempoBeatEditRequest, TempoEditChange, TempoEditRequest,
    TempoMeterEditChange, TempoMeterEditRequest, TempoStartEditChange, TempoStartEditRequest,
    TrackAlbumEditChange, TrackAlbumEditRequest, TrackArtistEditChange, TrackArtistEditRequest,
    TrackAverageBpmEditChange, TrackAverageBpmEditRequest, TrackBitRateEditChange,
    TrackBitRateEditRequest, TrackColourEditChange, TrackColourEditRequest,
    TrackCommentsEditChange, TrackCommentsEditRequest, TrackComposerEditChange,
    TrackComposerEditRequest, TrackDateAddedEditChange, TrackDateAddedEditRequest,
    TrackDateModifiedEditChange, TrackDateModifiedEditRequest, TrackDeleteRequest,
    TrackDiscNumberEditChange, TrackDiscNumberEditRequest, TrackGenreEditChange,
    TrackGenreEditRequest, TrackGroupingEditChange, TrackGroupingEditRequest, TrackKindEditChange,
    TrackKindEditRequest, TrackLabelEditChange, TrackLabelEditRequest, TrackLastPlayedEditChange,
    TrackLastPlayedEditRequest, TrackMixEditChange, TrackMixEditRequest, TrackNameEditChange,
    TrackNameEditRequest, TrackNumberEditChange, TrackNumberEditRequest, TrackPlayCountEditChange,
    TrackPlayCountEditRequest, TrackRatingEditChange, TrackRatingEditRequest,
    TrackRemixerEditChange, TrackRemixerEditRequest, TrackSampleRateEditChange,
    TrackSampleRateEditRequest, TrackSizeEditChange, TrackSizeEditRequest, TrackTonalityEditChange,
    TrackTonalityEditRequest, TrackTotalTimeEditChange, TrackTotalTimeEditRequest,
    TrackYearEditChange, TrackYearEditRequest,
};

pub use oxidj_onelibrary::{
    Error as OneLibraryError, Playlist as OneLibraryPlaylist, Preview as OneLibraryPreview,
    SCHEMA_PROFILE_NATIVE as ONELIBRARY_SCHEMA_PROFILE_NATIVE, SourceId as OneLibrarySourceId,
    Track as OneLibraryTrack,
};

/// Validate a complete OneLibrary reader response before exposing it to callers.
pub fn validate_onelibrary_preview(
    preview: OneLibraryPreview,
) -> Result<OneLibraryPreview, OneLibraryError> {
    oxidj_onelibrary::validate(preview)
}

/// Read a one-shot key from an explicit non-terminal stdin pipe, then invoke the sibling reader.
pub fn preview_onelibrary_stdin(
    path: impl AsRef<Path>,
) -> Result<OneLibraryPreview, OneLibraryError> {
    if io::stdin().is_terminal() {
        return Err(OneLibraryError::ReaderFailed);
    }
    let mut secret = Zeroizing::new(Vec::with_capacity(4097));
    io::stdin()
        .lock()
        .take(4097)
        .read_to_end(&mut secret)
        .map_err(|_| OneLibraryError::ReaderFailed)?;
    if secret.is_empty() || secret.len() > 4096 {
        secret.zeroize();
        return Err(OneLibraryError::ReaderFailed);
    }
    preview_onelibrary(path, secret.as_slice())
}

/// Run the installed sibling reader once and expose only a fully decoded response.
/// The key is piped directly to the reader's stdin and never placed in argv/env.
pub fn preview_onelibrary(
    path: impl AsRef<Path>,
    key: &[u8],
) -> Result<OneLibraryPreview, OneLibraryError> {
    if key.is_empty() || key.len() > 4096 {
        return Err(OneLibraryError::ReaderFailed);
    }
    let executable = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .map(|parent| parent.join("oxidj-onelibrary-reader"))
        })
        .ok_or(OneLibraryError::ReaderFailed)?;
    let mut child = Command::new(executable)
        .arg("--key-stdin")
        .arg(path.as_ref())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| OneLibraryError::ReaderFailed)?;
    let mut secret = Zeroizing::new(key.to_vec());
    let write_result = child
        .stdin
        .take()
        .ok_or(OneLibraryError::ReaderFailed)?
        .write_all(secret.as_slice());
    secret.zeroize();
    if write_result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(OneLibraryError::ReaderFailed);
    }
    let output = child
        .wait_with_output()
        .map_err(|_| OneLibraryError::ReaderFailed)?;
    if !output.status.success() {
        return Err(OneLibraryError::ReaderFailed);
    }
    oxidj_onelibrary::decode_preview(&output.stdout)
}

/// Read-only preview handler; parsing and vendor-specific semantics stay in the adapter.
pub fn preview_rekordbox_xml(path: impl AsRef<Path>) -> Result<Preview, RekordboxError> {
    oxidj_rekordbox::parse_file(path)
}

/// Read-only preview of a DeviceSQL export.pdb source.
pub fn preview_rekordbox_pdb(path: impl AsRef<Path>) -> Result<PdbPreview, PdbPreviewError> {
    oxidj_rekordbox::parse_pdb_file(path)
}

/// A source-bound prepared playlist-member reorder retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberReorderExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    prepared: PreparedPlaylistReorder,
}

impl PreparedPlaylistMemberReorderExport {
    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn bytes(&self) -> &[u8] {
        &self.prepared.bytes
    }
}

/// Prepare a guarded playlist-member reorder for preview and source-bound export.
pub fn prepare_rekordbox_playlist_member_reorder_export(
    source: impl AsRef<Path>,
    request: &PlaylistReorderRequest,
) -> Result<PreparedPlaylistMemberReorderExport, RekordboxError> {
    let source_path = source.as_ref().to_path_buf();
    let source_bytes =
        std::fs::read(&source_path).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_reorder(&source_bytes, request)?;
    Ok(PreparedPlaylistMemberReorderExport {
        source: source_path,
        source_bytes,
        prepared,
    })
}

/// Revalidate source bytes and publish prepared reorder bytes to a new file.
pub fn export_prepared_rekordbox_playlist_member_reorder(
    prepared: &PreparedPlaylistMemberReorderExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist-member reorder preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.prepared.bytes)
}

/// A source-bound prepared playlist-member append retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberAppendExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedPlaylistMemberAppendExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare a guarded playlist-member append for preview and source-bound export.
pub fn prepare_rekordbox_playlist_member_append_export(
    source: impl AsRef<Path>,
    request: &PlaylistMemberAppendRequest,
) -> Result<PreparedPlaylistMemberAppendExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_member_append(&source_bytes, request)?;
    Ok(PreparedPlaylistMemberAppendExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared append bytes to a new file.
pub fn export_prepared_rekordbox_playlist_member_append(
    prepared: &PreparedPlaylistMemberAppendExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist-member append preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared empty-playlist creation retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistCreateExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedPlaylistCreateExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare a guarded empty playlist creation for preview and source-bound export.
pub fn prepare_rekordbox_playlist_create_export(
    source: impl AsRef<Path>,
    request: &PlaylistCreateRequest,
) -> Result<PreparedPlaylistCreateExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_create(&source_bytes, request)?;
    Ok(PreparedPlaylistCreateExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared playlist-creation bytes to a new file.
pub fn export_prepared_rekordbox_playlist_create(
    prepared: &PreparedPlaylistCreateExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist creation preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared playlist-member insertion retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberInsertExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedPlaylistMemberInsertExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare a guarded playlist-member insertion for preview and source-bound export.
pub fn prepare_rekordbox_playlist_member_insert_export(
    source: impl AsRef<Path>,
    request: &PlaylistMemberInsertRequest,
) -> Result<PreparedPlaylistMemberInsertExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_member_insert(&source_bytes, request)?;
    Ok(PreparedPlaylistMemberInsertExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared insertion bytes to a new file.
pub fn export_prepared_rekordbox_playlist_member_insert(
    prepared: &PreparedPlaylistMemberInsertExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist-member insertion preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared Track Name export retains the exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackNameExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
    change: TrackNameEditChange,
}

impl PreparedTrackNameExport {
    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn change(&self) -> &TrackNameEditChange {
        &self.change
    }
}

/// Source and exact-old guard for a previewable Track Name edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackNameEditPreparationRequest {
    pub source: std::path::PathBuf,
    pub track_id: String,
    pub expected_old_name: String,
    pub new_name: String,
}

/// Read-only application handler: prepare one guarded Track Name edit for preview.
pub fn prepare_rekordbox_track_name_edit(
    request: &TrackNameEditPreparationRequest,
) -> Result<PreparedTrackNameEdit, RekordboxError> {
    let source =
        std::fs::read(&request.source).map_err(|error| RekordboxError(error.to_string()))?;
    let adapter_request = TrackNameEditRequest {
        track_id: request.track_id.clone(),
        expected_old_name: request.expected_old_name.clone(),
        new_name: request.new_name.clone(),
    };
    prepare_rekordbox_track_name_edit_bytes(&source, &adapter_request)
}

/// Prepare a guarded Track Name edit for a source-bound, revalidated new-file export.
pub fn prepare_rekordbox_track_name_export(
    request: &TrackNameEditPreparationRequest,
) -> Result<PreparedTrackNameExport, RekordboxError> {
    let source =
        std::fs::read(&request.source).map_err(|error| RekordboxError(error.to_string()))?;
    let adapter_request = TrackNameEditRequest {
        track_id: request.track_id.clone(),
        expected_old_name: request.expected_old_name.clone(),
        new_name: request.new_name.clone(),
    };
    let prepared = prepare_rekordbox_track_name_edit_bytes(&source, &adapter_request)?;
    Ok(PreparedTrackNameExport {
        source: request.source.clone(),
        source_bytes: source,
        bytes: prepared.bytes,
        change: prepared.change,
    })
}

/// Export only a source-bound prepared Track Name edit after confirming its source is unchanged.
pub fn export_prepared_rekordbox_track_name_edit(
    prepared: &PreparedTrackNameExport,
    output: impl AsRef<Path>,
) -> Result<TrackNameEditChange, RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after Track Name edit preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change.clone())
}

fn prepare_rekordbox_track_name_edit_bytes(
    source: &[u8],
    request: &TrackNameEditRequest,
) -> Result<oxidj_rekordbox::PreparedTrackNameEdit, RekordboxError> {
    oxidj_rekordbox::prepare_track_name_edit(source, request)
}

/// Export one guarded track Name edit to a new, non-overwriting file.
pub fn export_rekordbox_track_name_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackNameEditRequest,
) -> Result<TrackNameEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = prepare_rekordbox_track_name_edit_bytes(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Comments edit to a new, non-overwriting file.
pub fn export_rekordbox_track_comments_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackCommentsEditRequest,
) -> Result<TrackCommentsEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_comments_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Album edit to a new, non-overwriting file.
pub fn export_rekordbox_track_album_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackAlbumEditRequest,
) -> Result<TrackAlbumEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_album_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Genre edit to a new, non-overwriting file.
pub fn export_rekordbox_track_genre_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackGenreEditRequest,
) -> Result<TrackGenreEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_genre_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Label edit to a new, non-overwriting file.
pub fn export_rekordbox_track_label_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackLabelEditRequest,
) -> Result<TrackLabelEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_label_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Rating edit to a new, non-overwriting file.
pub fn export_rekordbox_track_rating_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackRatingEditRequest,
) -> Result<TrackRatingEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_rating_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded raw Track Colour edit to a new, non-overwriting file.
pub fn export_rekordbox_track_colour_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackColourEditRequest,
) -> Result<TrackColourEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_colour_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Composer edit to a new, non-overwriting file.
pub fn export_rekordbox_track_composer_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackComposerEditRequest,
) -> Result<TrackComposerEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_composer_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Grouping edit to a new, non-overwriting file.
pub fn export_rekordbox_track_grouping_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackGroupingEditRequest,
) -> Result<TrackGroupingEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_grouping_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Remixer edit to a new, non-overwriting file.
pub fn export_rekordbox_track_remixer_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackRemixerEditRequest,
) -> Result<TrackRemixerEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_remixer_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Mix edit to a new, non-overwriting file.
pub fn export_rekordbox_track_mix_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackMixEditRequest,
) -> Result<TrackMixEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_mix_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Tonality edit to a new, non-overwriting file.
pub fn export_rekordbox_track_tonality_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackTonalityEditRequest,
) -> Result<TrackTonalityEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_tonality_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded raw Track DateAdded edit to a new, non-overwriting file.
pub fn export_rekordbox_track_date_added_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackDateAddedEditRequest,
) -> Result<TrackDateAddedEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_date_added_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded raw Track DateModified edit to a new, non-overwriting file.
pub fn export_rekordbox_track_date_modified_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackDateModifiedEditRequest,
) -> Result<TrackDateModifiedEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_date_modified_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_play_count_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackPlayCountEditRequest,
) -> Result<TrackPlayCountEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_play_count_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_average_bpm_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackAverageBpmEditRequest,
) -> Result<TrackAverageBpmEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_average_bpm_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_disc_number_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackDiscNumberEditRequest,
) -> Result<TrackDiscNumberEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_disc_number_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_number_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackNumberEditRequest,
) -> Result<TrackNumberEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_number_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_size_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackSizeEditRequest,
) -> Result<TrackSizeEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_size_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_total_time_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackTotalTimeEditRequest,
) -> Result<TrackTotalTimeEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_total_time_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_bit_rate_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackBitRateEditRequest,
) -> Result<TrackBitRateEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_bit_rate_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_sample_rate_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackSampleRateEditRequest,
) -> Result<TrackSampleRateEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_sample_rate_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

pub fn export_rekordbox_track_last_played_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackLastPlayedEditRequest,
) -> Result<TrackLastPlayedEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_last_played_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded raw track Year edit to a new, non-overwriting file.
pub fn export_rekordbox_track_year_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackYearEditRequest,
) -> Result<TrackYearEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_year_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded raw track Kind edit to a new, non-overwriting file.
pub fn export_rekordbox_track_kind_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackKindEditRequest,
) -> Result<TrackKindEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_kind_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export one guarded track Artist edit to a new, non-overwriting file.
pub fn export_rekordbox_track_artist_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackArtistEditRequest,
) -> Result<TrackArtistEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_artist_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export deletion of one guarded playlist node to a new, non-overwriting file.
pub fn export_rekordbox_playlist_delete(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistDeleteRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_delete(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared playlist-member removal retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMemberDeleteExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedPlaylistMemberDeleteExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare one guarded playlist-member removal for preview and source-bound export.
pub fn prepare_rekordbox_playlist_member_delete_export(
    request: &PlaylistMemberDeletePreparationRequest,
) -> Result<PreparedPlaylistMemberDeleteExport, RekordboxError> {
    let source = request.source.clone();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared =
        oxidj_rekordbox::prepare_playlist_member_delete(&source_bytes, &request.removal)?;
    Ok(PreparedPlaylistMemberDeleteExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared removal bytes to a new file.
pub fn export_prepared_rekordbox_playlist_member_delete(
    prepared: &PreparedPlaylistMemberDeleteExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist-member removal preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// Request to prepare a guarded playlist-member removal from a source-bound path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMemberDeletePreparationRequest {
    pub source: std::path::PathBuf,
    pub removal: PlaylistMemberDeleteRequest,
}

/// Export one guarded playlist member removal to a new, non-overwriting file.
pub fn export_rekordbox_playlist_member_delete(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistMemberDeleteRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_member_delete(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared playlist clear retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistClearExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedPlaylistClearExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare one guarded playlist clear for preview and source-bound export.
pub fn prepare_rekordbox_playlist_clear_export(
    source: impl AsRef<Path>,
    request: &PlaylistClearRequest,
) -> Result<PreparedPlaylistClearExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_clear(&source_bytes, request)?;
    Ok(PreparedPlaylistClearExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared clear bytes to a new file.
pub fn export_prepared_rekordbox_playlist_clear(
    prepared: &PreparedPlaylistClearExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist clear preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded clearing of playlist membership to a new file.
pub fn export_rekordbox_playlist_clear(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistClearRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_clear(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded append of an existing collection track to a playlist.
pub fn export_rekordbox_playlist_member_append(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistMemberAppendRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_member_append(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded insertion of an existing collection track into a playlist.
pub fn export_rekordbox_playlist_member_insert(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistMemberInsertRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_member_insert(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded collection track deletion and all its playlist memberships.
pub fn export_rekordbox_track_delete(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackDeleteRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_delete(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
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

/// Export a guarded playlist reorder to a new, non-overwriting file.
pub fn export_rekordbox_playlist_reorder(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistReorderRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_reorder(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export a guarded sibling-playlist reorder to a new, non-overwriting file.
pub fn export_rekordbox_sibling_playlist_reorder(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &SiblingPlaylistReorderRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_sibling_playlist_reorder(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export a guarded sibling-folder reorder to a new, non-overwriting file.
pub fn export_rekordbox_sibling_folder_reorder(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &SiblingFolderReorderRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_sibling_folder_reorder(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded playlist move to a new, non-overwriting file.
pub fn export_rekordbox_playlist_move(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistMoveRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_move(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded folder subtree move to a new, non-overwriting file.
pub fn export_rekordbox_folder_move(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &FolderMoveRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_folder_move(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Prepare a guarded sibling-playlist reorder without writing a file.
pub fn prepare_rekordbox_sibling_playlist_reorder(
    input: impl AsRef<Path>,
    request: &SiblingPlaylistReorderRequest,
) -> Result<PreparedSiblingPlaylistReorder, RekordboxError> {
    let source = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_sibling_playlist_reorder(&source, request)
}

/// Export one guarded folder rename to a new, non-overwriting file.
pub fn export_rekordbox_folder_rename(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &FolderRenameRequest,
) -> Result<FolderRenameChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_folder_rename(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Export creation of one guarded empty child folder to a new, non-overwriting file.
pub fn export_rekordbox_folder_create(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &FolderCreateRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_folder_create(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export one guarded empty child playlist to a new, non-overwriting file.
pub fn export_rekordbox_playlist_create(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PlaylistCreateRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_playlist_create(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
}

/// Export deletion of one guarded empty folder to a new, non-overwriting file.
pub fn export_rekordbox_folder_delete(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &FolderDeleteRequest,
) -> Result<(), RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_folder_delete(&source, request)?;
    publish_new_file(&output, &prepared.bytes)
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

/// Export deletion of exactly one guarded position mark to a new file.
pub fn export_rekordbox_position_mark_delete(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &PositionMarkDeleteRequest,
) -> Result<PositionMarkDeleteChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_position_mark_delete(&source, request)?;
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

/// Prepare one guarded tempo-point Battito edit without writing any file.
pub fn prepare_rekordbox_tempo_beat_edit(
    input: impl AsRef<Path>,
    request: &TempoBeatEditRequest,
) -> Result<PreparedTempoBeatEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_tempo_beat_edit(&bytes, request)
}

/// Export one guarded tempo-point Battito edit to a new file.
pub fn export_rekordbox_tempo_beat_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TempoBeatEditRequest,
) -> Result<TempoBeatEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_tempo_beat_edit(&source, request)?;
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change)
}

/// Prepare one guarded raw tempo-point Metro edit without writing any file.
pub fn prepare_rekordbox_tempo_meter_edit(
    input: impl AsRef<Path>,
    request: &TempoMeterEditRequest,
) -> Result<PreparedTempoMeterEdit, RekordboxError> {
    let bytes = std::fs::read(input).map_err(|error| RekordboxError(error.to_string()))?;
    oxidj_rekordbox::prepare_tempo_meter_edit(&bytes, request)
}

/// Export one guarded raw tempo-point Metro edit to a new file.
pub fn export_rekordbox_tempo_meter_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TempoMeterEditRequest,
) -> Result<TempoMeterEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_tempo_meter_edit(&source, request)?;
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

#[cfg(test)]
mod playlist_member_reorder_preparation_tests {
    use super::*;

    fn fixture_source() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../xtask/tests/fixtures/f02-playlist-member-reorder/source.xml")
            .canonicalize()
            .unwrap()
    }

    fn request() -> PlaylistReorderRequest {
        PlaylistReorderRequest {
            playlist_index: 0,
            expected_order: vec!["100".into(), "200".into(), "300".into()],
            new_order: vec!["300".into(), "100".into(), "200".into()],
        }
    }

    #[test]
    fn prepared_reorder_revalidates_source_and_refuses_alias_or_existing_destination() {
        let source = fixture_source();
        let prepared =
            prepare_rekordbox_playlist_member_reorder_export(&source, &request()).unwrap();
        let mut duplicate = request();
        duplicate.new_order = vec!["300".into(), "100".into(), "100".into()];
        assert!(prepare_rekordbox_playlist_member_reorder_export(&source, &duplicate).is_err());
        let base =
            std::env::temp_dir().join(format!("oxidj-member-reorder-{}", std::process::id()));
        let existing = base.with_extension("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_playlist_member_reorder(&prepared, &source).is_err());
        assert!(export_prepared_rekordbox_playlist_member_reorder(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        let stale_source = base.with_extension("source.xml");
        let stale_output = base.with_extension("stale.xml");
        std::fs::copy(&source, &stale_source).unwrap();
        let stale =
            prepare_rekordbox_playlist_member_reorder_export(&stale_source, &request()).unwrap();
        std::fs::write(&stale_source, b"<stale/>").unwrap();
        assert!(export_prepared_rekordbox_playlist_member_reorder(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        let _ = std::fs::remove_file(existing);
        let _ = std::fs::remove_file(stale_source);
    }
}

#[cfg(test)]
mod track_name_preparation_tests {
    use super::*;

    fn fixture_source() -> std::path::PathBuf {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
        path.canonicalize().unwrap()
    }

    #[test]
    fn preparation_returns_preview_bytes_without_writing_source_or_output() {
        let source = fixture_source();
        let before = std::fs::read(&source).unwrap();
        let destination =
            std::env::temp_dir().join(format!("oxidj-track-name-output-{}", std::process::id()));
        let prepared = prepare_rekordbox_track_name_edit(&TrackNameEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_name: "A".into(),
            new_name: "New < title".into(),
        })
        .unwrap();
        assert_eq!(prepared.change.track_id, "100");
        assert_eq!(prepared.change.old_name, "A");
        assert_eq!(prepared.change.new_name, "New < title");
        assert_ne!(prepared.bytes, before);
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert!(!destination.exists());
    }

    #[test]
    fn preparation_rejects_malformed_stale_and_unknown_track_sources() {
        let source = fixture_source();
        for (track_id, expected_old_name) in [("100", "stale"), ("999", "A")] {
            assert!(
                prepare_rekordbox_track_name_edit(&TrackNameEditPreparationRequest {
                    source: source.clone(),
                    track_id: track_id.into(),
                    expected_old_name: expected_old_name.into(),
                    new_name: "New".into(),
                })
                .is_err()
            );
        }
        let malformed =
            std::env::temp_dir().join(format!("oxidj-track-name-malformed-{}", std::process::id()));
        std::fs::write(&malformed, b"<broken").unwrap();
        assert!(
            prepare_rekordbox_track_name_edit(&TrackNameEditPreparationRequest {
                source: malformed.clone(),
                track_id: "100".into(),
                expected_old_name: "A".into(),
                new_name: "New".into(),
            })
            .is_err()
        );
        std::fs::remove_file(malformed).unwrap();
    }
}

#[cfg(test)]
mod playlist_create_preparation_tests {
    use super::*;

    #[test]
    fn playlist_create_preparation_binds_source_and_exports_only_new_destination() {
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
            .canonicalize()
            .unwrap();
        let request = PlaylistCreateRequest {
            parent_index: 0,
            expected_parent_name: "Sets".into(),
            name: "Practice".into(),
        };
        let prepared = prepare_rekordbox_playlist_create_export(&source, &request).unwrap();
        let output =
            std::env::temp_dir().join(format!("oxidj-create-{}-out.xml", std::process::id()));
        let existing =
            std::env::temp_dir().join(format!("oxidj-create-{}-existing.xml", std::process::id()));
        std::fs::write(&existing, b"keep").unwrap();
        assert!(export_prepared_rekordbox_playlist_create(&prepared, &source).is_err());
        assert!(export_prepared_rekordbox_playlist_create(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        export_prepared_rekordbox_playlist_create(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        let _ = std::fs::remove_file(output);
        let _ = std::fs::remove_file(existing);
    }
}
