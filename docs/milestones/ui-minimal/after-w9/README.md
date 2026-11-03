# UI W9 — waveform panel polish, decks check (provisional library UI)

Real full-screen composited captures of the release build of the final W9 tree
(`prototypes/oxidj-library-ui`; the running copy was byte-compared with
`target/release/oxidj-library-ui`) on the isolated Bot Screen (X11, 1440x900),
2026-11-03, same synthetic catalog as W8 (scratch data bind-mounted at `/mnt`;
no private library data). Selection, panel toggles, style and Play were real
clicks on a fresh instance.

| File | Shows |
|---|---|
| `01-details-decks-mono-night-1440.png` | Tokyo Night, track 2 selected, Details + Decks on, Mono: slider rows read `Low ━━○ 1.00` with one label column and aligned rails (Position shares the column and rail width); the detail strip uses the spare height (140 px); captions on short wrapping lines; deck bar with truncated name column |
| `02-three-band-deck-a-play-night-1440.png` | Same, 3-band style, Deck A Play toggled on: the button keeps its label and width, so Cue and Deck B do not move |

Decks bar check (acceptance 4): the first W9 capture showed Play turning into
`Playing · demo`, which widened the button and shifted Cue and the whole of
Deck B on every toggle; a long filename would also push Play/Cue off-screen.
Fixed: fixed `Play` label (selected fill shows the state, hover says demo only)
and a 200 px truncated name column with the full name on hover.

## Verification

- `w9_captions_fit_slider_rows_align_and_strip_height_follows_details_height`
  (production `LibraryApp::ui`, Tokyo Night and Day, 750x900 → details at its
  320 px minimum, 1440x900, 1440x600; grid + vendor track): every caption glyph
  inside the panel and its clip; label column aligned, label before rail, value
  after the rail, rails/values aligned, rows ≥ 24 px; strip 96 < h ≤ 140 with the
  vendor strip still inside the panel at 900 px, exactly 96 px at 600 px; panel
  and strip stable on the next frame.
- `decks_bar_truncates_long_names_and_keeps_play_cue_reachable` (1440 and
  900 px, 160-character filename): Play/Cue inside the decks panel, ≥ 24 px,
  name painted ≤ 200 px; a real Play click toggles deck A and no deck control
  moves. Both new tests failed against the pre-fix code.
- Details panel minimum raised 300 → 320 px: the two-line SHA-256 row needs
  ~296 px of content, so the panel had silently been 312 px at its "minimum".
- Waveform frame time (release, `waveform_frame_time`, 1440 px, run+tessellate):
  0.07–0.63 ms/frame (3-band highest), < 2 ms. Allocation tests unchanged and
  passing.

## NOT RUN

- Tokyo Day live capture (the Ctrl+Alt+2 hotkey reached the app as Ctrl+2;
  Day is covered by the harness test only).
- 750/900 px live captures (harness only); owner's native display; screen
  reader; vendor PWV strip capture (synthetic catalog has none).
