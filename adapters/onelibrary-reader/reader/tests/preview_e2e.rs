use rusqlite::{Connection, OpenFlags};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const SECRET: &[u8] = b"synthetic-test-secret-9f24";

fn temp_db() -> std::path::PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("oxidj-onelibrary-{n}.db"))
}

fn encrypted_fixture(path: &std::path::Path) {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .unwrap();
    // SAFETY: test-only key bytes live for the call; fixture uses no personal data.
    let rc = unsafe {
        rusqlite::ffi::sqlite3_key(conn.handle(), SECRET.as_ptr().cast(), SECRET.len() as i32)
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    conn.execute_batch("PRAGMA user_version=1;
        CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT, artist TEXT, album TEXT);
        CREATE TABLE playlists(id TEXT PRIMARY KEY, parent_id TEXT REFERENCES playlists(id), sibling_order INTEGER NOT NULL, name TEXT NOT NULL);
        CREATE TABLE playlist_tracks(playlist_id TEXT NOT NULL REFERENCES playlists(id), position INTEGER NOT NULL, track_id TEXT NOT NULL REFERENCES tracks(id));
        INSERT INTO tracks VALUES('src:1','Night & Day','A DJ','Blue'),('src:2','Second','B DJ','Green');
        INSERT INTO playlists VALUES('p-root',NULL,0,'Root'),('p-child','p-root',0,'Set'),('p-dupe-a',NULL,1,'Twin'),('p-dupe-b',NULL,2,'Twin');
        INSERT INTO playlist_tracks VALUES('p-child',0,'src:1'),('p-child',1,'src:1'),('p-child',2,'src:2');").unwrap();
}

fn run(path: &std::path::Path, key: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_oxidj-onelibrary-reader"))
        .arg("--key-stdin")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(key).unwrap();
    child.wait_with_output().unwrap()
}

fn mutate_fixture(path: &std::path::Path, sql: &str) {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let rc = unsafe {
        rusqlite::ffi::sqlite3_key(conn.handle(), SECRET.as_ptr().cast(), SECRET.len() as i32)
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    conn.execute_batch(sql).unwrap();
}

fn assert_rejected_unchanged(path: &std::path::Path) {
    let before = std::fs::read(path).unwrap();
    let output = run(path, SECRET);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "OneLibrary reader failed"
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn encrypted_synthetic_profile_produces_complete_preview_without_mutating_source() {
    let path = temp_db();
    encrypted_fixture(&path);
    let before = std::fs::read(&path).unwrap();
    let output = run(&path, SECRET);
    assert!(
        output.status.success(),
        "reader should accept the synthetic encrypted profile; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["protocol_version"], 1);
    assert_eq!(json["schema_profile"], 1);
    assert_eq!(json["tracks"].as_array().unwrap().len(), 2);
    assert_eq!(json["tracks"][0]["title"], "Night & Day");
    assert_eq!(json["tracks"][0]["artist"], "A DJ");
    assert_eq!(json["tracks"][0]["album"], "Blue");
    assert!(json["tracks"][0].get("duration_ms").is_none());
    let child = json["playlists"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "p-child")
        .unwrap();
    assert_eq!(child["parent_id"], "p-root");
    assert_eq!(
        child["track_ids"],
        serde_json::json!(["src:1", "src:1", "src:2"])
    );
    assert_eq!(
        json["playlists"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["name"] == "Twin")
            .count(),
        2
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_orphan_membership_owner_and_generated_schema_column() {
    let orphan = temp_db();
    encrypted_fixture(&orphan);
    mutate_fixture(
        &orphan,
        "PRAGMA foreign_keys=OFF; INSERT INTO playlist_tracks VALUES('missing-playlist',0,'src:1')",
    );
    assert_rejected_unchanged(&orphan);
    let generated = temp_db();
    encrypted_fixture(&generated);
    mutate_fixture(
        &generated,
        "ALTER TABLE tracks ADD COLUMN unexpected TEXT GENERATED ALWAYS AS (title) VIRTUAL",
    );
    assert_rejected_unchanged(&generated);
    let _ = std::fs::remove_file(orphan);
    let _ = std::fs::remove_file(generated);
}

#[test]
fn rejects_unsupported_version_table_and_column_without_partial_output() {
    for sql in [
        "PRAGMA user_version=2",
        "CREATE TABLE unexpected(id TEXT)",
        "ALTER TABLE tracks ADD COLUMN unexpected TEXT",
    ] {
        let path = temp_db();
        encrypted_fixture(&path);
        mutate_fixture(&path, sql);
        assert_rejected_unchanged(&path);
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn rejects_dangling_relations_and_invalid_ordering_without_partial_output() {
    for sql in [
        "PRAGMA foreign_keys=OFF; INSERT INTO playlist_tracks VALUES('p-child',3,'missing-track')",
        "PRAGMA foreign_keys=OFF; UPDATE playlists SET parent_id='missing-parent' WHERE id='p-child'",
        "UPDATE playlists SET sibling_order=4 WHERE id='p-dupe-a'",
        "UPDATE playlist_tracks SET position=4 WHERE playlist_id='p-child' AND position=1",
    ] {
        let path = temp_db();
        encrypted_fixture(&path);
        mutate_fixture(&path, sql);
        assert_rejected_unchanged(&path);
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn rejects_missing_schema_and_invalid_key_bounds_without_output() {
    let missing = temp_db();
    encrypted_fixture(&missing);
    mutate_fixture(&missing, "DROP TABLE playlist_tracks");
    assert_rejected_unchanged(&missing);
    let path = temp_db();
    encrypted_fixture(&path);
    let before = std::fs::read(&path).unwrap();
    for key in [b"".as_slice(), &vec![b'x'; 4097]] {
        let output = run(&path, key);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            "OneLibrary reader failed"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let _ = std::fs::remove_file(missing);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_each_present_sqlite_sidecar_without_output() {
    for suffix in ["-wal", "-shm", "-journal"] {
        let path = temp_db();
        encrypted_fixture(&path);
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = std::path::PathBuf::from(sidecar);
        std::fs::write(&sidecar, b"synthetic sidecar").unwrap();
        let before = std::fs::read(&path).unwrap();
        let output = run(&path, SECRET);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            "OneLibrary reader failed"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let _ = std::fs::remove_file(sidecar);
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn rejects_duplicate_ids_and_parent_cycles_without_partial_output() {
    // Rebuild with duplicate track IDs but the exact profile columns, proving the reader
    // rejects ambiguous source identity rather than relying on a vendor uniqueness constraint.
    let duplicate = temp_db();
    encrypted_fixture(&duplicate);
    mutate_fixture(
        &duplicate,
        "DROP TABLE playlist_tracks; DROP TABLE tracks; CREATE TABLE tracks(id TEXT, title TEXT, artist TEXT, album TEXT); INSERT INTO tracks VALUES('src:1','one',NULL,NULL),('src:1','two',NULL,NULL); CREATE TABLE playlist_tracks(playlist_id TEXT NOT NULL, position INTEGER NOT NULL, track_id TEXT NOT NULL); INSERT INTO playlist_tracks VALUES('p-child',0,'src:1');",
    );
    assert_rejected_unchanged(&duplicate);

    let duplicate_playlist = temp_db();
    encrypted_fixture(&duplicate_playlist);
    mutate_fixture(
        &duplicate_playlist,
        "DROP TABLE playlist_tracks; DROP TABLE playlists; CREATE TABLE playlists(id TEXT, parent_id TEXT, sibling_order INTEGER NOT NULL, name TEXT NOT NULL); INSERT INTO playlists VALUES('p-root',NULL,0,'Root'),('p-root',NULL,1,'Again'); CREATE TABLE playlist_tracks(playlist_id TEXT NOT NULL, position INTEGER NOT NULL, track_id TEXT NOT NULL);",
    );
    assert_rejected_unchanged(&duplicate_playlist);

    let cycle = temp_db();
    encrypted_fixture(&cycle);
    mutate_fixture(
        &cycle,
        "UPDATE playlists SET parent_id='p-child' WHERE id='p-root'",
    );
    assert_rejected_unchanged(&cycle);
    let _ = std::fs::remove_file(duplicate);
    let _ = std::fs::remove_file(duplicate_playlist);
    let _ = std::fs::remove_file(cycle);
}

#[test]
fn cli_application_sibling_reader_success_and_failure_are_end_to_end() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap();
    let build = std::process::Command::new("cargo")
        .args([
            "build",
            "--package",
            "oxidj-cli",
            "--bin",
            "oxidj-cli",
            "--locked",
            "--offline",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(build.success());

    let dir = temp_db().with_extension("bundle");
    std::fs::create_dir_all(&dir).unwrap();
    let cli = dir.join("oxidj-cli");
    let reader = dir.join("oxidj-onelibrary-reader");
    std::fs::copy(root.join("target/debug/oxidj-cli"), &cli).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/debug/oxidj-onelibrary-reader"),
        &reader,
    )
    .unwrap();
    let path = dir.join("synthetic.db");
    encrypted_fixture(&path);
    let before = std::fs::read(&path).unwrap();
    for (key, expected_success) in [(SECRET, true), (b"wrong-key".as_slice(), false)] {
        let mut child = std::process::Command::new(&cli)
            .args(["onelibrary", "preview", "--db"])
            .arg(&path)
            .arg("--key-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(key).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.success(), expected_success);
        if expected_success {
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("Title=Some(\"Night & Day\")"));
            assert!(text.contains("p-child") && text.contains("Parent=Some(\"p-root\")"));
        } else {
            assert!(output.stdout.is_empty());
            assert_eq!(
                String::from_utf8_lossy(&output.stderr).trim(),
                "error: OneLibrary preview rejected input"
            );
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn wrong_key_fails_without_output_and_does_not_disclose_secret() {
    let path = temp_db();
    encrypted_fixture(&path);
    let before = std::fs::read(&path).unwrap();
    let output = run(&path, b"wrong synthetic key");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "failed reads must not emit partial protocol output"
    );
    let secret = String::from_utf8_lossy(SECRET);
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret.as_ref()));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let _ = std::fs::remove_file(path);
}
