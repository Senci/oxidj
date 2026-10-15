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

The repo-owned gate is `cargo xtask` (xtask/src/main.rs). Current tier configuration is defined by `AGENTS.md` and `.github/workflows/ci.yml`; the research gate report is background, not current implementation. The required evidence format for handoffs is in `docs/handoff-checklist.md`.

- **Tier 1 — every coding handoff (stable, fast).**
  `cargo xtask ci` runs `layers` → `fmt` → `clippy` → `check`. All four must
  report `PASS`. This **must pass** before a task is marked complete.
- **Tier 2 — required CI / pre-merge gate.** GitHub CI runs `cargo xtask ci-full`, including the workspace tests and mandatory `cargo deny check licenses` licence gate. The workflow installs `cargo-deny` 0.20.2 before the full gate.
- **Optional advisory checks.** CI attempts `cargo audit`, `cargo machete --json`, and `typos --format json` as best-effort checks with `continue-on-error`; missing local binaries are not a handoff failure. The current gate uses native `cargo test`, not `cargo-nextest`.
- **Tier 3 — deferred analyses (not configured in repo CI).** Candidate slow/nightly checks include `cargo +nightly miri test`, `cargo +nightly udeps`, `cargo +nightly semver-checks check-release`, `cargo mutants`, `cargo hack`, `cargo bloat`, `cargo vet`, and `cargo outdated`.

A pull request must pass **Tier 1** (the local handoff gate) and the required
`cargo xtask ci-full` CI run (tests + licence gate); best-effort advisory checks
are reported separately and do not block the merge. The only workflow currently
in `.github/workflows` runs on push and pull request; no Tier-3 schedule is
configured. If Tier 3 is later approved and scheduled, use a pinned nightly and
keep those checks off the stable handoff path.

> Tier 1 is **stable-only and fast**. It does not query remote advisory
> databases; a fresh Cargo cache may still fetch locked crate dependencies, so
> do not describe the command as guaranteed network-independent.

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
