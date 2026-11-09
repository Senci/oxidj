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
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// A per-deck artwork load in progress.
pub(crate) struct Job {
    rx: Receiver<Result<Option<Artwork>, String>>,
}

impl Job {
    /// Start extraction; `gate` (when present) blocks the worker
    /// until the test releases it, so the placeholder state is observable.
    pub(crate) fn start_after_gate(
        db_path: PathBuf,
        track_id: u64,
        gate: Option<std::sync::Arc<std::sync::Barrier>>,
    ) -> Self {
        let (tx, rx) = channel();
        let started = std::thread::Builder::new()
            .name("oxidj-artwork".into())
            .spawn(move || {
                if let Some(gate) = gate {
                    gate.wait();
                }
                let result = (|| {
                    let mut catalog = Catalog::open(&db_path).map_err(|error| error.to_string())?;
                    let id = TrackId::new(track_id).map_err(|_| "invalid track id".to_owned())?;
                    catalog.load_artwork(id).map_err(|error| error.to_string())
                })();
                let _ = tx.send(result);
            });
        if started.is_err() {
            let (tx, rx) = channel();
            let _ = tx.send(Err("could not start the artwork worker".into()));
            return Self { rx };
        }
        Self { rx }
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
        db_path: PathBuf,
        gate: Option<std::sync::Arc<std::sync::Barrier>>,
    ) {
        self.job = Some(Job::start_after_gate(db_path, track_id, gate));
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

/// `color` at `alpha` (0..=255), unmultiplied.
fn alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}
