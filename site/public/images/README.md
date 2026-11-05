# Public image provenance

All screenshots are crops of reviewed, inspected Bot Screen captures (X11, 1440x900,
release builds of the provisional `prototypes/oxidj-library-ui`, Tokyo Night). Audio is
synthetic only, generated for the captures and bind-mounted at `/mnt`; no personal
music collection, home directory, username or private library name is visible. The
pixels were inspected again before publication (2026-11-06).

| Source capture (under `docs/milestones/`)                     | Build                              | Catalog                                                                                                                   |
| ------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `v13-computed-beat-grid/01-ventoy-style-computed-grid.png`    | release `9a9b96c`                  | five synthetic loops in a Ventoy-style layout, no imported library                                                        |
| `v13-computed-beat-grid/02-click-track-computed-grid.png`     | release `9a9b96c`                  | one synthetic 120 BPM click track                                                                                         |
| `v14-merged-decks/04-grid-fallback-no-grid-and-analysing.png` | release `ef5c601` (M8 D1)          | four generated 4:00 WAV click/kick loops with synthetic grids, cues and a loop; `c-nogrid.wav` on A, `d-pending.wav` on B |
| `v14-merged-decks/05-collapsed-detail-library-grows.png`      | release `ef5c601` (M8 D1)          | same set, detail waveform collapsed                                                                                       |
| `v15-tempo-keylock-sync/04-sync-refused-no-beat-grid.png`     | release `fc46d0f` (M8 D3)          | same set; SYNC refusal on deck B                                                                                          |
| `v17-waveform-bands/01-before-v16-monochrome.png`             | release of the D4 tree (`8b6087b`) | same set; copy of the v16 mixer capture, both decks playing (the "before")                                                |
| `v17-waveform-bands/03-after-both-playing-played-dimmed.png`  | release of the W12 tree            | same set; 3-band waveforms, mixer shown, both decks playing                                                               |
| `v17-waveform-bands/04-after-sync-keylock-both-playing.png`   | release `d1e6c21` (W12)            | same set; KEY and SYNC on deck B, mixer shown, both decks playing                                                         |

`scripts/crop-screenshots.py` regenerates every file below from those sources and
deletes superseded crops (`PYTHONDONTWRITEBYTECODE=1 uv run --no-project --with pillow
python scripts/crop-screenshots.py` from `site/`). Full-width crops also get a `-720`
half-width lossy q80 copy for 1x screens (`srcset`); every full-size file is lossless.

| File                       | Source and crop (x0, y0, x1, y1)                | Notes                                                              |
| -------------------------- | ----------------------------------------------- | ------------------------------------------------------------------ |
| `app-overview.webp`        | v17 `04-after-sync...` (0, 65, 1440, 745)       | Hero: synced decks, strip, mixer, overviews, table rows.           |
| `decks-merged.webp`        | v17 `03-after-both...` (0, 100, 1440, 356)      | Merged 3-band detail waveform, both decks playing.                 |
| `bands-before.webp`        | v17 `01-before...` (0, 130, 1440, 552)          | v16 deck area: near-monochrome detail, flat overviews.             |
| `bands-after.webp`         | v17 `03-after-both...` (0, 130, 1440, 552)      | Same area after W12: three bands in detail and overviews.          |
| `decks-synced.webp`        | v17 `04-after-sync...` (440, 128, 1000, 384)    | Seam readout 0 ms / +0.00 BPM, KEY and SYNC lit on B.              |
| `sync-refused.webp`        | v15 `04-sync-refused...` (720, 356, 1440, 412)  | Deck B strip: `SYNC: deck B has no beat grid`.                     |
| `decks-mixer.webp`         | v17 `04-after-sync...` (0, 356, 1440, 476)      | Control strip and the mixer row with live meters.                  |
| `grid-fallback.webp`       | v14 `04-grid-fallback...` (0, 100, 1440, 492)   | mm:ss time grid without a beat grid or waveform (pre-W12 colours). |
| `decks-collapsed.webp`     | v14 `05-collapsed...` (0, 100, 1440, 400)       | Detail collapsed; strip, overviews, table rows (pre-W12 colours).  |
| `waveform-beat-grid.webp`  | v13 `01-ventoy-style...` (1081, 362, 1440, 800) | Details waveform panel, computed grid 127.95 BPM.                  |
| `waveform-click-grid.webp` | v13 `02-click-track...` (1081, 362, 1440, 800)  | Details waveform panel, computed grid 120.00 BPM.                  |
| `social.png`               | composed from `app-overview`                    | 1200x630 Open Graph card set in Epilogue.                          |

Every capture contains the Bot Screen pointer and its name badge (over the track table,
or over the table header in v17 `01`–`03`); every crop excludes them. The v15, v16 and
v17 captures ran the engine's silent renderer (no audio device), so their status bar,
cropped out here, reads `NO AUDIO`; the transport, key lock, SYNC and mixer paths are
the same. The grid-fallback and collapsed crops predate the W12 band colours; those
states were not re-captured.

Every image is captioned as the provisional prototype; each crop is also the
full-resolution file the lightbox opens. Earlier crops (`track-list.webp`,
`decks-bar.webp`, the v12 overview and the v14–v16 deck crops) were replaced; the
deployed site keeps them until the next Pages deployment.

All images are project material under the repository's MIT licence.
