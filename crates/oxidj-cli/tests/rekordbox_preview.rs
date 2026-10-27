use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use std::{fs, process::Command, time::SystemTime};

#[test]
fn preview_and_export_plan_report_track_kind_states_and_export_preserves_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Kind=\"AIFF\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Kind=\"\"",
        )
        .into_bytes();
    let input = temporary_path("kind-input");
    let output = temporary_path("kind-output");
    fs::write(&input, &source).unwrap();
    for args in [
        vec!["rekordbox", "preview", "--xml", input.to_str().unwrap()],
        vec!["rekordbox", "export-plan", "--xml", input.to_str().unwrap()],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(stdout.contains("AIFF"));
        assert!(stdout.contains("Some(\"\")"));
        assert!(stdout.contains("None"));
    }
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(&output).unwrap(), source);
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn preview_and_export_plan_report_track_year_states_and_export_preserves_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Year=\"2024\" Size=\"9223372036854775807\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Year=\"\" Size=\"\"",
        )
        .into_bytes();
    let input = temporary_path("year-input");
    let output = temporary_path("year-output");
    fs::write(&input, &source).unwrap();
    for args in [
        vec!["rekordbox", "preview", "--xml", input.to_str().unwrap()],
        vec!["rekordbox", "export-plan", "--xml", input.to_str().unwrap()],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        for (track_id, expected_size) in [
            ("100", "Some(\"9223372036854775807\")"),
            ("200", "Some(\"\")"),
            ("300", "None"),
        ] {
            let row = stdout
                .lines()
                .find(|line| {
                    line.starts_with(&format!("{track_id}\t"))
                        || line.starts_with(&format!("Track\t{track_id}\t"))
                })
                .unwrap_or_else(|| panic!("missing TrackID {track_id} row in:\\n{stdout}"));
            assert_eq!(row.split('\t').next_back(), Some(expected_size), "{row}");
        }
    }
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(&output).unwrap(), source);
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn preview_prints_tracks_ordered_playlists_tempos_and_marks() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", &fixture])
        .output()
        .expect("run CLI");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(
        "700\tTempo cue independence\tNone\tNone\tNone\tfile://localhost/music/tempo.flac"
    ));
    assert!(stdout.contains("TEMPO\t10\t128\t4/4\t1"));
    assert!(stdout.contains("POSITION_MARK\tIntro\t0\t5\t0\tNone"));
    assert!(stdout.contains("Playlists:"));
}

#[test]
fn preview_reports_unsupported_content_as_warning() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let xml = fs::read_to_string(fixture)
        .expect("read synthetic fixture")
        .replace("<PRODUCT", "<EXPERIMENTAL_FEATURE/>\n  <PRODUCT");
    let path = temporary_path("preview");
    fs::write(&path, xml).expect("write temporary synthetic XML");
    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", path.to_str().unwrap()])
        .output()
        .expect("run CLI");
    let _ = fs::remove_file(path);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("warning: unsupported content: EXPERIMENTAL_FEATURE")
    );
}

#[test]
fn export_preserves_bytes_and_refuses_source_alias_existing_and_invalid_outputs() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let output = temporary_path("export");
    let run = |input: &str, target: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args(["rekordbox", "export", "--xml", input, "--output", target])
            .output()
            .unwrap()
    };
    let out = run(&fixture, output.to_str().unwrap());
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(&output).unwrap(), source);
    assert_eq!(fs::read(&fixture).unwrap(), source);

    let f07 = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let f07_source = fs::read(&f07).unwrap();
    let f07_output = temporary_path("export-f07");
    let f07_result = run(&f07, f07_output.to_str().unwrap());
    assert_eq!(
        f07_result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&f07_result.stderr)
    );
    assert_eq!(fs::read(&f07_output).unwrap(), f07_source);
    assert_eq!(fs::read(&f07).unwrap(), f07_source);
    assert_ne!(run(&fixture, &fixture).status.code(), Some(0));
    assert_eq!(fs::read(&fixture).unwrap(), source);
    assert_ne!(
        run(&fixture, output.to_str().unwrap()).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&output).unwrap(), source);
    let invalid = temporary_path("invalid");
    let bad = temporary_path("bad-output");
    fs::write(&invalid, b"<DJ_PLAYLISTS Version=\"1.0.0\"/>").unwrap();
    assert_ne!(
        run(invalid.to_str().unwrap(), bad.to_str().unwrap())
            .status
            .code(),
        Some(0)
    );
    assert!(!bad.exists());
    let _ = fs::remove_file(invalid);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(f07_output);
}

#[test]
fn export_plan_shows_ordered_f10_units_without_writing() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("export-plan");
    let plan_path = input.with_extension("plan");
    fs::write(&input, &source).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .expect("run CLI");

    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        concat!(
            "Decision: ALLOW_EXPORT\n",
            "Selected units:\n",
            "Track\t1000\tVariable tempo\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tfile://localhost/music/variable.flac\tNone\tNone\tNone\n",
            "TEMPO\t1000\t0\t120\t4/4\t1\n",
            "TEMPO\t1000\t8\t128\t4/4\t1\n",
            "TEMPO\t1000\t16\t132\t7/8\t3\n",
            "Withheld units: 0\n",
            "Loss report: []\n"
        )
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    assert!(!plan_path.exists());
    let _ = fs::remove_file(input);
}

#[test]
fn export_plan_accepts_existing_f02_and_f07_subsets() {
    for case in ["F-02", "F-07"] {
        let fixture = format!(
            "{}/../../xtask/tests/fixtures/rekordbox-a1/{case}/source.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let source = fs::read(&fixture).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args(["rekordbox", "export-plan", "--xml", &fixture])
            .output()
            .expect("run CLI");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{case}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8(out.stdout)
                .unwrap()
                .starts_with("Decision: ALLOW_EXPORT\n")
        );
        assert_eq!(fs::read(&fixture).unwrap(), source);
    }
}

#[test]
fn export_plan_rejects_unsupported_input_without_changing_source() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-10/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let xml = fs::read_to_string(fixture)
        .unwrap()
        .replace("TrackID=\"1000\"", "TrackID=\"1000\" Extra=\"x\"");
    let input = temporary_path("export-plan-rejected");
    fs::write(&input, &xml).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .expect("run CLI");

    assert_ne!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("unsupported TRACK attribute Extra")
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), xml);
    let _ = fs::remove_file(input);
}

#[test]
fn tempo_edit_exports_one_bpm_change_and_preserves_all_other_source_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("tempo-edit-input");
    let output = temporary_path("tempo-edit-output");
    fs::write(&input, &source).unwrap();
    assert!(!output.exists());

    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "0",
            "--expected-bpm",
            "120",
            "--bpm",
            "125",
        ])
        .output()
        .expect("run CLI");

    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected =
        String::from_utf8(source.clone())
            .unwrap()
            .replacen("Bpm=\"120\"", "Bpm=\"125\"", 1);
    assert_eq!(fs::read_to_string(&output).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn tempo_start_edit_exports_one_guarded_inizio_change_and_preserves_source() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("tempo-start-input");
    let output = temporary_path("tempo-start-output");
    fs::write(&input, &source).unwrap();
    assert!(!output.exists());

    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo-start",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "1",
            "--expected-start",
            "10",
            "--start",
            "12.5",
        ])
        .output()
        .expect("run CLI");

    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected =
        String::from_utf8(source.clone())
            .unwrap()
            .replacen("Inizio=\"10\"", "Inizio=\"12.5\"", 1);
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let existing = temporary_path("tempo-start-existing");
    fs::write(&existing, b"keep this file").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo-start",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            existing.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "1",
            "--expected-start",
            "10",
            "--start",
            "12.5",
        ])
        .output()
        .expect("run CLI");
    assert_ne!(rejected.status.code(), Some(0));
    assert_eq!(fs::read(&existing).unwrap(), b"keep this file");

    for (index, expected_start, new_start) in
        [("1", "9", "12"), ("1", "10", "NaN"), ("8", "10", "12")]
    {
        let rejected = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-tempo-start",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--track-id",
                "700",
                "--tempo-index",
                index,
                "--expected-start",
                expected_start,
                "--start",
                new_start,
            ])
            .output()
            .expect("run CLI");
        assert_ne!(rejected.status.code(), Some(0));
        assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
        assert_eq!(fs::read(&input).unwrap(), source);
    }

    let alias = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo-start",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            input.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "1",
            "--expected-start",
            "10",
            "--start",
            "12.5",
        ])
        .output()
        .expect("run CLI");
    assert_ne!(alias.status.code(), Some(0));
    assert_eq!(fs::read(&input).unwrap(), source);

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(existing);
}

#[test]
fn tempo_beat_edit_exports_one_guarded_battito_change_without_other_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("tempo-beat-input");
    let output = temporary_path("tempo-beat-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, index: &str, expected: &str, beat: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-tempo-beat",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "700",
                "--tempo-index",
                index,
                "--expected-beat",
                expected,
                "--beat",
                beat,
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "0", "1", "2");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected =
        String::from_utf8(source.clone())
            .unwrap()
            .replacen("Battito=\"1\"", "Battito=\"2\"", 1);
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let existing = temporary_path("tempo-beat-existing");
    fs::write(&existing, b"keep this file").unwrap();
    assert_ne!(invoke(&existing, "0", "1", "2").status.code(), Some(0));
    assert_eq!(fs::read(&existing).unwrap(), b"keep this file");
    assert_ne!(invoke(&input, "0", "1", "2").status.code(), Some(0));
    for (label, index, expected_beat, new_beat) in [
        ("stale", "0", "9", "2"),
        ("missing", "9", "1", "2"),
        ("malformed", "0", "1", "1.5"),
    ] {
        let rejected = temporary_path(&format!("tempo-beat-{label}"));
        assert_ne!(
            invoke(&rejected, index, expected_beat, new_beat)
                .status
                .code(),
            Some(0)
        );
        assert!(!rejected.exists());
    }
    let unsupported_input = temporary_path("tempo-beat-unsupported-input");
    let unsupported = String::from_utf8(source.clone())
        .unwrap()
        .replace("TrackID=\"700\"", "TrackID=\"700\" Extra=\"x\"");
    fs::write(&unsupported_input, unsupported).unwrap();
    let unsupported_output = temporary_path("tempo-beat-unsupported-output");
    let unsupported_result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo-beat",
            "--xml",
            unsupported_input.to_str().unwrap(),
            "--output",
            unsupported_output.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "0",
            "--expected-beat",
            "1",
            "--beat",
            "2",
        ])
        .output()
        .unwrap();
    assert_ne!(unsupported_result.status.code(), Some(0));
    assert!(!unsupported_output.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(existing);
    let _ = fs::remove_file(unsupported_input);
}

#[test]
fn tempo_meter_edit_exports_one_guarded_raw_metro_change_without_other_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("tempo-meter-input");
    let output = temporary_path("tempo-meter-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, index: &str, expected: &str, meter: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-tempo-meter",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "700",
                "--tempo-index",
                index,
                "--expected-meter",
                expected,
                "--meter",
                meter,
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "0", "4/4", "3/4");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected =
        String::from_utf8(source.clone())
            .unwrap()
            .replacen("Metro=\"4/4\"", "Metro=\"3/4\"", 1);
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let existing = temporary_path("tempo-meter-existing");
    fs::write(&existing, b"keep this file").unwrap();
    assert_ne!(invoke(&existing, "0", "4/4", "3/4").status.code(), Some(0));
    assert_eq!(fs::read(&existing).unwrap(), b"keep this file");
    assert_ne!(invoke(&input, "0", "4/4", "3/4").status.code(), Some(0));
    for (label, index, expected_meter, new_meter) in [
        ("stale", "0", "9/8", "3/4"),
        ("missing", "9", "4/4", "3/4"),
        ("empty", "0", "4/4", ""),
        ("invalid-text", "0", "4/4", "bad\u{0001}"),
    ] {
        let rejected = temporary_path(&format!("tempo-meter-{label}"));
        assert_ne!(
            invoke(&rejected, index, expected_meter, new_meter)
                .status
                .code(),
            Some(0)
        );
        assert!(!rejected.exists());
    }
    let unsupported_input = temporary_path("tempo-meter-unsupported-input");
    let unsupported = String::from_utf8(source.clone())
        .unwrap()
        .replace("TrackID=\"700\"", "TrackID=\"700\" Extra=\"x\"");
    fs::write(&unsupported_input, unsupported).unwrap();
    let unsupported_output = temporary_path("tempo-meter-unsupported-output");
    let unsupported_result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-tempo-meter",
            "--xml",
            unsupported_input.to_str().unwrap(),
            "--output",
            unsupported_output.to_str().unwrap(),
            "--track-id",
            "700",
            "--tempo-index",
            "0",
            "--expected-meter",
            "4/4",
            "--meter",
            "3/4",
        ])
        .output()
        .unwrap();
    assert_ne!(unsupported_result.status.code(), Some(0));
    assert!(!unsupported_output.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(existing);
    let _ = fs::remove_file(unsupported_input);
}

#[test]
fn edit_mark_name_escapes_xml_and_targets_ordinal_by_decoded_old_name() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap().replace(
        "Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>",
        "Name=\"Cue &amp; one\" Type=\"0\" Start=\"5\" Num=\"0\"/>\n      <POSITION_MARK Type=\"0\" Name=\"Second\" Start=\"6\" Num=\"1\"/>",
    );
    let input = temporary_path("mark-name-input");
    let output = temporary_path("mark-name-output");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-mark-name",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--expected-name",
            "Cue & one",
            "--name",
            "A \"new\" & <cue>",
        ])
        .output()
        .expect("run CLI");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected = source.replace(
        "Name=\"Cue &amp; one\"",
        "Name=\"A &quot;new&quot; &amp; &lt;cue&gt;\"",
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn edit_mark_name_rejects_invalid_stale_missing_and_unsupported_without_writes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let original = fs::read(&fixture).unwrap();
    let input = temporary_path("mark-name-rejected-input");
    fs::write(&input, &original).unwrap();
    let unsupported = String::from_utf8(original.clone())
        .unwrap()
        .replace("Name=\"Intro\"", "Name=\"Intro\" Extra=\"x\"")
        .into_bytes();
    for (label, bytes, index, old_name, new_name) in [
        ("stale", original.clone(), "0", "stale", "New".to_owned()),
        ("missing", original.clone(), "9", "Intro", "New".to_owned()),
        ("empty", original.clone(), "0", "Intro", String::new()),
        (
            "control",
            original.clone(),
            "0",
            "Intro",
            "bad\u{0001}".to_owned(),
        ),
        (
            "xml-forbidden",
            original.clone(),
            "0",
            "Intro",
            "bad\u{FFFF}".to_owned(),
        ),
        (
            "unsupported",
            unsupported.clone(),
            "0",
            "Intro",
            "New".to_owned(),
        ),
    ] {
        fs::write(&input, &bytes).unwrap();
        let output = temporary_path(&format!("mark-name-{label}-output"));
        let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-mark-name",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--track-id",
                "700",
                "--mark-index",
                index,
                "--expected-name",
                old_name,
                "--name",
                &new_name,
            ])
            .output()
            .unwrap();
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!output.exists(), "{label} published output");
        assert_eq!(fs::read(&input).unwrap(), bytes, "{label} changed source");
    }
    let alias = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-mark-name",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            input.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--expected-name",
            "Intro",
            "--name",
            "New",
        ])
        .output()
        .unwrap();
    assert_ne!(alias.status.code(), Some(0));
    assert_eq!(fs::read(&input).unwrap(), unsupported);
    let existing = temporary_path("mark-name-existing-output");
    fs::write(&existing, b"keep existing").unwrap();
    let no_overwrite = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-mark-name",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            existing.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--expected-name",
            "Intro",
            "--name",
            "New",
        ])
        .output()
        .unwrap();
    assert_ne!(no_overwrite.status.code(), Some(0));
    assert_eq!(fs::read(&existing).unwrap(), b"keep existing");
    let _ = fs::remove_file(existing);
    let _ = fs::remove_file(input);
}

#[test]
fn edit_mark_name_targets_nonzero_ordinal_exactly() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap().replace(
        "Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>",
        "Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>\n      <POSITION_MARK Name=\"Second\" Type=\"0\" Start=\"6\" Num=\"1\"/>",
    );
    let input = temporary_path("mark-name-ordinal-input");
    let output = temporary_path("mark-name-ordinal-output");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-mark-name",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "1",
            "--expected-name",
            "Second",
            "--name",
            "Renamed",
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(&output).unwrap(),
        source
            .replace("Name=\"Second\"", "Name=\"Renamed\"")
            .as_bytes()
    );
    assert_eq!(fs::read(&input).unwrap(), source.as_bytes());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn position_mark_end_edit_exports_one_guarded_end_and_refuses_unsafe_edits() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap().replace(
        "Type=\"0\" Start=\"5\" Num=\"0\"",
        "Type=\"4\" Start=\"5\" End=\"10\" Num=\"0\"",
    );
    let input = temporary_path("end-edit-input");
    let output = temporary_path("end-edit-output");
    fs::write(&input, &source).unwrap();
    let invoke = |destination: &std::path::Path, old: &str, new: &str, index: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-mark-end",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                destination.to_str().unwrap(),
                "--track-id",
                "700",
                "--mark-index",
                index,
                "--expected-end",
                old,
                "--end",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke(&output, "10", "12.5", "0");
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert_eq!(
        fs::read(&output).unwrap(),
        source.replace("End=\"10\"", "End=\"12.5\"").as_bytes()
    );
    assert_eq!(fs::read(&input).unwrap(), source.as_bytes());
    let existing = temporary_path("end-edit-existing");
    fs::write(&existing, b"keep").unwrap();
    for (dest, old, new, index, payload) in [
        (&existing, "10", "13", "0", source.clone()),
        (&input, "10", "13", "0", source.clone()),
        (
            &temporary_path("end-edit-missing"),
            "10",
            "13",
            "9",
            source.clone(),
        ),
        (
            &temporary_path("end-edit-stale"),
            "9",
            "13",
            "0",
            source.clone(),
        ),
        (
            &temporary_path("end-edit-invalid"),
            "10",
            "NaN",
            "0",
            source.clone(),
        ),
        (
            &temporary_path("end-edit-crossing"),
            "10",
            "5",
            "0",
            source.clone(),
        ),
    ] {
        fs::write(&input, &payload).unwrap();
        let result = invoke(dest, old, new, index);
        assert_ne!(result.status.code(), Some(0));
        if dest == &existing {
            assert_eq!(fs::read(dest).unwrap(), b"keep");
        } else if dest != &input {
            assert!(!dest.exists());
        }
        assert_eq!(fs::read(&input).unwrap(), payload.as_bytes());
    }
    let absent = source.replace(" End=\"10\"", "");
    fs::write(&input, &absent).unwrap();
    let absent_out = temporary_path("end-edit-absent");
    assert_ne!(invoke(&absent_out, "10", "12", "0").status.code(), Some(0));
    assert!(!absent_out.exists());
    assert_eq!(fs::read(&input).unwrap(), absent.as_bytes());
    for path in [&input, &output, &existing, &absent_out] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn position_mark_edit_changes_only_start_and_preserves_optional_end() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap().replace(
        "Type=\"0\" Start=\"5\" Num=\"0\"/>",
        "Type=\"0\" Start=\"5\" Num=\"0\"/>\n      <POSITION_MARK Name=\"Range\" Type=\"4\" Start=\"10\" End=\"20\" Num=\"1\"/>",
    );
    assert!(source.contains("Type=\"4\" Start=\"10\" End=\"20\" Num=\"1\""));
    let input = temporary_path("position-edit-input");
    let output = temporary_path("position-edit-output");
    fs::write(&input, &source).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-position",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "1",
            "--expected-start",
            "10",
            "--start",
            "11",
        ])
        .output()
        .expect("run CLI");

    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected = source.replace(
        "Type=\"4\" Start=\"10\" End=\"20\" Num=\"1\"",
        "Type=\"4\" Start=\"11\" End=\"20\" Num=\"1\"",
    );
    assert_eq!(fs::read_to_string(&output).unwrap(), expected);
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn position_mark_edit_keeps_absent_end_absent_for_point_marks() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap();
    let input = temporary_path("position-point-input");
    let output = temporary_path("position-point-output");
    fs::write(&input, &source).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "edit-position",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--expected-start",
            "5",
            "--start",
            "0",
        ])
        .output()
        .expect("run CLI");

    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let expected = source.replace("Start=\"5\"", "Start=\"0\"");
    assert_eq!(fs::read_to_string(&output).unwrap(), expected);
    assert!(!expected.contains(" End="));
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn position_mark_edit_rejects_stale_missing_invalid_and_crossing_end_without_writes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(&fixture).unwrap().replace(
        "Type=\"0\" Start=\"5\" Num=\"0\"",
        "Type=\"4\" Start=\"5\" End=\"10\" Num=\"0\"",
    );
    let input = temporary_path("position-edit-rejected-input");
    fs::write(&input, &source).unwrap();

    for (label, mark_index, old_start, new_start) in [
        ("stale", "0", "4", "6"),
        ("missing", "4", "5", "6"),
        ("invalid", "0", "5", "fast"),
        ("crossing-end", "0", "5", "10"),
    ] {
        let output = temporary_path(&format!("position-edit-{label}-output"));
        let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-position",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--track-id",
                "700",
                "--mark-index",
                mark_index,
                "--expected-start",
                old_start,
                "--start",
                new_start,
            ])
            .output()
            .expect("run CLI");

        assert_ne!(result.status.code(), Some(0), "{label} edit accepted");
        assert!(!output.exists(), "{label} edit created output");
        assert_eq!(
            fs::read_to_string(&input).unwrap(),
            source,
            "{label} changed source"
        );
    }

    let _ = fs::remove_file(input);
}

#[test]
fn tempo_edit_rejects_stale_missing_and_invalid_changes_without_writes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("tempo-edit-rejected-input");
    fs::write(&input, &source).unwrap();

    for (label, tempo_index, old_bpm, new_bpm) in [
        ("stale", "0", "119", "125"),
        ("missing", "9", "120", "125"),
        ("invalid", "0", "120", "fast"),
        ("ambiguous", "0", "120", "125"),
    ] {
        let case_source = if label == "ambiguous" {
            let xml = String::from_utf8(source.clone())
                .unwrap()
                .replace("Entries=\"1\"", "Entries=\"2\"")
                .replace(
                    "</COLLECTION>",
                    "    <TRACK TrackID=\"700\" Name=\"Duplicate\" Location=\"file://localhost/music/duplicate.flac\"/>\n  </COLLECTION>",
                );
            xml.into_bytes()
        } else {
            source.clone()
        };
        fs::write(&input, &case_source).unwrap();
        let output = temporary_path(&format!("tempo-edit-{label}-output"));
        let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-tempo",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--track-id",
                "700",
                "--tempo-index",
                tempo_index,
                "--expected-bpm",
                old_bpm,
                "--bpm",
                new_bpm,
            ])
            .output()
            .expect("run CLI");

        assert_ne!(result.status.code(), Some(0), "{label} edit accepted");
        assert!(!output.exists(), "{label} edit created output");
        assert_eq!(
            fs::read(&input).unwrap(),
            case_source,
            "{label} edit changed source"
        );
    }

    let _ = fs::remove_file(input);
}

#[test]
fn folder_rename_exports_one_guarded_name_and_rejects_unsafe_cases() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap())
        .unwrap()
        .replace("<NODE Type=\"0\" Name=\"ROOT\" Count=\"1\">", "<NODE Type=\"0\" Name=\"ROOT\" Count=\"2\">")
        .replace(
            "      </NODE>\n    </NODE>",
            "      </NODE>\n      <NODE Type=\"1\" Name=\"Clash\" Entries=\"0\" KeyType=\"0\"/>\n    </NODE>",
        )
        .into_bytes();
    let input = temporary_path("folder-rename-input");
    let output = temporary_path("folder-rename-output");
    fs::write(&input, &source).unwrap();
    let invoke = |old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "rename-folder",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--folder-index",
                "0",
                "--expected-name",
                old,
                "--name",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("Sets", "Live & Studio", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let mut expected = source.clone();
    let from = b"Name=\"Sets\"";
    let at = expected
        .windows(from.len())
        .position(|w| w == from)
        .unwrap();
    expected.splice(
        at..at + from.len(),
        b"Name=\"Live &amp; Studio\"".iter().copied(),
    );
    assert_eq!(fs::read(&output).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), source);
    assert_ne!(
        invoke("Sets", "Replacement", &output).status.code(),
        Some(0)
    );
    for (tag, old, new) in [
        ("collision", "Sets", "Clash"),
        ("stale", "stale", "Next"),
        ("invalid", "Sets", "bad\u{1}name"),
    ] {
        let rejected = temporary_path(&format!("folder-rename-{tag}"));
        assert_ne!(
            invoke(old, new, &rejected).status.code(),
            Some(0),
            "{tag} rename accepted"
        );
        assert!(!rejected.exists(), "{tag} rename published output");
    }
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn playlist_rename_exports_one_guarded_name_and_rejects_unsafe_cases() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("playlist-rename-input");
    fs::write(&input, &source).unwrap();
    let output = temporary_path("playlist-rename-output");
    let invoke = |old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "rename-playlist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "0",
                "--expected-name",
                old,
                "--name",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("House", "Summer & Sun", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let mut expected = source.clone();
    let marker = b"Name=\"House\"";
    assert_eq!(
        expected
            .windows(marker.len())
            .filter(|w| *w == marker)
            .count(),
        1
    );
    let position = expected
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    expected.splice(
        position..position + marker.len(),
        b"Name=\"Summer &amp; Sun\"".iter().copied(),
    );
    assert_eq!(fs::read(&output).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), source);
    let overwrite = invoke("House", "Replacement", &output);
    assert_ne!(overwrite.status.code(), Some(0));
    assert_eq!(fs::read(&output).unwrap(), expected);

    for (label, old, new) in [
        ("stale", "stale", "Next"),
        ("duplicate", "House", "Techno"),
        ("invalid", "House", "bad\u{1}name"),
    ] {
        let rejected = temporary_path(&format!("playlist-rename-{label}"));
        assert_ne!(
            invoke(old, new, &rejected).status.code(),
            Some(0),
            "{label} rename accepted"
        );
        assert!(!rejected.exists(), "{label} rename published output");
    }
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn track_name_edit_exports_one_guarded_name_and_rejects_stale_or_existing_output() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).expect("read synthetic F-02 fixture");
    let input = temporary_path("track-name-input");
    let output = temporary_path("track-name-output");
    fs::write(&input, &source).expect("write synthetic input");
    let invoke = |expected: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-name",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-name",
                expected,
                "--name",
                "Renamed track",
            ])
            .output()
            .expect("run track-name CLI")
    };

    let success = invoke("A", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert!(String::from_utf8_lossy(&success.stdout).contains("Renamed track"));
    let mut expected = source.clone();
    let from = b"TrackID=\"100\" Name=\"A\"";
    let at = expected
        .windows(from.len())
        .position(|w| w == from)
        .expect("target Track Name");
    expected.splice(
        at..at + from.len(),
        b"TrackID=\"100\" Name=\"Renamed track\"".iter().copied(),
    );
    assert_eq!(fs::read(&output).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-name-stale");
    let stale = invoke("stale", &stale_output);
    assert_ne!(stale.status.code(), Some(0), "stale guard accepted");
    assert!(!stale_output.exists(), "stale guard created output");
    let rejected_existing = invoke("A", &output);
    assert_ne!(
        rejected_existing.status.code(),
        Some(0),
        "existing destination accepted"
    );
    assert_eq!(
        fs::read(&output).unwrap(),
        expected,
        "existing destination changed"
    );
    assert_eq!(fs::read(&input).unwrap(), source);

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn track_album_edit_preserves_optional_state_and_only_changes_selected_album() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Album=\"old &amp; record\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-album-input");
    let output = temporary_path("track-album-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Track\t100\tA\tNone\tNone\tSome(\"old & record\")"));
    assert!(preview_text.contains("Track\t200\tB\tNone\tNone\tNone"));

    let empty_input = temporary_path("album-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Album=\"old &amp; record\"", "Album=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);

    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-album",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-album",
                old,
                "--album",
                new,
            ])
            .output()
            .unwrap()
    };
    let value = "A\tB\nC\rD & <x>";
    let success = invoke("100", "old & record", value, &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    let encoded = "Album=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
    assert!(String::from_utf8_lossy(&output_bytes).contains(encoded));
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "Album=\"old &amp; record\"",
        encoded,
        1,
    );
    assert_eq!(output_bytes, expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = Command::new("python3")
        .args([
            "-c",
            "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); c=r.find('COLLECTION'); a=next(x for x in c if x.get('TrackID')=='100'); b=next(x for x in c if x.get('TrackID')=='200'); assert a.get('Album')=='A\\tB\\nC\\rD & <x>'; assert b.get('Album') is None",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");

    for (label, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & record", "bad\u{1}"),
    ] {
        let rejected = temporary_path(&format!("album-{label}"));
        assert_ne!(invoke(id, old, new, &rejected).status.code(), Some(0));
        assert!(!rejected.exists(), "{label} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("album-duplicate");
    assert_ne!(
        invoke("100", "old & record", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & record", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("album-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & record", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_comments_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Comments=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-comments-input");
    let output = temporary_path("track-comments-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Track\t100\tA\tNone\tSome(\"old & note\")\tNone\tNone"));
    assert!(preview_text.contains("Track\t200\tB\tNone\tNone\tNone\tNone"));
    let empty_input = temporary_path("comments-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Comments=\"old &amp; note\"", "Comments=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&empty_preview.stdout)
            .contains("Track\t100\tA\tNone\tSome(\"\")\tNone\tNone")
    );
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-comments",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-comments",
                old,
                "--comments",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Comments=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = std::process::Command::new("python3").args(["-c", "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Comments')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Comments') is None", output.to_str().unwrap()]).status().unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (label, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("comments-{label}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{label} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("comments-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("comments-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_genre_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Genre=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-genre-input");
    let output = temporary_path("track-genre-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Track\t100\tA\tNone\tNone\tNone\tSome(\"old & note\")\tNone"));
    assert!(preview_text.contains("Track\t200\tB\tNone\tNone\tNone"));
    let empty_input = temporary_path("genre-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Genre=\"old &amp; note\"", "Genre=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&empty_preview.stdout)
            .contains("Track\t100\tA\tNone\tNone\tNone\tSome(\"\")\tNone")
    );
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-genre",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-genre",
                old,
                "--genre",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Genre=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = std::process::Command::new("python3").args(["-c", "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Genre')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Genre') is None", output.to_str().unwrap()]).status().unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (label, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("genre-{label}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{label} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("genre-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("genre-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_label_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Label=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-label-input");
    let output = temporary_path("track-label-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Some(\"old & note\")"));
    assert!(preview_text.contains("Track\t200\tB\tNone\tNone\tNone"));
    let empty_input = temporary_path("label-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Label=\"old &amp; note\"", "Label=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-label",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-label",
                old,
                "--label",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Label=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = std::process::Command::new("python3").args(["-c", "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Label')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Label') is None", output.to_str().unwrap()]).status().unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (label, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("label-{label}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{label} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("label-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("label-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_composer_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Composer=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-composer-input");
    let output = temporary_path("track-composer-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Some(\"old & note\")"));
    let empty_input = temporary_path("composer-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Composer=\"old &amp; note\"", "Composer=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-composer",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-composer",
                old,
                "--composer",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Composer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = Command::new("python3")
        .args([
            "-c",
            "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Composer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Composer') is None",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (case, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("composer-{case}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{case} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("composer-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("composer-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_grouping_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Grouping=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-grouping-input");
    let output = temporary_path("track-grouping-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Some(\"old & note\")"));
    let empty_input = temporary_path("grouping-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Grouping=\"old &amp; note\"", "Grouping=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-grouping",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-grouping",
                old,
                "--grouping",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Grouping=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = Command::new("python3")
        .args([
            "-c",
            "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Grouping')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Grouping') is None",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (case, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("grouping-{case}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{case} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("grouping-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("grouping-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_remixer_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Remixer=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-remixer-input");
    let output = temporary_path("track-remixer-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&preview.stdout).contains("Some(\"old & note\")"));
    let empty_input = temporary_path("remixer-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Remixer=\"old &amp; note\"", "Remixer=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-remixer",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-remixer",
                old,
                "--remixer",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes)
            .unwrap()
            .contains("Remixer=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = Command::new("python3")
        .args([
            "-c",
            "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Remixer')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Remixer') is None",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (case, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("remixer-{case}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{case} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("remixer-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("remixer-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_mix_edit_preserves_optional_state_and_exports_one_guarded_value() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Mix=\"old &amp; note\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Mix=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-mix-input");
    let output = temporary_path("track-mix-output");
    fs::write(&input, &source).unwrap();

    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview = String::from_utf8_lossy(&preview.stdout);
    assert!(preview.contains("Some(\"old & note\")"));
    assert!(preview.contains("Some(\"\")"));

    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-mix",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-mix",
                old,
                "--mix",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8_lossy(&output_bytes)
            .contains("Mix=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = Command::new("python3")
        .args([
            "-c",
            "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); ts={x.get('TrackID'):x for x in r.find('COLLECTION')}; assert ts['100'].get('Mix')=='A\\tB\\nC\\rD & <x>'; assert ts['200'].get('Mix')==''; assert ts['300'].get('Mix') is None",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");

    for (case, id, old, new) in [
        ("absent", "300", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("track-mix-{case}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{case} created output");
    }

    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"400\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("track-mix-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);

    let malformed = b"<DJ_PLAYLISTS><COLLECTION>";
    fs::write(&input, malformed).unwrap();
    let malformed_output = temporary_path("track-mix-malformed");
    assert_ne!(
        invoke("100", "old & note", "x", &malformed_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!malformed_output.exists());
    assert_eq!(fs::read(&input).unwrap(), malformed);

    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("track-mix-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [
        &input,
        &output,
        &existing,
        &duplicate_output,
        &malformed_output,
    ] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_tonality_edit_exports_one_guarded_value_and_rejects_without_mutation() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = String::from_utf8(fs::read(fixture).unwrap()).unwrap();
    let source = base
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Tonality=\"old &amp; note\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-tonality-input");
    let output = temporary_path("track-tonality-output");
    fs::write(&input, &source).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "export-plan", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(preview_text.contains("Some(\"old & note\")"));
    assert!(preview_text.contains("Track\t200\tB\tNone\tNone\tNone"));
    let empty_input = temporary_path("tonality-empty-preview");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace("Tonality=\"old &amp; note\"", "Tonality=\"\"");
    fs::write(&empty_input, empty_source).unwrap();
    let empty_preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "export-plan",
            "--xml",
            empty_input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(empty_preview.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty_preview.stdout).contains("Some(\"\")"));
    let _ = fs::remove_file(empty_input);
    let invoke = |id: &str, old: &str, new: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-tonality",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                id,
                "--expected-tonality",
                old,
                "--tonality",
                new,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "old & note", "A\tB\nC\rD & <x>", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let output_bytes = fs::read(&output).unwrap();
    assert!(
        String::from_utf8(output_bytes.clone())
            .unwrap()
            .contains("Tonality=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let decoded = std::process::Command::new("python3").args(["-c", "import sys,xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); t=next(x for x in r.find('COLLECTION') if x.get('TrackID')=='100'); assert t.get('Tonality')=='A\\tB\\nC\\rD & <x>'; assert next(x for x in r.find('COLLECTION') if x.get('TrackID')=='200').get('Tonality') is None", output.to_str().unwrap()]).status().unwrap();
    assert!(decoded.success(), "independent ElementTree decode failed");
    for (tonality, id, old, new) in [
        ("absent", "200", "", "x"),
        ("unknown", "999", "x", "y"),
        ("stale", "100", "bad", "x"),
        ("invalid", "100", "old & note", "bad\u{1}"),
    ] {
        let target = temporary_path(&format!("tonality-{tonality}"));
        assert_ne!(invoke(id, old, new, &target).status.code(), Some(0));
        assert!(!target.exists(), "{tonality} created output");
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("tonality-duplicate");
    assert_ne!(
        invoke("100", "old & note", "x", &duplicate_output)
            .status
            .code(),
        Some(0)
    );
    assert!(!duplicate_output.exists());
    assert_eq!(fs::read(&input).unwrap(), duplicate);
    fs::write(&input, &source).unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &input).status.code(),
        Some(0)
    );
    let existing = temporary_path("tonality-existing");
    fs::write(&existing, b"keep").unwrap();
    assert_ne!(
        invoke("100", "old & note", "x", &existing).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&existing).unwrap(), b"keep");
    assert_eq!(fs::read(&input).unwrap(), source);
    for path in [&input, &output, &existing, &duplicate_output] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn track_artist_edit_exports_only_artist_and_preserves_absence_and_source() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source =
        String::from_utf8(fs::read(&fixture).expect("read synthetic F-02 fixture")).unwrap();
    let source = source.replacen(
        "TrackID=\"100\" Name=\"A\"",
        "TrackID=\"100\" Name=\"A\" Artist=\"DJ &amp; Co\"",
        1,
    );
    let source = source.into_bytes();
    let input = temporary_path("track-artist-input");
    let output = temporary_path("track-artist-output");
    fs::write(&input, &source).expect("write synthetic input");
    let invoke = |track_id: &str, expected: &str, artist: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-artist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-artist",
                expected,
                "--artist",
                artist,
            ])
            .output()
            .expect("run track-artist CLI")
    };

    let success = invoke("100", "DJ & Co", "New & Artist", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let expected = String::from_utf8(source.clone())
        .unwrap()
        .replacen("Artist=\"DJ &amp; Co\"", "Artist=\"New &amp; Artist\"", 1)
        .into_bytes();
    assert_eq!(fs::read(&output).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), source);

    let whitespace_output = temporary_path("track-artist-whitespace");
    let whitespace = invoke("100", "DJ & Co", "A\tB\nC\rD", &whitespace_output);
    assert_eq!(
        whitespace.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&whitespace.stderr)
    );
    let whitespace_bytes = fs::read(&whitespace_output).unwrap();
    assert!(
        String::from_utf8(whitespace_bytes)
            .unwrap()
            .contains("Artist=\"A&#9;B&#10;C&#13;D\"")
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(whitespace_output);

    for (label, id, old, new) in [
        ("stale", "100", "stale", "Next"),
        ("absent", "200", "", "New Artist"),
        ("unknown", "999", "Nobody", "New Artist"),
    ] {
        let rejected = temporary_path(&format!("track-artist-{label}"));
        assert_ne!(invoke(id, old, new, &rejected).status.code(), Some(0));
        assert!(!rejected.exists(), "{label} request published output");
    }

    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("TrackID=\"200\"", "TrackID=\"100\"", 1)
        .into_bytes();
    fs::write(&input, &duplicate).unwrap();
    let duplicate_output = temporary_path("track-artist-duplicate");
    assert_ne!(
        invoke("100", "DJ & Co", "Next", &duplicate_output)
            .status
            .code(),
        Some(0),
        "duplicate target accepted"
    );
    assert!(!duplicate_output.exists());
    fs::write(&input, &source).unwrap();

    let rejected_existing = invoke("100", "DJ & Co", "Next", &output);
    assert_ne!(rejected_existing.status.code(), Some(0));
    assert_eq!(fs::read(&output).unwrap(), expected);
    let rejected_same_path = invoke("100", "DJ & Co", "Next", &input);
    assert_ne!(rejected_same_path.status.code(), Some(0));
    assert_eq!(fs::read(&input).unwrap(), source);

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn delete_track_cli_removes_shared_f02_track_with_exact_guard_and_new_output() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("track-delete-input");
    let output = temporary_path("track-delete-output");
    let source = fs::read(&fixture).unwrap();
    fs::write(&input, &source).unwrap();
    let invoke = |guard: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "delete-track",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "200",
                "--expected-memberships",
                guard,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("0:100,200,300;1:200,400", &output);
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"3\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"2\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"1\""));
    assert!(!exported.contains("TrackID=\"200\""));
    assert!(!exported.contains("Key=\"200\""));
    assert_eq!(fs::read(&input).unwrap(), source);
    let existing = fs::read(&output).unwrap();
    assert_ne!(
        invoke("0:100,200,300;1:200,400", &output).status.code(),
        Some(0)
    );
    assert_eq!(fs::read(&output).unwrap(), existing);
    for (label, guard) in [
        ("stale", "0:100,300;1:400"),
        ("incomplete", "0:100,200,300"),
    ] {
        let rejected = temporary_path(&format!("track-delete-{label}"));
        assert_ne!(invoke(guard, &rejected).status.code(), Some(0));
        assert!(!rejected.exists());
    }
    assert_ne!(
        invoke("0:100,200,300;1:200,400", &input).status.code(),
        Some(0)
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn remove_playlist_member_cli_retains_track_and_rejects_stale_guard() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("member-delete-input");
    let output = temporary_path("member-delete-output");
    let source = fs::read(fixture).unwrap();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "remove-playlist-member",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "0",
                "--expected-order",
                order,
                "--track-id",
                "200",
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "100,200,300");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"2\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"2\""));
    assert!(exported.contains("<TRACK TrackID=\"200\""));
    assert!(exported.contains("<TRACK Key=\"200\"/>"));
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale = temporary_path("member-delete-stale");
    assert_ne!(invoke(&stale, "100,300").status.code(), Some(0));
    assert!(!stale.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn append_playlist_member_cli_adds_existing_track_and_rejects_stale_guard() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("member-append-input");
    let output = temporary_path("member-append-output");
    let source = fs::read(fixture).unwrap();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "append-playlist-member",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "1",
                "--expected-order",
                order,
                "--track-id",
                "100",
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "200,400");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"3\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"3\""));
    assert!(exported.contains("<TRACK Key=\"100\"/>"));
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale = temporary_path("member-append-stale");
    assert_ne!(invoke(&stale, "200").status.code(), Some(0));
    assert!(!stale.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn append_playlist_member_cli_expands_empty_self_closing_playlist() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("empty-member-append-input");
    let output = temporary_path("empty-member-append-output");
    let source = fs::read_to_string(fixture)
        .unwrap()
        .replace(
            "<NODE Type=\"1\" Name=\"Techno\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
            "<NODE Type=\"1\" Name=\"Techno\" Entries=\"0\" KeyType=\"0\"/>",
        )
        .into_bytes();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, expected_order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "append-playlist-member",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "1",
                "--expected-order",
                expected_order,
                "--track-id",
                "100",
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"3\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"1\""));
    assert!(exported.contains(
        "<NODE Type=\"1\" Name=\"Techno\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"100\"/></NODE>"
    ));
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale = temporary_path("empty-member-append-stale");
    assert_ne!(invoke(&stale, "100").status.code(), Some(0));
    assert!(!stale.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn clear_playlist_cli_retains_collection_and_other_playlist() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("playlist-clear-input");
    let output = temporary_path("playlist-clear-output");
    let source = fs::read(fixture).unwrap();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "clear-playlist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "0",
                "--expected-order",
                order,
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "100,200,300");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"0\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"2\""));
    let house_start = exported
        .find("<NODE Type=\"1\" Name=\"House\" Entries=\"0\"")
        .unwrap();
    let house_end = exported[house_start..].find("</NODE>").unwrap() + house_start;
    assert!(!exported[house_start..house_end].contains("<TRACK"));
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale = temporary_path("playlist-clear-stale");
    assert_ne!(invoke(&stale, "100,300").status.code(), Some(0));
    assert!(!stale.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn insert_playlist_member_cli_targets_guarded_zero_based_position() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("playlist-insert-input");
    let output = temporary_path("playlist-insert-output");
    let source = fs::read(fixture).unwrap();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str, position: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "insert-playlist-member",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "1",
                "--expected-order",
                order,
                "--track-id",
                "100",
                "--position",
                position,
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "200,400", "1");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"3\""));
    assert!(exported.contains("Name=\"Techno\" Entries=\"3\""));
    let techno_start = exported.find("Name=\"Techno\"").unwrap();
    let techno_end = exported[techno_start..].find("</NODE>").unwrap() + techno_start;
    let techno = &exported[techno_start..techno_end];
    let first = techno.find("Key=\"200\"").unwrap();
    let second = techno.find("Key=\"100\"").unwrap();
    let third = techno.find("Key=\"400\"").unwrap();
    assert!(first < second && second < third);
    assert_eq!(fs::read(&input).unwrap(), source);
    for (label, order, position) in [("stale", "200", "1"), ("out-of-range", "200,400", "3")] {
        let rejected = temporary_path(&format!("playlist-insert-{label}"));
        assert_ne!(invoke(&rejected, order, position).status.code(), Some(0));
        assert!(!rejected.exists());
    }
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

fn temporary_path(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidj-rekordbox-{label}-{}-{stamp}.xml",
        std::process::id()
    ))
}
