//! B2b computed tempo/beat-grid persistence (`track_tempo_grids`).
//!
//! Grids **computed by OxiDJ** for a local catalog track (`tracks.id`). Kept
//! strictly apart from vendor data: nothing here reads or writes the imported
//! `library_beats` / `library_vendor_waveforms` tables or computed waveform peaks.
//!
//! One row per track. Beats are one blob of little-endian `u32` millisecond
//! offsets rather than one row per beat: a grid is always read and replaced as
//! a whole, never queried per beat, so a blob keeps the write a single atomic
//! row replace and a 6-minute track at 128 BPM costs ~3 KB instead of ~770 rows.
//!
//! Each row records the SHA-256 of the exact source bytes it was computed from
//! (hashed before and after decoding by the caller), the analysis basis (sample
//! rate, decoded duration, BPM fold range) and the algorithm label, so a changed
//! file, range or algorithm is detectable and triggers re-analysis.

use super::{Catalog, CatalogError};
use oxidj_core::TrackId;
use oxidj_core::tempo::TempoGrid;
use rusqlite::OptionalExtension;

/// Encoding version of a `track_tempo_grids` row. Version 1: `beats` is
/// little-endian `u32` milliseconds, strictly ascending.
pub const TEMPO_GRID_FORMAT_VERSION: u32 = 1;

/// What a computed grid was derived from, besides the source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoBasis {
    /// Sample rate of the decoded audio, Hz.
    pub sample_rate: u32,
    /// Decoded audio duration, milliseconds.
    pub duration_ms: u64,
    /// BPM fold range used, inclusive whole BPM.
    pub range_min_bpm: u32,
    pub range_max_bpm: u32,
}

/// A stored computed grid with its provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredBeatGrid {
    /// SHA-256 of the source bytes the grid was computed from.
    pub source_sha256: [u8; 32],
    pub basis: TempoBasis,
    pub grid: TempoGrid,
}

fn track_key(id: TrackId) -> Result<i64, CatalogError> {
    i64::try_from(id.as_u64())
        .map_err(|_| CatalogError::InvalidRecord("track ID exceeds SQLite range"))
}

fn bad() -> CatalogError {
    CatalogError::InvalidRecord("stored tempo grid is invalid")
}

impl Catalog {
    /// Store (or replace) the computed grid of catalog track `id`. Refused,
    /// storing nothing, for an unknown track or an invalid basis. Idempotent:
    /// storing the same values again leaves the row identical.
    pub fn store_computed_beat_grid(
        &mut self,
        id: TrackId,
        source_sha256: [u8; 32],
        basis: TempoBasis,
        grid: &TempoGrid,
    ) -> Result<(), CatalogError> {
        let key = track_key(id)?;
        if basis.sample_rate == 0
            || basis.range_min_bpm == 0
            || u64::from(basis.range_max_bpm) < 2 * u64::from(basis.range_min_bpm)
            || grid.beats_ms().len() < 2
        {
            return Err(CatalogError::InvalidRecord("tempo grid basis is invalid"));
        }
        let duration_ms = i64::try_from(basis.duration_ms)
            .map_err(|_| CatalogError::InvalidRecord("duration exceeds SQLite range"))?;
        let bpm_hundredths = (grid.bpm() * 100.0).round();
        if !(1.0..=f64::from(u32::MAX)).contains(&bpm_hundredths) {
            return Err(CatalogError::InvalidRecord("tempo grid BPM out of range"));
        }
        let beats: Vec<u8> = grid
            .beats_ms()
            .iter()
            .flat_map(|b| b.to_le_bytes())
            .collect();
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let known: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id=?1)",
            [key],
            |row| row.get(0),
        )?;
        if !known {
            return Err(CatalogError::InvalidRecord("unknown track"));
        }
        tx.execute(
            "INSERT INTO track_tempo_grids(track_id,format_version,algorithm,bpm_hundredths,\
             confidence,sample_rate,duration_ms,range_min_bpm,range_max_bpm,source_sha256,beats) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) \
             ON CONFLICT(track_id) DO UPDATE SET format_version=excluded.format_version,\
             algorithm=excluded.algorithm,bpm_hundredths=excluded.bpm_hundredths,\
             confidence=excluded.confidence,sample_rate=excluded.sample_rate,\
             duration_ms=excluded.duration_ms,range_min_bpm=excluded.range_min_bpm,\
             range_max_bpm=excluded.range_max_bpm,source_sha256=excluded.source_sha256,\
             beats=excluded.beats",
            rusqlite::params![
                key,
                TEMPO_GRID_FORMAT_VERSION,
                grid.algorithm(),
                bpm_hundredths as i64,
                f64::from(grid.confidence()),
                basis.sample_rate,
                duration_ms,
                basis.range_min_bpm,
                basis.range_max_bpm,
                source_sha256.as_slice(),
                beats
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Remove the computed grid of track `id` (a stale grid whose source no
    /// longer yields one). Returns whether a row was removed.
    pub fn remove_computed_beat_grid(&mut self, id: TrackId) -> Result<bool, CatalogError> {
        let removed = self.connection.execute(
            "DELETE FROM track_tempo_grids WHERE track_id=?1",
            [track_key(id)?],
        )?;
        Ok(removed > 0)
    }

    /// The computed grid of track `id`; `None` when none is stored. A row with
    /// an unknown format version or invalid contents is refused.
    pub fn computed_beat_grid(&self, id: TrackId) -> Result<Option<StoredBeatGrid>, CatalogError> {
        let row = self
            .connection
            .query_row(
                "SELECT format_version,algorithm,bpm_hundredths,confidence,sample_rate,\
                 duration_ms,range_min_bpm,range_max_bpm,source_sha256,beats \
                 FROM track_tempo_grids WHERE track_id=?1",
                [track_key(id)?],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, Vec<u8>>(8)?,
                        row.get::<_, Vec<u8>>(9)?,
                    ))
                },
            )
            .optional()?;
        let Some((version, algorithm, bpm, confidence, rate, duration, lo, hi, digest, blob)) = row
        else {
            return Ok(None);
        };
        if version != i64::from(TEMPO_GRID_FORMAT_VERSION) {
            return Err(CatalogError::InvalidRecord(
                "stored tempo grid format version is not supported",
            ));
        }
        if blob.is_empty() || !blob.len().is_multiple_of(4) {
            return Err(bad());
        }
        let beats = blob
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        let grid = TempoGrid::new(bpm as f64 / 100.0, beats, confidence as f32, algorithm)
            .map_err(|_| bad())?;
        Ok(Some(StoredBeatGrid {
            source_sha256: digest.try_into().map_err(|_| bad())?,
            basis: TempoBasis {
                sample_rate: u32::try_from(rate).map_err(|_| bad())?,
                duration_ms: u64::try_from(duration).map_err(|_| bad())?,
                range_min_bpm: u32::try_from(lo).map_err(|_| bad())?,
                range_max_bpm: u32::try_from(hi).map_err(|_| bad())?,
            },
            grid,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("oxidj-tempo-{}-{nonce}.sqlite", std::process::id()))
    }

    const BASIS: TempoBasis = TempoBasis {
        sample_rate: 44_100,
        duration_ms: 30_000,
        range_min_bpm: 70,
        range_max_bpm: 180,
    };

    fn grid(bpm: f64) -> TempoGrid {
        TempoGrid::new(bpm, vec![300, 769, 1_238], 0.42, "taktus 0.1.0").unwrap()
    }

    /// Every row of every vendor/imported/computed-waveform table, as text.
    fn vendor_rows(catalog: &Catalog) -> Vec<String> {
        [
            "library_tracks",
            "library_beats",
            "library_cues",
            "library_vendor_waveforms",
            "track_waveforms",
        ]
        .iter()
        .flat_map(|table| {
            let mut statement = catalog
                .connection
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let columns = statement.column_count();
            statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get_ref(i).map(|v| format!("{table}:{v:?}")))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .map(|r| r.unwrap().join("|"))
                .collect::<Vec<_>>()
        })
        .collect()
    }

    #[test]
    fn computed_grid_round_trips_replaces_idempotently_and_refuses_bad_input() {
        let path = db();
        let mut catalog = Catalog::open(&path).unwrap();
        let track = catalog.insert_track("op", "/a.wav", [7; 32], 10).unwrap();
        assert_eq!(catalog.computed_beat_grid(track.id).unwrap(), None);
        catalog
            .store_computed_beat_grid(track.id, [7; 32], BASIS, &grid(128.0))
            .unwrap();
        catalog
            .store_computed_beat_grid(track.id, [7; 32], BASIS, &grid(128.0))
            .unwrap();
        drop(catalog);
        let mut catalog = Catalog::open(&path).unwrap();
        let stored = catalog.computed_beat_grid(track.id).unwrap().unwrap();
        assert_eq!(
            stored,
            StoredBeatGrid {
                source_sha256: [7; 32],
                basis: BASIS,
                grid: grid(128.0)
            }
        );
        let count: i64 = catalog
            .connection
            .query_row("SELECT COUNT(*) FROM track_tempo_grids", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "idempotent writes keep one row");
        // Replacement (new source digest) overwrites the whole row.
        catalog
            .store_computed_beat_grid(track.id, [9; 32], BASIS, &grid(127.5))
            .unwrap();
        let replaced = catalog.computed_beat_grid(track.id).unwrap().unwrap();
        assert_eq!(
            (replaced.source_sha256, replaced.grid.bpm()),
            ([9; 32], 127.5)
        );
        // Unknown track, invalid basis or a one-beat grid: refused, nothing changes.
        let unknown = TrackId::new(99).unwrap();
        assert!(
            catalog
                .store_computed_beat_grid(unknown, [7; 32], BASIS, &grid(128.0))
                .is_err()
        );
        let narrow = TempoBasis {
            range_max_bpm: 139,
            ..BASIS
        };
        assert!(
            catalog
                .store_computed_beat_grid(track.id, [7; 32], narrow, &grid(128.0))
                .is_err()
        );
        let one = TempoGrid::new(128.0, vec![5], 0.4, "x").unwrap();
        assert!(
            catalog
                .store_computed_beat_grid(track.id, [7; 32], BASIS, &one)
                .is_err()
        );
        assert_eq!(catalog.computed_beat_grid(unknown).unwrap(), None);
        assert_eq!(
            catalog.computed_beat_grid(track.id).unwrap(),
            Some(replaced)
        );
        // Corrupt blob or unknown version fails closed on read; CHECK pins the version.
        catalog
            .connection
            .execute("UPDATE track_tempo_grids SET beats=x'0500000001000000'", [])
            .unwrap();
        assert!(
            catalog.computed_beat_grid(track.id).is_err(),
            "descending beats"
        );
        assert!(
            catalog
                .connection
                .execute("UPDATE track_tempo_grids SET format_version=2", [])
                .is_err()
        );
        assert!(catalog.remove_computed_beat_grid(track.id).unwrap());
        assert!(!catalog.remove_computed_beat_grid(track.id).unwrap());
        assert_eq!(catalog.computed_beat_grid(track.id).unwrap(), None);
        drop(catalog);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn computed_grid_never_touches_vendor_imported_or_waveform_tables() {
        use crate::{LibraryBeat, LibrarySnapshot, LibraryTrack, VendorWaveform};
        let path = db();
        let mut catalog = Catalog::open(&path).unwrap();
        let track = catalog.insert_track("op", "/a.wav", [7; 32], 10).unwrap();
        catalog
            .import_library(&LibrarySnapshot {
                tracks: vec![LibraryTrack {
                    title: "T".into(),
                    tempo_hundredths_bpm: 12_000,
                    location: "file://localhost/a.wav".into(),
                    beat_grid: vec![
                        LibraryBeat {
                            time_ms: 10,
                            tempo_hundredths_bpm: 12_000,
                            beat_in_bar: 1,
                        },
                        LibraryBeat {
                            time_ms: 510,
                            tempo_hundredths_bpm: 12_000,
                            beat_in_bar: 2,
                        },
                    ],
                    vendor_waveforms: vec![VendorWaveform::RekordboxDetail(vec![[3, 1]])],
                    ..LibraryTrack::default()
                }],
                roots: vec![],
            })
            .unwrap();
        let before = vendor_rows(&catalog);
        assert!(before.iter().any(|r| r.starts_with("library_beats:")));
        catalog
            .store_computed_beat_grid(track.id, [7; 32], BASIS, &grid(128.0))
            .unwrap();
        catalog
            .store_computed_beat_grid(track.id, [8; 32], BASIS, &grid(126.0))
            .unwrap();
        assert_eq!(vendor_rows(&catalog), before);
        catalog.remove_computed_beat_grid(track.id).unwrap();
        assert_eq!(vendor_rows(&catalog), before);
        drop(catalog);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn v6_catalog_migrates_to_v7_keeping_data_and_durability() {
        let path = db();
        {
            let mut connection = rusqlite::Connection::open(&path).unwrap();
            rusqlite_migration::Migrations::from_slice(&crate::MIGRATION_LIST[..6])
                .to_latest(&mut connection)
                .unwrap();
            let version: i64 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(version, 6);
            connection
                .execute(
                    "INSERT INTO tracks(id,path,sha256,byte_length) VALUES(1,'/a.wav',?1,10)",
                    [[7_u8; 32].as_slice()],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO track_waveforms(track_id,format_version,sample_rate,\
                     samples_per_peak,sample_count,source_sha256,peaks) \
                     VALUES(1,1,44100,294,1400,?1,x'010203040506')",
                    [[7_u8; 32].as_slice()],
                )
                .unwrap();
        }
        let mut catalog = Catalog::open(&path).unwrap();
        let version: i64 = catalog
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let journal: String = catalog
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(
            (version, journal.to_ascii_lowercase().as_str()),
            (crate::SCHEMA_VERSION, "wal")
        );
        assert_eq!(crate::SCHEMA_VERSION, 7);
        let id = catalog.list_tracks().unwrap()[0].id;
        let kept: Vec<u8> = catalog
            .connection
            .query_row(
                "SELECT peaks FROM track_waveforms WHERE track_id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, [1, 2, 3, 4, 5, 6], "v6 data kept byte for byte");
        assert_eq!(catalog.computed_beat_grid(id).unwrap(), None);
        catalog
            .store_computed_beat_grid(id, [7; 32], BASIS, &grid(128.0))
            .unwrap();
        drop(catalog);
        assert!(
            Catalog::open(&path)
                .unwrap()
                .computed_beat_grid(id)
                .unwrap()
                .is_some()
        );
        let _ = std::fs::remove_file(&path);
    }
}
