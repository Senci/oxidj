# OxiDJ UI progress timeline

How the provisional OxiDJ UI and its Rekordbox XML interchange grew, one version per capability era, in date order. Versions are documentation labels for this timeline (not releases); dates and commits come from `git log` of each evidence directory. Each version links to a curated directory with at most three key screenshots; all raw per-change evidence is in [`archive/`](archive/README.md).

v0.2–v0.6 are interoperability evidence (CLI/test oracles rendered as cards), not app UI. Everything is synthetic data on the isolated Bot Screen; nothing here is compatibility, accessibility, platform or performance qualification. Live status: [`plan/STATUS.md`](../../plan/STATUS.md).

| Version | Date | Commits | Headline | Key screenshot |
|---|---|---|---|---|
| [v0.1](v01-library-and-gui-probe/README.md) | 2026-10-25 – 10-27 | `c3b256f..41dd2a6` | First pixels: a persisted local-library window and the 200,000-row virtualized GUI probe. | <img src="v01-library-and-gui-probe/01-local-library-startup.png" width="240" alt="v0.1 key screenshot"> |
| [v0.2](v02-position-mark-and-tempo-edits/README.md) | 2026-10-28 | `364977d..680e7a2` | Guarded Rekordbox XML edits begin: POSITION_MARK Start/End/Name and TEMPO Battito/Metro/Inizio. | <img src="v02-position-mark-and-tempo-edits/01-position-mark-start-oracle.png" width="240" alt="v0.2 key screenshot"> |
| [v0.3](v03-track-text-field-edits/README.md) | 2026-10-28 | `16bbd98..44228d7` | Guarded edits for twelve TRACK text fields (Name, Artist, Album, Genre, Comments, …, Year). | <img src="v03-track-text-field-edits/01-track-name-oracle.png" width="240" alt="v0.3 key screenshot"> |
| [v0.4](v04-track-raw-field-edits/README.md) | 2026-10-28 | `f1d322f..626f2b6` | Guarded raw edits for numeric/date TRACK attributes (Kind, Rating, Colour, dates, BPM, sizes, rates). | <img src="v04-track-raw-field-edits/01-average-bpm-oracle.png" width="240" alt="v0.4 key screenshot"> |
| [v0.5](v05-no-edit-export-retention/README.md) | 2026-10-28 | `58bfc3a..d561cc8` | No-edit export is byte-faithful: raw TRACK attributes and playlist NODE bytes survive present/empty/absent. | <img src="v05-no-edit-export-retention/01-sample-rate-no-edit.png" width="240" alt="v0.5 key screenshot"> |
| [v0.6](v06-position-mark-and-tempo-boundaries/README.md) | 2026-10-28 | `dd2364e..459387d` | Cue/tempo parsing boundaries: optional End, signed Num, Battito range — plus the B1→B2 reference transition. | <img src="v06-position-mark-and-tempo-boundaries/01-signed-num-preview.png" width="240" alt="v0.6 key screenshot"> |
| [v0.7](v07-themes/README.md) | 2026-10-30 | `fe4f672..03636b5` | Themes: Tokyo Night, Tokyo Day and an editable Custom palette that survives relaunch. | <img src="v07-themes/01-tokyo-night.png" width="240" alt="v0.7 key screenshot"> |
| [v0.8](v08-waveform/README.md) | 2026-11-03 | `107f656..e768ed1` | Waveforms: computed peaks in Mono/3-band/RGB, smooth anti-aliased envelopes, bar-aware detail window from the imported beat grid. | <img src="v08-waveform/01-three-band-smooth.png" width="240" alt="v0.8 key screenshot"> |
| [v0.9](v09-design-audit/README.md) | 2026-11-03 | `0f49c89..31ed107` | Design audit W7: outlines, contrast, 24 px targets, type scale, stable table and details panel, scrollable XML window. | <img src="v09-design-audit/02-after-populated.png" width="240" alt="v0.9 key screenshot"> |
| [v0.10](v10-dense-shell/README.md) | 2026-11-03 | `91144a4` | Dense minimal shell W8: one-row toolbar, full-width sortable track table, toggleable Nav/Details/Decks panels. | <img src="v10-dense-shell/02-after-dense-shell.png" width="240" alt="v0.10 key screenshot"> |
| [v0.11](v11-waveform-panel-polish/README.md) | 2026-11-03 | `89cd073` | W9 polish: aligned waveform slider rows, taller detail strip, stable decks bar. | <img src="v11-waveform-panel-polish/01-details-decks-mono.png" width="240" alt="v0.11 key screenshot"> |
| [v0.12](v12-ui-polish/README.md) | 2026-11-04 | `339372b..` | W11 polish: segmented panel toggles, right-aligned numbers, search with clear, compact details, visible Position slider. | <img src="v12-ui-polish/04-after-day.png" width="240" alt="v0.12 key screenshot"> |
| [v0.13](v13-computed-beat-grid/README.md) | 2026-11-04 | `9a9b96c..` | Computed BPM and beat grid (taktus) in track details when no imported grid exists; import always wins; a suggestion, not Rekordbox parity. | <img src="v13-computed-beat-grid/02-click-track-computed-grid.png" width="240" alt="v0.13 key screenshot"> |
| [v0.14](v14-merged-decks/README.md) | 2026-11-05 | `9b9acbd..` | M8 D1: merged two-deck detail waveform (A warm above, B cool below, one playhead), club-standard control strip, two separate overviews, collapsible sections; silent transport, no audio yet. | <img src="v14-merged-decks/02-after-both-decks-loaded.png" width="240" alt="v0.14 key screenshot"> |
| [v0.15](v15-tempo-keylock-sync/README.md) | 2026-11-05 | `bdeecf5..` | M8 D3: tempo fader with key lock (Signalsmith Stretch behind `TimeStretch`), SYNC (tempo + nearest-beat phase, imported grid wins) with visible refusals, live BPM and seam phase readout, warped beat lines. | <img src="v15-tempo-keylock-sync/02-after-sync-b-keylock-matched.png" width="240" alt="v0.15 key screenshot"> |

## Before → after

| | Before | After |
|---|---|---|
| v0.9 design audit | ![before](v09-design-audit/01-before-populated.png) | ![after](v09-design-audit/02-after-populated.png) |
| v0.10 dense shell | ![before](v10-dense-shell/01-before-w7-shell.png) | ![after](v10-dense-shell/02-after-dense-shell.png) |
| v0.12 W11 polish | ![before](v12-ui-polish/01-before-night.png) | ![after](v12-ui-polish/03-after-night.png) |
| v0.14 merged decks | ![before](v14-merged-decks/01-before-library-only.png) | ![after](v14-merged-decks/02-after-both-decks-loaded.png) |
| v0.15 tempo/key lock/SYNC | ![before](v15-tempo-keylock-sync/01-before-sync-unmatched.png) | ![after](v15-tempo-keylock-sync/02-after-sync-b-keylock-matched.png) |
