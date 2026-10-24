//! Local catalog handlers. SQL and SQLite stay inside this crate.

use rusqlite::{Connection, OptionalExtension};
use rusqlite_migration::{M, Migrations};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
const MIGRATION_LIST: &[M<'static>] = &[M::up(
    "CREATE TABLE tracks (\
            id INTEGER PRIMARY KEY CHECK (id > 0),\
            path TEXT NOT NULL,\
            sha256 BLOB NOT NULL CHECK (length(sha256) = 32),\
            byte_length INTEGER NOT NULL CHECK (byte_length >= 0)\
        );\
        CREATE TABLE operations (\
            operation_id TEXT PRIMARY KEY,\
            track_id INTEGER NOT NULL UNIQUE REFERENCES tracks(id),\
            path TEXT NOT NULL,\
            sha256 BLOB NOT NULL CHECK (length(sha256) = 32),\
            byte_length INTEGER NOT NULL CHECK (byte_length >= 0)\
        );",
)];
const MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATION_LIST);

/// A storage or schema error from a catalog operation.
#[derive(Debug)]
pub enum CatalogError {
    Sqlite(rusqlite::Error),
    Migration(rusqlite_migration::Error),
    UnsupportedSchema(i64),
    OperationConflict(String),
    TrackLimit,
    InvalidRecord(&'static str),
    DestinationExists,
    Cleanup(std::io::Error),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "catalog database error: {error}"),
            Self::Migration(error) => write!(f, "catalog migration error: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(
                    f,
                    "catalog schema version {version} is newer than supported"
                )
            }
            Self::OperationConflict(operation_id) => {
                write!(
                    f,
                    "operation ID {operation_id:?} was used with a different payload"
                )
            }
            Self::TrackLimit => write!(f, "catalog track limit of 10000 reached"),
            Self::InvalidRecord(reason) => write!(f, "invalid track reference: {reason}"),
            Self::DestinationExists => write!(f, "catalog destination already exists"),
            Self::Cleanup(error) => write!(f, "catalog temporary-file cleanup failed: {error}"),
        }
    }
}

impl Error for CatalogError {}

impl From<rusqlite::Error> for CatalogError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<rusqlite_migration::Error> for CatalogError {
    fn from(value: rusqlite_migration::Error) -> Self {
        Self::Migration(value)
    }
}

/// An open, migrated local catalog.
pub struct Catalog {
    connection: Connection,
}

impl Catalog {
    /// Create a backup from an existing source without creating or migrating it.
    pub fn backup_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<(), CatalogError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        if source == destination {
            return Err(CatalogError::InvalidRecord(
                "source and destination must differ",
            ));
        }
        if destination.exists() {
            return Err(CatalogError::DestinationExists);
        }
        validate_database(source)?;
        let connection =
            Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let temporary = temporary_path(destination);
        let result = (|| {
            let mut target = Connection::open(&temporary)?;
            {
                let backup = rusqlite::backup::Backup::new(&connection, &mut target)?;
                backup.run_to_completion(16, std::time::Duration::from_millis(10), None)?;
            }
            drop(target);
            validate_database(&temporary)?;
            publish_new_file(&temporary, destination)
                .map_err(|_| CatalogError::Sqlite(rusqlite::Error::InvalidQuery))?;
            Ok(())
        })();
        if let Err(error) = result {
            return Err(cleanup_failed_operation(&temporary, error));
        }
        result
    }

    /// Create a verified SQLite online backup at a new path.
    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<(), CatalogError> {
        let destination = destination.as_ref();
        if destination.exists() {
            return Err(CatalogError::DestinationExists);
        }
        let temporary = temporary_path(destination);
        let result = (|| {
            let mut target = Connection::open(&temporary)?;
            {
                let backup = rusqlite::backup::Backup::new(&self.connection, &mut target)?;
                backup.run_to_completion(16, std::time::Duration::from_millis(10), None)?;
            }
            drop(target);
            validate_database(&temporary)?;
            publish_new_file(&temporary, destination)
                .map_err(|_| CatalogError::Sqlite(rusqlite::Error::InvalidQuery))?;
            Ok(())
        })();
        if let Err(error) = result {
            return Err(cleanup_failed_operation(&temporary, error));
        }
        result
    }

    /// Restore a verified backup into a new catalog path.
    pub fn restore_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<(), CatalogError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        if source == destination {
            return Err(CatalogError::InvalidRecord(
                "source and destination must differ",
            ));
        }
        if destination.exists() {
            return Err(CatalogError::DestinationExists);
        }
        validate_database(source)?;
        let temporary = temporary_path(destination);
        let result = (|| {
            let source_connection =
                Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let mut target = Connection::open(&temporary)?;
            {
                let backup = rusqlite::backup::Backup::new(&source_connection, &mut target)?;
                backup.run_to_completion(16, std::time::Duration::from_millis(10), None)?;
            }
            drop(target);
            drop(source_connection);
            validate_database(&temporary)?;
            {
                let catalog = Catalog::open(&temporary)?;
                catalog.list_tracks()?;
            }
            remove_database_sidecars(&temporary).map_err(CatalogError::Cleanup)?;
            publish_new_file(&temporary, destination)
                .map_err(|_| CatalogError::Sqlite(rusqlite::Error::InvalidQuery))?;
            Ok(())
        })();
        if let Err(error) = result {
            return Err(cleanup_failed_operation(&temporary, error));
        }
        result
    }

    /// Open or create a catalog, applying known migrations without resetting data.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let journal_mode: String =
            connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(CatalogError::Sqlite(rusqlite::Error::InvalidQuery));
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        let synchronous: i64 =
            connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
        if synchronous != 2 {
            return Err(CatalogError::Sqlite(rusqlite::Error::InvalidQuery));
        }

        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(CatalogError::UnsupportedSchema(version));
        }
        MIGRATIONS.to_latest(&mut connection)?;
        Ok(Self { connection })
    }

    /// List reference-only tracks in stable ID order.
    pub fn list_tracks(&self) -> Result<Vec<TrackReference>, CatalogError> {
        let mut statement = self
            .connection
            .prepare("SELECT id, path, sha256, byte_length FROM tracks ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let digest: Vec<u8> = row.get(2)?;
            let byte_length: i64 = row.get(3)?;
            Ok((id, row.get::<_, String>(1)?, digest, byte_length))
        })?;

        let mut tracks = Vec::new();
        for row in rows {
            let (id, path, digest, byte_length) = row?;
            let id = u64::try_from(id).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let sha256: [u8; 32] = digest
                .try_into()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            tracks.push(TrackReference {
                id: oxidj_core::TrackId::new(id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                path,
                sha256,
                byte_length: u64::try_from(byte_length)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
            });
        }
        Ok(tracks)
    }

    /// Insert a reference with transactional operation-ID replay protection.
    pub fn insert_track(
        &mut self,
        operation_id: &str,
        path: &str,
        sha256: [u8; 32],
        byte_length: u64,
    ) -> Result<TrackReference, CatalogError> {
        if operation_id.is_empty() {
            return Err(CatalogError::InvalidRecord(
                "operation ID must not be empty",
            ));
        }
        if path.is_empty() {
            return Err(CatalogError::InvalidRecord("path must not be empty"));
        }
        let byte_length = i64::try_from(byte_length)
            .map_err(|_| CatalogError::InvalidRecord("byte length exceeds SQLite range"))?;

        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let prior = transaction
            .query_row(
                "SELECT track_id, path, sha256, byte_length FROM operations WHERE operation_id = ?1",
                [operation_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?;
        if let Some((id, prior_path, prior_digest, prior_length)) = prior {
            if prior_path != path
                || prior_digest.as_slice() != sha256
                || prior_length != byte_length
            {
                return Err(CatalogError::OperationConflict(operation_id.to_string()));
            }
            let record = TrackReference {
                id: oxidj_core::TrackId::new(
                    u64::try_from(id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                )
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                path: prior_path,
                sha256,
                byte_length: u64::try_from(prior_length)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
            };
            transaction.commit()?;
            return Ok(record);
        }

        let count: i64 =
            transaction.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))?;
        if count >= 10_000 {
            return Err(CatalogError::TrackLimit);
        }
        let prior_id: i64 =
            transaction.query_row("SELECT COALESCE(MAX(id), 0) FROM tracks", [], |row| {
                row.get(0)
            })?;
        let id = prior_id
            .checked_add(1)
            .filter(|id| *id > 0)
            .ok_or(CatalogError::InvalidRecord("track ID space exhausted"))?;
        let track_id = oxidj_core::TrackId::new(
            u64::try_from(id)
                .map_err(|_| CatalogError::InvalidRecord("track ID space exhausted"))?,
        )
        .map_err(|_| CatalogError::InvalidRecord("track ID space exhausted"))?;
        let record_byte_length = u64::try_from(byte_length)
            .map_err(|_| CatalogError::InvalidRecord("byte length is negative"))?;
        transaction.execute(
            "INSERT INTO tracks (id, path, sha256, byte_length) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, path, sha256.as_slice(), byte_length],
        )?;
        transaction.execute(
            "INSERT INTO operations (operation_id, track_id, path, sha256, byte_length) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![operation_id, id, path, sha256.as_slice(), byte_length],
        )?;
        transaction.commit()?;

        Ok(TrackReference {
            id: track_id,
            path: path.to_string(),
            sha256,
            byte_length: record_byte_length,
        })
    }

    /// Read one track by its stable local identity.
    pub fn read_track(
        &self,
        id: oxidj_core::TrackId,
    ) -> Result<Option<TrackReference>, CatalogError> {
        let row = self
            .connection
            .query_row(
                "SELECT path, sha256, byte_length FROM tracks WHERE id = ?1",
                [i64::try_from(id.as_u64())
                    .map_err(|_| CatalogError::InvalidRecord("track ID exceeds SQLite range"))?],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(
            |(path, digest, byte_length)| -> Result<TrackReference, rusqlite::Error> {
                let sha256: [u8; 32] = digest
                    .try_into()
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(TrackReference {
                    id,
                    path,
                    sha256,
                    byte_length: u64::try_from(byte_length)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                })
            },
        )
        .transpose()
        .map_err(CatalogError::from)
    }
}

fn validate_database(path: &Path) -> Result<(), CatalogError> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(CatalogError::UnsupportedSchema(version));
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(CatalogError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if version != SCHEMA_VERSION {
        return Err(CatalogError::InvalidRecord(
            "backup schema is not supported",
        ));
    }
    validate_table_columns(
        &connection,
        "tracks",
        &[
            ("id", "INTEGER"),
            ("path", "TEXT"),
            ("sha256", "BLOB"),
            ("byte_length", "INTEGER"),
        ],
    )?;
    validate_table_columns(
        &connection,
        "operations",
        &[
            ("operation_id", "TEXT"),
            ("track_id", "INTEGER"),
            ("path", "TEXT"),
            ("sha256", "BLOB"),
            ("byte_length", "INTEGER"),
        ],
    )?;
    connection.prepare(
        "SELECT operation_id, track_id, path, sha256, byte_length FROM operations LIMIT 0",
    )?;
    validate_catalog_write_contract(&connection)?;
    Ok(())
}

fn validate_catalog_write_contract(source: &Connection) -> Result<(), CatalogError> {
    let mut probe = Connection::open_in_memory()?;
    {
        let backup = rusqlite::backup::Backup::new(source, &mut probe)?;
        backup.run_to_completion(16, std::time::Duration::from_millis(10), None)?;
    }

    let transaction = probe.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let track_id: i64 =
        transaction.query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM tracks", [], |row| {
            row.get(0)
        })?;
    let mut suffix = 0_u64;
    let operation_id = loop {
        let candidate = format!("oxidj-schema-probe-{suffix}");
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE operation_id = ?1)",
            [candidate.as_str()],
            |row| row.get(0),
        )?;
        if !exists {
            break candidate;
        }
        suffix = suffix.checked_add(1).ok_or(CatalogError::InvalidRecord(
            "schema probe ID space exhausted",
        ))?;
    };
    transaction.execute(
        "INSERT INTO tracks (id, path, sha256, byte_length) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            track_id,
            "/oxidj-schema-probe.wav",
            [0_u8; 32].as_slice(),
            0
        ],
    )?;
    transaction.execute(
        "INSERT INTO operations (operation_id, track_id, path, sha256, byte_length) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            operation_id,
            track_id,
            "/oxidj-schema-probe.wav",
            [0_u8; 32].as_slice(),
            0
        ],
    )?;
    transaction.rollback()?;
    Ok(())
}

fn validate_table_columns(
    connection: &Connection,
    table: &str,
    expected: &[(&str, &str)],
) -> Result<(), CatalogError> {
    let mut statement = connection.prepare("SELECT name, type FROM pragma_table_info(?1)")?;
    let actual = statement
        .query_map([table], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if actual.len() != expected.len()
        || expected.iter().any(|(name, kind)| {
            !actual.iter().any(|(actual_name, actual_kind)| {
                actual_name == name && actual_kind.eq_ignore_ascii_case(kind)
            })
        })
    {
        return Err(CatalogError::InvalidRecord(
            "backup catalog schema is incomplete",
        ));
    }
    Ok(())
}

fn temporary_path(destination: &Path) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    destination.with_file_name(format!(".oxidj-{}-{nonce}.tmp", std::process::id()))
}

fn publish_new_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    wait_at_publication_test_barrier(temporary);
    // Same-directory hard-link creation is atomic and never replaces a path
    // created concurrently after the initial existence check.
    std::fs::hard_link(temporary, destination)?;
    std::fs::remove_file(temporary)
}

#[cfg(test)]
static PUBLICATION_TEST_BARRIERS: std::sync::Mutex<
    Option<(
        std::sync::Arc<std::sync::Barrier>,
        std::sync::Arc<std::sync::Barrier>,
    )>,
> = std::sync::Mutex::new(None);

#[cfg(test)]
static PUBLICATION_TEST_TEMP: std::sync::Mutex<Option<std::path::PathBuf>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn wait_at_publication_test_barrier(temporary: &Path) {
    *PUBLICATION_TEST_TEMP.lock().unwrap() = Some(temporary.to_path_buf());
    let barriers = PUBLICATION_TEST_BARRIERS.lock().unwrap().clone();
    if let Some((arrived, release)) = barriers {
        arrived.wait();
        release.wait();
    }
}

fn remove_database_sidecars(path: &Path) -> std::io::Result<()> {
    let mut first_error = None;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar_name = path.as_os_str().to_os_string();
        sidecar_name.push(suffix);
        let sidecar = std::path::PathBuf::from(sidecar_name);
        if let Err(error) = std::fs::remove_file(sidecar)
            && error.kind() != std::io::ErrorKind::NotFound
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn cleanup_failed_operation(path: &Path, original: CatalogError) -> CatalogError {
    match remove_database_files(path) {
        Ok(()) => original,
        Err(cleanup) => CatalogError::Cleanup(cleanup),
    }
}

fn remove_database_files(path: &Path) -> std::io::Result<()> {
    let file_result = match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    };
    let sidecar_result = remove_database_sidecars(path);
    file_result.and(sidecar_result)
}

/// Minimum accepted local catalog record. Digest is the raw 32-byte SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackReference {
    pub id: oxidj_core::TrackId,
    pub path: String,
    pub sha256: [u8; 32],
    pub byte_length: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database_path() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "oxidj-library-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn remove_database(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn catalog_open_failed_migration_rolls_back_and_retries() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE operations (marker TEXT NOT NULL);\
                 INSERT INTO operations (marker) VALUES ('sentinel');",
            )
            .unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let schema = connection
            .prepare(
                "SELECT type, name, tbl_name, sql FROM sqlite_schema \
                 ORDER BY type, name, tbl_name, sql",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let sentinel: String = connection
            .query_row("SELECT marker FROM operations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        assert_eq!(sentinel, "sentinel");
        drop(connection);

        assert!(matches!(
            Catalog::open(&path),
            Err(CatalogError::Migration(_))
        ));

        let connection = Connection::open(&path).unwrap();
        let actual_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let actual_schema = connection
            .prepare(
                "SELECT type, name, tbl_name, sql FROM sqlite_schema \
                 ORDER BY type, name, tbl_name, sql",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let actual_sentinel: String = connection
            .query_row("SELECT marker FROM operations", [], |row| row.get(0))
            .unwrap();
        let tracks_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'tracks')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(actual_version, version);
        assert_eq!(actual_schema, schema);
        assert_eq!(actual_sentinel, sentinel);
        assert!(!tracks_exists);
        drop(connection);

        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("ALTER TABLE operations RENAME TO fixture_operations;")
            .unwrap();
        drop(connection);

        let catalog = Catalog::open(&path).unwrap();
        let version: i64 = catalog
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let track_count: i64 = catalog
            .connection
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();
        let sentinel: String = catalog
            .connection
            .query_row("SELECT marker FROM fixture_operations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(track_count, 0);
        assert_eq!(sentinel, "sentinel");
        drop(catalog);
        remove_database(&path);
    }

    #[test]
    fn operation_replay_returns_same_track_and_changed_payload_conflicts() {
        let path = database_path();
        let mut catalog = Catalog::open(&path).unwrap();
        let first = catalog
            .insert_track("op-1", "/synthetic/a.wav", [1; 32], 42)
            .unwrap();
        let replay = catalog
            .insert_track("op-1", "/synthetic/a.wav", [1; 32], 42)
            .unwrap();

        assert_eq!(replay, first);
        assert_eq!(catalog.list_tracks().unwrap(), vec![first.clone()]);
        assert!(matches!(
            catalog.insert_track("op-1", "/synthetic/other.wav", [1; 32], 42),
            Err(CatalogError::OperationConflict(id)) if id == "op-1"
        ));
        assert!(matches!(
            catalog.insert_track("op-1", "/synthetic/a.wav", [2; 32], 42),
            Err(CatalogError::OperationConflict(id)) if id == "op-1"
        ));
        assert!(matches!(
            catalog.insert_track("op-1", "/synthetic/a.wav", [1; 32], 43),
            Err(CatalogError::OperationConflict(id)) if id == "op-1"
        ));
        assert_eq!(catalog.list_tracks().unwrap(), vec![first]);
        drop(catalog);
        remove_database(&path);
    }

    #[test]
    fn failed_insert_rolls_back_and_retry_with_same_operation_id_succeeds() {
        let path = database_path();
        let mut catalog = Catalog::open(&path).unwrap();
        catalog
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_operation BEFORE INSERT ON operations BEGIN SELECT RAISE(ABORT, 'injected operation failure'); END;",
            )
            .unwrap();

        let error = catalog
            .insert_track("retry-op", "/synthetic/retry.wav", [7; 32], 17)
            .unwrap_err();
        assert!(matches!(error, CatalogError::Sqlite(_)));
        assert!(error.to_string().contains("catalog database error"));
        assert_eq!(catalog.list_tracks().unwrap(), []);

        catalog
            .connection
            .execute_batch("DROP TRIGGER reject_operation;")
            .unwrap();
        let inserted = catalog
            .insert_track("retry-op", "/synthetic/retry.wav", [7; 32], 17)
            .unwrap();
        let replay = catalog
            .insert_track("retry-op", "/synthetic/retry.wav", [7; 32], 17)
            .unwrap();
        assert_eq!(replay, inserted);
        assert_eq!(catalog.list_tracks().unwrap(), vec![inserted]);
        drop(catalog);
        remove_database(&path);
    }

    #[test]
    fn opening_catalog_twice_keeps_schema_and_effective_durability_settings() {
        let path = database_path();
        let catalog = Catalog::open(&path).unwrap();
        let version: i64 = catalog
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let journal_mode: String = catalog
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let synchronous: i64 = catalog
            .connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(synchronous, 2);
        drop(catalog);

        let catalog = Catalog::open(&path).unwrap();
        assert_eq!(catalog.list_tracks().unwrap(), []);
        drop(catalog);
        remove_database(&path);
    }

    #[test]
    fn full_catalog_rejects_additional_track_without_partial_record() {
        let path = database_path();
        let mut catalog = Catalog::open(&path).unwrap();
        catalog
            .connection
            .execute_batch(
                "WITH RECURSIVE ids(id) AS (\
                    SELECT 1 UNION ALL SELECT id + 1 FROM ids WHERE id < 10000\
                )\
                INSERT INTO tracks (id, path, sha256, byte_length)\
                SELECT id, '/synthetic/fixture.wav', zeroblob(32), 0 FROM ids;",
            )
            .unwrap();

        assert!(matches!(
            catalog.insert_track("op-over-cap", "/synthetic/extra.wav", [2; 32], 1),
            Err(CatalogError::TrackLimit)
        ));
        assert_eq!(catalog.list_tracks().unwrap().len(), 10_000);
        drop(catalog);
        remove_database(&path);
    }

    #[test]
    fn newer_schema_version_is_refused_without_rewriting_it() {
        let path = database_path();
        let connection = Connection::open(&path).unwrap();
        connection.pragma_update(None, "user_version", 42).unwrap();
        drop(connection);

        assert!(matches!(
            Catalog::open(&path),
            Err(CatalogError::UnsupportedSchema(42))
        ));
        let connection = Connection::open(&path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 42);
        drop(connection);
        remove_database(&path);
    }

    #[test]
    fn restore_rejects_malformed_operations_schema_without_publishing() {
        let source = database_path();
        let destination = database_path();
        let connection = Connection::open(&source).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE tracks (id INTEGER PRIMARY KEY, path TEXT NOT NULL, sha256 BLOB NOT NULL, byte_length INTEGER NOT NULL);\
                 CREATE TABLE operations (unrelated TEXT);\
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        drop(connection);
        let source_before = std::fs::read(&source).unwrap();

        assert!(Catalog::restore_from(&source, &destination).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), source_before);
        assert!(!destination.exists());
        remove_database(&source);
        remove_database(&destination);
    }

    fn assert_restore_rejects_unusable_operations_schema(schema_sql: &str) {
        let source = database_path();
        let destination = database_path();
        let connection = Connection::open(&source).unwrap();
        connection.execute_batch(schema_sql).unwrap();
        drop(connection);
        let source_before = std::fs::read(&source).unwrap();

        assert!(Catalog::restore_from(&source, &destination).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), source_before);
        assert!(!destination.exists());
        remove_database(&source);
        remove_database(&destination);
    }

    #[test]
    fn restore_rejects_operations_view_without_publishing() {
        assert_restore_rejects_unusable_operations_schema(
            "CREATE TABLE tracks (id INTEGER PRIMARY KEY CHECK(id>0),path TEXT NOT NULL,sha256 BLOB NOT NULL CHECK(length(sha256)=32),byte_length INTEGER NOT NULL CHECK(byte_length>=0));\
             CREATE TABLE backing_operations (operation_id TEXT PRIMARY KEY,track_id INTEGER NOT NULL UNIQUE REFERENCES tracks(id),path TEXT NOT NULL,sha256 BLOB NOT NULL CHECK(length(sha256)=32),byte_length INTEGER NOT NULL CHECK(byte_length>=0));\
             CREATE VIEW operations AS SELECT * FROM backing_operations;\
             PRAGMA user_version=1;",
        );
    }

    #[test]
    fn restore_rejects_incompatible_operations_constraint_without_publishing() {
        assert_restore_rejects_unusable_operations_schema(
            "CREATE TABLE tracks (id INTEGER PRIMARY KEY CHECK(id>0),path TEXT NOT NULL,sha256 BLOB NOT NULL CHECK(length(sha256)=32),byte_length INTEGER NOT NULL CHECK(byte_length>=0));\
             CREATE TABLE operations (operation_id TEXT PRIMARY KEY,track_id INTEGER NOT NULL UNIQUE REFERENCES tracks(id) CHECK(track_id<0),path TEXT NOT NULL,sha256 BLOB NOT NULL CHECK(length(sha256)=32),byte_length INTEGER NOT NULL CHECK(byte_length>=0));\
             PRAGMA user_version=1;",
        );
    }

    #[test]
    fn restore_accepts_valid_catalog_with_schema_probe_operation_id() {
        let source = database_path();
        let destination = database_path();
        let mut catalog = Catalog::open(&source).unwrap();
        catalog
            .insert_track(
                "oxidj-schema-probe-2",
                "/synthetic/probe-collision.wav",
                [3; 32],
                7,
            )
            .unwrap();
        let expected = catalog.list_tracks().unwrap();
        drop(catalog);

        assert!(Catalog::restore_from(&source, &destination).is_ok());
        let restored = Catalog::open(&destination).unwrap();
        assert_eq!(restored.list_tracks().unwrap(), expected);
        drop(restored);
        remove_database(&source);
        remove_database(&destination);
    }

    #[test]
    fn cleanup_removes_only_sqlite_appended_sidecars() {
        let temporary = database_path().with_extension("tmp");
        let unrelated = temporary.with_file_name("unrelated-preserve-me");
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = if suffix.is_empty() {
                temporary.clone()
            } else {
                std::path::PathBuf::from(format!("{}{suffix}", temporary.display()))
            };
            std::fs::write(path, b"owned-temp").unwrap();
        }
        std::fs::write(&unrelated, b"not-owned").unwrap();

        remove_database_files(&temporary).unwrap();

        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = if suffix.is_empty() {
                temporary.clone()
            } else {
                std::path::PathBuf::from(format!("{}{suffix}", temporary.display()))
            };
            assert!(
                !path.exists(),
                "owned temp artifact remains: {}",
                path.display()
            );
        }
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"not-owned");
        let _ = std::fs::remove_file(unrelated);
    }

    #[test]
    fn cleanup_failure_is_returned() {
        let path = database_path();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            cleanup_failed_operation(&path, CatalogError::DestinationExists),
            CatalogError::Cleanup(_)
        ));
        std::fs::remove_dir(path).unwrap();
    }

    static RACE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn race_publication(backup: bool) {
        let _guard = RACE_TEST_LOCK.lock().unwrap();
        let source = database_path();
        let backup_path = database_path();
        let destination = database_path();
        let unrelated = destination.with_extension("unrelated");
        let mut catalog = Catalog::open(&source).unwrap();
        catalog
            .insert_track("race-op", "/synthetic/race.wav", [7; 32], 7)
            .unwrap();
        drop(catalog);
        if !backup {
            Catalog::open(&source)
                .unwrap()
                .backup_to(&backup_path)
                .unwrap();
        }
        let arrived = std::sync::Arc::new(std::sync::Barrier::new(2));
        let release = std::sync::Arc::new(std::sync::Barrier::new(2));
        *PUBLICATION_TEST_BARRIERS.lock().unwrap() = Some((arrived.clone(), release.clone()));
        let competitor_destination = destination.clone();
        let competitor_unrelated = unrelated.clone();
        let competitor = std::thread::spawn(move || {
            arrived.wait();
            let temporary = PUBLICATION_TEST_TEMP.lock().unwrap().clone().unwrap();
            for suffix in ["-wal", "-shm", "-journal"] {
                let mut sidecar = temporary.as_os_str().to_os_string();
                sidecar.push(suffix);
                std::fs::write(sidecar, b"sqlite-appended-temp").unwrap();
            }
            std::fs::write(&competitor_destination, b"competing-sentinel").unwrap();
            std::fs::write(&competitor_unrelated, b"unrelated-sentinel").unwrap();
            release.wait();
        });
        *PUBLICATION_TEST_TEMP.lock().unwrap() = None;
        let result = if backup {
            Catalog::open(&source).unwrap().backup_to(&destination)
        } else {
            Catalog::restore_from(&backup_path, &destination)
        };
        competitor.join().unwrap();
        *PUBLICATION_TEST_BARRIERS.lock().unwrap() = None;
        let temporary = PUBLICATION_TEST_TEMP.lock().unwrap().take().unwrap();
        let mut temporary_clean = !temporary.exists();
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = temporary.as_os_str().to_os_string();
            sidecar.push(suffix);
            temporary_clean &= !std::path::PathBuf::from(sidecar).exists();
        }
        let returned_explicit_error = matches!(
            &result,
            Err(CatalogError::Sqlite(rusqlite::Error::InvalidQuery))
        );
        let destination_preserved = returned_explicit_error
            && std::fs::read(&destination).is_ok_and(|bytes| bytes == b"competing-sentinel");
        let unrelated_preserved =
            std::fs::read(&unrelated).is_ok_and(|bytes| bytes == b"unrelated-sentinel");
        remove_database(&source);
        remove_database(&backup_path);
        remove_database(&destination);
        let _ = std::fs::remove_file(&unrelated);
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = temporary.as_os_str().to_os_string();
            sidecar.push(suffix);
            let _ = std::fs::remove_file(sidecar);
        }
        drop(_guard);
        assert!(
            destination_preserved,
            "publication replaced the concurrent destination"
        );
        assert!(
            returned_explicit_error,
            "operation returned no explicit error"
        );
        assert!(unrelated_preserved, "cleanup removed an unrelated file");
        assert!(temporary_clean, "owned temporary artifacts remain");
    }

    #[test]
    fn backup_publication_race_preserves_competing_destination() {
        race_publication(true);
    }

    #[test]
    fn restore_publication_race_preserves_competing_destination() {
        race_publication(false);
    }
}
