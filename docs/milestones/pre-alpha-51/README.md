# Pre-alpha 51 — fail-closed POSITION_MARK Num parsing

Two screenshot-style milestone cards document the present-empty `POSITION_MARK Num` rejection in the synthetic F-07 CLI preview. The regression verifies a clear diagnostic, nonzero exit, and unchanged input bytes.

The parser boundary is only the existing Rust `u32` validation. No cue-index interpretation, numbering, ordering, renumbering, or vendor behavior is inferred. Synthetic-only; no private library data, Rekordbox/OneLibrary compatibility, global F-09, or ADR-005 qualification claim. These images are retrospective documentation, not captures of Rekordbox or a production OxiDJ UI.
