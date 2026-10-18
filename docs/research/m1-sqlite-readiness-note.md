# M1 SQLite prototype readiness note

**Task:** t_45a179ff
**Author:** sova (Researcher)
**Date:** 2026-10-15
**Scope:** Independent executability assessment of the 500k-track SQLite prototype protocol. This is a readiness proposal, not a benchmark result. It follows the accepted ADR-002 engine/durability decision and does not reopen it.

---

## Verdict: NOT READY

The readiness assessment remains **NOT READY** after owner ratification: remaining work is technical—freeze the synthetic workload, complete the semantic oracle, and implement/test enforcement. Owner decisions are recorded; no additional approval is needed for these reversible technical choices. The initial assessment below records proposal-time findings; the current checklist and updates supersede the old pending-approval labels.

## Owner ratification update (2026-10-15)

The owner approved the recommendations recorded in the canonical admission protocol:

- **Finite-campaign sampling:** scheduled open-loop offers; five-minute fixed warm-up outside 30 measured minutes; three separate runs; exact nearest-rank p95/p99 per class/run; descriptive floors of 2,000 p95 and 10,000 p99 observations per class/run; under-floor cells are NOT DEMONSTRATED, not PASS; no pooling or population-confidence claim. Extend the common measured window after the class mix is set if needed to reach the p99 floor for every in-scope class. These floors are not confidence guarantees.
- **Recovery:** coverage-first P0 matrix replaces 30 kills as acceptance definition; independent commit/ack witness, exact operation/history oracle, two stable-boundary online backup restores, three deterministic seeds per applicable site/shape. P1 OS/physical interruption is separate from generic dependency admission and requires separate authorization/evidence for a named deployment/support claim.
- **WAL/disk and qualification budgets:** first-prototype pass/fail targets are approved, unmeasured, and not product guarantees. The current live-WAL/checkpoint/free-space envelope and enforcement rules are in `m1-sqlite-admission-protocol.md`; `T` must be bounded before claiming the hard ceiling. No total-storage cap until fixture, assets, backup scope, and retention are fixed.
- **Cold-cache:** use a disposable pre-provisioned VM or dedicated runner; do not clear caches on the normal workstation.
- **ADR-005:** source-native records + neutral projection + edit overlays are approved only as a candidate direction for drafting and fixture validation, not as final schema acceptance.

Still open as lead-owned technical inputs: exact read-class taxonomy/proportions/selectivities and manual-edit share (manual edits are included in the approved 10 requests/s total), analysis payloads, scan-insert/growth and transaction-shape assumptions, synthetic test-only asset and backup fixtures/qualification work, executable VFS/witness/backup oracles, ADR-005 relationship fixtures, reference-device revalidation, and selected bundled SQLite build. Product audio/artwork copy/reference and product backup-retention policy remain unsettled and owner-reserved; this note does not resolve that product intent. D1 inclusion topology is closed; the illustrative 80/20 split is not ratified and no additional owner decision is required for the listed technical inputs. No benchmark, fault experiment, prototype, or persistence dependency has started or is authorized before the full protocol and semantic oracle are independently reviewed.

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

These workload and budget items originated as proposals. Since this assessment, the owner ratified the finite-campaign sample rules, first-prototype acceptance targets, recovery P0/P1 policy, WAL/disk envelope, and disposable cold-cache method in the update above. Those inputs are now owner-approved but remain unmeasured and are not product guarantees. Still open are payload/fan-out and scan-growth assumptions, read-class proportions/selectivities, manual-edit share, assets/backups/retention, executable overload and recovery harness details, the per-batch WAL-growth bound, and ADR-005-dependent semantics. The 20% per-class illustration remains unratified and is not a representative mix.

| Proposal | Source | Status |
|---------|--------|--------|
| Deterministic 500k-track fixture, deterministic seed (proposed default workload) | m1-sqlite-admission-protocol.md:27 | Base size/seed approved 2026-10-15; payload distribution/fan-out and fixture specifics remain open (NOT an ADR-002 accepted fact) |
| 30-minute measured mixed load, 2 producers (100 scan inserts/s + 100 analysis updates/s), 4 readers (10 req/s total including manual edits) | m1-sqlite-admission-protocol.md:30 | Base rates and manual-edit inclusion topology approved; 5-minute warm-up, three separate runs, finite-campaign method and duration-extension rule approved; class proportions/selectivities, payload and manual-edit share remain lead-owned technical inputs |
| Read p95<=100ms, p99<=250ms; write submission-to-BEGIN p95<=100ms, p99<=500ms; txn duration p99<=50ms, max 250ms | m1-sqlite-admission-protocol.md:32 | Owner-approved first-prototype targets; unmeasured and not product guarantees |
| Queue bound 1024 jobs/64MiB; RSS<=1GiB; post-checkpoint WAL<=64MiB; live WAL<=256MiB | m1-sqlite-admission-protocol.md:32-33 | Owner-approved first-prototype targets; hard live ceiling requires a validated per-batch growth bound; total storage remains open |
| Overload 2x writes for 60s, explicit backpressure, recover within 60s | m1-sqlite-admission-protocol.md:32 | Target approved; reader schedule and exact rejection/retry semantics still need executable specification |
| Migration<=60s, reopen<=2s, restore<=120s | m1-sqlite-admission-protocol.md:36 | Owner-approved first-prototype targets; unmeasured; schema-dependent implementation remains open |
| 30 process kills across transactions/checkpoints/migration | m1-sqlite-admission-protocol.md:37 | Superseded: owner approved coverage-first P0 matrix; 30 kills alone are not the acceptance definition |
| Two online backups, stable-boundary restore without original DB/WAL | m1-sqlite-admission-protocol.md:37 | Two qualification restores approved; product asset/backup retention scope remains open |
| Reference device `viper` (Ryzen 5 5600X, 31GiB, Btrfs zstd:3 over LUKS on NVMe) | m1-sqlite-admission-protocol.md:28 | Lead-selected candidate; revalidate full environment at run start |

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

## 2. Proposal-time gap detail (current status is summarized above)

### 2.1 Query/arrival/sample accounting

**Proposal-time state (superseded by the owner decision summarized above):** The earlier protocol listed interactive operations (track lookup, selective/broad search, sorted first/next 100-row pages, playlist pages, manual edits) and a 10 req/s aggregate reader rate without defining whether edits were included. The owner subsequently closed the D1 inclusion topology: manual edits are inside the 10/s total. The current unresolved items are the read-class grouping/proportions/selectivities and exact manual-edit share. The proposal-time assessment identified these gaps:
- The proportion of each query class
- Whether arrivals are open-load (Poisson/fixed-rate independent of completion) or closed-loop (wait for response before next request)
- The percentile estimator (e.g., reservoir sampling, t-digest, exact histogram)
- Minimum sample counts per class per run
- Cold-cache procedure

**Proposal-time definitions (current owner-approved choices are summarized at the top; remaining specifics are lead-owned technical defaults):**

- **Arrival model:** owner-approved scheduled open-loop. Generate offers independently of completion from a predeclared deterministic schedule; report actual inter-arrival jitter and all terminal outcomes.
- **Percentile estimator:** retain raw monotonic durations; calculate exact nearest-rank p95 and p99 per class and run. Do not use approximate histogram buckets for pass/fail. p99.9 is not an acceptance percentile.
- **Minimum samples:** owner-approved descriptive rank-resolution floors are 2,000 observations for p95 and 10,000 for p99 per class/run. These are not statistical confidence guarantees. Under-floor cells are NOT DEMONSTRATED and cannot pass; keep runs separate, with no pooling or post-hoc regrouping. After the synthetic class rates are frozen, extend the common measured window to satisfy the p99 floor for each in-scope positive-rate class.
- **Cold-cache procedure:** owner-approved disposable pre-provisioned VM or dedicated runner. Validate that environment before qualification. Do not use host-wide `drop_caches` on the normal workstation.
- **Query-class mix:** if representative usage data is unavailable, use a deterministic synthetic coverage profile only; clearly state that it does not estimate typical OxiDJ usage. The lead owns the exact class proportions, selectivities, and manual-edit share as reversible test inputs.

**Verdict:** These owner-level method choices are resolved. The protocol remains NOT READY until the lead freezes a concrete synthetic schedule, exact per-class rates, payload/growth inputs, and the executable accounting harness; no further owner approval is needed.

### 2.2 Latency estimators and minimum samples

**At assessment time:** p95/p99 were mentioned without an estimator or minimum sample count; the owner-approved method and floors are recorded above.

**Current approach:** raw duration samples, exact nearest-rank p95/p99 per class and run; owner-approved descriptive floors are 2,000 samples for p95 and 10,000 for p99. These floors are not confidence guarantees. Run separately; no pooling. Extend the common measurement interval from the synthetic class rates so all in-scope classes can meet the p99 floor.

**Verdict:** Sampling method and descriptive floors are owner-approved; exact class rates and sufficient duration remain to be frozen from the synthetic coverage profile.

### 2.3 Offered/accepted/completed write accounting

**At assessment time:** The protocol said "count blocked/rejected submissions, no lost accepted work" and "completed throughput must match offered throughput" under target load, but did not define the accounting boundaries below. The following are lead-owned executable defaults, not requests for owner approval:
- The boundary between "offered" and "accepted" (is queue admission the acceptance point?)
- Whether rejected work at target load is a failure or expected behavior
- How to account for writes that are accepted but whose transaction fails at commit

**Proposed definitions:**

- **Offered:** a producer attempts to submit a write job to the queue. Record timestamp and payload bytes.
- **Accepted:** the job is admitted into the bounded queue. Record timestamp, queue depth, and payload bytes.
- **Completed:** the job's transaction commits successfully and the acknowledgement is sent. Record timestamp.
- **Rejected:** the queue is full (1024 jobs or 64MiB). Record timestamp, payload bytes, and rejection reason.
- **Failed:** the job was accepted but the transaction failed at commit. Record timestamp and error. A failed job is NOT a lost accepted work if the failure is reported to the producer and the producer may retry; it IS a lost accepted work if the failure is silent.

**Accounting rule:** every offer has one recorded terminal outcome: completed after commit and acknowledgement; rejected before queue acceptance; or failed after acceptance. No silent drops. At normal target load, any rejection or failure prevents PASS because completed throughput must match offered. During the 2x overload interval, explicit queue-full rejections are permitted but reported; no retry is issued during that interval so scheduled arrivals stay fixed. A later retry is a new offer. Backlog is accepted but not-yet-terminal work, sampled each second. These are lead-owned test-harness rules; no further owner approval is needed.

**Backlog metric:** accepted minus completed, sampled every second. Report as a time series, not just a final value.

**Verdict:** The accounting rules are defined; executable instrumentation and independent review remain before protocol freeze.

### 2.4 Workload duration

**Current state:** Owner-approved base is five minutes of warm-up outside 30 measured minutes, with three independent runs; the measured window must extend to satisfy every in-scope p99 class floor.

**Assessment:** 30 measured minutes is the approved baseline, not a promise that all classes meet the p99 sample floor. The common measured window will be extended from the final synthetic class rates. This is a finite-campaign qualification, not a population-confidence study. Long-duration effects outside the frozen test envelope are not claimed.

**Current method:** five-minute warm-up, then a common measured duration of at least 30 minutes and long enough for each in-scope p99 class to reach 10,000 samples; three independently reset runs. The warm-up is excluded from percentile calculations but its offered work and fixture growth are recorded.

**Verdict:** Five-minute warm-up outside 30 measured minutes is owner-approved; extend the duration after the in-scope mix is fixed to meet per-class p99 floors.

### 2.5 Overload/backpressure

**Current state:** The owner approved this overload target. The protocol now specifies normal-rate readers, reported queue-full rejection without retry during the fixed 60-second burst, then recovery until accepted backlog is drained and returns to its pre-burst baseline within 60 seconds.

**Gaps:**
- What is the reader schedule during overload? Do readers continue at 10 req/s or are they throttled?
- What does "explicit backpressure" mean operationally? Reject new writes? Slow producers? Both?
- How is "recover/drain within 60s" measured? From the end of the overload period?

**Lead-owned executable defaults:** readers continue at 10 interactive requests/s. When the queue is full, each offered write gets an immediate, counted `Busy` rejection; no retry is issued during the fixed 60-second overload interval, preserving the scheduled arrival stream. At overload end, resume base rates and measure until all previously accepted work is complete and backlog returns to its pre-burst baseline; require <=60 seconds. Target-load rejection or failure is a run failure.

**Verdict:** No further owner input is needed; the implementation and independent review remain open.

### 2.6 Live WAL/free-disk bounds/checkpoint starvation

**Current state:** the owner approved a 128 MiB high-water response, 256 MiB live-WAL qualification ceiling, <=64 MiB only after verified checkpoint/recovery, managed read snapshots <=5 seconds, and >=1 GiB free after operation reservation with resume only at >=2 GiB plus headroom. `journal_size_limit` remains a residual size control, not a live cap.

**SQLite semantics (official sources):**
- WAL mode uses a shared-memory index (shm file) and a write-ahead log (wal file). The WAL file grows as transactions commit and is reset when a checkpoint completes. [SQLite WAL documentation](https://sqlite.org/wal.html)
- Checkpoint starvation occurs when readers prevent the WAL from being fully checkpointed, causing the WAL file to grow without bound. [SQLite WAL documentation, section 6](https://sqlite.org/wal.html)
- journal_size_limit is NOT a checkpoint trigger. Per the official pragma docs, it limits the size of rollback-journal and WAL files **left in the file-system after transactions or checkpoints**: each time a transaction commits or a WAL file resets, SQLite compares the WAL size to the limit and truncates it if larger (the default limit is -1 = no limit). Checkpoints are triggered by `wal_autocheckpoint` (default 1000 pages), not by `journal_size_limit`. The WAL can still exceed the limit between checkpoints (e.g., during checkpoint starvation, when no checkpoint runs to apply the truncation), so this is not a live hard cap — see the proposed live-WAL bound in 2.6. [SQLite pragma documentation](https://sqlite.org/pragma.html#pragma_journal_size_limit)

**Executable WAL/disk policy:** at 128 MiB, stop starting new writes and apply explicit backpressure. Enforce the 256 MiB live ceiling only when the harness has a conservative per-batch WAL-extension bound `T` and admits work only when `W+T<=256 MiB`; otherwise split/reject rather than claim a hard cap. After a real reader gap, run a bounded TRUNCATE checkpoint and verify residual WAL <=64 MiB. Keep managed read snapshots <=5 seconds; enforce >=1 GiB free after reserving the next operation's conservative allocation, and do not resume until >=2 GiB plus headroom. No product-wide storage cap is inferred.

**Checkpoint-starvation test:** hold a reader while writers run; continue until the 128 MiB response activates or 60 seconds elapse. At the threshold, stop admitting writes, verify backpressure, release the reader, checkpoint/TRUNCATE, and verify the residual target. If neither the threshold nor a valid timeout boundary is reached, record NOT DEMONSTRATED. This is a technical test definition, not an additional owner decision.

**Verdict:** Owner thresholds are recorded; executable `T` admission, free-space reservations, and checkpoint-starvation instrumentation remain technical prerequisites.

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

**Lead-owned qualification oracle:** take two online backups at separately recorded stable boundaries and restore each into a clean location without the original database or WAL. Compare every persisted row with the stable-boundary snapshot, then run the semantic oracle from ADR-005. Use a deterministic test-only asset-stub manifest with SHA-256 entries; this does not settle product audio/artwork copy/reference policy or ongoing retention. Measure restore-to-first-result against the owner-approved 120-second target; record full verification time separately.

**Verdict:** Backup/restore cases are owner-approved; comparison, manifest, and timing instrumentation remain technical implementation work.

### 2.10 Acknowledgment witness

**Current state:** "Independently persist a witness of acknowledged transaction IDs; verify it is not in-memory or unflushed."

**Lead-owned witness default:** append each transaction ID to a separate application-layer witness file; after the SQLite transaction commits, sync the witness before acknowledging success. Reconcile the witness against committed database IDs after each recovery case: every acknowledged ID must be present; unacknowledged IDs may or may not be present. A witness-write/sync failure prevents acknowledgement. Independent review must verify ordering and uncertainty handling.

**Verdict:** The witness requirement is owner-approved; concrete implementation and independent validation remain.

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

**Injection method:** use a test-only SQLite VFS wrapper/shim around the selected underlying VFS, with deterministic failure counters at the required write/sync/recovery sites. Before qualification, prove the wrapper's exact error propagation through the selected SQLite build and Rust binding; invalid or unreachable cells are NOT DEMONSTRATED. This is a lead-owned technical implementation choice, not a user approval gate.

**Verdict:** The coverage-first matrix is owner-approved; the VFS shim, error mapping, and executable case manifest remain technical prerequisites.

### 2.12 What process-kill tests cannot prove

**Current state:** "Process kills are not power-loss simulations; the zero-loss target is conditional on the named supported stack honoring synchronization."

**SQLite semantics (official sources):**
- A process kill terminates the application but does not clear kernel caches or simulate device power loss. Committed transactions with synchronous=FULL survive process kills because the WAL fsync has already been issued to the kernel. [SQLite WAL documentation](https://sqlite.org/wal.html)
- Power loss clears kernel caches and may leave the WAL file in an incomplete state. Whether the commit survives depends on the storage device's behavior during power loss: only with `synchronous=FULL` does SQLite issue a WAL sync at each commit to make transactions durable across power loss, and `synchronous=NORMAL` may lose committed transactions after power/system failure. This is a WAL-mode behavior; the official *atomic commit* article documents only the rollback-journal mechanism and does not cover WAL-mode power-loss behavior. [SQLite WAL documentation](https://sqlite.org/wal.html); [SQLite synchronous pragma](https://sqlite.org/pragma.html#pragma_synchronous)
- A finite coverage-first process-kill matrix provides bounded evidence only for the reachable cells actually exercised: acknowledged commits survive the specific tested kill boundaries and the application does not report false success. It does NOT prove correct handling of all kill points, all failure modes, power loss, device failure, or behavior on arbitrary hardware. [SQLite synchronous pragma documentation](https://sqlite.org/pragma.html#pragma_synchronous)

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

No official SQLite source provides device-specific fsync latency ranges or OxiDJ-specific performance feasibility assessments. Statements such as "fsync costs 0.1-1ms on NVMe" or "p95 should be <10ms on NVMe" are generic estimates from third-party benchmarks, not OxiDJ evidence. The owner ratified the protocol's numeric values as first-prototype pass/fail targets. They remain unmeasured, not product guarantees, and feasibility is unknown until the post-freeze qualification run.

**Budget assessment:**

| Budget | Value | Status |
|--------|-------|--------|
| Read p95 | <=100ms | Owner-approved first-prototype target; reads do not fsync under FULL. Feasibility unmeasured, not a product guarantee. |
| Read p99 | <=250ms | Owner-approved first-prototype target; feasibility unmeasured, not a product guarantee. |
| Manual-edit commit p95 | <=100ms | Owner-approved first-prototype target; single-row update with FULL fsync. Feasibility unmeasured. |
| Manual-edit commit p99 | <=250ms | Owner-approved first-prototype target; feasibility unmeasured, not a product guarantee. |
| Submission-to-BEGIN p95 | <=100ms | Owner-approved first-prototype target; queue wait under target load. Feasibility unmeasured. |
| Submission-to-BEGIN p99 | <=500ms | Owner-approved first-prototype target; feasibility unmeasured, not a product guarantee. |
| Transaction duration p99 | <=50ms | Owner-approved first-prototype target; short batched transaction with FULL fsync. Feasibility unmeasured. |
| Transaction duration max | <=250ms | Owner-approved first-prototype maximum bound; not a percentile target. Feasibility unmeasured. |
| RSS | <=1GiB | Owner-approved first-prototype target; feasibility unmeasured, not a product guarantee. |
| Post-checkpoint WAL | <=64MiB | Owner-approved residual target. Not a live bound (see 2.6). |
| Queue | 1024 jobs/64MiB | Owner-approved capacity bound, not a performance target. |
| Overload recovery | <=60s | Owner-approved first-prototype target; drain after 60s at 2x writes. Feasibility unmeasured. |
| Migration | <=60s | Owner-approved first-prototype target; schema migration on 500k rows. Feasibility unmeasured. |
| Reopen | <=2s | Owner-approved first-prototype target; open WAL db + no-op migration check. Feasibility unmeasured. |
| Restore | <=120s | Owner-approved first-prototype target; restore + first query. Feasibility unmeasured. |

**Conclusion:** The owner-approved values are first-prototype acceptance targets, not measurements or product guarantees. No official SQLite source or repository evidence supports device-specific feasibility. The target set is ready as a decision; execution remains blocked on protocol/oracle completion and independent review.

**Verdict:** Budget values owner-approved; feasibility remains unmeasured.

---

## 4. Current gate checklist after owner ratification

| Item | Current status | Next work (lead-owned; no further owner approval needed) |
|------|----------------|-----------------------------------------------------------|
| Initial fixture and base rates | Owner-approved: 500k deterministic tracks; 100 scan inserts/s; 100 analysis updates/s; 10 interactive requests/s total | Define synthetic fixture payloads and growth semantics; no typical-usage claim |
| Read/manual-edit class mix | Synthetic fallback approved if real usage is unknown | Freeze a balanced, deterministic coverage schedule and selectivities |
| Arrival and percentile method | Owner-approved: scheduled open-loop; raw durations; nearest-rank p95/p99 per class/run | Implement event schedule and retain all terminal outcomes |
| Sample floors and duration | Owner-approved: 2,000 p95 / 10,000 p99 per class/run; three separate runs; under-floor NOT DEMONSTRATED | Derive common measured duration from the final class rates |
| Warm-up | Owner-approved: five minutes outside 30 measured minutes | Apply consistently; extend measured window as needed for p99 floors |
| Cold-cache method | Owner-approved: disposable pre-provisioned VM/runner | Provision and validate runner; no cache clearing on normal workstation |
| Latency/resource targets | Owner-approved first-prototype thresholds; unmeasured, not product guarantees | Implement measurements and report each run separately |
| WAL/checkpoint/free-space envelope | Owner-approved: 128 MiB high-water, 256 MiB live ceiling, <=64 MiB after verified checkpoint/recovery, <=5 s snapshots, >=1 GiB free after reservation, resume at >=2 GiB plus headroom | Derive and validate per-batch growth bound `T`; implement pause/backpressure/checkpoint verification |
| Overload and accounting | Target and terminal-outcome accounting approved | Specify deterministic reader/rejection behavior and recovery timer |
| Recovery matrix | P0 coverage-first matrix, witness/oracle, two backup restores and three seeds approved; P1 separate | Implement injectors, durable witness, restore oracle, and case manifest |
| ADR-005 direction | Candidate approved for drafting only; no final schema accepted | Draft schema and pass relationship-aware import-edit-export fixtures |
| Reference device / SQLite build | `viper` remains a candidate; exact build unselected | Lead selects/revalidates runner and checks bundled SQLite WAL-reset fix before qualification |
| Total storage budget | No product-wide cap approved | Measure the frozen synthetic fixture/assets/backups; do not infer a product cap |
| Durability and engine choice | READY: WAL/FULL conditional contract; SQLite + WAL | Dependency admission still waits for every qualification gate |

---

## 5. Recommended path to READY

1. **Finalize synthetic coverage inputs:** choose explicit read classes, equal scheduled shares, deterministic query/selectivity cases, transaction/payload shapes, scan-growth behavior, and test-only asset/backup scope. Label the workload synthetic; make no typical-usage claim.
2. **ADR-005:** draft the approved candidate direction and relationship-aware import-edit-export fixtures; run the required independent decision review before accepting a schema.
3. **Build the qualification harness:** implement per-class raw-sample accounting, queue/backpressure outcomes, bounded `T` admission, WAL/checkpoint verification, independent acknowledgement witness, VFS fault injection, and backup/restore oracle.
4. **Cold-cache runner:** provision and verify a disposable pre-seeded VM/dedicated runner; no cache clearing on the normal workstation.
5. **Freeze and independently review:** all workload cells, budgets, harness, recovery matrix, semantic oracle, supported stack, and SQLite build must be executable and frozen before any measurement.
6. **Run only after freeze:** execute the three separate qualification runs and report each cell/run; no pooling to rescue a failure or under-sampled class. No persistence dependency unless every agreed gate passes.

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
- No SQLite dependencies or product code were added, and no benchmark/fault experiment was run; no private music libraries were accessed and no paid model routing was used. This note is now the canonical readiness record and is updated alongside the admission protocol and milestone status.
- All remaining NOT READY items have a lead-owned execution path in the current gate checklist; none waits for additional owner approval.
- The viper host is a lead-selected reference candidate; exact environment details must be revalidated at run start.
- Owner ratified first-prototype budgets and the disposable cold-cache method; feasibility and runner provisioning remain unverified.
- This file's detailed gap sections are a pre-ratification review snapshot; the owner-update and current checklist above supersede stale proposal-time statuses.
