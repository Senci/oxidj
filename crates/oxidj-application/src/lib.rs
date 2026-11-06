//! Shared application handlers for CLI and future UI callers.

use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
    process::{Command, Stdio},
};
use zeroize::{Zeroize, Zeroizing};

mod library_import;
mod tempo;
mod waveform;
pub use library_import::{
    COMPUTED_GRID_MIN_CONFIDENCE, ComputedGridDecision, DriveLetter, LibraryExportReport,
    LibraryImportReport, LibraryWorkflowError, NotCarried, OneLibraryCrossCheck,
    create_library_playlist, drive_qualified_location, export_library_xml, import_library_from_pdb,
    library_from_pdb_preview, list_library_tracks, location_uri, onelibrary_cross_check,
};
/// The neutral peak model, re-exported so UI adapters need no direct core edge.
pub use oxidj_core::waveform::{Band as WaveformBand, Peak as WaveformPeak, WaveformPeaks};
pub use oxidj_library::{LibraryCounts, LibraryNode, LibrarySnapshot, LibraryTrack};
pub use tempo::{
    DEFAULT_TEMPO_RANGE, FileTempo, TempoAnalysisError, TempoOutcome, TempoReport,
    analyze_catalog_tempo, analyze_tempo_file, analyze_track_tempo,
};
pub use waveform::{
    WaveformAnalysisError, WaveformOutcome, WaveformReport, analyze_catalog_waveforms,
    analyze_track_waveform, analyze_waveform_file,
};

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
    PreparedTrackArtistEdit, PreparedTrackAttributeEdit, PreparedTrackAverageBpmEdit,
    PreparedTrackBitRateEdit, PreparedTrackColourEdit, PreparedTrackCommentsEdit,
    PreparedTrackComposerEdit, PreparedTrackDateAddedEdit, PreparedTrackDateModifiedEdit,
    PreparedTrackDelete, PreparedTrackDiscNumberEdit, PreparedTrackGenreEdit,
    PreparedTrackGroupingEdit, PreparedTrackKindEdit, PreparedTrackLabelEdit,
    PreparedTrackLastPlayedEdit, PreparedTrackMixEdit, PreparedTrackNameEdit,
    PreparedTrackNumberEdit, PreparedTrackPlayCountEdit, PreparedTrackRatingEdit,
    PreparedTrackRemixerEdit, PreparedTrackSampleRateEdit, PreparedTrackSizeEdit,
    PreparedTrackTotalTimeEdit, PreparedTrackYearEdit, Preview, SiblingFolderReorderRequest,
    SiblingPlaylistReorderRequest, TempoBeatEditChange, TempoBeatEditRequest, TempoEditChange,
    TempoEditRequest, TempoMeterEditChange, TempoMeterEditRequest, TempoStartEditChange,
    TempoStartEditRequest, TrackAlbumEditChange, TrackAlbumEditRequest, TrackArtistEditChange,
    TrackArtistEditRequest, TrackAttribute, TrackAttributeEditChange, TrackAttributeEditRequest,
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
/// Rekordbox XML preview hierarchy nodes, for presentation-layer navigation.
pub use oxidj_rekordbox::{Folder, Playlist, Track};

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

/// A source-bound prepared sibling-playlist reorder retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedSiblingPlaylistReorderExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    prepared: PreparedSiblingPlaylistReorder,
}

impl PreparedSiblingPlaylistReorderExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.prepared.bytes
    }
}

pub fn prepare_rekordbox_sibling_playlist_reorder_export(
    source: impl AsRef<Path>,
    request: &SiblingPlaylistReorderRequest,
) -> Result<PreparedSiblingPlaylistReorderExport, RekordboxError> {
    let source_path = source.as_ref().to_path_buf();
    let source_bytes =
        std::fs::read(&source_path).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_sibling_playlist_reorder(&source_bytes, request)?;
    Ok(PreparedSiblingPlaylistReorderExport {
        source: source_path,
        source_bytes,
        prepared,
    })
}

pub fn export_prepared_rekordbox_sibling_playlist_reorder(
    prepared: &PreparedSiblingPlaylistReorderExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after sibling-playlist reorder preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.prepared.bytes)
}

/// A source-bound prepared sibling-folder reorder retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedSiblingFolderReorderExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    prepared: PreparedSiblingFolderReorder,
}
impl PreparedSiblingFolderReorderExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.prepared.bytes
    }
}
pub fn prepare_rekordbox_sibling_folder_reorder_export(
    source: impl AsRef<Path>,
    request: &SiblingFolderReorderRequest,
) -> Result<PreparedSiblingFolderReorderExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_sibling_folder_reorder(&source_bytes, request)?;
    Ok(PreparedSiblingFolderReorderExport {
        source,
        source_bytes,
        prepared,
    })
}
pub fn export_prepared_rekordbox_sibling_folder_reorder(
    prepared: &PreparedSiblingFolderReorderExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after sibling-folder reorder preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.prepared.bytes)
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

/// A source-bound prepared empty-folder deletion retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderDeleteExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}
impl PreparedFolderDeleteExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
/// Prepare a guarded empty-folder deletion for preview and source-bound export.
pub fn prepare_rekordbox_folder_delete_export(
    source: impl AsRef<Path>,
    request: &FolderDeleteRequest,
) -> Result<PreparedFolderDeleteExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_folder_delete(&source_bytes, request)?;
    Ok(PreparedFolderDeleteExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}
/// Revalidate source bytes and publish prepared folder-deletion bytes to a new file.
pub fn export_prepared_rekordbox_folder_delete(
    prepared: &PreparedFolderDeleteExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after folder deletion preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared folder rename retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderRenameExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}
impl PreparedFolderRenameExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
/// Prepare a guarded non-root folder rename for preview and source-bound export.
pub fn prepare_rekordbox_folder_rename_export(
    source: impl AsRef<Path>,
    request: &FolderRenameRequest,
) -> Result<PreparedFolderRenameExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_folder_rename(&source_bytes, request)?;
    Ok(PreparedFolderRenameExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}
/// Revalidate source bytes and publish prepared rename bytes to a new file.
pub fn export_prepared_rekordbox_folder_rename(
    prepared: &PreparedFolderRenameExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after folder rename preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared folder-subtree move retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderMoveExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedFolderMoveExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare a guarded folder-subtree move for preview and source-bound export.
pub fn prepare_rekordbox_folder_move_export(
    source: impl AsRef<Path>,
    request: &FolderMoveRequest,
) -> Result<PreparedFolderMoveExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_folder_move(&source_bytes, request)?;
    Ok(PreparedFolderMoveExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared subtree-move bytes to a new file.
pub fn export_prepared_rekordbox_folder_move(
    prepared: &PreparedFolderMoveExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after folder move preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)
}

/// A source-bound prepared empty-folder creation retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFolderCreateExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}

impl PreparedFolderCreateExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare a guarded empty-folder creation for preview and source-bound export.
pub fn prepare_rekordbox_folder_create_export(
    source: impl AsRef<Path>,
    request: &FolderCreateRequest,
) -> Result<PreparedFolderCreateExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_folder_create(&source_bytes, request)?;
    Ok(PreparedFolderCreateExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish prepared folder-creation bytes to a new file.
pub fn export_prepared_rekordbox_folder_create(
    prepared: &PreparedFolderCreateExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after folder creation preparation".into(),
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

/// A source-bound prepared Track edit export retains the exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackEditExport<C> {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
    change: C,
}

impl<C: Clone> PreparedTrackEditExport<C> {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn change(&self) -> &C {
        &self.change
    }

    /// Publish to a new path only if the source still matches the preview snapshot.
    fn export(&self, output: impl AsRef<Path>, attribute: &str) -> Result<C, RekordboxError> {
        let (current_source, output) = read_new_export_source(&self.source, output)?;
        if current_source != self.source_bytes {
            return Err(RekordboxError(format!(
                "source changed after Track {attribute} edit preparation"
            )));
        }
        publish_new_file(&output, &self.bytes)?;
        Ok(self.change.clone())
    }
}

/// Per-attribute source-bound preparation API: one row per UI-editable Track attribute.
macro_rules! track_edit_exports {
    ($($label:literal => $PrepRequest:ident { $expected:ident, $new:ident } => $Request:ident, $Change:ident, $Export:ident, $adapter:ident, $prepare:ident, $export:ident;)*) => {$(
        #[doc = concat!("Source and exact-old guard for a previewable Track ", $label, " edit.")]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $PrepRequest {
            pub source: std::path::PathBuf,
            pub track_id: String,
            pub $expected: String,
            pub $new: String,
        }
        pub type $Export = PreparedTrackEditExport<$Change>;
        #[doc = concat!("Prepare one guarded ", $label, " edit without writing any file.")]
        pub fn $prepare(request: &$PrepRequest) -> Result<$Export, RekordboxError> {
            let source =
                std::fs::read(&request.source).map_err(|error| RekordboxError(error.to_string()))?;
            let prepared = oxidj_rekordbox::$adapter(
                &source,
                &$Request {
                    track_id: request.track_id.clone(),
                    $expected: request.$expected.clone(),
                    $new: request.$new.clone(),
                },
            )?;
            Ok(PreparedTrackEditExport {
                source: request.source.clone(),
                source_bytes: source,
                bytes: prepared.bytes,
                change: prepared.change,
            })
        }
        #[doc = concat!("Export a prepared ", $label, " edit only after source revalidation, to a new path.")]
        pub fn $export(prepared: &$Export, output: impl AsRef<Path>) -> Result<$Change, RekordboxError> {
            prepared.export(output, $label)
        }
    )*};
}

#[rustfmt::skip]
track_edit_exports! {
    "Name" => TrackNameEditPreparationRequest { expected_old_name, new_name } => TrackNameEditRequest, TrackNameEditChange, PreparedTrackNameExport, prepare_track_name_edit, prepare_rekordbox_track_name_export, export_prepared_rekordbox_track_name_edit;
    "Artist" => TrackArtistEditPreparationRequest { expected_old_artist, new_artist } => TrackArtistEditRequest, TrackArtistEditChange, PreparedTrackArtistExport, prepare_track_artist_edit, prepare_rekordbox_track_artist_export, export_prepared_rekordbox_track_artist_edit;
    "Comments" => TrackCommentsEditPreparationRequest { expected_old_comments, new_comments } => TrackCommentsEditRequest, TrackCommentsEditChange, PreparedTrackCommentsExport, prepare_track_comments_edit, prepare_rekordbox_track_comments_export, export_prepared_rekordbox_track_comments_edit;
    "Album" => TrackAlbumEditPreparationRequest { expected_old_album, new_album } => TrackAlbumEditRequest, TrackAlbumEditChange, PreparedTrackAlbumExport, prepare_track_album_edit, prepare_rekordbox_track_album_export, export_prepared_rekordbox_track_album_edit;
    "Genre" => TrackGenreEditPreparationRequest { expected_old_genre, new_genre } => TrackGenreEditRequest, TrackGenreEditChange, PreparedTrackGenreExport, prepare_track_genre_edit, prepare_rekordbox_track_genre_export, export_prepared_rekordbox_track_genre_edit;
    "Label" => TrackLabelEditPreparationRequest { expected_old_label, new_label } => TrackLabelEditRequest, TrackLabelEditChange, PreparedTrackLabelExport, prepare_track_label_edit, prepare_rekordbox_track_label_export, export_prepared_rekordbox_track_label_edit;
    "Composer" => TrackComposerEditPreparationRequest { expected_old_composer, new_composer } => TrackComposerEditRequest, TrackComposerEditChange, PreparedTrackComposerExport, prepare_track_composer_edit, prepare_rekordbox_track_composer_export, export_prepared_rekordbox_track_composer_edit;
    "Grouping" => TrackGroupingEditPreparationRequest { expected_old_grouping, new_grouping } => TrackGroupingEditRequest, TrackGroupingEditChange, PreparedTrackGroupingExport, prepare_track_grouping_edit, prepare_rekordbox_track_grouping_export, export_prepared_rekordbox_track_grouping_edit;
    "Remixer" => TrackRemixerEditPreparationRequest { expected_old_remixer, new_remixer } => TrackRemixerEditRequest, TrackRemixerEditChange, PreparedTrackRemixerExport, prepare_track_remixer_edit, prepare_rekordbox_track_remixer_export, export_prepared_rekordbox_track_remixer_edit;
    "Mix" => TrackMixEditPreparationRequest { expected_old_mix, new_mix } => TrackMixEditRequest, TrackMixEditChange, PreparedTrackMixExport, prepare_track_mix_edit, prepare_rekordbox_track_mix_export, export_prepared_rekordbox_track_mix_edit;
    "Tonality" => TrackTonalityEditPreparationRequest { expected_old_tonality, new_tonality } => TrackTonalityEditRequest, TrackTonalityEditChange, PreparedTrackTonalityExport, prepare_track_tonality_edit, prepare_rekordbox_track_tonality_export, export_prepared_rekordbox_track_tonality_edit;
    "Kind" => TrackKindEditPreparationRequest { expected_old_kind, new_kind } => TrackKindEditRequest, TrackKindEditChange, PreparedTrackKindExport, prepare_track_kind_edit, prepare_rekordbox_track_kind_export, export_prepared_rekordbox_track_kind_edit;
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
    oxidj_rekordbox::prepare_track_name_edit(&source, &adapter_request)
}

/// Read-only application handler: prepare one guarded Track Artist edit for preview.
pub fn prepare_rekordbox_track_artist_edit(
    request: &TrackArtistEditPreparationRequest,
) -> Result<PreparedTrackArtistEdit, RekordboxError> {
    let source =
        std::fs::read(&request.source).map_err(|error| RekordboxError(error.to_string()))?;
    let adapter_request = TrackArtistEditRequest {
        track_id: request.track_id.clone(),
        expected_old_artist: request.expected_old_artist.clone(),
        new_artist: request.new_artist.clone(),
    };
    oxidj_rekordbox::prepare_track_artist_edit(&source, &adapter_request)
}

/// Export one guarded single-attribute Track edit to a new, non-overwriting file.
pub fn export_rekordbox_track_attribute_edit(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    request: &TrackAttributeEditRequest,
) -> Result<TrackAttributeEditChange, RekordboxError> {
    let (source, output) = read_new_export_source(input, output)?;
    let prepared = oxidj_rekordbox::prepare_track_attribute_edit(&source, request)?;
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

/// A source-bound prepared whole-playlist deletion retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistDeleteExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
}
impl PreparedPlaylistDeleteExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Prepare deletion of one guarded playlist node for preview and source-bound export.
pub fn prepare_rekordbox_playlist_delete_export(
    source: impl AsRef<Path>,
    request: &PlaylistDeleteRequest,
) -> Result<PreparedPlaylistDeleteExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_delete(&source_bytes, request)?;
    Ok(PreparedPlaylistDeleteExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
    })
}

/// Revalidate source bytes and publish only the prepared deletion to a new file.
pub fn export_prepared_rekordbox_playlist_delete(
    prepared: &PreparedPlaylistDeleteExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist deletion preparation".into(),
        ));
    }
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

/// A source-bound prepared playlist rename retains its exact preview snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistRenameExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    bytes: Vec<u8>,
    change: PlaylistRenameChange,
}

impl PreparedPlaylistRenameExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn change(&self) -> &PlaylistRenameChange {
        &self.change
    }
}

/// Prepare one guarded playlist rename for preview and new-file export.
pub fn prepare_rekordbox_playlist_rename_export(
    source: impl AsRef<Path>,
    request: &PlaylistRenameRequest,
) -> Result<PreparedPlaylistRenameExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_rename(&source_bytes, request)?;
    Ok(PreparedPlaylistRenameExport {
        source,
        source_bytes,
        bytes: prepared.bytes,
        change: prepared.change,
    })
}

/// Revalidate the source and publish only previously prepared rename bytes.
pub fn export_prepared_rekordbox_playlist_rename(
    prepared: &PreparedPlaylistRenameExport,
    output: impl AsRef<Path>,
) -> Result<PlaylistRenameChange, RekordboxError> {
    let (current_source, output) = read_new_export_source(&prepared.source, output)?;
    if current_source != prepared.source_bytes {
        return Err(RekordboxError(
            "source changed after playlist rename preparation".into(),
        ));
    }
    publish_new_file(&output, &prepared.bytes)?;
    Ok(prepared.change.clone())
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

/// Source-bound prepared playlist move for preview and revalidated export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlaylistMoveExport {
    source: std::path::PathBuf,
    source_bytes: Vec<u8>,
    prepared: PreparedPlaylistMove,
}
impl PreparedPlaylistMoveExport {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn bytes(&self) -> &[u8] {
        &self.prepared.bytes
    }
}

/// Prepare a guarded playlist move without writing an output file.
pub fn prepare_rekordbox_playlist_move_export(
    source: impl AsRef<Path>,
    request: &PlaylistMoveRequest,
) -> Result<PreparedPlaylistMoveExport, RekordboxError> {
    let source = source.as_ref().to_path_buf();
    let source_bytes = std::fs::read(&source).map_err(|error| RekordboxError(error.to_string()))?;
    let prepared = oxidj_rekordbox::prepare_playlist_move(&source_bytes, request)?;
    Ok(PreparedPlaylistMoveExport {
        source,
        source_bytes,
        prepared,
    })
}

/// Revalidate the source snapshot and publish only the previously prepared bytes.
pub fn export_prepared_rekordbox_playlist_move(
    prepared: &PreparedPlaylistMoveExport,
    output: impl AsRef<Path>,
) -> Result<(), RekordboxError> {
    let current =
        std::fs::read(&prepared.source).map_err(|error| RekordboxError(error.to_string()))?;
    if current != prepared.source_bytes {
        return Err(RekordboxError("source changed since preparation".into()));
    }
    let output = output.as_ref();
    if same_canonical_path(output, &prepared.source) {
        return Err(RekordboxError("output aliases source".into()));
    }
    publish_new_file(output, &prepared.prepared.bytes)
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

/// True when both paths resolve to the same file, even via symlinks or
/// relative spellings; the output need not exist yet.
fn same_canonical_path(output: &Path, source: &Path) -> bool {
    let canonical_output = std::fs::canonicalize(output).or_else(|_| {
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = output
            .file_name()
            .ok_or_else(|| std::io::Error::other("no file name"))?;
        std::fs::canonicalize(parent).map(|p| p.join(name))
    });
    match (canonical_output, std::fs::canonicalize(source)) {
        (Ok(a), Ok(b)) => a == b,
        _ => output == source,
    }
}

pub(crate) fn publish_new_file(output: &Path, bytes: &[u8]) -> Result<(), RekordboxError> {
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
    // Only a temp file this call created may be removed on failure; a
    // pre-existing file at the temp name belongs to someone else.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| RekordboxError(e.to_string()))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|e| RekordboxError(e.to_string()))?;
        file.sync_all().map_err(|e| RekordboxError(e.to_string()))?;
        oxidj_core::publish::publish_no_clobber(&temp, output).map_err(|e| match e {
            oxidj_core::publish::PublishError::DestinationExists => {
                RekordboxError("refusing to overwrite existing output".into())
            }
            e => RekordboxError(e.to_string()),
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod playlist_rename_preparation_tests {
    use super::*;

    fn source_fixture() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../xtask/tests/fixtures/f02-playlist-rename/source.xml")
            .canonicalize()
            .unwrap()
    }

    fn request() -> PlaylistRenameRequest {
        PlaylistRenameRequest {
            playlist_index: 0,
            expected_old_name: "House".into(),
            new_name: "Summer & Sun".into(),
        }
    }

    #[test]
    fn prepared_rename_is_read_only_and_export_revalidates_source_and_destination() {
        let source = source_fixture();
        let before = std::fs::read(&source).unwrap();
        let prepared = prepare_rekordbox_playlist_rename_export(&source, &request()).unwrap();
        assert_eq!(prepared.source(), source);
        assert_eq!(std::fs::read(&source).unwrap(), before);
        let base =
            std::env::temp_dir().join(format!("oxidj-playlist-rename-{}", std::process::id()));
        let existing = base.with_extension("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(export_prepared_rekordbox_playlist_rename(&prepared, &source).is_err());
        assert!(export_prepared_rekordbox_playlist_rename(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        let stale_source = base.with_extension("source.xml");
        let stale_output = base.with_extension("stale.xml");
        std::fs::copy(&source, &stale_source).unwrap();
        let stale = prepare_rekordbox_playlist_rename_export(&stale_source, &request()).unwrap();
        std::fs::write(&stale_source, b"<stale/>").unwrap();
        assert!(export_prepared_rekordbox_playlist_rename(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        let _ = std::fs::remove_file(existing);
        let _ = std::fs::remove_file(stale_source);
    }
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
mod track_artist_preparation_tests {
    use super::*;

    fn copy_fixture(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-artist-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        source
    }

    #[test]
    fn preparation_and_export_are_source_bound_and_refuse_aliases_and_overwrites() {
        let root = std::env::temp_dir().join(format!(
            "oxidj-track-artist-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = copy_fixture(&root);
        let original = std::fs::read(&source).unwrap();
        let request = TrackArtistEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_artist: "Old & exact".into(),
            new_artist: "New < artist & \"guest\"\tline\ncarriage\r".into(),
        };
        let prepared = prepare_rekordbox_track_artist_export(&request).unwrap();
        assert_eq!(prepared.change().old_artist, "Old & exact");
        assert_eq!(prepared.change().new_artist, request.new_artist);
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let alias = export_prepared_rekordbox_track_artist_edit(&prepared, &source);
        assert!(alias.is_err(), "source alias must be refused");
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_artist_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");

        let output = root.join("edited.xml");
        let change = export_prepared_rekordbox_track_artist_edit(&prepared, &output).unwrap();
        assert_eq!(change.new_artist, request.new_artist);
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared.bytes());
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("New &lt; artist &amp; &quot;guest&quot;&#9;line&#10;carriage&#13;"));
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let reopened = preview_rekordbox_xml(&output).unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .artist
                .as_deref(),
            Some(request.new_artist.as_str())
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "200")
                .unwrap()
                .artist,
            None
        );

        let stale = prepare_rekordbox_track_artist_export(&request).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_artist_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preparation_refuses_absent_stale_and_invalid_artist_values() {
        let root = std::env::temp_dir().join(format!(
            "oxidj-track-artist-refusal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = copy_fixture(&root);
        for (track_id, expected_old_artist, new_artist) in [
            ("200", "Old & exact", "New"),
            ("100", "stale", "New"),
            ("100", "Old & exact", "bad\u{1}"),
        ] {
            let request = TrackArtistEditPreparationRequest {
                source: source.clone(),
                track_id: track_id.into(),
                expected_old_artist: expected_old_artist.into(),
                new_artist: new_artist.into(),
            };
            assert!(prepare_rekordbox_track_artist_export(&request).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn present_empty_artist_can_be_prepared_without_inserting_absent_attributes() {
        let root = std::env::temp_dir().join(format!(
            "oxidj-track-artist-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = copy_fixture(&root);
        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Artist=\"Old &amp; exact\"", "Artist=\"\"");
        std::fs::write(&source, xml).unwrap();
        let request = TrackArtistEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_artist: String::new(),
            new_artist: "Restored".into(),
        };
        assert!(prepare_rekordbox_track_artist_edit(&request).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod track_comments_preparation_tests {
    use super::*;

    fn copy_fixture(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-comments-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        source
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oxidj-track-comments-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn prepared_comments_export_is_source_bound_and_never_overwrites() {
        let root = temp_root("export");
        std::fs::create_dir_all(&root).unwrap();
        let source = copy_fixture(&root);
        let original = std::fs::read(&source).unwrap();
        let request = TrackCommentsEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_comments: "old & note".into(),
            new_comments: "new < note\nsecond line".into(),
        };
        let prepared = prepare_rekordbox_track_comments_export(&request).unwrap();
        assert_eq!(prepared.change().old_comments, "old & note");
        assert_eq!(prepared.change().new_comments, request.new_comments);
        assert_eq!(std::fs::read(&source).unwrap(), original);

        assert!(export_prepared_rekordbox_track_comments_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_comments_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");

        let output = root.join("edited.xml");
        let change = export_prepared_rekordbox_track_comments_edit(&prepared, &output).unwrap();
        assert_eq!(change.new_comments, request.new_comments);
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(bytes, prepared.bytes());
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Comments=\"new &lt; note&#10;second line\""));
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let reopened = preview_rekordbox_xml(&output).unwrap();
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "100")
                .unwrap()
                .comments
                .as_deref(),
            Some(request.new_comments.as_str())
        );
        assert_eq!(
            reopened
                .tracks
                .iter()
                .find(|track| track.id == "200")
                .unwrap()
                .comments,
            None
        );

        let stale = prepare_rekordbox_track_comments_export(&request).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_comments_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preparation_refuses_absent_stale_and_invalid_comments_but_accepts_present_empty() {
        let root = temp_root("guards");
        std::fs::create_dir_all(&root).unwrap();
        let source = copy_fixture(&root);
        for (track_id, expected_old_comments, new_comments) in [
            ("200", "old & note", "new"),
            ("100", "stale", "new"),
            ("100", "old & note", "bad\u{1}"),
        ] {
            let request = TrackCommentsEditPreparationRequest {
                source: source.clone(),
                track_id: track_id.into(),
                expected_old_comments: expected_old_comments.into(),
                new_comments: new_comments.into(),
            };
            assert!(prepare_rekordbox_track_comments_export(&request).is_err());
        }

        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Comments=\"old &amp; note\"", "Comments=\"\"");
        std::fs::write(&source, xml).unwrap();
        let request = TrackCommentsEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_comments: String::new(),
            new_comments: "restored".into(),
        };
        assert!(prepare_rekordbox_track_comments_export(&request).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod track_album_preparation_tests {
    use super::*;

    fn setup() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-track-album-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-album-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        (root, source)
    }

    fn request(source: std::path::PathBuf, new_album: &str) -> TrackAlbumEditPreparationRequest {
        TrackAlbumEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_album: "old & record".into(),
            new_album: new_album.into(),
        }
    }

    #[test]
    fn prepared_album_export_is_source_bound_and_never_overwrites() {
        let (root, source) = setup();
        let original = std::fs::read(&source).unwrap();
        let request = request(source.clone(), "new < record");
        let prepared = prepare_rekordbox_track_album_export(&request).unwrap();
        assert_eq!(prepared.change().old_album, "old & record");
        assert_eq!(prepared.change().new_album, "new < record");
        assert_eq!(std::fs::read(&source).unwrap(), original);

        assert!(export_prepared_rekordbox_track_album_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_album_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");

        let output = root.join("edited.xml");
        let change = export_prepared_rekordbox_track_album_edit(&prepared, &output).unwrap();
        assert_eq!(change.new_album, request.new_album);
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        assert_eq!(std::fs::read(&source).unwrap(), original);

        let stale = prepare_rekordbox_track_album_export(&request).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_album_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn album_preparation_refuses_absent_stale_and_invalid_values_but_accepts_empty() {
        let (root, source) = setup();
        for (track_id, expected_old_album, new_album) in [
            ("200", "old & record", "new"),
            ("100", "stale", "new"),
            ("100", "old & record", "bad\u{1}"),
        ] {
            let mut request = request(source.clone(), new_album);
            request.track_id = track_id.into();
            request.expected_old_album = expected_old_album.into();
            assert!(prepare_rekordbox_track_album_export(&request).is_err());
        }

        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Album=\"old &amp; record\"", "Album=\"\"");
        std::fs::write(&source, xml).unwrap();
        let request = TrackAlbumEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_album: String::new(),
            new_album: "restored".into(),
        };
        assert!(prepare_rekordbox_track_album_export(&request).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod playlist_delete_preparation_tests {
    use super::*;

    #[test]
    fn whole_playlist_delete_preparation_is_source_bound_and_never_overwrites() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-playlist-delete-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../xtask/tests/fixtures/f02-playlist-delete/source.xml"),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let request = PlaylistDeleteRequest {
            playlist_index: 1,
            expected_name: "Techno".into(),
            expected_order: vec!["200".into(), "400".into()],
        };
        let prepared = prepare_rekordbox_playlist_delete_export(&source, &request).unwrap();
        assert_ne!(prepared.bytes(), original);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(export_prepared_rekordbox_playlist_delete(&prepared, &source).is_err());
        assert!(export_prepared_rekordbox_playlist_delete(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        let output = root.join("deleted.xml");
        export_prepared_rekordbox_playlist_delete(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        let stale_source = root.join("stale.xml");
        std::fs::copy(&source, &stale_source).unwrap();
        let stale = prepare_rekordbox_playlist_delete_export(&stale_source, &request).unwrap();
        std::fs::write(&stale_source, b"changed").unwrap();
        let stale_output = root.join("stale-output.xml");
        assert!(export_prepared_rekordbox_playlist_delete(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod folder_create_preparation_tests {
    use super::*;

    #[test]
    fn folder_create_preparation_is_read_only_source_bound_and_never_overwrites() {
        let root =
            std::env::temp_dir().join(format!("oxidj-folder-create-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
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
        let request = FolderCreateRequest {
            parent_index: 0,
            expected_parent_name: "Sets".into(),
            name: "Practice".into(),
        };
        let prepared = prepare_rekordbox_folder_create_export(&source, &request).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(export_prepared_rekordbox_folder_create(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(export_prepared_rekordbox_folder_create(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep");
        std::fs::write(&source, b"stale source").unwrap();
        let stale = root.join("stale.xml");
        assert!(export_prepared_rekordbox_folder_create(&prepared, &stale).is_err());
        assert!(!stale.exists());
        let _ = std::fs::remove_dir_all(root);
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

#[cfg(test)]
mod track_genre_preparation_tests {
    use super::*;

    fn setup() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-track-genre-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-genre-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        (root, source)
    }

    fn request(
        source: std::path::PathBuf,
        old: &str,
        new: &str,
    ) -> TrackGenreEditPreparationRequest {
        TrackGenreEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_genre: old.into(),
            new_genre: new.into(),
        }
    }

    #[test]
    fn prepared_genre_export_is_source_bound_and_never_overwrites() {
        let (root, source) = setup();
        let original = std::fs::read(&source).unwrap();
        let req = request(source.clone(), "old & style", "new < style");
        let prepared = prepare_rekordbox_track_genre_export(&req).unwrap();
        assert_eq!(prepared.change().old_genre, "old & style");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(export_prepared_rekordbox_track_genre_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_genre_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_genre_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let stale = prepare_rekordbox_track_genre_export(&req).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_genre_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn genre_preparation_refuses_absent_and_stale_values_but_accepts_empty() {
        let (root, source) = setup();
        for (id, old) in [("200", "old & style"), ("100", "stale")] {
            let mut req = request(source.clone(), old, "new");
            req.track_id = id.into();
            assert!(prepare_rekordbox_track_genre_export(&req).is_err());
        }
        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Genre=\"old &amp; style\"", "Genre=\"\"");
        std::fs::write(&source, xml).unwrap();
        let prepared =
            prepare_rekordbox_track_genre_export(&request(source.clone(), "", "new")).unwrap();
        assert_eq!(prepared.change().old_genre, "");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod track_label_preparation_tests {
    use super::*;

    fn setup() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-track-label-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-label-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        (root, source)
    }

    fn request(
        source: std::path::PathBuf,
        old: &str,
        new: &str,
    ) -> TrackLabelEditPreparationRequest {
        TrackLabelEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_label: old.into(),
            new_label: new.into(),
        }
    }

    #[test]
    fn prepared_label_export_is_source_bound_and_never_overwrites() {
        let (root, source) = setup();
        let original = std::fs::read(&source).unwrap();
        let req = request(source.clone(), "old & imprint", "new < imprint");
        let prepared = prepare_rekordbox_track_label_export(&req).unwrap();
        assert_eq!(prepared.change().old_label, "old & imprint");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(export_prepared_rekordbox_track_label_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_label_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_label_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let stale = prepare_rekordbox_track_label_export(&req).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_label_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn label_preparation_refuses_absent_and_stale_values_but_accepts_empty() {
        let (root, source) = setup();
        for (id, old) in [("200", "old & imprint"), ("100", "stale")] {
            let mut req = request(source.clone(), old, "new");
            req.track_id = id.into();
            assert!(prepare_rekordbox_track_label_export(&req).is_err());
        }
        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Label=\"old &amp; imprint\"", "Label=\"\"");
        std::fs::write(&source, xml).unwrap();
        let prepared =
            prepare_rekordbox_track_label_export(&request(source.clone(), "", "new")).unwrap();
        assert_eq!(prepared.change().old_label, "");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod track_remixer_preparation_tests {
    use super::*;

    fn setup() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-track-remixer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-remixer-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        (root, source)
    }

    fn request(
        source: std::path::PathBuf,
        old: &str,
        new: &str,
    ) -> TrackRemixerEditPreparationRequest {
        TrackRemixerEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_remixer: old.into(),
            new_remixer: new.into(),
        }
    }

    #[test]
    fn prepared_remixer_export_is_source_bound_and_never_overwrites() {
        let (root, source) = setup();
        let original = std::fs::read(&source).unwrap();
        let req = request(source.clone(), "old & mix", "new < mix");
        let prepared = prepare_rekordbox_track_remixer_export(&req).unwrap();
        assert_eq!(prepared.change().old_remixer, "old & mix");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(export_prepared_rekordbox_track_remixer_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_remixer_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_remixer_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let stale = prepare_rekordbox_track_remixer_export(&req).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_remixer_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remixer_preparation_refuses_absent_and_stale_values_but_accepts_empty() {
        let (root, source) = setup();
        for (id, old) in [("200", "old & mix"), ("100", "stale")] {
            let mut req = request(source.clone(), old, "new");
            req.track_id = id.into();
            assert!(prepare_rekordbox_track_remixer_export(&req).is_err());
        }
        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Remixer=\"old &amp; mix\"", "Remixer=\"\"");
        std::fs::write(&source, xml).unwrap();
        let prepared =
            prepare_rekordbox_track_remixer_export(&request(source.clone(), "", "new")).unwrap();
        assert_eq!(prepared.change().old_remixer, "");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod track_mix_preparation_tests {
    use super::*;

    fn setup() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "oxidj-app-track-mix-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-mix-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        (root, source)
    }

    fn request(source: std::path::PathBuf, old: &str, new: &str) -> TrackMixEditPreparationRequest {
        TrackMixEditPreparationRequest {
            source,
            track_id: "100".into(),
            expected_old_mix: old.into(),
            new_mix: new.into(),
        }
    }

    #[test]
    fn prepared_mix_export_is_source_bound_and_never_overwrites() {
        let (root, source) = setup();
        let original = std::fs::read(&source).unwrap();
        let req = request(source.clone(), "old & set", "new < set");
        let prepared = prepare_rekordbox_track_mix_export(&req).unwrap();
        assert_eq!(prepared.change().old_mix, "old & set");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(export_prepared_rekordbox_track_mix_edit(&prepared, &source).is_err());
        let existing = root.join("existing.xml");
        std::fs::write(&existing, b"preserve").unwrap();
        assert!(export_prepared_rekordbox_track_mix_edit(&prepared, &existing).is_err());
        assert_eq!(std::fs::read(&existing).unwrap(), b"preserve");
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_mix_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let stale = prepare_rekordbox_track_mix_export(&req).unwrap();
        std::fs::write(&source, b"<changed/>").unwrap();
        let stale_output = root.join("stale.xml");
        assert!(export_prepared_rekordbox_track_mix_edit(&stale, &stale_output).is_err());
        assert!(!stale_output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mix_preparation_refuses_absent_and_stale_values_but_accepts_empty() {
        let (root, source) = setup();
        for (id, old) in [("200", "old & set"), ("100", "stale")] {
            let mut req = request(source.clone(), old, "new");
            req.track_id = id.into();
            assert!(prepare_rekordbox_track_mix_export(&req).is_err());
        }
        let xml = std::fs::read_to_string(&source)
            .unwrap()
            .replace("Mix=\"old &amp; set\"", "Mix=\"\"");
        std::fs::write(&source, xml).unwrap();
        let prepared =
            prepare_rekordbox_track_mix_export(&request(source.clone(), "", "new")).unwrap();
        assert_eq!(prepared.change().old_mix, "");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tonality_preparation_is_source_bound_and_refuses_absent_values() {
        let root = std::env::temp_dir().join(format!("oxidj-app-tonality-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-tonality-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        let request = TrackTonalityEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_tonality: "old & key".into(),
            new_tonality: "new < key".into(),
        };
        let original = std::fs::read(&source).unwrap();
        let prepared = prepare_rekordbox_track_tonality_export(&request).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_tonality_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        let mut absent = request.clone();
        absent.track_id = "200".into();
        assert!(prepare_rekordbox_track_tonality_export(&absent).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn kind_preparation_is_source_bound_and_refuses_absent_values() {
        let root = std::env::temp_dir().join(format!("oxidj-app-kind-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.xml");
        std::fs::copy(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../xtask/tests/fixtures/f02-track-kind-edit/source.xml"
            ),
            &source,
        )
        .unwrap();
        let request = TrackKindEditPreparationRequest {
            source: source.clone(),
            track_id: "100".into(),
            expected_old_kind: "old & kind".into(),
            new_kind: "new < kind".into(),
        };
        let original = std::fs::read(&source).unwrap();
        let prepared = prepare_rekordbox_track_kind_export(&request).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let output = root.join("out.xml");
        export_prepared_rekordbox_track_kind_edit(&prepared, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), prepared.bytes());
        let mut absent = request.clone();
        absent.track_id = "200".into();
        assert!(prepare_rekordbox_track_kind_export(&absent).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod publish_new_file_tests {
    use super::*;

    #[test]
    fn publish_refusal_never_removes_a_preexisting_temp_name_file() {
        let root = std::env::temp_dir().join(format!("oxidj-app-publish-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("out.xml");
        let foreign = root.join(format!(".out.xml.oxidj-tmp-{}", std::process::id()));
        std::fs::write(&foreign, b"not ours").unwrap();
        assert!(publish_new_file(&output, b"new").is_err());
        assert_eq!(std::fs::read(&foreign).unwrap(), b"not ours");
        assert!(!output.exists());
        std::fs::remove_file(&foreign).unwrap();
        publish_new_file(&output, b"new").unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"new");
        assert!(
            !foreign.exists(),
            "own temp file is cleaned up after publish"
        );
        assert!(publish_new_file(&output, b"other").is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"new", "never overwrite");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod canonical_alias_tests {
    use super::*;

    #[test]
    fn symlink_and_relative_spellings_alias_the_source() {
        let root = std::env::temp_dir().join(format!("oxidj-app-alias-{}", std::process::id()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let source = root.join("src.xml");
        std::fs::write(&source, b"x").unwrap();
        std::os::unix::fs::symlink(&source, root.join("link.xml")).unwrap();
        assert!(same_canonical_path(&root.join("link.xml"), &source));
        assert!(same_canonical_path(&root.join("sub/../src.xml"), &source));
        assert!(!same_canonical_path(&root.join("new.xml"), &source));
        std::fs::remove_dir_all(root).unwrap();
    }
}
