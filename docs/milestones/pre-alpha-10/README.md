# Pre-alpha 10 — F-09 guarded TRACK.Genre oracle

Archived evidence cards for one bounded synthetic F-02 post-edit oracle.

- `01-genre-edit-oracle.png` records the frozen TrackID 100 Genre `old & style`→`new < style` case and absent Genre state on TrackID 200.
- `02-scope-and-controls.png` records stale/wrong-target refusals, semantic mutation controls, and explicit evidence limits.

`xtask/tests/f02_track_genre_edit_oracle.rs` independently decodes output with test-only `quick-xml` and compares the complete decoded tree against the frozen source with only target Genre changed. The focused regression passed (1/1); frozen source bytes remain unchanged.

Synthetic evidence only. Global F-09 remains NOT DEMONSTRATED; no Rekordbox compatibility or ADR-005 qualification is claimed. No private library data or audio was used.
