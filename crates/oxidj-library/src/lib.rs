//! Local catalog handlers. SQL and SQLite stay inside this crate.

use rusqlite::{Connection, OptionalExtension};
use rusqlite_migration::{M, Migrations};
use sha2::Digest;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 2;
const MIGRATION_LIST: &[M<'static>] = &[
    M::up(
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
    ),
    M::up(
        "CREATE TABLE audio_facts (\
            track_id INTEGER PRIMARY KEY REFERENCES tracks(id),\
            duration_seconds REAL NOT NULL CHECK (duration_seconds >= 0),\
            sample_rate INTEGER NOT NULL CHECK (sample_rate > 0),\
            channels INTEGER NOT NULL CHECK (channels > 0),\
            bit_depth INTEGER NOT NULL CHECK (bit_depth > 0),\
            missing INTEGER NOT NULL DEFAULT 0 CHECK (missing IN (0, 1))\
        );\
        CREATE INDEX tracks_path_idx ON tracks(path);",
    ),
];
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

    /// Backwards-compatible WAV import entry point.
    pub fn import_wav_directory(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<ImportReport, CatalogError> {
        self.import_audio_directory(root)
    }

    /// Import supported WAV/FLAC/MP3 references from a directory without modifying source files.
    /// A failed traversal is incomplete and therefore never marks old rows missing.
    pub fn import_audio_directory(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<ImportReport, CatalogError> {
        let input_root = root.as_ref();
        let mut report = ImportReport {
            complete: true,
            outcomes: Vec::new(),
        };
        let mut component_path = if input_root.is_absolute() {
            PathBuf::new()
        } else {
            std::env::current_dir().unwrap_or_default()
        };
        let has_symlink_component = input_root.components().any(|component| {
            component_path.push(component.as_os_str());
            std::fs::symlink_metadata(&component_path)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
        });
        if has_symlink_component {
            report.complete = false;
            return Ok(report);
        }
        if !matches!(std::fs::metadata(input_root), Ok(metadata) if metadata.is_dir()) {
            report.complete = false;
            return Ok(report);
        }
        let root = std::fs::canonicalize(input_root).unwrap_or_else(|_| input_root.to_path_buf());
        let mut paths = Vec::new();
        let mut complete = true;
        collect_regular_files(&root, &mut paths, &mut complete);
        paths.sort();
        let mut report = ImportReport {
            complete,
            outcomes: Vec::new(),
        };
        let mut seen = std::collections::HashSet::new();
        for path in paths {
            let path_text = path.to_string_lossy().into_owned();
            seen.insert(path_text.clone());
            if !path.extension().is_some_and(|ext| {
                ["wav", "flac", "mp3"]
                    .iter()
                    .any(|supported| ext.eq_ignore_ascii_case(supported))
            }) {
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Unsupported,
                });
                continue;
            }
            let (digest, byte_length) = match file_sha256(&path) {
                Ok(result) => result,
                Err(_) => {
                    report.complete = false;
                    report.outcomes.push(FileOutcome {
                        path: path_text,
                        status: ImportStatus::Unreadable,
                    });
                    continue;
                }
            };
            let prior = self
                .connection
                .query_row(
                    "SELECT id, sha256, byte_length FROM tracks WHERE path = ?1 ORDER BY id LIMIT 1",
                    [&path_text],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, Vec<u8>>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )
                .optional()?;
            if !matches!(file_sha256(&path), Ok((current_digest, current_length)) if current_digest == digest && current_length == byte_length)
            {
                report.complete = false;
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Unreadable,
                });
                continue;
            }
            if let Some((id, prior_digest, prior_length)) = prior {
                if prior_digest.as_slice() != digest || prior_length != byte_length as i64 {
                    report.outcomes.push(FileOutcome {
                        path: path_text,
                        status: ImportStatus::Conflict,
                    });
                    continue;
                }
                self.connection
                    .execute("UPDATE audio_facts SET missing=0 WHERE track_id=?1", [id])?;
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Unchanged,
                });
                continue;
            }
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if extension == "flac" || extension == "mp3" {
                let facts = if extension == "flac" {
                    decode_flac_facts(&path)
                } else {
                    decode_mp3_facts(&path)
                };
                let facts = match facts {
                    Ok(facts) => facts,
                    Err(status) => {
                        report.outcomes.push(FileOutcome {
                            path: path_text,
                            status,
                        });
                        continue;
                    }
                };
                if !matches!(file_sha256(&path), Ok((current_digest, current_length)) if current_digest == digest && current_length == byte_length)
                {
                    report.complete = false;
                    report.outcomes.push(FileOutcome {
                        path: path_text,
                        status: ImportStatus::Unreadable,
                    });
                    continue;
                }
                let path_hash: [u8; 32] = sha2::Sha256::digest(path_text.as_bytes()).into();
                let op = format!(
                    "audio-import:{}:{}",
                    hex_digest(&path_hash),
                    hex_digest(&digest)
                );
                self.insert_import_track_with_facts(&op, &path_text, digest, byte_length, facts)?;
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Imported,
                });
                continue;
            }
            let Some((padding_offsets, supported)) = wav_container_padding_offsets(&path) else {
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Malformed,
                });
                continue;
            };
            if !supported {
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Unsupported,
                });
                continue;
            }
            let source = match PaddingStrippingReader::open(&path, padding_offsets) {
                Ok(source) => source,
                Err(_) => {
                    report.outcomes.push(FileOutcome {
                        path: path_text,
                        status: ImportStatus::Unreadable,
                    });
                    continue;
                }
            };
            let mut reader = match hound::WavReader::new(source) {
                Ok(reader) => reader,
                Err(_) => {
                    report.outcomes.push(FileOutcome {
                        path: path_text,
                        status: ImportStatus::Malformed,
                    });
                    continue;
                }
            };
            let spec = reader.spec();
            let expected_sample_count = u64::from(reader.len());
            let decoded = if spec.sample_format == hound::SampleFormat::Float {
                reader
                    .samples::<f32>()
                    .try_fold(0_u64, |count, sample| sample.map(|_| count + 1))
            } else {
                reader
                    .samples::<i32>()
                    .try_fold(0_u64, |count, sample| sample.map(|_| count + 1))
            };
            let Ok(sample_count) = decoded else {
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Malformed,
                });
                continue;
            };
            if sample_count != expected_sample_count
                || spec.channels == 0
                || spec.sample_rate == 0
                || sample_count % u64::from(spec.channels) != 0
            {
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Malformed,
                });
                continue;
            }
            if !matches!(file_sha256(&path), Ok((current_digest, current_length)) if current_digest == digest && current_length == byte_length)
            {
                report.complete = false;
                report.outcomes.push(FileOutcome {
                    path: path_text,
                    status: ImportStatus::Unreadable,
                });
                continue;
            }
            let duration =
                (sample_count / u64::from(spec.channels)) as f64 / f64::from(spec.sample_rate);
            let path_hash: [u8; 32] = sha2::Sha256::digest(path_text.as_bytes()).into();
            let op = format!(
                "wav-import:{}:{}",
                hex_digest(&path_hash),
                hex_digest(&digest)
            );
            self.insert_import_track_with_facts(
                &op,
                &path_text,
                digest,
                byte_length,
                AudioFacts {
                    duration,
                    sample_rate: spec.sample_rate,
                    channels: spec.channels,
                    bit_depth: spec.bits_per_sample,
                },
            )?;
            report.outcomes.push(FileOutcome {
                path: path_text,
                status: ImportStatus::Imported,
            });
        }
        if report.complete {
            let missing_ids = {
                let mut stmt = self
                    .connection
                    .prepare("SELECT track_id FROM audio_facts WHERE missing=0")?;
                let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            for id in missing_ids {
                let path: String = self.connection.query_row(
                    "SELECT path FROM tracks WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )?;
                if !Path::new(&path).starts_with(&root) {
                    continue;
                }
                if !seen.contains(&path) {
                    self.connection
                        .execute("UPDATE audio_facts SET missing=1 WHERE track_id=?1", [id])?;
                }
            }
        }
        Ok(report)
    }

    fn insert_import_track_with_facts(
        &mut self,
        operation_id: &str,
        path: &str,
        digest: [u8; 32],
        byte_length: u64,
        facts: AudioFacts,
    ) -> Result<(), CatalogError> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some((id, prior_digest, prior_length)) = tx
            .query_row(
                "SELECT id,sha256,byte_length FROM tracks WHERE path=?1 ORDER BY id LIMIT 1",
                [path],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
        {
            if prior_digest.as_slice() != digest
                || prior_length
                    != i64::try_from(byte_length).map_err(|_| {
                        CatalogError::InvalidRecord("byte length exceeds SQLite range")
                    })?
            {
                return Err(CatalogError::OperationConflict(operation_id.to_string()));
            }
            let facts: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM audio_facts WHERE track_id=?1)",
                [id],
                |r| r.get(0),
            )?;
            if !facts {
                return Err(CatalogError::InvalidRecord(
                    "existing imported track has no audio facts",
                ));
            }
            tx.commit()?;
            return Ok(());
        }
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        if count >= 10_000 {
            return Err(CatalogError::TrackLimit);
        }
        let id: i64 = tx.query_row("SELECT COALESCE(MAX(id),0)+1 FROM tracks", [], |r| r.get(0))?;
        let length = i64::try_from(byte_length)
            .map_err(|_| CatalogError::InvalidRecord("byte length exceeds SQLite range"))?;
        tx.execute(
            "INSERT INTO tracks(id,path,sha256,byte_length) VALUES(?1,?2,?3,?4)",
            rusqlite::params![id, path, digest.as_slice(), length],
        )?;
        tx.execute("INSERT INTO operations(operation_id,track_id,path,sha256,byte_length) VALUES(?1,?2,?3,?4,?5)", rusqlite::params![operation_id,id,path,digest.as_slice(),length])?;
        tx.execute("INSERT INTO audio_facts(track_id,duration_seconds,sample_rate,channels,bit_depth,missing) VALUES(?1,?2,?3,?4,?5,0)", rusqlite::params![id,facts.duration,facts.sample_rate,facts.channels,facts.bit_depth])?;
        tx.commit()?;
        Ok(())
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

    /// List tracks with decoded technical facts and current filesystem availability.
    /// Results are ordered by stable local ID; presentation sorting belongs to callers.
    pub fn list_track_details(&self) -> Result<Vec<TrackDetails>, CatalogError> {
        let mut statement = self.connection.prepare(
            "SELECT t.id,t.path,t.sha256,t.byte_length,f.duration_seconds,f.sample_rate,\
             f.channels,f.bit_depth,f.missing FROM tracks t \
             LEFT JOIN audio_facts f ON f.track_id=t.id ORDER BY t.id",
        )?;
        let rows = statement.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let digest: Vec<u8> = row.get(2)?;
            let byte_length: i64 = row.get(3)?;
            Ok((
                id,
                row.get::<_, String>(1)?,
                digest,
                byte_length,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<u32>>(5)?,
                row.get::<_, Option<u16>>(6)?,
                row.get::<_, Option<u16>>(7)?,
                row.get::<_, Option<bool>>(8)?,
            ))
        })?;
        let mut tracks = Vec::new();
        for row in rows {
            let (
                id,
                path,
                digest,
                byte_length,
                duration,
                sample_rate,
                channels,
                bit_depth,
                missing,
            ) = row?;
            let availability = if missing.unwrap_or(false) || !Path::new(&path).exists() {
                TrackAvailability::Missing
            } else {
                TrackAvailability::Available
            };
            tracks.push(TrackDetails {
                track: TrackReference {
                    id: oxidj_core::TrackId::new(
                        u64::try_from(id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    )
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    path,
                    sha256: digest
                        .try_into()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    byte_length: u64::try_from(byte_length)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                },
                duration_seconds: duration,
                sample_rate,
                channels,
                bit_depth,
                availability,
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

/// Per-file WAV import result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportStatus {
    Imported,
    Unchanged,
    Conflict,
    Unsupported,
    Unreadable,
    Malformed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOutcome {
    pub path: String,
    pub status: ImportStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportReport {
    pub complete: bool,
    pub outcomes: Vec<FileOutcome>,
}

const MAX_WAV_CHUNKS: usize = 4096;

#[derive(Clone, Copy)]
struct AudioFacts {
    duration: f64,
    sample_rate: u32,
    channels: u16,
    bit_depth: u16,
}

fn decode_flac_facts(path: &Path) -> Result<AudioFacts, ImportStatus> {
    let file = std::fs::File::open(path).map_err(|_| ImportStatus::Unreadable)?;
    let mut reader = claxon::FlacReader::new_ext(
        file,
        claxon::FlacReaderOptions {
            metadata_only: false,
            read_vorbis_comment: false,
        },
    )
    .map_err(|_| ImportStatus::Malformed)?;
    let info = reader.streaminfo();
    if info.sample_rate == 0 || info.channels == 0 || info.bits_per_sample == 0 {
        return Err(ImportStatus::Malformed);
    }
    let samples = reader
        .samples()
        .try_fold(0_u64, |count, sample| {
            sample.map(|_| count.saturating_add(1))
        })
        .map_err(|_| ImportStatus::Malformed)?;
    if samples == 0 || samples % u64::from(info.channels) != 0 {
        return Err(ImportStatus::Malformed);
    }
    if info
        .samples
        .is_some_and(|declared| declared != 0 && declared != samples / u64::from(info.channels))
    {
        return Err(ImportStatus::Malformed);
    }
    Ok(AudioFacts {
        duration: (samples / u64::from(info.channels)) as f64 / f64::from(info.sample_rate),
        sample_rate: info.sample_rate,
        channels: info.channels as u16,
        bit_depth: info.bits_per_sample as u16,
    })
}

fn mp3_frame_size(header: [u8; 4]) -> Result<usize, ImportStatus> {
    let bits = u32::from_be_bytes(header);
    if bits >> 21 != 0x7ff {
        return Err(ImportStatus::Malformed);
    }
    let version = (bits >> 19) & 3;
    let layer = (bits >> 17) & 3;
    if version == 1 || layer == 0 {
        return Err(ImportStatus::Malformed);
    }
    if layer != 1 {
        return Err(ImportStatus::Unsupported);
    }
    let bitrate_index = ((bits >> 12) & 15) as usize;
    let sample_rate_index = ((bits >> 10) & 3) as usize;
    if bitrate_index == 15 || sample_rate_index == 3 {
        return Err(ImportStatus::Malformed);
    }
    if bitrate_index == 0 {
        return Err(ImportStatus::Unsupported);
    }
    let bitrates = if version == 3 {
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
        ]
    } else {
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160]
    };
    let mut sample_rates = [44_100, 48_000, 32_000];
    if version == 2 {
        sample_rates.iter_mut().for_each(|rate| *rate /= 2);
    } else if version == 0 {
        sample_rates.iter_mut().for_each(|rate| *rate /= 4);
    }
    let bitrate = bitrates[bitrate_index] as usize * 1000;
    let sample_rate = sample_rates[sample_rate_index] as usize;
    let padding = ((bits >> 9) & 1) as usize;
    let coefficient = if version == 3 { 144 } else { 72 };
    Ok(coefficient * bitrate / sample_rate + padding)
}

fn validate_mp3_frame_layout(
    file: &mut std::fs::File,
    start: u64,
    end: u64,
) -> Result<(), ImportStatus> {
    use std::io::{Read, Seek, SeekFrom};
    let mut offset = start;
    let mut frames = 0_u64;
    while offset < end {
        if end - offset < 4 {
            return Err(ImportStatus::Malformed);
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| ImportStatus::Unreadable)?;
        let mut header = [0_u8; 4];
        file.read_exact(&mut header)
            .map_err(|_| ImportStatus::Malformed)?;
        let frame_size = mp3_frame_size(header)? as u64;
        offset = offset
            .checked_add(frame_size)
            .filter(|next| *next <= end)
            .ok_or(ImportStatus::Malformed)?;
        frames += 1;
    }
    if frames == 0 {
        return Err(ImportStatus::Malformed);
    }
    Ok(())
}

fn decode_mp3_facts(path: &Path) -> Result<AudioFacts, ImportStatus> {
    use std::io::{Read, Seek, SeekFrom};
    const INPUT: usize = 32 * 1024;
    const MAX_ID3_TAG: u64 = 16 * 1024 * 1024;
    let mut file = std::fs::File::open(path).map_err(|_| ImportStatus::Unreadable)?;
    let file_len = file.metadata().map_err(|_| ImportStatus::Unreadable)?.len();
    let mut header = [0_u8; 10];
    let header_len = file
        .read(&mut header)
        .map_err(|_| ImportStatus::Unreadable)?;
    let audio_start = if header_len >= 3 && &header[..3] == b"ID3" {
        if header_len != 10
            || !matches!(header[3], 2..=4)
            || header[4] == 0xff
            || header[5] & 15 != 0
            || header[6..10].iter().any(|byte| byte & 0x80 != 0)
        {
            return Err(ImportStatus::Malformed);
        }
        let tag_len = u64::from(header[6]) * 2_097_152
            + u64::from(header[7]) * 16_384
            + u64::from(header[8]) * 128
            + u64::from(header[9])
            + 10
            + if header[5] & 16 != 0 { 10 } else { 0 };
        if tag_len > MAX_ID3_TAG {
            return Err(ImportStatus::Unsupported);
        }
        if tag_len > file_len {
            return Err(ImportStatus::Malformed);
        }
        tag_len
    } else {
        0
    };
    let mut audio_end = file_len;
    if file_len.saturating_sub(audio_start) >= 128 {
        file.seek(SeekFrom::Start(file_len - 128))
            .map_err(|_| ImportStatus::Unreadable)?;
        let mut tag = [0_u8; 3];
        file.read_exact(&mut tag)
            .map_err(|_| ImportStatus::Unreadable)?;
        if &tag == b"TAG" {
            audio_end -= 128;
        }
    }
    validate_mp3_frame_layout(&mut file, audio_start, audio_end)?;
    file.seek(SeekFrom::Start(audio_start))
        .map_err(|_| ImportStatus::Unreadable)?;
    let mut file = file.take(audio_end - audio_start);
    let mut input = [0_u8; INPUT];
    let mut length = file
        .read(&mut input)
        .map_err(|_| ImportStatus::Unreadable)?;
    let mut decoder = nanomp3::Decoder::new();
    let mut pcm = [0_f32; nanomp3::MAX_SAMPLES_PER_FRAME];
    let (mut rate, mut channels, mut samples, mut frames) = (0_u32, 0_u16, 0_u64, 0_u64);
    loop {
        if length == 0 {
            break;
        }
        let (consumed, decoded) = decoder.decode(&input[..length], &mut pcm);
        if consumed > length {
            return Err(ImportStatus::Malformed);
        }
        if consumed == 0 {
            if length == INPUT {
                return Err(ImportStatus::Malformed);
            }
            let read = file
                .read(&mut input[length..])
                .map_err(|_| ImportStatus::Unreadable)?;
            if read == 0 {
                break;
            }
            length += read;
            continue;
        }
        input.copy_within(consumed..length, 0);
        length -= consumed;
        let info = match decoded {
            Ok(info) => {
                samples = samples
                    .checked_add(info.samples_produced as u64)
                    .ok_or(ImportStatus::Malformed)?;
                info
            }
            Err(nanomp3::DecodeError::ReservoirUnavailable(info)) => {
                // The initial reservoir-dependent frames are valid frames, but
                // their PCM is not reconstructable without preceding audio.
                samples = samples
                    .checked_add(if info.sample_rate >= 32_000 {
                        1152
                    } else {
                        576
                    })
                    .ok_or(ImportStatus::Malformed)?;
                info
            }
            Err(nanomp3::DecodeError::UnsupportedLayer(info)) => {
                let _ = info;
                return Err(ImportStatus::Unsupported);
            }
            Err(nanomp3::DecodeError::NoFrame | nanomp3::DecodeError::Corrupt(_)) => {
                return Err(ImportStatus::Malformed);
            }
            Err(_) => return Err(ImportStatus::Malformed),
        };
        if info.layer != 3 {
            return Err(ImportStatus::Unsupported);
        }
        if rate == 0 {
            rate = info.sample_rate;
            channels = u16::from(info.channels.num());
        }
        if info.sample_rate != rate || u16::from(info.channels.num()) != channels {
            return Err(ImportStatus::Unsupported);
        }
        frames += 1;
        if length < INPUT {
            let read = file
                .read(&mut input[length..])
                .map_err(|_| ImportStatus::Unreadable)?;
            length += read;
            if read == 0 && length == 0 {
                break;
            }
        }
    }
    if frames == 0 || rate == 0 || channels == 0 || samples == 0 {
        return Err(ImportStatus::Malformed);
    }
    Ok(AudioFacts {
        duration: samples as f64 / f64::from(rate),
        sample_rate: rate,
        channels,
        bit_depth: 16,
    })
}

// Supported M2 WAV matrix: integer PCM 8/16/24/32 and IEEE float32.
// Other structurally valid RIFF/WAVE format-tag/bit-depth combinations are Unsupported.
fn wav_container_padding_offsets(path: &Path) -> Option<(Vec<u64>, bool)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if file_len < 12 {
        return None;
    }
    let mut header = [0u8; 12];
    if file.read_exact(&mut header).is_err() || &header[..4] != b"RIFF" || &header[8..] != b"WAVE" {
        return None;
    }
    let end = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
    if end != file_len || end < 12 {
        return None;
    }
    let mut offset = 12u64;
    let mut fmt = false;
    let mut data = false;
    let mut encoding_supported = false;
    let mut chunks = 0;
    let mut padding_offsets = Vec::new();
    while offset < end {
        if end - offset < 8 {
            return None;
        }
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return None;
        }
        let mut chunk = [0u8; 8];
        if file.read_exact(&mut chunk).is_err() {
            return None;
        }
        chunks += 1;
        if chunks > MAX_WAV_CHUNKS {
            return None;
        }
        let size = u64::from(u32::from_le_bytes(chunk[4..].try_into().unwrap()));
        let chunk_end = offset.checked_add(8).and_then(|v| v.checked_add(size))?;
        let pad_len = size & 1;
        let padded_end = chunk_end.checked_add(pad_len)?;
        if padded_end > end {
            return None;
        }
        match &chunk[..4] {
            b"fmt " => {
                if size < 16 || file.seek(SeekFrom::Start(offset + 8)).is_err() {
                    return None;
                }
                let mut basic = [0u8; 16];
                if file.read_exact(&mut basic).is_err() {
                    return None;
                }
                let tag = u16::from_le_bytes(basic[..2].try_into().unwrap());
                let bits = u16::from_le_bytes(basic[14..16].try_into().unwrap());
                encoding_supported =
                    (tag == 1 && matches!(bits, 8 | 16 | 24 | 32)) || (tag == 3 && bits == 32);
                if !matches!(tag, 1 | 3 | 0xfffe) || !matches!(bits, 8 | 16 | 24 | 32 | 64) {
                    return None;
                }
                if tag == 0xfffe {
                    // WAVE_FORMAT_EXTENSIBLE has a 22-byte extension and a subtype GUID.
                    // Validate its framing, but leave decoding outside the supported matrix.
                    if size < 40 {
                        return None;
                    }
                    let mut extension = [0u8; 24];
                    if file.read_exact(&mut extension).is_err()
                        || u16::from_le_bytes(extension[..2].try_into().unwrap()) < 22
                    {
                        return None;
                    }
                    encoding_supported = false;
                }
                fmt = true;
            }
            b"data" => data = true,
            _ => {}
        }
        if pad_len == 1 {
            padding_offsets.push(chunk_end);
        }
        offset = padded_end;
    }
    (offset == end && fmt && data).then_some((padding_offsets, encoding_supported))
}

struct PaddingStrippingReader {
    file: std::fs::File,
    padding_offsets: Vec<u64>,
    next_padding: usize,
}

impl PaddingStrippingReader {
    fn open(path: &Path, padding_offsets: Vec<u64>) -> std::io::Result<Self> {
        Ok(Self {
            file: std::fs::File::open(path)?,
            padding_offsets,
            next_padding: 0,
        })
    }
}

impl std::io::Read for PaddingStrippingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Seek, SeekFrom};
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut written = 0;
        while written < buffer.len() {
            let position = self.file.stream_position()?;
            while self.padding_offsets.get(self.next_padding) == Some(&position) {
                self.file.seek(SeekFrom::Current(1))?;
                self.next_padding += 1;
            }
            let next = self.padding_offsets.get(self.next_padding).copied();
            let capacity = next
                .map(|padding| (padding - position) as usize)
                .unwrap_or(buffer.len() - written)
                .min(buffer.len() - written);
            if capacity == 0 {
                continue;
            }
            let count = self.file.read(&mut buffer[written..written + capacity])?;
            if count == 0 {
                break;
            }
            written += count;
        }
        Ok(written)
    }
}

fn file_sha256(path: &Path) -> std::io::Result<([u8; 32], u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut length = 0_u64;
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| std::io::Error::other("file length overflow"))?;
    }
    Ok((hasher.finalize().into(), length))
}

fn hex_digest(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn collect_regular_files(root: &Path, paths: &mut Vec<PathBuf>, complete: &mut bool) {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => {
            *complete = false;
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                *complete = false;
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(_) => {
                *complete = false;
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_regular_files(&entry.path(), paths, complete);
        } else if file_type.is_file() {
            paths.push(entry.path());
        }
    }
}

/// Filesystem presence reported by a catalog query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackAvailability {
    Available,
    Missing,
}

/// Track reference plus persisted audio facts and current availability.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackDetails {
    pub track: TrackReference,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bit_depth: Option<u16>,
    pub availability: TrackAvailability,
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
    fn committed_operation_reconciles_after_lost_acknowledgement_and_reopen() {
        let path = database_path();
        let original = {
            let mut catalog = Catalog::open(&path).unwrap();
            // The write commits; deliberately discard the acknowledgement to model an
            // uncertain commit observed by a caller after a connection loss.
            let acknowledgement = catalog
                .insert_track("uncertain-op", "/synthetic/uncertain.wav", [9; 32], 23)
                .unwrap();
            let committed_track = catalog.list_tracks().unwrap().remove(0);
            drop(acknowledgement);
            drop(catalog);
            committed_track
        };

        let mut reopened = Catalog::open(&path).unwrap();
        let reconciled = reopened
            .insert_track("uncertain-op", "/synthetic/uncertain.wav", [9; 32], 23)
            .unwrap();
        assert_eq!(reconciled, original);
        assert_eq!(reopened.list_tracks().unwrap(), vec![original.clone()]);
        assert!(matches!(
            reopened.insert_track("uncertain-op", "/synthetic/other.wav", [9; 32], 23),
            Err(CatalogError::OperationConflict(id)) if id == "uncertain-op"
        ));
        assert_eq!(reopened.list_tracks().unwrap(), vec![original]);
        drop(reopened);
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
                SELECT id, printf('/synthetic/fixture-%d.wav', id), zeroblob(32), 0 FROM ids;",
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
    fn flac_import_records_decoder_facts_and_malformed_files_per_file() {
        let root = database_path().with_extension("flac-import");
        std::fs::create_dir_all(&root).unwrap();
        let flac = root.join("tone.flac");
        std::fs::write(&flac, include_bytes!("../tests/fixtures/tone.flac")).unwrap();
        let truncated = root.join("truncated.flac");
        let mut truncated_bytes = [b'f', b'L', b'a', b'C', 0x80, 0, 0, 34].to_vec();
        truncated_bytes.extend_from_slice(b"broken");
        std::fs::write(&truncated, truncated_bytes).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_audio_directory(&root).unwrap();
        assert!(report.complete);
        assert_eq!(report.outcomes.len(), 2);
        assert_eq!(report.outcomes[0].status, ImportStatus::Imported);
        assert_eq!(report.outcomes[1].status, ImportStatus::Malformed);
        let facts: (u32, u32, u32) = catalog
            .connection
            .query_row(
                "SELECT sample_rate,channels,bit_depth FROM audio_facts WHERE track_id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(facts, (44_100, 1, 16));
        let details = catalog.list_track_details().unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].sample_rate, Some(44_100));
        assert_eq!(details[0].channels, Some(1));
        assert_eq!(details[0].bit_depth, Some(16));
        assert_eq!(details[0].availability, TrackAvailability::Available);
        assert_eq!(
            std::fs::read(&flac).unwrap(),
            include_bytes!("../tests/fixtures/tone.flac")
        );
        std::fs::remove_file(&flac).unwrap();
        assert_eq!(
            catalog.list_track_details().unwrap()[0].availability,
            TrackAvailability::Missing
        );
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn flac_import_rejects_missing_declared_samples_after_whole_frame_truncation() {
        let root = database_path().with_extension("flac-total-import");
        std::fs::create_dir_all(&root).unwrap();
        let valid = root.join("valid.flac");
        let truncated = root.join("truncated.flac");
        std::fs::write(&valid, include_bytes!("../tests/fixtures/tone-total.flac")).unwrap();
        std::fs::write(
            &truncated,
            include_bytes!("../tests/fixtures/tone-total-truncated.flac"),
        )
        .unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_audio_directory(&root).unwrap();
        assert_eq!(report.outcomes.len(), 2);
        assert_eq!(
            report
                .outcomes
                .iter()
                .find(|outcome| outcome.path.ends_with("valid.flac"))
                .unwrap()
                .status,
            ImportStatus::Imported
        );
        assert_eq!(
            report
                .outcomes
                .iter()
                .find(|outcome| outcome.path.ends_with("truncated.flac"))
                .unwrap()
                .status,
            ImportStatus::Malformed
        );
        let facts: (f64, u32, u32, u32) = catalog
            .connection
            .query_row(
                "SELECT duration_seconds,sample_rate,channels,bit_depth FROM audio_facts JOIN tracks ON tracks.id=audio_facts.track_id WHERE path=?1",
                [valid.to_string_lossy().as_ref()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(facts, (0.1, 44_100, 1, 16));
        assert_eq!(
            std::fs::read(&valid).unwrap(),
            include_bytes!("../tests/fixtures/tone-total.flac")
        );
        assert_eq!(
            std::fs::read(&truncated).unwrap(),
            include_bytes!("../tests/fixtures/tone-total-truncated.flac")
        );
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn mp3_import_records_stream_facts_and_isolates_truncated_input() {
        let root = database_path().with_extension("mp3-import");
        std::fs::create_dir_all(&root).unwrap();
        let mp3 = root.join("tone.mp3");
        std::fs::write(&mp3, include_bytes!("../tests/fixtures/tone.mp3")).unwrap();
        let tagged = root.join("tagged.mp3");
        let mut tagged_bytes = [b'I', b'D', b'3', 4, 0, 0, 0, 0, 0, 0].to_vec();
        tagged_bytes.extend_from_slice(include_bytes!("../tests/fixtures/tone.mp3"));
        std::fs::write(&tagged, tagged_bytes).unwrap();
        let truncated = root.join("truncated.mp3");
        let fixture = include_bytes!("../tests/fixtures/tone.mp3");
        std::fs::write(&truncated, &fixture[..32]).unwrap();
        let truncated_tail = root.join("truncated-tail.mp3");
        std::fs::write(&truncated_tail, &fixture[..fixture.len() - 20]).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_audio_directory(&root).unwrap();
        assert_eq!(report.outcomes.len(), 4);
        assert_eq!(report.outcomes[0].status, ImportStatus::Imported);
        assert_eq!(report.outcomes[1].status, ImportStatus::Imported);
        assert_eq!(report.outcomes[2].status, ImportStatus::Malformed);
        assert_eq!(report.outcomes[3].status, ImportStatus::Malformed);
        let facts: (u32, u32, u32) = catalog
            .connection
            .query_row(
                "SELECT sample_rate,channels,bit_depth FROM audio_facts WHERE track_id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(facts, (44_100, 1, 16));
        assert_eq!(
            std::fs::read(&mp3).unwrap(),
            include_bytes!("../tests/fixtures/tone.mp3")
        );
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn mp3_import_validates_frame_boundaries_and_keeps_channel_duration() {
        let root = database_path().with_extension("mp3-boundaries-import");
        std::fs::create_dir_all(&root).unwrap();
        let junk_prefix = root.join("junk-prefix.mp3");
        let mut junk_bytes = b"NOT AN AUDIO FRAME".repeat(10);
        junk_bytes.extend_from_slice(include_bytes!("../tests/fixtures/tone.mp3"));
        std::fs::write(&junk_prefix, junk_bytes).unwrap();
        let junk_between = root.join("junk-between-frames.mp3");
        let fixture = include_bytes!("../tests/fixtures/tone.mp3");
        let mut junk_between_bytes = fixture[..209].to_vec();
        junk_between_bytes.extend_from_slice(b"junk");
        junk_between_bytes.extend_from_slice(&fixture[209..]);
        std::fs::write(&junk_between, junk_between_bytes).unwrap();
        let id3v1 = root.join("id3v1.mp3");
        let mut id3v1_bytes = include_bytes!("../tests/fixtures/tone.mp3").to_vec();
        id3v1_bytes.extend_from_slice(b"TAG");
        id3v1_bytes.extend_from_slice(&[0; 125]);
        std::fs::write(&id3v1, id3v1_bytes).unwrap();
        let mono = root.join("tone-mono.mp3");
        std::fs::write(&mono, include_bytes!("../tests/fixtures/tone.mp3")).unwrap();
        let stereo = root.join("tone-stereo.mp3");
        std::fs::write(&stereo, include_bytes!("../tests/fixtures/tone-stereo.mp3")).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_audio_directory(&root).unwrap();
        for (name, expected) in [
            ("junk-prefix.mp3", ImportStatus::Malformed),
            ("junk-between-frames.mp3", ImportStatus::Malformed),
            ("id3v1.mp3", ImportStatus::Imported),
            ("tone-mono.mp3", ImportStatus::Imported),
            ("tone-stereo.mp3", ImportStatus::Imported),
        ] {
            assert_eq!(
                report
                    .outcomes
                    .iter()
                    .find(|outcome| outcome.path.ends_with(name))
                    .unwrap()
                    .status,
                expected,
                "unexpected outcome for {name}"
            );
        }
        for (path, expected_duration, expected_channels) in [
            (&mono, 8_064_f64 / 44_100.0, 1_u32),
            (&stereo, 10_368_f64 / 44_100.0, 2_u32),
        ] {
            let facts: (f64, u32, u32, u32) = catalog
                .connection
                .query_row(
                    "SELECT duration_seconds,sample_rate,channels,bit_depth FROM audio_facts JOIN tracks ON tracks.id=audio_facts.track_id WHERE path=?1",
                    [path.to_string_lossy().as_ref()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
            assert_eq!(facts, (expected_duration, 44_100, expected_channels, 16));
        }
        assert_eq!(
            std::fs::read(&mono).unwrap(),
            include_bytes!("../tests/fixtures/tone.mp3")
        );
        assert_eq!(
            std::fs::read(&stereo).unwrap(),
            include_bytes!("../tests/fixtures/tone-stereo.mp3")
        );
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    fn write_test_wav(path: &Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn wav_container_validator_rejects_bad_riff_extent_and_accepts_exact_extent() {
        let root = database_path().with_extension("riff-test");
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("fixture.wav");
        write_test_wav(&wav, &[1, 2, 3, 4]);
        let valid = std::fs::read(&wav).unwrap();
        assert!(wav_container_padding_offsets(&wav).is_some());
        let mut undersized = valid.clone();
        undersized[4..8].copy_from_slice(&4u32.to_le_bytes());
        std::fs::write(&wav, undersized).unwrap();
        assert!(wav_container_padding_offsets(&wav).is_none());
        let mut trailing = valid.clone();
        trailing.extend_from_slice(&[0; 4]);
        std::fs::write(&wav, trailing).unwrap();
        assert!(wav_container_padding_offsets(&wav).is_none());
        std::fs::write(&wav, valid).unwrap();
        assert!(wav_container_padding_offsets(&wav).is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn valid_float64_wav_is_reported_unsupported_by_import() {
        let root = database_path().with_extension("float64-wav");
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("float64.wav");
        write_test_wav(&wav, &[1, 2]);
        let mut bytes = std::fs::read(&wav).unwrap();
        let fmt = 12 + 8;
        bytes[fmt..fmt + 2].copy_from_slice(&3u16.to_le_bytes());
        bytes[fmt + 12..fmt + 14].copy_from_slice(&8u16.to_le_bytes());
        bytes[fmt + 14..fmt + 16].copy_from_slice(&64u16.to_le_bytes());
        let riff_size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        std::fs::write(&wav, &bytes).unwrap();
        assert!(!wav_container_padding_offsets(&wav).unwrap().1);

        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_wav_directory(&root).unwrap();
        assert_eq!(report.outcomes[0].status, ImportStatus::Unsupported);
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn valid_extensible_pcm24_wav_is_reported_unsupported_by_import() {
        let root = database_path().with_extension("extensible-wav");
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("extensible.wav");
        write_test_wav(&wav, &[1, 2]);
        let mut bytes = std::fs::read(&wav).unwrap();
        let fmt = 12 + 8;
        bytes[fmt..fmt + 2].copy_from_slice(&0xfffeu16.to_le_bytes());
        bytes[fmt - 4..fmt].copy_from_slice(&40u32.to_le_bytes());
        bytes[fmt + 8..fmt + 12].copy_from_slice(&144_000u32.to_le_bytes());
        bytes[fmt + 12..fmt + 14].copy_from_slice(&3u16.to_le_bytes());
        bytes[fmt + 14..fmt + 16].copy_from_slice(&24u16.to_le_bytes());
        let mut extension = Vec::new();
        extension.extend_from_slice(&22u16.to_le_bytes());
        extension.extend_from_slice(&24u16.to_le_bytes());
        extension.extend_from_slice(&4u32.to_le_bytes());
        extension.extend_from_slice(&[1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
        bytes.splice(fmt + 16..fmt + 16, extension);
        let riff_size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        std::fs::write(&wav, &bytes).unwrap();
        let original = std::fs::read(&wav).unwrap();
        let (_, supported) = wav_container_padding_offsets(&wav).unwrap();
        assert!(!supported);

        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_wav_directory(&root).unwrap();
        assert_eq!(report.outcomes[0].status, ImportStatus::Unsupported);
        assert_eq!(std::fs::read(&wav).unwrap(), original);
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn directory_wav_import_accepts_odd_sized_ancillary_chunk_and_preserves_source() {
        let root = database_path().with_extension("padded-wav");
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("padded.wav");
        write_test_wav(&wav, &[1, 2, 3, 4]);
        let mut bytes = std::fs::read(&wav).unwrap();
        let data_chunk = 12 + 8 + 16;
        let mut junk = b"JUNK".to_vec();
        junk.extend_from_slice(&1u32.to_le_bytes());
        junk.extend_from_slice(&[0xA5, 0]);
        bytes.splice(data_chunk..data_chunk, junk);
        let riff_size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        std::fs::write(&wav, &bytes).unwrap();
        let original = std::fs::read(&wav).unwrap();
        let (padding, supported) = wav_container_padding_offsets(&wav).unwrap();
        assert!(supported);
        assert_eq!(padding, vec![45]);
        assert!(
            hound::WavReader::new(PaddingStrippingReader::open(&wav, padding).unwrap()).is_ok()
        );

        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_wav_directory(&root).unwrap();
        assert_eq!(report.outcomes[0].status, ImportStatus::Imported);
        assert_eq!(std::fs::read(&wav).unwrap(), original);
        let facts: (i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT sample_rate,channels,bit_depth FROM audio_facts",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(facts, (48_000, 2, 16));
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn directory_import_rejects_symlink_in_root_ancestor() {
        use std::os::unix::fs::symlink;
        let base = database_path().with_extension("root-link");
        let real = base.join("real");
        std::fs::create_dir_all(real.join("sub")).unwrap();
        write_test_wav(&real.join("sub/test.wav"), &[1, 2]);
        symlink(&real, base.join("alias")).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog
            .import_wav_directory(base.join("alias/sub"))
            .unwrap();
        assert!(!report.complete);
        assert!(catalog.list_tracks().unwrap().is_empty());
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn failed_audio_facts_insert_rolls_back_import_and_reopen_retry_succeeds() {
        let root = database_path().with_extension("atomic-import");
        std::fs::create_dir_all(&root).unwrap();
        write_test_wav(&root.join("test.wav"), &[1, 2]);
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        catalog.connection.execute_batch("CREATE TRIGGER fail_audio_facts BEFORE INSERT ON audio_facts BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(catalog.import_wav_directory(&root).is_err());
        let count: i64 = catalog
            .connection
            .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        drop(catalog);
        let mut catalog = Catalog::open(&db).unwrap();
        catalog
            .connection
            .execute_batch("DROP TRIGGER fail_audio_facts;")
            .unwrap();
        let report = catalog.import_wav_directory(&root).unwrap();
        assert_eq!(report.outcomes[0].status, ImportStatus::Imported);
        let facts: i64 = catalog
            .connection
            .query_row("SELECT COUNT(*) FROM audio_facts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(facts, 1);
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn directory_wav_import_is_idempotent_reports_conflicts_and_preserves_sources() {
        let root = database_path().with_extension("media");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        let wav = root.join("nested/test.wav");
        let unsupported = root.join("notes.txt");
        let malformed = root.join("bad.wav");
        let truncated = root.join("truncated.wav");
        write_test_wav(&wav, &[1, 2, 3, 4]);
        let duplicate_path = root.join("other.wav");
        std::fs::copy(&wav, &duplicate_path).unwrap();
        write_test_wav(&truncated, &[11, 12, 13, 14]);
        let mut truncated_bytes = std::fs::read(&truncated).unwrap();
        truncated_bytes.truncate(truncated_bytes.len() - 2);
        std::fs::write(&truncated, truncated_bytes).unwrap();
        std::fs::write(&unsupported, b"text").unwrap();
        std::fs::write(&malformed, b"not a wave").unwrap();
        let original = std::fs::read(&wav).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let first = catalog.import_wav_directory(&root).unwrap();
        assert!(first.complete);
        assert!(
            first
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("test.wav") && o.status == ImportStatus::Imported)
        );
        assert!(
            first
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("other.wav") && o.status == ImportStatus::Imported)
        );
        assert!(
            first
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("notes.txt") && o.status == ImportStatus::Unsupported)
        );
        assert!(
            first
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("bad.wav") && o.status == ImportStatus::Malformed)
        );
        assert!(
            first
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("truncated.wav") && o.status == ImportStatus::Malformed)
        );
        let facts: (f64, i64, i64, i64) = catalog
            .connection
            .query_row(
                "SELECT duration_seconds,sample_rate,channels,bit_depth FROM audio_facts",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(facts, (2.0 / 48_000.0, 48_000, 2, 16));
        assert_eq!(
            std::fs::read(&wav).unwrap(),
            original,
            "import must not modify source bytes"
        );
        let second = catalog.import_wav_directory(&root).unwrap();
        assert!(
            second
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("test.wav") && o.status == ImportStatus::Unchanged)
        );
        assert_eq!(catalog.list_tracks().unwrap().len(), 2);
        write_test_wav(&wav, &[7, 8]);
        let changed = catalog.import_wav_directory(&root).unwrap();
        assert!(
            changed
                .outcomes
                .iter()
                .any(|o| o.path.ends_with("test.wav") && o.status == ImportStatus::Conflict)
        );
        assert_eq!(std::fs::read(&wav).unwrap(), {
            let changed_bytes = std::fs::read(&wav).unwrap();
            assert_ne!(changed_bytes, original);
            changed_bytes
        });
        assert_eq!(std::fs::read(&unsupported).unwrap(), b"text");
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn directory_import_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;
        let root = database_path().with_extension("symlinks");
        let outside = database_path().with_extension("outside.wav");
        std::fs::create_dir_all(&root).unwrap();
        write_test_wav(&outside, &[1, 2]);
        symlink(&outside, root.join("link.wav")).unwrap();
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        let report = catalog.import_wav_directory(&root).unwrap();
        assert!(report.complete);
        assert!(report.outcomes.is_empty());
        assert!(catalog.list_tracks().unwrap().is_empty());
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_file(outside);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_is_only_marked_after_complete_scan() {
        let root = database_path().with_extension("missing-scan");
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("present.wav");
        write_test_wav(&wav, &[1, 2]);
        let db = database_path();
        let mut catalog = Catalog::open(&db).unwrap();
        catalog.import_wav_directory(&root).unwrap();
        std::fs::remove_file(&wav).unwrap();
        let complete = catalog.import_wav_directory(&root).unwrap();
        assert!(complete.complete);
        let missing: i64 = catalog
            .connection
            .query_row("SELECT missing FROM audio_facts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(missing, 1);

        let other = database_path().with_extension("incomplete-scan");
        std::fs::create_dir_all(&other).unwrap();
        let other_wav = other.join("present.wav");
        write_test_wav(&other_wav, &[3, 4]);
        catalog.import_wav_directory(&other).unwrap();
        let incomplete = catalog
            .import_wav_directory(other.join("does-not-exist"))
            .unwrap();
        assert!(!incomplete.complete);
        let state: Vec<i64> = {
            let mut statement = catalog
                .connection
                .prepare("SELECT missing FROM audio_facts ORDER BY track_id")
                .unwrap();
            statement
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(state, vec![1, 0]);
        drop(catalog);
        remove_database(&db);
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(other);
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
