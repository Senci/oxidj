# M1 SQLite prototype readiness note

**Task:** t_45a179ff
**Author:** sova (Researcher)
**Date:** 2026-10-15
**Scope:** Independent executability assessment of the 500k-track SQLite prototype protocol. This is a readiness proposal, not a benchmark result. It follows the accepted ADR-002 engine/durability decision and does not reopen it.

---

## Verdict: NOT READY

The protocol is structurally sound and grounded in accepted decisions, but it is not yet executable. Six categories of inputs remain unfrozen: fixture distributions, arrival/sample accounting, live WAL/disk envelope, semantic oracle, recovery fault matrix, and numerical budget feasibility. Each is identified below with its smallest missing evidence or decision.

---

## 1. Repository-supported facts vs. assumptions

### Facts (accepted repository decisions)

These are traceable to accepted ADR decisions, not to the draft protocol.

| Fact | Source |
|------|--------|
| 500k initial tracks | ADR-002:11 (admission gate) |
| WAL mode, bundled rusqlite, rusqlite_migration, single writer, BEGIN IMMEDIATE | ADR-002:24-29 |
| synchronous=FULL on every writer; ack only after successful commit | ADR-002:30; durability amendment :129 |
| busy_timeout=5000, foreign_keys=ON, journal_size_limit=64MiB | ADR-002:29 |
| Zero-loss target conditional on supported stack honoring sync | ADR-002:31; durability amendment :129 |
| Neutral domain model, per-field provenance, vendor passthrough, no silent loss | ADR-003:25-35 |
| 38 hotcues, 0-5 ratings, playlist trees, point vs loop cues, constant vs variable beat grids | ../semantic-mapping.md; mixxx-semantics.md |

### Proposals with owner update (2026-10-15)

These workload definitions and numerical budgets originated as proposals in the draft admission protocol. The owner has since approved the explicit base 500k-track fixture shape and the 30-minute mixed-load rates shown below, and confirmed the no-benchmark/no-dependency gate until the complete protocol and semantic oracle are executable and independently reviewed. This does not settle payload distributions/fan-out, read-class proportions/query selectivities, whether manual edits are included in the 10 req/s stream, a statistical adequacy/confidence target, latency/resource budgets, recovery cases, or the cold-cache method. The protocol's original provisional-budget statements (m1-sqlite-admission-protocol.md:5,44) remain applicable to all unapproved numeric criteria; proposals are not measured performance and must not be promoted into accepted targets.

| Proposal | Source | Status |
|---------|--------|--------|
| Deterministic 500k-track fixture, deterministic seed (proposed default workload) | m1-sqlite-admission-protocol.md:27 | Base size/seed approved 2026-10-15; payload distribution/fan-out and fixture specifics remain open (NOT an ADR-002 accepted fact) |
| 30-minute mixed load, 2 producers (100 scan inserts/s + 100 analysis updates/s), 4 readers (10 req/s total) | m1-sqlite-admission-protocol.md:31 | Owner-approved base duration/rates 2026-10-15; read-class proportions/query selectivities, analysis payload, and manual-edit share remain open |
| Read p95<=100ms, p99<=250ms; write submission-to-BEGIN p95<=100ms, p99<=500ms; txn duration p99<=50ms, max 250ms | m1-sqlite-admission-protocol.md:33 | Provisional; pending owner ratification |
| Queue bound 1024 jobs/64MiB; RSS<=1GiB; post-drain WAL<=64MiB | m1-sqlite-admission-protocol.md:33 | Provisional; pending owner ratification |
| Overload 2x writes for 60s, explicit backpressure, recover within 60s | m1-sqlite-admission-protocol.md:33 | Provisional; pending owner ratification |
| Migration<=60s, reopen<=2s, restore<=120s | m1-sqlite-admission-protocol.md:35 | Provisional; pending owner ratification |
| 30 process kills across transactions/checkpoints/migration | m1-sqlite-admission-protocol.md:37 | Provisional; pending owner ratification |
| Two online backups, restore after edits/deletion, exact snapshot match | m1-sqlite-admission-protocol.md:37 | Provisional; pending owner ratification |
| Reference device `viper` (Ryzen 5 5600X, 31GiB, Btrfs zstd:3 over LUKS on NVMe) | m1-sqlite-admission-protocol.md:29 | Candidate; pending owner selection and pre-run re-recording |

### Assumptions (not established by repository evidence)

| Assumption | Gap |
|------------|-----|
| Payload size distributions (p50/p95/max) | No fixture data; protocol says "never assume 100MB" |
| Fan-out ratios (playlists/track, cues/track, grids/track) | Vendor maxima known, but actual distributions unknown |
| Analysis payload contents and sizes | Not specified anywhere |
| Query mix percentages | Listed but not quantified |
| Manual edit rate | Not specified |
| Arrival schedule (open-loop vs closed-loop) | Not defined |
| Cold-cache procedure | Not defined |
| Overload reader schedule | Not defined |
| Reader lifetime policy | Not defined |
| Free-disk floor | Not defined |
| Schema shape | ADR-005 unresolved |
| Import/export round-trip test details | ADR-005 carries field-relationship risk |

---

## 2. Gap-by-gap assessment

### 2.1 Query/arrival/sample accounting

**Current state:** The protocol lists interactive operations (track lookup, selective/broad search, sorted first/next 100-row pages, playlist pages, manual edits) and a 10 req/s aggregate reader rate, but does not define which are read classes, whether selective/broad search or first/next pages are separate classes, or whether the 10 req/s reader rate includes manual edits. It also does not define:
- The proportion of each query class
- Whether arrivals are open-load (Poisson/fixed-rate independent of completion) or closed-loop (wait for response before next request)
- The percentile estimator (e.g., reservoir sampling, t-digest, exact histogram)
- Minimum sample counts per class per run for statistical validity
- Cold-cache procedure (OS page cache drop between runs? SQLite cache flush?)

**Proposed definitions (executable defaults, pending owner approval):**

- **Arrival model:** open-loop. Each reader issues requests at a fixed rate independent of completion. This prevents a slow query from artificially reducing arrival rate and masking backlog. Record actual inter-arrival jitter.
- **Percentile estimator:** exact histogram. Bucket width is a resolution trade-off, not an approximation: 1ms buckets provide millisecond-level resolution only — they do NOT provide sub-millisecond resolution. If sub-millisecond resolution is required, use finer buckets (e.g., 0.1ms or microsecond). Report p50, p95, p99 (the protocol's acceptance percentiles — m1-sqlite-admission-protocol.md:33), plus max, min, mean, stddev. p99.9 is NOT an acceptance percentile. Do not report it as a reliable measured metric without a separately justified sample plan; if shown for exploration, mark it descriptive-only, include the class sample count, and never use it for pass/fail.
- **Minimum samples:** proposed default, recalculated from the protocol's actual aggregate rate. The protocol specifies 4 readers at 10 req/s total over a 30-minute steady-state window, yielding approximately 10 x 30 x 60 = 18,000 total reader samples. For illustration only, if the 10 req/s stream is reads-only and the approved mix contains five read classes at uniform 20% each, that is approximately 10 x 0.20 x 30 x 60 = 3,600 samples per class. The protocol does not yet settle the exact read-class grouping or whether manual edits are part of that reader rate; recompute after those inputs are ratified. At ~3,600 samples/class the p99 tail has only ~36 observations (the ~3,564th ordered value is p99), and the p99.9 tail has only ~3.6 observations (the ~3,596th ordered value is p99.9) — too few for reliable p99.9 resolution. For this illustration, p95 and p99 yield approximately 180 and 36 upper-tail observations, respectively; these counts permit empirical point estimates but do not establish statistical confidence or adequacy, which remains an owner-approved protocol input. This does NOT support p99.9 as an acceptance or reliably measured metric. A 1,000-sample class would yield only ~1 observation beyond p99.9, which is insufficient; do not treat that count alone as a confidence standard without a justified sample plan. This is a **proposed** target derived from the provisional protocol, not an accepted statistical standard.
- **Cold-cache procedure:** PROVISIONAL — NOT READY. Dropping the OS page cache (`echo 3 > /proc/sys/vm/drop_caches`) requires root and evicts filesystem cache for the **entire host**. Linux `drop_caches` is global and cannot be scoped to a single filesystem or path, so it cannot be safely run on a shared host without displacing other workloads' caches. It must NOT be recommended as a casual step. Cold-cache measurement remains NOT READY until a controlled method is defined and validated, e.g. a fresh disposable VM with the fixture pre-provisioned and untouched since boot, or explicitly authorized host-wide cache eviction on a dedicated disposable host. For warm runs, execute a warm-up pass equal to 10% of the measurement duration before recording.
- **Query class proportions:** a uniform mix is a simple synthetic coverage baseline that exercises each approved class; it is not evidence of a representative OxiDJ workload. The class set, proportions, and whether manual edits are included in the 10 req/s reader rate all need owner ratification.

**Verdict:** NOT READY. Query class proportions, minimum sample targets, and the cold-cache method need owner ratification and a controlled runner; host-owner authorization is required if the method evicts the host-wide page cache. The definitions above are proposals, not approved criteria.

### 2.2 Latency estimators and minimum samples

**Current state:** p95/p99 are mentioned but no estimator or minimum sample count is defined.

**Proposed:** See 2.1 above. Exact histogram with 1ms bucket resolution (or finer if sub-ms is required); proposed sample target recalculated from the protocol's aggregate rate (~3,600 per class over the 30-minute steady-state window). Both the estimator choice and the sample count are provisional defaults.

**Verdict:** NOT READY (pending owner approval of the estimator choice and sample-count target).

### 2.3 Offered/accepted/completed write accounting

**Current state:** The protocol says "count blocked/rejected submissions, no lost accepted work" and "completed throughput must match offered throughput" under target load. It does not define:
- The boundary between "offered" and "accepted" (is queue admission the acceptance point?)
- Whether rejected work at target load is a failure or expected behavior
- How to account for writes that are accepted but whose transaction fails at commit

**Proposed definitions:**

- **Offered:** a producer attempts to submit a write job to the queue. Record timestamp and payload bytes.
- **Accepted:** the job is admitted into the bounded queue. Record timestamp, queue depth, and payload bytes.
- **Completed:** the job's transaction commits successfully and the acknowledgement is sent. Record timestamp.
- **Rejected:** the queue is full (1024 jobs or 64MiB). Record timestamp, payload bytes, and rejection reason.
- **Failed:** the job was accepted but the transaction failed at commit. Record timestamp and error. A failed job is NOT a lost accepted work if the failure is reported to the producer and the producer may retry; it IS a lost accepted work if the failure is silent.

**Accounting rule:** under target load, completed must equal offered (per m1-sqlite-admission-protocol.md:33) — i.e. no silent drops of accepted work. Each accepted job is accounted for as completed (successful commit + ack), failed (accepted but its commit failed and was reported — NOT silently dropped, not completed), or rejected (queue full, must be reported). `completed == accepted` holds only with zero commit failures; a reported failure is counted as failed, and its retry is a new offered job. This rule is provisional pending owner ratification, particularly of the target-load "completed must equal offered" requirement and whether any target-load rejection is permitted.

**Backlog metric:** accepted minus completed, sampled every second. Report as a time series, not just a final value.

**Verdict:** NOT READY. The definitions above are executable but need owner ratification, particularly the "completed must match offered" rule and whether any target-load rejection is permitted.

### 2.4 Workload duration

**Current state:** Owner-approved initial 30-minute duration; sufficiency and warm-up treatment remain unresolved.

**Assessment:** The owner approved 30 minutes as an initial run duration, but that duration is not established as sufficient. No OxiDJ evidence supports the claim that 30 minutes captures steady-state behavior, WAL growth patterns, or checkpoint/starvation dynamics on the 500k-track fixture — the protocol itself records all budgets as provisional and unmeasured (m1-sqlite-admission-protocol.md:5,44). Insufficiency is an explicit risk: 30 minutes may not capture steady-state tail latency, WAL/checkpoint behavior under checkpoint starvation, or long-run fragmentation/auto-vacuum effects. Sufficiency must be validated (or rejected) by the prototype run, not assumed here. It may serve as a minimum for a first gate only if the owner explicitly ratifies the sufficiency hypothesis and accepts the insufficiency risk. The protocol should specify whether this is a single continuous run or includes warm-up/cool-down phases.

**Proposed:** 30 minutes of steady-state measurement following a 5-minute warm-up (not recorded). Total wall-clock: 35 minutes per run. Three independently reset runs.

**Verdict:** NOT READY (pending owner approval of warm-up exclusion).

### 2.5 Overload/backpressure

**Current state:** "Overload at twice offered writes for 60s must backpressure explicitly, retain interactive budgets and recover/drain within 60s."

**Gaps:**
- What is the reader schedule during overload? Do readers continue at 10 req/s or are they throttled?
- What does "explicit backpressure" mean operationally? Reject new writes? Slow producers? Both?
- How is "recover/drain within 60s" measured? From the end of the overload period?

**Proposed definitions:**

- **Reader schedule during overload:** readers continue at their normal 10 req/s. The overload tests whether interactive reads retain their budgets under write pressure.
- **Explicit backpressure:** when the queue is full, producers receive an immediate rejection (not a silent drop). The rejection is counted and reported. Producers may retry with exponential backoff (base 100ms, max 5s, jittered).
- **Recovery measurement:** from the end of the 60s overload period, measure time until backlog returns to the pre-overload level and all queued writes complete. Must be <=60s.

**Verdict:** NOT READY. Reader schedule during overload and backpressure semantics need owner ratification.

### 2.6 Live WAL/free-disk bounds/checkpoint starvation

**Current state:** journal_size_limit=64MiB is set but acknowledged as "not a live hard cap." The post-drain WAL target (<=64MiB) does not bound live WAL growth during reader/checkpoint starvation. No live WAL bound, free-disk floor, or reader-lifetime policy is defined.

**SQLite semantics (official sources):**
- WAL mode uses a shared-memory index (shm file) and a write-ahead log (wal file). The WAL file grows as transactions commit and is reset when a checkpoint completes. [SQLite WAL documentation](https://sqlite.org/wal.html)
- Checkpoint starvation occurs when readers prevent the WAL from being fully checkpointed, causing the WAL file to grow without bound. [SQLite WAL documentation, section 6](https://sqlite.org/wal.html)
- journal_size_limit is NOT a checkpoint trigger. Per the official pragma docs, it limits the size of rollback-journal and WAL files **left in the file-system after transactions or checkpoints**: each time a transaction commits or a WAL file resets, SQLite compares the WAL size to the limit and truncates it if larger (the default limit is -1 = no limit). Checkpoints are triggered by `wal_autocheckpoint` (default 1000 pages), not by `journal_size_limit`. The WAL can still exceed the limit between checkpoints (e.g., during checkpoint starvation, when no checkpoint runs to apply the truncation), so this is not a live hard cap — see the proposed live-WAL bound in 2.6. [SQLite pragma documentation](https://sqlite.org/pragma.html#pragma_journal_size_limit)

**Proposed definitions:**

- **Live WAL bound:** WAL file size must not exceed 256 MiB at any point during the run. This is 4x the journal_size_limit and provides headroom for checkpoint starvation while still bounding disk usage. If exceeded, the run fails.
- **Free-disk floor:** free disk space must not fall below 1 GiB during the run. If it does, the run fails.
- **Reader lifetime policy:** a read transaction must not hold a read lock for more than 5 seconds. If a reader exceeds this, it is aborted and retried. This prevents indefinite checkpoint starvation.
- **Checkpoint starvation scenario:** a dedicated test where a reader holds an open read transaction for 30 seconds while writers continue. Measure WAL growth rate and verify it stays within the live WAL bound. This is a separate test from the main workload.

**Verdict:** NOT READY. The 256 MiB live WAL bound and 5s reader lifetime are engineering proposals that need owner ratification. The checkpoint starvation scenario is a new test that needs owner approval.

### 2.7 Semantic oracle

**Current state:** ADR-003 requires relationship-aware validation. ADR-005 is unresolved. The protocol says "zero semantic-oracle mismatches" but does not define the oracle.

**Repository-supported oracle inputs:**
- ADR-003:25-35 defines the neutral domain model constraints (identity independence, vendor passthrough, per-field provenance, representation preservation, non-destructive import/export)
- ../semantic-mapping.md defines concrete cardinalities (38 hotcues, 0-5 ratings, playlist trees, point vs loop cues, constant vs variable beat grids)
- mixxx-semantics.md defines beat grid types, cue types, provenance distinctions
- interoperability.md defines the fixture plan for import/export round-trips

**Proposed oracle definition:**

The semantic oracle is an independent checker that operates on the in-memory domain model, not on the database. It verifies:

1. **Identity:** track IDs are stable across re-imports; path changes do not create new tracks.
2. **Provenance:** every field records its source (ANALYZER | FILE_METADATA | USER | vendor import). User edits do not silently overwrite analyzed or imported values.
3. **Representation:** constant-tempo and variable-tempo beat grids are distinct. Point cues are distinct from loop ranges. No flattening occurs.
4. **Vendor passthrough:** unknown/unrepresentable fields are preserved as opaque payloads, not discarded.
5. **Capacity overflow:** hotcue count >38 is reported, not silently truncated.
6. **Round-trip:** import -> export -> import produces identical semantic content (not just parseable bytes).

**Relationship-aware validation (ADR-003 field-relationship risk):** the oracle must check that edits to linked fields (e.g., changing a beat grid invalidates dependent cue positions) are handled explicitly, not silently. This requires ADR-005 to define the relationship rules.

**Verdict:** NOT READY. The oracle definition above is executable for items 1-5, but item 6 (relationship-aware validation) is blocked by ADR-005. The smallest missing evidence is an ADR-005 schema decision that defines field relationships.

### 2.8 Migration cases

**Current state:** "Fresh creation, populated upgrade including table rebuild, repeat-open/no-op migration and injected migration failure must leave exactly old or new valid state, never partial; proposed migration <=60s."

**Gaps:**
- No schema exists yet (ADR-005 unresolved), so migration SQL cannot be written.
- "Injected migration failure" is not defined (which statement fails? how is failure injected?).
- "Populated upgrade" assumes a prior schema exists.

**Proposed test matrix:**

| Case | Description | Expected outcome |
|------|-------------|------------------|
| Fresh creation | New database, run all migrations from empty | New schema, valid state |
| Populated upgrade | Database at schema N, upgrade to N+1 with data | New schema, data preserved, valid state |
| Repeat-open/no-op | Database at latest schema, reopen | No migration runs, valid state |
| Injected failure | Migration N+1 fails at statement K | Database remains at schema N, valid state, error reported |
| Concurrent migration | Two processes attempt migration simultaneously | Out of scope: the accepted protocol is single-process (m1-sqlite-admission-protocol.md:13). May be added only by a separate multi-process migration requirement. |

**Verdict:** NOT READY. Blocked by ADR-005 (no schema). The test matrix is executable once a schema exists.

### 2.9 Restore oracle

**Current state:** "Restore two separately retained online backups after later edits/deletion, into clean directories without original DB/WAL; exact snapshot oracle/provenance/payload and asset-manifest match."

**Gaps:**
- "Exact snapshot oracle" is not defined (what is compared? bytes? semantic content? both?).
- "Asset-manifest match" is not defined (what assets? how hashed?).
- Restore timing boundaries are not defined (start/end points).

**Proposed definitions:**

- **Snapshot oracle:** after restore, the database must pass the same semantic oracle as the live database (2.7). Additionally, a row-by-row comparison of all tables must match the pre-backup state. This is a stronger check than semantic oracle alone.
- **Asset manifest:** a SHA-256 manifest of all external asset files (audio, artwork) is generated at backup time. After restore, the manifest is regenerated and compared. Any mismatch fails.
- **Restore timing:** start = restore command issued; end = first query returns a result. This is separate from full verification (which may take longer).

**Verdict:** NOT READY. The definitions above are executable but need owner ratification. The asset manifest format needs a decision.

### 2.10 Acknowledgment witness

**Current state:** "Independently persist a witness of acknowledged transaction IDs; verify it is not in-memory or unflushed."

**Proposed definition:**

- **Witness:** an append-only file on the same filesystem as the database. Each acknowledged transaction ID is written as a line (64-bit integer + newline) and flushed (fsync) before the acknowledgement is sent to the producer.
- **Verification:** after a process kill, the witness file is read and compared against the database's committed transaction IDs. Every acknowledged ID must be present in the database. Unacknowledged IDs may or may not be present (permitted uncertainty).
- **Independence:** the witness is not the database's own commit log. It is a separate file written by the application layer.

**Verdict:** NOT READY. The definition above is executable but needs owner ratification, particularly the fsync-before-ack ordering and the witness file format.

### 2.11 Sync/I/O fault points

**Current state:** "Inject I/O/sync failures around commit/ack boundaries and never report a failed/uncertain commit as saved."

**SQLite semantics (official sources):**
- A commit is atomic in WAL mode (either all changes are persisted or none are). The official *atomic commit* article documents only the rollback-journal mechanism (it states "The information in this article applies only when SQLite is operating in 'rollback mode'"); WAL-mode atomicity is described by the synchronous pragma ("FULL is atomic, consistent, isolated, and durable (ACID) in WAL mode") and the WAL documentation. [SQLite synchronous pragma](https://sqlite.org/pragma.html#pragma_synchronous); [SQLite WAL documentation](https://sqlite.org/wal.html)
- synchronous=FULL adds an fsync of the WAL file after each commit. A failed sync during a COMMIT in WAL mode may lose durability without corrupting the database file; the transaction outcome is uncertain and requires reconciliation — the commit is NOT automatically rolled back. [SQLite howtocorrupt.html, section 3.1](https://sqlite.org/howtocorrupt.html); [SQLite synchronous pragma](https://sqlite.org/pragma.html#pragma_synchronous)
- A failed fsync may surface as an I/O error: by default SQLite C APIs return the primary code SQLITE_IOERR (10), and the more specific extended code SQLITE_IOERR_FSYNC (1034) is returned only when extended result codes are enabled — whether that exact code is surfaced depends on the SQLite C API / Rust `rusqlite` binding configuration, which is not verified here. The commit then returns an error and its outcome is uncertain; the application must not report success. [SQLite rescode.html — SQLITE_IOERR / SQLITE_IOERR_FSYNC](https://sqlite.org/rescode.html#ioerr). Reconciliation is required per the ADR-002 durability amendment.
- If a commit returns an error, the transaction outcome is uncertain. The application must not report success. [SQLite durability amendment, ADR-002:129]

**Proposed fault injection points:**

| Point | Injection | Expected behavior |
|-------|-----------|-------------------|
| Before COMMIT | Fail the next write() syscall | Transaction fails, no ack sent, database unchanged |
| During COMMIT (after WAL write, before fsync) | Fail the next fsync() | Sync failure: durability may be lost without database corruption; outcome is uncertain, no ack sent; requires reconciliation per ADR-002:129 |
| After COMMIT (after fsync, before ack) | Fail the next write() to witness | Ack not sent, transaction is committed but unacknowledged; permitted uncertainty |
| After ACK | Kill process | Transaction is committed and acknowledged; must survive |

**Injection method:** candidate methods include a FUSE filesystem or an LD_PRELOAD wrapper that fails the Nth occurrence of a specified syscall. These are candidate techniques requiring feasibility checks before use — neither is verified as "standard" for this stack — and neither requires modifying SQLite.

**Verdict:** NOT READY. The fault injection method (FUSE vs LD_PRELOAD) needs owner ratification. The test matrix is executable once the method is chosen.

### 2.12 What process-kill tests cannot prove

**Current state:** "Process kills are not power-loss simulations; the zero-loss target is conditional on the named supported stack honoring synchronization."

**SQLite semantics (official sources):**
- A process kill terminates the application but does not clear kernel caches or simulate device power loss. Committed transactions with synchronous=FULL survive process kills because the WAL fsync has already been issued to the kernel. [SQLite WAL documentation](https://sqlite.org/wal.html)
- Power loss clears kernel caches and may leave the WAL file in an incomplete state. Whether the commit survives depends on the storage device's behavior during power loss: only with `synchronous=FULL` does SQLite issue a WAL sync at each commit to make transactions durable across power loss, and `synchronous=NORMAL` may lose committed transactions after power/system failure. This is a WAL-mode behavior; the official *atomic commit* article documents only the rollback-journal mechanism and does not cover WAL-mode power-loss behavior. [SQLite WAL documentation](https://sqlite.org/wal.html); [SQLite synchronous pragma](https://sqlite.org/pragma.html#pragma_synchronous)
- A finite test campaign (30 process kills) provides **bounded evidence** for the process-kill fault cases actually exercised — namely that acknowledged commits survive the specific kill points tested and that the application does not report false success after a kill. It does NOT prove correct handling of all kill points, all failure modes, power loss, device failure, or behavior on arbitrary hardware. [SQLite synchronous pragma documentation](https://sqlite.org/pragma.html#pragma_synchronous)

**What process-kill tests CAN prove:**
- Acknowledged commits survive process termination.
- Unacknowledged transactions may or may not survive (permitted uncertainty).
- The application does not report false success after a kill.

**What process-kill tests CANNOT prove:**
- Acknowledged commits survive power loss.
- Acknowledged commits survive device failure.
- The storage stack honors fsync semantics.
- Behavior on arbitrary hardware.

**Verdict:** READY (as a statement of limitation). This is correctly scoped in the protocol and does not need owner input.

---

## 3. Numerical budget review with synchronous=FULL

**SQLite semantics (official sources):**
- synchronous=FULL adds an fsync of the WAL file after each commit. This is the most durable setting in WAL mode. [SQLite synchronous pragma documentation](https://sqlite.org/pragma.html#pragma_synchronous)
- WAL mode provides reader/writer overlap: readers do not block the writer and the writer does not block readers. [SQLite WAL documentation](https://sqlite.org/wal.html)

No official SQLite source provides device-specific fsync latency ranges or OxiDJ-specific performance feasibility assessments. Statements such as "fsync costs 0.1-1ms on NVMe" or "p95 should be <10ms on NVMe" are generic estimates from third-party benchmarks, not OxiDJ evidence and not supported by any cited official source. The following budgets are provisional engineering targets proposed in the draft protocol; they are NOT evidence-backed measurements and must be validated by the prototype run before they can serve as acceptance criteria.

**Budget assessment:**

| Budget | Value | Status |
|--------|-------|--------|
| Read p95 | <=100ms | Provisional target; reads do not fsync under FULL. Feasibility unmeasured. |
| Read p99 | <=250ms | Provisional target; feasibility unmeasured. |
| Manual-edit commit p95 | <=100ms | Provisional target; single-row update with FULL fsync. Feasibility unmeasured. |
| Manual-edit commit p99 | <=250ms | Provisional target; feasibility unmeasured. |
| Submission-to-BEGIN p95 | <=100ms | Provisional target; queue wait under target load. Feasibility unmeasured. |
| Submission-to-BEGIN p99 | <=500ms | Provisional target; feasibility unmeasured. |
| Transaction duration p99 | <=50ms | Provisional target; short batched transaction with FULL fsync. Feasibility unmeasured. |
| Transaction duration max | <=250ms | Provisional tail bound; not a target. Feasibility unmeasured. |
| RSS | <=1GiB | Provisional target; feasibility unmeasured. |
| Post-drain WAL | <=64MiB | Provisional post-checkpoint target. Not a live bound (see 2.6). |
| Queue | 1024 jobs/64MiB | Provisional capacity bound, not a performance target. |
| Overload recovery | <=60s | Provisional target; drain after 60s at 2x writes. Feasibility unmeasured. |
| Migration | <=60s | Provisional target; schema migration on 500k rows. Feasibility unmeasured. |
| Reopen | <=2s | Provisional target; open WAL db + no-op migration check. Feasibility unmeasured. |
| Restore | <=120s | Provisional target; restore + first query. Feasibility unmeasured. |

**Conclusion:** The numerical budgets are provisional engineering targets from the draft protocol. No official SQLite source or repository evidence supports device-specific performance feasibility for these targets. They are NOT evidence-backed measurements and must be reviewed before execution. They should not be presented as product guarantees or OxiDJ evidence.

**Verdict:** NOT READY (pending owner review of budget feasibility).

---

## 4. READY/NOT READY checklist

| Item | Verdict | Smallest missing evidence/decision |
|------|---------|-----------------------------------|
| Fixture distributions | NOT READY | Owner-approved payload sizes and fan-out ratios |
| Query class proportions | NOT READY | Owner choice of the read-class set and representative or synthetic mix; manual-edit rate remains separate unless explicitly included |
| Manual edit rate | NOT READY | Owner-specified rate |
| Arrival schedule | NOT READY | Owner approval of open-loop model |
| Percentile estimator | NOT READY | Owner approval of exact histogram and bucket width |
| Minimum samples | NOT READY | Owner-approved class set/rates and statistical adequacy target; ~3,600 samples/class is illustrative only |
| Cold-cache procedure | NOT READY | Validated controlled-runner method (pre-provisioned untouched fixture or authorized host-wide `drop_caches`) |
| Offered/accepted/completed accounting | NOT READY | Owner ratification of accounting rules |
| Workload duration | NOT READY | Owner approval of warm-up exclusion and the 30-minute duration-sufficiency hypothesis (unmeasured; insufficiency is an explicit risk) |
| Overload reader schedule | NOT READY | Owner-specified reader behavior during overload |
| Backpressure semantics | NOT READY | Owner ratification of rejection + retry policy |
| Live WAL bound | NOT READY | Owner approval of 256 MiB bound |
| Free-disk floor | NOT READY | Owner approval of 1 GiB floor |
| Reader lifetime policy | NOT READY | Owner approval of 5s limit |
| Checkpoint starvation scenario | NOT READY | Owner approval of new test |
| Semantic oracle (items 1-5) | NOT READY | Owner ratification of oracle definition |
| Semantic oracle (item 6: relationship-aware) | NOT READY | ADR-005 schema decision |
| Migration cases | NOT READY | ADR-005 schema decision |
| Restore oracle | NOT READY | Owner ratification of snapshot comparison method |
| Acknowledgment witness | NOT READY | Owner ratification of witness format and fsync ordering |
| Sync/I/O fault points | NOT READY | Owner choice of injection method (FUSE vs LD_PRELOAD) |
| Process-kill limitations | READY | Correctly scoped; no owner input needed |
| Numerical budgets | NOT READY | Owner review of budget feasibility with FULL |
| Reference device | NOT READY | viper is a candidate; pending owner selection and pre-run state re-recording |
| Durability contract | READY | WAL/FULL, commit-success ack, conditional zero-loss |
| Engine choice | READY | SQLite + WAL + rusqlite + rusqlite_migration |
| Concurrency model | READY | Single writer, read pool, BEGIN IMMEDIATE |

---

## 5. Recommended path to READY

1. **Owner ratification of defaults:** the definitions proposed in sections 2.1-2.11 are executable defaults. The owner can approve them, modify them, or replace them. This is the largest unblock.
2. **ADR-005 schema decision:** unblocks migration cases, relationship-aware semantic oracle, and fixture generation. This is on a separate track and does not block the non-schema items.
3. **Fixture generation:** once ADR-005 is resolved, generate a deterministic 500k-track fixture with the approved distributions. This unblocks the workload.
4. **Sample-count review:** once query-class proportions and the reader rate are frozen, validate that each class yields enough samples for the acceptance percentiles (p95/p99). Under the illustrative five-read-class, reads-only mix in §2.1, the proposed ~3,600 samples/class gives ~36 upper-tail observations for p99; this is not a statistical confidence target, which remains unresolved. The same count does NOT support p99.9 (~3.6 tail observations); p99.9 is not an acceptance metric without a separately justified sample plan. Do not promote provisional targets to accepted criteria without owner review.
5. **Controlled-runner authorization:** validate a cold-cache setup on a dedicated disposable VM/host, for example by pre-provisioning the fixture and leaving it untouched since boot; any use of `drop_caches` requires host-owner approval for host-wide cache eviction and must not affect unrelated workloads.
6. **Protocol freeze:** once all checklist items are READY, freeze the protocol. Per the pre-run gate in plan/STATUS.md:62 and ADR-002:11,119-123, do not run benchmarks or feasibility measurements until the protocol is executable and reviewed; numerical budgets remain provisional until pre-run review (m1-sqlite-admission-protocol.md:44). The earlier "quick feasibility check" proposed before freeze conflicts with this gate and was not run.
7. **Feasibility check and runs (post-freeze only):** only after the protocol is frozen and owner-approved, conduct a feasibility check of the numerical budgets against `synchronous=FULL` on the qualified reference device, then execute the three independently reset runs (cold-open and warm measurements; every run must pass).

---

## 6. SQLite citations

All SQLite semantics in this note are sourced from official documentation:

- [SQLite WAL mode](https://sqlite.org/wal.html) — reader/writer overlap, checkpoint starvation, WAL file growth
- [SQLite synchronous pragma](https://sqlite.org/pragma.html#pragma_synchronous) — FULL adds fsync per commit; NORMAL may roll back after power loss
- [SQLite howtocorrupt.html, section 3.1](https://sqlite.org/howtocorrupt.html) — a failed sync during COMMIT in WAL mode may lose durability without corrupting the database file; only checkpoint sync failures risk corruption
- [SQLite rescode.html — SQLITE_IOERR / SQLITE_IOERR_FSYNC](https://sqlite.org/rescode.html#ioerr) — an fsync failure may surface as an I/O error (primary code SQLITE_IOERR by default; SQLITE_IOERR_FSYNC is an extended code returned only when extended result codes are enabled); commit outcome is uncertain
- [SQLite journal_size_limit pragma](https://sqlite.org/pragma.html#pragma_journal_size_limit) — size limit on rollback-journal/WAL files left after transactions/checkpoints (truncation target applied on commit / WAL-reset); NOT a checkpoint trigger (`wal_autocheckpoint` is); default limit -1 = no limit
- [SQLite atomic commit](https://sqlite.org/atomiccommit.html) — rollback-journal atomicity and the dependence of commit safety on underlying flush behavior (this article documents only the rollback-journal mechanism; WAL-mode atomicity and FULL/NORMAL power-loss behavior are covered by the synchronous pragma and WAL documentation above)
- [SQLite foreign keys pragma](https://sqlite.org/pragma.html#pragma_foreign_keys) — not persistent, must be set per connection

---

## 7. Boundaries respected

- No ADR-002 durability wording changed; no universal zero-loss claim made.
- No SQLite dependencies added; no schema implemented; no benchmark run; no private music libraries accessed; no paid model routing used.
- No existing shared docs modified. This note is a revised, self-contained file in the isolated worktree. Changes are limited to correcting source references, durability semantics, statistical framing, and demoting provisional proposals; no new evidence or measurements were generated.
- All SQLite semantics cited to official sources.
- All NOT READY items explicitly marked with smallest missing evidence/decision.
- The viper reference device is recorded as a candidate, not an accepted fact.
- Workload budgets are recorded as provisional proposals, not accepted criteria.
- The cold-cache procedure is NOT READY pending controlled-runner authorization.
