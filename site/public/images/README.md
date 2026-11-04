# Public image provenance

All screenshots are crops of reviewed, inspected Bot Screen captures (X11, 1440x900,
release builds of the provisional `prototypes/oxidj-library-ui`, Tokyo Night,
2026-11-04). Audio is synthetic only, generated for the captures and bind-mounted at
`/mnt`; no personal music collection, home directory, username or private library
name is visible. The pixels were inspected again before publication.

| Source capture (under `docs/milestones/`)                  | Build (added in)              | Catalog                                                            |
| ---------------------------------------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `v12-ui-polish/03-after-night.png`                         | final W11 tree (`9714324`)    | synthetic W8 set: ten generated WAV files, one removed on purpose  |
| `v13-computed-beat-grid/01-ventoy-style-computed-grid.png` | release `9a9b96c` (`d90cb51`) | five synthetic loops in a Ventoy-style layout, no imported library |
| `v13-computed-beat-grid/02-click-track-computed-grid.png`  | release `9a9b96c` (`d90cb51`) | one synthetic 120 BPM click track                                  |

`scripts/crop-screenshots.py` regenerates every file below from those sources and
deletes superseded crops (`uv run --no-project --with pillow python scripts/crop-screenshots.py`
from `site/`).

| File                       | Source and crop (x0, y0, x1, y1)            | Notes                                                      |
| -------------------------- | ------------------------------------------- | ---------------------------------------------------------- |
| `app-overview.webp`        | `03-after-night` (0, 65, 1440, 856)         | Whole app window; desktop bar, title bar and dock removed. |
| `app-overview-720.webp`    | same, half size                             | Lossy q84 for 1x screens (`srcset`).                       |
| `track-list.webp`          | `03-after-night` (176, 97, 1080, 362)       | Track table only; lossless.                                |
| `decks-bar.webp`           | `03-after-night` (0, 801, 828, 832)         | Polished W11 decks bar (Play and Cue groups); lossless.    |
| `waveform-beat-grid.webp`  | `01-ventoy-style...` (1081, 362, 1440, 800) | Waveform panel, computed grid 127.95 BPM; lossless.        |
| `waveform-click-grid.webp` | `02-click-track...` (1081, 362, 1440, 800)  | Waveform panel, computed grid 120.00 BPM; lossless.        |
| `social.png`               | composed from `app-overview`                | 1200x630 Open Graph card set in Epilogue.                  |

The v13 captures contain the Bot Screen pointer badge over the empty track table; the
waveform crops exclude it. In `03-after-night.png` the Availability to SHA-256 rows
show one stale composited frame (a slightly larger gap), as noted in its README.

Every image is captioned as the provisional prototype; each crop is also the
full-resolution file the lightbox opens. The after-W9 crops (`waveform-mono.webp`,
`waveform-3band.webp` and the pre-W11 overview, track list and decks bar) were
replaced; the deployed site keeps them until the next Pages deployment.

All images are project material under the repository's MIT licence.
