use oxidj_application as _;
use oxidj_core as _;
use oxidj_library::Catalog;
use quick_xml as _;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .output()
        .expect("failed to run oxidj-cli")
}

fn database_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidj-catalog-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn set_rollback_journal_header_for_fixture(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[18..20], &[2, 2]);
    // SQLite format header uses 1 for rollback journaling and 2 for WAL.
    bytes[18..20].copy_from_slice(&[1, 1]);
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn fresh_process_audio_facts_reader() {
    let database = std::env::var_os("OXIDJ_CATALOG_FACTS_CHILD_DB");
    if let Some(database) = database {
        let catalog = Catalog::open(database).expect("child opens persisted catalog");
        let details = catalog
            .list_track_details()
            .expect("read persisted details");
        assert_eq!(details.len(), 1);
        let detail = &details[0];
        assert_eq!(detail.track.id.as_u64(), 1);
        assert_eq!(
            detail.track.path,
            std::env::var("OXIDJ_CATALOG_FACTS_CHILD_PATH").unwrap()
        );
        assert_eq!(detail.duration_seconds, Some(4.0 / 48_000.0));
        assert_eq!(detail.sample_rate, Some(48_000));
        assert_eq!(detail.channels, Some(2));
        assert_eq!(detail.bit_depth, Some(16));
    }
}

#[test]
fn imported_audio_facts_survive_fresh_process_reopen() {
    let database = database_path();
    let root = database.with_extension("audio");
    std::fs::create_dir(&root).unwrap();
    let wav = root.join("known.wav");
    let samples = [1_i16, 2, 3, 4, 5, 6, 7, 8];
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32 + data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&192_000_u32.to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data);
    std::fs::write(&wav, bytes).unwrap();

    let mut catalog = Catalog::open(&database).unwrap();
    let report = catalog.import_wav_directory(&root).unwrap();
    assert!(report.complete);
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(catalog.list_tracks().unwrap()[0].id.as_u64(), 1);
    drop(catalog);

    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fresh_process_audio_facts_reader", "--nocapture"])
        .env("OXIDJ_CATALOG_FACTS_CHILD_DB", &database)
        .env("OXIDJ_CATALOG_FACTS_CHILD_PATH", &wav)
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(0),
        "child stdout: {:?}; stderr: {:?}",
        child.stdout,
        child.stderr
    );

    let _ = std::fs::remove_file(database);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn catalog_record_survives_a_fresh_cli_process() {
    let database = database_path();
    let database_arg = database.to_str().expect("temporary path is not UTF-8");

    let init = run(&["catalog", "init", "--db", database_arg]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init stderr: {:?}",
        init.stderr
    );
    assert!(database.is_file());

    let list = run(&["catalog", "list", "--db", database_arg]);
    assert_eq!(
        list.status.code(),
        Some(0),
        "list stderr: {:?}",
        list.stderr
    );
    assert_eq!(list.stdout, b"TrackId\tPath\tSHA-256\tBytes\n");
    assert!(list.stderr.is_empty());

    let mut catalog = Catalog::open(&database).expect("open catalog through library API");
    let inserted = catalog
        .insert_track("operation-1", "/scratch/synthetic.wav", [0xab; 32], 7)
        .expect("insert synthetic track");
    assert_eq!(inserted.id.as_u64(), 1);
    assert_eq!(
        catalog.read_track(inserted.id).expect("read track"),
        Some(inserted)
    );
    drop(catalog);

    let reopened = run(&["catalog", "list", "--db", database_arg]);
    assert_eq!(
        reopened.status.code(),
        Some(0),
        "list stderr: {:?}",
        reopened.stderr
    );
    assert_eq!(
        reopened.stdout,
        b"TrackId\tPath\tSHA-256\tBytes\n1\t/scratch/synthetic.wav\tabababababababababababababababababababababababababababababababab\t7\n"
    );
    assert!(reopened.stderr.is_empty());

    std::fs::remove_file(database).expect("remove temporary catalog");
}

#[test]
fn catalog_backup_and_fresh_path_restore_preserve_records_and_refuse_unsafe_paths() {
    let database = database_path();
    let backup = database.with_file_name(format!(
        "backup-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    let restored = database.with_file_name(format!(
        "restored-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    let db_arg = database.to_str().unwrap();
    let backup_arg = backup.to_str().unwrap();
    let restored_arg = restored.to_str().unwrap();

    assert_eq!(
        run(&["catalog", "init", "--db", db_arg]).status.code(),
        Some(0)
    );
    let mut catalog = Catalog::open(&database).unwrap();
    let expected = catalog
        .insert_track("backup-op", "/synthetic/backup.wav", [0x5a; 32], 1234)
        .unwrap();
    assert_eq!(
        catalog
            .insert_track("backup-op", "/synthetic/backup.wav", [0x5a; 32], 1234)
            .unwrap(),
        expected
    );
    drop(catalog);

    let backup_output = run(&["catalog", "backup", "--db", db_arg, "--to", backup_arg]);
    assert_eq!(
        backup_output.status.code(),
        Some(0),
        "backup stderr: {:?}",
        backup_output.stderr
    );
    assert!(backup.is_file());
    let restore_output = run(&[
        "catalog",
        "restore",
        "--from",
        backup_arg,
        "--db",
        restored_arg,
    ]);
    assert_eq!(
        restore_output.status.code(),
        Some(0),
        "restore stderr: {:?}",
        restore_output.stderr
    );

    let list = run(&["catalog", "list", "--db", restored_arg]);
    assert_eq!(
        list.status.code(),
        Some(0),
        "list stderr: {:?}",
        list.stderr
    );
    assert_eq!(list.stdout, b"TrackId\tPath\tSHA-256\tBytes\n1\t/synthetic/backup.wav\t5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a\t1234\n");
    let mut restored_catalog = Catalog::open(&restored).unwrap();
    assert_eq!(
        restored_catalog
            .insert_track("backup-op", "/synthetic/backup.wav", [0x5a; 32], 1234)
            .unwrap(),
        expected
    );
    drop(restored_catalog);

    let backup_before = std::fs::read(&backup).unwrap();
    let existing_backup = run(&["catalog", "backup", "--db", db_arg, "--to", backup_arg]);
    assert_ne!(existing_backup.status.code(), Some(0));
    assert_eq!(std::fs::read(&backup).unwrap(), backup_before);
    let restored_before = std::fs::read(&restored).unwrap();
    let existing_restore = run(&[
        "catalog",
        "restore",
        "--from",
        backup_arg,
        "--db",
        restored_arg,
    ]);
    assert_ne!(existing_restore.status.code(), Some(0));
    assert_eq!(std::fs::read(&restored).unwrap(), restored_before);

    let corrupt = database.with_file_name(format!(
        "corrupt-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    let corrupt_destination = database.with_file_name(format!(
        "bad-restore-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&corrupt, b"not a sqlite database").unwrap();
    let corrupt_before = std::fs::read(&corrupt).unwrap();
    let corrupt_result = run(&[
        "catalog",
        "restore",
        "--from",
        corrupt.to_str().unwrap(),
        "--db",
        corrupt_destination.to_str().unwrap(),
    ]);
    assert_ne!(corrupt_result.status.code(), Some(0));
    assert!(!corrupt_destination.exists());
    assert_eq!(std::fs::read(&corrupt).unwrap(), corrupt_before);

    let newer_schema = database.with_file_name(format!(
        "newer-schema-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    let newer_destination = database.with_file_name(format!(
        "newer-restore-{}",
        database.file_name().unwrap().to_string_lossy()
    ));
    let mut newer_bytes = std::fs::read(&backup).unwrap();
    newer_bytes[60..64].copy_from_slice(&99_i32.to_be_bytes());
    std::fs::write(&newer_schema, &newer_bytes).unwrap();
    let newer_before = std::fs::read(&newer_schema).unwrap();
    let newer_result = run(&[
        "catalog",
        "restore",
        "--from",
        newer_schema.to_str().unwrap(),
        "--db",
        newer_destination.to_str().unwrap(),
    ]);
    assert_ne!(newer_result.status.code(), Some(0));
    assert!(!newer_destination.exists());
    assert_eq!(std::fs::read(&newer_schema).unwrap(), newer_before);

    for path in [database, backup, restored, corrupt, newer_schema] {
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn backup_of_missing_source_creates_neither_source_nor_destination() {
    let source = database_path();
    let destination = source.with_file_name(format!(
        "missing-backup-{}",
        source.file_name().unwrap().to_string_lossy()
    ));
    let result = run(&[
        "catalog",
        "backup",
        "--db",
        source.to_str().unwrap(),
        "--to",
        destination.to_str().unwrap(),
    ]);
    assert_ne!(result.status.code(), Some(0));
    assert!(!source.exists(), "missing source was created");
    assert!(
        !destination.exists(),
        "backup of missing source was published"
    );
}

#[test]
fn backup_rejects_source_destination_filesystem_alias_without_mutation() {
    let source = database_path();
    let seed = database_path();
    let alias = source.with_file_name(format!(
        "alias-{}",
        source.file_name().unwrap().to_string_lossy()
    ));
    Catalog::open(&seed).unwrap().backup_to(&source).unwrap();
    set_rollback_journal_header_for_fixture(&source);
    let _ = std::fs::remove_file(seed);
    let bytes_before = std::fs::read(&source).unwrap();
    #[cfg(unix)]
    std::fs::hard_link(&source, &alias).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::hard_link(&source, &alias).unwrap();
    let alias_before = std::fs::read(&alias).unwrap();

    let result = run(&[
        "catalog",
        "backup",
        "--db",
        source.to_str().unwrap(),
        "--to",
        alias.to_str().unwrap(),
    ]);
    assert_ne!(result.status.code(), Some(0));
    assert!(
        std::fs::read(&source).unwrap() == bytes_before,
        "backup changed source bytes"
    );
    assert!(
        std::fs::read(&alias).unwrap() == alias_before,
        "backup changed alias bytes"
    );

    let _ = std::fs::remove_file(alias);
    let _ = std::fs::remove_file(source);
}

#[test]
fn backup_rejects_identical_source_and_destination_without_mutation() {
    let source = database_path();
    let seed = database_path();
    Catalog::open(&seed).unwrap().backup_to(&source).unwrap();
    set_rollback_journal_header_for_fixture(&source);
    let _ = std::fs::remove_file(seed);
    let before = std::fs::read(&source).unwrap();
    let result = run(&[
        "catalog",
        "backup",
        "--db",
        source.to_str().unwrap(),
        "--to",
        source.to_str().unwrap(),
    ]);
    assert_ne!(result.status.code(), Some(0));
    assert!(
        std::fs::read(&source).unwrap() == before,
        "backup changed source bytes"
    );
    let _ = std::fs::remove_file(source);
}

#[test]
fn analyze_waveform_command_stores_peaks_idempotently_and_refuses_bad_input() {
    let database = database_path();
    let db = database.to_str().unwrap();
    let root = database.with_extension("waveform-audio");
    std::fs::create_dir(&root).unwrap();
    let flac = include_bytes!("../../oxidj-library/tests/fixtures/tone.flac");
    std::fs::write(root.join("tone.flac"), flac).unwrap();
    std::fs::write(root.join("broken.flac"), b"fLaC broken").unwrap();
    let import = run(&[
        "catalog",
        "import",
        "--dir",
        root.to_str().unwrap(),
        "--db",
        db,
    ]);
    assert_eq!(import.status.code(), Some(0), "{:?}", import.stderr);

    let first = run(&["catalog", "analyze-waveform", "--db", db]);
    assert_eq!(first.status.code(), Some(0), "{:?}", first.stderr);
    assert_eq!(
        String::from_utf8(first.stdout).unwrap(),
        "Analyzed\t1\nAnalyzed: 1\nUnchanged: 0\nFailed: 0\n"
    );
    let stored = Catalog::open(&database)
        .unwrap()
        .read_waveform(oxidj_core::TrackId::new(1).unwrap())
        .unwrap()
        .expect("peaks stored");
    let again = run(&["catalog", "analyze-waveform", "--track-id", "1", "--db", db]);
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(again.stdout).unwrap(),
        "Unchanged\t1\nAnalyzed: 0\nUnchanged: 1\nFailed: 0\n"
    );
    let reread = Catalog::open(&database)
        .unwrap()
        .read_waveform(oxidj_core::TrackId::new(1).unwrap())
        .unwrap();
    assert_eq!(reread, Some(stored));
    assert_eq!(std::fs::read(root.join("tone.flac")).unwrap(), flac);

    for args in [
        vec!["catalog", "analyze-waveform", "--db", db, "--track-id", "7"],
        vec![
            "catalog",
            "analyze-waveform",
            "--db",
            "/nonexistent/oxidj.sqlite",
        ],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(out.stdout.is_empty());
    }
    for args in [
        vec!["catalog", "analyze-waveform"],
        vec!["catalog", "analyze-waveform", "--db", db, "--track-id", "0"],
        vec!["catalog", "analyze-waveform", "--db", db, "--bogus"],
    ] {
        assert_eq!(run(&args).status.code(), Some(2), "{args:?}");
    }
    assert!(!std::path::Path::new("/nonexistent/oxidj.sqlite").exists());
    let _ = std::fs::remove_file(&database);
    let _ = std::fs::remove_dir_all(root);
}

/// A1: the same 24-bit stereo signal stored as WAV and as AIFF (big-endian, odd
/// padding-free) yields identical stored peaks through `catalog analyze-waveform`.
#[test]
fn analyze_waveform_aiff_and_wav_of_the_same_signal_store_identical_peaks() {
    let database = database_path();
    let db = database.to_str().unwrap();
    let root = database.with_extension("aiff-audio");
    std::fs::create_dir(&root).unwrap();
    let (rate, frames) = (22_050_u32, 30_000_usize);
    let samples: Vec<i32> = (0..frames * 2)
        .map(|i| {
            let t = (i / 2) as f64 / f64::from(rate);
            let hz = if i % 2 == 0 { 440.0 } else { 97.0 };
            (f64::sin(t * hz * std::f64::consts::TAU) * 8_000_000.0 * (1.0 - t / 2.0)) as i32
        })
        .collect();
    let data_len = (samples.len() * 3) as u32;

    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 6).to_le_bytes());
    wav.extend_from_slice(&6_u16.to_le_bytes());
    wav.extend_from_slice(&24_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in &samples {
        wav.extend_from_slice(&s.to_le_bytes()[..3]);
    }

    let mut comm = 2_u16.to_be_bytes().to_vec();
    comm.extend_from_slice(&(frames as u32).to_be_bytes());
    comm.extend_from_slice(&24_u16.to_be_bytes());
    let msb = 31 - rate.leading_zeros();
    comm.extend_from_slice(&(16_383 + msb as u16).to_be_bytes());
    comm.extend_from_slice(&(u64::from(rate) << (63 - msb)).to_be_bytes());
    let mut body = b"AIFFCOMM".to_vec();
    body.extend_from_slice(&18_u32.to_be_bytes());
    body.extend_from_slice(&comm);
    body.extend_from_slice(b"SSND");
    body.extend_from_slice(&(8 + data_len).to_be_bytes());
    body.extend_from_slice(&[0; 8]);
    for s in &samples {
        body.extend_from_slice(&s.to_be_bytes()[1..]);
    }
    let mut aiff = b"FORM".to_vec();
    aiff.extend_from_slice(&(body.len() as u32).to_be_bytes());
    aiff.extend_from_slice(&body);

    std::fs::write(root.join("a.wav"), &wav).unwrap();
    // Real-library writer quirk: odd final `ID3 ` chunk whose pad byte the
    // FORM size counts but the file omits (FORM overruns the file by 1).
    let mut tagged = aiff.clone();
    tagged.extend_from_slice(b"ID3 ");
    tagged.extend_from_slice(&3_u32.to_be_bytes());
    tagged.extend_from_slice(b"tag");
    let size = (tagged.len() - 8 + 1) as u32;
    tagged[4..8].copy_from_slice(&size.to_be_bytes());

    // A2: dominant real-library shape: EVEN final `ID3 ` chunk ending at EOF
    // with the FORM size one byte larger than the file.
    let mut even = aiff.clone();
    even.extend_from_slice(b"ID3 ");
    even.extend_from_slice(&4_u32.to_be_bytes());
    even.extend_from_slice(b"tags");
    let size = (even.len() - 8 + 1) as u32;
    even[4..8].copy_from_slice(&size.to_be_bytes());

    std::fs::write(root.join("b.aiff"), &aiff).unwrap();
    std::fs::write(root.join("c.aiff"), &tagged).unwrap();
    std::fs::write(root.join("d.aiff"), &even).unwrap();
    let import = run(&[
        "catalog",
        "import",
        "--dir",
        root.to_str().unwrap(),
        "--db",
        db,
    ]);
    assert_eq!(import.status.code(), Some(0), "{:?}", import.stderr);
    let out = run(&["catalog", "analyze-waveform", "--db", db]);
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "Analyzed\t1\nAnalyzed\t2\nAnalyzed\t3\nAnalyzed\t4\nAnalyzed: 4\nUnchanged: 0\nFailed: 0\n"
    );
    let catalog = Catalog::open(&database).unwrap();
    let details = catalog.list_track_details().unwrap();
    assert_eq!(details.len(), 4);
    for detail in &details {
        assert_eq!(detail.sample_rate, Some(rate));
        assert_eq!(detail.channels, Some(2));
        assert_eq!(detail.bit_depth, Some(24));
        assert_eq!(
            detail.duration_seconds,
            Some(frames as f64 / f64::from(rate))
        );
    }
    let peaks = |id| {
        catalog
            .read_waveform(oxidj_core::TrackId::new(id).unwrap())
            .unwrap()
            .expect("peaks stored")
            .peaks
    };
    let (wav_peaks, aiff_peaks) = (peaks(1), peaks(2));
    assert!(wav_peaks.level_count() > 1);
    assert_eq!(aiff_peaks, wav_peaks);
    assert_eq!(peaks(3), wav_peaks);
    assert_eq!(peaks(4), wav_peaks);
    assert_eq!(std::fs::read(root.join("b.aiff")).unwrap(), aiff);
    assert_eq!(std::fs::read(root.join("c.aiff")).unwrap(), tagged);
    assert_eq!(std::fs::read(root.join("d.aiff")).unwrap(), even);
    drop(catalog);
    let _ = std::fs::remove_file(&database);
    let _ = std::fs::remove_dir_all(root);
}

/// 16-bit mono 22.05 kHz, 30 s WAV of a kick+click track at `bpm` (first beat
/// at 0.3 s); silence when `bpm` is 0.
fn click_wav(bpm: f64) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let n = 30 * RATE as usize;
    let mut out = vec![0.0f64; n];
    if bpm > 0.0 {
        let len = (0.25 * f64::from(RATE)) as usize;
        let mut hit = 0.3;
        while hit < 30.0 {
            let start = (hit * f64::from(RATE)).round() as usize;
            for (i, slot) in out.iter_mut().skip(start).take(len).enumerate() {
                let t = i as f64 / f64::from(RATE);
                *slot += 0.6 * (std::f64::consts::TAU * 55.0 * t).sin() * (-t / 0.06).exp()
                    + 0.3 * (std::f64::consts::TAU * 1_800.0 * t).sin() * (-t / 0.004).exp();
            }
            hit += 60.0 / bpm;
        }
    }
    let data: Vec<u8> = out
        .iter()
        .flat_map(|s| ((s * 32_000.0) as i16).to_le_bytes())
        .collect();
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(data);
    wav
}

fn assert_grid(grid: &oxidj_core::tempo::TempoGrid, bpm: f64) {
    assert!((grid.bpm() - bpm).abs() <= 0.5, "bpm {}", grid.bpm());
    assert!(grid.beats_ms().len() > 50);
    let period = 60.0 / bpm;
    for &ms in grid.beats_ms() {
        let s = f64::from(ms) / 1000.0;
        let off = (s - (0.3 + ((s - 0.3) / period).round() * period)).abs();
        assert!(off <= 0.030, "beat {ms} ms off by {off}");
    }
}

/// B2b: `catalog analyze-tempo` end to end — stored 128 BPM grid, Unchanged on
/// rerun, re-analysis after a source change, silence → NoGrid with no row,
/// corrupt → Failed while others continue, `--track-id`, range refusal, and
/// the imported vendor grid/waveforms left untouched.
#[test]
fn analyze_tempo_command_stores_grids_and_keeps_vendor_data_untouched() {
    use oxidj_library::{LibraryBeat, LibrarySnapshot, LibraryTrack, VendorWaveform};
    let database = database_path();
    let db = database.to_str().unwrap();
    let root = database.with_extension("tempo-audio");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a-click.wav"), click_wav(128.0)).unwrap();
    std::fs::write(root.join("b-silence.wav"), click_wav(0.0)).unwrap();
    std::fs::write(root.join("c-corrupt.wav"), click_wav(120.0)).unwrap();
    let import = run(&[
        "catalog",
        "import",
        "--dir",
        root.to_str().unwrap(),
        "--db",
        db,
    ]);
    assert_eq!(import.status.code(), Some(0), "{:?}", import.stderr);
    // Import refuses a truncated WAV, so corrupt the catalogued file afterwards.
    let mut corrupt = click_wav(120.0);
    corrupt.truncate(corrupt.len() - 1_001);
    std::fs::write(root.join("c-corrupt.wav"), &corrupt).unwrap();
    let location = format!("file://localhost{}", root.join("a-click.wav").display());
    let vendor_beats = vec![
        LibraryBeat {
            time_ms: 11,
            tempo_hundredths_bpm: 12_345,
            beat_in_bar: 1,
        },
        LibraryBeat {
            time_ms: 497,
            tempo_hundredths_bpm: 12_345,
            beat_in_bar: 2,
        },
    ];
    let vendor_waveform = vec![VendorWaveform::RekordboxDetail(vec![[9, 3]])];
    let vendor = |catalog: &Catalog| {
        (
            catalog.library_snapshot().unwrap(),
            catalog.beat_grid_for_locations(&[&location]).unwrap(),
            catalog
                .vendor_waveforms_for_locations(&[&location])
                .unwrap(),
        )
    };
    let vendor_before = {
        let mut catalog = Catalog::open(&database).unwrap();
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "Vendor".into(),
                    tempo_hundredths_bpm: 12_345,
                    location: location.clone(),
                    beat_grid: vendor_beats.clone(),
                    vendor_waveforms: vendor_waveform.clone(),
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        vendor(&catalog)
    };
    assert_eq!(vendor_before.1, vendor_beats);
    let sources: Vec<Vec<u8>> = ["a-click.wav", "b-silence.wav", "c-corrupt.wav"]
        .iter()
        .map(|n| std::fs::read(root.join(n)).unwrap())
        .collect();
    let id = |n| oxidj_core::TrackId::new(n).unwrap();
    let stdout = |out: &Output| String::from_utf8(out.stdout.clone()).unwrap();

    let first = run(&["catalog", "analyze-tempo", "--db", db]);
    assert_eq!(
        first.status.code(),
        Some(1),
        "a failed track is a non-zero exit: {}",
        String::from_utf8_lossy(&first.stdout)
    );
    assert_eq!(
        stdout(&first),
        "Analyzed\t1\nNoGrid\t2\nFailed\t3\tcorrupt or truncated audio\n\
         Analyzed: 1\nUnchanged: 0\nNoGrid: 1\nFailed: 1\n"
    );
    let catalog = Catalog::open(&database).unwrap();
    let stored = catalog
        .computed_beat_grid(id(1))
        .unwrap()
        .expect("grid stored");
    assert_grid(&stored.grid, 128.0);
    assert_eq!(stored.grid.algorithm(), "taktus 0.1.0");
    assert_eq!(
        (stored.basis.range_min_bpm, stored.basis.range_max_bpm),
        (70, 180)
    );
    assert_eq!(catalog.computed_beat_grid(id(2)).unwrap(), None);
    assert_eq!(catalog.computed_beat_grid(id(3)).unwrap(), None);
    assert_eq!(vendor(&catalog), vendor_before, "vendor data untouched");
    drop(catalog);

    let again = run(&["catalog", "analyze-tempo", "--track-id", "1", "--db", db]);
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(
        stdout(&again),
        "Unchanged\t1\nAnalyzed: 0\nUnchanged: 1\nNoGrid: 0\nFailed: 0\n"
    );
    let silent = run(&["catalog", "analyze-tempo", "--db", db, "--track-id", "2"]);
    assert_eq!(silent.status.code(), Some(0), "NoGrid is not a failure");
    assert_eq!(
        stdout(&silent),
        "NoGrid\t2\nAnalyzed: 0\nUnchanged: 0\nNoGrid: 1\nFailed: 0\n"
    );
    let sources_after: Vec<Vec<u8>> = ["a-click.wav", "b-silence.wav", "c-corrupt.wav"]
        .iter()
        .map(|n| std::fs::read(root.join(n)).unwrap())
        .collect();
    assert_eq!(sources_after, sources, "sources are only read");

    // Changed source: re-analysed at its new tempo; a custom range is honoured.
    std::fs::write(root.join("a-click.wav"), click_wav(140.0)).unwrap();
    let changed = run(&[
        "catalog",
        "analyze-tempo",
        "--db",
        db,
        "--bpm-range",
        "60-200",
        "--track-id",
        "1",
    ]);
    assert_eq!(changed.status.code(), Some(0), "{:?}", changed.stderr);
    assert_eq!(
        stdout(&changed),
        "Analyzed\t1\nAnalyzed: 1\nUnchanged: 0\nNoGrid: 0\nFailed: 0\n"
    );
    let catalog = Catalog::open(&database).unwrap();
    let stored = catalog.computed_beat_grid(id(1)).unwrap().unwrap();
    assert_grid(&stored.grid, 140.0);
    assert_eq!(
        (stored.basis.range_min_bpm, stored.basis.range_max_bpm),
        (60, 200)
    );
    assert_eq!(vendor(&catalog), vendor_before, "vendor data untouched");
    drop(catalog);

    for args in [
        vec!["catalog", "analyze-tempo", "--db", db, "--track-id", "7"],
        vec![
            "catalog",
            "analyze-tempo",
            "--db",
            "/nonexistent/oxidj.sqlite",
        ],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(out.stdout.is_empty());
    }
    for bad in [
        "100-150",
        "0-180",
        "180-70",
        "70",
        "70-",
        "x-180",
        "70-180-200",
    ] {
        let out = run(&["catalog", "analyze-tempo", "--db", db, "--bpm-range", bad]);
        assert_eq!(out.status.code(), Some(2), "{bad}");
        assert!(out.stdout.is_empty(), "{bad}: nothing analysed");
    }
    for args in [
        vec!["catalog", "analyze-tempo"],
        vec!["catalog", "analyze-tempo", "--db", db, "--track-id", "0"],
        vec!["catalog", "analyze-tempo", "--db", db, "--bogus", "x"],
        vec!["catalog", "analyze-tempo", "--db", db, "--db", db],
        vec!["catalog", "analyze-tempo", "--db"],
    ] {
        assert_eq!(run(&args).status.code(), Some(2), "{args:?}");
    }
    // Refusals changed nothing.
    let catalog = Catalog::open(&database).unwrap();
    assert_eq!(catalog.computed_beat_grid(id(1)).unwrap().unwrap(), stored);
    assert!(!std::path::Path::new("/nonexistent/oxidj.sqlite").exists());
    drop(catalog);
    let _ = std::fs::remove_file(&database);
    let _ = std::fs::remove_dir_all(root);
}
