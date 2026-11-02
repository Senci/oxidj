//! Guarded TRACK attribute, tempo, and position-mark edits.
use super::*;

/// One optimistic, ordinal-scoped tempo edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_bpm: String,
    pub new_bpm: String,
}
/// One optimistic, ordinal-scoped tempo-point start edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoStartEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_start_seconds: String,
    pub new_start_seconds: String,
}
/// One optimistic, ordinal-scoped tempo-point Battito edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoBeatEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_beat: String,
    pub new_beat: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoBeatEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_beat: String,
    pub new_beat: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoBeatEdit {
    pub bytes: Vec<u8>,
    pub change: TempoBeatEditChange,
}
/// One optimistic, ordinal-scoped raw tempo-point Metro edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMeterEditRequest {
    pub track_id: String,
    pub tempo_index: usize,
    pub expected_old_meter: String,
    pub new_meter: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMeterEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_meter: String,
    pub new_meter: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoMeterEdit {
    pub bytes: Vec<u8>,
    pub change: TempoMeterEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoStartEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_start_seconds: String,
    pub new_start_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoStartEdit {
    pub bytes: Vec<u8>,
    pub change: TempoStartEditChange,
}
/// The exact change prepared for a source track's tempo point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoEditChange {
    pub track_id: String,
    pub tempo_index: usize,
    pub old_bpm: String,
    pub new_bpm: String,
}
/// Validated, transformed XML bytes plus the proposed change summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTempoEdit {
    pub bytes: Vec<u8>,
    pub change: TempoEditChange,
}
/// One optimistic, ordinal-scoped position mark start edit request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_start_seconds: String,
    pub new_start_seconds: String,
}
/// The exact change prepared for a source track's position mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_start_seconds: String,
    pub new_start_seconds: String,
}
/// Validated, transformed XML bytes plus the proposed change summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkEditChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkNameEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkNameEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_name: String,
    pub new_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkNameEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkNameEditChange,
}
/// Exact raw-field guard for deleting one position mark by track-local ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkDeleteRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_type: String,
    pub expected_start: String,
    pub expected_name: String,
    pub expected_end: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkDeleteChange {
    pub track_id: String,
    pub mark_index: usize,
    pub deleted_type: String,
    pub deleted_start: String,
    pub deleted_name: String,
    pub deleted_end: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkDelete {
    pub bytes: Vec<u8>,
    pub change: PositionMarkDeleteChange,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEndEditRequest {
    pub track_id: String,
    pub mark_index: usize,
    pub expected_old_end_seconds: String,
    pub new_end_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMarkEndEditChange {
    pub track_id: String,
    pub mark_index: usize,
    pub old_end_seconds: String,
    pub new_end_seconds: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPositionMarkEndEdit {
    pub bytes: Vec<u8>,
    pub change: PositionMarkEndEditChange,
}

pub(crate) fn track_attribute_range(
    bytes: &[u8],
    id: &str,
    attribute: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut result = None;
    loop {
        let start = reader.buffer_position() as usize;
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?
        {
            Event::Start(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|p| p == "COLLECTION") =>
            {
                if attr(&e, "TrackID")? == id {
                    result = Some(attribute_value_range(&e, start, attribute, expected)?);
                }
                stack.push("TRACK".into());
            }
            Event::Empty(e)
                if e.name().as_ref() == b"TRACK"
                    && stack.last().is_some_and(|p| p == "COLLECTION") =>
            {
                if attr(&e, "TrackID")? == id {
                    result = Some(attribute_value_range(&e, start, attribute, expected)?);
                }
            }
            Event::Start(e) => stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    result.ok_or_else(|| Error(format!("Track {attribute} location missing")))
}

/// Value access for a TRACK field: `Name` is always present, the rest are optional.
trait TrackFieldValue {
    fn value(&self) -> Option<&str>;
    fn put(&mut self, value: String);
}
impl TrackFieldValue for String {
    fn value(&self) -> Option<&str> {
        Some(self)
    }
    fn put(&mut self, value: String) {
        *self = value;
    }
}
impl TrackFieldValue for Option<String> {
    fn value(&self) -> Option<&str> {
        self.as_deref()
    }
    fn put(&mut self, value: String) {
        *self = Some(value);
    }
}

/// Table of guarded single-attribute TRACK edits. Each row is one field descriptor
/// (variant, XML attribute name, `Track` field) plus its per-field public API names,
/// which are thin wrappers over [`prepare_track_attribute_edit`].
macro_rules! track_attribute_table {
    ($($variant:ident => $xml:literal, $field:ident: $prepare:ident, $Request:ident { $expected:ident, $new:ident }, $Change:ident { $old:ident }, $Prepared:ident;)*) => {
        /// A single-valued TRACK attribute editable with an exact-old-value guard.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum TrackAttribute {
            $($variant,)*
        }
        impl TrackAttribute {
            /// The Rekordbox XML attribute name.
            pub fn xml_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $xml,)*
                }
            }
            fn value(self, track: &Track) -> Option<&str> {
                match self {
                    $(Self::$variant => track.$field.value(),)*
                }
            }
            fn put(self, track: &mut Track, value: String) {
                match self {
                    $(Self::$variant => track.$field.put(value),)*
                }
            }
        }
        $(
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $Request {
                pub track_id: String,
                pub $expected: String,
                pub $new: String,
            }
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $Change {
                pub track_id: String,
                pub $old: String,
                pub $new: String,
            }
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $Prepared {
                pub bytes: Vec<u8>,
                pub change: $Change,
            }
            #[doc = concat!("Prepare one exact-old-value guarded raw Track ", $xml, " edit.")]
            pub fn $prepare(bytes: &[u8], request: &$Request) -> Result<$Prepared, Error> {
                let prepared = prepare_track_attribute_edit(
                    bytes,
                    &TrackAttributeEditRequest {
                        track_id: request.track_id.clone(),
                        attribute: TrackAttribute::$variant,
                        expected_old: request.$expected.clone(),
                        new: request.$new.clone(),
                    },
                )?;
                Ok($Prepared {
                    bytes: prepared.bytes,
                    change: $Change {
                        track_id: prepared.change.track_id,
                        $old: prepared.change.old,
                        $new: prepared.change.new,
                    },
                })
            }
        )*
    };
}

#[rustfmt::skip]
track_attribute_table! {
    Name => "Name", name: prepare_track_name_edit, TrackNameEditRequest { expected_old_name, new_name }, TrackNameEditChange { old_name }, PreparedTrackNameEdit;
    Comments => "Comments", comments: prepare_track_comments_edit, TrackCommentsEditRequest { expected_old_comments, new_comments }, TrackCommentsEditChange { old_comments }, PreparedTrackCommentsEdit;
    Album => "Album", album: prepare_track_album_edit, TrackAlbumEditRequest { expected_old_album, new_album }, TrackAlbumEditChange { old_album }, PreparedTrackAlbumEdit;
    Genre => "Genre", genre: prepare_track_genre_edit, TrackGenreEditRequest { expected_old_genre, new_genre }, TrackGenreEditChange { old_genre }, PreparedTrackGenreEdit;
    Label => "Label", label: prepare_track_label_edit, TrackLabelEditRequest { expected_old_label, new_label }, TrackLabelEditChange { old_label }, PreparedTrackLabelEdit;
    Rating => "Rating", rating: prepare_track_rating_edit, TrackRatingEditRequest { expected_old_rating, new_rating }, TrackRatingEditChange { old_rating }, PreparedTrackRatingEdit;
    Colour => "Colour", colour: prepare_track_colour_edit, TrackColourEditRequest { expected_old_colour, new_colour }, TrackColourEditChange { old_colour }, PreparedTrackColourEdit;
    Composer => "Composer", composer: prepare_track_composer_edit, TrackComposerEditRequest { expected_old_composer, new_composer }, TrackComposerEditChange { old_composer }, PreparedTrackComposerEdit;
    Grouping => "Grouping", grouping: prepare_track_grouping_edit, TrackGroupingEditRequest { expected_old_grouping, new_grouping }, TrackGroupingEditChange { old_grouping }, PreparedTrackGroupingEdit;
    Remixer => "Remixer", remixer: prepare_track_remixer_edit, TrackRemixerEditRequest { expected_old_remixer, new_remixer }, TrackRemixerEditChange { old_remixer }, PreparedTrackRemixerEdit;
    Mix => "Mix", mix: prepare_track_mix_edit, TrackMixEditRequest { expected_old_mix, new_mix }, TrackMixEditChange { old_mix }, PreparedTrackMixEdit;
    Tonality => "Tonality", tonality: prepare_track_tonality_edit, TrackTonalityEditRequest { expected_old_tonality, new_tonality }, TrackTonalityEditChange { old_tonality }, PreparedTrackTonalityEdit;
    Year => "Year", year: prepare_track_year_edit, TrackYearEditRequest { expected_old_year, new_year }, TrackYearEditChange { old_year }, PreparedTrackYearEdit;
    DateAdded => "DateAdded", date_added: prepare_track_date_added_edit, TrackDateAddedEditRequest { expected_old_date_added, new_date_added }, TrackDateAddedEditChange { old_date_added }, PreparedTrackDateAddedEdit;
    DateModified => "DateModified", date_modified: prepare_track_date_modified_edit, TrackDateModifiedEditRequest { expected_old_date_modified, new_date_modified }, TrackDateModifiedEditChange { old_date_modified }, PreparedTrackDateModifiedEdit;
    LastPlayed => "LastPlayed", last_played: prepare_track_last_played_edit, TrackLastPlayedEditRequest { expected_old_last_played, new_last_played }, TrackLastPlayedEditChange { old_last_played }, PreparedTrackLastPlayedEdit;
    PlayCount => "PlayCount", play_count: prepare_track_play_count_edit, TrackPlayCountEditRequest { expected_old_play_count, new_play_count }, TrackPlayCountEditChange { old_play_count }, PreparedTrackPlayCountEdit;
    AverageBpm => "AverageBpm", average_bpm: prepare_track_average_bpm_edit, TrackAverageBpmEditRequest { expected_old_average_bpm, new_average_bpm }, TrackAverageBpmEditChange { old_average_bpm }, PreparedTrackAverageBpmEdit;
    DiscNumber => "DiscNumber", disc_number: prepare_track_disc_number_edit, TrackDiscNumberEditRequest { expected_old_disc_number, new_disc_number }, TrackDiscNumberEditChange { old_disc_number }, PreparedTrackDiscNumberEdit;
    TrackNumber => "TrackNumber", track_number: prepare_track_number_edit, TrackNumberEditRequest { expected_old_track_number, new_track_number }, TrackNumberEditChange { old_track_number }, PreparedTrackNumberEdit;
    Size => "Size", size: prepare_track_size_edit, TrackSizeEditRequest { expected_old_size, new_size }, TrackSizeEditChange { old_size }, PreparedTrackSizeEdit;
    TotalTime => "TotalTime", total_time: prepare_track_total_time_edit, TrackTotalTimeEditRequest { expected_old_total_time, new_total_time }, TrackTotalTimeEditChange { old_total_time }, PreparedTrackTotalTimeEdit;
    BitRate => "BitRate", bit_rate: prepare_track_bit_rate_edit, TrackBitRateEditRequest { expected_old_bit_rate, new_bit_rate }, TrackBitRateEditChange { old_bit_rate }, PreparedTrackBitRateEdit;
    SampleRate => "SampleRate", sample_rate: prepare_track_sample_rate_edit, TrackSampleRateEditRequest { expected_old_sample_rate, new_sample_rate }, TrackSampleRateEditChange { old_sample_rate }, PreparedTrackSampleRateEdit;
    Kind => "Kind", kind: prepare_track_kind_edit, TrackKindEditRequest { expected_old_kind, new_kind }, TrackKindEditChange { old_kind }, PreparedTrackKindEdit;
    Artist => "Artist", artist: prepare_track_artist_edit, TrackArtistEditRequest { expected_old_artist, new_artist }, TrackArtistEditChange { old_artist }, PreparedTrackArtistEdit;
}

/// Exact-old guarded request to replace one TRACK attribute value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackAttributeEditRequest {
    pub track_id: String,
    pub attribute: TrackAttribute,
    pub expected_old: String,
    pub new: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackAttributeEditChange {
    pub track_id: String,
    pub attribute: TrackAttribute,
    pub old: String,
    pub new: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrackAttributeEdit {
    pub bytes: Vec<u8>,
    pub change: TrackAttributeEditChange,
}

/// Prepare one exact-old-value guarded TRACK attribute edit with byte-preserving output.
///
/// The attribute must be present (absent attributes are refused, never inserted), the new
/// value must be XML 1.0 text, and the re-parsed output must differ from the source only in
/// that attribute. `Name` keeps its historical escaping (TAB/LF/CR are not character-encoded).
pub fn prepare_track_attribute_edit(
    bytes: &[u8],
    request: &TrackAttributeEditRequest,
) -> Result<PreparedTrackAttributeEdit, Error> {
    let attribute = request.attribute;
    let xml = attribute.xml_name();
    let before = preflight_export(bytes)?;
    let mut matches = before
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id);
    let track = match (matches.next(), matches.next()) {
        (None, _) => return Err(Error(format!("unknown TrackID {}", request.track_id))),
        (Some(track), None) => track,
        _ => return Err(Error(format!("duplicate TrackID {}", request.track_id))),
    };
    let old = attribute
        .value(track)
        .ok_or_else(|| Error(format!("track {xml} missing")))?;
    if old != request.expected_old {
        return Err(Error(format!("track {xml} changed")));
    }
    if !valid_xml_text(&request.new) {
        return Err(Error(format!("new {xml} contains invalid XML 1.0 text")));
    }
    let range = track_attribute_range(bytes, &request.track_id, xml, &request.expected_old)?;
    let escaped = escape_attribute(&request.new, attribute != TrackAttribute::Name);
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let after = preflight_export(&output)?;
    let mut expected = before.selected_tracks.clone();
    let target = expected
        .iter_mut()
        .find(|t| t.id == request.track_id)
        .unwrap();
    attribute.put(target, request.new.clone());
    if after.selected_tracks != expected {
        return Err(Error(format!("track {xml} edit verification failed")));
    }
    Ok(PreparedTrackAttributeEdit {
        bytes: output,
        change: TrackAttributeEditChange {
            track_id: request.track_id.clone(),
            attribute,
            old: old.to_owned(),
            new: request.new.clone(),
        },
    })
}

pub(crate) fn tempo_bpm_range(
    bytes: &[u8],
    request: &TempoEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut tempo_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    tempo_index = 0;
                } else if name == "TEMPO" && track_id.as_deref() == Some(&request.track_id) {
                    if tempo_index == request.tempo_index {
                        value_range = Some(bpm_attribute_range(
                            &element,
                            event_start,
                            &request.expected_old_bpm,
                        )?);
                    }
                    tempo_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TEMPO" && track_id.as_deref() == Some(&request.track_id) {
                    if tempo_index == request.tempo_index {
                        value_range = Some(bpm_attribute_range(
                            &element,
                            event_start,
                            &request.expected_old_bpm,
                        )?);
                    }
                    tempo_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })
}

pub(crate) fn tempo_attribute_range(
    bytes: &[u8],
    expected_track_id: &str,
    expected_tempo_index: usize,
    key: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_track_id: Option<String> = None;
    let mut current_tempo_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    current_track_id = Some(attr(&element, "TrackID")?);
                    current_tempo_index = 0;
                } else if name == "TEMPO" && current_track_id.as_deref() == Some(expected_track_id)
                {
                    if current_tempo_index == expected_tempo_index {
                        value_range = Some(tempo_attribute_value_range(
                            &element,
                            event_start,
                            key,
                            expected,
                        )?);
                    }
                    current_tempo_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TEMPO" && current_track_id.as_deref() == Some(expected_track_id) {
                    if current_tempo_index == expected_tempo_index {
                        value_range = Some(tempo_attribute_value_range(
                            &element,
                            event_start,
                            key,
                            expected,
                        )?);
                    }
                    current_tempo_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "TRACK") {
                    current_track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no tempo point {expected_tempo_index} for TrackID {expected_track_id}"
        ))
    })
}

pub(crate) fn tempo_attribute_value_range(
    element: &quick_xml::events::BytesStart<'_>,
    event_start: usize,
    key: &str,
    expected: &str,
) -> Result<std::ops::Range<usize>, Error> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error(error.to_string()))?;
        if attribute.key.as_ref() == key.as_bytes() {
            let actual = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, element.decoder())
                .map_err(|error| Error(error.to_string()))?;
            if actual != expected {
                return Err(Error(format!(
                    "tempo {key} changed: expected {expected}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error(format!("TEMPO missing {key} attribute")))
}

pub(crate) fn tempo_start_range(
    bytes: &[u8],
    request: &TempoStartEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Inizio",
        &request.expected_old_start_seconds,
    )
}

pub(crate) fn tempo_beat_range(
    bytes: &[u8],
    request: &TempoBeatEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Battito",
        &request.expected_old_beat,
    )
}

pub(crate) fn position_mark_start_range(
    bytes: &[u8],
    request: &PositionMarkEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut mark_index = 0usize;
    let mut value_range = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|error| Error(format!("malformed XML: {error}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    mark_index = 0;
                } else if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id)
                {
                    if mark_index == request.mark_index {
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Start",
                            &request.expected_old_start_seconds,
                        )?);
                    }
                    mark_index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id) {
                    if mark_index == request.mark_index {
                        value_range = Some(attribute_value_range(
                            &element,
                            event_start,
                            "Start",
                            &request.expected_old_start_seconds,
                        )?);
                    }
                    mark_index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|open| open == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    value_range.ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })
}

pub(crate) fn position_mark_end_range(
    bytes: &[u8],
    request: &PositionMarkEndEditRequest,
) -> Result<std::ops::Range<usize>, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut track_id: Option<String> = None;
    let mut index = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        match event {
            Event::Start(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|parent| parent == "COLLECTION") {
                    track_id = Some(attr(&element, "TrackID")?);
                    index = 0;
                } else if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id)
                {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &element,
                            start,
                            "End",
                            &request.expected_old_end_seconds,
                        )?);
                    }
                    index += 1;
                }
                stack.push(name);
            }
            Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "POSITION_MARK" && track_id.as_deref() == Some(&request.track_id) {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &element,
                            start,
                            "End",
                            &request.expected_old_end_seconds,
                        )?);
                    }
                    index += 1;
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.name().as_ref()).into_owned();
                if name == "TRACK" && stack.last().is_some_and(|open| open == "TRACK") {
                    track_id = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    range.ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })
}

/// Prepare one position mark Start edit while preserving every other source byte.
pub fn prepare_position_mark_edit(
    bytes: &[u8],
    request: &PositionMarkEditRequest,
) -> Result<PreparedPositionMarkEdit, Error> {
    let source = preflight_export(bytes)?;
    let source_track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = source_track.marks.get(request.mark_index).ok_or_else(|| {
        Error(format!(
            "no position mark {} for TrackID {}",
            request.mark_index, request.track_id
        ))
    })?;
    if mark.start_seconds != request.expected_old_start_seconds {
        return Err(Error(format!(
            "position mark Start changed: expected {}, found {}",
            request.expected_old_start_seconds, mark.start_seconds
        )));
    }
    validate_decimal("new position mark Start", &request.new_start_seconds, false)?;
    if let Some(end) = &mark.end_seconds {
        let new_start = request
            .new_start_seconds
            .parse::<f64>()
            .map_err(|_| Error("invalid new position mark Start".into()))?;
        let end = end
            .parse::<f64>()
            .map_err(|_| Error("invalid position mark End".into()))?;
        if new_start >= end {
            return Err(Error(
                "edited position mark Start must be before End".into(),
            ));
        }
    }

    let range = position_mark_start_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_start_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = source_track.clone();
    expected_track.marks[request.mark_index].start_seconds = request.new_start_seconds.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited position mark verification failed".into()));
    }
    Ok(PreparedPositionMarkEdit {
        bytes: output,
        change: PositionMarkEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_start_seconds: request.expected_old_start_seconds.clone(),
            new_start_seconds: request.new_start_seconds.clone(),
        },
    })
}

pub fn prepare_position_mark_end_edit(
    bytes: &[u8],
    request: &PositionMarkEndEditRequest,
) -> Result<PreparedPositionMarkEndEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = track
        .marks
        .get(request.mark_index)
        .ok_or_else(|| Error("position mark missing".into()))?;
    let old = mark
        .end_seconds
        .as_ref()
        .ok_or_else(|| Error("POSITION_MARK missing End attribute".into()))?;
    if old != &request.expected_old_end_seconds {
        return Err(Error(format!(
            "position mark End changed: expected {}, found {old}",
            request.expected_old_end_seconds
        )));
    }
    validate_decimal("new position mark End", &request.new_end_seconds, false)?;
    let new_end = request
        .new_end_seconds
        .parse::<f64>()
        .map_err(|_| Error("invalid new position mark End".into()))?;
    let start = mark
        .start_seconds
        .parse::<f64>()
        .map_err(|_| Error("invalid position mark Start".into()))?;
    if new_end <= start {
        return Err(Error("edited position mark End must be after Start".into()));
    }
    let range = position_mark_end_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_end_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected = track.clone();
    expected.marks[request.mark_index].end_seconds = Some(request.new_end_seconds.clone());
    let actual = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual != &expected {
        return Err(Error("edited position mark verification failed".into()));
    }
    Ok(PreparedPositionMarkEndEdit {
        bytes: output,
        change: PositionMarkEndEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_end_seconds: request.expected_old_end_seconds.clone(),
            new_end_seconds: request.new_end_seconds.clone(),
        },
    })
}

pub fn prepare_position_mark_name_edit(
    bytes: &[u8],
    request: &PositionMarkNameEditRequest,
) -> Result<PreparedPositionMarkNameEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let mark = track
        .marks
        .get(request.mark_index)
        .ok_or_else(|| Error("position mark missing".into()))?;
    if mark.name != request.expected_old_name {
        return Err(Error("position mark Name changed".into()));
    }
    if request.new_name.is_empty() || !valid_xml_text(&request.new_name) {
        return Err(Error("invalid position mark Name".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut tid: Option<String> = None;
    let mut index = 0usize;
    let mut range = None;
    loop {
        let start = reader.buffer_position() as usize;
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(e.to_string()))?
        {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if tag == "TRACK" && stack.last().is_some_and(|x| x == "COLLECTION") {
                    tid = Some(attr(&e, "TrackID")?);
                    index = 0;
                } else if tag == "POSITION_MARK" && tid.as_deref() == Some(&request.track_id) {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    index += 1;
                }
                stack.push(tag);
            }
            Event::Empty(e) => {
                if e.name().as_ref() == b"POSITION_MARK"
                    && tid.as_deref() == Some(&request.track_id)
                {
                    if index == request.mark_index {
                        range = Some(attribute_value_range(
                            &e,
                            start,
                            "Name",
                            &request.expected_old_name,
                        )?);
                    }
                    index += 1;
                }
            }
            Event::End(e) => {
                if e.name().as_ref() == b"TRACK" && stack.last().is_some_and(|x| x == "TRACK") {
                    tid = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let range = range.ok_or_else(|| Error("position mark missing".into()))?;
    let escaped = escape_attribute(&request.new_name, false);
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let verified = preflight_export(&output)?;
    let actual = verified
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .ok_or_else(|| Error("edited track missing".into()))?;
    let mut expected = track.clone();
    expected.marks[request.mark_index].name = request.new_name.clone();
    if actual != &expected {
        return Err(Error("edited Name verification failed".into()));
    }
    Ok(PreparedPositionMarkNameEdit {
        bytes: output,
        change: PositionMarkNameEditChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            old_name: request.expected_old_name.clone(),
            new_name: request.new_name.clone(),
        },
    })
}

pub fn prepare_position_mark_delete(
    bytes: &[u8],
    request: &PositionMarkDeleteRequest,
) -> Result<PreparedPositionMarkDelete, Error> {
    let source = preflight_export(bytes)?;
    let mut tracks = source
        .selected_tracks
        .iter()
        .filter(|t| t.id == request.track_id);
    let track = tracks
        .next()
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    if tracks.next().is_some() {
        return Err(Error(format!("duplicate TrackID {}", request.track_id)));
    }
    let mark = track
        .marks
        .get(request.mark_index)
        .ok_or_else(|| Error("position mark missing".into()))?;
    if mark.kind != request.expected_type
        || mark.start_seconds != request.expected_start
        || mark.name != request.expected_name
        || mark.end_seconds != request.expected_end
    {
        return Err(Error("position mark guard changed".into()));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut tid: Option<String> = None;
    let mut index = 0usize;
    let mut range = None;
    let mut removal_start = 0usize;
    let mut remove_depth = None;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error(format!("malformed XML: {e}")))?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if tag == "TRACK" && stack.last().is_some_and(|x| x == "COLLECTION") {
                    tid = Some(attr(&e, "TrackID")?);
                    index = 0;
                } else if tag == "POSITION_MARK" && tid.as_deref() == Some(&request.track_id) {
                    if index == request.mark_index {
                        let actual = (
                            attr(&e, "Type")?,
                            attr(&e, "Start")?,
                            attr(&e, "Name")?,
                            optional(&e, "End")?,
                        );
                        if actual
                            != (
                                request.expected_type.clone(),
                                request.expected_start.clone(),
                                request.expected_name.clone(),
                                request.expected_end.clone(),
                            )
                        {
                            return Err(Error("position mark raw-field guard changed".into()));
                        }
                        removal_start = start;
                        remove_depth = Some(stack.len());
                    }
                    index += 1;
                }
                stack.push(tag);
            }
            Event::Empty(e) => {
                if e.name().as_ref() == b"POSITION_MARK"
                    && tid.as_deref() == Some(&request.track_id)
                {
                    if index == request.mark_index {
                        let actual = (
                            attr(&e, "Type")?,
                            attr(&e, "Start")?,
                            attr(&e, "Name")?,
                            optional(&e, "End")?,
                        );
                        if actual
                            != (
                                request.expected_type.clone(),
                                request.expected_start.clone(),
                                request.expected_name.clone(),
                                request.expected_end.clone(),
                            )
                        {
                            return Err(Error("position mark raw-field guard changed".into()));
                        }
                        range = Some(start..end);
                    }
                    index += 1;
                }
            }
            Event::End(e) => {
                if let Some(depth) = remove_depth
                    && stack.len() == depth + 1
                    && e.name().as_ref() == b"POSITION_MARK"
                {
                    range = Some(removal_start..end);
                    remove_depth = None;
                }
                if e.name().as_ref() == b"TRACK" && stack.last().is_some_and(|x| x == "TRACK") {
                    tid = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let range = range.ok_or_else(|| Error("position mark missing".into()))?;
    let mut output = bytes.to_vec();
    output.drain(range);
    let verified = preflight_export(&output)?;
    if verified
        .selected_tracks
        .iter()
        .find(|t| t.id == request.track_id)
        .is_none_or(|actual| {
            let mut expected = track.clone();
            expected.marks.remove(request.mark_index);
            actual != &expected
        })
    {
        return Err(Error("position mark deletion verification failed".into()));
    }
    Ok(PreparedPositionMarkDelete {
        bytes: output,
        change: PositionMarkDeleteChange {
            track_id: request.track_id.clone(),
            mark_index: request.mark_index,
            deleted_type: request.expected_type.clone(),
            deleted_start: request.expected_start.clone(),
            deleted_name: request.expected_name.clone(),
            deleted_end: request.expected_end.clone(),
        },
    })
}

pub(crate) fn bpm_attribute_range(
    element: &quick_xml::events::BytesStart<'_>,
    event_start: usize,
    expected_old_bpm: &str,
) -> Result<std::ops::Range<usize>, Error> {
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| Error(error.to_string()))?;
        if attribute.key.as_ref() == b"Bpm" {
            let actual = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, element.decoder())
                .map_err(|error| Error(error.to_string()))?;
            if actual != expected_old_bpm {
                return Err(Error(format!(
                    "tempo BPM changed: expected {expected_old_bpm}, found {actual}"
                )));
            }
            let event_bytes = element.as_ref();
            let value_offset = attribute.value.as_ptr() as usize - event_bytes.as_ptr() as usize;
            let start = event_start + 1 + value_offset;
            return Ok(start..start + attribute.value.len());
        }
    }
    Err(Error("TEMPO missing Bpm attribute".into()))
}

/// Prepare a single BPM edit while preserving every other source byte.
pub fn prepare_tempo_edit(
    bytes: &[u8],
    request: &TempoEditRequest,
) -> Result<PreparedTempoEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.bpm != request.expected_old_bpm {
        return Err(Error(format!(
            "tempo BPM changed: expected {}, found {}",
            request.expected_old_bpm, tempo.bpm
        )));
    }
    validate_decimal("new BPM", &request.new_bpm, true)?;
    let range = tempo_bpm_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_bpm.bytes());
    let verified = preflight_export(&output)?;
    let actual = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .and_then(|track| track.tempos.get(request.tempo_index))
        .ok_or_else(|| Error("edited tempo point missing after validation".into()))?;
    if actual.bpm != request.new_bpm {
        return Err(Error(format!(
            "edited BPM verification failed: expected {}, found {}",
            request.new_bpm, actual.bpm
        )));
    }
    Ok(PreparedTempoEdit {
        bytes: output,
        change: TempoEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_bpm: request.expected_old_bpm.clone(),
            new_bpm: request.new_bpm.clone(),
        },
    })
}

/// Prepare a single tempo-point Inizio edit while preserving every other source byte.
pub fn prepare_tempo_start_edit(
    bytes: &[u8],
    request: &TempoStartEditRequest,
) -> Result<PreparedTempoStartEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.start_seconds != request.expected_old_start_seconds {
        return Err(Error(format!(
            "tempo Inizio changed: expected {}, found {}",
            request.expected_old_start_seconds, tempo.start_seconds
        )));
    }
    validate_decimal("new tempo Inizio", &request.new_start_seconds, false)?;
    let range = tempo_start_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_start_seconds.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].start_seconds = request.new_start_seconds.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Inizio verification failed".into()));
    }
    Ok(PreparedTempoStartEdit {
        bytes: output,
        change: TempoStartEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_start_seconds: request.expected_old_start_seconds.clone(),
            new_start_seconds: request.new_start_seconds.clone(),
        },
    })
}

/// Prepare a single tempo-point Battito edit while preserving every other source byte.
pub fn prepare_tempo_beat_edit(
    bytes: &[u8],
    request: &TempoBeatEditRequest,
) -> Result<PreparedTempoBeatEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.beat != request.expected_old_beat {
        return Err(Error(format!(
            "tempo Battito changed: expected {}, found {}",
            request.expected_old_beat, tempo.beat
        )));
    }
    validate_tempo_beat(&tempo.meter, &request.new_beat)
        .map_err(|_| Error(format!("invalid tempo Battito {}", request.new_beat)))?;
    let range = tempo_beat_range(bytes, request)?;
    let mut output = bytes.to_vec();
    output.splice(range, request.new_beat.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].beat = request.new_beat.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Battito verification failed".into()));
    }
    Ok(PreparedTempoBeatEdit {
        bytes: output,
        change: TempoBeatEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_beat: request.expected_old_beat.clone(),
            new_beat: request.new_beat.clone(),
        },
    })
}

/// Prepare a single raw tempo-point Metro edit while preserving every other source byte.
pub fn prepare_tempo_meter_edit(
    bytes: &[u8],
    request: &TempoMeterEditRequest,
) -> Result<PreparedTempoMeterEdit, Error> {
    let source = preflight_export(bytes)?;
    let track = source
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error(format!("unknown TrackID {}", request.track_id)))?;
    let tempo = track.tempos.get(request.tempo_index).ok_or_else(|| {
        Error(format!(
            "no tempo point {} for TrackID {}",
            request.tempo_index, request.track_id
        ))
    })?;
    if tempo.meter != request.expected_old_meter {
        return Err(Error(format!(
            "tempo Metro changed: expected {}, found {}",
            request.expected_old_meter, tempo.meter
        )));
    }
    if request.new_meter.is_empty() || !valid_xml_text(&request.new_meter) {
        return Err(Error("invalid tempo Metro XML text".into()));
    }
    let range = tempo_attribute_range(
        bytes,
        &request.track_id,
        request.tempo_index,
        "Metro",
        &request.expected_old_meter,
    )?;
    let escaped = escape_attribute(&request.new_meter, false);
    let mut output = bytes.to_vec();
    output.splice(range, escaped.bytes());
    let verified = preflight_export(&output)?;
    let mut expected_track = track.clone();
    expected_track.tempos[request.tempo_index].meter = request.new_meter.clone();
    let actual_track = verified
        .selected_tracks
        .iter()
        .find(|track| track.id == request.track_id)
        .ok_or_else(|| Error("edited track missing after validation".into()))?;
    if actual_track != &expected_track {
        return Err(Error("edited tempo Metro verification failed".into()));
    }
    Ok(PreparedTempoMeterEdit {
        bytes: output,
        change: TempoMeterEditChange {
            track_id: request.track_id.clone(),
            tempo_index: request.tempo_index,
            old_meter: request.expected_old_meter.clone(),
            new_meter: request.new_meter.clone(),
        },
    })
}
