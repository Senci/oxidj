use oxidj_onelibrary::{Playlist, Preview, SourceId, Track};
use rusqlite::OptionalExtension;
use std::{
    ffi::c_void,
    io::{self, IsTerminal, Read, Write},
    os::unix::ffi::OsStrExt,
    path::Path,
    process,
};
use zeroize::Zeroize;

const MAX_KEY_BYTES: usize = 4096;

struct Secret(Vec<u8>);
impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

fn main() {
    if run().is_err() {
        // No SQLite or input diagnostics are allowed across the process boundary.
        eprintln!("OneLibrary reader failed");
        process::exit(2);
    }
}

fn run() -> Result<(), ()> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--key-stdin")) {
        return Err(());
    }
    let source = args.next().ok_or(())?;
    if args.next().is_some() || io::stdin().is_terminal() {
        return Err(());
    }
    let source = Path::new(&source);
    if !source.is_file()
        || has_sidecar(source, "-wal")
        || has_sidecar(source, "-journal")
        || has_sidecar(source, "-shm")
    {
        return Err(());
    }

    let mut secret = Secret(Vec::with_capacity(MAX_KEY_BYTES + 1));
    io::stdin()
        .lock()
        .take((MAX_KEY_BYTES + 1) as u64)
        .read_to_end(&mut secret.0)
        .map_err(|_| ())?;
    if secret.0.is_empty() || secret.0.len() > MAX_KEY_BYTES {
        return Err(());
    }
    let preview = read_preview(source, &secret.0)?;
    let payload = oxidj_onelibrary::encode_preview(preview).map_err(|_| ())?;
    // Write only after all input, schema, joins and ordering have validated.
    io::stdout().lock().write_all(&payload).map_err(|_| ())?;
    io::stdout().lock().write_all(b"\n").map_err(|_| ())?;
    Ok(())
}

fn has_sidecar(path: &Path, suffix: &str) -> bool {
    let mut bytes = path.as_os_str().as_bytes().to_vec();
    bytes.extend_from_slice(suffix.as_bytes());
    std::fs::metadata(std::ffi::OsStr::from_bytes(&bytes)).is_ok()
}

fn uri_path(path: &Path) -> Result<String, ()> {
    let canonical = path.canonicalize().map_err(|_| ())?;
    let mut out = String::from("file:");
    for byte in canonical.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'/' | b'.' | b'_' | b'-' | b'~') {
            out.push(*byte as char);
        } else {
            use std::fmt::Write as _;
            write!(out, "%{byte:02X}").map_err(|_| ())?;
        }
    }
    out.push_str("?mode=ro&immutable=1");
    Ok(out)
}

fn read_preview(path: &Path, key: &[u8]) -> Result<Preview, ()> {
    use rusqlite::{Connection, OpenFlags};
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let conn = Connection::open_with_flags(uri_path(path)?, flags).map_err(|_| ())?;
    // SAFETY: a live bounded key slice is passed directly to SQLCipher and never formatted as SQL.
    let rc = unsafe {
        rusqlite::ffi::sqlite3_key(
            conn.handle(),
            key.as_ptr().cast::<c_void>(),
            i32::try_from(key.len()).map_err(|_| ())?,
        )
    };
    if rc != rusqlite::ffi::SQLITE_OK
        || unsafe { rusqlite::ffi::sqlite3_db_readonly(conn.handle(), c"main".as_ptr()) } != 1
    {
        return Err(());
    }
    let cipher_version: String = conn
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(|_| ())?;
    if cipher_version.is_empty() {
        return Err(());
    }
    conn.pragma_update(None, "query_only", "ON")
        .map_err(|_| ())?;
    conn.pragma_update(None, "trusted_schema", "OFF")
        .map_err(|_| ())?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .map_err(|_| ())?;
    let query_only: i64 = conn
        .query_row("PRAGMA query_only", [], |row| row.get(0))
        .map_err(|_| ())?;
    let trusted_schema: i64 = conn
        .query_row("PRAGMA trusted_schema", [], |row| row.get(0))
        .map_err(|_| ())?;
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| ())?;
    if query_only != 1 || trusted_schema != 0 || user_version != 1 {
        return Err(());
    }
    let table_names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|_| ())?
        .query_map([], |row| row.get(0))
        .map_err(|_| ())?
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;
    if table_names != ["playlist_tracks", "playlists", "tracks"] {
        return Err(());
    }

    verify_table(
        &conn,
        "tracks",
        &[
            ("id", "TEXT", true),
            ("title", "TEXT", false),
            ("artist", "TEXT", false),
            ("album", "TEXT", false),
        ],
    )?;
    verify_table(
        &conn,
        "playlists",
        &[
            ("id", "TEXT", true),
            ("parent_id", "TEXT", false),
            ("sibling_order", "INTEGER", false),
            ("name", "TEXT", false),
        ],
    )?;
    verify_table(
        &conn,
        "playlist_tracks",
        &[
            ("playlist_id", "TEXT", false),
            ("position", "INTEGER", false),
            ("track_id", "TEXT", false),
        ],
    )?;

    let mut tracks = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id,title,artist,album FROM tracks ORDER BY id")
        .map_err(|_| ())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|_| ())?;
    for row in rows {
        let (id, title, artist, album) = row.map_err(|_| ())?;
        tracks.push(Track {
            id: SourceId::new(id).map_err(|_| ())?,
            title,
            artist,
            album,
        });
    }
    drop(stmt);

    let mut playlists = Vec::new();
    let mut membership_stmt = conn
        .prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id=?1 ORDER BY position")
        .map_err(|_| ())?;
    let mut stmt = conn.prepare("SELECT id,parent_id,sibling_order,name FROM playlists ORDER BY COALESCE(parent_id,''),sibling_order").map_err(|_| ())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| ())?;
    let mut previous_parent: Option<Option<String>> = None;
    let mut expected_sibling = 0i64;
    for row in rows {
        let (raw_id, raw_parent, order, name) = row.map_err(|_| ())?;
        if previous_parent.as_ref() != Some(&raw_parent) {
            expected_sibling = 0;
            previous_parent = Some(raw_parent.clone());
        }
        if order != expected_sibling {
            return Err(());
        }
        expected_sibling += 1;
        let id = SourceId::new(raw_id.clone()).map_err(|_| ())?;
        let parent_id = raw_parent.map(SourceId::new).transpose().map_err(|_| ())?;
        let mut members = Vec::new();
        let member_rows = membership_stmt
            .query_map([&raw_id], |row| row.get::<_, String>(0))
            .map_err(|_| ())?;
        for (position, member) in member_rows.enumerate() {
            let (actual_position,): (i64,) = conn.query_row("SELECT position FROM playlist_tracks WHERE playlist_id=?1 ORDER BY position LIMIT 1 OFFSET ?2", rusqlite::params![raw_id, i64::try_from(position).map_err(|_| ())?], |r| Ok((r.get(0)?,))).map_err(|_| ())?;
            if actual_position != position as i64 {
                return Err(());
            }
            members.push(SourceId::new(member.map_err(|_| ())?).map_err(|_| ())?);
        }
        playlists.push(Playlist {
            id,
            parent_id,
            sibling_order: u32::try_from(order).map_err(|_| ())?,
            name,
            track_ids: members,
        });
    }
    drop(stmt);
    let preview = Preview {
        protocol_version: oxidj_onelibrary::PROTOCOL_VERSION,
        schema_profile: oxidj_onelibrary::SCHEMA_PROFILE_V1,
        tracks,
        playlists,
    };
    oxidj_onelibrary::validate(preview).map_err(|_| ())
}

fn verify_table(
    conn: &rusqlite::Connection,
    name: &str,
    columns: &[(&str, &str, bool)],
) -> Result<(), ()> {
    let kind: Option<String> = conn
        .query_row(
            "SELECT type FROM sqlite_schema WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| ())?;
    if kind.as_deref() != Some("table") {
        return Err(());
    }
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({name})"))
        .map_err(|_| ())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(|_| ())?;
    let actual: Vec<_> = rows.map(|r| r.map_err(|_| ())).collect::<Result<_, _>>()?;
    if actual.len() != columns.len() {
        return Err(());
    }
    for (column, ty, primary) in columns {
        let found: Vec<_> = actual.iter().filter(|(n, _, _, _)| n == column).collect();
        if found.len() != 1 || found[0].1 != *ty || found[0].3 != i64::from(*primary) {
            return Err(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_wal_and_journal_sidecars() {
        let base = std::env::temp_dir().join(format!("oxidj-reader-{}", process::id()));
        let wal = std::ffi::OsStr::from_bytes(&[base.as_os_str().as_bytes(), b"-wal"].concat())
            .to_owned();
        let journal =
            std::ffi::OsStr::from_bytes(&[base.as_os_str().as_bytes(), b"-journal"].concat())
                .to_owned();
        std::fs::write(&wal, b"synthetic").unwrap();
        assert!(has_sidecar(&base, "-wal"));
        assert!(!has_sidecar(&base, "-journal"));
        std::fs::write(&journal, b"synthetic").unwrap();
        assert!(has_sidecar(&base, "-journal"));
        std::fs::remove_file(wal).unwrap();
        std::fs::remove_file(journal).unwrap();
    }
}
