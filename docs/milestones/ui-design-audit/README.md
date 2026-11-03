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
- `after/` — release build of the final W7 tree (round 2).
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

| 15 | Narrow (900 px): demo-deck bar orphaned Deck B's Play/Cue on a second line (`before-r2/01`) | Each deck wraps as one unit (row break when < 340 px remain) | round 2 |
| 16 | Empty-library message justified (stretched word spacing) at 900 px (`before-r2/01`) | Centred top-down layout without justification | round 2 |
| 17 | Narrow: Format facts / Availability columns clipped and unreachable; "3 tracks" clipped (`before-r2/02`) | Track table scrolls horizontally (`ScrollArea::both`); sort row wraps, count moves as a whole | round 2 |
| 18 | ID/Filename headers and the path line misaligned with the selectable cell text by the button padding (`before-r2/02`) | Headers and path inset by `button_padding.x` | round 2 |
| 19 | Track facts rows overlapped after selecting a track (wrapped values sized from a previous frame/width) (`before-r2/03`) | Values truncate (full text on hover); SHA-256 on two fixed 32-digit lines; discard + repaint on selection change | round 2 |
| 20 | Details panel jumped 340 → 420 px when a track was selected (ScrollArea auto-shrink) and the waveform was cut off at 791 px height | Details panel wrapped in a non-shrinking vertical scroll area | round 2 |
| 21 | Keyboard focus ring indistinguishable from hover (both 1 px focus colour) (`before-r2/04`) | Focused/pressed widgets get a 2 px focus-coloured ring | round 2 |
| 22 | XML window: Name section used plain strong text and sections alternated separator/no separator (`before-r2/05`) | Every workflow section starts with `SectionHeading`; separators only around pending sub-blocks | round 2 |
| 23 | `toolbar_controls_fit_narrow_and_wide_viewports` tested a replica of the old toolbar | Test renders the production `toolbar()` at 800 and 1440 px and checks every Button/TextInput/ComboBox from AccessKit bounds (inside the viewport, no overlaps) | round 2 |

All existing UI tests pass with unchanged assertions (178 passed, 1 ignored,
including 2 new theme tests); the only test edit is #23, as requested in review.

## Contrast report

`python3 contrast.py` defaults to the **shipped** tokens (`theme.rs`, border
`#6b7394` Night / `#737aa2` Day, selection tint 0.20) and prints 0 FAIL. The
pre-audit borders are reproducible with `python3 contrast.py n:border:3b3d55,d:border:b8bed0`.

## After captures (final release build, Bot Screen, synthetic data)

| File | Shows |
|---|---|
| `after/03-empty-library-1440.png` | Empty catalog, Tokyo Night: centred empty-state copy |
| `after/04-populated-1440.png` | Populated list, no selection; details panel at its stable 420 px |
| `after/05-selection-mono-1440.png` | Track 2 selected: facts grid (no overlap, SHA on two lines), waveform Mono with bar grid |
| `after/06-three-band-1440.png` | Same, 3-band |
| `after/07-rgb-1440.png` | Same, RGB |
| `after/08-missing-unanalysed-1440.png` | Missing track selected: Missing badge, "No computed waveform: track not analysed" state |
| `after/09-tokyo-day-list-1440.png` | Tokyo Day, populated list |
| `after/10-populated-900.png` | 900 px wide: four-row toolbar, wrapped sort row, table with horizontal scroll, decks on two rows |

`before-r2/` are 900 px captures of intermediate builds that showed findings 15–22.
The first-round `after/01`/`02` were removed: they predate the final tree.

## NOT RUN / NOT DEMONSTRATED

- Final-build captures of the XML window, a selection at 900 px, and Tokyo Day
  with a selection: NOT RUN. The XML window (scrolling, all sections reachable)
  was captured at 900 px from an intermediate build (`before-r2/05`), and
  selection at 900 px from an intermediate build (`before-r2/03`); the fixes for
  #19, #20 and #22 are only in the 1440 px after captures. Cause: after app
  restarts, foreground clicks/keys stopped reaching the app (twice, including
  after a Bot Screen restart); Ctrl+Alt+2 arrived as Ctrl+2. The first launcher
  also left earlier instances running (fixed mid-session).
- Hover/active states were seen in captures (hover outline on fields, Tab moved
  focus to Preview XML in `before-r2/04`); disabled states were not exercised live.
- Owner's native display, accessibility / screen reader (Orca), keyboard focus
  order on a real desktop: NOT DEMONSTRATED.
