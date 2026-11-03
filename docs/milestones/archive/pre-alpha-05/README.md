# Pre-alpha 05 — F-09 TEMPO Inizio oracle

Captured 2026-10-28 as Chromium-rendered evidence cards after the focused regression.

- `01-inizio-oracle.png` summarizes the bounded A1 F-07 TrackID 700 TEMPO ordinal 1 `Inizio` `10`→`11` result.
- `02-scope-and-controls.png` records stale/wrong-ordinal and semantic mutation controls, plus explicit evidence limits.

The test-only `quick-xml` oracle compares the complete decoded output tree against the frozen A1 source with only the selected `Inizio` changed. It asserts the original fixture bytes remain unchanged. The focused regression passed (1/1). Final `cargo xtask ci-full` passed (exit 0; `ci=ok test=ok deny=ok`). Cargo-deny emitted its existing five `license-not-encountered` allowlist warnings; the license gate passed.

These rendered cards document synthetic evidence only. They do not establish Rekordbox/OneLibrary compatibility, global F-09 completion, beat-grid interpolation/coupling, or ADR-005 qualification. No private library data or audio was used.
