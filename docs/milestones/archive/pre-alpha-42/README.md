# Pre-alpha 42 — raw TRACK.Kind no-edit export preservation

Two retrospective Chromium-rendered evidence cards archive this bounded synthetic CLI regression:

- `01-kind-no-edit.png` — raw Kind present, empty, and absent states.
- `02-scope-and-proof.png` — exact-byte checks and explicit evidence limits.

Executable evidence is `crates/oxidj-cli/tests/rekordbox_preview.rs`, test `no_edit_export_preserves_raw_kind_present_empty_and_absent_bytes`. It checks byte-identical output and unchanged input using synthetic F-02 XML only. Kind stays uninterpreted raw text; this does not demonstrate format interpretation, Rekordbox compatibility, global F-09, or ADR-005 qualification. No private library data or audio was used.
