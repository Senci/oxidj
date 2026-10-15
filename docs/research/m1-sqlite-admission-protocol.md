# M1 SQLite admission criteria — independent decision packet

DECISION

What predeclared functional and quantitative criteria should qualify ADR-002's representative 500k-track prototype? Approve a conditional protocol, not a benchmark result or final schema. All numbers below are proposed engineering budgets, not observed performance.

WHY NOW

ADR-002 is Accepted, but its admission_gate blocks persistence dependencies until this prototype passes. Defining targets afterwards invites moving the goalposts.

CONSTRAINTS

Single process, supported local filesystem, WAL, bundled rusqlite, rusqlite_migration, coordinated bounded single writer, short BEGIN IMMEDIATE batches, read pool, and ADR-002 PRAGMAs including synchronous=FULL. All budgets remain proposed, not measured. No product edits, dependency admission, implementation or benchmarks in this task. ADR-005 must independently resolve linked-field semantics; preserving opaque bytes alone does not prove safe edit/export.

OPTIONS (initial-review packet; final ruling below supersedes the unselected option)

A. Adopt the following provisional budgets, freeze unresolved workload/device/durability choices before execution, and require every criterion to pass. Initial reviewer recommendation: conditional only.

B. Require zero acknowledged-write loss after power failure, revisiting NORMAL versus FULL before qualification; retain A's other criteria. Selected by owner; follow-up Sol review and owner approval resolved it as WAL/FULL with a conditional supported-stack guarantee.

C. Use generic throughput figures or a tiny schema. Rejected: neither qualifies the actual workload.

FINAL LEAD RULING (2026-10-15)

The owner selected zero acknowledged-write loss after OS/power interruption. The follow-up Sol review recommended WAL/FULL and acknowledgement only after successful durable commit, conditional on a supported stack honoring sync; the owner accepted that condition. ADR-002 is amended accordingly. This acceptance does not make the 500k prototype executable: fixture/workload distributions, measurement accounting, live WAL/disk envelope, semantic oracle and recovery matrix must still be frozen. Proposed numerical budgets remain provisional until pre-run review. Current Linux reference-device candidate details appear below.

1. Data: exactly 500,000 initial tracks, deterministic seed, nonuniform text/search selectivity, Unicode, duplicates, nulls, playlists/memberships, cues/loops, variable grids, per-field provenance and opaque payloads. Record cardinalities, payload p50/p95/max, indexes, query plans and actual DB/WAL/asset bytes. Exercise both ADR-003-permitted representation shapes if ADR-005 is unresolved. Payload distributions/fan-out must be grounded in permitted fixtures and approved before running; absent evidence means NOT READY, never assume 100MB. External audio/artwork remain separate assets with a hashed restore manifest.

2. Environment: reference-device candidate is the current Linux host `viper`: kernel `7.2.5-3-omarchy`, Omarchy 4.0.4, AMD Ryzen 5 5600X (6C/12T), 31 GiB RAM, Btrfs (`compress=zstd:3`) over LUKS on an ADATA SX8200PNP 1.9 TB NVMe; repository resides under `/home` in the `@home` subvolume. Re-record exact mount/device/kernel/build at run start. Use this as the single Linux performance qualification device; host-only results do not establish cross-platform durability. Freeze SQLite build, FULL PRAGMAs, connection/producer counts, cache policy and power mode. Three independently reset runs, including cold-open and warm measurements; every run must pass, report per-operation histograms and counts, no pooled averaging.

3. Mixed load: proposed 30 minutes, two producers offering 100 scan inserts/s and 100 analysis updates/s concurrently, four readers offering 10 interactive requests/s total. Interleave track lookup, selective/broad search, sorted first/next 100-row pages, playlist pages and manual edits. Measure request-to-materialized-result including queue/pool waits; measure writes from attempted submission to commit. Rates, representative analysis payload, query mix and selectivities need pre-run fixture/owner ratification, not inference from generic TPS.

4. Budgets: each read class p95 <=100ms, p99 <=250ms; manual-edit commit p95 <=100ms, p99 <=250ms. All write classes submission-to-BEGIN p95 <=100ms, p99 <=500ms; transaction duration p99 <=50ms, maximum 250ms. Bound queue to 1,024 jobs/64MiB; count blocked/rejected submissions, no lost accepted work. Under target load completed throughput must match offered throughput, final five-minute backlog must not exceed first steady five-minute backlog, and shutdown drain <=5s. Overload at twice offered writes for 60s must backpressure explicitly, retain interactive budgets and recover/drain within 60s. Proposed process RSS <=1GiB; checkpoint/drain WAL <=64MiB afterwards (journal_size_limit is not a live hard cap).

5. Safety: zero integrity_check/foreign_key_check errors or semantic-oracle mismatches. Fresh creation, populated upgrade including table rebuild, repeat-open/no-op migration and injected migration failure must leave exactly old or new valid state, never partial; proposed migration <=60s, normal reopen-to-first-result <=2s, restore-to-first-result <=120s, timed separately from full verification.

6. Recovery: minimum 30 reproducible process kills across transactions/checkpoints/migration; retain all acknowledged commits and atomicity. Independently persist a witness of acknowledged transaction IDs; verify it is not in-memory or unflushed. Restore two separately retained online backups after later edits/deletion, into clean directories without original DB/WAL; exact snapshot oracle/provenance/payload and asset-manifest match. Use the accepted WAL/FULL contract; inject I/O/sync failures around commit/ack boundaries and never report a failed/uncertain commit as saved. Process kills are not power-loss simulations; the zero-loss target is conditional on the named supported stack honoring synchronization. Do not imply that a finite test campaign proves arbitrary hardware behavior.


## Owner decisions received 2026-10-15

- **Durability requirement and contract:** owner accepted zero acknowledged-write loss after OS/power interruption, conditional on the supported SQLite/VFS/OS/filesystem/storage stack honoring sync/recovery semantics. By the independent Sol recommendation, ADR-002 now specifies WAL `synchronous=FULL` on all writers and acknowledgement only after successful durable transaction commit. This is not a universal guarantee against defective storage or arbitrary hardware failure.
- **Performance qualification scope:** one real Linux reference device for the measured gate; use current host `viper` as the candidate reference and capture its exact kernel/mount/storage/runtime state at run start. Cross-platform compatibility is a separate requirement and is not established by this qualification.
- **Base workload and gate sequencing:** owner approved the current proposed 500k-track workload direction and the explicit 30-minute mixed-load rates: two producers offering 100 scan inserts/s and 100 analysis updates/s, plus four readers offering 10 interactive requests/s total. The user also confirmed no benchmark or persistence dependency until the full protocol and semantic oracle are executable and independently reviewed. This does not settle per-class proportions/query selectivities, analysis payload, whether manual edits count in the reader rate, or a statistical confidence/adequacy criterion; the illustrative uniform 20%/class calculation is not a representative mix or confidence guarantee.
- **Proposed numeric budgets:** retain as provisional pre-run engineering budgets. Review questionable targets before execution; they are not measured or binding product guarantees yet.

The owner accepted the conditional storage-stack guarantee on 2026-10-15. The ADR amendment below adopts WAL/FULL and commit-success acknowledgement; that is a supported-stack contract, not a universal guarantee against arbitrary hardware failures. The base 500k/30-minute mixed workload shape is now owner-approved, but detailed fixture and query-class distributions, sample-adequacy accounting, live WAL/disk envelope, correctness oracle, and recovery-test protocol remain NOT READY until frozen. No benchmark or dependency admission is authorized by these choices alone.

## DECISIVE EVIDENCE

ADR-002:11,29–35,119–133 establishes the dependency gate, single-writer coordination, conditional FULL durability and recovery contract. persistence-evaluation.md:26–35 marks third-party TPS/size estimates as generic, not OxiDJ measurements; :219–220 flags checkpoint/migration risks. ADR-003:119–123 requires relationship-aware validation. SQLite's official pragma_synchronous documentation states NORMAL/WAL commits may roll back after power/system failure but survive application crashes; WAL documentation warns checkpoint starvation.

OUTPUT CONTRACT

Return RECOMMENDATION; WHY (3–5 reasons); STRONGEST COUNTERARGUMENT; CONFIDENCE; WHAT WOULD CHANGE THE DECISION. Identify unjustified budgets or missing blockers. Advisory only; lead owns final criteria. No tools, delegation, implementation, backlog or benchmarks.
