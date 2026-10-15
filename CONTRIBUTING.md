# Contributing to OxiDJ

OxiDJ is in M0 (research and architecture). Before contributing, read
`idea.md` for the project vision and `docs/architecture.md` for the current
design direction. This guide covers the prerequisite toolchain, the tiered
quality gate, commit/PR expectations, and the licence situation.

## Prerequisite toolchain

OxiDJ targets **Rust stable 1.99.0**, pinned by `rust-toolchain.toml`
(`channel = "1.99.0"`, `profile = "minimal"`, components
`rustfmt` / `clippy` / `rust-analyzer` / `rust-src`). This is a **compiler pin
for reproducibility**, not an MSRV assertion: the declared MSRV lives as
`rust-version = "1.99.0"` in the workspace `Cargo.toml`, with `msrv = "1.99.0"`
in `clippy.toml` (docs/development.md §Setup; docs/research/rust-quality-gates.md §0.4, §7).

On a fresh machine, install rustup and components (docs/development.md §Setup):

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://sh.rustup.rs -o rustup-init.sh
sh rustup-init.sh -y --profile minimal --default-toolchain stable
. "$HOME/.cargo/env"
rustup component add --toolchain stable rustfmt clippy rust-analyzer rust-src
```

`rust-toolchain.toml` makes `rustup` install the exact 1.99.0 toolchain
automatically on the first `cargo` invocation — no separate install step needed
(docs/handoff-checklist.md §Fresh clone). A native C compiler/linker is required
for build-dependencies; `cc --version` succeeded on the reference host
(docs/development.md §Setup).

## The tiered quality gate

The repo-owned gate is `cargo xtask` (xtask/src/main.rs). Authoritative tier
definitions are in `docs/research/rust-quality-gates.md §7`; the required
evidence format for handoffs is in `docs/handoff-checklist.md`.

- **Tier 1 — every coding handoff (stable, offline, fast).**
  `cargo xtask ci` runs `layers` → `fmt` → `clippy` → `check`. All four must
  report `PASS`. This **must pass** before a task is marked complete.
- **Tier 2 — before requesting review (CI / pre-merge).**
  `cargo xtask test` (all tests pass), `cargo machete --json` (no unused deps),
  `typos --format json` (no findings), `cargo audit` (no advisories), and the
  licence gate `cargo deny check licenses` (ADR-009).
- **Tier 3 — nightly / scheduled / release only (not on every handoff).**
  `cargo +nightly miri test`, `cargo +nightly udeps`,
  `cargo +nightly semver-checks check-release`, `cargo mutants`,
  `cargo hack`, `cargo bloat`, `cargo vet`, `cargo outdated`.

A pull request must pass **Tier 1** (the handoff gate) and **Tier 2** (tests
and the licence gate before merge). CI runs `cargo xtask ci-full`
(Tier 1 + tests + `cargo deny check licenses`) on every push and pull request
to `master` (`.github/workflows/ci.yml`). Tier 3 runs on a scheduled nightly
against a pinned nightly toolchain — never on the stable handoff host
(docs/handoff-checklist.md §Tier 3; docs/research/rust-quality-gates.md §7).

> Tier 1 is **offline by design** (rust-quality-gates.md §7 item 3): network-
> dependent tools never run on the handoff path, so a worker does not fail
> because the network is unavailable.

## Commits and pull requests

- Work in an **isolated Git worktree** — one task per branch, one worker per
  profile (AGENTS.md).
- Run `cargo xtask ci` locally and paste the output before requesting review
  (docs/handoff-checklist.md). Do not paste output you did not run.
- A **fresh clone reproduces** your result with a single command
  (`git clone <repo> && cd oxidj && cargo xtask ci`) — the gate is
  self-documenting, not per-worker config (docs/handoff-checklist.md §Fresh
  clone).
- PRs target `master`; CI runs `cargo xtask ci-full`.
- Do not commit credentials or secrets (AGENTS.md; `.gitignore` covers
  `.env*` and `/target/`).
- The repository does not enforce a commit-message format; write a clear
  narrative, one focused concern per PR.

## Licence

OxiDJ is **MIT-licensed** (ADR-009). Copyright (c) 2026 Senad Ličina; see
`LICENSE` and `deny.toml`.

- **Contributions are made under the MIT licence** (same as the project).
- A **permissive-only dependency policy is enforced by the build gate**:
  `cargo deny check licenses` (`cargo xtask deny` / `deny.toml`) allows only
  MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2016, and Zlib;
  GPL / LGPL / AGPL / SSPL are denied by default. This is a **build gate, not a
  convention** — an accidental copyleft dependency would silently change the
  licence of the distributed binary (ADR-009; Cargo.toml comment; deny.toml).
- The project may later **dual-license as `MIT OR Apache-2.0`** (Apache-2.0
  adds a patent grant some corporate contributors require). This is a one-line
  change now and requires a contributor agreement once outside contributions
  exist. It is noted but not acted on (ADR-009, "Open question").
