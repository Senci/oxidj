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
    if query_only != 1 || trusted_schema != 0 {
        return Err(());
    }
    if user_version == 0 {
        return read_native_preview(&conn);
    }
    if user_version != 1 {
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

    // Validate all membership owners/targets before iterating playlists; otherwise
    // orphan rows disappear from the per-playlist query and look like success.
    let mut membership_check = conn
        .prepare("SELECT playlist_id,track_id FROM playlist_tracks ORDER BY playlist_id,position")
        .map_err(|_| ())?;
    let membership_rows = membership_check
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| ())?;
    let playlist_ids: std::collections::HashSet<String> = conn
        .prepare("SELECT id FROM playlists")
        .map_err(|_| ())?
        .query_map([], |row| row.get(0))
        .map_err(|_| ())?
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;
    let track_ids: std::collections::HashSet<String> = tracks
        .iter()
        .map(|track| track.id.as_str().to_owned())
        .collect();
    for row in membership_rows {
        let (playlist_id, track_id) = row.map_err(|_| ())?;
        if !playlist_ids.contains(&playlist_id) || !track_ids.contains(&track_id) {
            return Err(());
        }
    }
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

fn read_native_preview(conn: &rusqlite::Connection) -> Result<Preview, ()> {
    let application_id: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|_| ())?;
    if application_id != 0 || !matches_native_schema(conn)? {
        return Err(());
    }
    let mut tracks = Vec::new();
    let mut track_ids = std::collections::HashSet::new();
    let mut stmt = conn.prepare("SELECT c.content_id,c.title,c.artist_id_artist,a.artist_id,a.name,c.album_id,b.album_id,b.name FROM content c LEFT JOIN artist a ON a.artist_id=c.artist_id_artist LEFT JOIN album b ON b.album_id=c.album_id ORDER BY c.content_id").map_err(|_| ())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<i64>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|_| ())?;
    for row in rows {
        let (id, title, artist_id, joined_artist_id, artist, album_id, joined_album_id, album) =
            row.map_err(|_| ())?;
        if artist_id != joined_artist_id || album_id != joined_album_id {
            return Err(());
        }
        let source_id = SourceId::new(format!("content:{id}")).map_err(|_| ())?;
        if !track_ids.insert(source_id.as_str().to_owned()) {
            return Err(());
        }
        tracks.push(Track {
            id: source_id,
            title,
            artist,
            album,
        });
    }
    drop(stmt);

    let mut memberships: std::collections::HashMap<i64, Vec<(i64, i64)>> =
        std::collections::HashMap::new();
    let mut membership_stmt = conn.prepare("SELECT playlist_id,content_id,sequenceNo FROM playlist_content ORDER BY playlist_id,sequenceNo").map_err(|_| ())?;
    let rows = membership_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| ())?;
    let mut last: Option<(i64, i64)> = None;
    for row in rows {
        let (playlist_id, content_id, sequence) = row.map_err(|_| ())?;
        if sequence < 1
            || !track_ids.contains(&format!("content:{content_id}"))
            || last == Some((playlist_id, sequence))
        {
            return Err(());
        }
        last = Some((playlist_id, sequence));
        memberships
            .entry(playlist_id)
            .or_default()
            .push((sequence, content_id));
    }
    drop(membership_stmt);

    let mut playlists = Vec::new();
    let mut sibling_order: std::collections::HashMap<Option<i64>, u32> =
        std::collections::HashMap::new();
    let mut sibling_sequence: std::collections::HashSet<(Option<i64>, i64)> =
        std::collections::HashSet::new();
    let mut stmt = conn.prepare("SELECT playlist_id,sequenceNo,name,playlist_id_parent FROM playlist ORDER BY playlist_id_parent,sequenceNo,playlist_id").map_err(|_| ())?;
    let mut rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        })
        .map_err(|_| ())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    drop(stmt);
    let playlist_ids: std::collections::HashSet<i64> =
        rows.iter().map(|(id, _, _, _)| *id).collect();
    let mut missing_parent_counts: std::collections::HashMap<i64, usize> =
        std::collections::HashMap::new();
    for (_, _, _, parent) in &rows {
        if let Some(parent) = parent.filter(|parent| !playlist_ids.contains(parent)) {
            *missing_parent_counts.entry(parent).or_default() += 1;
        }
    }
    let repeated_root_sentinel = match missing_parent_counts
        .iter()
        .filter(|(_, count)| **count >= 2)
        .map(|(parent, _)| *parent)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => None,
        [sentinel] if missing_parent_counts.len() == 1 => Some(*sentinel),
        _ => return Err(()),
    };
    if missing_parent_counts
        .keys()
        .any(|parent| Some(*parent) != repeated_root_sentinel)
    {
        return Err(());
    }
    rows.sort_by_key(|(id, sequence, _, raw_parent)| {
        (
            raw_parent.filter(|value| Some(*value) != repeated_root_sentinel),
            *sequence,
            *id,
        )
    });
    for (raw_id, sequence, name, raw_parent) in rows {
        let parent = raw_parent.filter(|value| Some(*value) != repeated_root_sentinel);
        if sequence < 0 || !sibling_sequence.insert((parent, sequence)) {
            return Err(());
        }
        let order = sibling_order.entry(parent).or_default();
        let mut members = memberships.remove(&raw_id).unwrap_or_default();
        members.sort_by_key(|(position, _)| *position);
        let track_ids = members
            .into_iter()
            .map(|(_, id)| SourceId::new(format!("content:{id}")).map_err(|_| ()))
            .collect::<Result<Vec<_>, _>>()?;
        playlists.push(Playlist {
            id: SourceId::new(format!("playlist:{raw_id}")).map_err(|_| ())?,
            parent_id: parent
                .map(|id| SourceId::new(format!("playlist:{id}")))
                .transpose()
                .map_err(|_| ())?,
            sibling_order: *order,
            name,
            track_ids,
        });
        *order = order.checked_add(1).ok_or(())?;
    }
    if !memberships.is_empty() {
        return Err(());
    }
    let preview = Preview {
        protocol_version: oxidj_onelibrary::PROTOCOL_VERSION,
        schema_profile: oxidj_onelibrary::SCHEMA_PROFILE_NATIVE,
        tracks,
        playlists,
    };
    oxidj_onelibrary::validate(preview).map_err(|_| ())
}

fn matches_native_schema(conn: &rusqlite::Connection) -> Result<bool, ()> {
    let reference = rusqlite::Connection::open_in_memory().map_err(|_| ())?;
    reference
        .execute_batch(include_str!("native-schema.sql"))
        .map_err(|_| ())?;
    Ok(schema_signature(conn)? == schema_signature(&reference)?)
}

fn schema_signature(conn: &rusqlite::Connection) -> Result<Vec<String>, ()> {
    let mut signature = Vec::new();
    let mut stmt = conn.prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").map_err(|_| ())?;
    let objects = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|_| ())?;
    let objects = objects.collect::<Result<Vec<_>, _>>().map_err(|_| ())?;
    drop(stmt);
    for (kind, name, table, sql) in objects {
        let sql = sql
            .ok_or(())?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        signature.push(format!("{kind}:{name}:{table}:{sql}"));
        if kind == "table" {
            let mut cols = conn.prepare(r#"SELECT name,type,"notnull",dflt_value,pk,hidden FROM pragma_table_xinfo(?1) ORDER BY cid"#).map_err(|_| ())?;
            let rows = cols
                .query_map([&name], |r| {
                    Ok(format!(
                        "{}|{}|{}|{:?}|{}|{}",
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?
                    ))
                })
                .map_err(|_| ())?;
            for row in rows {
                signature.push(format!("column:{name}:{}", row.map_err(|_| ())?));
            }
        } else if kind == "index" {
            let mut cols = conn
                .prepare("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
                .map_err(|_| ())?;
            let rows = cols
                .query_map([&name], |r| r.get::<_, Option<String>>(0))
                .map_err(|_| ())?;
            for row in rows {
                signature.push(format!(
                    "index-column:{name}:{}",
                    row.map_err(|_| ())?.ok_or(())?
                ));
            }
        }
    }
    Ok(signature)
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
        .prepare(&format!("PRAGMA table_xinfo({name})"))
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
