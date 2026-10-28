use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use std::{fs, process::Command, time::SystemTime};

#[test]
fn preview_lists_nested_duplicate_and_empty_folders_in_source_order() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "<NODE Type=\"0\" Name=\"Sets\" Count=\"2\">",
            "<NODE Type=\"0\" Name=\"Sets\" Count=\"5\">\n        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n        <NODE Type=\"0\" Name=\"Paired\" Count=\"0\"></NODE>",
        )
        .into_bytes();
    let input = temporary_path("folder-preview");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    let folders: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("Folder\t"))
        .collect();
    assert_eq!(
        folders,
        [
            "Folder\t0\tPath=[\"ROOT\"]\tName=\"ROOT\"\tCount=Some(\"1\")",
            "Folder\t1\tPath=[\"ROOT\", \"Sets\"]\tName=\"Sets\"\tCount=Some(\"5\")",
            "Folder\t2\tPath=[\"ROOT\", \"Sets\", \"Empty\"]\tName=\"Empty\"\tCount=Some(\"0\")",
            "Folder\t3\tPath=[\"ROOT\", \"Sets\", \"Empty\"]\tName=\"Empty\"\tCount=Some(\"0\")",
            "Folder\t4\tPath=[\"ROOT\", \"Sets\", \"Paired\"]\tName=\"Paired\"\tCount=Some(\"0\")",
        ]
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_displays_raw_playlist_node_attribute_states_for_empty_nodes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
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
        )
        .into_bytes();
    let input = temporary_path("playlist-node-preview");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(
        stdout.contains("Sets/House\t\tEntries=Some(\"\")\tKeyType=Some(\"0\")"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Sets/Techno\t\tEntries=None\tKeyType=Some(\"\")"),
        "{stdout}"
    );
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_reports_raw_composer_states_without_changing_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Composer=\"Raw &amp; Composer\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Composer=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-composer-preview");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    for (id, expected) in [
        ("100", "Some(\"Raw & Composer\")"),
        ("200", "Some(\"\")"),
        ("300", "None"),
    ] {
        let row = stdout
            .lines()
            .find(|line| line.starts_with(&format!("{id}\t")))
            .unwrap();
        assert!(row.contains(expected), "expected {expected} in {row}");
    }
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_reports_raw_remixer_states_without_changing_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Remixer=\"Raw &amp; Remixer\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Remixer=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-remixer-preview");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    for (id, expected) in [
        ("100", "Some(\"Raw & Remixer\")"),
        ("200", "Some(\"\")"),
        ("300", "None"),
    ] {
        let row = stdout
            .lines()
            .find(|line| line.starts_with(&format!("{id}\t")))
            .unwrap();
        assert!(row.contains(expected), "expected {expected} in {row}");
    }
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_reports_raw_genre_states_without_changing_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Genre=\"Raw &amp; Genre\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Genre=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-genre-preview");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    for (id, expected) in [
        ("100", "Some(\"Raw & Genre\")"),
        ("200", "Some(\"\")"),
        ("300", "None"),
    ] {
        let row = stdout
            .lines()
            .find(|line| line.starts_with(&format!("{id}\t")))
            .unwrap();
        assert!(row.contains(expected), "expected {expected} in {row}");
    }
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_and_export_plan_report_raw_play_count_states_and_preserve_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" DateAdded=\"2024-02-03\" DateModified=\"2024-02-04\" LastPlayed=\"2024-02-05\" PlayCount=\"not-a-number\" Rating=\"-1\" AverageBpm=\"126.12500000000001\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" DateAdded=\"\" DateModified=\"\" LastPlayed=\"\" PlayCount=\"\" Rating=\"\" AverageBpm=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-dates-input");
    let output = temporary_path("raw-dates-output");
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
        let lines = stdout.lines().collect::<Vec<_>>();
        let first = lines.iter().find(|line| line.contains("100\tA\t")).unwrap();
        let second = lines.iter().find(|line| line.contains("200\tB\t")).unwrap();
        let third = lines.iter().find(|line| line.contains("300\tC\t")).unwrap();
        for (row, added, modified, last_played, play_count, rating, average_bpm) in [
            (
                first,
                "Some(\"2024-02-03\")",
                "Some(\"2024-02-04\")",
                "Some(\"2024-02-05\")",
                "Some(\"not-a-number\")",
                "Some(\"-1\")",
                "Some(\"126.12500000000001\")",
            ),
            (
                second,
                "Some(\"\")",
                "Some(\"\")",
                "Some(\"\")",
                "Some(\"\")",
                "Some(\"\")",
                "Some(\"\")",
            ),
            (third, "None", "None", "None", "None", "None", "None"),
        ] {
            let fields = row.split('\t').collect::<Vec<_>>();
            let suffix = [
                added,
                modified,
                last_played,
                play_count,
                rating,
                "None",
                average_bpm,
            ]
            .join("\t");
            assert!(row.ends_with(&suffix), "{row}");
            assert!(row.contains(average_bpm), "{row}");
            assert_eq!(fields[fields.len() - 3], rating, "{row}");
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
fn preview_and_export_plan_report_raw_colour_states_and_export_preserves_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Colour=\"#12aBcF\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Colour=\"\"",
        )
        .into_bytes();
    let input = temporary_path("colour-input");
    let output = temporary_path("colour-output");
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
        for (id, colour) in [
            ("100", "Some(\"#12aBcF\")"),
            ("200", "Some(\"\")"),
            ("300", "None"),
        ] {
            let row = stdout
                .lines()
                .find(|line| {
                    line.starts_with(&format!("{id}\t"))
                        || line.starts_with(&format!("Track\t{id}\t"))
                })
                .unwrap();
            assert_eq!(row.split('\t').rev().nth(1), Some(colour), "{row}");
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
            "TrackID=\"100\" Name=\"A\" Year=\"2024\" Size=\"9223372036854775807\" DiscNumber=\"-2147483648\" TrackNumber=\"-2147483648\" TotalTime=\"3.141592653589793\" BitRate=\"-2147483648\" SampleRate=\"48000.0\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Year=\"\" Size=\"\" DiscNumber=\"\" TrackNumber=\"\" TotalTime=\"\" BitRate=\"\" SampleRate=\"\"",
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
        for (track_id, expected_track_number) in [
            ("100", "Some(\"-2147483648\")"),
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
            assert_eq!(
                row.split('\t').rev().nth(11),
                Some(expected_track_number),
                "{row}"
            );
            let expected_total_time = match track_id {
                "100" => "Some(\"3.141592653589793\")",
                "200" => "Some(\"\")",
                _ => "None",
            };
            assert_eq!(
                row.split('\t').rev().nth(8),
                Some(match track_id {
                    "100" => "Some(\"-2147483648\")",
                    "200" => "Some(\"\")",
                    _ => "None",
                }),
                "{row}"
            );
            assert!(
                row.split('\t').any(|field| field == expected_total_time),
                "{row}"
            );
            assert!(
                row.split('\t').any(|field| field
                    == match track_id {
                        "100" => "Some(\"48000.0\")",
                        "200" => "Some(\"\")",
                        _ => "None",
                    }),
                "{row}"
            );
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
        "700\tTempo cue independence\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tfile://localhost/music/tempo.flac"
    ));
    assert!(stdout.contains("TEMPO\t10\t128\t4/4\t1"));
    assert!(
        stdout
            .contains("  POSITION_MARK\t700\t0\tType=0\tStart=5\tNum=0\tName=\"Intro\"\tEnd=None")
    );
    assert!(stdout.contains("Playlists:"));
}

#[test]
fn preview_lists_raw_marks_with_track_local_ordinals_and_end_presence() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(fixture)
        .unwrap()
        .replace("<COLLECTION Entries=\"1\">", "<COLLECTION Entries=\"2\">")
        .replace(
            "<POSITION_MARK Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>",
            "<POSITION_MARK Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>\n      <POSITION_MARK Name=\"\" Type=\"4\" Start=\"7\" Num=\"1\" End=\"0\"/>",
        )
        .replace(
            "  </COLLECTION>",
            "    <TRACK TrackID=\"701\" Name=\"Second\" Location=\"file://localhost/music/second.flac\">\n      <POSITION_MARK Name=\"Cue &amp; Two\" Type=\"0\" Start=\"8\" Num=\"2\"/>\n    </TRACK>\n  </COLLECTION>",
        );
    let input = temporary_path("raw-position-mark-preview");
    fs::write(&input, source.as_bytes()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    let marks: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("  POSITION_MARK\t"))
        .collect();
    assert_eq!(
        marks,
        [
            "  POSITION_MARK\t700\t0\tType=0\tStart=5\tNum=0\tName=\"Intro\"\tEnd=None",
            "  POSITION_MARK\t700\t1\tType=4\tStart=7\tNum=1\tName=\"\"\tEnd=Some(\"0\")",
            "  POSITION_MARK\t701\t0\tType=0\tStart=8\tNum=2\tName=\"Cue & Two\"\tEnd=None",
        ]
    );
    assert_eq!(fs::read(&input).unwrap(), source.as_bytes());
    let _ = fs::remove_file(input);
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
            "Track\t1000\tVariable tempo\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tfile://localhost/music/variable.flac\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\tNone\n",
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
fn delete_mark_cli_removes_exactly_one_raw_guarded_mark_and_preserves_source() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("mark-delete-input");
    let output = temporary_path("mark-delete-output");
    fs::write(&input, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "delete-mark",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--type",
            "0",
            "--start",
            "5",
            "--name",
            "Intro",
            "--no-end",
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let edited = fs::read(&output).unwrap();
    let expected = String::from_utf8(source.clone())
        .unwrap()
        .replace(
            "<POSITION_MARK Name=\"Intro\" Type=\"0\" Start=\"5\" Num=\"0\"/>",
            "",
        )
        .into_bytes();
    assert_eq!(edited, expected);
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale_output = temporary_path("mark-delete-stale-output");
    let stale = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "delete-mark",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            stale_output.to_str().unwrap(),
            "--track-id",
            "700",
            "--mark-index",
            "0",
            "--type",
            "stale",
            "--start",
            "5",
            "--name",
            "Intro",
            "--no-end",
        ])
        .output()
        .unwrap();
    assert_ne!(stale.status.code(), Some(0));
    assert!(!stale_output.exists());
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
fn folder_create_cli_exports_escaped_child_and_refuses_collision_without_output() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("folder-create-input");
    let output = temporary_path("folder-create-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, name: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "create-folder",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--parent-index",
                "0",
                "--expected-name",
                "Sets",
                "--name",
                name,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "R&B <late>");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Sets\" Count=\"3\""));
    assert!(exported.contains("Name=\"R&amp;B &lt;late&gt;\" Count=\"0\""));
    assert_eq!(fs::read(&input).unwrap(), source);
    let rejected = temporary_path("folder-create-collision");
    assert_ne!(invoke(&rejected, "House").status.code(), Some(0));
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn playlist_create_cli_exports_escaped_child_and_refuses_collision_without_output() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("playlist-create-input");
    let output = temporary_path("playlist-create-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, name: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "create-playlist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--parent-index",
                "0",
                "--expected-name",
                "Sets",
                "--name",
                name,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "R&B <late>");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Sets\" Count=\"3\""));
    assert!(exported.contains("Name=\"R&amp;B &lt;late&gt;\" KeyType=\"0\" Entries=\"0\""));
    assert_eq!(fs::read(&input).unwrap(), source);
    let rejected = temporary_path("playlist-create-collision");
    assert_ne!(invoke(&rejected, "House").status.code(), Some(0));
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn sibling_playlist_reorder_cli_exports_new_order_and_preserves_input() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read(&fixture).unwrap();
    let input = temporary_path("sibling-reorder-input");
    let output = temporary_path("sibling-reorder-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "reorder-sibling-playlists",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--parent-index",
                "0",
                "--expected-name",
                "Sets",
                "--expected-order",
                "House,Techno",
                "--order",
                order,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "Techno,House");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.find("Name=\"Techno\"").unwrap() < exported.find("Name=\"House\"").unwrap());
    assert_eq!(fs::read(&input).unwrap(), source);
    let rejected = temporary_path("sibling-reorder-stale");
    assert_ne!(invoke(&rejected, "House,House").status.code(), Some(0));
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn sibling_folder_reorder_cli_exports_subtree_order_and_preserves_input() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap()).unwrap()
        .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"4\"")
        .replace("        <NODE Type=\"1\" Name=\"Techno\"", "        <NODE Type=\"0\" Name=\"FolderA\" Count=\"1\"><NODE Type=\"1\" Name=\"A-child\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"100\"/></NODE></NODE>\n        <NODE Type=\"1\" Name=\"Techno\"")
        .replace("      </NODE>\n    </NODE>\n  </PLAYLISTS>", "        <NODE Type=\"0\" Name=\"FolderB\" Count=\"1\"><NODE Type=\"1\" Name=\"B-child\" Entries=\"1\" KeyType=\"0\"><TRACK Key=\"200\"/></NODE></NODE>\n      </NODE>\n    </NODE>\n  </PLAYLISTS>");
    let input = temporary_path("sibling-folder-reorder-input");
    let output = temporary_path("sibling-folder-reorder-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "reorder-sibling-folders",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--parent-index",
                "0",
                "--expected-name",
                "Sets",
                "--expected-order",
                "FolderA,FolderB",
                "--order",
                order,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "FolderB,FolderA");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(
        exported.find("Name=\"FolderB\"").unwrap() < exported.find("Name=\"FolderA\"").unwrap()
    );
    assert!(exported.contains("Name=\"B-child\" Entries=\"1\" KeyType=\"0\""));
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let rejected = temporary_path("sibling-folder-reorder-stale");
    assert_ne!(invoke(&rejected, "FolderA,FolderA").status.code(), Some(0));
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn folder_move_cli_moves_subtree_updates_counts_and_preserves_input() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap())
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
    let input = temporary_path("folder-move-input");
    let output = temporary_path("folder-move-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, target_index: &str, target_name: &str, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "move-folder",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--folder-index",
                "1",
                "--expected-name",
                "MoveMe",
                "--source-parent-index",
                "0",
                "--source-parent-name",
                "Sets",
                "--expected-source-order",
                "0:MoveMe,1:Techno,0:Target",
                "--target-parent-index",
                target_index,
                "--target-parent-name",
                target_name,
                "--expected-target-order",
                order,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "2", "Target", "");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Sets\" Count=\"2\""));
    assert!(exported.contains("Name=\"Target\" Count=\"1\">"));
    assert!(exported.find("Name=\"House\"").unwrap() > exported.find("Name=\"Target\"").unwrap());
    assert!(exported.contains("<TRACK Key=\"100\"/><TRACK Key=\"200\"/><TRACK Key=\"300\"/>"));
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let rejected = temporary_path("folder-move-cycle");
    assert_ne!(
        invoke(&rejected, "1", "MoveMe", "1:House").status.code(),
        Some(0)
    );
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn playlist_move_cli_updates_parent_counts_and_preserves_input() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap())
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
    let input = temporary_path("playlist-move-input");
    let output = temporary_path("playlist-move-output");
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "move-playlist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                "0",
                "--expected-name",
                "House",
                "--expected-members",
                "100,200,300",
                "--source-parent-index",
                "1",
                "--source-parent-name",
                "Source",
                "--target-parent-index",
                "2",
                "--target-parent-name",
                "Target",
                "--expected-target-order",
                order,
            ])
            .output()
            .unwrap()
    };
    let created = invoke(&output, "Other");
    assert_eq!(created.status.code(), Some(0), "{:?}", created.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Source\" Count=\"1\""));
    assert!(exported.contains("Name=\"Target\" Count=\"2\""));
    assert!(exported.find("Name=\"Techno\"").unwrap() < exported.find("Name=\"Other\"").unwrap());
    assert!(exported.find("Name=\"Other\"").unwrap() < exported.find("Name=\"House\"").unwrap());
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let rejected = temporary_path("playlist-move-stale");
    assert_ne!(invoke(&rejected, "stale").status.code(), Some(0));
    assert!(!rejected.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn playlist_move_cli_expands_empty_target_and_moves_empty_playlist() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap())
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
    let input = temporary_path("playlist-move-empty-input");
    let output = temporary_path("playlist-move-empty-output");
    fs::write(&input, &source).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "move-playlist",
            "--xml",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--playlist-index",
            "0",
            "--expected-name",
            "House",
            "--expected-members",
            "",
            "--source-parent-index",
            "0",
            "--source-parent-name",
            "Source",
            "--target-parent-index",
            "1",
            "--target-parent-name",
            "Target",
            "--expected-target-order",
            "",
        ])
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(0), "{:?}", run.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Source\" Count=\"1\""));
    assert!(exported.contains("Name=\"Target\" Count=\"1\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"0\" KeyType=\"0\"/>"));
    assert!(exported.find("Name=\"Techno\"").unwrap() < exported.find("Name=\"House\"").unwrap());
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn empty_folder_delete_cli_is_guarded_and_preserves_input() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(&fixture).unwrap())
        .unwrap()
        .replace("Name=\"Sets\" Count=\"2\"", "Name=\"Sets\" Count=\"3\"")
        .replace(
            "      </NODE>\n    </NODE>",
            "        <NODE Type=\"0\" Name=\"Empty\" Count=\"0\"/>\n      </NODE>\n    </NODE>",
        );
    let input = temporary_path("empty-folder-delete-input");
    let output = temporary_path("empty-folder-delete-output");
    fs::write(&input, &source).unwrap();
    let invoke = |index: &str, name: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "delete-empty-folder",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--folder-index",
                index,
                "--expected-name",
                name,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("1", "Empty", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("Name=\"Sets\" Count=\"2\""));
    assert!(!exported.contains("Name=\"Empty\""));
    assert_eq!(fs::read_to_string(&input).unwrap(), source);
    let stale = temporary_path("empty-folder-delete-stale");
    assert_ne!(invoke("1", "stale", &stale).status.code(), Some(0));
    assert!(!stale.exists());
    let nonempty = temporary_path("empty-folder-delete-nonempty");
    assert_ne!(invoke("0", "Sets", &nonempty).status.code(), Some(0));
    assert!(!nonempty.exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
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
fn track_colour_edit_escapes_xml_and_rejects_stale_guard_without_writing() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Colour=\"old &amp; colour\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-colour-input");
    let output = temporary_path("track-colour-output");
    fs::write(&input, &source).unwrap();
    let invoke = |expected: &str, colour: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-colour",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-colour",
                expected,
                "--colour",
                colour,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("old & colour", "A\tB\nC\rD & <x>", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let encoded = "Colour=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "Colour=\"old &amp; colour\"",
        encoded,
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-colour-stale");
    let stale = invoke("stale", "replacement", &stale_output);
    assert_ne!(stale.status.code(), Some(0), "stale guard accepted");
    assert!(!stale_output.exists(), "stale guard created output");
    assert_eq!(fs::read(&input).unwrap(), source);
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
fn preview_reports_raw_track_label_states_and_preserves_source_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let base = fs::read(fixture).unwrap();
    let source = String::from_utf8(base)
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Label=\"raw &amp; label\"",
            1,
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Label=\"\"",
        );
    let input = temporary_path("label-preview");
    fs::write(&input, source.as_bytes()).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let text = String::from_utf8_lossy(&preview.stdout);
    let track_row = |id: &str| {
        text.lines()
            .find(|line| line.starts_with(&format!("{id}\t")))
            .unwrap()
            .split('\t')
            .nth(6)
            .unwrap()
            .to_owned()
    };
    assert_eq!(track_row("100"), "Some(\"raw & label\")");
    assert_eq!(track_row("200"), "Some(\"\")");
    assert_eq!(track_row("300"), "None");
    assert_eq!(fs::read(&input).unwrap(), source.as_bytes());
    let _ = fs::remove_file(input);
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
fn preview_reports_raw_track_tonality_states_and_preserves_source_bytes() {
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
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Tonality=\"\"",
        );
    let input = temporary_path("tonality-preview");
    fs::write(&input, source.as_bytes()).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(0));
    let preview_text = String::from_utf8_lossy(&preview.stdout);
    assert!(
        preview_text.contains("Some(\"old & note\")"),
        "{preview_text}"
    );
    assert!(preview_text.contains("Some(\"\")"), "{preview_text}");
    assert!(
        preview_text.contains("200\tB\tNone\tNone\tNone\tNone\tNone\tSome(\"\")"),
        "{preview_text}"
    );
    assert!(
        preview_text.contains("300\tC\tNone\tNone\tNone\tNone"),
        "{preview_text}"
    );
    assert_eq!(fs::read(&input).unwrap(), source.as_bytes());
    let _ = fs::remove_file(input);
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
fn delete_playlist_cli_guards_ordinal_and_preserves_source() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let input = temporary_path("playlist-delete-input");
    let output = temporary_path("playlist-delete-output");
    let source = fs::read_to_string(fixture)
        .unwrap()
        .replace("Name=\"Techno\"", "Name=\"House\"")
        .into_bytes();
    fs::write(&input, &source).unwrap();
    let invoke = |target: &std::path::Path, index: &str, name: &str, order: &str| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "delete-playlist",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--playlist-index",
                index,
                "--expected-name",
                name,
                "--expected-order",
                order,
            ])
            .output()
            .unwrap()
    };
    let result = invoke(&output, "1", "House", "200,400");
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = fs::read_to_string(&output).unwrap();
    assert!(exported.contains("<COLLECTION Entries=\"4\">"));
    assert!(exported.contains("<NODE Type=\"0\" Name=\"Sets\" Count=\"1\">"));
    assert!(exported.contains("Name=\"House\" Entries=\"3\""));
    assert!(!exported.contains("<NODE Type=\"1\" Name=\"House\" Entries=\"2\""));
    assert_eq!(fs::read(&input).unwrap(), source);
    let stale = temporary_path("playlist-delete-stale");
    assert_ne!(invoke(&stale, "1", "House", "200").status.code(), Some(0));
    assert!(!stale.exists());

    let empty_input = temporary_path("playlist-delete-empty-input");
    let empty_output = temporary_path("playlist-delete-empty-output");
    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replace(
            "<NODE Type=\"1\" Name=\"House\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
            "<NODE Type=\"1\" Name=\"House\" Entries=\"0\" KeyType=\"0\"/>",
        )
        .into_bytes();
    fs::write(&empty_input, &empty_source).unwrap();
    let empty_result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "delete-playlist",
            "--xml",
            empty_input.to_str().unwrap(),
            "--output",
            empty_output.to_str().unwrap(),
            "--playlist-index",
            "1",
            "--expected-name",
            "House",
            "--expected-order",
            "",
        ])
        .output()
        .unwrap();
    assert_eq!(
        empty_result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&empty_result.stderr)
    );
    let empty_export = fs::read_to_string(&empty_output).unwrap();
    assert!(empty_export.contains("Name=\"House\" Entries=\"3\""));
    assert!(!empty_export.contains("<NODE Type=\"1\" Name=\"House\" Entries=\"0\""));
    assert!(empty_export.contains("<NODE Type=\"0\" Name=\"Sets\" Count=\"1\">"));
    assert_eq!(fs::read(&empty_input).unwrap(), empty_source);

    // Duplicate names remain addressable by ordinal plus the member-order guard.
    let duplicate_output = temporary_path("playlist-delete-duplicate");
    let duplicate_result = invoke(&duplicate_output, "1", "House", "200,400");
    assert_eq!(duplicate_result.status.code(), Some(0));
    let duplicate_export = fs::read_to_string(&duplicate_output).unwrap();
    assert!(duplicate_export.contains("<NODE Type=\"1\" Name=\"House\" Entries=\"3\""));
    assert!(!duplicate_export.contains("<NODE Type=\"1\" Name=\"House\" Entries=\"2\""));
    let ambiguous_input = temporary_path("playlist-delete-ambiguous-input");
    let ambiguous_source = String::from_utf8(source.clone())
        .unwrap()
        .replace(
            "<NODE Type=\"1\" Name=\"House\" Entries=\"2\" KeyType=\"0\">\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"400\"/>\n        </NODE>",
            "<NODE Type=\"1\" Name=\"House\" Entries=\"3\" KeyType=\"0\">\n          <TRACK Key=\"100\"/>\n          <TRACK Key=\"200\"/>\n          <TRACK Key=\"300\"/>\n        </NODE>",
        )
        .into_bytes();
    fs::write(&ambiguous_input, &ambiguous_source).unwrap();
    let ambiguous_output = temporary_path("playlist-delete-ambiguous");
    let ambiguous_result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "delete-playlist",
            "--xml",
            ambiguous_input.to_str().unwrap(),
            "--output",
            ambiguous_output.to_str().unwrap(),
            "--playlist-index",
            "1",
            "--expected-name",
            "House",
            "--expected-order",
            "100,200,300",
        ])
        .output()
        .unwrap();
    assert_ne!(ambiguous_result.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&ambiguous_result.stderr)
            .contains("playlist deletion guard is ambiguous")
    );
    assert!(!ambiguous_output.exists());

    // Malformed input must be rejected without creating an output file.
    let malformed_input = temporary_path("playlist-delete-malformed-input");
    let malformed_output = temporary_path("playlist-delete-malformed-output");
    fs::write(&malformed_input, b"<DJ_PLAYLISTS>").unwrap();
    let malformed_result = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args([
            "rekordbox",
            "delete-playlist",
            "--xml",
            malformed_input.to_str().unwrap(),
            "--output",
            malformed_output.to_str().unwrap(),
            "--playlist-index",
            "0",
            "--expected-name",
            "House",
            "--expected-order",
            "100,200,300",
        ])
        .output()
        .unwrap();
    assert_ne!(malformed_result.status.code(), Some(0));
    assert!(!malformed_output.exists());

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_input);
    let _ = fs::remove_file(empty_output);
    let _ = fs::remove_file(duplicate_output);
    let _ = fs::remove_file(ambiguous_input);
    let _ = fs::remove_file(ambiguous_output);
    let _ = fs::remove_file(malformed_input);
    let _ = fs::remove_file(malformed_output);
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

#[test]
fn preview_reports_raw_grouping_states_and_preserves_source_bytes() {
    let source = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(source)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Grouping=\"Raw &amp; Group\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Grouping=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-grouping-preview");
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
        for (id, name, expected) in [
            ("100", "A", "Some(\"Raw & Group\")"),
            ("200", "B", "Some(\"\")"),
            ("300", "C", "None"),
        ] {
            let row = stdout
                .lines()
                .find(|line| line.contains(&format!("{id}\t{name}\t")))
                .unwrap();
            assert!(row.contains(expected), "expected {expected} in {row}");
        }
    }
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn preview_reports_raw_mix_states_and_preserves_source_bytes() {
    let base = fs::read(format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let source = String::from_utf8(base)
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Mix=\"Raw &amp; Mix\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Mix=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-mix-preview");
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
        for (id, name, expected) in [
            ("100", "A", "Some(\"Raw & Mix\")"),
            ("200", "B", "Some(\"\")"),
            ("300", "C", "None"),
        ] {
            let row = stdout
                .lines()
                .find(|line| line.contains(&format!("{id}\t{name}\t")))
                .unwrap();
            assert!(row.contains(expected), "expected {expected} in {row}");
        }
    }
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
}

#[test]
fn no_edit_export_preserves_raw_optional_track_metadata_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replace(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Genre=\"Raw &amp; Genre\" Label=\"Raw Label\" Tonality=\"Raw Key\" Composer=\"Raw Composer\" Grouping=\"Raw Group\" Remixer=\"Raw Remixer\" Mix=\"Raw Mix\"",
        )
        .replace(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Genre=\"\" Label=\"\" Tonality=\"\" Composer=\"\" Grouping=\"\" Remixer=\"\" Mix=\"\"",
        )
        .into_bytes();
    let input = temporary_path("raw-metadata-no-edit-input");
    let output = temporary_path("raw-metadata-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_sample_rate_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" SampleRate=\"48000.0 Hz &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" SampleRate=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("sample-rate-no-edit-input");
    let output = temporary_path("sample-rate-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_size_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Size=\"9223372036854775807 &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Size=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("size-no-edit-input");
    let output = temporary_path("size-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_bit_rate_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" BitRate=\"320.5 kbps &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" BitRate=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("bit-rate-no-edit-input");
    let output = temporary_path("bit-rate-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_track_number_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" TrackNumber=\"001/012 &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" TrackNumber=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-number-no-edit-input");
    let output = temporary_path("track-number-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_disc_number_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" DiscNumber=\"01/02 &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" DiscNumber=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("disc-number-no-edit-input");
    let output = temporary_path("disc-number-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_total_time_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" TotalTime=\"03:21 &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" TotalTime=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("total-time-no-edit-input");
    let output = temporary_path("total-time-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_year_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Year=\"20&amp;24 raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Year=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("year-no-edit-input");
    let output = temporary_path("year-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_average_bpm_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" AverageBpm=\"120.5 &amp; raw\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" AverageBpm=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("average-bpm-no-edit-input");
    let output = temporary_path("average-bpm-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_colour_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Colour=\"old &amp; palette\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Colour=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("colour-no-edit-input");
    let output = temporary_path("colour-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_rating_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Rating=\"old &amp; stars\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Rating=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("rating-no-edit-input");
    let output = temporary_path("rating-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_kind_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Kind=\"old &amp; codec\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" Kind=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("kind-no-edit-input");
    let output = temporary_path("kind-no-edit-output");
    fs::write(&input, &source).unwrap();
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
fn no_edit_export_preserves_raw_play_count_present_empty_and_absent_bytes() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" PlayCount=\"old &amp; count\"",
            1,
        )
        .replacen(
            "TrackID=\"200\" Name=\"B\"",
            "TrackID=\"200\" Name=\"B\" PlayCount=\"\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("play-count-no-edit-input");
    let output = temporary_path("play-count-no-edit-output");
    fs::write(&input, &source).unwrap();
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

#[test]
fn track_date_added_edit_escapes_xml_and_rejects_stale_guard_without_writing() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" DateAdded=\"old &amp; date_added\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-date-added-input");
    let output = temporary_path("track-date-added-output");
    fs::write(&input, &source).unwrap();
    let invoke = |expected: &str, date_added: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-date-added",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-date-added",
                expected,
                "--date-added",
                date_added,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("old & date_added", "A\tB\nC\rD & <x>", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let encoded = "DateAdded=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "DateAdded=\"old &amp; date_added\"",
        encoded,
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-date-added-stale");
    let stale = invoke("stale", "replacement", &stale_output);
    assert_ne!(stale.status.code(), Some(0), "stale guard accepted");
    assert!(!stale_output.exists(), "stale guard created output");
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn track_last_played_edit_escapes_xml_and_rejects_stale_guard_without_writing() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" LastPlayed=\"old &amp; last_played\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-last-played-input");
    let output = temporary_path("track-last-played-output");
    fs::write(&input, &source).unwrap();
    let invoke = |expected: &str, last_played: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-last-played",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-last-played",
                expected,
                "--last-played",
                last_played,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("old & last_played", "A\tB\nC\rD & <x>", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let encoded = "LastPlayed=\"A&#9;B&#10;C&#13;D &amp; &lt;x&gt;\"";
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "LastPlayed=\"old &amp; last_played\"",
        encoded,
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-last-played-stale");
    let stale = invoke("stale", "replacement", &stale_output);
    assert_ne!(stale.status.code(), Some(0), "stale guard accepted");
    assert!(!stale_output.exists(), "stale guard created output");
    assert_eq!(fs::read(&input).unwrap(), source);
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn track_play_count_edit_preserves_raw_xml_and_rejects_stale_guard() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" PlayCount=\"raw &amp; old\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-play-count-input");
    let output = temporary_path("track-play-count-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-play-count",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-play-count",
                expected,
                "--play-count",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "raw & old", "not numeric & <new>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "PlayCount=\"raw &amp; old\"",
        "PlayCount=\"not numeric &amp; &lt;new&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    let stale_output = temporary_path("track-play-count-stale");
    assert_ne!(
        invoke("100", "stale", "x", &stale_output).status.code(),
        Some(0)
    );
    assert!(!stale_output.exists());
    assert_eq!(fs::read(&input).unwrap(), source);

    let empty_source = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" PlayCount=\"raw &amp; old\"", " PlayCount=\"\"", 1)
        .into_bytes();
    fs::write(&input, &empty_source).unwrap();
    let empty_output = temporary_path("track-play-count-empty");
    let empty = invoke("100", "", "0", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    let expected_empty =
        String::from_utf8(empty_source)
            .unwrap()
            .replacen("PlayCount=\"\"", "PlayCount=\"0\"", 1);
    assert_eq!(fs::read(&empty_output).unwrap(), expected_empty.as_bytes());

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" PlayCount=\"raw &amp; old\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str); 4] = [
        ("missing-track", source.clone(), "999", "raw & old"),
        ("missing-field", missing_field, "100", ""),
        ("invalid-xml-text", source.clone(), "100", "bad\u{1}"),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "raw & old",
        ),
    ];
    for (label, malformed, track_id, expected) in cases {
        fs::write(&input, malformed).unwrap();
        let rejected_output = temporary_path(&format!("track-play-count-{label}"));
        let result = invoke(track_id, expected, "new", &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(stale_output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_average_bpm_edit_preserves_raw_xml_and_rejects_invalid_requests() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" AverageBpm=\"126.12500000000001\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-average-bpm-input");
    let output = temporary_path("track-average-bpm-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-average-bpm",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-average-bpm",
                expected,
                "--average-bpm",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "126.12500000000001", "raw & <odd>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "AverageBpm=\"126.12500000000001\"",
        "AverageBpm=\"raw &amp; &lt;odd&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" AverageBpm=\"126.12500000000001\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str, &str); 5] = [
        ("stale", source.clone(), "100", "stale", "new"),
        (
            "missing-track",
            source.clone(),
            "999",
            "126.12500000000001",
            "new",
        ),
        ("missing-field", missing_field, "100", "", "new"),
        (
            "invalid-xml-text",
            source.clone(),
            "100",
            "126.12500000000001",
            "bad\u{1}",
        ),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "126.12500000000001",
            "new",
        ),
    ];
    for (label, bytes, track_id, old, new) in cases {
        fs::write(&input, bytes).unwrap();
        let rejected_output = temporary_path(&format!("track-average-bpm-{label}"));
        let result = invoke(track_id, old, new, &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }

    let empty_source = String::from_utf8(source).unwrap().replacen(
        " AverageBpm=\"126.12500000000001\"",
        " AverageBpm=\"\"",
        1,
    );
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-average-bpm-empty");
    let empty = invoke("100", "", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("AverageBpm=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_disc_number_edit_preserves_raw_xml_and_rejects_invalid_requests() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" DiscNumber=\"02\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-disc-number-input");
    let output = temporary_path("track-disc-number-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-disc-number",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-disc-number",
                expected,
                "--disc-number",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "02", "raw & <odd>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "DiscNumber=\"02\"",
        "DiscNumber=\"raw &amp; &lt;odd&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" DiscNumber=\"02\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str, &str); 5] = [
        ("stale", source.clone(), "100", "stale", "new"),
        ("missing-track", source.clone(), "999", "02", "new"),
        ("missing-field", missing_field, "100", "", "new"),
        ("invalid-xml-text", source.clone(), "100", "02", "bad\u{1}"),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "02",
            "new",
        ),
    ];
    for (label, bytes, track_id, old, new) in cases {
        fs::write(&input, bytes).unwrap();
        let rejected_output = temporary_path(&format!("track-disc-number-{label}"));
        let result = invoke(track_id, old, new, &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }

    let empty_source =
        String::from_utf8(source)
            .unwrap()
            .replacen(" DiscNumber=\"02\"", " DiscNumber=\"\"", 1);
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-disc-number-empty");
    let empty = invoke("100", "", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("DiscNumber=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_number_edit_preserves_raw_xml_and_rejects_invalid_requests() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" TrackNumber=\"1\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-number-input");
    let output = temporary_path("track-number-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-number",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-track-number",
                expected,
                "--track-number",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "1", "raw & <odd>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "TrackNumber=\"1\"",
        "TrackNumber=\"raw &amp; &lt;odd&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" TrackNumber=\"1\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str, &str); 5] = [
        ("stale", source.clone(), "100", "stale", "new"),
        ("missing-track", source.clone(), "999", "1", "new"),
        ("missing-field", missing_field, "100", "", "new"),
        ("invalid-xml-text", source.clone(), "100", "1", "bad\u{1}"),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "1",
            "new",
        ),
    ];
    for (label, bytes, track_id, old, new) in cases {
        fs::write(&input, bytes).unwrap();
        let rejected_output = temporary_path(&format!("track-number-{label}"));
        let result = invoke(track_id, old, new, &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }

    let empty_source =
        String::from_utf8(source)
            .unwrap()
            .replacen(" TrackNumber=\"1\"", " TrackNumber=\"\"", 1);
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-number-empty");
    let empty = invoke("100", "", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("TrackNumber=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_size_edit_preserves_raw_xml_and_rejects_invalid_requests() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" Size=\"1\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-size-input");
    let output = temporary_path("track-size-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-size",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-size",
                expected,
                "--size",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "1", "raw & <odd>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "Size=\"1\"",
        "Size=\"raw &amp; &lt;odd&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" Size=\"1\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str, &str); 5] = [
        ("stale", source.clone(), "100", "stale", "new"),
        ("missing-track", source.clone(), "999", "1", "new"),
        ("missing-field", missing_field, "100", "", "new"),
        ("invalid-xml-text", source.clone(), "100", "1", "bad\u{1}"),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "1",
            "new",
        ),
    ];
    for (label, bytes, track_id, old, new) in cases {
        fs::write(&input, bytes).unwrap();
        let rejected_output = temporary_path(&format!("track-size-{label}"));
        let result = invoke(track_id, old, new, &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }

    let empty_source = String::from_utf8(source)
        .unwrap()
        .replacen(" Size=\"1\"", " Size=\"\"", 1);
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-size-empty");
    let empty = invoke("100", "", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("Size=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_total_time_edit_preserves_raw_xml_and_rejects_invalid_requests() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" TotalTime=\"3.141592653589793\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-total-time-input");
    let output = temporary_path("track-total-time-output");
    fs::write(&input, &source).unwrap();
    let invoke = |track_id: &str, expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-total-time",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                track_id,
                "--expected-total-time",
                expected,
                "--total-time",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("100", "3.141592653589793", "raw & <odd>\t", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "TotalTime=\"3.141592653589793\"",
        "TotalTime=\"raw &amp; &lt;odd&gt;&#9;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let missing_field = String::from_utf8(source.clone())
        .unwrap()
        .replacen(" TotalTime=\"3.141592653589793\"", "", 1)
        .into_bytes();
    let cases: [(&str, Vec<u8>, &str, &str, &str); 5] = [
        ("stale", source.clone(), "100", "stale", "new"),
        (
            "missing-track",
            source.clone(),
            "999",
            "3.141592653589793",
            "new",
        ),
        ("missing-field", missing_field, "100", "", "new"),
        (
            "invalid-xml-text",
            source.clone(),
            "100",
            "3.141592653589793",
            "bad\u{1}",
        ),
        (
            "malformed-xml",
            b"<DJ_PLAYLISTS><COLLECTION><TRACK TrackID=\"100\"".to_vec(),
            "100",
            "3.141592653589793",
            "new",
        ),
    ];
    for (label, bytes, track_id, old, new) in cases {
        fs::write(&input, bytes).unwrap();
        let rejected_output = temporary_path(&format!("track-total-time-{label}"));
        let result = invoke(track_id, old, new, &rejected_output);
        assert_ne!(result.status.code(), Some(0), "{label} accepted");
        assert!(!rejected_output.exists(), "{label} created output");
        let _ = fs::remove_file(rejected_output);
    }

    let empty_source = String::from_utf8(source).unwrap().replacen(
        " TotalTime=\"3.141592653589793\"",
        " TotalTime=\"\"",
        1,
    );
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-total-time-empty");
    let empty = invoke("100", "", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("TotalTime=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_bit_rate_edit_preserves_raw_xml_and_rejects_stale_values() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" BitRate=\"320.5 kbps\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-bit-rate-input");
    let output = temporary_path("track-bit-rate-output");
    fs::write(&input, &source).unwrap();
    let invoke = |expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-bit-rate",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-bit-rate",
                expected,
                "--bit-rate",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("320.5 kbps", "raw & <odd>", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "BitRate=\"320.5 kbps\"",
        "BitRate=\"raw &amp; &lt;odd&gt;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-bit-rate-stale");
    let stale = invoke("stale", "new", &stale_output);
    assert_ne!(stale.status.code(), Some(0));
    assert!(!stale_output.exists());

    let empty_source =
        String::from_utf8(source)
            .unwrap()
            .replacen(" BitRate=\"320.5 kbps\"", " BitRate=\"\"", 1);
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-bit-rate-empty");
    let empty = invoke("", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("BitRate=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(stale_output);
    let _ = fs::remove_file(empty_output);
}

#[test]
fn track_sample_rate_edit_preserves_raw_xml_and_rejects_stale_values() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-02/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = String::from_utf8(fs::read(fixture).unwrap())
        .unwrap()
        .replacen(
            "TrackID=\"100\" Name=\"A\"",
            "TrackID=\"100\" Name=\"A\" SampleRate=\"48000.0 Hz\"",
            1,
        )
        .into_bytes();
    let input = temporary_path("track-sample-rate-input");
    let output = temporary_path("track-sample-rate-output");
    fs::write(&input, &source).unwrap();
    let invoke = |expected: &str, value: &str, target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args([
                "rekordbox",
                "edit-track-sample-rate",
                "--xml",
                input.to_str().unwrap(),
                "--output",
                target.to_str().unwrap(),
                "--track-id",
                "100",
                "--expected-sample-rate",
                expected,
                "--sample-rate",
                value,
            ])
            .output()
            .unwrap()
    };
    let success = invoke("48000.0 Hz", "raw & <odd>", &output);
    assert_eq!(success.status.code(), Some(0), "{:?}", success.stderr);
    let expected = String::from_utf8(source.clone()).unwrap().replacen(
        "SampleRate=\"48000.0 Hz\"",
        "SampleRate=\"raw &amp; &lt;odd&gt;\"",
        1,
    );
    assert_eq!(fs::read(&output).unwrap(), expected.as_bytes());
    assert_eq!(fs::read(&input).unwrap(), source);

    let stale_output = temporary_path("track-sample-rate-stale");
    let stale = invoke("stale", "new", &stale_output);
    assert_ne!(stale.status.code(), Some(0));
    assert!(!stale_output.exists());

    let empty_source = String::from_utf8(source).unwrap().replacen(
        " SampleRate=\"48000.0 Hz\"",
        " SampleRate=\"\"",
        1,
    );
    fs::write(&input, empty_source.as_bytes()).unwrap();
    let empty_output = temporary_path("track-sample-rate-empty");
    let empty = invoke("", "still raw", &empty_output);
    assert_eq!(empty.status.code(), Some(0), "{:?}", empty.stderr);
    assert!(
        String::from_utf8(fs::read(&empty_output).unwrap())
            .unwrap()
            .contains("SampleRate=\"still raw\"")
    );
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
    let _ = fs::remove_file(stale_output);
    let _ = fs::remove_file(empty_output);
}
