//! Local catalog handlers. SQL and SQLite stay inside this crate.

use rusqlite::{Connection, OptionalExtension};
use rusqlite_migration::{M, Migrations};
use std::error::Error;
use std::fmt;
use std::path::Path;

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
}
