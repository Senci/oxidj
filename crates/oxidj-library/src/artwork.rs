//! Deck album artwork: extract embedded/cover art, decode it, and cache it.
//!
//! Artwork comes from the track's own audio file (embedded cover art, read with
//! `lofty`), falling back to a `cover.jpg`/`folder.jpg` beside the file. The
//! extracted image is decoded once to RGBA and cached per catalog track in
//! SQLite (`track_artwork`), so loading the same track again is a cache hit.
//! The cache is bounded by a configurable LRU byte budget
//! ([`DEFAULT_ARTWORK_CACHE_BYTES`]); inserting past the budget evicts the
//! least-recently-used entries first.
//!
//! This module never decodes audio: `lofty` reads container metadata only.

use crate::{Catalog, CatalogError, TrackReference};
use oxidj_core::TrackId;
use rusqlite::OptionalExtension;
use std::path::{Path, PathBuf};

/// Default decoded-artwork cache budget: 100 MB of RGBA pixels.
pub const DEFAULT_ARTWORK_CACHE_BYTES: u64 = 100 * 1024 * 1024;

/// Cover-file names searched beside a track, in priority order.
pub const COVER_FILE_NAMES: [&str; 4] = ["cover.jpg", "folder.jpg", "cover.png", "folder.png"];

/// Decoded album artwork: 8-bit RGBA pixels, row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artwork {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Artwork {
    /// Build from RGBA pixels; refuses a buffer that does not match the size.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ArtworkError> {
        if width == 0 || height == 0 {
            return Err(ArtworkError::Decode(format!(
                "artwork has zero dimension ({width}x{height})"
            )));
        }
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(ArtworkError::Decode(format!(
                "artwork buffer is {} bytes, expected {expected} for {width}x{height}",
                rgba.len()
            )));
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// Encoded size in bytes (the cache's accounting unit).
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }
}

/// Artwork extraction/decoding failure.
#[derive(Debug)]
pub enum ArtworkError {
    Catalog(CatalogError),
    Io(std::io::Error),
    Decode(String),
}

impl std::fmt::Display for ArtworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "catalog error: {error}"),
            Self::Io(error) => write!(f, "artwork I/O error: {error}"),
            Self::Decode(reason) => write!(f, "artwork decode error: {reason}"),
        }
    }
}

impl std::error::Error for ArtworkError {}

impl From<CatalogError> for ArtworkError {
    fn from(value: CatalogError) -> Self {
        Self::Catalog(value)
    }
}

impl From<std::io::Error> for ArtworkError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

fn track_key(id: TrackId) -> Result<i64, CatalogError> {
    i64::try_from(id.as_u64())
        .map_err(|_| CatalogError::InvalidRecord("track ID exceeds SQLite range"))
}

fn now_nanos() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(i64::MAX as u128) as i64
}

/// Extract artwork for a track: embedded first, then a cover file beside it.
pub fn extract_for_track(path: &Path) -> Result<Option<Artwork>, ArtworkError> {
    if let Some(artwork) = extract_embedded(path)? {
        return Ok(Some(artwork));
    }
    if let Some(cover) = find_cover_file(path) {
        return decode_image_file(&cover);
    }
    Ok(None)
}

/// Read the embedded cover art of an audio file, if it has one.
pub fn extract_embedded(path: &Path) -> Result<Option<Artwork>, ArtworkError> {
    use lofty::prelude::*;
    let tagged = lofty::probe::Probe::open(path)
        .map_err(|error| ArtworkError::Decode(format!("cannot open audio file: {error}")))?
        .guess_file_type()?
        .read()
        .map_err(|error| ArtworkError::Decode(format!("cannot read audio metadata: {error}")))?;
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(None);
    };
    let pictures = tag.pictures();
    // Prefer the front cover; fall back to the first stored picture.
    let chosen = pictures
        .iter()
        .find(|picture| picture.pic_type() == lofty::picture::PictureType::CoverFront)
        .or_else(|| pictures.first());
    let Some(picture) = chosen else {
        return Ok(None);
    };
    decode_image_bytes(picture.data())
}

/// Decode an encoded image (JPEG/PNG/BMP/GIF/WebP/TIFF) to RGBA.
pub fn decode_image_bytes(bytes: &[u8]) -> Result<Option<Artwork>, ArtworkError> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let image = image::load_from_memory(bytes)
        .map_err(|error| ArtworkError::Decode(format!("cannot decode artwork: {error}")))?;
    Ok(Some(dynamic_to_artwork(image)))
}

/// Decode an image file to RGBA.
pub fn decode_image_file(path: &Path) -> Result<Option<Artwork>, ArtworkError> {
    let bytes = std::fs::read(path)?;
    decode_image_bytes(&bytes)
}

fn dynamic_to_artwork(image: image::DynamicImage) -> Artwork {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Artwork {
        width,
        height,
        rgba: rgba.into_raw(),
    }
}

/// Find a cover file beside `track`: the first of [`COVER_FILE_NAMES`] that
/// exists (exact name, then a case-insensitive match).
pub fn find_cover_file(track: &Path) -> Option<PathBuf> {
    let dir = track.parent()?;
    for name in COVER_FILE_NAMES {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut fallback = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let lower = name.to_string_lossy().to_ascii_lowercase();
        if COVER_FILE_NAMES.contains(&lower.as_str()) && entry.path().is_file() {
            fallback = Some(entry.path());
            break;
        }
    }
    fallback
}

impl Catalog {
    /// Read the cached decoded artwork for `id`, marking it most-recently used.
    pub fn read_artwork(&mut self, id: TrackId) -> Result<Option<Artwork>, CatalogError> {
        let key = track_key(id)?;
        let row = self
            .connection
            .query_row(
                "SELECT width,height,rgba FROM track_artwork WHERE track_id=?1",
                [key],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((width, height, rgba)) = row else {
            return Ok(None);
        };
        self.connection.execute(
            "UPDATE track_artwork SET last_used=?1 WHERE track_id=?2",
            rusqlite::params![now_nanos(), key],
        )?;
        let width = u32::try_from(width)
            .map_err(|_| CatalogError::InvalidRecord("stored artwork width is invalid"))?;
        let height = u32::try_from(height)
            .map_err(|_| CatalogError::InvalidRecord("stored artwork height is invalid"))?;
        Ok(Some(Artwork::new(width, height, rgba).map_err(|_| {
            CatalogError::InvalidRecord("stored artwork dimensions do not match its buffer")
        })?))
    }

    /// Store decoded artwork for `id`, validated against the track's recorded
    /// digest, then evict least-recently-used entries past the budget.
    pub fn store_artwork(
        &mut self,
        id: TrackId,
        source_sha256: [u8; 32],
        artwork: &Artwork,
    ) -> Result<(), CatalogError> {
        let key = track_key(id)?;
        if artwork.width == 0
            || artwork.height == 0
            || artwork.rgba.len() != artwork.width as usize * artwork.height as usize * 4
        {
            return Err(CatalogError::InvalidRecord(
                "artwork dimensions do not match its buffer",
            ));
        }
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
                    "artwork source digest does not match the track",
                ));
            }
            Some(_) => {}
        }
        tx.execute(
            "INSERT INTO track_artwork(track_id,width,height,rgba,source_sha256,last_used) \
             VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(track_id) DO UPDATE SET \
             width=excluded.width,height=excluded.height,rgba=excluded.rgba,\
             source_sha256=excluded.source_sha256,last_used=excluded.last_used",
            rusqlite::params![
                key,
                artwork.width,
                artwork.height,
                artwork.rgba.as_slice(),
                source_sha256.as_slice(),
                now_nanos()
            ],
        )?;
        tx.commit()?;
        self.evict_artwork_lru()
    }

    /// Load artwork for `id`: the cache first, else extract it from the track's
    /// own file and cache the result. `Ok(None)` = no artwork anywhere.
    pub fn load_artwork(&mut self, id: TrackId) -> Result<Option<Artwork>, ArtworkError> {
        if let Some(artwork) = self.read_artwork(id)? {
            return Ok(Some(artwork));
        }
        let TrackReference { path, sha256, .. } = self
            .read_track(id)?
            .ok_or_else(|| ArtworkError::Decode("track is not in the catalog".into()))?;
        let Some(artwork) = extract_for_track(Path::new(&path))? else {
            return Ok(None);
        };
        self.store_artwork(id, sha256, &artwork)?;
        Ok(Some(artwork))
    }

    /// Set the cache byte budget, evicting least-recently-used entries past it.
    pub fn set_artwork_cache_budget(&mut self, bytes: u64) -> Result<(), CatalogError> {
        self.artwork_budget = bytes;
        self.evict_artwork_lru()
    }

    /// The current cache byte budget.
    pub fn artwork_cache_budget(&self) -> u64 {
        self.artwork_budget
    }

    /// Total decoded-artwork bytes currently cached.
    pub fn artwork_cache_bytes(&self) -> Result<u64, CatalogError> {
        let total: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(length(rgba)),0) FROM track_artwork",
            [],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(total).unwrap_or(u64::MAX))
    }

    /// Drop least-recently-used entries until the cache is within budget.
    fn evict_artwork_lru(&mut self) -> Result<(), CatalogError> {
        let budget = i64::try_from(self.artwork_budget).unwrap_or(i64::MAX);
        loop {
            let total: i64 = self.connection.query_row(
                "SELECT COALESCE(SUM(length(rgba)),0) FROM track_artwork",
                [],
                |row| row.get(0),
            )?;
            if total <= budget {
                break;
            }
            let deleted = self.connection.execute(
                "DELETE FROM track_artwork WHERE track_id = \
                 (SELECT track_id FROM track_artwork ORDER BY last_used ASC, track_id ASC LIMIT 1)",
                [],
            )?;
            if deleted == 0 {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Catalog;

    fn scratch(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "oxidj-artwork-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn db(path: &Path) -> PathBuf {
        path.join("catalog.sqlite")
    }

    fn remove_db(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    /// A 2x2 red/green/blue/white JPEG.
    fn jpeg_bytes() -> Vec<u8> {
        let mut image = image::RgbImage::new(2, 2);
        image.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        image.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        image.put_pixel(0, 1, image::Rgb([0, 0, 255]));
        image.put_pixel(1, 1, image::Rgb([255, 255, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        bytes
    }

    /// Attach `jpeg` as the front cover of an audio file with lofty.
    fn attach_cover(path: &Path, jpeg: &[u8]) {
        use lofty::config::WriteOptions;
        use lofty::file::TaggedFileExt;
        use lofty::picture::{MimeType, Picture, PictureType};
        use lofty::prelude::*;
        let mut tagged = lofty::probe::Probe::open(path)
            .unwrap()
            .guess_file_type()
            .unwrap()
            .read()
            .unwrap();
        if tagged.primary_tag_mut().is_none() {
            tagged.insert_tag(lofty::tag::Tag::new(tagged.primary_tag_type()));
        }
        let tag = tagged.primary_tag_mut().unwrap();
        tag.push_picture(
            Picture::unchecked(jpeg.to_vec())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::Jpeg)
                .build(),
        );
        tagged.save_to_path(path, WriteOptions::default()).unwrap();
    }

    // ---- minimal AIFF with an `ID3 ` chunk (built byte by byte) --------------

    fn ext80(rate: u32) -> [u8; 10] {
        let msb = 31 - rate.leading_zeros();
        let mut out = [0_u8; 10];
        out[0..2].copy_from_slice(&(16_383 + msb as u16).to_be_bytes());
        out[2..10].copy_from_slice(&((u64::from(rate)) << (63 - msb)).to_be_bytes());
        out
    }

    fn aiff_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// ID3v2.3 tag with one front-cover APIC frame.
    fn id3v2_cover(jpeg: &[u8]) -> Vec<u8> {
        let mut data = vec![0_u8]; // ISO-8859-1
        data.extend_from_slice(b"image/jpeg\0");
        data.push(3); // CoverFront
        data.push(0); // empty description
        data.extend_from_slice(jpeg);
        let mut frame = b"APIC".to_vec();
        frame.extend_from_slice(&(data.len() as u32).to_be_bytes());
        frame.extend_from_slice(&[0, 0]); // flags
        frame.extend_from_slice(&data);
        let size = frame.len();
        let mut tag = b"ID3".to_vec();
        tag.extend_from_slice(&[3, 0, 0]);
        tag.push(((size >> 21) & 0x7F) as u8);
        tag.push(((size >> 14) & 0x7F) as u8);
        tag.push(((size >> 7) & 0x7F) as u8);
        tag.push((size & 0x7F) as u8);
        tag.extend_from_slice(&frame);
        tag
    }

    fn aiff_with_cover(jpeg: &[u8]) -> Vec<u8> {
        let mut comm = Vec::new();
        comm.extend_from_slice(&1u16.to_be_bytes()); // mono
        comm.extend_from_slice(&8u32.to_be_bytes()); // frames
        comm.extend_from_slice(&16u16.to_be_bytes()); // bits
        comm.extend_from_slice(&ext80(44_100));
        let mut ssnd = Vec::new();
        ssnd.extend_from_slice(&0u32.to_be_bytes()); // offset
        ssnd.extend_from_slice(&0u32.to_be_bytes()); // block size
        ssnd.extend(std::iter::repeat_n(0_i16.to_be_bytes(), 8).flatten());
        let chunks = [
            aiff_chunk(b"COMM", &comm),
            aiff_chunk(b"SSND", &ssnd),
            aiff_chunk(b"ID3 ", &id3v2_cover(jpeg)),
        ];
        let mut body = b"AIFF".to_vec();
        for chunk in chunks {
            body.extend_from_slice(&chunk);
        }
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn embedded_artwork_is_extracted_from_flac_mp3_and_aiff() {
        let root = scratch("embedded");
        let jpeg = jpeg_bytes();
        for (name, bytes) in [
            (
                "tone.flac",
                include_bytes!("../tests/fixtures/tone.flac").as_slice(),
            ),
            (
                "tone.mp3",
                include_bytes!("../tests/fixtures/tone.mp3").as_slice(),
            ),
        ] {
            let path = root.join(name);
            std::fs::write(&path, bytes).unwrap();
            attach_cover(&path, &jpeg);
            let artwork = extract_for_track(&path).unwrap().unwrap();
            assert_eq!((artwork.width, artwork.height), (2, 2), "{name}");
            assert_eq!(artwork.rgba.len(), 16, "{name}");
        }
        let aiff = root.join("cover.aiff");
        std::fs::write(&aiff, aiff_with_cover(&jpeg)).unwrap();
        let artwork = extract_for_track(&aiff).unwrap().unwrap();
        assert_eq!((artwork.width, artwork.height), (2, 2));
        assert_eq!(artwork.rgba.len(), 16);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cover_file_fallback_is_used_when_no_embedded_artwork_exists() {
        let root = scratch("fallback");
        let track = root.join("tone.flac");
        std::fs::write(&track, include_bytes!("../tests/fixtures/tone.flac")).unwrap();
        assert!(extract_embedded(&track).unwrap().is_none());
        std::fs::write(root.join("cover.jpg"), jpeg_bytes()).unwrap();
        let artwork = extract_for_track(&track).unwrap().unwrap();
        assert_eq!((artwork.width, artwork.height), (2, 2));
        // `folder.jpg` is found when `cover.jpg` is absent.
        std::fs::remove_file(root.join("cover.jpg")).unwrap();
        std::fs::write(root.join("folder.jpg"), jpeg_bytes()).unwrap();
        let artwork = extract_for_track(&track).unwrap().unwrap();
        assert_eq!((artwork.width, artwork.height), (2, 2));
        assert!(find_cover_file(&track).is_some());
        // No artwork anywhere: `None`, not an error.
        std::fs::remove_file(root.join("folder.jpg")).unwrap();
        assert!(extract_for_track(&track).unwrap().is_none());
        assert!(find_cover_file(&track).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_round_trips_and_evicts_least_recently_used() {
        let root = scratch("cache");
        let database = db(&root);
        let mut catalog = Catalog::open(&database).unwrap();
        let track = catalog.insert_track("op", "/a.wav", [7; 32], 10).unwrap();
        assert_eq!(catalog.read_artwork(track.id).unwrap(), None);

        let first = Artwork::new(2, 2, vec![0; 16]).unwrap();
        catalog.store_artwork(track.id, [7; 32], &first).unwrap();
        assert_eq!(catalog.read_artwork(track.id).unwrap().unwrap(), first);
        assert_eq!(catalog.artwork_cache_bytes().unwrap(), 16);

        // A corrupt stored row fails closed on read (right encoding, wrong size).
        catalog
            .connection
            .execute("UPDATE track_artwork SET rgba=x'00000000'", [])
            .unwrap();
        assert!(catalog.read_artwork(track.id).is_err());
        catalog.store_artwork(track.id, [7; 32], &first).unwrap();

        // A second track, then a smaller budget: the older entry goes first.
        let second = catalog.insert_track("op2", "/b.wav", [9; 32], 10).unwrap();
        let other = Artwork::new(2, 2, vec![255; 16]).unwrap();
        catalog.store_artwork(second.id, [9; 32], &other).unwrap();
        assert_eq!(catalog.artwork_cache_bytes().unwrap(), 32);
        catalog.set_artwork_cache_budget(16).unwrap();
        assert_eq!(catalog.artwork_cache_bytes().unwrap(), 16);
        assert_eq!(catalog.read_artwork(track.id).unwrap(), None);
        assert_eq!(
            catalog.read_artwork(second.id).unwrap().as_ref(),
            Some(&other)
        );
        // Touch the survivor, then evict everything.
        assert_eq!(
            catalog.read_artwork(second.id).unwrap().as_ref(),
            Some(&other)
        );
        catalog.set_artwork_cache_budget(0).unwrap();
        assert_eq!(catalog.artwork_cache_bytes().unwrap(), 0);
        assert_eq!(catalog.read_artwork(second.id).unwrap(), None);

        // A digest mismatch or unknown track refuses the store.
        assert!(catalog.store_artwork(track.id, [8; 32], &first).is_err());
        let unknown = TrackId::new(99).unwrap();
        assert!(catalog.store_artwork(unknown, [7; 32], &first).is_err());
        drop(catalog);
        remove_db(&database);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_artwork_extracts_once_then_serves_the_cache() {
        let root = scratch("load");
        let track = root.join("tone.flac");
        std::fs::write(&track, include_bytes!("../tests/fixtures/tone.flac")).unwrap();
        attach_cover(&track, &jpeg_bytes());
        let database = db(&root);
        let mut catalog = Catalog::open(&database).unwrap();
        let digest = crate::file_sha256(&track).unwrap().0;
        let record = catalog
            .insert_track("op", track.to_str().unwrap(), digest, 10)
            .unwrap();
        let first = catalog.load_artwork(record.id).unwrap().unwrap();
        assert_eq!((first.width, first.height), (2, 2));
        assert_eq!(catalog.artwork_cache_bytes().unwrap(), 16);
        // The second load is a cache hit: removing the source artwork file
        // does not change the result.
        let cached = catalog.load_artwork(record.id).unwrap().unwrap();
        assert_eq!(cached, first);
        drop(catalog);
        remove_db(&database);
        let _ = std::fs::remove_dir_all(&root);
    }
}
