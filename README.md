# OxiDJ

OxiDJ is a modern, open-source DJ library manager and, eventually, performance application written primarily in Rust. Its long-term goal is a credible Rekordbox alternative without locking users into one vendor ecosystem (idea.md). The bounded local reference catalog exists as an implementation candidate; import, analysis, playback, and GUI workflows remain absent.

## Status

M0 is verified complete. The named `local-catalog-alpha-v1` dependency exception is integrated by PR #13 (`671ae2b3df933ada97297bbba19ea1394657c14e`) for the reference-only, single-process, local-filesystem catalog capped at 10,000 tracks. A local implementation candidate now initializes and lists a versioned catalog, stores reference-only records through `oxidj-library`, and demonstrates fresh-process listing. Independent acceptance and full M1 remain incomplete (plan/STATUS.md). The separate representative 500k qualification remains NOT READY and unpassed.

**What exists today** (Cargo.toml, crates/oxidj-core/src/lib.rs, crates/oxidj-library/src/lib.rs, crates/oxidj-cli/src/main.rs, xtask/src/main.rs):

- `oxidj-core` — a domain `TrackId` type that reserves `0`. Unit, integration, and doctests pass under the gate (docs/testing.md).
- `oxidj-library` — local SQLite catalog, versioned schema, reference-record handlers, operation-ID replay protection, and a 10,000-track cap.
- `oxidj-cli` — catalog commands, read-only Rekordbox XML preview and export preflight, plus bounded no-edit, tempo-edit, and position-mark edit exports.
- `oxidj-rekordbox` — isolated adapter for a strict Rekordbox XML subset, tested against synthetic A1 fixtures; preflight rejects unsupported input and never writes.
- `xtask` — a repo-owned quality gate: `cargo xtask ci` (Tier 1 per-handoff gate) and `cargo xtask ci-full` (Tier 1 + tests + licence gate, used by CI).
- Six accepted ADRs (GUI, persistence, metadata, crate boundaries, audio, licence) and the architecture baseline (docs/adr/index.md, docs/architecture.md).

**Deferred** — do not expect these yet (idea.md MVP §Deferred; docs/architecture.md §11; plan/MVP.md):

- M1: bounded implementation candidate is under independent review; backup/restore, migration-failure, transaction-failure and complete M1 evidence remain open. Vendor DJ metadata schema remains proposed and unaccepted (ADR-005). Persistence dependencies for representative-scale qualification remain gated on the separate 500k-track route (ADR-002).
- M2: filesystem import (plan/MVP.md).
- M3: usable library UI — prototype direction only (egui/eframe, ADR-001), not committed to production.
- M4: complete Rekordbox/OneLibrary interoperability remains unqualified. XML preview, preflight, no-edit export, and guarded edits to selected tempo and position-mark fields cover a bounded synthetic subset only; no vendor application acceptance or undocumented-format compatibility is claimed (interoperability.md §Fixture D).
- M4+: realtime DJ functionality — decks, mixing, effects, stems, streaming, broad controller support (idea.md §Priorities/MVP).

The first major product proof — the MVP (idea.md) — is library-first: import a real DJ library, manage it safely in OxiDJ, export it again, and continue using compatible DJ software/hardware without unexpected metadata loss.

## Documentation

- [docs/index.md](docs/index.md) — curated guide to key project documents, grouped by purpose.
- [docs/glossary.md](docs/glossary.md) — DJ and domain vocabulary (beatgrid, cues, loops, provenance, vendor formats).
- [site/README.md](site/README.md) — the "FILE #0042" teaser site: what it is, and how to build and test it locally.
- [plan/STATUS.md](plan/STATUS.md) — current milestone, blockers, and next tasks.
- [AGENTS.md](AGENTS.md) — rules for agent contributors.

## Toolchain and quality gate

Toolchain: **Rust 1.99.0**, declared as the MSRV and pinned by `rust-toolchain.toml` (see docs/development.md §MSRV).

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
