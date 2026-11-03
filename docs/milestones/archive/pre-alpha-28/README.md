# Pre-alpha 28 — guarded raw TRACK.Size F-09 evidence

Two retrospective Chromium renders of the bounded synthetic post-edit oracle:

- `01-size-edit-oracle.png` — exact-old raw Size edit and independent whole-tree comparison.
- `02-size-scope-and-controls.png` — field-presence, mutation controls, and scope boundary.

The renders are illustrative retrospective evidence cards, not application screenshots or vendor validation. The executable evidence is `xtask/tests/f02_track_size_edit_oracle.rs` against `xtask/tests/fixtures/f02-track-size-edit/source.xml`. This one synthetic F-02 case does not establish global F-09, Rekordbox compatibility, or ADR-005 qualification. Size is uninterpreted raw text; no numeric parsing, units, or conversion.