# Pre-alpha 23 — guarded raw TRACK.LastPlayed F-09 evidence

Two retrospective Chromium renders document the bounded synthetic F-02 post-edit oracle:

- `01-lastplayed-edit-oracle.png` — exact-old raw LastPlayed edit and independent whole-tree check.
- `02-scope-and-controls.png` — frozen source, absent-state track, mutation controls, and explicit scope limits.

The source fixture and test are `xtask/tests/fixtures/f02-track-lastplayed-edit/source.xml` and `xtask/tests/f02_track_last_played_edit_oracle.rs`. These are retrospective visual summaries of committed test evidence, not runtime captures or Rekordbox application evidence. LastPlayed remains raw; no date/timezone conversion is tested or claimed. One synthetic case only; global F-09, vendor compatibility, and ADR-005 qualification remain NOT DEMONSTRATED.
