//! F-09 independent post-edit oracle for the accepted A1 F-02 guarded Track Name edit.
use oxidj_rekordbox::{TrackNameEditRequest, prepare_track_name_edit};
use serde_json as _;
use sha2 as _;

#[path = "common/f02_track_name_oracle_support.rs"]
mod oracle_support;

const SOURCE: &[u8] = include_bytes!("fixtures/rekordbox-a1/F-02/source.bin");

#[test]
fn independently_verifies_guarded_track_name_edit_and_rejects_mutations() {
    let frozen = SOURCE.to_vec();
    let request = TrackNameEditRequest {
        track_id: "100".into(),
        expected_old_name: "A".into(),
        new_name: "New < title".into(),
    };
    let output = prepare_track_name_edit(SOURCE, &request).unwrap().bytes;
    assert_eq!(SOURCE, frozen.as_slice(), "source fixture bytes changed");
    oracle_support::check(&output).unwrap();
    for (track_id, expected_old_name) in [("100", "stale"), ("999", "A")] {
        let bad = TrackNameEditRequest {
            track_id: track_id.into(),
            expected_old_name: expected_old_name.into(),
            ..request.clone()
        };
        assert!(
            prepare_track_name_edit(SOURCE, &bad).is_err(),
            "accepted stale/wrong target guard"
        );
    }
    let text = String::from_utf8(output).unwrap();
    for (from, to) in [
        ("Name=\"New &lt; title\"", "Name=\"Altered\""),
        ("Name=\"B\"", "Name=\"Altered\""),
        ("TrackID=\"100\"", "TrackID=\"101\""),
        (
            "Location=\"file://localhost/music/a.flac\"",
            "Location=\"file://localhost/music/x.flac\"",
        ),
        ("Entries=\"3\"", "Entries=\"2\""),
        ("Key=\"100\"", "Key=\"101\""),
    ] {
        let mutated = text.replacen(from, to, 1);
        assert_ne!(mutated, text, "mutation fixture did not match: {from}");
        assert!(
            oracle_support::check(mutated.as_bytes()).is_err(),
            "oracle accepted semantic mutation {from}"
        );
    }
}
