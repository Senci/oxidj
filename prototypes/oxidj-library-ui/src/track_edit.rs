//! One guarded single-attribute TRACK edit workflow, driven by a field table.
//!
//! Every editable attribute (except the original Name edit, whose contract
//! predates selection/proposal guards and re-open; see `name_edit.rs`) shares
//! the same state machine:
//!
//! prepare (absent attribute and missing document refused) → pending preview →
//! cancel, or export to a new file only after the loaded source, editable path,
//! selected track, proposed value and previewed old value are all unchanged →
//! re-open the exported file as the loaded document.
//!
//! The per-attribute table at the bottom binds each attribute to its
//! `oxidj_application::TrackAttribute`, its preview field, its UI state fields and
//! its preserved panel presentation.

use crate::LibraryApp;
use crate::components::SectionHeading;
use crate::theme;
use eframe::egui;
use oxidj_application::{PreparedTrackEditExport, RekordboxError, Track, TrackAttribute};
use std::path::{Path, PathBuf};

/// A prepared, source-bound pending TRACK edit (preview only; nothing written).
#[derive(Debug, Clone)]
pub(crate) struct PendingTrackEdit<C> {
    pub(crate) source: PathBuf,
    pub(crate) track_id: String,
    pub(crate) prepared: PreparedTrackEditExport<C>,
}

/// Panel heading style.
#[derive(Clone, Copy)]
pub(crate) enum Heading {
    /// Separator + "Prepare one guarded X edit".
    Strong,
    /// "Track X edit (provisional)" section heading.
    Section,
}

/// How the current attribute value is shown above the editor.
#[derive(Clone, Copy)]
pub(crate) enum Current {
    Hidden,
    /// "present" / "present but empty" only (value not echoed).
    Presence,
    /// The value, Debug-quoted.
    Quoted,
    /// The value, Debug-quoted, plus a "(present but empty)" note.
    QuotedEmptyNote,
}

/// Panel behaviour when the selected track has no such attribute.
#[derive(Clone, Copy)]
pub(crate) enum Absent {
    /// Show the note and still show the editor (preparation then refuses).
    ShowEditor,
    /// Show the note only.
    Note,
    /// Discard any pending edit and show the note.
    ClearAndNote,
}

#[derive(Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) heading: Heading,
    pub(crate) current: Current,
    pub(crate) absent: Absent,
    pub(crate) multiline: bool,
}

/// One row of the field table: binds an attribute to its application API and UI state.
pub(crate) trait TrackEditField {
    type Change: Clone;
    const ATTRIBUTE: TrackAttribute;
    const LAYOUT: Layout;
    fn current(track: &Track) -> Option<&str>;
    fn old(change: &Self::Change) -> &str;
    fn new(change: &Self::Change) -> &str;
    fn input(app: &mut LibraryApp) -> &mut String;
    fn export_path(app: &mut LibraryApp) -> &mut String;
    fn pending(app: &mut LibraryApp) -> &mut Option<PendingTrackEdit<Self::Change>>;
    fn prepare(
        source: PathBuf,
        track_id: &str,
        old: &str,
        new: String,
    ) -> Result<PreparedTrackEditExport<Self::Change>, RekordboxError>;
    fn export(
        prepared: &PreparedTrackEditExport<Self::Change>,
        output: &Path,
    ) -> Result<Self::Change, RekordboxError>;
}

impl LibraryApp {
    pub(crate) fn prepare_track_edit<F: TrackEditField>(
        &mut self,
        track_id: &str,
        old: Option<&str>,
    ) {
        let label = F::ATTRIBUTE.xml_name();
        let Some(old) = old else {
            *F::pending(self) = None;
            self.xml_edit_error = Some(format!(
                "Track {label} preparation refused: selected track has no {label} attribute"
            ));
            return;
        };
        let Some(source) = self.xml_document_source.clone() else {
            *F::pending(self) = None;
            self.xml_edit_error = Some(format!(
                "Track {label} preparation refused: no XML document is loaded"
            ));
            return;
        };
        let new = F::input(self).clone();
        match F::prepare(source.clone(), track_id, old, new) {
            Ok(prepared) => {
                *F::pending(self) = Some(PendingTrackEdit {
                    source,
                    track_id: track_id.to_owned(),
                    prepared,
                });
                self.xml_edit_error = None;
            }
            Err(error) => {
                *F::pending(self) = None;
                self.xml_edit_error = Some(format!("Track {label} preparation refused: {error}"));
            }
        }
    }

    pub(crate) fn cancel_track_edit<F: TrackEditField>(&mut self) {
        *F::pending(self) = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_track_edit<F: TrackEditField>(&mut self) {
        let label = F::ATTRIBUTE.xml_name();
        self.notice.clear();
        let Some(pending) = F::pending(self).clone() else {
            self.xml_edit_error = Some(format!(
                "Track {label} export refused: no prepared pending edit"
            ));
            return;
        };
        let proposal = pending.prepared.change();
        let selected = self
            .xml_preview
            .as_ref()
            .and_then(|preview| preview.as_ref().ok())
            .and_then(|preview| preview.tracks.iter().find(|t| t.id == pending.track_id))
            .and_then(F::current)
            .map(str::to_owned);
        if self.xml_document_source.as_ref() != Some(&pending.source)
            || PathBuf::from(&self.xml_path) != pending.source
            || self.xml_selected_track.as_deref() != Some(&pending.track_id)
            || F::input(self).as_str() != F::new(proposal)
            || selected.as_deref() != Some(F::old(proposal))
        {
            *F::pending(self) = None;
            self.xml_edit_error = Some(format!(
                "Track {label} export refused: source, selected track, or proposed value changed"
            ));
            return;
        }
        let output = PathBuf::from(F::export_path(self).as_str());
        if F::export_path(self).trim().is_empty() {
            self.xml_edit_error = Some(format!(
                "Track {label} export refused: choose a new XML destination"
            ));
            return;
        }
        match F::export(&pending.prepared, &output) {
            Ok(change) => {
                *F::pending(self) = None;
                self.xml_edit_error = None;
                self.notice = format!("Track {label} exported to {}", output.display());
                *F::export_path(self) = output.display().to_string();
                self.xml_path = output.display().to_string();
                self.xml_document_source = Some(output.clone());
                self.xml_preview = Some(
                    oxidj_application::preview_rekordbox_xml(&output)
                        .map_err(|error| error.to_string()),
                );
                *F::input(self) = F::new(&change).to_owned();
            }
            Err(error) => {
                self.xml_edit_error = Some(format!("Track {label} export failed: {error}"));
            }
        }
    }

    /// Generic panel for one attribute of the selected preview track.
    fn track_edit_panel<F: TrackEditField>(
        &mut self,
        ui: &mut egui::Ui,
        tokens: theme::ColorTokens,
        preview: &oxidj_application::Preview,
    ) {
        let label = F::ATTRIBUTE.xml_name();
        let layout = F::LAYOUT;
        match layout.heading {
            Heading::Strong => {
                SectionHeading {
                    title: &format!("Prepare one guarded {label} edit"),
                }
                .show(ui, tokens);
            }
            Heading::Section => SectionHeading {
                title: &format!("Track {label} edit (provisional)"),
            }
            .show(ui, tokens),
        }
        let Some(track_id) = self.xml_selected_track.clone() else {
            ui.label(format!(
                "Select one track above to prepare a pending {label} edit."
            ));
            return;
        };
        let Some(track) = preview.tracks.iter().find(|t| t.id == track_id) else {
            *F::pending(self) = None;
            ui.label("Selected TrackID is no longer in the loaded preview.");
            return;
        };
        ui.label(format!("Selected TrackID: {track_id}"));
        let current = F::current(track);
        match (current, layout.absent) {
            (None, Absent::ShowEditor) => {
                ui.label(format!(
                    "Selected track has no {label} attribute; insertion is not supported."
                ));
            }
            (None, absent) => {
                if matches!(absent, Absent::ClearAndNote) {
                    *F::pending(self) = None;
                }
                ui.label(format!(
                    "Selected track has no {label} attribute; insertion is not supported."
                ));
                return;
            }
            (Some(value), _) => match layout.current {
                Current::Hidden => {}
                Current::Presence => {
                    ui.label(if value.is_empty() {
                        format!("Current {label}: present but empty")
                    } else {
                        format!("Current {label}: present")
                    });
                }
                Current::Quoted => {
                    ui.label(format!("Current {label}: {value:?}"));
                }
                Current::QuotedEmptyNote => {
                    let note = if value.is_empty() {
                        " (present but empty)"
                    } else {
                        ""
                    };
                    ui.label(format!("Current {label}: {value:?}{note}"));
                }
            },
        }
        let changed = if layout.multiline {
            ui.add(
                egui::TextEdit::multiline(F::input(self))
                    .desired_rows(4)
                    .hint_text(label),
            )
            .changed()
        } else {
            ui.horizontal(|ui| {
                ui.label(format!("Proposed {label}"));
                ui.text_edit_singleline(F::input(self)).changed()
            })
            .inner
        };
        if changed {
            *F::pending(self) = None;
        }
        if ui.button(format!("Prepare pending {label} edit")).clicked() {
            self.prepare_track_edit::<F>(&track.id, current);
        }
        let Some((source, pending_track, old, new)) = F::pending(self).as_ref().map(|p| {
            (
                p.source.display().to_string(),
                p.track_id.clone(),
                F::old(p.prepared.change()).to_owned(),
                F::new(p.prepared.change()).to_owned(),
            )
        }) else {
            return;
        };
        ui.separator();
        ui.strong(format!("Pending {label} edit · not exported or saved"));
        ui.label(format!("Source: {source}"));
        ui.label(format!("TrackID: {pending_track}"));
        ui.label(format!("Original {label}: {old:?}"));
        ui.label(format!("Proposed {label}: {new:?}"));
        ui.horizontal(|ui| {
            ui.label("New XML destination");
            ui.text_edit_singleline(F::export_path(self));
        });
        if ui.button(format!("Export prepared {label} edit")).clicked() {
            self.export_track_edit::<F>();
        }
        if ui.button(format!("Cancel pending {label} edit")).clicked() {
            self.cancel_track_edit::<F>();
        }
    }
}

/// The field table. Each row: marker type, attribute, preview field, application
/// types/handlers, UI state fields, the historical per-field method names (kept
/// as test-only thin wrappers), and the preserved panel layout.
/// Row order is panel order.
macro_rules! track_edit_fields {
    ($(
        $Marker:ident: $Variant:ident, $field:ident,
        $Change:ident { $old:ident, $new:ident },
        $Request:ident { $expected:ident, $new_req:ident },
        $prepare_api:ident, $export_api:ident,
        state($input:ident, $path:ident, $pending:ident) as $Alias:ident,
        methods($prepare_m:ident, $cancel_m:ident, $export_m:ident),
        $layout:expr;
    )*) => {
        $(
            pub(crate) struct $Marker;
            pub(crate) type $Alias = PendingTrackEdit<oxidj_application::$Change>;
            impl TrackEditField for $Marker {
                type Change = oxidj_application::$Change;
                const ATTRIBUTE: TrackAttribute = TrackAttribute::$Variant;
                const LAYOUT: Layout = $layout;
                fn current(track: &Track) -> Option<&str> {
                    track.$field.as_deref()
                }
                fn old(change: &Self::Change) -> &str {
                    &change.$old
                }
                fn new(change: &Self::Change) -> &str {
                    &change.$new
                }
                fn input(app: &mut LibraryApp) -> &mut String {
                    &mut app.$input
                }
                fn export_path(app: &mut LibraryApp) -> &mut String {
                    &mut app.$path
                }
                fn pending(app: &mut LibraryApp) -> &mut Option<$Alias> {
                    &mut app.$pending
                }
                fn prepare(
                    source: PathBuf,
                    track_id: &str,
                    old: &str,
                    new: String,
                ) -> Result<PreparedTrackEditExport<Self::Change>, RekordboxError> {
                    oxidj_application::$prepare_api(&oxidj_application::$Request {
                        source,
                        track_id: track_id.to_owned(),
                        $expected: old.to_owned(),
                        $new_req: new,
                    })
                }
                fn export(
                    prepared: &PreparedTrackEditExport<Self::Change>,
                    output: &Path,
                ) -> Result<Self::Change, RekordboxError> {
                    oxidj_application::$export_api(prepared, output)
                }
            }
        )*

        // Historical per-field entry points, kept for the acceptance tests.
        #[cfg(test)]
        impl LibraryApp {
            $(
                pub(crate) fn $prepare_m(&mut self, track_id: &str, old: Option<&str>) {
                    self.prepare_track_edit::<$Marker>(track_id, old);
                }
                pub(crate) fn $cancel_m(&mut self) {
                    self.cancel_track_edit::<$Marker>();
                }
                pub(crate) fn $export_m(&mut self) {
                    self.export_track_edit::<$Marker>();
                }
            )*
        }

        impl LibraryApp {

            /// Discard every pending single-attribute edit (Name excluded).
            pub(crate) fn discard_track_attribute_edits(&mut self) {
                $(self.$pending = None;)*
            }

            /// Load every attribute input from the selected preview track and
            /// discard pending edits (Name excluded).
            pub(crate) fn load_track_attribute_inputs(&mut self, track_id: &str) {
                let track = self
                    .xml_preview
                    .as_ref()
                    .and_then(|preview| preview.as_ref().ok())
                    .and_then(|preview| preview.tracks.iter().find(|t| t.id == track_id))
                    .cloned();
                $(
                    self.$input = track
                        .as_ref()
                        .and_then(|t| t.$field.clone())
                        .unwrap_or_default();
                )*
                self.discard_track_attribute_edits();
            }

            /// All attribute panels in table order.
            pub(crate) fn track_attribute_panels(
                &mut self,
                ui: &mut egui::Ui,
                tokens: theme::ColorTokens,
                preview: &oxidj_application::Preview,
            ) {
                $(self.track_edit_panel::<$Marker>(ui, tokens, preview);)*
            }
        }
    };
}

const STRONG_EDITOR_WHEN_ABSENT: Layout = Layout {
    heading: Heading::Strong,
    current: Current::Hidden,
    absent: Absent::ShowEditor,
    multiline: false,
};
const STRONG_PRESENCE: Layout = Layout {
    heading: Heading::Strong,
    current: Current::Presence,
    absent: Absent::Note,
    multiline: false,
};
const SECTION_PRESENCE: Layout = Layout {
    heading: Heading::Section,
    current: Current::Presence,
    absent: Absent::ClearAndNote,
    multiline: false,
};
const SECTION_QUOTED: Layout = Layout {
    current: Current::Quoted,
    ..SECTION_PRESENCE
};
const SECTION_QUOTED_EMPTY_NOTE: Layout = Layout {
    current: Current::QuotedEmptyNote,
    ..SECTION_PRESENCE
};

#[rustfmt::skip]
track_edit_fields! {
    ArtistField: Artist, artist, TrackArtistEditChange { old_artist, new_artist }, TrackArtistEditPreparationRequest { expected_old_artist, new_artist },
        prepare_rekordbox_track_artist_export, export_prepared_rekordbox_track_artist_edit,
        state(xml_edit_artist, xml_artist_export_path, xml_pending_artist_edit) as PendingTrackArtistEdit,
        methods(prepare_xml_artist_edit, cancel_xml_artist_edit, export_xml_artist_edit),
        STRONG_EDITOR_WHEN_ABSENT;
    CommentsField: Comments, comments, TrackCommentsEditChange { old_comments, new_comments }, TrackCommentsEditPreparationRequest { expected_old_comments, new_comments },
        prepare_rekordbox_track_comments_export, export_prepared_rekordbox_track_comments_edit,
        state(xml_edit_comments, xml_comments_export_path, xml_pending_comments_edit) as PendingTrackCommentsEdit,
        methods(prepare_xml_comments_edit, cancel_xml_comments_edit, export_xml_comments_edit),
        Layout { multiline: true, ..STRONG_PRESENCE };
    AlbumField: Album, album, TrackAlbumEditChange { old_album, new_album }, TrackAlbumEditPreparationRequest { expected_old_album, new_album },
        prepare_rekordbox_track_album_export, export_prepared_rekordbox_track_album_edit,
        state(xml_edit_album, xml_album_export_path, xml_pending_album_edit) as PendingTrackAlbumEdit,
        methods(prepare_xml_album_edit, cancel_xml_album_edit, export_xml_album_edit),
        STRONG_PRESENCE;
    GenreField: Genre, genre, TrackGenreEditChange { old_genre, new_genre }, TrackGenreEditPreparationRequest { expected_old_genre, new_genre },
        prepare_rekordbox_track_genre_export, export_prepared_rekordbox_track_genre_edit,
        state(xml_edit_genre, xml_genre_export_path, xml_pending_genre_edit) as PendingTrackGenreEdit,
        methods(prepare_xml_genre_edit, cancel_xml_genre_edit, export_xml_genre_edit),
        SECTION_PRESENCE;
    RemixerField: Remixer, remixer, TrackRemixerEditChange { old_remixer, new_remixer }, TrackRemixerEditPreparationRequest { expected_old_remixer, new_remixer },
        prepare_rekordbox_track_remixer_export, export_prepared_rekordbox_track_remixer_edit,
        state(xml_edit_remixer, xml_remixer_export_path, xml_pending_remixer_edit) as PendingTrackRemixerEdit,
        methods(prepare_xml_remixer_edit, cancel_xml_remixer_edit, export_xml_remixer_edit),
        SECTION_QUOTED;
    TonalityField: Tonality, tonality, TrackTonalityEditChange { old_tonality, new_tonality }, TrackTonalityEditPreparationRequest { expected_old_tonality, new_tonality },
        prepare_rekordbox_track_tonality_export, export_prepared_rekordbox_track_tonality_edit,
        state(xml_edit_tonality, xml_tonality_export_path, xml_pending_tonality_edit) as PendingTrackTonalityEdit,
        methods(prepare_xml_tonality_edit, cancel_xml_tonality_edit, export_xml_tonality_edit),
        SECTION_QUOTED_EMPTY_NOTE;
    KindField: Kind, kind, TrackKindEditChange { old_kind, new_kind }, TrackKindEditPreparationRequest { expected_old_kind, new_kind },
        prepare_rekordbox_track_kind_export, export_prepared_rekordbox_track_kind_edit,
        state(xml_edit_kind, xml_kind_export_path, xml_pending_kind_edit) as PendingTrackKindEdit,
        methods(prepare_xml_kind_edit, cancel_xml_kind_edit, export_xml_kind_edit),
        SECTION_QUOTED_EMPTY_NOTE;
    MixField: Mix, mix, TrackMixEditChange { old_mix, new_mix }, TrackMixEditPreparationRequest { expected_old_mix, new_mix },
        prepare_rekordbox_track_mix_export, export_prepared_rekordbox_track_mix_edit,
        state(xml_edit_mix, xml_mix_export_path, xml_pending_mix_edit) as PendingTrackMixEdit,
        methods(prepare_xml_mix_edit, cancel_xml_mix_edit, export_xml_mix_edit),
        SECTION_PRESENCE;
    ComposerField: Composer, composer, TrackComposerEditChange { old_composer, new_composer }, TrackComposerEditPreparationRequest { expected_old_composer, new_composer },
        prepare_rekordbox_track_composer_export, export_prepared_rekordbox_track_composer_edit,
        state(xml_edit_composer, xml_composer_export_path, xml_pending_composer_edit) as PendingTrackComposerEdit,
        methods(prepare_xml_composer_edit, cancel_xml_composer_edit, export_xml_composer_edit),
        SECTION_PRESENCE;
    GroupingField: Grouping, grouping, TrackGroupingEditChange { old_grouping, new_grouping }, TrackGroupingEditPreparationRequest { expected_old_grouping, new_grouping },
        prepare_rekordbox_track_grouping_export, export_prepared_rekordbox_track_grouping_edit,
        state(xml_edit_grouping, xml_grouping_export_path, xml_pending_grouping_edit) as PendingTrackGroupingEdit,
        methods(prepare_xml_grouping_edit, cancel_xml_grouping_edit, export_xml_grouping_edit),
        SECTION_PRESENCE;
    LabelField: Label, label, TrackLabelEditChange { old_label, new_label }, TrackLabelEditPreparationRequest { expected_old_label, new_label },
        prepare_rekordbox_track_label_export, export_prepared_rekordbox_track_label_edit,
        state(xml_edit_label, xml_label_export_path, xml_pending_label_edit) as PendingTrackLabelEdit,
        methods(prepare_xml_label_edit, cancel_xml_label_edit, export_xml_label_edit),
        SECTION_PRESENCE;
}

/// The original Track Name edit keeps its own contract: no selection/proposal
/// export guard and no re-open of the output (its tests pin this behaviour).
pub(crate) type PendingTrackNameEdit = PendingTrackEdit<oxidj_application::TrackNameEditChange>;

impl LibraryApp {
    pub(crate) fn prepare_xml_name_edit(&mut self, track_id: &str, old_name: &str) {
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

    pub(crate) fn cancel_xml_name_edit(&mut self) {
        self.xml_pending_edit = None;
        self.xml_edit_error = None;
    }

    pub(crate) fn export_xml_name_edit(&mut self) {
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
}
