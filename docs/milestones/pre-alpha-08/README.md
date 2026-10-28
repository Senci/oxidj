# Pre-alpha 08 — F-09 guarded TRACK.Comments oracle

Captured 2026-10-28 as Chromium-rendered evidence cards for one synthetic F-02 post-edit oracle.

- `01-comments-edit-oracle.png` summarizes the frozen TrackID 100 `old & note`→`new < note` edit and absent Comments state on TrackID 200.
- `02-scope-and-controls.png` records stale/wrong-target refusal, semantic mutation controls, and explicit evidence limits.

`xtask/tests/f02_track_comments_edit_oracle.rs` uses a test-only `quick-xml` tree reader, not the production importer, to compare the entire decoded output tree with a source-derived expected tree that changes only target Comments. The focused regression passed (1/1); source fixture bytes remain unchanged.

These cards document synthetic evidence only. Global F-09 remains NOT DEMONSTRATED; no Rekordbox application compatibility or ADR-005 qualification is claimed. No private library data or audio was used.
