//! W2 waveform persistence. Two deliberately separate stores:
//!
//! - `track_waveforms`: peaks **computed by OxiDJ** for a local catalog track
//!   (`tracks.id`), tagged with the source file digest they were computed from.
//! - `library_vendor_waveforms`: waveforms **imported verbatim from a vendor DJ
//!   library** for an imported library track (`library_tracks.position`), labelled
//!   by vendor kind. Vendor data is never written into, or read as, computed peaks.

use super::{Catalog, CatalogError};
use oxidj_core::TrackId;
use oxidj_core::waveform::{Band, Peak, WaveformPeaks};
use rusqlite::OptionalExtension;

/// Encoding version of the `track_waveforms.peaks` blob. Version 1: level-0 peaks
/// only (the mipmap is rebuilt on read), band-major low/mid/high, each peak stored
/// as `[min, max]` bytes.
pub const WAVEFORM_FORMAT_VERSION: u32 = 1;

/// Computed peaks plus the SHA-256 of the source file they were computed from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredWaveform {
    pub source_sha256: [u8; 32],
    pub peaks: WaveformPeaks,
}

/// A vendor-derived waveform, carried exactly as the vendor stored it (decoded to
/// fields, never recomputed or resampled). Kept apart from computed peaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VendorWaveform {
    /// Rekordbox ANLZ `PWV3` detail: per column `[height 0..=31, whiteness 0..=7]`.
    RekordboxDetail(Vec<[u8; 2]>),
    /// Rekordbox ANLZ `PWV5` colour detail: per column
    /// `[red 0..=7, green 0..=7, blue 0..=7, height 0..=31]`.
    RekordboxColourDetail(Vec<[u8; 4]>),
}

impl VendorWaveform {
    /// Stored `kind` label; every label names its vendor source.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::RekordboxDetail(_) => "rekordbox-anlz-pwv3",
            Self::RekordboxColourDetail(_) => "rekordbox-anlz-pwv5",
        }
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, CatalogError> {
        let valid = match self {
            Self::RekordboxDetail(columns) => columns.iter().all(|&[h, w]| h <= 31 && w <= 7),
            Self::RekordboxColourDetail(columns) => columns
                .iter()
                .all(|&[r, g, b, h]| r <= 7 && g <= 7 && b <= 7 && h <= 31),
        };
        if !valid {
            return Err(CatalogError::InvalidRecord(
                "vendor waveform value out of range",
            ));
        }
        Ok(match self {
            Self::RekordboxDetail(columns) => columns.concat(),
            Self::RekordboxColourDetail(columns) => columns.concat(),
        })
    }

    pub(crate) fn decode(kind: &str, data: &[u8]) -> Result<Self, CatalogError> {
        let bad = || CatalogError::InvalidRecord("stored vendor waveform is invalid");
        let decoded = match kind {
            "rekordbox-anlz-pwv3" if data.len().is_multiple_of(2) => {
                Self::RekordboxDetail(data.as_chunks::<2>().0.to_vec())
            }
            "rekordbox-anlz-pwv5" if data.len().is_multiple_of(4) => {
                Self::RekordboxColourDetail(data.as_chunks::<4>().0.to_vec())
            }
            _ => return Err(bad()),
        };
        decoded.encode().map_err(|_| bad())?;
        Ok(decoded)
    }

    /// Canonical storage/read-back order of kinds within one track.
    pub(crate) const fn order(&self) -> u8 {
        match self {
            Self::RekordboxDetail(_) => 0,
            Self::RekordboxColourDetail(_) => 1,
        }
    }
}

fn track_key(id: TrackId) -> Result<i64, CatalogError> {
    i64::try_from(id.as_u64())
        .map_err(|_| CatalogError::InvalidRecord("track ID exceeds SQLite range"))
}

fn encode_peaks(peaks: &WaveformPeaks) -> Vec<u8> {
    Band::ALL
        .iter()
        .flat_map(|&band| peaks.band(0, band).unwrap_or(&[]).iter())
        .flat_map(|peak| [peak.min(), peak.max()])
        .collect()
}

fn decode_peaks(
    sample_rate: i64,
    samples_per_peak: i64,
    sample_count: i64,
    blob: &[u8],
) -> Result<WaveformPeaks, CatalogError> {
    let bad = || CatalogError::InvalidRecord("stored waveform is invalid");
    let sample_rate = u32::try_from(sample_rate).map_err(|_| bad())?;
    let samples_per_peak = u32::try_from(samples_per_peak).map_err(|_| bad())?;
    let sample_count = u64::try_from(sample_count).map_err(|_| bad())?;
    if blob.is_empty() || !blob.len().is_multiple_of(6) {
        return Err(bad());
    }
    let columns = blob.len() / 6;
    let mut bands: [Vec<Peak>; 3] = Default::default();
    for (band, bytes) in bands.iter_mut().zip(blob.chunks(columns * 2)) {
        *band = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[min, max]| Peak::new(min, max).ok_or_else(bad))
            .collect::<Result<_, _>>()?;
    }
    WaveformPeaks::from_base(sample_rate, samples_per_peak, sample_count, bands).map_err(|_| bad())
}

impl Catalog {
    /// Store (or replace) the computed peaks of catalog track `id`, computed from a
    /// source whose digest is `source_sha256`. Refused, storing nothing, when the
    /// track is unknown or its recorded digest differs.
    pub fn store_waveform(
        &mut self,
        id: TrackId,
        source_sha256: [u8; 32],
        peaks: &WaveformPeaks,
    ) -> Result<(), CatalogError> {
        let key = track_key(id)?;
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let recorded: Option<Vec<u8>> = tx
            .query_row("SELECT sha256 FROM tracks WHERE id=?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        match recorded {
            None => return Err(CatalogError::InvalidRecord("unknown track")),
            Some(digest) if digest.as_slice() != source_sha256 => {
                return Err(CatalogError::InvalidRecord(
                    "waveform source digest does not match the track",
                ));
            }
            Some(_) => {}
        }
        let sample_count = i64::try_from(peaks.sample_count())
            .map_err(|_| CatalogError::InvalidRecord("sample count exceeds SQLite range"))?;
        tx.execute(
            "INSERT INTO track_waveforms(track_id,format_version,sample_rate,samples_per_peak,\
             sample_count,source_sha256,peaks) VALUES(?1,?2,?3,?4,?5,?6,?7) \
             ON CONFLICT(track_id) DO UPDATE SET format_version=excluded.format_version,\
             sample_rate=excluded.sample_rate,samples_per_peak=excluded.samples_per_peak,\
             sample_count=excluded.sample_count,source_sha256=excluded.source_sha256,\
             peaks=excluded.peaks",
            rusqlite::params![
                key,
                WAVEFORM_FORMAT_VERSION,
                peaks.sample_rate(),
                peaks.samples_per_peak(),
                sample_count,
                source_sha256.as_slice(),
                encode_peaks(peaks)
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Read the computed peaks of track `id`; `None` when none are stored. A row
    /// with an unknown format version or an invalid blob is refused.
    pub fn read_waveform(&self, id: TrackId) -> Result<Option<StoredWaveform>, CatalogError> {
        let row = self
            .connection
            .query_row(
                "SELECT format_version,sample_rate,samples_per_peak,sample_count,source_sha256,\
                 peaks FROM track_waveforms WHERE track_id=?1",
                [track_key(id)?],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, Vec<u8>>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((version, rate, spp, count, digest, blob)) = row else {
            return Ok(None);
        };
        if version != i64::from(WAVEFORM_FORMAT_VERSION) {
            return Err(CatalogError::InvalidRecord(
                "stored waveform format version is not supported",
            ));
        }
        Ok(Some(StoredWaveform {
            source_sha256: digest
                .try_into()
                .map_err(|_| CatalogError::InvalidRecord("stored waveform is invalid"))?,
            peaks: decode_peaks(rate, spp, count, &blob)?,
        }))
    }

    /// Vendor-derived waveforms of the imported library track matched by
    /// [`Self::library_position_for_locations`] (exact, else unambiguous
    /// device-relative suffix), in
    /// [`VendorWaveform`] declaration order. Empty when no such track or none
    /// stored. Never touches computed peaks.
    pub fn vendor_waveforms_for_locations(
        &self,
        locations: &[&str],
    ) -> Result<Vec<VendorWaveform>, CatalogError> {
        let Some(position) = self.library_position_for_locations(locations)? else {
            return Ok(Vec::new());
        };
        let mut statement = self
            .connection
            .prepare("SELECT kind,data FROM library_vendor_waveforms WHERE track_position=?1")?;
        let mut waveforms = statement
            .query_map([position], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?
            .map(|row| {
                let (kind, data) = row?;
                VendorWaveform::decode(&kind, &data)
            })
            .collect::<Result<Vec<_>, _>>()?;
        waveforms.sort_by_key(VendorWaveform::order);
        Ok(waveforms)
    }

    /// Position of the imported library track for `locations` (one local file:
    /// its path and/or `file://localhost` URI). The single lookup used by
    /// [`Self::beat_grid_for_locations`] and [`Self::vendor_waveforms_for_locations`].
    ///
    /// 1. Exact: the lowest position whose stored `location` equals one of
    ///    `locations` byte for byte. Always wins when it finds anything.
    /// 2. Device-relative fallback: a stored location whose percent-decoded path
    ///    starts with `/Contents/` (a Rekordbox device export, as the importer
    ///    stores it; never normalised) matches a local path that ends with that
    ///    decoded path. Because the stored path begins with `/`, the match is
    ///    always on a path-component boundary; comparison is case-sensitive
    ///    bytes. More than one matching library track fails closed (`None`).
    ///    Drive-qualified or other absolute library locations never take part.
    pub(crate) fn library_position_for_locations(
        &self,
        locations: &[&str],
    ) -> Result<Option<i64>, CatalogError> {
        if let Some(position) = self.library_position_exact(locations)? {
            return Ok(Some(position));
        }
        let wanted: Vec<Vec<u8>> = locations.iter().filter_map(|l| decoded_path(l)).collect();
        if wanted.is_empty() {
            return Ok(None);
        }
        let mut statement = self
            .connection
            .prepare("SELECT position, location FROM library_tracks ORDER BY position")?;
        let mut rows = statement.query([])?;
        let mut found = None;
        while let Some(row) = rows.next()? {
            let location: String = row.get(1)?;
            let Some(stored) = decoded_path(&location) else {
                continue;
            };
            if !stored.starts_with(b"/Contents/") {
                continue;
            }
            if wanted.iter().any(|local| local.ends_with(&stored)) {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(row.get::<_, i64>(0)?);
            }
        }
        Ok(found)
    }

    fn library_position_exact(&self, locations: &[&str]) -> Result<Option<i64>, CatalogError> {
        let mut position = None;
        for location in locations {
            let found: Option<i64> = self
                .connection
                .query_row(
                    "SELECT MIN(position) FROM library_tracks WHERE location=?1",
                    [location],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            position = match (position, found) {
                (Some(a), Some(b)) => Some(std::cmp::min(a, b)),
                (a, b) => a.or(b),
            };
        }
        Ok(position)
    }
}

/// The path a location names, as raw bytes: `file://localhost/…` URIs are
/// percent-decoded; plain absolute paths are taken verbatim. `None` for other
/// forms or malformed escapes.
pub(crate) fn decoded_path(location: &str) -> Option<Vec<u8>> {
    let Some(encoded) = location.strip_prefix("file://localhost") else {
        return location
            .starts_with('/')
            .then(|| location.as_bytes().to_vec());
    };
    if !encoded.starts_with('/') {
        return None;
    }
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "oxidj-waveform-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn peaks(seed: u8) -> WaveformPeaks {
        let band = |offset: u8| {
            (0..5u8)
                .map(|i| Peak::new(100 - i - offset, 140 + i + seed).unwrap())
                .collect::<Vec<_>>()
        };
        WaveformPeaks::from_base(44_100, 294, 1_400, [band(0), band(3), band(7)]).unwrap()
    }

    #[test]
    fn computed_peaks_round_trip_replace_and_refuse_mismatched_sources() {
        let path = db();
        let mut catalog = Catalog::open(&path).unwrap();
        let track = catalog.insert_track("op", "/a.wav", [7; 32], 10).unwrap();
        assert_eq!(catalog.read_waveform(track.id).unwrap(), None);
        catalog
            .store_waveform(track.id, [7; 32], &peaks(0))
            .unwrap();
        drop(catalog);
        let mut catalog = Catalog::open(&path).unwrap();
        let stored = catalog.read_waveform(track.id).unwrap().unwrap();
        assert_eq!(stored.peaks, peaks(0));
        assert_eq!(stored.source_sha256, [7; 32]);
        catalog
            .store_waveform(track.id, [7; 32], &peaks(9))
            .unwrap();
        assert_eq!(
            catalog.read_waveform(track.id).unwrap().unwrap().peaks,
            peaks(9)
        );
        // Wrong digest or unknown track: refused, nothing changes.
        assert!(
            catalog
                .store_waveform(track.id, [8; 32], &peaks(0))
                .is_err()
        );
        let unknown = TrackId::new(99).unwrap();
        assert!(catalog.store_waveform(unknown, [7; 32], &peaks(0)).is_err());
        assert_eq!(catalog.read_waveform(unknown).unwrap(), None);
        assert_eq!(
            catalog.read_waveform(track.id).unwrap().unwrap().peaks,
            peaks(9)
        );
        // An unknown stored version or a corrupt blob fails closed on read.
        catalog
            .connection
            .execute("UPDATE track_waveforms SET peaks=x'FF00FF00FF00'", [])
            .unwrap();
        assert!(catalog.read_waveform(track.id).is_err());
        assert!(
            catalog
                .connection
                .execute("UPDATE track_waveforms SET format_version=2", [])
                .is_err(),
            "schema CHECK pins the format version"
        );
        drop(catalog);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn v5_catalog_migrates_to_v6_keeping_data_and_durability() {
        let path = db();
        {
            let mut connection = rusqlite::Connection::open(&path).unwrap();
            rusqlite_migration::Migrations::from_slice(&crate::MIGRATION_LIST[..5])
                .to_latest(&mut connection)
                .unwrap();
            let version: i64 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(version, 5);
            connection
                .execute(
                    "INSERT INTO tracks(id,path,sha256,byte_length) VALUES(1,'/a.wav',?1,10)",
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
        let synchronous: i64 = catalog
            .connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(
            (version, journal.to_ascii_lowercase().as_str(), synchronous),
            (crate::SCHEMA_VERSION, "wal", 2)
        );
        let tracks = catalog.list_tracks().unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(catalog.read_waveform(tracks[0].id).unwrap(), None);
        catalog
            .store_waveform(tracks[0].id, [7; 32], &peaks(1))
            .unwrap();
        drop(catalog);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn vendor_waveforms_validate_ranges_and_labels() {
        let detail = VendorWaveform::RekordboxDetail(vec![[31, 7], [0, 0]]);
        let colour = VendorWaveform::RekordboxColourDetail(vec![[7, 0, 3, 31]]);
        for w in [&detail, &colour] {
            assert!(w.kind().starts_with("rekordbox-anlz-"));
            assert_eq!(
                &VendorWaveform::decode(w.kind(), &w.encode().unwrap()).unwrap(),
                w
            );
        }
        assert!(
            VendorWaveform::RekordboxDetail(vec![[32, 0]])
                .encode()
                .is_err()
        );
        assert!(
            VendorWaveform::RekordboxColourDetail(vec![[8, 0, 0, 0]])
                .encode()
                .is_err()
        );
        assert!(VendorWaveform::decode("rekordbox-anlz-pwv3", &[1]).is_err());
        assert!(VendorWaveform::decode("computed", &[1, 1]).is_err());
    }
}
