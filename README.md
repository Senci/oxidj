# OxiDJ

OxiDJ is a modern, open-source DJ library manager and, eventually, performance application written primarily in Rust. Its long-term goal is a credible Rekordbox alternative without locking users into one vendor ecosystem (idea.md). It is **not** a working DJ tool today: no library management, analysis, playback, or GUI exists yet. The project is in M0 — research and architecture.

## Status

M0. No DJ library-management or performance features exist. The repository contains a minimal Rust workspace and the architecture evidence base that produced the accepted ADRs, but no product beyond a single domain type (plan/STATUS.md, docs/architecture.md).

**What exists today** (Cargo.toml, crates/oxidj-core/src/lib.rs, crates/oxidj-cli/src/main.rs, xtask/src/main.rs):

- `oxidj-core` — a domain `TrackId` type that enforces the invariant that `0` is reserved and can never exist at rest. Unit, integration, and doctests pass under the gate (docs/testing.md).
- `oxidj-cli` — a thin command-line entry point exposing one operation (`track-id <n>`).
- `xtask` — a repo-owned quality gate: `cargo xtask ci` (Tier 1 per-handoff gate) and `cargo xtask ci-full` (Tier 1 + tests + licence gate, used by CI).
- Six accepted ADRs (GUI, persistence, metadata, crate boundaries, audio, licence) and the architecture baseline (docs/adr/index.md, docs/architecture.md).

M1 (library core) is gated on a verified M0 checklist (plan/STATUS.md).

**Deferred** — do not expect these yet (idea.md MVP §Deferred; docs/architecture.md §11; plan/MVP.md):

- M1: DJ metadata schema (ADR-005 pending); persistence layer (`rusqlite` stays out of the workspace until a 500k-track prototype passes — ADR-002).
- M2: filesystem import (plan/MVP.md).
- M3: usable library UI — prototype direction only (egui/eframe, ADR-001), not committed to production.
- M4: OneLibrary/Rekordbox interoperability — requires empirical validation of undocumented formats (interoperability.md §Fixture D).
- M4+: realtime DJ functionality — decks, mixing, effects, stems, streaming, broad controller support (idea.md §Priorities/MVP).

The first major product proof — the MVP (idea.md) — is library-first: import a real DJ library, manage it safely in OxiDJ, export it again, and continue using compatible DJ software/hardware without unexpected metadata loss.

## Documentation

- [docs/index.md](docs/index.md) — every document in the repository, grouped by purpose.
- [docs/glossary.md](docs/glossary.md) — DJ and domain vocabulary (beatgrid, cues, loops, provenance, vendor formats).
- [site/README.md](site/README.md) — the "FILE #0042" teaser site: what it is, and how to build and test it locally.
- [plan/STATUS.md](plan/STATUS.md) — current milestone, blockers, and next tasks.
- [AGENTS.md](AGENTS.md) — rules for agent contributors.

## Toolchain and quality gate

Toolchain: **Rust stable 1.99.0**, pinned by `rust-toolchain.toml` (compiler pin, not an MSRV declaration — see docs/development.md §Setup).

```sh
cargo xtask ci        # Tier 1: layers + fmt + clippy + check (every handoff must pass)
cargo xtask ci-full   # Tier 1 + tests + licence gate (what CI runs)
```

A fresh clone reproduces the gate with one command:

```sh
git clone <repo> && cd oxidj && cargo xtask ci
```

The gate is tiered: Tier 1 (stable, fast; no advisory scans) on every coding handoff; Tier 2 on CI/pre-merge; Tier 3 analyses are deferred and not yet wired to an in-repository schedule. Every handoff must paste real Tier-1 output (`docs/handoff-checklist.md`); current commands live in `cargo xtask` and `.github/workflows/ci.yml`.

## Licence

**MIT.** Copyright (c) 2026 Senad Ličina. See [LICENSE](LICENSE). The licence is decided by ADR-009 and enforced as a build gate: `cargo deny check licenses` with an allowlist of permissive licenses (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unicode-DFS-2016, Zlib) and GPL/LGPL/AGPL/SSPL denied by default (deny.toml).
