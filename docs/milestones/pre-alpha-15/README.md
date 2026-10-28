# Pre-alpha 15 — F-09 guarded TRACK.Remixer oracle

Archived two Chromium-rendered evidence cards for one bounded synthetic F-02 post-edit oracle:

- `01-remixer-edit-oracle.png` records TrackID 100 Remixer `old & mix`→`new < mix` and absent Remixer on TrackID 200.
- `02-scope-and-controls.png` records whole-tree comparison, exact-old/wrong-target refusals, semantic mutation controls, and evidence limits.

The matching `.html` files are the render sources. These are retrospective Chromium renders, not captures from the original test run. `xtask/tests/f02_track_remixer_edit_oracle.rs` applies the existing guarded adapter and independently decodes output with test-only `quick-xml`; the focused test passed (1/1), and source bytes remain unchanged.

Synthetic evidence only. Remixer semantics, global F-09, Rekordbox compatibility, and ADR-005 qualification remain NOT DEMONSTRATED. No private library data or audio was used.
