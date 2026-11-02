//! F-09 independent post-edit oracles for guarded F-02 TRACK attribute edits.
//!
//! One table row per attribute: frozen synthetic source fixture, expected old and
//! new raw values, and the existing guarded per-attribute adapter. Each row keeps
//! its own named test: independent test-only quick-xml decode of the frozen
//! source with only TrackID 100's attribute changed, a whole-tree comparison
//! with the actual adapter output, stale/wrong-target guard rejection, and
//! semantic mutation rejection. Never the production importer.
use oxidj_rekordbox::{
    TrackAlbumEditRequest, TrackArtistEditRequest, TrackAverageBpmEditRequest,
    TrackBitRateEditRequest, TrackColourEditRequest, TrackCommentsEditRequest,
    TrackComposerEditRequest, TrackDateAddedEditRequest, TrackDateModifiedEditRequest,
    TrackDiscNumberEditRequest, TrackGenreEditRequest, TrackGroupingEditRequest,
    TrackKindEditRequest, TrackLabelEditRequest, TrackLastPlayedEditRequest, TrackMixEditRequest,
    TrackNumberEditRequest, TrackPlayCountEditRequest, TrackRatingEditRequest,
    TrackRemixerEditRequest, TrackSampleRateEditRequest, TrackSizeEditRequest,
    TrackTonalityEditRequest, TrackTotalTimeEditRequest, TrackYearEditRequest,
    prepare_track_album_edit, prepare_track_artist_edit, prepare_track_average_bpm_edit,
    prepare_track_bit_rate_edit, prepare_track_colour_edit, prepare_track_comments_edit,
    prepare_track_composer_edit, prepare_track_date_added_edit, prepare_track_date_modified_edit,
    prepare_track_disc_number_edit, prepare_track_genre_edit, prepare_track_grouping_edit,
    prepare_track_kind_edit, prepare_track_label_edit, prepare_track_last_played_edit,
    prepare_track_mix_edit, prepare_track_number_edit, prepare_track_play_count_edit,
    prepare_track_rating_edit, prepare_track_remixer_edit, prepare_track_sample_rate_edit,
    prepare_track_size_edit, prepare_track_tonality_edit, prepare_track_total_time_edit,
    prepare_track_year_edit,
};
use serde_json as _;
use sha2 as _;
#[path = "common/xml_tree.rs"]
mod xml_tree;
use xml_tree::{Node, decode, decode_with_text};

pub type Prepare = fn(&[u8], &str, &str, &str) -> Result<Vec<u8>, String>;

/// Adapts one existing guarded per-attribute adapter to the shared `Prepare` shape.
macro_rules! prepare {
    ($adapter:ident, $request:ident, $old:ident, $new:ident) => {
        |source, track_id, old, new| {
            $adapter(
                source,
                &$request {
                    track_id: track_id.into(),
                    $old: old.into(),
                    $new: new.into(),
                },
            )
            .map(|output| output.bytes)
            .map_err(|error| format!("{error:?}"))
        }
    };
}

/// One guarded TRACK attribute edit case.
pub struct Case {
    /// Frozen synthetic source: TrackID 100 has `attr` = `old`; TrackID 200 lacks it.
    pub source: &'static [u8],
    pub attr: &'static str,
    pub old: &'static str,
    pub new: &'static str,
    /// Replacement value for the target-value mutation control.
    pub altered: &'static str,
    /// Value for the injected-on-absent-track mutation control.
    pub unexpected: &'static str,
    /// Decode text/CDATA nodes too, with an inserted-text mutation control.
    pub text_nodes: bool,
    pub prepare: Prepare,
}

pub const ALBUM: Case = Case {
    source: include_bytes!("fixtures/f02-track-album-edit/source.xml"),
    attr: "Album",
    old: "old & record",
    new: "new < record",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_album_edit,
        TrackAlbumEditRequest,
        expected_old_album,
        new_album
    ),
};
pub const ARTIST: Case = Case {
    source: include_bytes!("fixtures/f02-track-artist-edit/source.xml"),
    attr: "Artist",
    old: "Old & exact",
    new: "New Artist",
    altered: "Altered",
    unexpected: "Unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_artist_edit,
        TrackArtistEditRequest,
        expected_old_artist,
        new_artist
    ),
};
pub const AVERAGE_BPM: Case = Case {
    source: include_bytes!("fixtures/f02-track-average-bpm-edit/source.xml"),
    attr: "AverageBpm",
    old: "old & bpm",
    new: "new < bpm",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_average_bpm_edit,
        TrackAverageBpmEditRequest,
        expected_old_average_bpm,
        new_average_bpm
    ),
};
pub const BIT_RATE: Case = Case {
    source: include_bytes!("fixtures/f02-track-bitrate-edit/source.xml"),
    attr: "BitRate",
    old: "old & rate",
    new: "new < rate",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_bit_rate_edit,
        TrackBitRateEditRequest,
        expected_old_bit_rate,
        new_bit_rate
    ),
};
pub const COLOUR: Case = Case {
    source: include_bytes!("fixtures/f02-track-colour-edit/source.xml"),
    attr: "Colour",
    old: "old & palette",
    new: "new < palette",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_colour_edit,
        TrackColourEditRequest,
        expected_old_colour,
        new_colour
    ),
};
pub const COMMENTS: Case = Case {
    source: include_bytes!("fixtures/f02-track-comments-edit/source.xml"),
    attr: "Comments",
    old: "old & note",
    new: "new < note",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_comments_edit,
        TrackCommentsEditRequest,
        expected_old_comments,
        new_comments
    ),
};
pub const COMPOSER: Case = Case {
    source: include_bytes!("fixtures/f02-track-composer-edit/source.xml"),
    attr: "Composer",
    old: "old & composer",
    new: "new < composer",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_composer_edit,
        TrackComposerEditRequest,
        expected_old_composer,
        new_composer
    ),
};
pub const DATE_ADDED: Case = Case {
    source: include_bytes!("fixtures/f02-track-date_added-edit/source.xml"),
    attr: "DateAdded",
    old: "old & stamp",
    new: "new < stamp",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_date_added_edit,
        TrackDateAddedEditRequest,
        expected_old_date_added,
        new_date_added
    ),
};
pub const DATE_MODIFIED: Case = Case {
    source: include_bytes!("fixtures/f02-track-datemodified-edit/source.xml"),
    attr: "DateModified",
    old: "old & stamp",
    new: "new < stamp",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_date_modified_edit,
        TrackDateModifiedEditRequest,
        expected_old_date_modified,
        new_date_modified
    ),
};
pub const DISC_NUMBER: Case = Case {
    source: include_bytes!("fixtures/f02-track-disc-number-edit/source.xml"),
    attr: "DiscNumber",
    old: "old & disc",
    new: "new < disc",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_disc_number_edit,
        TrackDiscNumberEditRequest,
        expected_old_disc_number,
        new_disc_number
    ),
};
pub const GENRE: Case = Case {
    source: include_bytes!("fixtures/f02-track-genre-edit/source.xml"),
    attr: "Genre",
    old: "old & style",
    new: "new < style",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_genre_edit,
        TrackGenreEditRequest,
        expected_old_genre,
        new_genre
    ),
};
pub const GROUPING: Case = Case {
    source: include_bytes!("fixtures/f02-track-grouping-edit/source.xml"),
    attr: "Grouping",
    old: "old & crate",
    new: "new < crate",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_grouping_edit,
        TrackGroupingEditRequest,
        expected_old_grouping,
        new_grouping
    ),
};
pub const KIND: Case = Case {
    source: include_bytes!("fixtures/f02-track-kind-edit/source.xml"),
    attr: "Kind",
    old: "old & kind",
    new: "new < kind",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_kind_edit,
        TrackKindEditRequest,
        expected_old_kind,
        new_kind
    ),
};
pub const LABEL: Case = Case {
    source: include_bytes!("fixtures/f02-track-label-edit/source.xml"),
    attr: "Label",
    old: "old & imprint",
    new: "new < imprint",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_label_edit,
        TrackLabelEditRequest,
        expected_old_label,
        new_label
    ),
};
pub const LAST_PLAYED: Case = Case {
    source: include_bytes!("fixtures/f02-track-lastplayed-edit/source.xml"),
    attr: "LastPlayed",
    old: "old & play",
    new: "new < play",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_last_played_edit,
        TrackLastPlayedEditRequest,
        expected_old_last_played,
        new_last_played
    ),
};
pub const MIX: Case = Case {
    source: include_bytes!("fixtures/f02-track-mix-edit/source.xml"),
    attr: "Mix",
    old: "old & set",
    new: "new < set",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_mix_edit,
        TrackMixEditRequest,
        expected_old_mix,
        new_mix
    ),
};
pub const TRACK_NUMBER: Case = Case {
    source: include_bytes!("fixtures/f02-track-number-edit/source.xml"),
    attr: "TrackNumber",
    old: "old & track",
    new: "new < track",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_number_edit,
        TrackNumberEditRequest,
        expected_old_track_number,
        new_track_number
    ),
};
pub const PLAY_COUNT: Case = Case {
    source: include_bytes!("fixtures/f02-track-playcount-edit/source.xml"),
    attr: "PlayCount",
    old: "old & count",
    new: "new < count",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_play_count_edit,
        TrackPlayCountEditRequest,
        expected_old_play_count,
        new_play_count
    ),
};
pub const RATING: Case = Case {
    source: include_bytes!("fixtures/f02-track-rating-edit/source.xml"),
    attr: "Rating",
    old: "old & stars",
    new: "new < stars",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_rating_edit,
        TrackRatingEditRequest,
        expected_old_rating,
        new_rating
    ),
};
pub const REMIXER: Case = Case {
    source: include_bytes!("fixtures/f02-track-remixer-edit/source.xml"),
    attr: "Remixer",
    old: "old & mix",
    new: "new < mix",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_remixer_edit,
        TrackRemixerEditRequest,
        expected_old_remixer,
        new_remixer
    ),
};
pub const SAMPLE_RATE: Case = Case {
    source: include_bytes!("fixtures/f02-track-sample-rate-edit/source.xml"),
    attr: "SampleRate",
    old: "old & rate",
    new: "new < rate",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_sample_rate_edit,
        TrackSampleRateEditRequest,
        expected_old_sample_rate,
        new_sample_rate
    ),
};
pub const SIZE: Case = Case {
    source: include_bytes!("fixtures/f02-track-size-edit/source.xml"),
    attr: "Size",
    old: "old & size",
    new: "new < size",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_size_edit,
        TrackSizeEditRequest,
        expected_old_size,
        new_size
    ),
};
pub const TONALITY: Case = Case {
    source: include_bytes!("fixtures/f02-track-tonality-edit/source.xml"),
    attr: "Tonality",
    old: "old & key",
    new: "new < key",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_tonality_edit,
        TrackTonalityEditRequest,
        expected_old_tonality,
        new_tonality
    ),
};
pub const TOTAL_TIME: Case = Case {
    source: include_bytes!("fixtures/f02-track-total-time-edit/source.xml"),
    attr: "TotalTime",
    old: "old & time",
    new: "new < time",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: true,
    prepare: prepare!(
        prepare_track_total_time_edit,
        TrackTotalTimeEditRequest,
        expected_old_total_time,
        new_total_time
    ),
};
pub const YEAR: Case = Case {
    source: include_bytes!("fixtures/f02-track-year-edit/source.xml"),
    attr: "Year",
    old: "20&24",
    new: "raw < year",
    altered: "altered",
    unexpected: "unexpected",
    text_nodes: false,
    prepare: prepare!(
        prepare_track_year_edit,
        TrackYearEditRequest,
        expected_old_year,
        new_year
    ),
};

impl Case {
    fn decode(&self, bytes: &[u8]) -> Node {
        if self.text_nodes {
            decode_with_text(bytes)
        } else {
            decode(bytes)
        }
    }
}

fn change_expected(n: &mut Node, attr: &str, old: &str, new: &str) -> bool {
    if n.name == "TRACK" && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == "100") {
        let value = n.attrs.iter_mut().find(|(k, _)| k == attr).unwrap();
        assert_eq!(value.1, old);
        value.1 = new.into();
        return true;
    }
    let mut found = false;
    for child in &mut n.children {
        found |= change_expected(child, attr, old, new);
    }
    found
}

fn has_attr(n: &Node, attr: &str, track_id: &str) -> bool {
    (n.name == "TRACK"
        && n.attrs.iter().any(|(k, v)| k == "TrackID" && v == track_id)
        && n.attrs.iter().any(|(k, _)| k == attr))
        || n.children.iter().any(|c| has_attr(c, attr, track_id))
}

pub fn check(case: &Case, bytes: &[u8]) -> Result<(), String> {
    check_with(case, bytes, case.new)
}

pub fn check_with(case: &Case, bytes: &[u8], expected_new: &str) -> Result<(), String> {
    let actual = case.decode(bytes);
    let mut expected = case.decode(case.source);
    assert!(
        change_expected(&mut expected, case.attr, case.old, expected_new),
        "fixture lacks TrackID 100 {}",
        case.attr
    );
    if actual != expected {
        return Err(format!("output differs beyond TrackID 100 {}", case.attr));
    }
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;")
}

fn verify(case: &Case) {
    let attr = case.attr;
    let frozen = case.source.to_vec();
    let output = (case.prepare)(case.source, "100", case.old, case.new).unwrap();
    assert_eq!(
        case.source,
        frozen.as_slice(),
        "source fixture bytes changed"
    );
    let before = case.decode(case.source);
    assert!(
        has_attr(&before, attr, "100"),
        "fixture must contain target {attr}"
    );
    assert!(
        !has_attr(&before, attr, "200"),
        "fixture must preserve absent {attr} state"
    );
    check(case, &output).unwrap();
    for (track_id, expected_old) in [("100", "stale"), ("200", case.old), ("999", case.old)] {
        assert!(
            (case.prepare)(case.source, track_id, expected_old, case.new).is_err(),
            "accepted stale/wrong-target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        (
            format!("{attr}=\"{}\"", xml_escape(case.new)),
            format!("{attr}=\"{}\"", case.altered),
        ),
        ("TrackID=\"100\"".into(), "TrackID=\"101\"".into()),
        (
            "Location=\"file://localhost/music/a.flac\"".into(),
            "Location=\"file://localhost/music/x.flac\"".into(),
        ),
        ("Entries=\"2\"".into(), "Entries=\"1\"".into()),
        ("Key=\"100\"".into(), "Key=\"101\"".into()),
        (
            "TrackID=\"200\" Name=\"B\"".into(),
            format!("TrackID=\"200\" Name=\"B\" {attr}=\"{}\"", case.unexpected),
        ),
    ] {
        let mutated = text.replacen(&from, &to, 1);
        assert_ne!(mutated, text, "mutation fixture did not match: {from}");
        assert!(
            check(case, mutated.as_bytes()).is_err(),
            "oracle accepted semantic mutation {from}"
        );
    }
    if case.text_nodes {
        let inserted = text.replacen(
            "<COLLECTION Entries=\"2\">",
            "<COLLECTION Entries=\"2\">injected",
            1,
        );
        assert_ne!(inserted, text);
        assert!(
            check(case, inserted.as_bytes()).is_err(),
            "oracle accepted inserted text node"
        );
    }
}

#[test]
fn independently_verifies_guarded_track_album_edit_and_rejects_mutations() {
    verify(&ALBUM);
}
#[test]
fn independently_verifies_guarded_track_artist_edit_and_rejects_mutations() {
    verify(&ARTIST);
}
#[test]
fn independently_verifies_guarded_track_play_bpm_edit_and_rejects_mutations() {
    verify(&AVERAGE_BPM);
}
#[test]
fn independently_verifies_guarded_track_bit_rate_edit_and_rejects_mutations() {
    verify(&BIT_RATE);
}
#[test]
fn independently_verifies_guarded_track_colour_edit_and_rejects_mutations() {
    verify(&COLOUR);
}
#[test]
fn independently_verifies_guarded_track_comments_edit_and_rejects_mutations() {
    verify(&COMMENTS);
}
#[test]
fn independently_verifies_guarded_track_composer_edit_and_rejects_mutations() {
    verify(&COMPOSER);
}
#[test]
fn independently_verifies_guarded_track_date_added_edit_and_rejects_mutations() {
    verify(&DATE_ADDED);
}
#[test]
fn independently_verifies_guarded_track_date_modified_edit_and_rejects_mutations() {
    verify(&DATE_MODIFIED);
}
#[test]
fn independently_verifies_guarded_track_disc_number_edit_and_rejects_mutations() {
    verify(&DISC_NUMBER);
}
#[test]
fn independently_verifies_guarded_track_genre_edit_and_rejects_mutations() {
    verify(&GENRE);
}
#[test]
fn independently_verifies_guarded_track_grouping_edit_and_rejects_mutations() {
    verify(&GROUPING);
}
#[test]
fn independently_verifies_guarded_track_kind_edit_and_rejects_mutations() {
    verify(&KIND);
}
#[test]
fn independently_verifies_guarded_track_label_edit_and_rejects_mutations() {
    verify(&LABEL);
}
#[test]
fn independently_verifies_guarded_track_last_played_edit_and_rejects_mutations() {
    verify(&LAST_PLAYED);
}
#[test]
fn independently_verifies_guarded_track_mix_edit_and_rejects_mutations() {
    verify(&MIX);
}
#[test]
fn independently_verifies_guarded_track_number_edit_and_rejects_mutations() {
    verify(&TRACK_NUMBER);
}
#[test]
fn independently_verifies_guarded_track_play_count_edit_and_rejects_mutations() {
    verify(&PLAY_COUNT);
}
#[test]
fn independently_verifies_guarded_track_rating_edit_and_rejects_mutations() {
    verify(&RATING);
}
#[test]
fn independently_verifies_guarded_track_remixer_edit_and_rejects_mutations() {
    verify(&REMIXER);
}
#[test]
fn independently_verifies_guarded_raw_track_sample_rate_edit_and_rejects_mutations() {
    verify(&SAMPLE_RATE);
}
#[test]
fn independently_verifies_guarded_track_size_edit_and_rejects_mutations() {
    verify(&SIZE);
}
#[test]
fn independently_verifies_guarded_track_tonality_edit_and_rejects_mutations() {
    verify(&TONALITY);
}
#[test]
fn independently_verifies_guarded_track_total_time_edit_and_rejects_mutations() {
    verify(&TOTAL_TIME);
}
#[test]
fn independently_verifies_guarded_track_year_edit_and_rejects_mutations() {
    verify(&YEAR);
}
