use oxidj_application as _;
use oxidj_core as _;
use oxidj_library::Catalog;
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
    newer_bytes[60..64].copy_from_slice(&3_i32.to_be_bytes());
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
