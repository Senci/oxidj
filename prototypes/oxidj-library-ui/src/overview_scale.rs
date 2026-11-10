//! Shared playback-time scale for the two deck overviews: equal pixels are
//! equal *playback* seconds (track seconds / effective rate), and both
//! needles sit at one x. Pure geometry, no egui state.

/// What the scale needs to know about one loaded deck.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DeckSpan {
    /// Playhead, track milliseconds.
    pub(crate) position_ms: f64,
    /// Track length, track milliseconds (> 0).
    pub(crate) duration_ms: f64,
    /// Effective speed multiplier now playing (tempo, bend, SYNC).
    pub(crate) rate: f64,
}

impl DeckSpan {
    fn rate(&self) -> f64 {
        if self.rate.is_finite() && self.rate > 1e-3 {
            self.rate
        } else {
            1.0
        }
    }
    fn position_ms(&self) -> f64 {
        self.position_ms.clamp(0.0, self.duration_ms)
    }
    /// Playback seconds already played / still to play.
    fn elapsed_s(&self) -> f64 {
        self.position_ms() / 1000.0 / self.rate()
    }
    fn remaining_s(&self) -> f64 {
        (self.duration_ms - self.position_ms()) / 1000.0 / self.rate()
    }
}

/// One scale for the whole strip: `px_per_s` pixels per playback second and
/// the shared needle at `needle_x`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SharedScale {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) needle_x: f32,
    pub(crate) px_per_s: f64,
}

impl SharedScale {
    /// `None` when no deck is loaded (or the strip has no width).
    pub(crate) fn new(decks: &[Option<DeckSpan>], left: f32, right: f32) -> Option<Self> {
        let loaded = || decks.iter().flatten().filter(|d| d.duration_ms > 0.0);
        let l = loaded().map(|d| d.elapsed_s()).fold(0.0, f64::max);
        let r = loaded().map(|d| d.remaining_s()).fold(0.0, f64::max);
        let width = f64::from(right - left);
        if loaded().next().is_none() || width.is_nan() || width < 1.0 || l + r <= 0.0 {
            return None;
        }
        let px_per_s = width / (l + r);
        Some(Self {
            left,
            right,
            needle_x: left + (l * px_per_s) as f32,
            px_per_s,
        })
    }

    /// Pixels per track millisecond for `deck`.
    fn px_per_ms(&self, deck: &DeckSpan) -> f64 {
        self.px_per_s / 1000.0 / deck.rate()
    }

    /// x of track time `ms` on `deck`'s strip (may lie outside the strip).
    /// The painters use [`Self::window`]; this is its point form.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn x_of(&self, deck: &DeckSpan, ms: f64) -> f32 {
        self.needle_x + ((ms - deck.position_ms()) * self.px_per_ms(deck)) as f32
    }

    /// Track time under `x`, clamped to the track.
    pub(crate) fn ms_at(&self, deck: &DeckSpan, x: f32) -> f64 {
        (deck.position_ms() + f64::from(x - self.needle_x) / self.px_per_ms(deck))
            .clamp(0.0, deck.duration_ms)
    }

    /// Track-time window `(t0, t1)` the whole strip shows for `deck`; it may
    /// reach before 0 or past the end (the track just stops there).
    pub(crate) fn window(&self, deck: &DeckSpan) -> (f64, f64) {
        let k = self.px_per_ms(deck);
        (
            deck.position_ms() - f64::from(self.needle_x - self.left) / k,
            deck.position_ms() + f64::from(self.right - self.needle_x) / k,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(position_ms: f64, duration_ms: f64, rate: f64) -> DeckSpan {
        DeckSpan {
            position_ms,
            duration_ms,
            rate,
        }
    }

    #[test]
    fn maps_time_and_x_invertibly_and_clamps() {
        let d = span(30_000.0, 120_000.0, 1.0);
        let s = SharedScale::new(&[Some(d)], 100.0, 1100.0).unwrap();
        for ms in [0.0, 1234.5, 30_000.0, 99_999.0, 120_000.0] {
            let back = s.ms_at(&d, s.x_of(&d, ms));
            assert!((back - ms).abs() < 0.5, "{ms} -> {back}");
        }
        assert_eq!(s.ms_at(&d, -1.0e6), 0.0);
        assert_eq!(s.ms_at(&d, 1.0e6), 120_000.0);
    }

    #[test]
    fn one_deck_fills_the_strip_with_the_needle_at_its_position() {
        // 30 s played, 90 s left at rate 1: 1000 px / 120 s.
        let d = span(30_000.0, 120_000.0, 1.0);
        let s = SharedScale::new(&[Some(d), None], 100.0, 1100.0).unwrap();
        assert!((s.px_per_s - 1000.0 / 120.0).abs() < 1e-9);
        assert!((f64::from(s.needle_x) - (100.0 + 250.0)).abs() < 1e-3);
        assert!((f64::from(s.x_of(&d, 0.0)) - 100.0).abs() < 1e-3);
        assert!((f64::from(s.x_of(&d, 120_000.0)) - 1100.0).abs() < 1e-3);
    }

    #[test]
    fn none_loaded_has_no_scale() {
        assert!(SharedScale::new(&[None, None], 0.0, 500.0).is_none());
        assert!(SharedScale::new(&[Some(span(0.0, 0.0, 1.0))], 0.0, 500.0).is_none());
        assert!(SharedScale::new(&[Some(span(0.0, 1000.0, 1.0))], 0.0, 0.5).is_none());
    }

    #[test]
    fn two_decks_share_one_scale_and_needle_and_stay_inside_the_strip() {
        // A: 100 s track at 1.0, at 40 s. B: 60 s track at 1.25, at 10 s.
        let a = span(40_000.0, 100_000.0, 1.0);
        let b = span(10_000.0, 60_000.0, 1.25);
        let s = SharedScale::new(&[Some(a), Some(b)], 0.0, 960.0).unwrap();
        // Oracle in seconds: elapsed A 40, B 8; remaining A 60, B 40.
        let (l, r) = (40.0_f64, 60.0_f64);
        assert!((s.px_per_s - 960.0 / (l + r)).abs() < 1e-9);
        assert!((f64::from(s.needle_x) - 960.0 * l / (l + r)).abs() < 1e-3);
        // 10 playback seconds ahead is the same x distance on both decks.
        let ahead = |d: &DeckSpan| s.x_of(d, d.position_ms + 10_000.0 * d.rate) - s.needle_x;
        assert!((ahead(&a) - ahead(&b)).abs() < 1e-3);
        assert!((f64::from(ahead(&a)) - 96.0).abs() < 1e-3);
        // Both tracks lie fully inside the strip.
        for d in [&a, &b] {
            assert!(s.x_of(d, 0.0) >= -1e-3, "start");
            assert!(s.x_of(d, d.duration_ms) <= 960.0 + 1e-3, "end");
        }
        // B ends earlier: 40 playback seconds ahead of the needle.
        assert!((f64::from(s.x_of(&b, 60_000.0) - s.needle_x) - 384.0).abs() < 1e-3);
    }

    #[test]
    fn a_rate_change_moves_a_marker_to_the_expected_x() {
        let mut d = span(0.0, 100_000.0, 1.0);
        let at = |d: &DeckSpan| {
            let s = SharedScale::new(&[Some(*d)], 0.0, 1000.0).unwrap();
            f64::from(s.x_of(d, 50_000.0))
        };
        // Rate 1: 100 s over 1000 px, marker at 50 s -> 500 px.
        assert!((at(&d) - 500.0).abs() < 1e-3);
        // Alone, the deck always fills the strip whatever its rate.
        d.rate = 1.25;
        assert!((at(&d) - 500.0).abs() < 1e-3);
        // Against a second deck (80 s at rate 1) the 1.25x deck is 80 s long
        // in playback time: the marker at 50 track-s = 40 playback-s.
        let other = span(0.0, 80_000.0, 1.0);
        let s = SharedScale::new(&[Some(d), Some(other)], 0.0, 1000.0).unwrap();
        assert!((f64::from(s.x_of(&d, 50_000.0)) - 500.0).abs() < 1e-3);
        d.rate = 1.0;
        let s = SharedScale::new(&[Some(d), Some(other)], 0.0, 1000.0).unwrap();
        // 100 s deck now fills the strip: 50 s -> 500 px; at 1.25 it was
        // 80 s long: 40 playback-s of 80 -> 500 px; the 2.0x case:
        assert!((f64::from(s.x_of(&d, 50_000.0)) - 500.0).abs() < 1e-3);
        d.rate = 2.0; // 50 playback-s long, other 80 s: scale 1000/80
        let s = SharedScale::new(&[Some(d), Some(other)], 0.0, 1000.0).unwrap();
        assert!((f64::from(s.x_of(&d, 50_000.0)) - 25.0 * 12.5).abs() < 1e-3);
    }

    #[test]
    fn window_matches_x_of() {
        let d = span(25_000.0, 90_000.0, 0.8);
        let o = span(5_000.0, 200_000.0, 1.0);
        let s = SharedScale::new(&[Some(d), Some(o)], 50.0, 850.0).unwrap();
        let (t0, t1) = s.window(&d);
        assert!((f64::from(s.x_of(&d, t0)) - 50.0).abs() < 1e-2);
        assert!((f64::from(s.x_of(&d, t1)) - 850.0).abs() < 1e-2);
    }
}
