---
id: 002
title: Use SQLite with WAL for local-first persistence
status: Accepted
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/research/persistence-evaluation.md, docs/research/soundcraft.md]
sol_review: completed
review_status: completed 2026-10-15 (gpt-6.1-sol, reasoning medium) — Option 1 amended; final ruling ACCEPTED 2026-10-15
review_confidence: medium
admission_gate: A representative 500k-track prototype must pass before the selected persistence dependencies enter the workspace. Acceptance of the decision is not admission of the dependency.
---

# ADR-002: Persistence Layer

> **Accepted (2026-10-15).** The independent Sol/medium review is complete and the final ruling is ACCEPT. SQLite + WAL with bundled `rusqlite` and `rusqlite_migration` is the selected engine. Acceptance is **not** dependency admission: the persistence dependencies stay out of the workspace until the representative 500k-track prototype gate passes.

## Context

OxiDJ is a single-process, local-first DJ library manager handling 100k–500k tracks with schema migrations, point-in-time recovery, and per-field provenance. The persistence layer must be simple to distribute, survive power loss, and degrade gracefully. This decision selects the storage engine and Rust stack for M1–M3.

## Decision

Adopt **SQLite with WAL mode** as the default local-first persistence engine:

- Rust binding: `rusqlite` with the `bundled` feature (consistent SQLite version, no system dependency).
- Schema migrations: `rusqlite_migration` (uses `user_version`, plain SQL strings).
- Connection model: single writer (bounded channel queue) + read pool (`deadpool`/`r2d2`), `BEGIN IMMEDIATE`.
- Required PRAGMAs on every connection: `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON`, `journal_size_limit=67108864`.
- Backups: `Litestream` is **optional** asynchronous replication, not the backup strategy and not a guaranteed RPO; the `SQLite .backup` API covers ad-hoc/CI. Verified restores and separate protection of external assets (audio files, artwork) are still required.
- Future-proofing: use **domain-facing repository interfaces** rather than a generic SQL-portability trait; add broader portability only when a second backend creates a concrete requirement.

This decision does NOT commit to final schema design — that is deferred to M1 (ADR-005, metadata schema). It commits to the storage engine and binding for the M0–M3 horizon. **Acceptance is separate from dependency admission:** the selected dependencies (`rusqlite`, `rusqlite_migration`) must not enter the workspace until the 500k-track prototype gate passes.

## Evidence

- `persistence-evaluation.md` executive summary (line 17): "SQLite with WAL mode is the recommended default" for single-process local-first architecture.
- `persistence-evaluation.md` lines 26-35: SQLite performance baselines on NVMe — 50K-100K read queries/sec, 10K-50K write TPS, ~1TB practical limit. OxiDJ's 500k tracks x ~100 columns = ~50-100MB, well within SQLite's comfort zone.
- `persistence-evaluation.md` lines 37-49: Required PRAGMA settings (WAL, NORMAL, busy_timeout, foreign_keys, journal_size_limit).
- `persistence-evaluation.md` lines 73-94: `rusqlite` + `bundled` chosen over `sqlx` (~174 crates, ~19s build) and `diesel` (~26 crates, ~21s build) — lighter, faster builds, honest synchronous API.
- `persistence-evaluation.md` lines 98-122: `rusqlite_migration` uses `PRAGMA user_version` (single integer), plain SQL, tiny crate.
- `persistence-evaluation.md` lines 128-147: Litestream for point-in-time recovery (RPO ~1s), `.backup` API for ad-hoc. Migration prep: abstraction trait, avoid SQLite-specific SQL (lines 197-201).
- `persistence-evaluation.md` lines 205-214: PostgreSQL/MySQL (overkill), DuckDB (OLAP), Redb/sled (no SQL), libSQL (fork risk), SQLCipher (not required).
- `soundcraft.md` line 64 (ADAPT: "Separation of stored metadata and decoded caches") and line 68 (REJECT: "Whole-session JSON plus eagerly decoded media as the library storage design"): separate durable provenance from replaceable analysis; a DJ collection is not a DAW session.

## Options Considered

| Option | Decision | Reason |
|--------|----------|--------|
| SQLite + WAL + rusqlite | **Provisional** | Fits single-process local-first; 500k tracks ~100MB; proven at scale |
| PostgreSQL | Rejected | Overkill for local app; operational cost; network latency |
| DuckDB | Rejected | OLAP columnar, not transactional |
| Redb / sled | Rejected | Key-value, no SQL, no migrations, no tooling |
| libSQL (local) | Not yet | Fork risk; server mode not needed in M0 |
| SQLCipher | Not needed | Encryption not an OxiDJ requirement (local-first, user controls files) |

## Consequences

- Distribution: bundled SQLite (via `rusqlite` `bundled`) — no system SQLite dependency.
- Migration path: domain-facing repository interfaces must be introduced early so schema changes can be tested.
- WAL files: Litestream is an optional replication sidecar, not a backup guarantee; `.backup` API available for CI snapshots. Verified restores are required.
- Concurrency: single-writer queue pattern avoids SQLite locking issues.
- Rollback cost: if PostgreSQL becomes necessary (multi-process writes), domain-facing repository interfaces limit the blast radius. Migration budgeted at weeks of engineering time.

## Open Questions / What Would Change This

- Does single-writer become a bottleneck if a future "OxiDJ server" mode emerges? (Threshold: >5% of writes waiting >100ms, or >10K sustained TPS.)
- Does PostgreSQL become necessary? (Threshold per persistence-evaluation.md lines 186-195: only when 2+ of {write contention, >10K TPS, multi-process writes, >1TB} hit simultaneously.)
- Network filesystem support: the WAL database requires a **supported local filesystem**; SQLite is explicitly unsupported on NFS/SMB and FUSE is not assured suitable. Document the local-filesystem requirement.

## Sol Review Packet

**DECISION:** Use SQLite with WAL mode, `rusqlite` (bundled), `rusqlite_migration`, domain-facing repository interfaces. Litestream optional, not the backup strategy.

**WHY NOW:** M1-M3 persistence must be chosen before schema design. SQLite is the lowest-risk option that meets all stated requirements.

**CONSTRAINTS:** Single-process local-first. Must handle 100k-500k tracks. Must support migrations, backups, provenance. No multi-process writes in M0 scope.

**OPTIONS:** SQLite (provisional), PostgreSQL (rejected: overkill), DuckDB (rejected: OLAP), Redb/sled (rejected: no SQL), libSQL (deferred: fork risk), SQLCipher (deferred: not required).

**EVIDENCE:** persistence-evaluation.md: SQLite 50-100K read QPS, 10-50K write TPS, ~1TB limit; 500k tracks = ~100MB; rusqlite ~9 crates/8s build vs sqlx ~174 crates/19s; botmonster.com + sesamedisk.com 2026 benchmarks; Litestream RPO ~1s.

**CURRENT RECOMMENDATION:** SQLite + WAL + bundled `rusqlite` + `rusqlite_migration` (+ optional Litestream). Confidence: medium. The QPS/TPS/size/build-time/RPO figures are **contextual third-party benchmarks, not OxiDJ acceptance evidence**; the estimated 500k-track footprint is not established. What changes it: proven multi-process write requirement, or dataset approaching 1TB while hitting 2+ other thresholds.

**OUTPUT CONTRACT:** None — the decision is **Accepted**, but the dependency-admission gate is still blocking. Future M1 work creates `prototypes/oxidj-sqlite-probe` to benchmark the OxiDJ schema with 500k tracks (indexes, migrations, concurrency, crash/reopen, retained-backup restore) with measurable pass criteria defined before running it. No Cargo dependency enters the M0 workspace until that prototype passes.

## Independent Review Outcome (2026-10-15)

**Reviewer:** `gpt-6.1-sol` via `openai-codex`, reasoning **medium**, isolated delegate (no tools, no delegation, this decision only).

**Recommendation returned:** Option 1 **amended** — keep SQLite + WAL + bundled `rusqlite` + `rusqlite_migration`, but (a) treat Litestream as *optional*, not as the complete backup strategy, (b) use **domain-facing repository interfaces** rather than a generic SQL-portability trait, and (c) require the prototype before any dependency enters the M0 workspace.

**Confidence: medium — the ADR's own "high confidence" claim was rejected.** The QPS/TPS/size/dependency figures are contextual claims from third-party benchmarks, not acceptance evidence; they do not model OxiDJ's indexes, analysis payloads, transaction sizes or durability settings.

**Lead verification of the load-bearing claims (checked against primary sources, not accepted on reviewer confidence):**

- **WAL gives reader/writer overlap, not parallel writers.** Confirmed at <https://sqlite.org/wal.html>: *"WAL provides more concurrency as readers do not block writers and a writer does not block readers."* The single-writer model is unchanged. Consequence: concurrent analysis workers must submit **bounded, short, batched transactions through a coordinated writer**. Safety is achievable; acceptable throughput and interactive latency remain **unproven**.
- **Litestream is continuous replication, not a backup story.** Confirmed at <https://litestream.io/how-it-works/>: *"a streaming replication tool… asynchronous replication"* which *"takes over the checkpointing process"* and replicates each database *"to a single replica destination"*. It cannot by itself establish recoverability — accidental deletion and logical corruption propagate, the cited ~1s RPO is not an OxiDJ guarantee, and verified restores plus separate backup of external assets (audio files, artwork) are still required.
- **The generic SQL-portability trait is premature abstraction.** Transactions, indexing, search and migrations stay engine-specific, so the trait buys little portability. Hide SQLite behind application/domain operations; add broader portability only when a second backend creates a concrete requirement.
- **Additional constraint found while verifying:** WAL *"does not work over a network filesystem"* — all processes must be on the same host. Harmless for local-first, but it forecloses a future shared-library-on-NAS design and belongs in the record.

**Status: `Accepted` (final ruling 2026-10-15).** The review gate is satisfied. Acceptance is separate from dependency admission.

## Final Ruling (2026-10-15)

**Reviewer:** `gpt-6.1-sol` via `openai-codex`, reasoning **medium**, isolated delegate (no tools, no delegation, this decision only).

**Ruling: ACCEPT Option 1 as amended. Transition ADR-002 to `Accepted`, with medium confidence.** The precondition is correctly scoped to **dependency admission**, not decision acceptance. Selecting SQLite now does not establish that OxiDJ's workload has passed validation or authorize adding the persistence dependencies yet.

**Carries forward:**
- SQLite + WAL, bundled `rusqlite`, and `rusqlite_migration`.
- A coordinated single writer with bounded, short, batched transactions; WAL permits reader/writer overlap, not parallel writers.
- Domain-facing repository interfaces — not a generic SQL-portability trait.
- Litestream as optional asynchronous replication, not the backup strategy or a guaranteed RPO.
- A **tracked, representative 500k-track prototype gate before the selected persistence dependencies enter the workspace**. It must exercise realistic indexes and payloads, migrations, simultaneous scans/analysis/search, writer-queue behavior, interactive latency, crash/reopen, and retained-backup restore. Define measurable pass criteria before running it. Failure on engine-level requirements reopens the choice; Litestream-specific failure removes Litestream, not SQLite.

**Accepted vs admission:** the decision is accepted; the dependencies stay out of the workspace until the prototype gate passes.

**What would reopen the engine choice:** a 500k-track prototype showing sustained writer-queue growth or unacceptable interactive latency under simultaneous scans, analysis updates and search despite short batched transactions and sound indexing; or a failed crash/reopen or retained-backup restore test. A Litestream-specific packaging or restore failure removes Litestream, not SQLite.
