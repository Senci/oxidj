use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use std::{fs, process::Command, time::SystemTime};

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
    assert!(stdout.contains("700\tTempo cue independence\tfile://localhost/music/tempo.flac"));
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
            "Track\t1000\tVariable tempo\tfile://localhost/music/variable.flac\n",
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
    let actual = fs::read_to_string(&output).unwrap();
    assert!(actual.contains("Name=\"A &quot;new&quot; &amp; &lt;cue&gt;\""));
    assert!(actual.contains("Name=\"Second\""));
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
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
