//! Deck album artwork: async extraction/cache and the deck-area display.
//!
//! When a track is loaded, [`Job`] extracts and decodes its artwork on a worker
//! thread (embedded cover art, else `cover.jpg`/`folder.jpg`, through
//! `oxidj_library::Catalog`'s LRU cache). The deck shows the vinyl placeholder
//! immediately and swaps in the decoded texture when the job finishes. The
//! image is drawn centered in the deck's waveform-detail half, aspect-preserved
//! and at most that half's height.

use crate::decks::DeckColours;
use crate::theme::ColorTokens;
use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, pos2};
use oxidj_library::{Artwork, Catalog, TrackId};
use oxidj_library::{ArtworkMemoryCache, DEFAULT_ARTWORK_CACHE_BYTES};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Shared across both decks. Only one decoder runs at a time. A quarter of
/// the setting is the decoded LRU; each deck image is <= 1/32 of the setting
/// (512px maximum), leaving room for queued pixels, CPU/GPU textures and decode
/// scratch. Decoder allocation limit is 1/4, capped at 64 MiB. These are pixel
/// budgets, not a claim about process RSS, driver or metadata-parser overhead.
#[derive(Clone)]
pub(crate) struct Cache {
    state: Arc<Mutex<CacheState>>,
    decoder: Arc<Mutex<()>>,
}
struct CacheState {
    pixels: ArtworkMemoryCache,
    budget: usize,
    generation: u64,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(CacheState {
                pixels: ArtworkMemoryCache::new(DEFAULT_ARTWORK_CACHE_BYTES as usize / 4),
                budget: DEFAULT_ARTWORK_CACHE_BYTES as usize,
                generation: 0,
            })),
            decoder: Arc::new(Mutex::new(())),
        }
    }
}
impl Cache {
    /// Never takes the decoder lock: a Settings change cannot stall a UI frame.
    pub(crate) fn configure(&self, bytes: usize) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.budget == bytes {
            return false;
        }
        state.budget = bytes;
        state.generation += 1;
        state.pixels = ArtworkMemoryCache::new(bytes / 4);
        true
    }

    fn load(
        &self,
        db: &std::path::Path,
        id: TrackId,
        cancelled: &AtomicBool,
    ) -> Result<Option<Artwork>, String> {
        let generation = self.state.lock().unwrap().generation;
        let _decoder = self.decoder.lock().unwrap();
        if cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let budget = {
            let mut state = self.state.lock().unwrap();
            if state.generation != generation {
                return Ok(None);
            }
            if let Some(art) = state.pixels.get(id) {
                return Ok(Some(art.clone()));
            }
            state.budget
        };
        let side = ((budget / 128) as f64).sqrt().floor().min(512.0) as u32;
        if side == 0 {
            return Ok(None);
        }
        let mut catalog = Catalog::open(db).map_err(|e| e.to_string())?;
        catalog
            .set_artwork_cache_budget(budget as u64)
            .map_err(|e| e.to_string())?;
        let art = catalog
            .load_display_artwork(id, (budget as u64 / 4).min(64 * 1024 * 1024), side)
            .map_err(|e| e.to_string())?;
        let mut state = self.state.lock().unwrap();
        if state.generation != generation || cancelled.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if let Some(art) = &art {
            state.pixels.insert(id, art.clone());
        }
        Ok(art)
    }

    #[cfg(test)]
    pub(crate) fn bytes(&self) -> usize {
        self.state.lock().unwrap().pixels.byte_len()
    }
}

/// A per-deck artwork load in progress.
pub(crate) struct Job {
    rx: Receiver<Result<Option<Artwork>, String>>,
    cancelled: Arc<AtomicBool>,
}

impl Job {
    /// Start extraction; `gate` (when present) blocks the worker
    /// until the test releases it, so the placeholder state is observable.
    pub(crate) fn start_after_gate(
        cache: Cache,
        db_path: PathBuf,
        track_id: u64,
        gate: Option<std::sync::Arc<std::sync::Barrier>>,
    ) -> Self {
        let (tx, rx) = channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let started = std::thread::Builder::new()
            .name("oxidj-artwork".into())
            .spawn(move || {
                if let Some(gate) = gate {
                    gate.wait();
                }
                let result = (|| {
                    let id = TrackId::new(track_id).map_err(|_| "invalid track id".to_owned())?;
                    cache.load(&db_path, id, &worker_cancelled)
                })();
                let _ = tx.send(result);
            });
        if started.is_err() {
            let (tx, rx) = channel();
            let _ = tx.send(Err("could not start the artwork worker".into()));
            return Self { rx, cancelled };
        }
        Self { rx, cancelled }
    }

    /// The result once the worker finished.
    pub(crate) fn poll(&self) -> Option<Result<Option<Artwork>, String>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("artwork worker stopped".into())),
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// One deck's artwork: a load in progress, or the decoded texture to draw.
#[derive(Default)]
pub(crate) struct DeckArt {
    job: Option<Job>,
    texture: Option<TextureHandle>,
    size: [u32; 2],
}

impl DeckArt {
    /// Start loading `track_id`'s artwork from the catalog at `db_path`.
    pub(crate) fn load(
        &mut self,
        track_id: u64,
        cache: Cache,
        db_path: PathBuf,
        gate: Option<std::sync::Arc<std::sync::Barrier>>,
    ) {
        self.job = Some(Job::start_after_gate(cache, db_path, track_id, gate));
        self.texture = None;
        self.size = [0, 0];
    }

    /// Collect a finished job and build the texture on the UI thread.
    pub(crate) fn poll(&mut self, ctx: &egui::Context, deck: usize) {
        let Some(job) = &self.job else { return };
        let Some(result) = job.poll() else { return };
        self.job = None;
        match result {
            Ok(Some(artwork)) => {
                self.size = [artwork.width, artwork.height];
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [artwork.width as usize, artwork.height as usize],
                    &artwork.rgba,
                );
                let name = format!("oxidj-deck-{deck}-artwork");
                self.texture = Some(ctx.load_texture(name, image, Default::default()));
            }
            Ok(None) | Err(_) => {
                self.texture = None;
                self.size = [0, 0];
            }
        }
    }

    pub(crate) fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    /// Decoded pixel dimensions (`[0, 0]` while placeholder/no artwork).
    pub(crate) fn size(&self) -> [u32; 2] {
        self.size
    }

    /// A artwork job is still running (placeholder is on screen).
    pub(crate) fn is_loading(&self) -> bool {
        self.job.is_some()
    }
}

/// The centered, aspect-preserving rect for a deck half: at most the half's
/// height and width, so the artwork never grows past the detail view.
pub(crate) fn artwork_rect(half: Rect, size: [u32; 2]) -> Rect {
    let max_w = half.width();
    let max_h = half.height();
    let (w, h) = if size[0] == 0 || size[1] == 0 {
        (max_h.min(max_w), max_h.min(max_w))
    } else {
        let scale = (max_h / size[1] as f32).min(max_w / size[0] as f32);
        (size[0] as f32 * scale, size[1] as f32 * scale)
    };
    Rect::from_center_size(half.center(), egui::vec2(w, h))
}

/// Paint the artwork, or the deck-colour vinyl-disc placeholder when none is
/// loaded (including while the async job is still running).
pub(crate) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    art: &DeckArt,
    colours: DeckColours,
    tokens: &ColorTokens,
) {
    if let Some(texture) = art.texture() {
        let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
        painter.image(texture.id(), rect, uv, Color32::WHITE);
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, colours.outer),
            egui::StrokeKind::Inside,
        );
        return;
    }
    paint_vinyl(painter, rect, colours, tokens);
}

/// A vinyl disc in the deck's accent colour: outer disc, grooves, label, hole.
fn paint_vinyl(painter: &egui::Painter, rect: Rect, colours: DeckColours, tokens: &ColorTokens) {
    let centre = rect.center();
    let radius = rect.height().min(rect.width()) / 2.0;
    if radius <= 1.0 {
        return;
    }
    painter.circle_filled(centre, radius, colours.outer);
    for groove in [0.82, 0.64, 0.46] {
        painter.circle_stroke(
            centre,
            radius * groove,
            egui::Stroke::new(1.0_f32, alpha(colours.outer, 130)),
        );
    }
    painter.circle_filled(centre, radius * 0.3, colours.wash);
    painter.circle_stroke(
        centre,
        radius * 0.3,
        egui::Stroke::new(1.0_f32, alpha(colours.outer, 170)),
    );
    painter.circle_filled(centre, (radius * 0.08).max(1.5), tokens.background);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_async_jobs_share_lru_and_evict_the_oldest_not_the_touched_track() {
        let root = std::env::temp_dir().join(format!("artwork-lru-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("catalog.sqlite");
        let mut catalog = Catalog::open(&db).unwrap();
        let ids: Vec<_> = (0..9)
            .map(|n| {
                let track = catalog
                    .insert_track(
                        &format!("art-{n}"),
                        &format!("/not-present-{n}.flac"),
                        [n; 32],
                        1,
                    )
                    .unwrap();
                catalog
                    .store_artwork(track.id, [n; 32], &Artwork::new(2, 2, vec![n; 16]).unwrap())
                    .unwrap();
                track.id
            })
            .collect();
        let cache = Cache::default();
        cache.configure(512); // 128-byte LRU, plus decoder/texture allowances.
        let load = |id: TrackId| {
            let job = Job::start_after_gate(cache.clone(), db.clone(), id.as_u64(), None);
            let start = std::time::Instant::now();
            loop {
                if let Some(result) = job.poll() {
                    break result.unwrap().unwrap();
                }
                assert!(start.elapsed().as_secs() < 10);
                std::thread::yield_now();
            }
        };
        for id in &ids[..8] {
            assert_eq!(load(*id).byte_len(), 16);
        }
        assert_eq!(cache.bytes(), 128);
        load(ids[0]); // promote the first entry
        load(ids[8]); // evict the second
        assert_eq!(cache.bytes(), 128);
        {
            let mut state = cache.state.lock().unwrap();
            assert!(state.pixels.get(ids[0]).is_some());
            assert!(state.pixels.get(ids[1]).is_none());
            assert!(state.pixels.get(ids[8]).is_some());
        }
        // Remove disk entries and source files are absent: only the shared RAM
        // hit can succeed, proving deck jobs really consume the LRU.
        catalog.set_artwork_cache_budget(0).unwrap();
        assert_eq!(load(ids[0]).byte_len(), 16);
        cache.configure(0);
        assert_eq!(cache.bytes(), 0);
        assert!(
            cache
                .load(&db, ids[0], &AtomicBool::new(false))
                .unwrap()
                .is_none()
        );
        drop(catalog);
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// `color` at `alpha` (0..=255), unmultiplied.
fn alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}
