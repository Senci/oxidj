# Pre-alpha 51 — fail-closed POSITION_MARK Num parsing

Two screenshot-style milestone cards document the present-empty `POSITION_MARK Num` rejection in the synthetic F-07 CLI preview. The regression verifies a clear diagnostic, nonzero exit, and unchanged input bytes.

The parser boundary is only the existing Rust `u32` validation. No cue-index interpretation, numbering, ordering, renumbering, or vendor behavior is inferred. Synthetic-only; no private library data, Rekordbox/OneLibrary compatibility, global F-09, or ADR-005 qualification claim. These images are retrospective documentation, not captures of Rekordbox or a production OxiDJ UI.

## B1/B2 logical snapshots — synthetic-only

`b1-boundary.png` and `b2-boundary.png` illustrate hand-authored logical snapshots; they are not runtime captures, physical backups, restore evidence, or qualification results. See `xtask/tests/fixtures/b1-b2-logical/` and its independent checker. No physical restore/recovery, C6 IDs, vendor compatibility, or qualification claim is made.
