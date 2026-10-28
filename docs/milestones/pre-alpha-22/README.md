# Pre-alpha 22 — F-09 guarded TRACK.DateModified oracle

Two retrospective Chromium-rendered evidence cards document one bounded synthetic F-02 post-edit oracle:

- `01-datemodified-edit-oracle.png` records TrackID 100 raw DateModified `old & stamp` → `new < stamp` and absent DateModified on TrackID 200.
- `02-scope-and-controls.png` records exact-old guard, test-only quick-xml whole-tree comparison, unchanged source bytes, stale/wrong-target refusal, semantic mutation controls, and evidence limits.

The `.html` files are render sources; these are retrospective renders, not captures from the original test run. Focused test: `cargo test -p xtask --test f02_track_date_modified_edit_oracle` (1 passed). DateModified remains uninterpreted raw text; no date/timezone conversion. This is one synthetic case only; global F-09, Rekordbox compatibility, and ADR-005 qualification remain NOT DEMONSTRATED. No private data was used.
