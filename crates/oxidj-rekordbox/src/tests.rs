use super::*;

#[test]
fn preview_preserves_folder_order_paths_raw_counts_and_empty_nodes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "<NODE Type=\"0\" Name=\"Sets\" Count=\"2\">",
                "<NODE Type=\"0\" Name=\"Sets\" Count=\"5\">\n        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n        <NODE Type=\"0\" Name=\"Paired\" Count=\"0\"></NODE>",
            );
    let preview = parse(source.as_bytes()).unwrap();
    assert_eq!(
        preview.folders,
        [
            Folder {
                ordinal: 0,
                path: vec!["ROOT".into()],
                name: "ROOT".into(),
                count: Some("1".into()),
            },
            Folder {
                ordinal: 1,
                path: vec!["ROOT".into(), "Sets".into()],
                name: "Sets".into(),
                count: Some("5".into()),
            },
            Folder {
                ordinal: 2,
                path: vec!["ROOT".into(), "Sets".into(), "Empty".into()],
                name: "Empty".into(),
                count: Some("0".into()),
            },
            Folder {
                ordinal: 3,
                path: vec!["ROOT".into(), "Sets".into(), "Empty".into()],
                name: "Empty".into(),
                count: Some("0".into()),
            },
            Folder {
                ordinal: 4,
                path: vec!["ROOT".into(), "Sets".into(), "Paired".into()],
                name: "Paired".into(),
                count: Some("0".into()),
            },
        ]
    );
}

#[test]
fn guarded_position_mark_delete_checks_raw_guard_optionality_and_preserves_other_data() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
    let request = PositionMarkDeleteRequest {
        track_id: "700".into(),
        mark_index: 0,
        expected_type: "0".into(),
        expected_start: "5".into(),
        expected_name: "Intro".into(),
        expected_end: None,
    };
    let prepared = prepare_position_mark_delete(source, &request).unwrap();
    assert!(!String::from_utf8_lossy(&prepared.bytes).contains("POSITION_MARK"));
    let before = preflight_export(source).unwrap();
    let after = preflight_export(&prepared.bytes).unwrap();
    let mut expected = before.selected_tracks[0].clone();
    expected.marks.remove(0);
    assert_eq!(after.selected_tracks[0], expected);
    assert_eq!(after.selected_playlists, before.selected_playlists);
    let with_end = String::from_utf8(source.to_vec()).unwrap().replace(
        "Start=\"5\" Num=\"0\"/>",
        "Start=\"5\" Num=\"0\" End=\"0\"/>",
    );
    let mut with_end_request = request.clone();
    with_end_request.expected_end = Some("0".into());
    assert!(prepare_position_mark_delete(with_end.as_bytes(), &with_end_request).is_ok());
    with_end_request.expected_end = None;
    assert!(prepare_position_mark_delete(with_end.as_bytes(), &with_end_request).is_err());
    let mut stale = request.clone();
    stale.expected_type = "00".into();
    assert!(prepare_position_mark_delete(source, &stale).is_err());
    let mut missing = request.clone();
    missing.mark_index = 9;
    assert!(prepare_position_mark_delete(source, &missing).is_err());
    assert!(prepare_position_mark_delete(b"<bad", &request).is_err());
}

#[test]
fn guarded_folder_create_adds_escaped_child_and_preserves_other_semantics() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let before = preflight_export(source).unwrap();
    let result = prepare_folder_create(
        source,
        &FolderCreateRequest {
            parent_index: 0,
            expected_parent_name: "Sets".into(),
            name: "R&B <late>".into(),
        },
    )
    .unwrap();
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    let output = String::from_utf8(result.bytes).unwrap();
    assert!(output.contains("Name=\"Sets\" Count=\"3\""));
    assert!(output.contains("<NODE Type=\"0\" Name=\"R&amp;B &lt;late&gt;\" Count=\"0\"/>"));
    let after = preflight_export(output.as_bytes()).unwrap();
    assert_eq!(after.selected_tracks, before.selected_tracks);
    assert_eq!(after.selected_playlists, before.selected_playlists);
}

#[test]
fn guarded_folder_create_rejects_root_stale_invalid_and_sibling_collision() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let req = |parent_index, expected_parent_name: &str, name: &str| FolderCreateRequest {
        parent_index,
        expected_parent_name: expected_parent_name.into(),
        name: name.into(),
    };
    assert!(prepare_folder_create(source, &req(0, "ROOT", "child")).is_err());
    assert!(prepare_folder_create(source, &req(0, "stale", "child")).is_err());
    assert!(prepare_folder_create(source, &req(99, "Sets", "child")).is_err());
    assert!(prepare_folder_create(source, &req(0, "Sets", "House")).is_err());
    assert!(prepare_folder_create(source, &req(0, "Sets", "bad\u{1}")).is_err());
    assert!(prepare_folder_create(b"<bad", &req(0, "Sets", "child")).is_err());
    let bad_count = String::from_utf8(source.to_vec())
        .unwrap()
        .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"9\"");
    assert!(prepare_folder_create(bad_count.as_bytes(), &req(0, "Sets", "child")).is_err());
}

#[test]
fn guarded_playlist_create_adds_escaped_child_and_preserves_other_semantics() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let before = preflight_export(source).unwrap();
    let result = prepare_playlist_create(
        source,
        &PlaylistCreateRequest {
            parent_index: 0,
            expected_parent_name: "Sets".into(),
            name: "R&B <late>".into(),
        },
    )
    .unwrap();
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    let output = String::from_utf8(result.bytes).unwrap();
    assert!(output.contains("Name=\"Sets\" Count=\"3\""));
    assert!(
        output.contains(
            "<NODE Type=\"1\" Name=\"R&amp;B &lt;late&gt;\" KeyType=\"0\" Entries=\"0\"/>"
        )
    );
    let after = preflight_export(output.as_bytes()).unwrap();
    assert_eq!(after.selected_tracks, before.selected_tracks);
    assert_eq!(
        after
            .selected_playlists
            .get(..before.selected_playlists.len()),
        Some(before.selected_playlists.as_slice())
    );
}

#[test]
fn guarded_playlist_create_rejects_root_stale_invalid_and_sibling_collision() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let req = |parent_index, expected_parent_name: &str, name: &str| PlaylistCreateRequest {
        parent_index,
        expected_parent_name: expected_parent_name.into(),
        name: name.into(),
    };
    assert!(prepare_playlist_create(source, &req(0, "ROOT", "child")).is_err());
    assert!(prepare_playlist_create(source, &req(0, "stale", "child")).is_err());
    assert!(prepare_playlist_create(source, &req(99, "Sets", "child")).is_err());
    assert!(prepare_playlist_create(source, &req(0, "Sets", "House")).is_err());
    assert!(prepare_playlist_create(source, &req(0, "Sets", "bad\u{1}")).is_err());
    assert!(prepare_playlist_create(b"<bad", &req(0, "Sets", "child")).is_err());
    let bad_count = String::from_utf8(source.to_vec())
        .unwrap()
        .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"9\"");
    assert!(prepare_playlist_create(bad_count.as_bytes(), &req(0, "Sets", "child")).is_err());
}

#[test]
fn guarded_empty_folder_delete_preserves_other_semantics_and_source() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    for node in [
        "<NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>",
        "<NODE Type=\"0\" Name=\"Empty\" Count=\"0\"></NODE>",
    ] {
        let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"")
            .replace(
                "      </NODE>\n    </NODE>",
                &format!("        {node}\n      </NODE>\n    </NODE>"),
            );
        let bytes = source.as_bytes();
        let before = preflight_export(bytes).unwrap();
        let result = prepare_folder_delete(
            bytes,
            &FolderDeleteRequest {
                folder_index: 1,
                expected_name: "Empty".into(),
            },
        )
        .unwrap();
        assert_eq!(bytes, source.as_bytes());
        let output = String::from_utf8(result.bytes).unwrap();
        assert!(output.contains("Name=\"Sets\" Count=\"2\""));
        assert!(!output.contains("Name=\"Empty\""));
        let after = preflight_export(output.as_bytes()).unwrap();
        assert_eq!(after.selected_tracks, before.selected_tracks);
        assert_eq!(after.selected_playlists, before.selected_playlists);
    }
}

#[test]
fn guarded_empty_folder_delete_rejects_root_nonempty_stale_and_bad_counts() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let req = |folder_index, expected_name: &str| FolderDeleteRequest {
        folder_index,
        expected_name: expected_name.into(),
    };
    assert!(prepare_folder_delete(fixture, &req(0, "ROOT")).is_err());
    assert!(prepare_folder_delete(fixture, &req(0, "stale")).is_err());
    assert!(prepare_folder_delete(fixture, &req(99, "Empty")).is_err());
    let nonempty = String::from_utf8(fixture.to_vec()).unwrap()
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"")
            .replace("      </NODE>\n    </NODE>", "        <NODE Type=\"0\" Name=\"Nope\" Count=\"1\"><NODE Type=\"1\" Name=\"x\" Entries=\"0\" KeyType=\"0\"/></NODE>\n      </NODE>\n    </NODE>");
    assert!(prepare_folder_delete(nonempty.as_bytes(), &req(1, "Nope")).is_err());
    let bad_parent = String::from_utf8(fixture.to_vec())
        .unwrap()
        .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"0\"")
        .replace(
            "      </NODE>\n    </NODE>",
            "        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n      </NODE>\n    </NODE>",
        );
    assert!(prepare_folder_delete(bad_parent.as_bytes(), &req(1, "Empty")).is_err());
}

#[test]
fn raw_position_mark_end_optionality_survives_both_track_parser_paths_and_export() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace(
                "<TRACK TrackID=\"100\" Name=\"A\" Location=\"file://localhost/music/a.flac\"/>",
                "<TRACK TrackID=\"100\" Name=\"A\" Location=\"file://localhost/music/a.flac\">\n      <POSITION_MARK Name=\"zero\" Type=\"0\" Start=\"0\" Num=\"0\" End=\"0\"></POSITION_MARK>\n    </TRACK>",
            )
            .replace(
                "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\"/>",
                "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\">\n      <POSITION_MARK Name=\"absent\" Type=\"0\" Start=\"0\" Num=\"0\"/>\n    </TRACK>",
            );
    let bytes = source.as_bytes();
    let parsed = parse(bytes).unwrap();
    assert_eq!(parsed.tracks[0].marks[0].end_seconds.as_deref(), Some("0"));
    assert_eq!(parsed.tracks[1].marks[0].end_seconds, None);
    assert!(
        parsed
            .tracks
            .iter()
            .any(|track| track.id == "300" && track.marks.is_empty())
    ); // self-closing TRACK path
    let plan = preflight_export(bytes).unwrap();
    assert_eq!(
        plan.selected_tracks[0].marks[0].end_seconds.as_deref(),
        Some("0")
    );
    assert_eq!(plan.selected_tracks[1].marks[0].end_seconds, None);
    assert_eq!(export_xml(bytes).unwrap(), bytes);
}

#[test]
fn raw_optional_dates_and_play_count_preserve_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" DateAdded=\"2024-02-03\" DateModified=\"2024-02-04\" LastPlayed=\"2024-02-05\" PlayCount=\"not-a-number\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" DateAdded=\"\" DateModified=\"\" LastPlayed=\"\" PlayCount=\"\"",
            );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(preview.tracks[0].date_added.as_deref(), Some("2024-02-03"));
    assert_eq!(preview.tracks[1].date_added.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].date_added, None);
    assert_eq!(
        preview.tracks[0].date_modified.as_deref(),
        Some("2024-02-04")
    );
    assert_eq!(preview.tracks[1].date_modified.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].date_modified, None);
    assert_eq!(preview.tracks[0].last_played.as_deref(), Some("2024-02-05"));
    assert_eq!(preview.tracks[1].last_played.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].last_played, None);
    assert_eq!(
        preview.tracks[0].play_count.as_deref(),
        Some("not-a-number")
    );
    assert_eq!(preview.tracks[1].play_count.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].play_count, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(
        plan.selected_tracks[0].date_added.as_deref(),
        Some("2024-02-03")
    );
    assert_eq!(plan.selected_tracks[1].date_added.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].date_added, None);
    assert_eq!(
        plan.selected_tracks[0].date_modified.as_deref(),
        Some("2024-02-04")
    );
    assert_eq!(plan.selected_tracks[1].date_modified.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].date_modified, None);
    assert_eq!(
        plan.selected_tracks[0].last_played.as_deref(),
        Some("2024-02-05")
    );
    assert_eq!(plan.selected_tracks[1].last_played.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].last_played, None);
    assert_eq!(
        plan.selected_tracks[0].play_count.as_deref(),
        Some("not-a-number")
    );
    assert_eq!(plan.selected_tracks[1].play_count.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].play_count, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn guarded_raw_rating_edit_rejects_stale_missing_invalid_and_malformed() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let xml = String::from_utf8(base.to_vec())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Rating=\"-1\" Name=\"A\"",
            1,
        )
        .into_bytes();
    let req = TrackRatingEditRequest {
        track_id: "100".into(),
        expected_old_rating: "-1".into(),
        new_rating: "A & <x>".into(),
    };
    let result = prepare_track_rating_edit(&xml, &req).unwrap();
    assert!(
        String::from_utf8_lossy(&result.bytes).contains("Rating=\"A &amp; &lt;x&gt;\" Name=\"A\"")
    );
    assert!(result.bytes.windows(6).any(|w| w == b"<TRACK"));
    let mut stale = req.clone();
    stale.expected_old_rating = "old".into();
    assert!(prepare_track_rating_edit(&xml, &stale).is_err());
    assert!(
        prepare_track_rating_edit(
            &xml,
            &TrackRatingEditRequest {
                track_id: "200".into(),
                expected_old_rating: "".into(),
                new_rating: "1".into()
            }
        )
        .is_err()
    );
    let mut invalid = req;
    invalid.new_rating = "bad\u{1}".into();
    assert!(prepare_track_rating_edit(&xml, &invalid).is_err());
    assert!(prepare_track_rating_edit(b"<broken", &invalid).is_err());
}

#[test]
fn guarded_raw_colour_edit_rejects_stale_missing_invalid_and_malformed() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let xml = String::from_utf8(base.to_vec())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Colour=\"-1\" Name=\"A\"",
            1,
        )
        .into_bytes();
    let req = TrackColourEditRequest {
        track_id: "100".into(),
        expected_old_colour: "-1".into(),
        new_colour: "A & <x>".into(),
    };
    let result = prepare_track_colour_edit(&xml, &req).unwrap();
    assert!(
        String::from_utf8_lossy(&result.bytes).contains("Colour=\"A &amp; &lt;x&gt;\" Name=\"A\"")
    );
    assert!(result.bytes.windows(6).any(|w| w == b"<TRACK"));
    let mut stale = req.clone();
    stale.expected_old_colour = "old".into();
    assert!(prepare_track_colour_edit(&xml, &stale).is_err());
    assert!(
        prepare_track_colour_edit(
            &xml,
            &TrackColourEditRequest {
                track_id: "200".into(),
                expected_old_colour: "".into(),
                new_colour: "1".into()
            }
        )
        .is_err()
    );
    let mut invalid = req;
    invalid.new_colour = "bad\u{1}".into();
    assert!(prepare_track_colour_edit(&xml, &invalid).is_err());
    assert!(prepare_track_colour_edit(b"<broken", &invalid).is_err());
}

#[test]
fn raw_optional_rating_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Rating=\"-1\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Rating=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(preview.tracks[0].rating.as_deref(), Some("-1"));
    assert_eq!(preview.tracks[1].rating.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].rating, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(plan.selected_tracks[0].rating.as_deref(), Some("-1"));
    assert_eq!(plan.selected_tracks[1].rating.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].rating, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn colour_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Colour=\"#12aBcF\" AverageBpm=\"126.12500000000001\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Colour=\"\" AverageBpm=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(preview.tracks[0].colour.as_deref(), Some("#12aBcF"));
    assert_eq!(preview.tracks[1].colour.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].colour, None);
    assert_eq!(
        preview.tracks[0].average_bpm.as_deref(),
        Some("126.12500000000001")
    );
    assert_eq!(preview.tracks[1].average_bpm.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].average_bpm, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(plan.selected_tracks[0].colour.as_deref(), Some("#12aBcF"));
    assert_eq!(plan.selected_tracks[1].colour.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].colour, None);
    assert_eq!(
        plan.selected_tracks[0].average_bpm.as_deref(),
        Some("126.12500000000001")
    );
    assert_eq!(plan.selected_tracks[1].average_bpm.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].average_bpm, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn kind_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Kind=\"AIFF\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Kind=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(preview.tracks[0].kind.as_deref(), Some("AIFF"));
    assert_eq!(preview.tracks[1].kind.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].kind, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(plan.selected_tracks[0].kind.as_deref(), Some("AIFF"));
    assert_eq!(plan.selected_tracks[1].kind.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].kind, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn year_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Year=\"2024\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Year=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(preview.tracks[0].year.as_deref(), Some("2024"));
    assert_eq!(preview.tracks[1].year.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].year, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(plan.selected_tracks[0].year.as_deref(), Some("2024"));
    assert_eq!(plan.selected_tracks[1].year.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].year, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn size_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Size=\"9223372036854775807\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Size=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(
        preview.tracks[0].size.as_deref(),
        Some("9223372036854775807")
    );
    assert_eq!(preview.tracks[1].size.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].size, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(
        plan.selected_tracks[0].size.as_deref(),
        Some("9223372036854775807")
    );
    assert_eq!(plan.selected_tracks[1].size.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].size, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn track_number_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "TrackID=\"100\" Name=\"A\"",
                "TrackID=\"100\" Name=\"A\" TrackNumber=\"-2147483648\" TotalTime=\"3.141592653589793\" BitRate=\"-2147483648\" SampleRate=\"48000.0\"",
            )
            .replace(
                "TrackID=\"200\" Name=\"B\"",
                "TrackID=\"200\" Name=\"B\" TrackNumber=\"\" TotalTime=\"\" BitRate=\"\" SampleRate=\"\"",
            );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(
        preview.tracks[0].track_number.as_deref(),
        Some("-2147483648")
    );
    assert_eq!(preview.tracks[1].track_number.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].track_number, None);
    assert_eq!(
        preview.tracks[0].total_time.as_deref(),
        Some("3.141592653589793")
    );
    assert_eq!(preview.tracks[1].total_time.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].total_time, None);
    assert_eq!(preview.tracks[0].bit_rate.as_deref(), Some("-2147483648"));
    assert_eq!(preview.tracks[1].bit_rate.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].bit_rate, None);
    assert_eq!(preview.tracks[0].sample_rate.as_deref(), Some("48000.0"));
    assert_eq!(preview.tracks[1].sample_rate.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].sample_rate, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(
        plan.selected_tracks[0].track_number.as_deref(),
        Some("-2147483648")
    );
    assert_eq!(plan.selected_tracks[1].track_number.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].track_number, None);
    assert_eq!(
        plan.selected_tracks[0].total_time.as_deref(),
        Some("3.141592653589793")
    );
    assert_eq!(plan.selected_tracks[1].total_time.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].total_time, None);
    assert_eq!(
        plan.selected_tracks[0].bit_rate.as_deref(),
        Some("-2147483648")
    );
    assert_eq!(plan.selected_tracks[1].bit_rate.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].bit_rate, None);
    assert_eq!(
        plan.selected_tracks[0].sample_rate.as_deref(),
        Some("48000.0")
    );
    assert_eq!(plan.selected_tracks[1].sample_rate.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].sample_rate, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn disc_number_optional_metadata_preserves_present_empty_absent_and_export_bytes() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" DiscNumber=\"-2147483648\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" DiscNumber=\"\"",
        );
    let source = source.as_bytes();
    let preview = parse(source).unwrap();
    assert_eq!(
        preview.tracks[0].disc_number.as_deref(),
        Some("-2147483648")
    );
    assert_eq!(preview.tracks[1].disc_number.as_deref(), Some(""));
    assert_eq!(preview.tracks[2].disc_number, None);
    let plan = preflight_export(source).unwrap();
    assert_eq!(
        plan.selected_tracks[0].disc_number.as_deref(),
        Some("-2147483648")
    );
    assert_eq!(plan.selected_tracks[1].disc_number.as_deref(), Some(""));
    assert_eq!(plan.selected_tracks[2].disc_number, None);
    assert_eq!(export_xml(source).unwrap(), source);
}

#[test]
fn guarded_playlist_delete_removes_exact_node_and_preserves_collection() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistDeleteRequest {
        playlist_index: 0,
        expected_name: "House".into(),
        expected_order: vec!["100".into(), "200".into(), "300".into()],
    };
    let prepared = prepare_playlist_delete(source, &request).unwrap();
    assert_eq!(
        preflight_export(source).unwrap().selected_playlists.len(),
        2
    );
    let after = preflight_export(&prepared.bytes).unwrap();
    assert_eq!(after.selected_playlists.len(), 1);
    assert_eq!(after.selected_playlists[0].name, "Techno");
    assert_eq!(after.selected_tracks.len(), 4);
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    assert!(
        prepare_playlist_delete(
            source,
            &PlaylistDeleteRequest {
                expected_name: "stale".into(),
                ..request
            }
        )
        .is_err()
    );
}

#[test]
fn guarded_playlist_delete_handles_duplicate_names_and_refuses_duplicate_guards() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let duplicate_name = String::from_utf8(source.to_vec())
        .unwrap()
        .replace("Name=\"Techno\"", "Name=\"House\"")
        .into_bytes();
    let request = PlaylistDeleteRequest {
        playlist_index: 1,
        expected_name: "House".into(),
        expected_order: vec!["200".into(), "400".into()],
    };
    let prepared = prepare_playlist_delete(&duplicate_name, &request).unwrap();
    let after = preflight_export(&prepared.bytes).unwrap();
    assert_eq!(after.selected_playlists.len(), 1);
    assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
    assert!(
        String::from_utf8(prepared.bytes)
            .unwrap()
            .contains("<NODE Type=\"0\" Name=\"Sets\" Count=\"1\">")
    );

    let ambiguous = String::from_utf8(duplicate_name)
            .unwrap()
            .replace(
                "<NODE Type=\"1\" Name=\"House\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                "<NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>",
            )
            .into_bytes();
    let ambiguous_error = prepare_playlist_delete(
        &ambiguous,
        &PlaylistDeleteRequest {
            playlist_index: 1,
            expected_name: "House".into(),
            expected_order: vec!["100".into(), "200".into(), "300".into()],
        },
    )
    .unwrap_err();
    assert_eq!(ambiguous_error.0, "playlist deletion guard is ambiguous");
}

#[test]
fn guarded_playlist_delete_handles_empty_self_closing_playlist() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let empty = String::from_utf8(source.to_vec())
            .unwrap()
            .replace(
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
            )
            .into_bytes();
    let prepared = prepare_playlist_delete(
        &empty,
        &PlaylistDeleteRequest {
            playlist_index: 1,
            expected_name: "Techno".into(),
            expected_order: vec![],
        },
    )
    .unwrap();
    let after = preflight_export(&prepared.bytes).unwrap();
    assert_eq!(after.selected_playlists.len(), 1);
    assert_eq!(after.selected_tracks.len(), 4);
    assert!(
        String::from_utf8(prepared.bytes)
            .unwrap()
            .contains("<NODE Type=\"0\" Name=\"Sets\" Count=\"1\">")
    );
}

#[test]
fn guarded_playlist_delete_refuses_malformed_xml_root_and_folder_selection() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let root = PlaylistDeleteRequest {
        playlist_index: 0,
        expected_name: "ROOT".into(),
        expected_order: vec![],
    };
    let folder = PlaylistDeleteRequest {
        playlist_index: 2,
        expected_name: "Sets".into(),
        expected_order: vec![],
    };
    assert!(prepare_playlist_delete(source, &root).is_err());
    assert!(prepare_playlist_delete(source, &folder).is_err());
    let malformed = b"<DJ_PLAYLISTS>";
    let playlist = PlaylistDeleteRequest {
        playlist_index: 0,
        expected_name: "House".into(),
        expected_order: vec!["100".into(), "200".into(), "300".into()],
    };
    assert!(prepare_playlist_delete(malformed, &playlist).is_err());
}

#[test]
fn guarded_playlist_member_delete_keeps_collection_and_other_playlist() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistMemberDeleteRequest {
        playlist_index: 0,
        expected_order: vec!["100".into(), "200".into(), "300".into()],
        track_id: "200".into(),
    };
    let result = prepare_playlist_member_delete(source, &request).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(
        after
            .selected_tracks
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["100", "200", "300", "400"]
    );
    assert_eq!(after.selected_playlists[0].tracks, ["100", "300"]);
    assert_eq!(after.selected_playlists[1].tracks, ["200", "400"]);
    assert!(
        result
            .bytes
            .windows(b"Name=\"House\" Entries=\"2\"".len())
            .any(|w| w == b"Name=\"House\" Entries=\"2\"")
    );
    assert!(
        result
            .bytes
            .windows(b"Name=\"Techno\" Entries=\"2\"".len())
            .any(|w| w == b"Name=\"Techno\" Entries=\"2\"")
    );
    assert!(
        prepare_playlist_member_delete(
            source,
            &PlaylistMemberDeleteRequest {
                expected_order: vec!["100".into(), "300".into()],
                ..request
            }
        )
        .is_err()
    );
}

#[test]
fn guarded_playlist_member_append_preserves_collection_and_other_playlist() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistMemberAppendRequest {
        playlist_index: 1,
        expected_order: vec!["200".into(), "400".into()],
        track_id: "100".into(),
    };
    let result = prepare_playlist_member_append(source, &request).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(
        after.selected_tracks,
        preflight_export(source).unwrap().selected_tracks
    );
    assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(after.selected_playlists[1].tracks, ["200", "400", "100"]);
    assert!(
        result
            .bytes
            .windows(b"Name=\"Techno\" Entries=\"3\"".len())
            .any(|w| w == b"Name=\"Techno\" Entries=\"3\"")
    );
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    for invalid in [
        PlaylistMemberAppendRequest {
            expected_order: vec!["200".into()],
            ..request.clone()
        },
        PlaylistMemberAppendRequest {
            track_id: "200".into(),
            ..request.clone()
        },
        PlaylistMemberAppendRequest {
            track_id: "999".into(),
            ..request.clone()
        },
    ] {
        assert!(prepare_playlist_member_append(source, &invalid).is_err());
    }
}

#[test]
fn guarded_playlist_member_append_expands_an_empty_self_closing_playlist() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace(
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
            )
            .into_bytes();
    let request = PlaylistMemberAppendRequest {
        playlist_index: 1,
        expected_order: vec![],
        track_id: "100".into(),
    };
    let result = prepare_playlist_member_append(&source, &request).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(
        after.selected_tracks,
        preflight_export(&source).unwrap().selected_tracks
    );
    assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(after.selected_playlists[1].tracks, ["100"]);
    assert!(
        result
            .bytes
            .windows(b"Name=\"Techno\" Entries=\"1\"".len())
            .any(|w| w == b"Name=\"Techno\" Entries=\"1\"")
    );
    assert!(
        result
            .bytes
            .windows(b"<TRACK Key=\"100\"/></NODE>".len())
            .any(|w| w == b"<TRACK Key=\"100\"/></NODE>")
    );
    assert_eq!(
            source,
            String::from_utf8(fixture.to_vec())
                .unwrap()
                .replace(
                    "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
                    "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
                )
                .as_bytes()
        );
}

#[test]
fn guarded_playlist_clear_removes_memberships_but_retains_collection_and_other_playlist() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistClearRequest {
        playlist_index: 0,
        expected_order: vec!["100".into(), "200".into(), "300".into()],
    };
    let result = prepare_playlist_clear(source, &request).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(
        after.selected_tracks,
        preflight_export(source).unwrap().selected_tracks
    );
    assert!(after.selected_playlists[0].tracks.is_empty());
    assert_eq!(after.selected_playlists[1].tracks, ["200", "400"]);
    assert!(
        result
            .bytes
            .windows(b"Name=\"House\" Entries=\"0\"".len())
            .any(|w| w == b"Name=\"House\" Entries=\"0\"")
    );
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    let stale = PlaylistClearRequest {
        expected_order: vec!["100".into()],
        ..request
    };
    assert!(prepare_playlist_clear(source, &stale).is_err());
    let malformed = String::from_utf8(source.to_vec()).unwrap().replace(
        "Name=\"House\" Entries=\"3\"",
        "Name=\"House\" Entries=\"2\"",
    );
    assert!(
        prepare_playlist_clear(
            malformed.as_bytes(),
            &PlaylistClearRequest {
                playlist_index: 0,
                expected_order: vec!["100".into(), "200".into(), "300".into()],
            }
        )
        .is_err()
    );
}

#[test]
fn guarded_playlist_member_insert_targets_one_position_and_rejects_invalid_position() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistMemberInsertRequest {
        playlist_index: 1,
        expected_order: vec!["200".into(), "400".into()],
        track_id: "100".into(),
        position: 1,
    };
    let result = prepare_playlist_member_insert(source, &request).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_playlists[0].tracks, ["100", "200", "300"]);
    assert_eq!(after.selected_playlists[1].tracks, ["200", "100", "400"]);
    assert_eq!(
        after.selected_tracks,
        preflight_export(source).unwrap().selected_tracks
    );
    assert_eq!(
        source,
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin")
    );
    let invalid_position = PlaylistMemberInsertRequest {
        position: 3,
        ..request.clone()
    };
    assert!(prepare_playlist_member_insert(source, &invalid_position).is_err());
    let stale = PlaylistMemberInsertRequest {
        expected_order: vec!["200".into()],
        ..request.clone()
    };
    assert!(prepare_playlist_member_insert(source, &stale).is_err());
    let present = PlaylistMemberInsertRequest {
        track_id: "200".into(),
        ..request.clone()
    };
    assert!(prepare_playlist_member_insert(source, &present).is_err());
    let unknown = PlaylistMemberInsertRequest {
        track_id: "999".into(),
        ..request
    };
    assert!(prepare_playlist_member_insert(source, &unknown).is_err());
}

#[test]
fn tempo_beat_validation_matches_signed_xml_type_and_exact_four_four_range() {
    for value in [i32::MIN, -1, 0, 1, 4, i32::MAX] {
        assert_eq!(
            validate_tempo_beat("3/4", &value.to_string()).unwrap(),
            value
        );
    }
    for value in ["", "2147483648", "-2147483649", "1.5"] {
        assert!(validate_tempo_beat("3/4", value).is_err(), "{value:?}");
    }
    for value in 1..=4 {
        assert_eq!(
            validate_tempo_beat("4/4", &value.to_string()).unwrap(),
            value
        );
    }
    for value in [i32::MIN, -1, 0, 5, i32::MAX] {
        assert!(validate_tempo_beat("4/4", &value.to_string()).is_err());
    }
    // Meter matching is deliberately exact; no semantics are inferred for other text.
    assert_eq!(validate_tempo_beat(" 4/4", "-7").unwrap(), -7);
}

#[test]
fn guarded_tempo_beat_edit_changes_only_selected_battito_and_rejects_invalid_requests() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
    let request = TempoBeatEditRequest {
        track_id: "700".into(),
        tempo_index: 0,
        expected_old_beat: "1".into(),
        new_beat: "2".into(),
    };
    let result = prepare_tempo_beat_edit(source, &request).unwrap();
    let expected =
        String::from_utf8(source.to_vec())
            .unwrap()
            .replacen("Battito=\"1\"", "Battito=\"2\"", 1);
    assert_eq!(result.bytes, expected.as_bytes());
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_tracks[0].tempos[0].beat, "2");
    assert_eq!(after.selected_tracks[0].tempos[1].beat, "1");

    for invalid in [
        TempoBeatEditRequest {
            new_beat: "0".into(),
            ..request.clone()
        },
        TempoBeatEditRequest {
            new_beat: "5".into(),
            ..request.clone()
        },
        TempoBeatEditRequest {
            new_beat: "2147483648".into(),
            ..request.clone()
        },
        TempoBeatEditRequest {
            expected_old_beat: "9".into(),
            ..request.clone()
        },
        TempoBeatEditRequest {
            tempo_index: 9,
            ..request.clone()
        },
        TempoBeatEditRequest {
            new_beat: "1.5".into(),
            ..request.clone()
        },
        TempoBeatEditRequest {
            track_id: "999".into(),
            ..request
        },
    ] {
        assert!(prepare_tempo_beat_edit(source, &invalid).is_err());
    }
}

#[test]
fn guarded_tempo_meter_edit_changes_only_selected_raw_value_and_rejects_invalid_text() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
    let request = TempoMeterEditRequest {
        track_id: "700".into(),
        tempo_index: 0,
        expected_old_meter: "4/4".into(),
        new_meter: "3/4".into(),
    };
    let result = prepare_tempo_meter_edit(source, &request).unwrap();
    let expected =
        String::from_utf8(source.to_vec())
            .unwrap()
            .replacen("Metro=\"4/4\"", "Metro=\"3/4\"", 1);
    assert_eq!(result.bytes, expected.as_bytes());
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_tracks[0].tempos[0].meter, "3/4");
    assert_eq!(after.selected_tracks[0].tempos[1].meter, "4/4");

    for invalid in [
        TempoMeterEditRequest {
            expected_old_meter: "stale".into(),
            ..request.clone()
        },
        TempoMeterEditRequest {
            tempo_index: 9,
            ..request.clone()
        },
        TempoMeterEditRequest {
            new_meter: String::new(),
            ..request.clone()
        },
        TempoMeterEditRequest {
            new_meter: "bad\u{0001}".into(),
            ..request.clone()
        },
        TempoMeterEditRequest {
            track_id: "999".into(),
            ..request
        },
    ] {
        assert!(prepare_tempo_meter_edit(source, &invalid).is_err());
    }
}

#[test]
fn guarded_track_delete_removes_collection_and_exact_shared_memberships() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = TrackDeleteRequest {
        track_id: "200".into(),
        expected_memberships: vec![
            (0, vec!["100".into(), "200".into(), "300".into()]),
            (1, vec!["200".into(), "400".into()]),
        ],
    };
    let prepared = prepare_track_delete(source, &request).unwrap();
    let after = preflight_export(&prepared.bytes).unwrap();
    assert_eq!(after.selected_tracks.len(), 3);
    assert_eq!(after.selected_playlists[0].tracks, ["100", "300"]);
    assert_eq!(after.selected_playlists[1].tracks, ["400"]);
    assert_eq!(
        after
            .selected_tracks
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["100", "300", "400"]
    );
    assert_eq!(
        after
            .selected_playlists
            .iter()
            .map(|p| p.tracks.len())
            .collect::<Vec<_>>(),
        [2, 1]
    );
    let expected_bytes = String::from_utf8(source.to_vec())
        .unwrap()
        .replace("<COLLECTION Entries=\"4\">", "<COLLECTION Entries=\"3\">")
        .replace(
            "<TRACK TrackID=\"200\" Name=\"B\" Location=\"file://localhost/music/b.flac\"/>",
            "",
        )
        .replace(
            "Name=\"House\" Entries=\"3\"",
            "Name=\"House\" Entries=\"2\"",
        )
        .replace(
            "Name=\"Techno\" Entries=\"2\"",
            "Name=\"Techno\" Entries=\"1\"",
        )
        .replace("<TRACK Key=\"200\"/>", "");
    assert_eq!(prepared.bytes, expected_bytes.as_bytes());
    assert_eq!(
        prepared.bytes.iter().filter(|b| **b == b'\n').count(),
        source.iter().filter(|b| **b == b'\n').count()
    );
    for stale in [
        TrackDeleteRequest {
            expected_memberships: vec![(0, vec!["100".into(), "200".into(), "300".into()])],
            ..request.clone()
        },
        TrackDeleteRequest {
            expected_memberships: vec![
                (0, vec!["100".into(), "300".into()]),
                (1, vec!["200".into(), "400".into()]),
            ],
            ..request.clone()
        },
    ] {
        assert!(prepare_track_delete(source, &stale).is_err());
    }
}

#[test]
fn guarded_track_delete_rejects_bad_target_references_and_counts() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = TrackDeleteRequest {
        track_id: "200".into(),
        expected_memberships: vec![
            (0, vec!["100".into(), "200".into(), "300".into()]),
            (1, vec!["200".into(), "400".into()]),
        ],
    };
    let duplicate_membership = String::from_utf8(source.to_vec()).unwrap().replacen(
        "<TRACK Key=\"200\"/>",
        "<TRACK Key=\"200\"/><TRACK Key=\"200\"/>",
        1,
    );
    let duplicate_guard = TrackDeleteRequest {
        expected_memberships: vec![
            (
                0,
                vec!["100".into(), "200".into(), "200".into(), "300".into()],
            ),
            (1, vec!["200".into(), "400".into()]),
        ],
        ..request.clone()
    };
    assert!(prepare_track_delete(duplicate_membership.as_bytes(), &duplicate_guard).is_err());
    for bad in [
        String::from_utf8(source.to_vec())
            .unwrap()
            .replace("Key=\"200\"", "Key=\"999\""),
        String::from_utf8(source.to_vec())
            .unwrap()
            .replace("Key=\"200\"", "Key=\"200\" Key=\"200\""),
        String::from_utf8(source.to_vec())
            .unwrap()
            .replace("Entries=\"4\"", "Entries=\"bad\""),
    ] {
        assert!(prepare_track_delete(bad.as_bytes(), &request).is_err());
    }
}

#[test]
fn guarded_track_name_edit_changes_only_selected_attribute_and_checks_xml_text() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = TrackNameEditRequest {
        track_id: "100".into(),
        expected_old_name: "A".into(),
        new_name: "New < title".into(),
    };
    let prepared = prepare_track_name_edit(source, &request).unwrap();
    let expected = String::from_utf8(source.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"New &lt; title\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    let stale = TrackNameEditRequest {
        expected_old_name: "stale".into(),
        ..request.clone()
    };
    assert!(prepare_track_name_edit(source, &stale).is_err());
    let invalid = TrackNameEditRequest {
        new_name: "bad\u{1}".into(),
        ..request
    };
    assert!(prepare_track_name_edit(source, &invalid).is_err());
}

#[test]
fn guarded_track_genre_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Genre=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].genre.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].genre, None);
    let request = TrackGenreEditRequest {
        track_id: "100".into(),
        expected_old_genre: "old & note".into(),
        new_genre: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_genre_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Genre=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].genre.as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Genre=\"old &amp; note\"",
        "Genre=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    for request in [
        TrackGenreEditRequest {
            track_id: "200".into(),
            expected_old_genre: "".into(),
            new_genre: "x".into(),
        },
        TrackGenreEditRequest {
            track_id: "999".into(),
            expected_old_genre: "x".into(),
            new_genre: "y".into(),
        },
        TrackGenreEditRequest {
            track_id: "100".into(),
            expected_old_genre: "stale".into(),
            new_genre: "y".into(),
        },
        TrackGenreEditRequest {
            track_id: "100".into(),
            expected_old_genre: "old & note".into(),
            new_genre: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_genre_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Genre=\"old &amp; note\"", "Genre=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0].genre.as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_label_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Label=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].label.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].label, None);
    let request = TrackLabelEditRequest {
        track_id: "100".into(),
        expected_old_label: "old & note".into(),
        new_label: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_label_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Label=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].label.as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Label=\"old &amp; note\"",
        "Label=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    for request in [
        TrackLabelEditRequest {
            track_id: "200".into(),
            expected_old_label: "".into(),
            new_label: "x".into(),
        },
        TrackLabelEditRequest {
            track_id: "999".into(),
            expected_old_label: "x".into(),
            new_label: "y".into(),
        },
        TrackLabelEditRequest {
            track_id: "100".into(),
            expected_old_label: "stale".into(),
            new_label: "y".into(),
        },
        TrackLabelEditRequest {
            track_id: "100".into(),
            expected_old_label: "old & note".into(),
            new_label: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_label_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Label=\"old &amp; note\"", "Label=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0].label.as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_composer_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Composer=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].composer.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].composer, None);
    let request = TrackComposerEditRequest {
        track_id: "100".into(),
        expected_old_composer: "old & note".into(),
        new_composer: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_composer_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Composer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0]
            .composer
            .as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Composer=\"old &amp; note\"",
        "Composer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
    let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Composer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Composer') is None",
                &xml,
            ])
            .status()
            .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for request in [
        TrackComposerEditRequest {
            track_id: "200".into(),
            expected_old_composer: "".into(),
            new_composer: "x".into(),
        },
        TrackComposerEditRequest {
            track_id: "999".into(),
            expected_old_composer: "x".into(),
            new_composer: "y".into(),
        },
        TrackComposerEditRequest {
            track_id: "100".into(),
            expected_old_composer: "stale".into(),
            new_composer: "y".into(),
        },
        TrackComposerEditRequest {
            track_id: "100".into(),
            expected_old_composer: "old & note".into(),
            new_composer: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_composer_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Composer=\"old &amp; note\"", "Composer=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0]
            .composer
            .as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_grouping_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Grouping=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].grouping.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].grouping, None);
    let request = TrackGroupingEditRequest {
        track_id: "100".into(),
        expected_old_grouping: "old & note".into(),
        new_grouping: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_grouping_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Grouping=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0]
            .grouping
            .as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Grouping=\"old &amp; note\"",
        "Grouping=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
    let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Grouping')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Grouping') is None",
                &xml,
            ])
            .status()
            .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for request in [
        TrackGroupingEditRequest {
            track_id: "200".into(),
            expected_old_grouping: "".into(),
            new_grouping: "x".into(),
        },
        TrackGroupingEditRequest {
            track_id: "999".into(),
            expected_old_grouping: "x".into(),
            new_grouping: "y".into(),
        },
        TrackGroupingEditRequest {
            track_id: "100".into(),
            expected_old_grouping: "stale".into(),
            new_grouping: "y".into(),
        },
        TrackGroupingEditRequest {
            track_id: "100".into(),
            expected_old_grouping: "old & note".into(),
            new_grouping: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_grouping_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Grouping=\"old &amp; note\"", "Grouping=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0]
            .grouping
            .as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_remixer_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Remixer=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].remixer.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].remixer, None);
    let request = TrackRemixerEditRequest {
        track_id: "100".into(),
        expected_old_remixer: "old & note".into(),
        new_remixer: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_remixer_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Remixer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].remixer.as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Remixer=\"old &amp; note\"",
        "Remixer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    let xml = String::from_utf8(prepared.bytes.clone()).unwrap();
    let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Remixer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Remixer') is None",
                &xml,
            ])
            .status()
            .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for request in [
        TrackRemixerEditRequest {
            track_id: "200".into(),
            expected_old_remixer: "".into(),
            new_remixer: "x".into(),
        },
        TrackRemixerEditRequest {
            track_id: "999".into(),
            expected_old_remixer: "x".into(),
            new_remixer: "y".into(),
        },
        TrackRemixerEditRequest {
            track_id: "100".into(),
            expected_old_remixer: "stale".into(),
            new_remixer: "y".into(),
        },
        TrackRemixerEditRequest {
            track_id: "100".into(),
            expected_old_remixer: "old & note".into(),
            new_remixer: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_remixer_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Remixer=\"old &amp; note\"", "Remixer=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0]
            .remixer
            .as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_mix_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Mix=\"old &amp; note\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Mix=\"\"",
            1,
        );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].mix.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].mix.as_deref(), Some(""));
    assert_eq!(parsed.tracks[2].mix, None);

    let request = TrackMixEditRequest {
        track_id: "100".into(),
        expected_old_mix: "old & note".into(),
        new_mix: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_mix_edit(source.as_bytes(), &request).unwrap();
    let output = String::from_utf8(prepared.bytes.clone()).unwrap();
    assert!(output.contains("Mix=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\""));
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].mix.as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Mix=\"old &amp; note\"",
        "Mix=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    let decoded = std::process::Command::new("python3")
            .args([
                "-c",
                "import sys,xml.etree.ElementTree as E; r=E.fromstring(sys.argv[1]); ts={x.get('TrackID'):x for x in r.find('COLLECTION')}; assert ts['100'].get('Mix')=='A\\tB\\nC\\rD & <x>'; assert ts['200'].get('Mix')==''; assert ts['300'].get('Mix') is None",
                &output,
            ])
            .status()
            .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");

    for request in [
        TrackMixEditRequest {
            track_id: "300".into(),
            expected_old_mix: "".into(),
            new_mix: "x".into(),
        },
        TrackMixEditRequest {
            track_id: "100".into(),
            expected_old_mix: "stale".into(),
            new_mix: "x".into(),
        },
        TrackMixEditRequest {
            track_id: "100".into(),
            expected_old_mix: "old & note".into(),
            new_mix: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_mix_edit(source.as_bytes(), &request).is_err());
    }
}

#[test]
fn guarded_track_tonality_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Tonality=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].tonality.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].tonality, None);
    let request = TrackTonalityEditRequest {
        track_id: "100".into(),
        expected_old_tonality: "old & note".into(),
        new_tonality: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_tonality_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Tonality=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0]
            .tonality
            .as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Tonality=\"old &amp; note\"",
        "Tonality=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    for request in [
        TrackTonalityEditRequest {
            track_id: "200".into(),
            expected_old_tonality: "".into(),
            new_tonality: "x".into(),
        },
        TrackTonalityEditRequest {
            track_id: "999".into(),
            expected_old_tonality: "x".into(),
            new_tonality: "y".into(),
        },
        TrackTonalityEditRequest {
            track_id: "100".into(),
            expected_old_tonality: "stale".into(),
            new_tonality: "y".into(),
        },
        TrackTonalityEditRequest {
            track_id: "100".into(),
            expected_old_tonality: "old & note".into(),
            new_tonality: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_tonality_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Tonality=\"old &amp; note\"", "Tonality=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0]
            .tonality
            .as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_comments_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Comments=\"old &amp; note\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].comments.as_deref(), Some("old & note"));
    assert_eq!(parsed.tracks[1].comments, None);
    let request = TrackCommentsEditRequest {
        track_id: "100".into(),
        expected_old_comments: "old & note".into(),
        new_comments: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_comments_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Comments=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0]
            .comments
            .as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen(
        "Comments=\"old &amp; note\"",
        "Comments=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"",
        1,
    );
    assert_eq!(prepared.bytes, expected.as_bytes());
    for request in [
        TrackCommentsEditRequest {
            track_id: "200".into(),
            expected_old_comments: "".into(),
            new_comments: "x".into(),
        },
        TrackCommentsEditRequest {
            track_id: "999".into(),
            expected_old_comments: "x".into(),
            new_comments: "y".into(),
        },
        TrackCommentsEditRequest {
            track_id: "100".into(),
            expected_old_comments: "stale".into(),
            new_comments: "y".into(),
        },
        TrackCommentsEditRequest {
            track_id: "100".into(),
            expected_old_comments: "old & note".into(),
            new_comments: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_comments_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replacen("Comments=\"old &amp; note\"", "Comments=\"\"", 1);
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0]
            .comments
            .as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_album_edit_preserves_optional_state_and_xml_whitespace() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Album=\"old &amp; record\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].album.as_deref(), Some("old & record"));
    assert_eq!(parsed.tracks[1].album, None);
    let request = TrackAlbumEditRequest {
        track_id: "100".into(),
        expected_old_album: "old & record".into(),
        new_album: "A\tB\nC\rD & <x>".into(),
    };
    let prepared = prepare_track_album_edit(source.as_bytes(), &request).unwrap();
    let encoded = "Album=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains(encoded)
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].album.as_deref(),
        Some("A\tB\nC\rD & <x>")
    );
    let expected = source.replacen("Album=\"old &amp; record\"", encoded, 1);
    assert_eq!(prepared.bytes, expected.as_bytes());
    for request in [
        TrackAlbumEditRequest {
            track_id: "200".into(),
            expected_old_album: "".into(),
            new_album: "x".into(),
        },
        TrackAlbumEditRequest {
            track_id: "999".into(),
            expected_old_album: "x".into(),
            new_album: "y".into(),
        },
        TrackAlbumEditRequest {
            track_id: "100".into(),
            expected_old_album: "stale".into(),
            new_album: "y".into(),
        },
        TrackAlbumEditRequest {
            track_id: "100".into(),
            expected_old_album: "old & record".into(),
            new_album: "bad\u{1}".into(),
        },
    ] {
        assert!(prepare_track_album_edit(source.as_bytes(), &request).is_err());
    }
    let empty = source.replace("Album=\"old &amp; record\"", "Album=\"\"");
    assert_eq!(
        parse(empty.as_bytes()).unwrap().tracks[0].album.as_deref(),
        Some("")
    );
}

#[test]
fn guarded_track_artist_edit_preserves_optional_value_and_changes_only_target() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(source.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Artist=\"DJ &amp; Co\"",
        1,
    );
    let parsed = parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.tracks[0].artist.as_deref(), Some("DJ & Co"));
    assert_eq!(parsed.tracks[1].artist, None);

    let request = TrackArtistEditRequest {
        track_id: "100".into(),
        expected_old_artist: "DJ & Co".into(),
        new_artist: "New & Artist".into(),
    };
    let prepared = prepare_track_artist_edit(source.as_bytes(), &request).unwrap();
    let expected = source.replacen("Artist=\"DJ &amp; Co\"", "Artist=\"New &amp; Artist\"", 1);
    assert_eq!(prepared.bytes, expected.as_bytes());
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].artist.as_deref(),
        Some("New & Artist")
    );

    let whitespace = TrackArtistEditRequest {
        expected_old_artist: "New & Artist".into(),
        new_artist: "A\tB\nC\rD".into(),
        ..request.clone()
    };
    let prepared = prepare_track_artist_edit(&prepared.bytes, &whitespace).unwrap();
    assert!(
        String::from_utf8(prepared.bytes.clone())
            .unwrap()
            .contains("Artist=\"A&#9;B&#10;C&#13;D\"")
    );
    assert_eq!(
        parse(&prepared.bytes).unwrap().tracks[0].artist.as_deref(),
        Some("A\tB\nC\rD")
    );

    let absent = TrackArtistEditRequest {
        track_id: "200".into(),
        expected_old_artist: "".into(),
        new_artist: "New Artist".into(),
    };
    assert!(prepare_track_artist_edit(source.as_bytes(), &absent).is_err());
    let stale = TrackArtistEditRequest {
        expected_old_artist: "stale".into(),
        ..request.clone()
    };
    assert!(prepare_track_artist_edit(source.as_bytes(), &stale).is_err());
    let invalid = TrackArtistEditRequest {
        new_artist: "bad\u{1}".into(),
        ..request
    };
    assert!(prepare_track_artist_edit(source.as_bytes(), &invalid).is_err());
}

#[test]
fn export_accepts_frozen_fixtures_byte_exact_and_rejects_unknown_attributes() {
    let f02 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let f07 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin");
    let f10 = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin");
    for source in [f02.as_slice(), f07.as_slice(), f10.as_slice()] {
        assert_eq!(export_xml(source).unwrap(), source);
        let preflight = preflight_export(source).unwrap();
        assert_eq!(preflight.decision, ExportDecision::AllowExport);
        assert!(preflight.withheld_units.is_empty());
        assert!(preflight.loss_report.is_empty());
    }
    let f10_plan = preflight_export(f10).unwrap();
    assert_eq!(f10_plan.selected_tracks.len(), 1);
    assert_eq!(
        f10_plan.selected_tracks[0]
            .tempos
            .iter()
            .map(|tempo| (
                tempo.start_seconds.as_str(),
                tempo.bpm.as_str(),
                tempo.meter.as_str(),
                tempo.beat.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("0", "120", "4/4", "1"),
            ("8", "128", "4/4", "1"),
            ("16", "132", "4/4", "1")
        ]
    );
    let f02_text = String::from_utf8(f02.to_vec()).unwrap();
    let bad_folder_count = f02_text.replacen("Count=\"2\"", "Count=\"3\"", 1);
    assert_ne!(
        bad_folder_count, f02_text,
        "fixture must exercise folder Count"
    );
    assert!(export_xml(bad_folder_count.as_bytes()).is_err());
    let bad_playlist_entries = f02_text.replacen("Entries=\"3\"", "Entries=\"4\"", 1);
    assert_ne!(
        bad_playlist_entries, f02_text,
        "fixture must exercise playlist Entries"
    );
    assert!(export_xml(bad_playlist_entries.as_bytes()).is_err());
    let mutated = f02_text.replace("Name=\"A\"", "Name=\"A\" Extra=\"x\"");
    assert!(export_xml(mutated.as_bytes()).is_err());
    assert!(export_xml(b"<DJ_PLAYLISTS Version=\"1.0.0\"/>").is_err());
}

#[test]
fn rejects_invalid_required_numeric_values() {
    let invalid = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Name=\"T\" Location=\"x\"><TEMPO Inizio=\"0\" Bpm=\"fast\" Metro=\"4/4\" Battito=\"1\"/></TRACK></COLLECTION></DJ_PLAYLISTS>";
    assert!(
        parse(invalid)
            .unwrap_err()
            .to_string()
            .contains("invalid BPM")
    );
}

#[test]
fn rejects_dangling_playlist_references_and_missing_required_attributes() {
    let dangling = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION></COLLECTION><PLAYLISTS><NODE Type=\"0\" Name=\"ROOT\"><NODE Type=\"1\" Name=\"X\"><TRACK Key=\"9\"/></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>";
    assert!(
        parse(dangling)
            .unwrap_err()
            .to_string()
            .contains("dangling playlist track reference 9")
    );
    let missing = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Location=\"x\"/></COLLECTION></DJ_PLAYLISTS>";
    assert!(
        parse(missing)
            .unwrap_err()
            .to_string()
            .contains("missing required attribute Name")
    );
}

#[test]
fn parses_a1_tracks_playlists_and_preserves_ordered_points() {
    let x = parse(include_bytes!(
        "../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin"
    ))
    .unwrap();
    assert_eq!(
        x.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        ["100", "200", "300", "400"]
    );
    assert_eq!(
        x.playlists
            .iter()
            .map(|p| p.tracks.iter().map(String::as_str).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [vec!["100", "200", "300"], vec!["200", "400"]]
    );
    let t = parse(include_bytes!(
        "../../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin"
    ))
    .unwrap();
    assert_eq!(
        t.tracks[0]
            .tempos
            .iter()
            .map(|p| p.bpm.as_str())
            .collect::<Vec<_>>(),
        ["120", "128", "132"]
    );
}

#[test]
fn preview_preserves_raw_playlist_node_attributes_for_paired_and_empty_nodes() {
    let source = String::from_utf8(
        include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin").to_vec(),
    )
    .unwrap()
    .replace(
        r#"        <NODE Type="1" Name="House" Entries="3" KeyType="0">
          <TRACK Key="100"/>
          <TRACK Key="200"/>
          <TRACK Key="300"/>
        </NODE>
        <NODE Type="1" Name="Techno" Entries="2" KeyType="0">
          <TRACK Key="200"/>
          <TRACK Key="400"/>
        </NODE>"#,
        r#"        <NODE Type="1" Name="House" Entries="" KeyType="0"></NODE>
        <NODE Type="1" Name="Techno" KeyType=""/>"#,
    );

    let preview = parse(source.as_bytes()).unwrap();
    assert_eq!(preview.playlists[0].tracks, Vec::<String>::new());
    assert_eq!(preview.playlists[1].tracks, Vec::<String>::new());
    assert_eq!(preview.playlist_nodes[0].entries.as_deref(), Some(""));
    assert_eq!(preview.playlist_nodes[0].key_type.as_deref(), Some("0"));
    assert_eq!(preview.playlist_nodes[1].entries, None);
    assert_eq!(preview.playlist_nodes[1].key_type.as_deref(), Some(""));
}
#[test]
fn rejects_malformed_and_unsupported_version() {
    assert!(
        parse(b"<oops>")
            .unwrap_err()
            .to_string()
            .contains("malformed XML")
    );
    let e = parse(b"<DJ_PLAYLISTS Version=\"2.0.0\"/>").unwrap_err();
    assert!(e.to_string().contains("unsupported XML version"));
}
#[test]
fn rejects_mismatched_closing_tags() {
    let malformed = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION></PLAYLISTS>";
    assert!(
        parse(malformed)
            .unwrap_err()
            .to_string()
            .contains("malformed XML")
    );
}

#[test]
fn guarded_playlist_reorder_preserves_other_xml_and_rejects_non_permutations() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let request = PlaylistReorderRequest {
        playlist_index: 0,
        expected_order: vec!["100".into(), "200".into(), "300".into()],
        new_order: vec!["300".into(), "100".into(), "200".into()],
    };
    let changed = prepare_playlist_reorder(source, &request).unwrap().bytes;
    let parsed = parse(&changed).unwrap();
    assert_eq!(parsed.playlists[0].tracks, request.new_order);
    assert_eq!(parsed.playlists[1].tracks, vec!["200", "400"]);
    assert_eq!(parsed.tracks, parse(source).unwrap().tracks);
    let stale = PlaylistReorderRequest {
        expected_order: vec!["200".into(), "100".into(), "300".into()],
        ..request.clone()
    };
    assert!(prepare_playlist_reorder(source, &stale).is_err());
    for bad in [
        vec!["300".into(), "100".into(), "100".into()],
        vec!["300".into(), "100".into(), "999".into()],
        vec!["300".into(), "100".into()],
    ] {
        let invalid = PlaylistReorderRequest {
            new_order: bad,
            ..request.clone()
        };
        assert!(prepare_playlist_reorder(source, &invalid).is_err());
    }
}

#[test]
fn guarded_sibling_playlist_reorder_preserves_interleaved_folder_and_memberships() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec()).unwrap()
            .replace("Name=\"ROOT\" Count=\"1\"", "Name=\"ROOT\" Count=\"2\"")
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"")
            .replace("        <NODE Type=\"1\" Name=\"Techno\"", "        <NODE Type=\"0\" Name=\"KeepSlot\" Count=\"0\"/>\n        <NODE Type=\"1\" Name=\"Techno\"")
            .replace("    </NODE>\n  </PLAYLISTS>", "      <NODE Type=\"0\" Name=\"Other\" Count=\"1\"><NODE Type=\"1\" Name=\"Elsewhere\" Entries=\"0\" KeyType=\"0\"/></NODE>\n    </NODE>\n  </PLAYLISTS>");
    let original = source.clone();
    let request = SiblingPlaylistReorderRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        expected_order: vec!["House".into(), "Techno".into()],
        new_order: vec!["Techno".into(), "House".into()],
    };
    let result = prepare_sibling_playlist_reorder(source.as_bytes(), &request).unwrap();
    assert_eq!(source, original);
    let output = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(output.contains("<NODE Type=\"0\" Name=\"KeepSlot\" Count=\"0\"/>"));
    let parsed = preflight_export(&result.bytes).unwrap();
    assert_eq!(
        parsed.selected_tracks,
        preflight_export(source.as_bytes()).unwrap().selected_tracks
    );
    assert_eq!(
        parsed
            .selected_playlists
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["Techno", "House", "Elsewhere"]
    );
    assert_eq!(parsed.selected_playlists[0].tracks, vec!["200", "400"]);
    assert_eq!(
        parsed.selected_playlists[1].tracks,
        vec!["100", "200", "300"]
    );

    let stale = SiblingPlaylistReorderRequest {
        expected_order: vec!["Techno".into(), "House".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_playlist_reorder(source.as_bytes(), &stale).is_err());
    let duplicate = SiblingPlaylistReorderRequest {
        new_order: vec!["House".into(), "House".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_playlist_reorder(source.as_bytes(), &duplicate).is_err());
    let duplicate_names = source.replace("Name=\"Techno\"", "Name=\"House\"");
    let ambiguous = SiblingPlaylistReorderRequest {
        expected_order: vec!["House".into(), "House".into()],
        new_order: vec!["House".into(), "House".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_playlist_reorder(duplicate_names.as_bytes(), &ambiguous).is_err());
    for invalid in [
        SiblingPlaylistReorderRequest {
            parent_index: 99,
            ..request.clone()
        },
        SiblingPlaylistReorderRequest {
            parent_index: 0,
            expected_parent_name: "ROOT".into(),
            ..request.clone()
        },
    ] {
        assert!(prepare_sibling_playlist_reorder(source.as_bytes(), &invalid).is_err());
    }
    assert!(prepare_sibling_playlist_reorder(b"<bad", &request).is_err());
}

#[test]
fn guarded_folder_move_preserves_subtree_and_updates_parent_counts() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"")
            .replace(
                "        <NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>",
                "        <NODE Type=\"0\" Name=\"MoveMe\" Count=\"1\"><NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\"><TRACK Key=\"100\"/><TRACK Key=\"200\"/><TRACK Key=\"300\"/></NODE></NODE>",
            )
            .replace(
                "      </NODE>\n    </NODE>\n  </PLAYLISTS>",
                "        <NODE Type=\"0\" Name=\"Target\" Count=\"0\"/>\n      </NODE>\n    </NODE>\n  </PLAYLISTS>",
            );
    let request = FolderMoveRequest {
        folder_index: 1,
        expected_folder_name: "MoveMe".into(),
        source_parent_index: 0,
        expected_source_parent_name: "Sets".into(),
        expected_source_order: vec!["0:MoveMe".into(), "1:Techno".into(), "0:Target".into()],
        target_parent_index: 2,
        expected_target_parent_name: "Target".into(),
        expected_target_order: vec![],
    };
    let original = source.clone();
    let result = prepare_folder_move(source.as_bytes(), &request).unwrap();
    assert_eq!(source, original);
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_tracks.len(), 4);
    let house = after
        .selected_playlists
        .iter()
        .find(|playlist| playlist.name == "House")
        .unwrap();
    assert_eq!(house.path, ["Sets", "Target", "MoveMe", "House"]);
    assert_eq!(house.tracks, ["100", "200", "300"]);
    let techno = after
        .selected_playlists
        .iter()
        .find(|playlist| playlist.name == "Techno")
        .unwrap();
    assert_eq!(techno.path, ["Sets", "Techno"]);
    let output = String::from_utf8(result.bytes).unwrap();
    assert!(output.contains("Name=\"Sets\" Count=\"2\""));
    assert!(output.contains("Name=\"Target\" Count=\"1\">"));
    assert!(output.contains("Name=\"MoveMe\" Count=\"1\"><NODE Type=\"1\" Name=\"House\""));

    let same_parent = FolderMoveRequest {
        target_parent_index: 0,
        expected_target_parent_name: "Sets".into(),
        expected_target_order: request.expected_source_order.clone(),
        ..request.clone()
    };
    assert!(prepare_folder_move(source.as_bytes(), &same_parent).is_err());
    let cycle = FolderMoveRequest {
        target_parent_index: 1,
        expected_target_parent_name: "MoveMe".into(),
        expected_target_order: vec!["1:House".into()],
        ..request.clone()
    };
    assert!(prepare_folder_move(source.as_bytes(), &cycle).is_err());
    let stale = FolderMoveRequest {
        expected_target_order: vec!["stale".into()],
        ..request
    };
    assert!(prepare_folder_move(source.as_bytes(), &stale).is_err());
    assert!(prepare_folder_move(b"<bad", &same_parent).is_err());
}

#[test]
fn guarded_sibling_folder_reorder_preserves_interleaved_playlists_and_subtrees() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec()).unwrap()
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"4\"")
            .replace("        <NODE Type=\"1\" Name=\"Techno\"", "        <NODE Type=\"0\" Name=\"FolderA\" Count=\"1\"><NODE Type=\"1\" Name=\"A-child\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"100\"/></NODE></NODE>\n        <NODE Type=\"1\" Name=\"Techno\"")
            .replace("      </NODE>\n    </NODE>\n  </PLAYLISTS>", "        <NODE Type=\"0\" Name=\"FolderB\" Count=\"1\"><NODE Type=\"1\" Name=\"B-child\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"200\"/></NODE></NODE>\n      </NODE>\n    </NODE>\n  </PLAYLISTS>");
    let original = source.clone();
    let request = SiblingFolderReorderRequest {
        parent_index: 0,
        expected_parent_name: "Sets".into(),
        expected_order: vec!["FolderA".into(), "FolderB".into()],
        new_order: vec!["FolderB".into(), "FolderA".into()],
    };
    let result = prepare_sibling_folder_reorder(source.as_bytes(), &request).unwrap();
    assert_eq!(source, original);
    let output = String::from_utf8(result.bytes.clone()).unwrap();
    assert!(output.find("Name=\"FolderB\"").unwrap() < output.find("Name=\"FolderA\"").unwrap());
    assert!(output.find("Name=\"House\"").unwrap() < output.find("Name=\"Techno\"").unwrap());
    assert!(output.contains("Name=\"Sets\" Count=\"4\""));
    let before = preflight_export(source.as_bytes()).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_tracks, before.selected_tracks);
    let mut before_playlists: Vec<_> = before
        .selected_playlists
        .iter()
        .map(|p| (p.path.clone(), p.name.clone(), p.tracks.clone()))
        .collect();
    let mut after_playlists: Vec<_> = after
        .selected_playlists
        .iter()
        .map(|p| (p.path.clone(), p.name.clone(), p.tracks.clone()))
        .collect();
    before_playlists.sort_unstable();
    after_playlists.sort_unstable();
    assert_eq!(after_playlists, before_playlists);

    let stale = SiblingFolderReorderRequest {
        expected_order: vec!["FolderB".into(), "FolderA".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_folder_reorder(source.as_bytes(), &stale).is_err());
    let invalid = SiblingFolderReorderRequest {
        new_order: vec!["FolderA".into(), "FolderA".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_folder_reorder(source.as_bytes(), &invalid).is_err());
    let duplicate = source.replace("Name=\"FolderB\"", "Name=\"FolderA\"");
    let ambiguous = SiblingFolderReorderRequest {
        expected_order: vec!["FolderA".into(), "FolderA".into()],
        new_order: vec!["FolderA".into(), "FolderA".into()],
        ..request.clone()
    };
    assert!(prepare_sibling_folder_reorder(duplicate.as_bytes(), &ambiguous).is_err());
    for invalid in [
        SiblingFolderReorderRequest {
            parent_index: 99,
            ..request.clone()
        },
        SiblingFolderReorderRequest {
            expected_parent_name: "ROOT".into(),
            ..request.clone()
        },
    ] {
        assert!(prepare_sibling_folder_reorder(source.as_bytes(), &invalid).is_err());
    }
    let empty_parent = b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION Entries=\"0\"></COLLECTION><PLAYLISTS><NODE Type=\"0\" Name=\"ROOT\" Count=\"1\"><NODE Type=\"0\" Name=\"Sets\" Count=\"0\"></NODE></NODE></PLAYLISTS></DJ_PLAYLISTS>";
    assert!(
        prepare_sibling_folder_reorder(
            empty_parent,
            &SiblingFolderReorderRequest {
                parent_index: 0,
                expected_parent_name: "Sets".into(),
                expected_order: vec![],
                new_order: vec![]
            }
        )
        .is_err()
    );
    assert!(prepare_sibling_folder_reorder(b"<bad", &request).is_err());
}

#[test]
fn guarded_playlist_move_updates_both_folder_counts_and_preserves_memberships() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace("Name=\"ROOT\" Count=\"1\"", "Name=\"ROOT\" Count=\"3\"")
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Source\" Count=\"2\"")
            .replace(
                "    <NODE Type=\"0\" Name=\"Source\"",
                "    <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n    <NODE Type=\"0\" Name=\"Source\"",
            )
            .replace(
                "      </NODE>\n    </NODE>\n  </PLAYLISTS>",
                "      </NODE>\n      <NODE Type=\"0\" Name=\"Target\" Count=\"1\"><NODE Type=\"1\" Name=\"Other\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"400\"/></NODE></NODE>\n    </NODE>\n  </PLAYLISTS>",
            );
    let original = source.clone();
    let request = PlaylistMoveRequest {
        playlist_index: 0,
        expected_playlist_name: "House".into(),
        expected_members: vec!["100".into(), "200".into(), "300".into()],
        source_parent_index: 1,
        expected_source_parent_name: "Source".into(),
        target_parent_index: 2,
        expected_target_parent_name: "Target".into(),
        expected_target_order: vec!["Other".into()],
    };
    let result = prepare_playlist_move(source.as_bytes(), &request).unwrap();
    assert_eq!(source, original);
    let before = preflight_export(source.as_bytes()).unwrap();
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_tracks, before.selected_tracks);
    assert_eq!(
        after
            .selected_playlists
            .iter()
            .map(|p| (p.path.clone(), p.name.clone(), p.tracks.clone()))
            .collect::<Vec<_>>(),
        [
            (
                vec!["Source".into(), "Techno".into()],
                "Techno".into(),
                vec!["200".into(), "400".into()]
            ),
            (
                vec!["Target".into(), "Other".into()],
                "Other".into(),
                vec!["400".into()]
            ),
            (
                vec!["Target".into(), "House".into()],
                "House".into(),
                vec!["100".into(), "200".into(), "300".into()]
            ),
        ]
    );
    let output = String::from_utf8(result.bytes).unwrap();
    assert!(output.contains("Name=\"Source\" Count=\"1\""));
    assert!(output.contains("Name=\"Target\" Count=\"2\""));

    for invalid in [
        PlaylistMoveRequest {
            expected_target_order: vec!["stale".into()],
            ..request.clone()
        },
        PlaylistMoveRequest {
            expected_members: vec!["300".into(), "200".into(), "100".into()],
            ..request.clone()
        },
        PlaylistMoveRequest {
            target_parent_index: request.source_parent_index,
            expected_target_parent_name: request.expected_source_parent_name.clone(),
            ..request.clone()
        },
        PlaylistMoveRequest {
            playlist_index: 1,
            ..request.clone()
        },
    ] {
        assert!(prepare_playlist_move(source.as_bytes(), &invalid).is_err());
    }
    let duplicate_target = source.replace("Name=\"Other\"", "Name=\"House\"");
    let ambiguous = PlaylistMoveRequest {
        expected_target_order: vec!["House".into()],
        ..request
    };
    assert!(prepare_playlist_move(duplicate_target.as_bytes(), &ambiguous).is_err());
    assert!(prepare_playlist_move(b"<bad", &ambiguous).is_err());
}

#[test]
fn guarded_playlist_move_handles_empty_playlist_and_self_closing_target() {
    let fixture = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(fixture.to_vec())
            .unwrap()
            .replace("Name=\"ROOT\" Count=\"1\"", "Name=\"ROOT\" Count=\"2\"")
            .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Source\" Count=\"2\"")
            .replace(
                "        <NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>",
                "        <NODE Type=\"1\" Name=\"House\" Entries=\"0\" KeyType=\"0\"/>",
            )
            .replace(
                "      </NODE>\n    </NODE>\n  </PLAYLISTS>",
                "      </NODE>\n      <NODE Type=\"0\" Name=\"Target\" Count=\"0\"/>\n    </NODE>\n  </PLAYLISTS>",
            );
    let request = PlaylistMoveRequest {
        playlist_index: 0,
        expected_playlist_name: "House".into(),
        expected_members: vec![],
        source_parent_index: 0,
        expected_source_parent_name: "Source".into(),
        target_parent_index: 1,
        expected_target_parent_name: "Target".into(),
        expected_target_order: vec![],
    };
    let original = source.clone();
    let result = prepare_playlist_move(source.as_bytes(), &request).unwrap();
    assert_eq!(source, original);
    assert!(
        preflight_export(source.as_bytes())
            .unwrap()
            .selected_playlists[0]
            .tracks
            .is_empty()
    );
    let after = preflight_export(&result.bytes).unwrap();
    assert_eq!(after.selected_playlists[0].name, "Techno");
    assert_eq!(after.selected_playlists[1].name, "House");
    assert_eq!(after.selected_playlists[1].path, ["Target", "House"]);
    let output = String::from_utf8(result.bytes).unwrap();
    assert!(output.contains("Name=\"Source\" Count=\"1\""));
    assert!(output.contains("Name=\"Target\" Count=\"1\">"));
    assert_eq!(
        preflight_export(source.as_bytes())
            .unwrap()
            .selected_playlists[1]
            .name,
        "Techno"
    );
}

#[test]
fn guarded_reorder_distinguishes_duplicate_playlist_names_by_node_identity() {
    let source = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let mut text = String::from_utf8(source.to_vec()).unwrap();
    let duplicate_branch = "      <NODE Type=\"0\" Name=\"Sets\" Count=\"1\">\n        <NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>\n      </NODE>\n";
    text = text.replace(
        "<NODE Type=\"0\" Name=\"ROOT\" Count=\"1\">",
        "<NODE Type=\"0\" Name=\"ROOT\" Count=\"2\">",
    );
    text = text.replace(
        "    </NODE>\n  </PLAYLISTS>",
        &format!("{duplicate_branch}    </NODE>\n  </PLAYLISTS>"),
    );
    let request = PlaylistReorderRequest {
        playlist_index: 0,
        expected_order: vec!["100".into(), "200".into(), "300".into()],
        new_order: vec!["300".into(), "100".into(), "200".into()],
    };
    let changed = prepare_playlist_reorder(text.as_bytes(), &request)
        .unwrap()
        .bytes;
    let parsed = parse(&changed).unwrap();
    assert_eq!(parsed.playlists[0].tracks, request.new_order);
    assert_eq!(parsed.playlists[1].tracks, vec!["200", "400"]);
    assert_eq!(parsed.playlists[2].tracks, vec!["100", "200", "300"]);
    assert!(
        String::from_utf8(changed)
            .unwrap()
            .contains(duplicate_branch)
    );
}

#[test]
fn guarded_track_year_edit_preserves_raw_value_and_rejects_stale_absent_invalid() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Year=\"20&amp;24\"",
        1,
    );
    let source = source.replacen(
        "TrackID=\"200\" Name=\"B\"",
        "TrackID=\"200\" Name=\"B\" Year=\"\"",
        1,
    );

    let request = TrackYearEditRequest {
        track_id: "100".into(),
        expected_old_year: "20&24".into(),
        new_year: "raw & <year>\t\n".into(),
    };
    let edited = prepare_track_year_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(edited.bytes.clone())
            .unwrap()
            .contains("Year=\"raw &amp; &lt;year&gt;&#9;&#10;\"")
    );
    assert_eq!(
        parse(&edited.bytes).unwrap().tracks[0].year.as_deref(),
        Some("raw & <year>\t\n")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[1].year.as_deref(),
        Some("")
    );
    assert_eq!(parse(source.as_bytes()).unwrap().tracks[2].year, None);
    for (id, old, new) in [
        ("100", "stale", "x"),
        ("300", "", "x"),
        ("999", "x", "y"),
        ("100", "20&24", "bad\u{1}"),
    ] {
        assert!(
            prepare_track_year_edit(
                source.as_bytes(),
                &TrackYearEditRequest {
                    track_id: id.into(),
                    expected_old_year: old.into(),
                    new_year: new.into()
                }
            )
            .is_err()
        );
    }
    assert!(prepare_track_year_edit(b"<bad", &request).is_err());
}

#[test]
fn guarded_track_date_added_edit_preserves_raw_value_and_rejects_stale_absent_invalid() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" DateAdded=\"20&amp;24\"",
        1,
    );
    let source = source.replacen(
        "TrackID=\"200\" Name=\"B\"",
        "TrackID=\"200\" Name=\"B\" DateAdded=\"\"",
        1,
    );

    let request = TrackDateAddedEditRequest {
        track_id: "100".into(),
        expected_old_date_added: "20&24".into(),
        new_date_added: "raw & <date_added>\t\n".into(),
    };
    let edited = prepare_track_date_added_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(edited.bytes.clone())
            .unwrap()
            .contains("DateAdded=\"raw &amp; &lt;date_added&gt;&#9;&#10;\"")
    );
    assert_eq!(
        parse(&edited.bytes).unwrap().tracks[0]
            .date_added
            .as_deref(),
        Some("raw & <date_added>\t\n")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[1]
            .date_added
            .as_deref(),
        Some("")
    );
    assert_eq!(parse(source.as_bytes()).unwrap().tracks[2].date_added, None);
    for (id, old, new) in [
        ("100", "stale", "x"),
        ("300", "", "x"),
        ("999", "x", "y"),
        ("100", "20&24", "bad\u{1}"),
    ] {
        assert!(
            prepare_track_date_added_edit(
                source.as_bytes(),
                &TrackDateAddedEditRequest {
                    track_id: id.into(),
                    expected_old_date_added: old.into(),
                    new_date_added: new.into()
                }
            )
            .is_err()
        );
    }
    assert!(prepare_track_date_added_edit(b"<bad", &request).is_err());
}

#[test]
fn guarded_track_date_modified_edit_preserves_raw_value_and_rejects_stale_absent_invalid() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" DateModified=\"20&amp;24\"",
        1,
    );
    let source = source.replacen(
        "TrackID=\"200\" Name=\"B\"",
        "TrackID=\"200\" Name=\"B\" DateModified=\"\"",
        1,
    );

    let request = TrackDateModifiedEditRequest {
        track_id: "100".into(),
        expected_old_date_modified: "20&24".into(),
        new_date_modified: "raw & <date_modified>\t\n".into(),
    };
    let edited = prepare_track_date_modified_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(edited.bytes.clone())
            .unwrap()
            .contains("DateModified=\"raw &amp; &lt;date_modified&gt;&#9;&#10;\"")
    );
    assert_eq!(
        parse(&edited.bytes).unwrap().tracks[0]
            .date_modified
            .as_deref(),
        Some("raw & <date_modified>\t\n")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[1]
            .date_modified
            .as_deref(),
        Some("")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[2].date_modified,
        None
    );
    for (id, old, new) in [
        ("100", "stale", "x"),
        ("300", "", "x"),
        ("999", "x", "y"),
        ("100", "20&24", "bad\u{1}"),
    ] {
        assert!(
            prepare_track_date_modified_edit(
                source.as_bytes(),
                &TrackDateModifiedEditRequest {
                    track_id: id.into(),
                    expected_old_date_modified: old.into(),
                    new_date_modified: new.into()
                }
            )
            .is_err()
        );
    }
    assert!(prepare_track_date_modified_edit(b"<bad", &request).is_err());
}

#[test]
fn guarded_track_last_played_edit_preserves_raw_value_and_rejects_stale_absent_invalid() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" LastPlayed=\"20&amp;24\"",
        1,
    );
    let source = source.replacen(
        "TrackID=\"200\" Name=\"B\"",
        "TrackID=\"200\" Name=\"B\" LastPlayed=\"\"",
        1,
    );

    let request = TrackLastPlayedEditRequest {
        track_id: "100".into(),
        expected_old_last_played: "20&24".into(),
        new_last_played: "raw & <last_played>\t\n".into(),
    };
    let edited = prepare_track_last_played_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(edited.bytes.clone())
            .unwrap()
            .contains("LastPlayed=\"raw &amp; &lt;last_played&gt;&#9;&#10;\"")
    );
    assert_eq!(
        parse(&edited.bytes).unwrap().tracks[0]
            .last_played
            .as_deref(),
        Some("raw & <last_played>\t\n")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[1]
            .last_played
            .as_deref(),
        Some("")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[2].last_played,
        None
    );
    for (id, old, new) in [
        ("100", "stale", "x"),
        ("300", "", "x"),
        ("999", "x", "y"),
        ("100", "20&24", "bad\u{1}"),
    ] {
        assert!(
            prepare_track_last_played_edit(
                source.as_bytes(),
                &TrackLastPlayedEditRequest {
                    track_id: id.into(),
                    expected_old_last_played: old.into(),
                    new_last_played: new.into()
                }
            )
            .is_err()
        );
    }
    assert!(prepare_track_last_played_edit(b"<bad", &request).is_err());
}

#[test]
fn guarded_track_kind_edit_preserves_raw_value_and_rejects_stale_absent_invalid() {
    let base = include_bytes!("../../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin");
    let source = String::from_utf8(base.to_vec()).unwrap().replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Kind=\"AIFF\"",
        1,
    );
    let source = source.replacen(
        "TrackID=\"200\" Name=\"B\"",
        "TrackID=\"200\" Name=\"B\" Kind=\"\"",
        1,
    );

    let request = TrackKindEditRequest {
        track_id: "100".into(),
        expected_old_kind: "AIFF".into(),
        new_kind: "raw & <kind>\t\n".into(),
    };
    let edited = prepare_track_kind_edit(source.as_bytes(), &request).unwrap();
    assert!(
        String::from_utf8(edited.bytes.clone())
            .unwrap()
            .contains("Kind=\"raw &amp; &lt;kind&gt;&#9;&#10;\"")
    );
    assert_eq!(
        parse(&edited.bytes).unwrap().tracks[0].kind.as_deref(),
        Some("raw & <kind>\t\n")
    );
    assert_eq!(
        parse(source.as_bytes()).unwrap().tracks[1].kind.as_deref(),
        Some("")
    );
    assert_eq!(parse(source.as_bytes()).unwrap().tracks[2].kind, None);
    for (id, old, new) in [
        ("100", "stale", "x"),
        ("300", "", "x"),
        ("999", "x", "y"),
        ("100", "AIFF", "bad\u{1}"),
    ] {
        assert!(
            prepare_track_kind_edit(
                source.as_bytes(),
                &TrackKindEditRequest {
                    track_id: id.into(),
                    expected_old_kind: old.into(),
                    new_kind: new.into()
                }
            )
            .is_err()
        );
    }
    assert!(prepare_track_kind_edit(b"<bad", &request).is_err());
}

#[test]
fn pdb_preview_fails_closed_and_never_mutates_source_bytes() {
    let malformed = b"not a DeviceSQL database";
    let original = malformed.to_vec();
    assert!(parse_pdb(malformed).is_err());
    assert_eq!(malformed, original.as_slice());

    let mut empty = vec![0u8; 8192];
    empty[4..8].copy_from_slice(&4096u32.to_le_bytes());
    empty[8..12].copy_from_slice(&1u32.to_le_bytes());
    empty[0x1c..0x20].copy_from_slice(&0u32.to_le_bytes());
    empty[0x24..0x28].copy_from_slice(&1u32.to_le_bytes());
    empty[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
    let original = empty.clone();
    let preview = parse_pdb(&empty).unwrap();
    assert!(preview.tracks.is_empty());
    assert!(preview.playlists.is_empty());
    assert_eq!(empty, original);
}

#[test]
fn rejects_duplicate_track_ids() {
    let x=b"<DJ_PLAYLISTS Version=\"1.0.0\"><COLLECTION><TRACK TrackID=\"1\" Name=\"a\" Location=\"x\"/><TRACK TrackID=\"1\" Name=\"b\" Location=\"y\"/></COLLECTION></DJ_PLAYLISTS>";
    assert!(
        parse(x)
            .unwrap_err()
            .to_string()
            .contains("duplicate track identity")
    );
}
