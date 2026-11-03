# UI W7 — design audit and fix pass (provisional library UI)

Real captures of `prototypes/oxidj-library-ui` (release build) on the isolated
Bot Screen (X11, 1440x900, full-screen composited grab via Hermes
`computer_use`), 2026-11-03. Synthetic data only: audio from
[`../waveform/gen_synthetic.py`](../waveform/gen_synthetic.py), grid from
[`../waveform/add_synthetic_grid.py`](../waveform/add_synthetic_grid.py), a third
imported file deleted afterwards (Missing state), and the synthetic F-02 fixture
`xtask/tests/fixtures/f02-playlist-member-insert/source.xml` for the XML window.
The scratch directory was bind-mounted at `/mnt`; no private library data.

- `before/` — release build of `e768ed1` (HEAD before this card).
- `after/` — release build of the final W7 tree (batch 2 on top of `c7aafb9`).
- [`contrast.py`](contrast.py) — WCAG ratios for every token pair the UI paints
  (`python3 contrast.py`; pass overrides as `n:border:6b7394,...`).

## Findings → fix → commit

| # | Finding (evidence) | Fix | Commit |
|---|---|---|---|
| 1 | Text fields and buttons had no outline (egui inactive stroke 0 px); Search/Import/XML fields looked like labels (`before/01`) | 1 px outline on inactive/hovered/active widgets | c7aafb9 |
| 2 | Border token failed non-text contrast: 1.62/1.42:1 (Night), 1.43/1.69:1 (Day) | Border raised to `#6b7394` (3.67/3.23:1) and `#737aa2` (3.22/3.79:1); test added | c7aafb9 |
| 3 | Tokyo Day selected-row text 4.25:1 (< 4.5) | Selection tint 0.28 → 0.20 (Day 4.68:1, Night 7.28:1); test added | c7aafb9 |
| 4 | Controls 18–22 px tall (< 24 px target) | `CONTROL_HEIGHT` 28 px, button padding 10×5, spacing 8×6; test added | c7aafb9 |
| 5 | Captions/badges/paths at egui's 9 px small size | Type scale: small 11.5, body/button 14, mono 13, heading 20; test added | c7aafb9 |
| 6 | Toolbar rows ragged, wide unexplained gaps, "Catalog:" colon inconsistent with other labels | Aligned label/field/action grid (2 groups per row ≥ 1200 px, 1 below); consistent labels; catalog path left-aligned, muted; field text vertically centred | c7aafb9, batch 2 |
| 7 | Track table used ~40 % of its width and wrapped filenames per character (`before/01`) | Filename column grows to content, capped and truncated with hover; muted small headers and paths | c7aafb9, batch 2 |
| 8 | Availability badges stretched to row height and grew every frame inside grid cells (Frame margin feedback loop) | Badge drawn in a control-height row | c7aafb9 |
| 9 | Details panel width jumped 205 ↔ 570 px with content; waveform and "Low/Mid/High" sliders clipped at 205 px (`before/03`, `before/04`) | Panel 420 px default, 340–520 px bound, resizable; aligned key/value facts grid with muted keys; gain sliders share one row sized to the panel, trailing fill | c7aafb9, batch 2 |
| 10 | XML window taller than the screen with no scroll; lower workflows unreachable (`before/05`) | Window scrolls, max height = viewport − 96 px | c7aafb9 |
| 11 | Mixed section-heading styles (strong text vs a faint 1 px separator tick) | One `SectionHeading`: 3 px primary accent bar + strong text, 4 px air above; Name/Artist/Comments/Album panels use it too | c7aafb9 |
| 12 | Hard-coded white overview window marker (`track_waveform.rs`) | `tokens.text.gamma_multiply(0.16)` | c7aafb9 |
| 13 | Table layout oscillated between frames (fixed-width cell inside a grid) so a press and release could land on different rows | Cell capped with `set_max_width` instead of `set_width` | batch 2 |
| 14 | Code quality: 850-line `ui()` | Split into `toolbar`, `xml_preview_window`, `navigation`, `demo_decks`, `details`, `track_table`; shared `file_name`/`availability_status` helpers; docs on pub theme/component items | c7aafb9 |

All existing UI tests pass with unchanged assertions (178 passed, 1 ignored,
including 2 new theme tests).

## After captures

| File | Shows |
|---|---|
| `after/01-selection-waveform-1440.png` | Tokyo Night, track 2 selected: outlined fields, aligned toolbar, table at content width, facts grid, waveform with bar grid and gain sliders in the panel |
| `after/02-tokyo-day-list.png` | Tokyo Day, populated list, Missing badge |

Batch 2 (trailing slider fill, filename-cap fix) landed after `after/01` was
captured; `after/01` comes from the build with the oscillation fix already in.

## NOT RUN / NOT DEMONSTRATED

- Narrow (~900 px) live capture: NOT RUN. The window manager kept the window
  maximised and ignored the resize. The 1-group-per-row toolbar branch is
  exercised only by code; `toolbar_controls_fit_narrow_and_wide_viewports`
  covers 800 px with its own replica layout.
- XML window after the fix, and Tokyo Day with a selection: NOT RUN live.
  Keyboard and pointer input stopped reaching the app after the theme combo
  closed. The scroll fix is `Window::vscroll(true)` and has no live capture.
- Owner's native display, accessibility / screen reader (Orca), keyboard focus
  order on a real desktop: NOT DEMONSTRATED.
