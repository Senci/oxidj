# v0.15 — Tempo fader, key lock and SYNC (M8 D3)

Date: 2026-11-05 · Commits: `bdeecf5` (engine: Signalsmith key lock behind `TimeStretch`, engine SYNC, RT allocation test), `fc46d0f` (UI: KEY/SYNC, refusals, live BPM, ADR-008 D3 addendum), this docs commit.

Real full-screen composited Bot Screen captures (X11, 1440x900) of the release build of `fc46d0f`. Synthetic data only: the catalog was written by the ignored helper test `seed_capture_catalog` (generated 4:00 WAV click/kick loops with synthetic grids), bind-mounted at `/mnt`. Audio output was deliberately unavailable (empty ALSA config), so the engine ran its silent paced renderer (status bar `NO AUDIO`); the transport, key lock and SYNC paths are the same. Inputs were real pointer clicks on the app (row, Load A/B, SYNC, KEY, PLAY).

| File | What it shows |
|---|---|
| `01-before-sync-unmatched.png` | **Before**: `a-120.wav` on A (120.00 BPM), `b-124.wav` on B (124.00 BPM), both faders at +0.0 %. Seam readout `Δ +124 ms · -4.00 BPM`; B's beat lines drift against A's. |
| `02-after-sync-b-keylock-matched.png` | **After**: KEY and SYNC pressed on B (both lit). B's fader moved to −3.2 %, B BPM readout 120.00, seam `Δ +0 ms · +0.00 BPM`; B's beat lines are warped onto A's. |
| `03-both-playing-synced-live-phase.png` | Both decks playing (A 0:04, B 0:02), still beat-aligned (the 2 s offset is a whole number of bars); the live seam readout shows `Δ -3 ms` while the decks run (UI-frame sampling, not the engine's sub-ms phase; the engine phase error is asserted < 1 ms in tests). |
| `04-sync-refused-no-beat-grid.png` | `c-nogrid.wav` loaded on B (mm:ss time grid, SYNC released); pressing SYNC shows the visible refusal `SYNC: deck B has no beat grid` in the strip. |

## Acceptance → evidence

1. Stretch admission — ADR-008 "D3 addendum" (evaluation table: CPU per deck, artefacts at ±8/±16 %, licence, dependency tree, `cargo deny`), `crates/oxidj-audio/examples/stretch_eval.rs`, `signalsmith-stretch =0.1.3` behind `crates/oxidj-audio/src/stretch.rs` `TimeStretch`; `cargo xtask layers` allow-lists it in `oxidj-audio` only.
2. Tempo fader (range from Settings), key lock on/off, % and BPM display, double-click reset — `deck_tests::key_lock_and_sync_buttons_match_tempo_and_phase`; `keylock_tests::key_lock_keeps_pitch_varispeed_does_not`, `key_lock_advances_at_speed_and_clicks_follow_the_tempo`.
3. SYNC tempo + phase, imported grid wins, refusals with exact reasons — `keylock_tests::sync_matches_tempo_and_phase_within_one_ms`, `deck_tests::grid_fit_is_sub_millisecond_and_detects_variable_grids`, `deck_tests::sync_refuses_with_exact_visible_reasons`.
4. Warped beat lines and live seam readout — `deck_tests::key_lock_and_sync_buttons_match_tempo_and_phase`.
5. Stretch ratio/length/no-clip — `keylock_tests::stretch_engine_ratio_length_and_no_clip_on_click_track`; RT safety — `crates/oxidj-audio/tests/rt_alloc.rs` (0 allocator events, 0 glibc heap changes over 1,500 `render` calls).

Gate: see the commit body of this docs commit.

## Limits

Listening on real material is the owner's check. NOT DEMONSTRATED: audible output in these captures (silent renderer), scratching, worst-case callback cost on an RT thread, tempo maps with tempo changes (refused visibly), non-Linux builds of the C++ dependency, native host display, 500k/60 fps qualification.

[← Timeline](../README.md)
