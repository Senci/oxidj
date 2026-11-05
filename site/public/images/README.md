# Public image provenance

All screenshots are crops of reviewed, inspected Bot Screen captures (X11, 1440x900,
release builds of the provisional `prototypes/oxidj-library-ui`, Tokyo Night). Audio is
synthetic only, generated for the captures and bind-mounted at `/mnt`; no personal
music collection, home directory, username or private library name is visible. The
pixels were inspected again before publication (2026-11-05).

| Source capture (under `docs/milestones/`)                     | Build                              | Catalog                                                                        |
| ------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------ |
| `v13-computed-beat-grid/01-ventoy-style-computed-grid.png`    | release `9a9b96c`                  | five synthetic loops in a Ventoy-style layout, no imported library             |
| `v13-computed-beat-grid/02-click-track-computed-grid.png`     | release `9a9b96c`                  | one synthetic 120 BPM click track                                              |
| `v14-merged-decks/02-after-both-decks-loaded.png`             | release `ef5c601` (M8 D1)          | four generated 4:00 WAV click/kick loops with synthetic grids, cues and a loop |
| `v14-merged-decks/04-grid-fallback-no-grid-and-analysing.png` | release `ef5c601` (M8 D1)          | same set: `c-nogrid.wav` on A, `d-pending.wav` on B                            |
| `v14-merged-decks/05-collapsed-detail-library-grows.png`      | release `ef5c601` (M8 D1)          | same set, detail waveform collapsed                                            |
| `v15-tempo-keylock-sync/02-after-sync-b-keylock-matched.png`  | release `fc46d0f` (M8 D3)          | same set; KEY and SYNC on deck B                                               |
| `v15-tempo-keylock-sync/04-sync-refused-no-beat-grid.png`     | release `fc46d0f` (M8 D3)          | same set; SYNC refusal on deck B                                               |
| `v16-mixer/01-mixer-both-playing-centre.png`                  | release of the D4 tree (`8b6087b`) | same set; mixer shown, both decks playing                                      |

`scripts/crop-screenshots.py` regenerates every file below from those sources and
deletes superseded crops (`uv run --no-project --with pillow python scripts/crop-screenshots.py`
from `site/`). Full-width crops also get a `-720` half-width lossy q84 copy for 1x
screens (`srcset`); every full-size file is lossless.

| File                       | Source and crop (x0, y0, x1, y1)                | Notes                                                  |
| -------------------------- | ----------------------------------------------- | ------------------------------------------------------ |
| `app-overview.webp`        | v16 `01-mixer...` (0, 65, 1440, 745)            | Hero: decks, strip, mixer, overviews, four table rows. |
| `decks-merged.webp`        | v14 `02-after...` (0, 100, 1440, 492)           | Merged waveform, strip and two overviews.              |
| `decks-synced.webp`        | v15 `02-after-sync...` (440, 128, 1000, 384)    | Seam readout +0 ms / +0.00 BPM, KEY and SYNC lit on B. |
| `sync-refused.webp`        | v15 `04-sync-refused...` (720, 356, 1440, 412)  | Deck B strip: `SYNC: deck B has no beat grid`.         |
| `decks-mixer.webp`         | v16 `01-mixer...` (0, 356, 1440, 476)           | Control strip and the mixer row.                       |
| `grid-fallback.webp`       | v14 `04-grid-fallback...` (0, 100, 1440, 492)   | mm:ss time grid without a beat grid or waveform.       |
| `decks-collapsed.webp`     | v14 `05-collapsed...` (0, 100, 1440, 400)       | Detail collapsed; strip, overviews, table rows.        |
| `waveform-beat-grid.webp`  | v13 `01-ventoy-style...` (1081, 362, 1440, 800) | Details waveform panel, computed grid 127.95 BPM.      |
| `waveform-click-grid.webp` | v13 `02-click-track...` (1081, 362, 1440, 800)  | Details waveform panel, computed grid 120.00 BPM.      |
| `social.png`               | composed from `app-overview`                    | 1200x630 Open Graph card set in Epilogue.              |

Every capture contains the Bot Screen pointer badge (over the empty track table, or
over deck B's PLAY button in v15 `03`); every crop excludes it, and v15 `03` is not
used. The v15 and v16 captures ran the engine's silent renderer (no audio device), so
their status bar, cropped out here, reads `NO AUDIO`; the transport, key lock, SYNC
and mixer paths are the same.

Every image is captioned as the provisional prototype; each crop is also the
full-resolution file the lightbox opens. The pre-M8 crops (`track-list.webp`,
`decks-bar.webp` and the v12 overview) were replaced; the deployed site keeps them
until the next Pages deployment.

All images are project material under the repository's MIT licence.
