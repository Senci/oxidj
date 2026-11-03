# Pre-alpha 12 — F-09 guarded TRACK.Tonality oracle

Archived two Chromium-rendered evidence cards for one bounded synthetic F-02 post-edit oracle.

- `01-tonality-edit-oracle.png` records the frozen TrackID 100 Tonality `old & key`→`new < key` edit and the absent Tonality state on TrackID 200.
- `02-scope-and-controls.png` records exact-old refusal controls, independent whole-tree comparison, semantic mutation controls, and evidence limits.

`xtask/tests/f02_track_tonality_edit_oracle.rs` applies the existing guarded adapter, decodes with test-only `quick-xml`, and compares the entire decoded tree with only target Tonality changed. The focused characterization test passed (1/1); source bytes remain unchanged.

Synthetic evidence only. Key notation/detection/normalization semantics and global F-09 remain NOT DEMONSTRATED; no Rekordbox compatibility or ADR-005 qualification is claimed. No private library data or audio was used.
