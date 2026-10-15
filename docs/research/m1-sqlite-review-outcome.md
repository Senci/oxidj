# M1 SQLite prototype criteria: actual independent review and lead ruling

Task: t_93530bfb
Scope: acceptance-criteria decision only; no implementation or benchmarks.
Companion packet: m1-sqlite-decision-packet.md (680 whitespace-delimited words).

## Review provenance

Immediately before dispatch, `hermes config get delegation.provider`, `.model`, `.reasoning_effort`, and `.max_iterations` returned, respectively, `openai-codex`, `gpt-6.1-sol`, `medium`, and `6` (combined command exit 0). No configuration was changed.

One isolated delegate received only the scoped packet and a no-tools/no-delegation instruction. The actual tool result returned `status=completed`, `model=gpt-6.1-sol`, `api_calls=1`, `exit_reason=completed`, `tool_trace=[]`, duration 25.25s, `cost_status=included`. This is an actual completed recommendation, not an availability claim. Effort provenance is the resolved pre-dispatch delegation setting; the result does not independently echo an effort field.

Source transcript handle: /home/senci/.hermes/profiles/oxidj-2/cache/delegation/live/deleg_ce37c4d2/task-0.log. The complete returned recommendation is preserved below so this deliverable does not depend on cache retention.

## Actual returned Sol recommendation (verbatim)

### RECOMMENDATION
**Approve A only as a conditional protocol—not as an executable qualification gate yet.** Freeze and obtain owner approval for the unresolved inputs below before running. Require every criterion in every run to pass; unresolved prerequisites mean **NOT READY**. Choose B if the owner requires zero acknowledged-write loss after power/system failure; amend ADR-002 before qualification.

### WHY
1. **The workload is appropriately representative, but not yet specified enough.** Fixture distributions, relationship fan-out, analysis payloads, query mix and selectivities remain blockers. Specify whether scan inserts grow the library during measurement and define the resulting qualification envelope. A 500,000-track starting point alone does not define workload size.

2. **The numerical limits are proposals, not evidence-backed requirements.** The latency, throughput, memory, migration, restore and recovery budgets need explicit product/device justification. They can serve as conservative engineering targets, but must not be presented as demonstrated user needs. The reference device and supported-platform admission boundary must be settled first.

3. **The measurement contract has material ambiguities.** Define offered versus accepted versus completed work, allowable target-load rejection, and backlog accounting by write class. Otherwise rejecting work can manufacture apparent stability. Freeze an arrival schedule independent of completion, percentile calculation and minimum sample counts; manual-edit frequency and the overload reader schedule are currently unspecified.

4. **Correctness needs an independent, relationship-aware oracle.** Checking opaque bytes and database integrity cannot establish safe linked-field edits or export. If ADR-005 remains unresolved, testing both representations provides architectural evidence but does not discharge that semantic blocker. Define interrupted-operation outcomes and the acknowledgement boundary so crash tests distinguish lost acknowledged work from permissible uncertainty about unacknowledged commits.

5. **Operational bounds need sharper definitions.** Queue limits must cover blocked producers and other retained payloads, not just enqueued jobs. The post-drain WAL limit does not bound disk growth during reader/checkpoint starvation; add a declared live disk/WAL envelope and reader-lifetime policy. Also specify restore timing boundaries and the coverage matrix for injected failures—30 kills alone establishes neither coverage nor power-loss durability.

### STRONGEST COUNTERARGUMENT
**B is the safer default:** acknowledged manual edits may reasonably imply survival of power loss, and owner acceptance of an unspecified loss bound risks approving a contract users would reject. However, that is a product durability decision requiring an ADR amendment—not a reason to silently test NORMAL against FULL-like guarantees.

### CONFIDENCE
**High** that the protocol needs these pre-run blockers resolved; **moderate** that A is the right durability choice until the owner explicitly accepts the power-loss contract. No confidence claim about achievable performance.

### WHAT WOULD CHANGE THE DECISION
- A requirement that acknowledged edits survive power/system failure would select **B**.
- Approved fixtures, device/support scope, measurement definitions, semantic oracle and durability contract would make **A ready to execute**.
- Evidence that a proposed budget is incompatible with supported hardware or actual interaction needs would justify revising it **before execution**, with recorded rationale—not relaxing it after observing results.

## Lead ruling

Accept the recommendation as advisory and adopt A ONLY as a conditional draft protocol. No executable gate, dependency admission, benchmark authorization, or schema approval follows from this task. ADR-002 remains Accepted with its original admission gate unmet. A task completing this decision packet is not the prototype passing.

Retain the packet's numerical budgets as proposed engineering budgets, not product guarantees or measured feasibility. Their rationale is bounded desktop response, bounded resource use, no silent loss and reproducible recovery—not the research note's generic TPS or 50–100MB estimate. Precise numeric feasibility and user suitability are unestablished. The reviewer correctly identifies that percentages without arrival/sample definitions and a post-drain WAL cap cannot prove stability or bounded live disk usage.

### Unresolved prerequisites — explicitly NOT READY

Owner decisions:

- Durability: acceptable acknowledged-write loss after OS/power interruption, if any, and recovery-point objective. NORMAL does not supply a zero-loss guarantee. If zero loss is required, revisit FULL and amend ADR-002 before measurement. Neither a loss duration nor an allowable transaction count has been chosen.
- Real qualification device and support boundary: approve or replace proposed 4-core/8GiB/local-SSD reference floor; decide whether HDD and additional operating systems gate dependency admission. No real device is designated here.
- Product budgets: approve or revise proposed latency, throughput, memory, migration, reopen and restore limits BEFORE results exist. The packet is a recommendation, not owner consent.

Technical protocol decisions still required before execution:

- Approved fixture sources and concrete distributions, relation fan-out, payload sizes, workload growth and analysis update contents. These are not established by the current research.
- Exact query class mix/selectivity, manual-edit rate, open-loop arrival schedule, percentile estimator, minimum samples per class/run and cold-cache procedure. No final values have been chosen.
- Offered/accepted/completed accounting by class. Lead clarification: target-load silent drops or rejections cannot count as successful throughput; overloaded work may be explicitly backpressured, not silently discarded. Attempts waiting outside the queue and their payload bytes must count in memory/backlog measurements.
- A live DB/WAL/free-disk envelope, maximum reader lifetime, checkpoint-starvation scenario, overload reader schedule and full recovery definition. The proposed 64MiB post-drain WAL target is NOT a live cap; live bounds remain unset.
- Independent operation/snapshot oracle, acknowledgement boundary, migration and crash failure-site matrix, restoration start/end boundaries and permitted OS/power fault-injection method. Thirty process kills are a proposed minimum, not coverage proof.
- Relationship-aware semantic fixtures remain binding on ADR-005. Comparing two storage shapes here cannot accept either schema or certify interoperability. A storage prototype must not claim to discharge unresolved export/edit semantics.

Pass/fail rule once the protocol is frozen: every agreed functional oracle and every agreed numerical limit must pass in every prescribed run, with raw counts/histograms, environment manifest and reproducible commands. Missing prerequisites or missing measurements mean NOT READY/NOT DEMONSTRATED, never PASS. Failure triggers diagnosis; genuine engine-level failures can reopen ADR-002, while Litestream-only failure does not disqualify SQLite.

## Evidence checked by the lead

Repository passages were read directly in this worktree:

- docs/adr/002-persistence.md:11,28–33,93,97–100,115–119: dependency-admission separation, required architecture, generic-figure caveat and explicit prototype/recovery requirements.
- docs/research/persistence-evaluation.md:29–35,184–200,217–233: third-party baseline context and checkpoint/migration risks. These are not local measurements. Its old optionality/portability recommendations do not override ADR-002's final ruling.
- docs/adr/003-metadata-representation.md:119–123 and plan/STATUS.md:69–70: unresolved relationship-aware representation and edit/export qualification.
- idea.md:24–25,34: no silent metadata loss and reproducible interoperability proof.

Primary-source verification, fetched during this task:

- https://sqlite.org/pragma.html#pragma_synchronous: "WAL mode does lose durability. A transaction committed in WAL mode with synchronous=NORMAL might roll back following a power loss or system crash. Transactions are durable across application crashes regardless of the synchronous setting or journal mode." This supports distinguishing process-kill retention from power-loss retention; no physical fault injection was performed.
- https://sqlite.org/wal.html section 6: checkpoint starvation can grow the WAL while readers prevent completion. Therefore journal_size_limit and post-drain size cannot establish a live disk bound.

No third-party performance estimate is promoted to an OxiDJ benchmark. No benchmark, test of SQLite, schema implementation, or product/dependency edit was performed.
