# Dependency update inventory — 2026-11-14

**Implementation progress, not full completion.** Latest-stable exceptions
remain open; no installation or push is authorized by this
record. Registry versions were rechecked against crates.io and npm on
2026-11-14; the git dependency was checked with `git ls-remote`.

## Scope and direct Rust dependencies

Tracked-manifest discovery found 14 `Cargo.toml` files and `site/package.json`,
covering the root workspace, isolated OneLibrary reader and both prototypes.
No tracked `pyproject.toml`, `requirements.txt` or `go.mod` was found. Internal
path/workspace dependencies are excluded. No prerelease was selected.

| Dependency | Before | Latest stable observed | Current implementation / exception |
|---|---|---|---|
| claxon | 0.4.3 | 0.4.3 | unchanged |
| cpal | 0.18.2 | 0.18.2 | unchanged |
| eframe | 0.32.3 | 0.37.0 | both prototypes migrated; package-scoped license exceptions, see below |
| hound | 3.5.1 | 3.5.1 | unchanged |
| image | 0.25.8 | 0.25.10 | updated library and UI |
| lofty | 0.25.5 | 0.25.5 | unchanged |
| nanomp3 | 0.2.0 | 0.2.0 | unchanged |
| quick-xml | 0.41.0 | 0.42.0 | all direct uses updated: adapter, CLI tests, xtask and library UI |
| raw-window-handle | 0.6.2 | 0.6.2 | unchanged |
| rtrb | 0.4.1 | 0.4.1 | unchanged |
| rusqlite | 0.40.2 | 0.42.0 | root/UI resolve 0.41.1; reader retains 0.40.2; two exact exceptions below |
| rusqlite_migration | 2.6.0 | 2.7.0 | updated |
| serde | 1.0.229 | 1.0.229 | unchanged |
| serde_json | 1.0.154 | 1.0.154 | unchanged |
| sha2 | 0.10.9 | 0.11.1 | library, UI and xtask updated |
| signalsmith-stretch | 0.1.3 | 0.1.3 | unchanged |
| taktus | 0.1.0 | 0.1.0 | unchanged |
| wayland-client | 0.31.15 | 0.31.15 | unchanged |
| wayland-protocols | 0.32.13 | 0.32.13 | unchanged |
| zeroize | 1.8.2 | 1.9.1 | updated |
| rekordbox-pdb git | b9896c4 | HEAD b9896c4 | already current HEAD; latest tag v0.1.0 is older |

Rust graph lockfiles are committed separately for root, reader and each
prototype. Eframe's transitive dependencies still include quick-xml 0.41.0;
this is not an unconverted OxiDJ direct dependency or a claim that every
transitive crate can use its latest breaking release.

### Precise latest-stable exceptions

- Root/library UI `rusqlite` resolves **0.41.1**, not 0.42.0:
  `rusqlite_migration` 2.7.0 (latest stable) requires `rusqlite ^0.41.0`.
  Upgrading only our direct requirement introduces incompatible connection
  types and conflicting SQLite `links` graphs. Do not fork the migration
  library or replace catalog migration behavior solely to bump this version.
  Resume when a stable migration release supports rusqlite 0.42.
- Isolated reader `rusqlite` remains **0.40.2**. The attempted 0.42.0 graph
  compiled and passed its 13 tests, but introduces `sqlcipher-src`
  420.0.0+sqlcipher-4.20.0-sqlite-3.54.1-libtomcrypt-1.18.2, declaring
  `MIT AND BSD-3-Clause AND blessing AND WTFPL`. The unchanged allowlist
  rejects `blessing` and `WTFPL`. Only this version bump was restored to the
  admitted graph; the reader boundary/version guard was not weakened.
  The scoped technical decision retains the admitted graph without a new
  license exception. Revisit if a compliant upgrade becomes available or the
  owner changes policy; update boundary pins/tests together, preserving SQLCipher isolation.
- Site **TypeScript 6.0.3**, not latest stable 7.0.2:
  latest `typescript-eslint` 8.72.2 declares peer `>=4.8.4 <6.1.0`;
  the prior 7.0.2 attempt failed lint. Keep the supported compiler/parser pair
  until a stable parser supports TypeScript 7; do not suppress the gate.

## Site Bun dependencies

| Direct package | Before | Latest stable observed | Current |
|---|---|---|---|
| @eslint/js | 9.39.5 | 10.0.1 | 10.0.1 |
| @types/node | 22.20.5 | 26.6.6 | 26.6.6 |
| @vitest/coverage-v8 | 3.2.7 | 5.1.0 | 5.1.0 |
| eslint | 9.39.5 | 10.15.0 | 10.15.0 |
| eslint-config-prettier | 9.1.2 | 10.1.8 | 10.1.8 |
| happy-dom | 21.0.4 | 21.0.4 | 21.0.4 |
| prettier | 3.9.10 | 3.10.1 | 3.10.1 |
| typescript | 5.9.3 | 7.0.2 | 6.0.3 — parser compatibility exception |
| typescript-eslint | 8.71.2 | 8.72.2 | 8.72.2 |
| vite | 6.4.4 | 8.3.8 | 8.3.8 |
| vitest | 3.2.7 | 5.1.0 | 5.1.0 |

`site/bun.lock` was refreshed. `bun install --frozen-lockfile` succeeds without
changes. Vite warns that future native config loading will not accept current
CommonJS-loaded TypeScript config/module syntax; current tests/build succeed.

## Eframe and quick-xml API adaptations

Both eframe apps implement the 0.37 `App::ui` entry point and use the root UI
for panel layout. The library UI migration updates IDs, AccessKit roles and
requests, storage removal, styles, window outer geometry, scroll offsets and
wheel-event handling. The tempo slider retains CDJ direction (down = faster)
and exposes ordered numeric AccessKit bounds/actions. Headless tests explicitly
consume texture deltas because they have no renderer; gesture/keyboard,
settings, import/export and independent XML-oracle assertions remain exercised.

Quick-xml 0.42 exposes decoded names/values. Direct call sites use the new
normalization API and explicit UTF-8 byte views where byte offsets/comparisons
are required. Source-based checker identity/provenance pins were refreshed only
for modified checker source; frozen vendor fixtures were not replaced.

## License admission — scoped exceptions

Both eframe 0.37 prototype graphs initially failed explicit standalone checks
under the previous `deny.toml`:

- `clipboard-win` 5.4.3 and `error-code` 3.4.0: **BSL-1.0**, via
  eframe → egui-winit → arboard (Windows clipboard).
- `epaint_default_fonts` 0.37.0: **OFL-1.1** and **Ubuntu-font-1.0**
  alongside MIT/Apache-2.0, via epaint default fonts.

The owner approved exceptions for precisely these three package names on
2026-11-14. The global license allowlist remains unchanged; clipboard and fonts
remain enabled. Both standalone graphs pass `cargo deny --locked --offline
--config deny.toml --manifest-path <prototype>/Cargo.toml check licenses` with
the scoped exceptions. This does not approve the reader's distinct SQLCipher
licenses or constitute independent review or installation approval.

## Wayland hidden-window fix

Upstream [egui PR #8631](https://github.com/emilk/egui/pull/8631), the relevant
hidden-window ANR fix, is included in eframe **0.37.0**. Its changelog lists
#8631; downloaded source contains `window.pre_present_notify()`, the Wayland
swap-interval change and redraw deadlines. Both migrated OxiDJ manifests now
select that release (winit 0.30.13). Separate PR #8660/issue #8776 concern other
suspended-window/hidden-viewport work; their status does not negate #8631.
**Isolated synthetic OxiDJ Wayland hide/show: PASS for both old and new
executables; original warning NOT REPRODUCED and causal fix UNPROVEN.**
The [runtime investigation](hidden-workspace-investigation.md) records real
workspace switching, automatic ping/pong, restored surface commits and a
stopped-owned-child negative control, plus the nested-compositor limitations.
No owner app or personal database was used for these checks.

## Verification and remaining handoff

2026-11-14 runs against the migrated tree:

- `cargo xtask ci`: exit 0,
  `=> ci summary: layers=ok fmt=ok clippy=ok check=ok  [all pass]`.
- `cargo xtask ci-full`: exit 0,
  `=> ci-full summary: ci=ok test=ok ui-test=ok deny=ok  [all pass]`.
  **Scope warning:** its `deny` step covers the root graph; it does not imply
  either standalone eframe graph passed its separate license check.
- Library UI: **351 passed, 0 failed, 2 ignored** (existing timing reports).
- `cargo test -p oxidj-rekordbox -p xtask --offline --quiet`: exit 0,
  including adapter 79 tests and all checker/inventory/provenance suites.
- Isolated reader, final retained graph: **13 passed**, exit 0.
- GUI probe: **14 passed**, exit 0.
- Site frozen install, typecheck, lint, **79 tests**, and four-route build/
  verifier: exit 0.
- Standalone prototype `cargo fmt -- --check`: exit 0. Additional strict
  `cargo clippy --all-targets --all-features -- -D warnings` probes failed
  (probe 4 diagnostics; UI 68 diagnostics, including existing style lints and
  migration-related needless borrows). These are not part of the root gate;
  they are not reported as passing or suppressed.
- Standalone license checks initially exited 4 for both eframe graphs; after
  the owner-approved package-scoped exceptions, both exit 0 (`licenses ok`).
  The attempted reader 0.42 graph exited 4; the retained reader graph passes
  the repository's layers/license boundary check.

Independent Sol review of the implemented migration at `b7d0195` approved its
code scope; it did not approve completion of the all-latest task. Its `ci-full`
exited 0, with UI 352 passed/2 ignored, reader 13, probe 14 and site 80 tests;
both prototype license checks passed. Three latest-stable exceptions remain.
At that dependency-review handoff, Wayland runtime was NOT TESTED and the
all-latest task was **not complete**. Subsequent bounded runtime evidence is
recorded [above](#wayland-hidden-window-fix); it does not establish a causal fix.
No push or installation was performed by that dependency handoff.
