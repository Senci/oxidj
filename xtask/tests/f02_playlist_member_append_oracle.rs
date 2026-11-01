use oxidj_rekordbox::{PlaylistMemberAppendRequest, prepare_playlist_member_append};
use serde_json as _;
use sha2 as _;
#[path = "common/f02_playlist_member_append_oracle_shared.rs"]
mod oracle;
use oracle::{SOURCE, check};
#[test]
fn independently_decodes_member_append_and_rejects_semantic_mutations() {
    let frozen = SOURCE.to_vec();
    let request = PlaylistMemberAppendRequest {
        playlist_index: 1,
        track_id: "100".into(),
        expected_order: vec!["200".into(), "400".into()],
    };
    let output = prepare_playlist_member_append(SOURCE, &request)
        .unwrap()
        .bytes;
    assert_eq!(
        SOURCE,
        frozen.as_slice(),
        "adapter must not mutate source bytes"
    );
    check(&output).unwrap();
    let text = String::from_utf8(output).unwrap();
    let mutations = [
        text.replacen("Key=\"100\"", "Key=\"200\"", 1),
        text.replacen("Key=\"400\"", "Key=\"999\"", 1),
        text.replacen("Entries=\"3\"", "Entries=\"2\"", 1),
        text.replacen("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"1\"", 1),
        text.replacen("TrackID=\"200\"", "TrackID=\"999\"", 1),
        text.replacen("Name=\"Techno\"", "Name=\"Other\"", 1),
        text.replacen("Name=\"House\"", "Name=\"Other\"", 1)
            .replacen("Name=\"Techno\"", "Name=\"House\"", 1)
            .replacen("Name=\"Other\"", "Name=\"Techno\"", 1),
    ];
    for mutated in mutations {
        assert!(
            check(mutated.as_bytes()).is_err(),
            "oracle accepted mutation: {mutated}"
        );
    }
    let wrong_playlist_output = prepare_playlist_member_append(
        SOURCE,
        &PlaylistMemberAppendRequest {
            playlist_index: 0,
            track_id: "400".into(),
            expected_order: vec!["100".into(), "200".into(), "300".into()],
        },
    )
    .unwrap()
    .bytes;
    assert!(
        check(&wrong_playlist_output).is_err(),
        "oracle accepted append to wrong playlist"
    );
    assert!(
        prepare_playlist_member_append(
            SOURCE,
            &PlaylistMemberAppendRequest {
                playlist_index: 1,
                track_id: "100".into(),
                expected_order: vec!["999".into()]
            }
        )
        .is_err()
    );
    assert!(
        prepare_playlist_member_append(
            SOURCE,
            &PlaylistMemberAppendRequest {
                playlist_index: 0,
                track_id: "999".into(),
                expected_order: vec!["200".into(), "400".into()]
            }
        )
        .is_err()
    );
}
