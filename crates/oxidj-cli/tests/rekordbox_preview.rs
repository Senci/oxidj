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
