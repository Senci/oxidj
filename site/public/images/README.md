# Public image provenance

All screenshots are crops of the two reviewed after-W9 captures in
`docs/milestones/archive/ui-minimal/after-w9/` (release build of the provisional
`prototypes/oxidj-library-ui`, Tokyo Night, isolated Bot Screen X11 at 1440x900,
2026-11-03, commit `89cd073`). The catalog is the synthetic W8/W9 test set: ten
generated WAV files under `/mnt/audio/synthetic-*` (a bind-mounted scratch path), not
a personal music collection. The pixels were inspected before publication; no home
directory, username or private library name is visible.

`scripts/crop-screenshots.py` regenerates every file below
(`uv run --no-project --with pillow python scripts/crop-screenshots.py`).

| File                    | Source and crop (x0, y0, x1, y1)             | Notes                                                     |
| ----------------------- | -------------------------------------------- | --------------------------------------------------------- |
| `app-overview.webp`     | `02-three-band...` (0, 63, 1440, 856)        | Whole app window; desktop bar and dock removed; lossless. |
| `app-overview-720.webp` | same, half size                              | Lossy q84 for 1x screens (`srcset`).                      |
| `track-list.webp`       | `01-details-decks...` (176, 97, 1080, 362)   | Track table only; lossless.                               |
| `decks-bar.webp`        | `02-three-band...` (0, 789, 834, 824)        | Demo decks bar, Deck A Play on; lossless.                 |
| `waveform-mono.webp`    | `01-details-decks...` (1081, 357, 1440, 762) | Waveform panel, Mono; lossless.                           |
| `waveform-3band.webp`   | `02-three-band...` (1081, 357, 1440, 762)    | Waveform panel, 3-band; lossless.                         |
| `social.png`            | composed from `app-overview`                 | 1200x630 Open Graph card set in Epilogue.                 |

Every image is captioned as the provisional prototype; each crop is also the
full-resolution file the lightbox opens.

The previous `library-prototype.png` hero image, and the social card made from it,
showed a local profile scratch path in the viewer header and Path column. Both were
replaced by the crops above and are no longer in this tree (the deployed site keeps
them until the next Pages deployment).

All images are project material under the repository's MIT licence.
