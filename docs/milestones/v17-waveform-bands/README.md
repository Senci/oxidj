# v0.17 — 3-band deck waveforms (W12)

Date: 2026-11-05 · Commit: the W12 commits (see `git log --grep "W12"`).

Real full-screen composited Bot Screen captures (X11, 1440x900) of the release build of the W12 working tree. Synthetic data only: the catalog from the ignored helper test `seed_capture_catalog` (generated 4:00 WAV click/kick loops with synthetic grids and cues), bind-mounted at `/mnt`. Audio output was deliberately unavailable (silent paced renderer, status bar `NO AUDIO`). Inputs were real pointer clicks (Decks, rows, Load A/B, PLAY A/B).

| File | What it shows |
|---|---|
| `01-before-v16-monochrome.png` | Before (copy of the v0.16 capture): detail halves near-monochrome (orange only / blue only), overviews flat cream/white bars, tiny cue marks. |
| `02-after-three-bands-loaded.png` | After, both decks loaded at 0:00: detail A in warm low/mid/high (red-orange, amber, pale-yellow core), B in cool (deep blue, cyan, ice-white core); overviews in the same three bands with auto-gain, 16-bar phrase ticks, cue flags 1-3 with numbers. |
| `03-after-both-playing-played-dimmed.png` | Both decks playing (~0:25): played part of each overview dimmed, cue flags drawn over the dimming, view window bracket. |
| `04-after-sync-keylock-both-playing.png` | Added 2026-11-06 for the site (release build of `d1e6c21`, same synthetic catalog, mixer shown): KEY and SYNC on deck B (tempo -3.2 %, seam readout 0 ms / +0.00 BPM), both decks playing (~0:15), live meters. Inputs were real pointer clicks (Decks, Mixer, rows, Load A/B, KEY B, SYNC B, PLAY A/B). |

## Acceptance → evidence

1. Detail: three opaque nested band layers per deck palette (Settings tokens, Classic preset kept) — `deck_tests::deck_band_layers_keep_their_own_colour_and_nest_at_min_mid_max` (each layer its own colour, low ⊇ mid ⊇ high, ring = weighted band share, painted bounds inside the clip at min/mid/max and a hat-only column), `deck_band_palette_contrast_in_both_themes`.
2. Overviews: same bands, time-compressed mean envelope with per-track auto-gain, played part dimmed, cue/loop markers >= 6 px with hover names, phrase ticks every 16 bars — `overview_mean_envelope_auto_gains_a_quiet_track_and_shows_breaks` (quiet track fills >= 60 %), `phrase_ticks_every_16_bars_inside_the_overview`, `production_decks_paint_three_bands_and_readable_overview_cues_with_hover_names`.
3. Real-data check: done privately on two real library tracks on the Bot Screen (3 iterations); those captures stay in the private scratch dir and are not in the repo. Finding: the stored 3-band peaks suffice for the overview (no re-analysis); real masters are mostly mid band with the 200 Hz / 2.5 kHz split, so the ring shares use display weights (`decks::SHARE_WEIGHTS`, low lifted) to keep the low ring visible.
4. Existing deck tests stay green (repository gate in the commit body).

## Limits

NOT DEMONSTRATED: Day-theme capture (covered by the contrast test), native host display, 500k/60 fps qualification. Waveform shape is a display choice, not Rekordbox parity.

[← Timeline](../README.md)
