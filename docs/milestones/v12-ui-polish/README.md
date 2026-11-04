# v0.12 — W11 visual polish of the provisional library UI

Date: 2026-11-04 · Commits: `339372b` (polish + tests), follow-up commit (Format width, row-sized badge, details re-layout on reopen, these docs).

Real full-screen composited Bot Screen captures (X11, 1440x900) of release builds; same synthetic W8 catalog bind-mounted at `/mnt` (no private data). Each state is a fresh instance: one click on track 2, Ctrl+I, Ctrl+D, one click on empty table space.

| File | Build | Theme |
|---|---|---|
| `01-before-night.png` | `c091a13` (before W11) | Tokyo Night |
| `02-before-day.png` | `c091a13` | Tokyo Day |
| `03-after-night.png` | final W11 tree | Tokyo Night |
| `04-after-day.png` | final W11 tree | Tokyo Day |

Capture caveat: the composited grab can lag the window by one frame. In `03-after-night.png` the Availability→SHA-256 rows show that stale frame (extra gap); a window capture of the same instance, and a later composited grab in Day (`04`), show even rows. The harness test `details_sha256_is_truncated_with_full_hover_and_copy` pins the even row pitch.

## Findings → fix

| # | Finding (v0.11) | Fix |
|---|---|---|
| 1 | Table: all left-aligned, verbose Format, Path repeats Filename, flat row states, wordy Availability | Status-dot column (state + path on row hover); ID and Duration (`m:ss`) right-aligned with right-aligned headers; Folder replaces Path; Format `44.1 kHz · 16-bit · mono`; zebra at 50 % surface, hover = surface, selected = selection tint + 3 px rail (all text in text colour), 2 px focus ring only for keyboard focus; Missing rows in muted text. Also fixed: cell colours were overridden by `override_text_color` (muted Path was painted in text colour). |
| 2 | Dead black block under few rows | Zebra stripes continue faintly to the bottom; header stays fixed above the scroll area. |
| 3 | Toggles looked like buttons; plain search; heavy theme caption | Nav \| Details \| Decks segmented group (selection tint + 2 px primary underline when on, muted text when off). Full-strength primary fill rejected: text on it is < 4.5:1 in Tokyo Day. Search: painted magnifier, framed field, clear (x) button when non-empty. Caption `Theme`, shortcuts in its tooltip. |
| 4 | Nav rhythm | One heading style (`Collection`, `Filters`), full-width 24 px rows, muted right-aligned counts, selected filter = tint + rail, 8 px insets. |
| 5 | Details key/values | Fixed 88 px key column, muted keys, SHA-256 one middle-truncated line (full digest on hover, click copies), availability badge painted at body-row height, 8 px section spacing. |
| 6 | Position slider nearly invisible; strip spacing; marker | Position uses the gains' primary trailing fill; overview 4 px under the detail strip with the same x-range (asserted); window marker gets a 1 px focus outline; Style is the segmented group in the label column. |
| 7 | Decks/status bars | Decks 32 px (8,4 inset) with Play \| Cue groups; status bar 24 px: status in body text colour left, muted small hints right; separators are a 50 % border hairline. |
| 8 | Chrome | 8 px outer insets for bars and panels; separators/panel edges one light hairline (control outlines keep the >= 3:1 border); every focus ring uses `focus`. Unchanged: details panel still lists Filename and Path (it is the place for the full path). |

## Constraints

Theme tokens only (zebra/hairline/selection are token tints). Contrast: `w11_row_fills_keep_text_contrast` plus the existing token tests; `archive/ui-design-audit/contrast.py` pairs unchanged. Targets >= 24 px and toolbar no-overlap asserted by existing tests (segmented cells and the clear button are 1 px apart). Waveform renderer and allocation tests unchanged and passing.

NOT DEMONSTRATED (unchanged): native display, screen reader, platform qualification.

[← Timeline](../README.md)
