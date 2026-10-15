# M1 SQLite admission criteria — independent decision packet

DECISION

What predeclared functional and quantitative criteria should qualify ADR-002's representative 500k-track prototype? Approve a conditional protocol, not a benchmark result or final schema. All numbers below are proposed engineering budgets, not observed performance.

WHY NOW

ADR-002 is Accepted, but its admission_gate blocks persistence dependencies until this prototype passes. Defining targets afterwards invites moving the goalposts.

CONSTRAINTS

Single process, supported local filesystem, WAL, bundled rusqlite, rusqlite_migration, coordinated bounded single writer, short BEGIN IMMEDIATE batches, read pool, and ADR-002 PRAGMAs including synchronous=NORMAL. No product edits, dependency admission, implementation or benchmarks in this task. ADR-005 must independently resolve linked-field semantics; preserving opaque bytes alone does not prove safe edit/export.

OPTIONS

A. Adopt the following provisional budgets, freeze unresolved workload/device/durability choices before execution, and require every criterion to pass.

B. Require zero acknowledged-write loss after power failure, revisiting NORMAL versus FULL before qualification; retain A's other criteria.

C. Use generic throughput figures or a tiny schema. Reject: neither qualifies the actual workload.

CURRENT LEAD PREFERENCE

A, conditional on explicit owner acceptance of the durability contract; otherwise B requires an ADR amendment. Proposed protocol:

1. Data: exactly 500,000 initial tracks, deterministic seed, nonuniform text/search selectivity, Unicode, duplicates, nulls, playlists/memberships, cues/loops, variable grids, per-field provenance and opaque payloads. Record cardinalities, payload p50/p95/max, indexes, query plans and actual DB/WAL/asset bytes. Exercise both ADR-003-permitted representation shapes if ADR-005 is unresolved. Payload distributions/fan-out must be grounded in permitted fixtures and approved before running; absent evidence means NOT READY, never assume 100MB. External audio/artwork remain separate assets with a hashed restore manifest.

2. Environment: freeze exact CPU, RAM, storage model, OS/filesystem, SQLite/build versions, PRAGMAs, reader/producer counts, cache policy and power mode. Proposed reference floor: 4 CPU cores, 8 GiB RAM, local SSD; owner must confirm a real qualification device and whether HDD/other OS support blocks admission. Three independently reset runs, including cold-open and warm measurements; every run must pass, report per-operation histograms and counts, no pooled averaging.

3. Mixed load: proposed 30 minutes, two producers offering 100 scan inserts/s and 100 analysis updates/s concurrently, four readers offering 10 interactive requests/s total. Interleave track lookup, selective/broad search, sorted first/next 100-row pages, playlist pages and manual edits. Measure request-to-materialized-result including queue/pool waits; measure writes from attempted submission to commit. Rates, representative analysis payload, query mix and selectivities need pre-run fixture/owner ratification, not inference from generic TPS.

4. Budgets: each read class p95 <=100ms, p99 <=250ms; manual-edit commit p95 <=100ms, p99 <=250ms. All write classes submission-to-BEGIN p95 <=100ms, p99 <=500ms; transaction duration p99 <=50ms, maximum 250ms. Bound queue to 1,024 jobs/64MiB; count blocked/rejected submissions, no lost accepted work. Under target load completed throughput must match offered throughput, final five-minute backlog must not exceed first steady five-minute backlog, and shutdown drain <=5s. Overload at twice offered writes for 60s must backpressure explicitly, retain interactive budgets and recover/drain within 60s. Proposed process RSS <=1GiB; checkpoint/drain WAL <=64MiB afterwards (journal_size_limit is not a live hard cap).

5. Safety: zero integrity_check/foreign_key_check errors or semantic-oracle mismatches. Fresh creation, populated upgrade including table rebuild, repeat-open/no-op migration and injected migration failure must leave exactly old or new valid state, never partial; proposed migration <=60s, normal reopen-to-first-result <=2s, restore-to-first-result <=120s, timed separately from full verification.

6. Recovery: minimum 30 reproducible process kills across transactions/checkpoints/migration; retain all acknowledged commits and atomicity. Restore two separately retained online backups after later edits/deletion, into clean directories without original DB/WAL; exact snapshot oracle/provenance/payload and asset-manifest match. OS/power interruption is distinct: NORMAL may lose committed transactions; acceptable loss bound and fault-injection method remain owner choices. No power-loss durability claim from process kills.

DECISIVE EVIDENCE

ADR-002:11,97,115–119 requires this gate, single-writer coordination, realistic concurrency and recovery. persistence-evaluation.md:29–35 generic NVMe TPS/size estimates are not OxiDJ measurements; :219–220 flags checkpoint/migration risks. ADR-003:119–123 requires relationship-aware validation. SQLite's official pragma_synchronous documentation states NORMAL/WAL commits may roll back after power/system failure but survive application crashes; WAL documentation warns checkpoint starvation.

OUTPUT CONTRACT

Return RECOMMENDATION; WHY (3–5 reasons); STRONGEST COUNTERARGUMENT; CONFIDENCE; WHAT WOULD CHANGE THE DECISION. Identify unjustified budgets or missing blockers. Advisory only; lead owns final criteria. No tools, delegation, implementation, backlog or benchmarks.
