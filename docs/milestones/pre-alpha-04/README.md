# Pre-alpha 04 — F-09 Metro edit oracle

Captured 2026-10-28 as Chromium-rendered evidence cards after the focused regression and full repository gate.

- `01-metro-oracle.png` summarizes the bounded A1 F-07 TrackID 700 TEMPO ordinal 0 Metro `4/4`→`3/4` oracle result.
- `02-scope-and-gate.png` records negative-control coverage, the full CI gate summary, and explicit qualification limits.

The independent test-only `quick-xml` oracle compares the complete decoded tree against the frozen A1 F-07 source with only target Metro changed. It asserts source-byte preservation, exact old-value guarding, and rejects wrong ordinal, stale values and semantic mutations. Focused test passed (1/1); `cargo xtask ci-full` passed (`ci=ok test=ok deny=ok`).

These rendered evidence cards document synthetic evidence only. They do not establish Rekordbox/OneLibrary compatibility, global F-09 completion, meter/beat coupling, or ADR-005 qualification. No private library data or audio was used.
