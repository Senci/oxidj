# Pre-alpha 11 — F-09 guarded TRACK.Label oracle

Archived two rendered evidence cards for one bounded synthetic F-02 post-edit oracle.

- `01-label-edit-oracle.png` records the frozen TrackID 100 Label `old & imprint`→`new < imprint` edit and the absent Label state on TrackID 200.
- `02-scope-and-controls.png` records the whole-tree independent comparison, stale/wrong-target guards, semantic mutation controls, and evidence limits.

`xtask/tests/f02_track_label_edit_oracle.rs` applies the existing guarded adapter, decodes with test-only `quick-xml`, and compares the entire decoded tree with only target Label changed. The focused regression passed (1/1); source bytes remain unchanged.

Synthetic evidence only. Global F-09 remains NOT DEMONSTRATED; no Rekordbox compatibility or ADR-005 qualification is claimed. No private library data or audio was used.
