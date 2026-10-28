# Pre-alpha 16 — F-09 guarded TRACK.Mix oracle

Archived two retrospective Chromium-rendered evidence cards for one bounded synthetic F-02 post-edit oracle:

- `01-mix-edit-oracle.png` records TrackID 100 Mix `old & set`→`new < set` and absent Mix on TrackID 200.
- `02-scope-and-controls.png` records whole-tree comparison, exact-old/wrong-target refusals, semantic mutation controls, and evidence limits.

The matching `.html` files are the render sources. These renders document the focused test evidence; they are not captures from the original test run. `xtask/tests/f02_track_mix_edit_oracle.rs` applies the existing guarded adapter and independently decodes output with test-only `quick-xml`; the focused test passed (1/1), and source bytes remain unchanged.

Synthetic evidence only. Mix semantics, global F-09, Rekordbox compatibility, and ADR-005 qualification remain NOT DEMONSTRATED. No private library data or audio was used.
