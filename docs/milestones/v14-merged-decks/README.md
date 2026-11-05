# v0.14 — Merged two-deck waveform view and control strip (M8 D1, no audio)

Date: 2026-11-05 · Commits: `9b9acbd` (feature + tests), `ef5c601` (strip buttons painted in their exact rect, deck-tinted tempo fader), this docs commit. Builds on D0 (`771f631`, `c66e2d4`: Settings window, Ctrl+,).

Real full-screen composited Bot Screen captures (X11, 1440x900) of the release build of `ef5c601`. Synthetic data only: the catalog was written by the ignored helper test `seed_capture_catalog` (four generated 4:00 WAV click/kick loops with synthetic grids, hot cues and a loop), bind-mounted at `/mnt`. Inputs were real pointer clicks on the app (Decks toggle, row, Load A/B, PLAY, Detail chevron).

| File | What it shows |
|---|---|
| `01-before-library-only.png` | **Before**: library only, deck area hidden (the v0.13 shell had no deck view). |
| `02-after-both-decks-loaded.png` | **After**: `a-120.wav` on A (120 BPM) and `b-124.wav` on B (123.97 BPM). One merged detail view: A warm (amber/orange, white-hot transients) above the seam, B cool (blue/cyan) below, one centre playhead, coloured rails, faint A/B watermarks, coloured bar/beat lines, numbered hot cues on both halves, seam readout `Δ +124 ms · -3.97 BPM`. Club-standard strip (A left, B right, CUE/PLAY outermost, 8 pads, IN/OUT/½/×2, tempo fader, SYNC/KEY disabled until D3). Two separate overviews at the bottom (A first, B second) with view-window boxes and cues. Table badges A/B. Status bar `NO AUDIO (D2)`. |
| `03-after-playing-scrolls-under-playhead.png` | Both decks after PLAY (silent wall-clock transport): content has scrolled under the fixed centre playhead, PAUSE lit, overview played part dimmed, seam phase readout updated. |
| `04-grid-fallback-no-grid-and-analysing.png` | `c-nogrid.wav` on A (waveform, no beat grid → mm:ss time grid) and `d-pending.wav` on B (no waveform yet → time grid with the quiet `analysing · waveform not ready` state in the detail half and the overview). |
| `05-collapsed-detail-library-grows.png` | Collapsed variant: Detail chevron closed; strip and overviews remain, the library takes the freed space. |

## Acceptance → tests

Production-path egui harness tests over `LibraryApp::ui` in `prototypes/oxidj-library-ui/src/deck_tests.rs`:

1. Merged detail, playhead, zoom, auto-gain, seam readout — `merged_detail_halves_share_one_seam_playhead_and_paint_inside_the_clip` (painted bounds inside the clip at min/mid/max), `zoom_by_keys_and_wheel_uses_bars_or_seconds_and_drag_scrubs`, `auto_gain_lifts_a_quiet_master_and_follows_the_setting`.
2. Deck identity and palettes (Warm/Cool, Classic preset, contrast in both themes) — `deck_palettes_are_warm_and_cool_with_contrast_in_both_themes`.
3. Time grid fallback, waveform replaces it in place, imported grid wins over computed — `time_grid_when_no_waveform_or_grid_and_the_waveform_replaces_it_in_place`.
4. Separate overviews (two strips, order, per-deck seek), cues and loops on detail and overview — `overviews_are_two_separate_strips_at_the_bottom_and_seek_their_own_deck`, `imported_cues_and_loops_draw_on_detail_and_overview`.
5. Control strip and loading (buttons, A/B keys, context menu, table badges) — `load_by_keys_button_and_context_menu_with_table_badges`, `pads_cue_play_tab_and_loops_drive_the_silent_transport`; strip reachability with long names — `shell_tests::decks_bar_truncates_long_names_and_keeps_play_cue_reachable`.
6. Every collapse combination, chevron/keyboard toggles, persistence, Ctrl+D — `every_section_combination_collapses_cleanly_toggles_and_persists`.
7. Silent transport, NO AUDIO status, Space/Q/P/1-8/Tab — `pads_cue_play_tab_and_loops_drive_the_silent_transport`, `load_by_keys_button_and_context_menu_with_table_badges`.

Gate: see the commit body of this docs commit (`cargo xtask ci-full`).

## Limits

No audio (D2), no sync/key-lock (D3), mixer is a hidden placeholder (D4). NOT DEMONSTRATED: native host display, screen reader, real-library visuals, 500k/60 fps qualification.

[← Timeline](../README.md)
