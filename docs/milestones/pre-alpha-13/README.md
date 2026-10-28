# Pre-alpha 13 — F-09 guarded TRACK.Composer oracle

Archived two Chromium-rendered evidence cards for one bounded synthetic F-02 post-edit oracle:

- `01-composer-edit.png` records the frozen TrackID 100 Composer `old & composer`→`new < composer` edit and absent Composer on TrackID 200.
- `02-scope-and-controls.png` records whole-tree comparison and the target/scope mutation controls and evidence limits.

`xtask/tests/f02_track_composer_edit_oracle.rs` applies the existing guarded adapter, decodes with test-only `quick-xml`, and compares the complete decoded tree with only target Composer changed. Focused test passed (1/1); source bytes remain unchanged.

Synthetic evidence only. Composer identity/credit semantics, global F-09, Rekordbox compatibility and ADR-005 qualification remain NOT DEMONSTRATED. No private library data or audio was used.
