# v0.13 — Computed BPM and beat grid in the library UI (B2c)

Date: 2026-11-04 · Commits: `9a9b96c` (UI + tests), this docs commit. Builds on B2a (`82dcee1`, `2390310`: taktus 0.1.0 + streaming `TempoBuilder`) and B2b (`da0220b`: per-track grids in the catalog, `catalog analyze-tempo`).

Real full-screen composited Bot Screen captures (X11, 1440x900) of the release build of `9a9b96c`. Synthetic audio only, generated for these captures and bind-mounted at `/mnt` (no private data). Each capture is a fresh instance: one click on the track, one click on the Details toggle, one click on empty table space. Catalogs were built with the release CLI: `catalog import`, `catalog analyze-waveform`, `catalog analyze-tempo`.

| File | Library | Track | Computed |
|---|---|---|---|
| `01-ventoy-style-computed-grid.png` | Ventoy-style layout (`/mnt/ventoy/oxidj-patriot-backup/Contents/<artist>/…`), five synthetic house-style loops (kick, offbeat hat, bass), no imported library | `Warm Pulse 128.wav` (made at 128 BPM) | 127.95 BPM, confidence 0.84 |
| `02-click-track-computed-grid.png` | One synthetic click track | `Click 120 BPM.wav` (kick+click every 500 ms from 250 ms) | 120.00 BPM, confidence 1.00 |

What the captures show: with no imported (vendor) grid, the detail strip uses the computed grid like the W6 imported-grid view: a four-bar window, faint beat lines and stronger bar lines with "Bar N" labels. Captions: `beat grid: computed (taktus 0.1.0), NNN.NN BPM` and `beats counted from first detected beat`. The facts list gains `BPM (computed)`. On the click track every click sits on a beat line.

## Behaviour

- **Source order:** an imported library grid (W6/W10 location match) always wins. The computed grid is used only when no usable imported grid exists. It must belong to the current file (stored source SHA-256 equals the track's), otherwise a warning badge is shown instead.
- **Bars:** the computed grid has no downbeat, so bars are counted in fours from the first detected beat (bar lines every 4th beat). The `beats counted from first detected beat` caption says so.
- **Facts row:** `BPM (computed)` shows the value with two decimals. A muted `low` badge appears below confidence 0.5. Confidence is taktus `sure` clamped to 0..1, not a calibrated probability. No row without a computed grid. The 8 s window without any grid is unchanged.
- **Unchanged:** track table (no new column), imported-grid path, waveform renderer and allocation test. The details key column grew from 88 to 104 px so the longer key never widens the panel past its 320 px minimum.

## Accuracy and maturity

A computed **suggestion**, not Rekordbox parity. B1 evaluation on 90 real tracks with Rekordbox grids: BPM within 2% of Rekordbox on 91% of tracks, F70 0.76. Octave errors happen. In this set the synthetic 87 BPM loop was detected as 173.98 (double tempo; not captured). No tempo changes, no downbeat, no grid editing, no XML export of computed grids.

## Tests

Harness tests over the production `LibraryApp::ui` in `prototypes/oxidj-library-ui/src/track_waveform.rs`:

- `computed_grid_is_the_fallback_with_its_own_caption_and_bpm_fact` covers three cases. A computed-only click track gets the four-bar window from its stored beats, line x positions at the stored beat times, a bar line every 4th beat, painted quads inside the view and clip rect, `Bar N` labels, both computed captions and the BPM row in the facts rhythm. The `low` badge appears only below the threshold and is outlined in the muted token. When imported and computed grids both exist, the import wins: imported window, lines and caption, no computed caption. With no grid at all the 8 s window is kept, with no BPM row and no grid captions.
- `w9_captions_fit_slider_rows_align_and_strip_height_follows_details_height` now also runs a computed-grid track at 750 and 1440 px in both themes. It checks that captions sit inside the panel and its clip rect, that slider rows stay aligned with at least 24 px targets, that the strip height stays bounded and the content inside the panel, and that the panel stays at its minimum width.
- `w11_row_fills_keep_text_contrast` checks text on the muted badge tint against both fills (at least 4.5:1).

Gate: `cargo xtask ci-full` exit 0 (`ci=ok test=ok ui-test=ok deny=ok`).

NOT DEMONSTRATED: native display, screen reader, platform qualification, accuracy beyond the B1 sample, Rekordbox behaviour with computed grids.

[← Timeline](../README.md)
