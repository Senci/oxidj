# v0.16 — Mixer (M8 D4)

Date: 2026-11-05 · Commit: the D4 feature commit (see `git log --grep "M8 D4"`).

Real full-screen composited Bot Screen capture (X11, 1440x900) of the release build of the D4 working tree. Synthetic data only: the catalog was written by the ignored helper test `seed_capture_catalog` (generated 4:00 WAV click/kick loops with synthetic grids), bind-mounted at `/mnt`. Audio output was deliberately unavailable (empty ALSA config), so the engine ran its silent paced renderer (status bar `NO AUDIO`); the mixer and meters run in the same renderer. Inputs were real pointer clicks (Decks, rows, Load A/B, Mixer toggle, PLAY A/B).

| File | What it shows |
|---|---|
| `01-mixer-both-playing-centre.png` | Mixer section shown (between the control strip and the overviews): channel A left (TRIM, HI, MID, LOW; FILTER, VOL, meter, assign A/THRU/B), master volume + master meter and the crossfader with its own icon toggle in the centre, channel B right. Both decks playing at the default centre/unity; channel meters near full scale, master meter lit with its clip box (two 0.9-peak tracks summed). |

## Acceptance → evidence

1. Trim, 3-band isolator EQ (kill to +6 dB, LR4 in-house), colour filter, channel fader, master — `mixer_tests::eq_at_0_db_is_flat_unity_magnitude`, `each_kill_removes_its_band_and_keeps_the_others`, `trim_fader_and_filter_sweep`; ADR-008 "D4 addendum".
2. Crossfader Smooth/Sharp/Cut (Settings), assign A/THRU/B, centre = both unity — `mixer_tests::crossfader_curve_invariants`, `mixer_routes_faders_crossfader_and_meters`; UI `deck_tests::mixer_controls_drive_the_engine_and_meters_through_the_ui` (curve via Settings reaches the engine params).
3. Peak + clip meters per channel and master from engine atomics, tokens-only drawing, no allocation/lock in the callback — `mixer_tests::mixer_routes_faders_crossfader_and_meters`, `deck_tests::mixer_controls_drive_the_engine_and_meters_through_the_ui` (painted meter fills ⊂ theme tokens), `tests/rt_alloc.rs`.
4. Optional and collapsible (mixer hidden by default; crossfader icon toggle, Ctrl+Shift+5), persisted; hidden = bit-identical bypass; keyboard reachable — `mixer_tests::hidden_mixer_is_bit_identical_to_no_mixer`, `deck_tests::hidden_mixer_is_bit_identical_whatever_the_knobs_say`, `deck_tests::mixer_is_fully_keyboard_reachable_and_operable`, `settings::tests::every_field_round_trips_through_storage`.
5. DSP properties (no NaN/denormal blow-up) — `mixer_tests::no_nan_or_denormals_under_extreme_settings_silence_and_bad_input`.

## Limits

Listening is the owner's check. NOT DEMONSTRATED: audible output in this capture (silent renderer), a crossfader/EQ position change on the live Bot Screen (live drags did not register; covered by the UI tests), worst-case callback cost on an RT thread, PFL/headphone cue (out of scope), native host display, 500k/60 fps qualification.

[← Timeline](../README.md)
