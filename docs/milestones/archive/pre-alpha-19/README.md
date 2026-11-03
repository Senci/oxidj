# Pre-alpha 19 — F-09 guarded TRACK.Rating oracle

Two retrospective Chromium-rendered evidence cards document one bounded synthetic F-02 post-edit oracle:

- `01-rating-edit-oracle.png` records TrackID 100 raw Rating `old & stars` → `new < stars` and absent Rating on TrackID 200.
- `02-scope-and-controls.png` records the independent whole-tree check, source immutability, stale/wrong-target refusal, semantic mutation controls, and evidence limits.

The matching `.html` files are render sources; these are retrospective renders, not captures from the original test run. The focused test `xtask/tests/f02_track_rating_edit_oracle.rs` runs the existing guarded adapter and independently decodes the output with test-only `quick-xml`; it passed (1/1).

Synthetic evidence only. Rating remains uninterpreted raw text; no star mapping, global F-09, Rekordbox compatibility, or ADR-005 qualification is claimed. No private library data was used.