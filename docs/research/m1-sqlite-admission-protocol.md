# M1 SQLite admission criteria — independent decision packet

DECISION

What predeclared functional and quantitative criteria should qualify ADR-002's representative 500k-track prototype? The owner has now ratified the protocol recommendations recorded below as first-prototype acceptance targets, not measured performance or binding product guarantees. The workload remains NOT READY until its class mix, payloads, growth, assets, backup scope, and semantic oracle are executable and reviewed.

WHY NOW

ADR-002 is Accepted, but its admission_gate blocks persistence dependencies until this prototype passes. Defining targets afterwards invites moving the goalposts.

CONSTRAINTS

Single process, supported local filesystem, WAL, bundled rusqlite, rusqlite_migration, coordinated bounded single writer, short BEGIN IMMEDIATE batches, read pool, and ADR-002 PRAGMAs including synchronous=FULL. Owner-approved numeric budgets are first-prototype pass/fail targets, still unmeasured and not product guarantees. No benchmark or dependency admission until the full protocol and semantic oracle are executable and independently reviewed. ADR-005 must resolve linked-field semantics; preserving opaque bytes alone does not prove safe edit/export.

OPTIONS (initial-review packet; final ruling below supersedes the unselected option)

A. Adopt the following provisional budgets, freeze unresolved workload/device/durability choices before execution, and require every criterion to pass. Initial reviewer recommendation: conditional only.

B. Require zero acknowledged-write loss after power failure, revisiting NORMAL versus FULL before qualification; retain A's other criteria. Selected by owner; follow-up Sol review and owner approval resolved it as WAL/FULL with a conditional supported-stack guarantee.

C. Use generic throughput figures or a tiny schema. Rejected: neither qualifies the actual workload.

FINAL LEAD RULING (2026-10-15)

The owner accepted the recommendations in the preceding decision packet. These are qualification criteria, not measured performance, final product SLOs, or dependency admission. ADR-002's WAL/FULL and successful-commit acknowledgement contract remains conditional on the supported SQLite/VFS/OS/filesystem/controller/device stack honoring sync and recovery semantics.

1. **Fixture:** exactly 500,000 initial tracks with a deterministic seed and the planned Unicode, duplicate, null, playlist/membership, cue/loop, variable-grid, provenance, and opaque-payload cases. Record cardinalities, payload p50/p95/max, indexes, query plans, and actual DB/WAL/asset bytes. Payload/fan-out distributions and transaction maxima remain lead-owned technical inputs; do not invent a 100 MB payload or extrapolate fixture growth. Use deterministic test-only asset stubs and a hashed manifest; this does not settle product audio/artwork copy/reference policy or retention.

2. **Environment and cold cache:** `viper` is the lead-selected Linux reference candidate (kernel `7.2.5-3-omarchy`, Omarchy 4.0.4, Ryzen 5 5600X, 31 GiB RAM, Btrfs `compress=zstd:3` over LUKS/NVMe); revalidate exact device, mount, kernel, SQLite build, and runtime at test start. Use a disposable pre-provisioned VM or dedicated runner for cold-cache qualification; do not clear caches on the normal workstation. Run three independently reset campaigns with cold-open recorded separately; every run/cell must pass separately, without pooling. One-device results do not establish cross-platform durability.

3. **Mixed load and sampling:** owner-approved base: two concurrent producers offer 100 scan inserts/s and 100 analysis updates/s; four reader workers offer 10 interactive requests/s total, including manual edits. Offers use a pre-generated scheduled open-loop clock independent of completion. Run five minutes of fixed warm-up, then at least 30 measured minutes per independently reset run. Use raw durations and exact nearest-rank p95/p99 per class/run. The approved descriptive rank-resolution floors are 2,000 observations for p95 and 10,000 for p99 per class/run; they are not confidence guarantees. Every run is assessed separately; no pooling or post-hoc regrouping. Under-floor cells are NOT DEMONSTRATED, not PASS. After class rates `r_i` are fixed, extend the common measured window enough for each in-scope positive-rate p99 class to reach its floor (`T >= max_i ceil(10,000/r_i)` seconds). Exact class taxonomy/proportions, selectivities, the manual-edit share within the approved total, analysis payload, and unique-insert versus upsert/revisit semantics are lead-owned technical inputs; if representative data is absent, use a balanced synthetic coverage profile and make no typical-usage claim.

4. **Owner-approved first-prototype targets (unmeasured; not product guarantees):** reads p95<=100ms and p99<=250ms; manual-edit commit p95<=100ms and p99<=250ms; writes submission-to-BEGIN p95<=100ms and p99<=500ms; transaction duration p99<=50ms, maximum 250ms; queue<=1,024 jobs/64MiB; process RSS<=1GiB; shutdown drain<=5s. At target load, completed throughput must match offered and the final five-minute backlog must not exceed the first steady five-minute backlog. Overload is 2x offered writes for 60s, explicit backpressure, retained interactive budgets, and recovery/drain<=60s. Migration<=60s; normal reopen-to-first-result<=2s; restore-to-first-result<=120s. During overload, keep readers at the normal 10 interactive requests/s; queue-full write submissions receive a reported `Busy` rejection and are not retried during the 60s overload window, so offered arrivals remain fixed. Target-load rejection or failure is a run failure. At overload end, resume the base offered rates and time recovery until all previously accepted work is complete and backlog returns to the pre-overload baseline; the limit is 60s.

5. **Owner-approved WAL/disk envelope:** 128 MiB high-water response; 256 MiB live-WAL ceiling; <=64 MiB residual after verified checkpoint/recovery; managed read snapshot<=5s; explicit `wal_autocheckpoint=1000` pages. At high-water, stop starting writes and backpressure; establish a real reader gap, perform bounded TRUNCATE, and verify. Enforce a hard ceiling only with a conservative per-batch WAL-growth bound `T` and admission where `W+T<=256 MiB`; if `T` is unknown, split/reject rather than claim a hard cap. Maintain >=1 GiB free after reserving the next operation's conservative allocation; resume only at >=2 GiB plus headroom. A product-wide total-storage budget is deferred; it is not a prerequisite for the synthetic qualification run.

6. **Safety and migrations:** zero integrity_check/foreign_key_check errors or semantic-oracle mismatches. Fresh creation, populated upgrade/table rebuild, repeat-open/no-op, and injected migration failure must leave exactly old or new valid state, never partial. Time migration to <=60s, reopen-to-first-result to <=2s, and restore-to-first-result to <=120s separately from full verification. The exact schema and relationship-aware oracle remain blocked on ADR-005.

7. **Owner-approved recovery policy:** replace the arbitrary 30-kill criterion with the coverage-first P0 matrix: controls; transaction interruption; commit/ack interruption; commit I/O/sync errors; checkpoint interruption; checkpoint I/O/sync errors; migration interruption; migration failure; two separately retained online backup restores at recorded stable boundaries without the original DB/WAL; interrupted/failed backup publication; and recovery-path interruption/failure. Require an independently durable commit/ack witness and exact operation/history oracle. Every predeclared reachable site/shape/seed cell must pass; unfired/invalid cells are NOT DEMONSTRATED and a true mismatch FAILS. Use three deterministic reset seeds per applicable site/shape (repeatability, not statistical confidence). P1 OS/physical interruption is separate and not required for generic dependency admission; authorize/evidence it for a named deployment/support claim. Process kills do not prove power-loss behavior. Detailed injector, witness, backup/asset-manifest, supported-stack, and schema-dependent oracle implementation remain technical freeze prerequisites.


## Owner decisions received 2026-10-15

- **Durability:** zero acknowledged-write loss after OS/power interruption is conditional on the supported SQLite/VFS/OS/filesystem/storage stack honoring sync/recovery semantics. ADR-002 specifies WAL `synchronous=FULL` for writers and acknowledgement only after successful durable transaction commit; this is not a universal guarantee.
- **Base workload/gate order:** 500,000 deterministic initial tracks; 100 scan inserts/s and 100 analysis updates/s; four reader workers offering 10 interactive requests/s total. No benchmark or persistence dependency until the full protocol and semantic oracle are executable and independently reviewed.
- **Sampling:** finite-campaign interpretation, not a population-confidence claim; scheduled open-loop offers; five-minute warm-up outside 30 measured minutes; three independent runs; exact nearest-rank p95/p99 per class/run; 2,000 p95 and 10,000 p99 observations per class/run as descriptive rank-resolution floors; under-floor cells NOT DEMONSTRATED and no pooling. Extend the measured window after fixing the class mix if needed to qualify all p99 classes.
- **WAL/disk and resource limits:** accepted as first-prototype pass/fail targets, not product guarantees; exact thresholds and enforcement are specified above. The hard live-WAL ceiling depends on defining and validating `T`; no combined total-storage limit is approved before fixture/assets/backup scope is fixed.
- **Recovery:** accepted P0 coverage-first matrix, independent commit/ack witness, exact operation/history oracle, two stable-boundary online backup restores, three deterministic seeds per applicable site/shape, with P1 OS/physical interruption separate from generic dependency admission and reserved for named support claims.
- **Cold cache:** use a disposable pre-provisioned VM or dedicated runner; no host-wide cache clearing on the normal workstation.
- **ADR-005:** source-native metadata records + neutral projection + explicit edit overlays are approved as the candidate direction for drafting and fixture validation only, not as a final schema.
- **Lead-owned technical work:** freeze named read classes, synthetic proportions/selectivities and manual-edit share; deterministic payload, fan-out, transaction/batch and scan-growth inputs; test-only asset/backup scope; WAL bound `T`; executable overload/recovery/accounting harnesses; ADR-005 relationship fixtures; revalidated reference device and supported SQLite build. No further owner decision or approval is required for this work; preserve the gate until it is executable and independently reviewed.

The full gate remains NOT READY. The estimates in this packet are targets to be measured only after protocol freeze, not results or product guarantees.

## DECISIVE EVIDENCE

ADR-002:11,29–35,119–133 establishes the dependency gate, single-writer coordination, conditional FULL durability and recovery contract. persistence-evaluation.md:26–35 marks third-party TPS/size estimates as generic, not OxiDJ measurements; :219–220 flags checkpoint/migration risks. ADR-003:119–123 requires relationship-aware validation. SQLite's official pragma_synchronous documentation states NORMAL/WAL commits may roll back after power/system failure but survive application crashes; WAL documentation warns checkpoint starvation.

OUTPUT CONTRACT

Return RECOMMENDATION; WHY (3–5 reasons); STRONGEST COUNTERARGUMENT; CONFIDENCE; WHAT WOULD CHANGE THE DECISION. Identify unjustified budgets or missing blockers. Advisory only; lead owns final criteria. No tools, delegation, implementation, backlog or benchmarks.
