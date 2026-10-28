# Pre-alpha 20 — F-09 guarded TRACK.Colour oracle

Two retrospective Chromium-rendered evidence cards document one bounded synthetic F-02 post-edit oracle:

- `01-colour-edit-oracle.png` records TrackID 100 raw Colour `old & palette` → `new < palette` and absent Colour on TrackID 200.
- `02-scope-and-controls.png` records whole-tree comparison, unchanged source bytes, stale/wrong-target refusal, semantic mutation controls, and evidence limits.

The `.html` files are render sources; these are retrospective renders, not captures from the original test run. Focused test: `cargo test -p xtask --test f02_track_colour_edit_oracle` (1 passed). Colour remains uninterpreted raw text; no palette/index mapping. This is one synthetic case only; global F-09, Rekordbox compatibility, and ADR-005 qualification remain NOT DEMONSTRATED. No private data was used.
