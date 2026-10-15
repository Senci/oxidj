# Test strategy (M0-08)

M0 ships a **minimal** test suite that proves the workspace builds, the domain
rules hold, and the gate runs. This document is the strategy the project follows
as M1–M4 add fixtures, persistence, parsers and analysis. It states requirements
for later implementation; it does **not** claim those tests exist yet.

## What M0 ships (proof, not a product suite)

- **Unit tests** (`oxidj-core` `mod tests`): pure domain rules (`TrackId`
  identity invariant). Cheap, deterministic, run on every handoff.
- **Integration tests** (`oxidj-core/tests/track_id.rs`): the public API the CLI
  and later GUI/adapters will consume. Exercises rules from outside the crate.
- **Doctests** (`oxidj-core` lib docs): keeps promised usage executable; a
  doctest that cannot compile is a broken promise.
- **CLI unit tests** (`oxidj-cli`): dispatch exit codes (`run`), so the CLI
  contract is testable without spawning a process.

## Requirements for M1+ (when the code exists)

- **Unit tests cover pure rules.** Domain invariants (identity, provenance,
  ordering, beat-grid/cue/tempo rules) are unit-tested independently of I/O.
- **Integration tests cover persistence / adapters / public API.** Reopen,
  migrations, transactions and interrupted writes are tested with real
  per-test temporary files/databases, never a shared or real user library.
- **Doctests keep the promised public API executable** — `cargo test --doc` is
  part of the gate (doctests are NOT covered by `--all-targets`; they run as
  their own target).

### Fixtures

- Store **small, redistributable or synthetic** fixtures with provenance:
  source, licence/permission, format-version, expected semantics, and any known
  loss. Never commit private music, credentials or personal library
  identifiers. Preserve immutable raw evidence separately from expected results.
- Resolve fixture paths **relative to the package manifest**, not a presumed
  repo-root cwd (per `cargo test` package-root cwd semantics).
- Keep fixtures deterministic; tests run concurrently and must not share mutable
  cwd, environment, caches or a user's real library.

### Semantic round-trips

- Import → edit → export → independent re-import checks compare **independent
  semantic expectations** for cues / loops / beatgrid / key / BPM / unknown
  metadata / ordering — not just bytes, and not an importer and exporter that
  can share the same mistake. Record intentional loss explicitly.
- This is the heart of the MVP (no unexpected metadata loss), so it is
  decision-bearing and must be wired as the first real integration suite.

### Fuzzing / property / performance

- Justify fuzzing (e.g. `cargo-fuzz`) and property testing (e.g. `proptest`)
  only once parsers, invariants and measured hot paths exist — **not** as
  bootstrap dependencies. At format boundaries, property tests should fuzz
  hostile / malformed input and assert the error is reported, not that the
  process survives silently.
- Performance tests (criterion) are added once a measured hot path exists
  (e.g. 500k-track import), with a CI budget.

## How the strategy is enforced

The gate runs the tests that exist today (`cargo xtask test` →
`cargo test --workspace --all-features --locked`), and CI runs the same
repo-owned command. As crates and fixtures arrive, the same gate covers the new
tests without any new per-crate command surface — the workspace shares one
lockfile and one gate.
