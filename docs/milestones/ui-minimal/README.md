# UI W8 — dense minimal shell (provisional library UI)

Owner goal: "track list full width; side panels toggleable; layout more
condensed and minimalistic, like Rekordbox / Mixxx".

Real captures of `prototypes/oxidj-library-ui` (release build) on the isolated
Bot Screen (X11, 1440x900, full-screen composited grab via Hermes
`computer_use`), 2026-11-03. Synthetic data only: audio from
[`../waveform/gen_synthetic.py`](../waveform/gen_synthetic.py) plus seven
generated sine tones, beat grid from
[`../waveform/add_synthetic_grid.py`](../waveform/add_synthetic_grid.py), one
imported copy deleted afterwards (Missing state). The scratch directory was
bind-mounted at `/mnt`; no private library data.

- `before/` — release build of `31ed107` (HEAD before this card; W7 shell).
- `after/` — release build of the final W8 tree (the `…` popup placement fix is
  included; the binary was byte-compared with the captured one).

## Before → after

| | Before (W7) | After (W8) |
|---|---|---|
| Toolbar | 4 rows: title + theme/tool buttons, then a Search / Catalog / Import / XML field grid (~115 px) | One 32 px row: `OxiDJ`, search (grows), `Nav` `Details` `Decks` toggles, theme, `…` |
| Catalog / import / XML | Always visible in the toolbar grid | `…` popup (catalog path + Reload, import folder + Import references, Rekordbox XML + Preview XML, Control concepts…, Waveform view…, Edit palette… for Custom) |
| Navigation | Always shown, 184 px | Toggle (Ctrl+B), on by default, 168 px |
| Details | Always shown, 340–520 px | Toggle (Ctrl+I), off by default; selecting a row does not open it |
| Decks | Always shown | Toggle (Ctrl+D), off by default |
| Track table | Sort row + "Tracks" heading, two-line ~53 px rows, ~40 % of the width | Edge to edge, sortable header (click ID/Filename/Duration, again to reverse), 24 px single-line rows, separate Path column, horizontal scroll only below the column minimum |
| Notices | Badge in the toolbar | One-line bottom status bar with track count and the panel shortcut legend |
| Metrics | 28 px controls, 14 px body, 8x6 spacing | 24 px controls/rows (WCAG target), 13 px body, 6x4 spacing; colours and contrast unchanged |

## Captures

| File | Shows |
|---|---|
| `before/01-default-day-1440.png` | W7 shell, Tokyo Day (persisted theme at the time): 4-row toolbar, sort row, two-line rows, empty details panel, deck bar |
| `after/01-default-1440.png` | Defaults, Tokyo Night: nav on, details and decks off; table spans the rest of the window |
| `after/02-all-panels-on-1440.png` | Nav + details + decks on, track 2 selected (selected-row rail, decks show the selection) |
| `after/03-all-panels-off-1440.png` | Nav toggled off: the table spans the full window width |
| `after/04-details-three-band-1440.png` | Details on, track 2 selected, waveform style 3-band with the beat-grid window |
| `after/05-more-menu-day-1440.png` | `…` popup open under its button (Tokyo Day) with the catalog/import/XML rows and tool buttons |
| `after/06-tokyo-day-default-1440.png` | Defaults, Tokyo Day |

## Verification

- Harness tests (`src/shell_tests.rs`) render the production `LibraryApp::ui`
  at 1440 and 900 px: defaults; toolbar one row (≤ 33 px), controls unclipped,
  non-overlapping, ≥ 24 px; full-width 24 px rows; table spans the full window
  with both side panels off; each toggle by click and by shortcut shows/hides
  its panel and the table shrinks/grows by exactly the panel width (decks:
  height); all panels on still fit; a row selection keeps details closed;
  header click sorts and reverses; the `…` popup is inside the window,
  right-aligned under its button, holds Reload / Import references / Preview
  XML / tool buttons and both text fields, and Preview XML opens the XML window
  and closes the popup.
- Existing keyboard/behaviour tests pass; the two tests that encoded the
  removed layout were updated (see the commit message).
- Live capture found two defects the headless harness had not: the toolbar
  grew to ~115 px (separators in a right-to-left layout fill the available
  height) and the `…` popup opened half off-screen (egui positioned it from a
  stale previous-frame size because it repaints only on input). Both were
  fixed (fixed-height toolbar/status rows; popup placed from its fixed width)
  before these captures, and the popup geometry is asserted in the harness.

## NOT RUN

- Owner's native display / Wayland session (Bot Screen X11 only).
- Screen reader (Orca) pass over the new toggles, header and rows; AccessKit
  names/roles are asserted in tests only.
- 900 px live capture (900 px is covered by the harness tests only).
- Large-catalog performance of the virtualised table (deferred qualification).
