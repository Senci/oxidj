# M0-06: Persistence Layer Evaluation

**Status:** Research complete — review ready
**Owner:** oxidj-2
**Date:** 2026-10-15

---

## Executive Summary

OxiDJ needs a local-first persistence layer for hundreds of thousands of tracks with:
- Schema migrations
- Backups / point-in-time recovery
- Concurrency (readers + single writer)
- Provenance (import source tracking)

**SQLite with WAL mode is the recommended default** for OxiDJ's single-process, local-first architecture. PostgreSQL/MySQL are premature optimizations unless multi-process writes or >1 TB data are proven requirements.

**No decision without scoped Sol review.**

---

## SQLite in Production (2026 Reality Check)

### Performance Baselines (Modern NVMe)

| Metric | Value | Source |
|--------|-------|--------|
| Read throughput | 50,000–100,000+ queries/sec | botmonster.com, sesamedisk.com |
| Write throughput | 10,000–50,000 transactions/sec | botmonster.com, daily.dev |
| Practical dataset limit | ~1 TB | daily.dev, sesamedisk.com |
| Single-writer latency | 0.1–1 ms per transaction | botmonster.com |
| Lock contention threshold | ~20 concurrent writers | Shivek Khurana benchmark |

**OxiDJ context:** A DJ library with 500k tracks × ~100 columns ≈ 50–100 MB. Well within SQLite's comfort zone. Write workload is import/analysis (batch, sequential), not high-concurrency OLTP.

### Required PRAGMA Settings (Production)

```sql
-- Apply on EVERY connection
PRAGMA journal_mode = WAL;           -- Readers never block writer; one writer
PRAGMA synchronous = NORMAL;         -- Safe with WAL; 2-5x faster writes
PRAGMA cache_size = -64000;          -- 64 MB page cache (adjust to 10-25% RAM)
PRAGMA mmap_size = 268435456;        -- 256 MB memory-map for reads
PRAGMA busy_timeout = 5000;          -- Wait 5s for write lock (not instant SQLITE_BUSY)
PRAGMA foreign_keys = ON;            -- Enforce referential integrity (not persistent!)
PRAGMA journal_size_limit = 67108864; -- Cap WAL at 64 MB
PRAGMA auto_vacuum = INCREMENTAL;    -- Incremental vacuuming
```

**Sources:** [botmonster.com](https://botmonster.com/coding/sqlite-application-database-when-how-to-use/), [sesamedisk.com](https://sesamedisk.com/sqlite-in-production-2026-benchmarks-limits/), [SQLite docs](https://sqlite.org/pragma.html).

### Concurrency Model for OxiDJ

```
┌─────────────────────────────────────────────────────────────┐
│                    Single Process (OxiDJ)                   │
├─────────────────────────────────────────────────────────────┤
│  Read Pool (N connections)     │  Writer (1 connection)    │
│  - SELECT queries              │  - INSERT/UPDATE/DELETE   │
│  - Concurrent, never blocked   │  - Serialized via queue   │
│  - Pool size: CPU cores × 2    │  - BEGIN IMMEDIATE        │
└─────────────────────────────────────────────────────────────┘
```

- **No multi-process writes** — OxiDJ is single-process by design (local-first)
- **Read pool** — Use `r2d2` or `deadpool` for connection pooling
- **Writer queue** — Single dedicated writer thread with bounded channel (see botmonster.com pattern)
- **BEGIN IMMEDIATE** — Grab write lock upfront; fail fast if busy

---

## Rust SQLite Stack

### Recommended: `rusqlite` (Sync) + `tokio-rusqlite` / `r2d2` for Async

| Crate | Version | Role | Notes |
|-------|---------|------|-------|
| `rusqlite` | 0.40+ | Core sync bindings | Use `bundled` feature for consistent SQLite version |
| `tokio-rusqlite` | 0.7+ | Async adapter | `spawn_blocking` wrapper; avoids blocking tokio workers |
| `r2d2` / `r2d2_sqlite` | — | Connection pool | For read pool |
| `rusqlite_migration` | 2.6+ | Schema migrations | Uses `user_version` (lightweight integer) |
| `sqlx` | 0.9+ | Alternative | Compile-time checked SQL; async; heavier deps (~174 crates) |

### Compile-Time Comparison (rustfinity.com)

| Stack | Crates | Clean Build |
|-------|--------|-------------|
| `rusqlite` (bundled) | ~9 | ~8 s |
| `sqlx` (sqlite + tokio + tls-rustls) | ~174 | ~19 s |
| `diesel` (postgres) | ~26 | ~21 s |
| `seaorm` (sqlx-postgres) | ~213 | ~29 s |

**For OxiDJ (SQLite-only, desktop app):** `rusqlite` is the standard choice — lighter, faster builds, honest synchronous API.

---

## Schema Migrations

### `rusqlite_migration` Approach

```rust
use rusqlite_migration::{Migration, Migrations};

static MIGRATIONS: Migrations = Migrations::new(vec![
    Migration::new(1, "CREATE TABLE tracks (...); CREATE INDEX ...;"),
    Migration::new(2, "ALTER TABLE tracks ADD COLUMN bpm REAL;"),
    // ...
]);

fn migrate(conn: &mut Connection) -> Result<()> {
    MIGRATIONS.to_latest(conn)?;
    Ok(())
}
```

**Advantages:**
- Uses `PRAGMA user_version` (single integer at fixed offset) — no migration tables to parse
- Fast compilation (tiny crate, no macros)
- Plain SQL strings in Rust code

**SQLite 3.35+ (2021+) supports `ALTER TABLE DROP COLUMN`** — full ALTER TABLE on 3.46+ (2024).

**Sources:** [rusqlite_migration docs](https://docs.rs/crate/rusqlite_migration/latest), [botmonster.com](https://botmonster.com/coding/sqlite-application-database-when-how-to-use/#schema-migrations), [rustfinity.com](https://www.rustfinity.com/blog/rust-orms).

---

## Backups & Point-in-Time Recovery

### The Problem
`cp database.db backup.db` is **unsafe** with WAL — WAL file may not be checkpointed.

### Solutions

| Tool | Approach | RPO | Complexity | Status |
|------|----------|-----|------------|--------|
| **Litestream** | Sidecar streams WAL → S3/GCS/Azure | ~1 sec | Low (single binary + config) | ✅ Active (v0.5+ LTX format) |
| **LiteFS** | FUSE filesystem replicates WAL to read replicas | ~100ms | Medium (FUSE, Fly.io) | ⚠️ Pre-1.0; Cloud sunset Oct 2024 |
| **Turso / libSQL** | Managed service; embedded replicas sync to edge | Configurable | Low (managed) | ✅ Active |
| **SQLite `.backup` API** | Online snapshot via `sqlite3_backup_init()` | Manual | Built-in | ✅ For ad-hoc |

### Recommendation for OxiDJ
- **Local development / CI:** SQLite `.backup` API (built-in, zero deps)
- **Production / user machines:** **Litestream** — battle-tested, actively maintained, S3-compatible, point-in-time restore with `litestream restore -o restored.db s3://bucket/db`
- **Future multi-device sync:** Evaluate Turso/libSQL embedded replicas when needed

**Sources:** [litestream.io](https://litestream.io/), [fly.io LiteFS](https://fly.io/docs/litefs/), [Turso](https://docs.turso.tech/features/embedded-replicas/introduction), [sesamedisk.com](https://sesamedisk.com/sqlite-in-production-2026-benchmarks-limits/), [Simon Willison](https://simonwillison.net/2025/Oct/3/).

---

## Provenance & Import Tracking

### OxiDJ Requirement
Track every track's origin: filesystem import, Rekordbox XML, Serato, Engine DJ, manual edit, analysis results.

### Schema Pattern

```sql
CREATE TABLE track_provenance (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK (source_type IN (
        'filesystem', 'rekordbox_xml', 'rekordbox_db', 'serato', 'engine_dj',
        'manual', 'analysis', 'import_batch'
    )),
    source_id TEXT,           -- e.g., XML file path, DB primary key, batch UUID
    source_field TEXT,        -- which field in source (optional)
    imported_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    imported_by TEXT,         -- user/app version
    checksum TEXT,            -- content hash for change detection
    PRIMARY KEY (track_id, source_type, source_id)
);

CREATE INDEX idx_provenance_track ON track_provenance(track_id);
CREATE INDEX idx_provenance_batch ON track_provenance(source_id) WHERE source_type = 'import_batch';
```

### Round-Trip Testing
- Export → re-import → compare provenance
- Property-based tests (proptest) for import/export invariants
- Golden master XML/DB fixtures from real libraries

---

## When to Migrate Away from SQLite

### Decision Framework (botmonster.com)

| Metric | Threshold | Action |
|--------|-----------|--------|
| Write lock contention | >5% of writes wait >100ms | Optimize queries first |
| Write throughput | Sustained >10K TPS | Evaluate connection architecture |
| Multi-process writes | >1 app instance needs writes | **Consider PostgreSQL** |
| Dataset size | Approaching 1 TB | Plan migration |

**Migrate when 2+ thresholds hit simultaneously.** Not before.

### Migration Preparation (Do Now)
- Use abstraction layer supporting both (SQLx, or custom trait)
- Avoid SQLite-specific SQL (`GLOB`, type affinity tricks)
- Test migration path regularly (not just at crisis)
- Budget weeks of engineering time

---

## Alternatives Considered & Rejected

| Alternative | Why Not for OxiDJ |
|-------------|-------------------|
| **PostgreSQL/MySQL** | Overkill for single-process local app; operational cost; network latency |
| **DuckDB** | OLAP columnar; not for transactional track metadata |
| **Redb / sled** | Key-value; no SQL, no migrations, no tooling |
| **libSQL / Turso (local)** | Fork risk; server mode not needed yet |
| **SQLCipher** | Encryption not an OxiDJ requirement (local-first, user controls files) |

---

## Open Risks

1. **WAL checkpoint lag** under heavy write bursts — monitor `PRAGMA wal_checkpoint(PASSIVE)`
2. **Schema migration locking** — only one connection can modify schema at a time; use migration lock or maintenance window
3. **Network filesystem** — SQLite explicitly unsupported on NFS/SMB (locking bugs)
4. **Concurrent writer processes** — if future "OxiDJ server" mode emerges, plan PostgreSQL migration

---

## Evidence Packets for Review

- [ ] PRAGMA configuration + benchmark results (local NVMe)
- [ ] Migration test suite (up/down, idempotent)
- [ ] Litestream backup/restore drill (local + S3)
- [ ] Concurrency test: N readers + 1 writer under load
- [ ] Provenance schema + round-trip test vectors
- [ ] `rusqlite` vs `sqlx` compile-time benchmark in OxiDJ workspace

---

## References

1. botmonster.com, *SQLite scales to production: 10K TPS, WAL mode, real benchmarks* (2026-05-16) — [article](https://botmonster.com/coding/sqlite-application-database-when-how-to-use/)
2. sesamedisk.com, *SQLite in Production 2026: Real Benchmarks, Limits, and When to Migrate* — [article](https://sesamedisk.com/sqlite-in-production-2026-benchmarks-limits/)
3. daily.dev, *SQLite Production Guide: When and How to Use It Beyond Prototyping* — [article](https://daily.dev/blog/sqlite-production-guide-when-how-to-use-beyond-prototyping/)
4. rustfinity.com, *Diesel, SQLx, SeaORM or Rusqlite? Choosing a Rust ORM in 2026* (2026-04-04) — [article](https://www.rustfinity.com/blog/rust-orms)
5. Litestream — [litestream.io](https://litestream.io/)
6. Turso / libSQL — [docs.turso.tech](https://docs.turso.tech/features/embedded-replicas/introduction)
7. Simon Willison, *Litestream v0.5.0 is Here* (2025-10-03) — [blog](https://simonwillison.net/2025/Oct/3/)
8. SQLite Documentation — [sqlite.org](https://sqlite.org/)