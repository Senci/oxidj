# Pre-alpha 03 — F-09 Battito edit oracle

Captured 2026-10-28 from the isolated Bot Screen after commit `17bbd35`.

- `01-battito-oracle.png` shows the bounded A1 F-07 `Battito` edit and verified gate result.
- `02-scope-and-limits.png` records Sol's independent approval and the compatibility/qualification limits.

The independent test-only `quick-xml` oracle checks one guarded edit on frozen A1 F-07 input: TrackID 700, TEMPO ordinal 0, `Battito` 1→2. It verifies the complete decoded tree, unchanged source bytes, and rejection controls. Focused test passed (1/1); `cargo xtask ci-full` passed (`ci=ok test=ok deny=ok`).

These screenshots document synthetic evidence only. They do not establish Rekordbox/OneLibrary compatibility, global F-09 completion, beat-grid coupling, or ADR-005 qualification. The F-10 third-tuple meter/beat disagreement remains a separate owner decision. No private library data or audio was opened.
