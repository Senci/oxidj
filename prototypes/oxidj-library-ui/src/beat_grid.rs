//! W6: bar-aware detail window from an imported (vendor-derived) beat grid.
//!
//! [`BeatGrid`] validates a library track's stored beats against the computed
//! track length; anything unusable (no beats, a single beat, unsorted or
//! duplicate times, no downbeat, everything past the end) yields `None` and the
//! caller keeps its fixed-length window. [`build_lines`] fills one reused
//! [`Mesh`] with thin vertical quads (faint beats, stronger bar lines) clamped
//! into the view rect, so a warmed steady state allocates nothing.

use crate::theme::ColorTokens;
use eframe::egui::{self, Color32, FontId, Mesh, Pos2, Rect, Shape, Ui};
use oxidj_library::LibraryBeat;
use std::sync::{Arc, Mutex};

/// Bars shown in the detail window.
pub const BARS: usize = 4;
/// Line widths in px (bar lines are stronger).
pub const BEAT_WIDTH: f32 = 1.0;
pub const BAR_WIDTH: f32 = 2.0;

/// A validated beat grid: strictly increasing beat times inside the track,
/// with at least two beats and one downbeat.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatGrid {
    /// `(time_ms, bar number)`; the number (1-based, counted from the first
    /// downbeat) is set only on downbeats.
    beats: Vec<(f64, Option<usize>)>,
    /// Per bar: index of its downbeat in `beats` and its nominal length in ms.
    bars: Vec<(usize, f64)>,
}

impl BeatGrid {
    /// `None` when the grid cannot place bar lines safely.
    pub fn new(beats: &[LibraryBeat], track_ms: f64) -> Option<Self> {
        if !(track_ms.is_finite() && track_ms > 0.0) {
            return None;
        }
        if beats.windows(2).any(|w| w[1].time_ms <= w[0].time_ms) {
            return None; // unsorted or duplicate: not a grid we can trust
        }
        let mut grid = Self {
            beats: Vec::new(),
            bars: Vec::new(),
        };
        for beat in beats {
            let t = f64::from(beat.time_ms);
            if t >= track_ms {
                break;
            }
            let bar = (beat.beat_in_bar == 1 && beat.tempo_hundredths_bpm > 0).then(|| {
                let bar_ms = 4.0 * 6_000_000.0 / f64::from(beat.tempo_hundredths_bpm);
                grid.bars.push((grid.beats.len(), bar_ms));
                grid.bars.len()
            });
            grid.beats.push((t, bar));
        }
        (grid.beats.len() >= 2 && !grid.bars.is_empty()).then_some(grid)
    }

    /// Number of bars (downbeats) in the track.
    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }

    /// Last bar index the detail window may start at (0-based).
    pub fn max_start_bar(&self) -> usize {
        self.bars.len().saturating_sub(BARS)
    }

    /// `[start, end)` in ms of the [`BARS`]-bar window starting at bar index
    /// `bar` (clamped): from that downbeat to the downbeat [`BARS`] bars later,
    /// or, near the end, the starting bar's nominal length times [`BARS`].
    pub fn window_ms(&self, bar: usize) -> (f64, f64) {
        let bar = bar.min(self.bars.len() - 1);
        let (index, bar_ms) = self.bars[bar];
        let start = self.beats[index].0;
        let end = match self.bars.get(bar + BARS) {
            Some(&(i, _)) => self.beats[i].0,
            None => start + bar_ms * BARS as f64,
        };
        (start, end)
    }

    /// Beats in `[start, end)`: `(time_ms, bar number if downbeat)`.
    pub fn beats_in(&self, (start, end): (f64, f64)) -> &[(f64, Option<usize>)] {
        let a = self.beats.partition_point(|b| b.0 < start);
        let b = self.beats.partition_point(|b| b.0 < end);
        &self.beats[a..b.max(a)]
    }
}

/// Line colours derived from theme tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridColours {
    pub beat: Color32,
    pub bar: Color32,
    pub label: Color32,
}

impl GridColours {
    pub fn from_tokens(t: ColorTokens) -> Self {
        let a = |c: Color32, alpha| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
        Self {
            beat: a(t.muted_text, 90),
            bar: a(t.text, 190),
            label: t.text,
        }
    }
}

/// x of `t_ms` in `rect` for `window` (unclamped).
pub fn x_of(t_ms: f64, (start, end): (f64, f64), rect: Rect) -> f32 {
    rect.left() + ((t_ms - start) / (end - start)) as f32 * rect.width()
}

/// Fill `mesh` (cleared first, buffers kept) with one full-height quad per
/// beat in `window`, clamped inside `rect`. Pure; allocation-free once warm.
pub fn build_lines(
    grid: &BeatGrid,
    window: (f64, f64),
    rect: Rect,
    colours: GridColours,
    mesh: &mut Mesh,
) {
    mesh.vertices.clear();
    mesh.indices.clear();
    let (start, end) = window;
    if !(rect.is_finite() && rect.width() >= 1.0 && end > start) {
        return;
    }
    for &(t, bar) in grid.beats_in(window) {
        let (width, colour) = match bar {
            Some(_) => (BAR_WIDTH, colours.bar),
            None => (BEAT_WIDTH, colours.beat),
        };
        let x = x_of(t, window, rect);
        let x0 = (x - width / 2.0).clamp(rect.left(), rect.right() - width);
        let quad = Rect::from_min_max(
            Pos2::new(x0, rect.top()),
            Pos2::new(x0 + width, rect.bottom()),
        );
        mesh.add_colored_rect(quad, colour);
    }
}

#[derive(Clone, Default)]
struct LineBuffer(Arc<Mutex<Arc<Mesh>>>);

/// Paint the grid over the detail view `rect` (keyed by `id`): reused line
/// mesh plus a "Bar N" label right of each visible bar line (skipped if it
/// would not fit inside `rect`).
pub fn paint(
    ui: &Ui,
    id: egui::Id,
    rect: Rect,
    grid: &BeatGrid,
    window: (f64, f64),
    tokens: ColorTokens,
) {
    let colours = GridColours::from_tokens(tokens);
    let buffer = ui.data_mut(|d| d.get_temp_mut_or_default::<LineBuffer>(id).clone());
    let mut slot = buffer.0.lock().unwrap_or_else(|e| e.into_inner());
    let mut mesh = Arc::get_mut(&mut slot)
        .map(std::mem::take)
        .unwrap_or_default();
    build_lines(grid, window, rect, colours, &mut mesh);
    match Arc::get_mut(&mut slot) {
        Some(dst) => *dst = mesh,
        None => *slot = Arc::new(mesh),
    }
    let painter = ui.painter_at(rect);
    if !slot.is_empty() {
        painter.add(Shape::Mesh(Arc::clone(&slot)));
    }
    for &(t, bar) in grid.beats_in(window) {
        let Some(n) = bar else { continue };
        let galley = painter.layout_no_wrap(
            format!("Bar {n}"),
            FontId::proportional(11.0),
            colours.label,
        );
        let pos = Pos2::new(x_of(t, window, rect) + BAR_WIDTH + 2.0, rect.top() + 2.0);
        if Rect::from_min_size(pos, galley.size()).max.x <= rect.right()
            && pos.y + galley.size().y <= rect.bottom()
        {
            painter.galley(pos, galley, colours.label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(time_ms: u32, beat_in_bar: u8) -> LibraryBeat {
        LibraryBeat {
            time_ms,
            tempo_hundredths_bpm: 12_000,
            beat_in_bar,
        }
    }

    /// 120 bpm: a 2-beat pickup at 0/500 ms, then bars from 1000 ms.
    fn grid_beats(count: u32) -> Vec<LibraryBeat> {
        (0..count)
            .map(|i| beat(i * 500, ((i + 2) % 4 + 1) as u8))
            .collect()
    }

    fn colours() -> GridColours {
        GridColours::from_tokens(crate::theme::ThemeMode::TokyoNight.tokens())
    }

    #[test]
    fn windows_are_four_bars_from_a_downbeat_and_numbered_from_the_first() {
        let grid = BeatGrid::new(&grid_beats(40), 60_000.0).unwrap();
        // Beats 2, 6, 10, ... are downbeats: bars at 1000 + 2000 k ms.
        assert_eq!(grid.bar_count(), 10);
        assert_eq!(grid.max_start_bar(), 6);
        assert_eq!(grid.window_ms(0), (1_000.0, 9_000.0));
        assert_eq!(grid.window_ms(2), (5_000.0, 13_000.0));
        // Past the last full window: extrapolated from the bar's tempo.
        assert_eq!(grid.window_ms(9), (19_000.0, 27_000.0));
        assert_eq!(grid.window_ms(99), grid.window_ms(9));
        let w = grid.beats_in(grid.window_ms(2));
        assert_eq!(w.len(), 16);
        assert_eq!(w[0], (5_000.0, Some(3)));
        assert_eq!(w[4], (7_000.0, Some(4)));
        assert_eq!(w[1].1, None);
    }

    #[test]
    fn degenerate_grids_are_rejected_without_panicking() {
        let track = 10_000.0;
        assert_eq!(BeatGrid::new(&[], track), None, "zero beats");
        assert_eq!(BeatGrid::new(&[beat(0, 1)], track), None, "single beat");
        let mut unsorted = grid_beats(12);
        unsorted.swap(3, 4);
        assert_eq!(BeatGrid::new(&unsorted, track), None, "unsorted");
        let mut duplicate = grid_beats(12);
        duplicate[5].time_ms = duplicate[4].time_ms;
        assert_eq!(BeatGrid::new(&duplicate, track), None, "duplicate");
        let beyond: Vec<_> = grid_beats(12)
            .into_iter()
            .map(|b| LibraryBeat {
                time_ms: b.time_ms + 20_000,
                ..b
            })
            .collect();
        assert_eq!(BeatGrid::new(&beyond, track), None, "beyond track end");
        let no_downbeat: Vec<_> = (0..8).map(|i| beat(i * 500, 2)).collect();
        assert_eq!(BeatGrid::new(&no_downbeat, track), None, "no downbeat");
        for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(BeatGrid::new(&grid_beats(12), t), None, "track {t}");
        }
        // Partly beyond the end: only beats inside the track are kept.
        let grid = BeatGrid::new(&grid_beats(40), 4_200.0).unwrap();
        assert_eq!(grid.bar_count(), 2);
        assert!(grid.beats_in((0.0, 1e9)).iter().all(|b| b.0 < 4_200.0));
    }

    #[test]
    fn lines_are_quads_at_beat_times_clamped_inside_the_rect() {
        let grid = BeatGrid::new(&grid_beats(40), 60_000.0).unwrap();
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(800.0, 96.0));
        let c = colours();
        let mut mesh = Mesh::default();
        for bar in [0, 3, 6, 9] {
            let window = grid.window_ms(bar);
            build_lines(&grid, window, rect, c, &mut mesh);
            let beats = grid.beats_in(window);
            assert_eq!(mesh.vertices.len(), beats.len() * 4, "bar {bar}");
            for (quad, &(t, n)) in mesh.vertices.chunks(4).zip(beats) {
                let bounds = quad.iter().fold(Rect::NOTHING, |r, v| {
                    r.union(Rect::from_min_max(v.pos, v.pos))
                });
                assert!(rect.contains_rect(bounds), "{bounds:?}");
                let colour = if n.is_some() { c.bar } else { c.beat };
                assert!(quad.iter().all(|v| v.color == colour));
                let x = rect.left() + ((t - window.0) / (window.1 - window.0)) as f32 * 800.0;
                assert!(
                    (bounds.center().x - x).abs() <= BAR_WIDTH / 2.0 + 1e-3,
                    "{t}"
                );
            }
        }
        // First line is the window's bar line, at the left edge.
        build_lines(&grid, grid.window_ms(0), rect, c, &mut mesh);
        assert_eq!(mesh.vertices[0].color, c.bar);
        assert_eq!(mesh.vertices[0].pos.x, rect.left());
        // Degenerate windows and rects paint nothing.
        for (window, r) in [
            ((5.0, 5.0), rect),
            ((9.0, 2.0), rect),
            (grid.window_ms(0), Rect::NOTHING),
            (
                grid.window_ms(0),
                Rect::from_min_size(Pos2::ZERO, egui::vec2(0.5, 9.0)),
            ),
        ] {
            build_lines(&grid, window, r, c, &mut mesh);
            assert!(mesh.is_empty());
        }
    }
}
