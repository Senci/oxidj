# Pre-alpha 14 — F-09 guarded TRACK.Grouping oracle

Archived two Chromium-rendered evidence cards for one bounded synthetic F-02 post-edit oracle:

- `01-grouping-edit-oracle.png` records frozen TrackID 100 Grouping `old & crate`→`new < crate` and absent Grouping on TrackID 200.
- `02-scope-and-controls.png` records whole-tree comparison, exact-old/wrong-target refusals, semantic mutation controls, and evidence limits.

`xtask/tests/f02_track_grouping_edit_oracle.rs` applies the existing guarded adapter and independently decodes output with test-only `quick-xml`, comparing the complete decoded tree with only target Grouping changed. The focused test passed (1/1); source bytes remain unchanged.

Synthetic evidence only. Grouping taxonomy/semantics, global F-09, Rekordbox compatibility, and ADR-005 qualification remain NOT DEMONSTRATED. No private library data or audio was used.
