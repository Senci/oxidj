# M1 SQLite prototype readiness note

**Task:** t_45a179ff
**Author:** sova (Researcher)
**Date:** 2026-10-15
**Scope:** Independent executability assessment of the 500k-track SQLite prototype protocol. This is a readiness proposal, not a benchmark result. It follows the accepted ADR-002 engine/durability decision and does not reopen it.

---

## Verdict: NOT READY

Owner-ratified numeric values are unchanged first-prototype pass/fail criteria only; they remain unmeasured and are not product SLOs. Their executable application remains gated by pre-run review and every existing admission prerequisite; unresolved prerequisites mean NO-GO. This clarification neither revises targets nor authorizes execution. The current live gate remains NOT READY; the checklist and updates below supersede the old pending-approval labels.

## Owner ratification update (2026-10-15)

The owner approved the recommendations recorded in the canonical admission protocol:

- **Finite-campaign sampling:** scheduled open-loop offers; five-minute fixed warm-up outside 30 measured minutes (historical approval); the current lead-owned duration is 7,000 measured seconds/run, derived from seven `10/7 req/s` classes and 10,000 p99 observations/class — a lead-owned derivation, not an owner decision; three separate runs; exact nearest-rank p95/p99 per class/run; descriptive floors of 2,000 p95 and 10,000 p99 observations per class/run; under-floor cells are NOT DEMONSTRATED, not PASS; no pooling or population-confidence claim. Extend the common measured window after the class mix is set if needed to reach the p99 floor for every in-scope class. These floors are not confidence guarantees.
- **Recovery:** coverage-first P0 matrix replaces 30 kills as acceptance definition; independent commit/ack witness, exact operation/history oracle, two stable-boundary online backup restores, three deterministic seeds per applicable site/shape. P1 OS/physical interruption is separate from generic dependency admission and requires separate authorization/evidence for a named deployment/support claim.
- **WAL/disk and qualification budgets:** first-prototype pass/fail targets are approved, unmeasured, and not product guarantees. The current live-WAL/checkpoint/free-space envelope and enforcement rules are in `m1-sqlite-admission-protocol.md`; `T` must be bounded before claiming the hard ceiling. No total-storage cap until fixture, assets, backup scope, and retention are fixed.
- **Cold-cache:** use a disposable pre-provisioned VM or dedicated runner; do not clear caches on the normal workstation.
- **ADR-005:** source-native records + neutral projection + edit overlays are approved only as a candidate direction for drafting and fixture validation, not as final schema acceptance.

Still open: analysis-update payload/field semantics, transaction/batch shape, backup implementation/publication/isolation/fault harness (the frozen qualification-only B1/B2 asset/restore contract is protocol-defined, not implemented or measured), executable VFS/witness/backup oracles, ADR-005 relationship fixtures (including semantic realization of the frozen coverage-v1 profile), reference-device selection, and selected bundled SQLite build. A logical-job/atomic-batch/post-COMMIT acknowledgement envelope is recorded in the admission protocol as a proposal-only candidate, not accepted or frozen; retry identity, durable witness/reconciliation, batch limits/fairness, WAL-growth `T`, and ADR-005-dependent semantics remain open. ADR-002:143 still describes proposed budgets as provisional pending pre-run review; this wording caveat does not change accepted values. Product audio/artwork copy/reference and product backup-retention policy remain unsettled and owner-reserved; this note does not resolve that product intent. Frozen qualification inputs: coverage-v1 logical profile (payload budgets and fan-out slots as abstract generator-side quantities), scan-insert growth, seven-class equal-share interactive schedule at 10/7 req/s per class, 300s warm-up, 7,000 measured seconds per run, three independent runs, no pooling. `viper` remains a candidate only. No benchmark, fault experiment, prototype, or persistence dependency has started or is authorized before the full protocol and semantic oracle are independently reviewed.

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

These workload and budget items originated as proposals. Since this assessment, the owner ratified the finite-campaign sample rules, first-prototype acceptance targets, recovery P0/P1 policy, WAL/disk envelope, and disposable cold-cache method in the update above. Those inputs are now owner-approved but remain unmeasured and are not product guarantees. The coverage-v1 logical profile (assigned payload-budget ceilings `B`, all nine initial and scan-offer cell counts, fixed seed/order, and fan-out slots), scan-growth semantics, read-class proportions/selectivities, manual-edit share, and measured duration are frozen qualification inputs. For each record, `A` is independently justified realized eligible logical byte-token count and must satisfy `0 <= A <= B` once counting rules exist; existing totals remain sums of assigned `B`, not realized `A`, measurements, or storage. No lower bound or occupancy threshold is approved. C1 remains OPEN until eligible-field/type/encoding/ownership/exclusion rules, an independently justified inventory/allocation, measurable meaningful-coverage acceptance, and C2/C3 compatibility are specified and independently reviewed. The ceiling alone cannot qualify tiny/empty payloads. Still open are analysis-update payload/field semantics, backup implementation/publication/isolation/fault harness (the frozen qualification-only B1/B2 asset/restore contract is protocol-defined, not implemented or measured), executable overload and recovery harness details, the per-batch WAL-growth bound, and ADR-005-dependent semantics. The 20% per-class illustration remains unratified and is not a representative mix.

| Proposal | Source | Status |
|---------|--------|--------|
| Deterministic 500k-track fixture, deterministic seed (proposed default workload) | m1-sqlite-admission-protocol.md:27 | Base size/seed approved 2026-10-15; assigned payload-budget ceilings `B`, cell counts and fan-out slots frozen as coverage-v1 logical profile (not DB/asset sizes); realized eligible byte-token count `A` and meaningful-coverage criterion remain open via ADR-005 |
| Measured mixed load, 2 producers (100 scan inserts/s + 100 analysis updates/s), 4 readers (10 req/s total including manual edits) | m1-sqlite-admission-protocol.md:30 | Base rates and manual-edit inclusion topology approved; 5-minute warm-up, three separate runs, finite-campaign method approved; seven equal classes at 10/7 req/s each, 7,000 measured seconds/run frozen; analysis-update payload/field semantics remain open |
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
| Analysis payload contents and sizes | Not specified anywhere; analysis-update field semantics remain open |
| Reader lifetime policy | Not defined |
| Free-disk floor | Not defined |
| Schema shape | ADR-005 unresolved |
| Import/export round-trip test details | ADR-005 carries field-relationship risk |

Frozen (no longer assumptions): assigned payload-budget ceilings `B`, cell counts and fan-out slots (coverage-v1 logical profile); realized eligible byte-token counts `A` and meaningful-coverage acceptance criteria remain OPEN, not frozen payload-size distributions. Also frozen: query mix percentages (seven equal classes at 10/7 req/s), manual edit rate (included in 10/s aggregate), arrival schedule (scheduled open-loop), cold-cache procedure (disposable pre-provisioned VM).

---

## 2. Proposal-time gap detail (current status is summarized above)

### 2.1 Query/arrival/sample accounting

**Current state:** The protocol defines seven equal scheduled classes (track lookup; selective search; broad search; sorted first 100-row page; sorted next 100-row page; playlist page; manual edit) at exactly 10/7 requests/s each, totaling 10 req/s. Manual edits are included in—not additional to—the aggregate 10/s. Arrivals are scheduled open-loop. The percentile estimator is exact nearest-rank p95/p99 per class/run. Minimum sample counts are frozen at 2,000 p95 and 10,000 p99 per class/run. Cold-cache procedure is a disposable pre-provisioned VM or dedicated runner. These are frozen qualification inputs, not proposals.

**Frozen definitions (supersede proposal-time text):**

- **Arrival model:** owner-approved scheduled open-loop. Generate offers independently of completion from a predeclared deterministic schedule; report actual inter-arrival jitter and all terminal outcomes.
- **Percentile estimator:** retain raw monotonic durations; calculate exact nearest-rank p95 and p99 per class and run. Do not use approximate histogram buckets for pass/fail. p99.9 is not an acceptance percentile.
- **Minimum samples:** owner-approved frozen descriptive rank-resolution floors are 2,000 observations for p95 and 10,000 for p99 per class/run. These are not statistical confidence guarantees. Under-floor cells are NOT DEMONSTRATED and cannot pass; keep runs separate, with no pooling or post-hoc regrouping.
- **Cold-cache procedure:** owner-approved disposable pre-provisioned VM or dedicated runner. Validate that environment before qualification. Do not use host-wide `drop_caches` on the normal workstation.
- **Query-class mix:** frozen seven equal scheduled classes at 10/7 req/s each. These are qualification-only, not typical-usage estimates. The lead owns the exact class proportions, selectivities, and manual-edit share as reversible test inputs.

**Verdict:** These owner-level method choices are resolved. The synthetic schedule, exact per-class rates, assigned payload-budget ceilings `B`, cell counts, fan-out slots and growth inputs are frozen (coverage-v1); realized eligible byte-token counts `A` and a measurable meaningful-coverage acceptance criterion remain OPEN. The protocol remains NOT READY until the executable accounting harness is implemented and independently reviewed; no further owner approval is needed.

### 2.2 Latency estimators and minimum samples

**At assessment time:** p95/p99 were mentioned without an estimator or minimum sample count; the owner-approved method and floors are recorded above.

**Current approach:** raw duration samples, exact nearest-rank p95/p99 per class and run; owner-approved frozen descriptive floors are 2,000 samples for p95 and 10,000 for p99. These floors are not confidence guarantees. Run separately; no pooling. The common measured interval is frozen at 7,000 seconds, derived from the frozen class rates (10/7 req/s per class) so all in-scope classes meet the p99 floor.

**Verdict:** Sampling method, descriptive floors, exact class rates, and duration (7,000 measured seconds) are frozen.

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

**Current state:** Owner-approved base is five minutes (300s) warm-up outside the measured window, with three independent runs. The frozen seven-class equal-share schedule at 10/7 req/s per class requires 10,000 observations per class, yielding 7,000 measured seconds (116m40s) per run. No pooling.

**Assessment:** 7,000 measured seconds is the frozen duration derived from the class rates and p99 floor, not a promise about representative behavior. This is a finite-campaign qualification, not a population-confidence study. Long-duration effects outside the frozen test envelope are not claimed.

**Current method:** five-minute warm-up, then 7,000 measured seconds per run; three independently reset runs. The warm-up is excluded from percentile calculations but its offered work and fixture growth are recorded.

**Verdict:** Five-minute warm-up outside 7,000 measured seconds is frozen; the duration is derived from the frozen class rates and p99 floor.

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

**Protocol-defined qualification fixture (not implemented):** the accepted protocol §1 and §7 freeze exactly three synthetic integrity sentinels: `stub-a`, `stub-b`, and `stub-c`, each with the exact UTF-8 payload `oxidj-m1-asset-stub-v1:<asset_id>\n`. Stable boundary B1 contains `stub-a`/`stub-b`; after the committed reference transition, B2 contains `stub-b`/`stub-c`. Each self-contained boundary bundle includes its own SQLite online backup, canonical `asset-manifest.json`, and physical copies of exactly its listed assets. The canonical manifest fields, entry fields/order, serialization, and payload hashes are specified in protocol §1; independent evidence records boundary/commit marker and backup, manifest, and logical-snapshot digests.

After B2 capture, the protocol requires later committed DB edits/deletions and live-stub alterations/removal while retained bundles remain unchanged. Restore each bundle separately in a fresh process/destination with original DB/WAL/SHM, live assets, and the other bundle unavailable; prohibit regeneration/download/fallback. Verify independent digests, exact asset-set equality, payload lengths/hashes, expected references, integrity/FK checks, and every persisted row/value against the boundary snapshot and ADR-005 oracle when defined. Compare logical DB content rather than backup-file byte identity. Disposable negative controls must fail for a missing asset, changed asset byte, and manifest tampering. These controls do not replace P0 backup-publication fault cells or applicable three-seed coverage.

Restore timing starts at request before validation/copy/open and ends when the predeclared first interactive query returns the correct boundary-specific result; each restore must meet the existing 120s target. Record full verification timing separately; bundles are locally available, so download time is excluded. This qualification-only fixture does not decide product media/audio/artwork copy/reference or retention policy.

**Verdict:** The synthetic fixture and two-boundary restore checks are protocol-defined, not implemented or measured. Open implementation work includes ADR-005 row IDs/relationship representation and executable semantic oracle, backup lifecycle/publication/isolation and restore harness, P0 fault cells, measured asset/DB bytes and timings, storage budget, and runner preflight/qualification (runner remains NOT PASSED). No experiment or implementation is implied; M1 remains NOT READY and the no-experiment gate remains in force.

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
| Initial fixture and base rates | Frozen: 500k deterministic tracks; 100 scan inserts/s; 100 analysis updates/s; 10 interactive requests/s total; coverage-v1 assigned payload-budget ceilings `B`, cell counts and fan-out slots | Realized eligible byte-token count `A` and meaningful-coverage criterion remain open via ADR-005; no typical-usage claim |
| Read/manual-edit class mix | Frozen: seven equal scheduled classes at 10/7 req/s each; manual edits included in aggregate 10/s | Implement event schedule and retain all terminal outcomes |
| Arrival and percentile method | Frozen: scheduled open-loop; raw durations; nearest-rank p95/p99 per class/run | Implement event schedule and retain all terminal outcomes |
| Sample floors and duration | Frozen: 2,000 p95 / 10,000 p99 per class/run; 7,000 measured seconds; three separate runs; under-floor NOT DEMONSTRATED; no pooling | Apply consistently |
| Warm-up | Frozen: 300s (five minutes) outside 7,000 measured seconds | Apply consistently |
| Cold-cache method | Frozen: disposable pre-provisioned VM/runner | Provision and validate runner; no cache clearing on normal workstation |
| Latency/resource targets | Owner-approved first-prototype thresholds; unmeasured, not product guarantees | Implement measurements and report each run separately |
| WAL/checkpoint/free-space envelope | Owner-approved: 128 MiB high-water, 256 MiB live ceiling, <=64 MiB after verified checkpoint/recovery, <=5 s snapshots, >=1 GiB free after reservation, resume at >=2 GiB plus headroom | Derive and validate per-batch growth bound `T`; implement pause/backpressure/checkpoint verification |
| Overload and accounting | Target and terminal-outcome accounting approved | Specify deterministic reader/rejection behavior and recovery timer |
| Recovery matrix | P0 coverage-first matrix, witness/oracle, two backup restores and three seeds approved; P1 separate | Implement injectors, durable witness, restore oracle, and case manifest |
| ADR-005 direction | Candidate approved for drafting only; no final schema accepted | Draft schema and pass relationship-aware import-edit-export fixtures; resolve C1 allocation and meaningful-coverage criterion and C2 endpoints/sharing; realize coverage-v1 cells legitimately |
| Reference device / SQLite build | `viper` remains a candidate; exact build unselected | Lead selects/revalidates runner and checks bundled SQLite WAL-reset fix before qualification |
| Total storage budget | No product-wide cap approved | Measure the frozen synthetic fixture/assets/backups; do not infer a product cap |
| Durability and engine choice | READY: WAL/FULL conditional contract; SQLite + WAL | Dependency admission still waits for every qualification gate |

---

## 5. Recommended path to READY

1. **Finalize remaining synthetic inputs:** transaction/batch shapes and backup implementation/publication/isolation remain open (the frozen qualification-only B1/B2 asset/restore contract is protocol-defined, not implemented or measured). The coverage-v1 logical profile, scan-growth behavior, read-class schedule, and duration are frozen. Label the workload synthetic; make no typical-usage claim.
2. **ADR-005:** draft the approved candidate direction and relationship-aware import-edit-export fixtures; realize coverage-v1 cells legitimately; run the required independent decision review before accepting a schema.
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
