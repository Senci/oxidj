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
    let rc = unsafe { rusqlite::ffi::sqlite3_key(conn.handle(), SECRET.as_ptr().cast(), SECRET.len() as i32) };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    conn.execute_batch(sql).unwrap();
}

fn assert_rejected_unchanged(path: &std::path::Path) {
    let before = std::fs::read(path).unwrap();
    let output = run(path, SECRET);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "OneLibrary reader failed");
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
    mutate_fixture(&orphan, "PRAGMA foreign_keys=OFF; INSERT INTO playlist_tracks VALUES('missing-playlist',0,'src:1')");
    assert_rejected_unchanged(&orphan);
    let generated = temp_db();
    encrypted_fixture(&generated);
    mutate_fixture(&generated, "ALTER TABLE tracks ADD COLUMN unexpected TEXT GENERATED ALWAYS AS (title) VIRTUAL");
    assert_rejected_unchanged(&generated);
    let _ = std::fs::remove_file(orphan);
    let _ = std::fs::remove_file(generated);
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
