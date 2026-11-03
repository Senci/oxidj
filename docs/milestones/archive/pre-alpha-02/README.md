# Pre-alpha 02 milestone screenshots

## ADR-005 F-09 bounded position-mark edit oracles — 2026-10-28

All captures show real command output in the isolated Bot Screen. Tests use
frozen synthetic XML only; no personal library data or audio was opened.

- `01-position-mark-start-oracle.png` — focused independent Start-edit oracle
  passed (1 test).
- `02-position-mark-end-and-ci-full.png` — focused independent End-edit oracle
  passed (1 test); `cargo xtask ci-full` finished with
  `ci=ok test=ok deny=ok`.
- `03-position-mark-name-oracle.png` — focused independent Name-edit oracle
  passed (1 test).

These records document synthetic absolute-time edit preservation only. They do
not prove Rekordbox/OneLibrary compatibility, global F-09 completion, beat lock,
or qualification. Vendor and qualification limits remain open in ADR-005.
