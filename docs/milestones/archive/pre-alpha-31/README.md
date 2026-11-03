# Pre-alpha 31 — guarded raw TRACK.SampleRate F-09 evidence

Two retrospective Chromium renders of the bounded synthetic post-edit oracle:

- `01-sample-rate-edit-oracle.png` — exact-old raw SampleRate edit and independent whole-tree comparison.
- `02-sample-rate-scope-and-controls.png` — field-presence, mutation controls, and scope boundary.

The renders are illustrative retrospective evidence cards, not application screenshots or vendor validation. Executable evidence is `xtask/tests/f02_track_sample_rate_edit_oracle.rs` against frozen synthetic input `xtask/tests/fixtures/f02-track-sample-rate-edit/source.xml`. SampleRate remains uninterpreted raw text: no numeric parsing, Hz interpretation, or conversion. One synthetic F-02 case does not establish global F-09, Rekordbox compatibility, or ADR-005 qualification. No private library data or audio was used.
