# Pre-alpha 30 — guarded raw TRACK.BitRate F-09 evidence

Two retrospective Chromium renders of the bounded synthetic post-edit oracle:

- `01-bitrate-edit-oracle.png` — exact-old raw BitRate edit and independent whole-tree comparison.
- `02-bitrate-scope-and-controls.png` — field-presence, mutation controls, and scope boundary.

The renders are illustrative retrospective evidence cards, not application screenshots or vendor validation. Executable evidence is `xtask/tests/f02_track_bit_rate_edit_oracle.rs` against frozen synthetic input `xtask/tests/fixtures/f02-track-bitrate-edit/source.xml`. BitRate remains uninterpreted raw text: no numeric parsing, units, or conversion. One synthetic F-02 case does not establish global F-09, Rekordbox compatibility, or ADR-005 qualification. No private library data or audio was used.
