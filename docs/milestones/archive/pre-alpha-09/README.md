# Pre-alpha 09 — F-09 guarded TRACK.Album oracle

Archived evidence cards for one bounded synthetic F-02 post-edit oracle.

- `01-album-edit-oracle.png` records the frozen TrackID 100 Album `old & record`→`new < record` case and absent Album state on TrackID 200.
- `02-scope-and-controls.png` records stale/wrong-target refusal, semantic mutation controls, and explicit evidence limits.

`xtask/tests/f02_track_album_edit_oracle.rs` independently decodes the output with test-only `quick-xml` and compares the whole decoded tree against the frozen source with only target Album changed. Focused regression passed (1/1); source fixture bytes remain unchanged.

Synthetic evidence only. Global F-09 remains NOT DEMONSTRATED; no Rekordbox compatibility or ADR-005 qualification is claimed. No private library data or audio was used.
