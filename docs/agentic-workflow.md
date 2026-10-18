# Agentic-Rust workflow for OxiDJ

Status: implemented (M0-D6, `sova`, `nous:poolside/laguna-s-2.1:free`).
Licence: MIT (ADR-009).

This document codifies OxiDJ's agentic-Rust workflow best practices. It is the
single source of truth for what a worker must do — and must show — on every Kanban
card. It was produced by turning the M0-D4 research
(`docs/research/agentic-rust-kanban.md`) into repository rules and tooling.

A new agent should read this file and AGENTS.md first, then read the linked
local documents (`docs/handoff-checklist.md`, `docs/layers-policy.md`,
`docs/rust-analyzer-cli.md`, `docs/research/rust-quality-gates.md`) for the
exact commands. Every rule here maps to a numbered item in the research's
**Implement these** section (items 1–16), so the implementation is auditable
against the research.

## How this file is organised

1. [Quick start for a new worker](#1-quick-start-for-a-new-worker)
2. [Card decomposition and acceptance criteria](#2-card-decomposition-and-acceptance-criteria)
3. [Worktree isolation and shared-file ownership](#3-worktree-isolation-and-shared-file-ownership)
4. [Verification evidence standards](#4-verification-evidence-standards)
5. [Handoff discipline](#5-handoff-discipline)
6. [Context and compaction](#6-context-and-compaction)
7. [Common failure modes and preventions](#7-common-failure-modes-and-preventions)
8. [What this task added](#8-what-this-task-added)
9. [Changes deferred to owning tasks](#9-changes-deferred-to-owning-tasks)

A mapping table at the end links each rule to its research item.

---

## 1. Quick start for a new worker

Before the first edit, confirm where you are and what you are allowed to touch.

```sh
pwd
git rev-parse --show-toplevel
git branch --show-current
git status --short
git -C "$(git rev-parse --show-toplevel)" status --short   # canonical checkout
```

- Work only inside `$HERMES_KANBAN_WORKSPACE`. Never write outside it (see
  [failure mode: drift to canonical root](#7-common-failure-modes-and-preventions)).
- One card per worktree/branch. Keep your scope to the files the card names.
- After the last edit, re-run the gate. Gate evidence is void after the last
  edit (Proof-or-Stop freshness; see [Verification evidence](#4-verification-evidence-standards)).

Then follow this order, cheapest-first, stop at the first failure:

```sh
cargo xtask layers        # boundary gate (see docs/layers-policy.md)
cargo xtask ci            # Tier 1: layers + fmt + clippy(-D warnings) + check
cargo xtask test          # Tier 2: tests
cargo xtask ci-full       # Tier 2 + licence gate (cargo deny check licenses)
```

`rust-toolchain.toml` pins `channel = "1.99.0"`; rustup installs the exact
toolchain on the first `cargo` invocation. The persistent
`.cargo/config.toml` sets `[build] warnings = "deny"` so the lint gate is
self-contained on a fresh clone.

---

## 2. Card decomposition and acceptance criteria

### 2.1 Card granularity — finish and verify in one sitting

Each card should be small enough to implement, test, and verify in a single
focused worker session. If an agent cannot picture finishing it and checking it
in one sitting, split it (research item 15, [ROGER]).

[KANBAN-DECOMP] offers roughly 15–50 tool-call turns as a planning
heuristic, not an OxiDJ-calibrated pass-rate or runtime threshold. Treat call
count as post-hoc telemetry—not as acceptance criteria or an automatic split
trigger. Split on independent deliverables and verification gates, overlapping
file ownership, risk, and asynchronous waits; calibrate by route and task class
from observed completion, rework, and quality rather than model reputation.

For long cards, define a safe checkpoint and timeout handoff. If the work will
not finish, preserve the artifact and record completed acceptance items, exact
checks/evidence, unresolved items, and the smallest next step. A retry must
inspect and continue from that state, not replay the whole task. After repeated
timeouts, the lead inspects the actual worktree and re-scopes the remainder;
do not raise the timeout merely to mask a scope or handoff problem.

- Do not create 20 tiny tasks; do not cram everything into 1 task. The
  decomposer uses 2–6 tasks for normal work ([KANBAN-DECOMP]).
- A card like "implement the full persistence layer" is too large.
- If two tasks can be done independently, give them no shared parent so the
  dispatcher fans them out at once ([KANBAN-DECOMP], research item 15).

### 2.2 Acceptance criteria an agent can test

Acceptance criteria must be checkable, not vibes. Stringer's test: *could you
write an automated check that returns pass or fail against this sentence?*
If not, it isn't acceptance criteria yet ([ROGER], research item 16).

Every card must carry task-specific acceptance criteria, plus the relevant
standing Definition of Done:

- **Per-task acceptance criteria** — what "done" means for this specific piece.
- **Coding cards** — apply the handoff checklist
  (`docs/handoff-checklist.md`): tests pass, no linter errors, no secrets
  committed, and docs updated if the public interface moved (research item 16).
- **Docs, research, and operations cards** — state the artifact-specific
  validation and handoff evidence in the card; a coding checklist is not a
  substitute for those criteria.

The Agentic Agile-V *Constrain* phase requires concrete constraints: "these
files only, touch nothing else"; "no new dependencies without justification";
"no broad refactor during a bug fix" ([AGILE-V], research item 2.2).

### 2.3 Dependency gating and fan-out

Dependencies are `parents=[...]` on card creation. A child stays `todo` until
every parent is `done`, then auto-promotes to `ready` ([HERMES-KANBAN]).
Independent tasks have no parents so they fan out at once ([KANBAN-DECOMP]).

The current card (`t_20f1ee12`) waited on parent `t_a00ec423` (the M0-D4
research) before it started — this is the mechanism working as designed
(research item 3, "Implement these" §3).

---

## 3. Worktree isolation and shared-file ownership

### 3.1 One worktree per agent; shared files are collision hotspots

Each agent works in a worktree of its own (`git worktree add`), on a branch of
its own, from its first edit, docs included. The main checkout is the owner's:
agents do not edit it ([STEELBRAIN], research item 4).

Completed work is merged on an integration worktree; the owner's branch is
fast-forwarded once the merge's checks pass ([STEELBRAIN]).

### 3.2 Build-directory isolation (CARGO_TARGET_DIR)

**Multiple Cargo instances cannot safely share a build cache.** Concurrent
instances sharing a `target/` will corrupt `.rmeta`/`.rlib` artifacts
([CARGO-16804]). The only safe option is per-worktree isolation via
`CARGO_TARGET_DIR`.

The committed rule is in `docs/worktree-isolation.md`. The short form:

- Each worktree keeps its own `target/` (Cargo's default when run from within
  the worktree puts `target/` in that worktree). This is already isolated.
- Never point `CARGO_TARGET_DIR` at a directory shared with another worktree,
  another toolchain profile, or the canonical checkout
  ([CARGO-16804], [STEELBRAIN], research item 4).
- With the current model (one worktree per agent, `max_in_progress: 2`), the
  per-worktree default is already safe; the rule is documentation, not code
  (research item 5, [INFERENCE]).

### 3.3 Shared-file ownership

The collision hotspots are the shared files multiple cards touch: `Cargo.toml`,
`AGENTS.md`, `docs/research/`, `.cargo/config.toml`, `clippy.toml`.

- The orchestrator must flag these with a `hotspot: <path> — <reason>` comment on
  the card and serialize edits rather than letting two agents overwrite a
  sibling's changes (research item 15, [CARGO-16804]).
- Do not edit files you do not own. If a change is needed in an owned file,
  record it as a needed change (see [Changes deferred](#9-changes-deferred-to-owning-tasks)).
- `docs/` `.obsidian/` views are the owner's; route through the lead.

---

## 4. Verification evidence standards

### 4.1 A task that has not run the gate is not complete

The gate, not the claim, moves a task to done. This is the local instance of the
Proof-or-Stop freshness gate ([PROOF-OR-STOP]). The model's completion claim
alone is not evidence (`docs/agent-model-policy.md`, "Task-level escalation
rule"; research items 8 and 13).

### 4.2 Freshness — re-run after the last edit

Evidence is void after the last edit. A worker that runs the gate, then edits
code (even a comment), then claims the gate evidence is still valid is wrong
(research item 9, [INFERENCE] from [PROOF-OR-STOP]).

Practical rule: **If any code changes after the gate is run, the gate must be
re-run; gate evidence is void after the last edit.**

### 4.3 Machine-readable evidence

The handoff checklist (`docs/handoff-checklist.md`) requires the real output of
each step. The minimum reproducible receipt is: command, exit code, and the
result summary (Proof-or-Stop execution attestation — research item 8.2,
[PROOF-OR-STOP]). For example:

```sh
cargo xtask ci        # mandatory local handoff gate: layers + fmt + clippy + check
cargo xtask ci-full   # required CI gate: native tests + mandatory cargo-deny licence check

# Optional, best-effort CI advisories (continue-on-error):
cargo audit
cargo machete --json
typos --format json
```

Tier policy (stable Rust/Cargo 1.99.0; Tier 1 is stable-only and fast, with no advisory registry scans; a cold Cargo cache may still fetch locked dependencies):

| Tier | Runs at | Toolset | Evidence format |
|------|---------|---------|-----------------|
| 1 | every handoff | `cargo xtask ci` (`fmt`, `clippy`, `cargo check`, layers guard) | per-step `PASS (exit 0)` |
| 2 | CI / pre-merge | Required: `cargo xtask ci-full` (native tests + mandatory `cargo deny check licenses`); optional: `cargo audit`, `cargo machete --json`, `typos` (`continue-on-error`) | required run link/summary; optional results if available |
| 3 | deferred / if scheduled | `cargo +nightly miri`, `udeps`, `semver-checks`, `cargo mutants` | pin + job link when configured; no current in-repo job |

Tier 1 must be stable-only and fast (target < ~2 min warm). Tier 3 remains
deferred: no scheduled workflow is configured in this repo. If introduced, run it
on a pinned nightly off the handoff path.

### 4.4 Bug-reproduction invariant

A bugfix handoff additionally includes: revision/diff, compiler version,
feature/target flags, exact command, smallest lawful fixture, **expected** vs
**actual**, RED output before the fix, GREEN output after, and the broader gate
results. A failing build is not the required behavioral RED. Do not delete,
ignore, or weaken a failing test without an explicit, reviewed behavior change
(research item 8.4, `docs/handoff-checklist.md`).

---

## 5. Handoff discipline

A coding handoff is a structured artifact, not a prose report:

1. **Run the gate** — `cargo xtask ci` (Tier 1) minimum; Tier 2 before requesting
   review. Paste real output, never a paraphrase ([MOKEVIN], research item 13).
2. **Do not fabricate output.** If a step did not run, say so; do not paste
   plausible-looking output you did not produce (`docs/handoff-checklist.md`,
   research item 13).
3. **Do not blanket-allow warnings** to turn a gate green — fix the diagnostic.
4. **Scope the diff.** Confirm the changed files match the card's stated scope;
   no unrelated files ([AGILE-V], research item 15).
5. **Fresh-clone reproducibility.** `git clone <repo> && cargo xtask ci` must
   reproduce the green result — one command, no manual toolchain setup
   (`rust-toolchain.toml` + `.cargo/config.toml` do the setup).
6. **Hand the card off** via `kanban_complete` / `kanban_request_review` with the
   evidence pasted, not just a claim.

After about three unsuccessful attempts on the same diagnostic cluster, stop and
report commands, errors, and attempted fixes for escalation — preserve real
output and the current diff ([development.md](development.md), research
item 1.3).

---

## 6. Context and compaction

Context rot is the loss of an agent's ability to use instructions and evidence
reliably as its working context grows (arXiv:2607.17937). The behavioral finding:
agents that gather context before editing and invest in validation succeed more
often than those that don't (arXiv:2604.02547) — it is trajectory *structure*,
not length, that matters (research item 11, [CONTEXT-ROT], [BEHAV]).

Three-tier memory to resist decay:

- **Hot memory** — constraints, plan, acceptance criteria. Always in the prompt.
  For Kanban workers, the card body *is* the hot memory.
- **Warm memory** — running summaries and a decision log.
- **Cold memory** — full artifacts, retrieved on demand.

Practices for OxiDJ workers:

- Keep a `write_todos` list of remaining steps on long tasks; do not rely on
  conversation history for state (prevents the CodeMem failure loop where an
  agent re-processes the same item indefinitely) ([CODEMEM], research item 14).
- `agent.max_turns` was lowered from 150 to **75** on this profile (config
  round, commit `f6d55de`). Loop guides recommend 15–25 for most bounded
  tasks; a high cap widens the blast radius of a runaway loop, so keep the
  cap bounded and lower it further for pure workers ([M-EH], research item 11).
- When a shared file is a collision hotspot, note it and serialize rather than
  piling on (research item 15).
- Hard caps (`max_turns`, `max_iterations`, `max_concurrent_children`),
  duplicate-action detection, and circuit breakers prevent the rate-limit loop
  failure mode (research item 7.2.4, [M-EH], [A-ER]).

---

## 7. Common failure modes and preventions

Drawing on the research's §4.2 and OxiDJ's incident record.

| # | Failure mode | Evidence | Prevention |
|---|--------------|----------|------------|
| 1 | Drift between worktree and canonical checkout | A probe wrote to the canonical repo root instead of the isolated benchmark directory | Record cwd + Git status before/after in both trees; never write outside `$HERMES_KANBAN_WORKSPACE` (§1) |
| 2 | Silently-ignored env vars | A worker sets `CARGO_BUILD_WARNINGS=deny` that the runner doesn't read, then reports success | Prefer committed `.cargo/config.toml` (`[build] warnings = "deny"`) over env-var-only gates; falsification-test any env var |
| 3 | Self-reported success without reproduced evidence | Worker claims "tests pass" without a run | Paste real exit codes / summary lines; Proof-or-Stop freshness gate (§4) |
| 4 | Provider rate-limit loops | ~300+ identical tool calls / ~4.6 hours, no iteration cap | `max_turns` / `max_iterations` caps; duplicate-action detection; backoff with jitter |
| 5 | Lost scratch artifacts | A scratch file needed later is cleaned up before use | Write durable deliverables to the task workspace; attach real artifacts via `kanban_attach` |
| 6 | Build cache corruption across worktrees | Concurrent Cargo instances sharing `target/` corrupt `.rmeta`/`.rlib` | Per-worktree `CARGO_TARGET_DIR` (§3.2, `.cargo/config.toml` default) and `max_in_progress: 2` |
| 7 | Context rot in long tasks | Requirements lost as context grows | Hot/warm/cold memory; `write_todos` anchor; bounded `max_turns` (§6) |
| 8 | Premature completion under budget pressure | A task is marked done before the gate runs | Evidence-gated completion: "a task that has not run the gate is not complete" (§4.1) |
| 9 | Nondeterministic test failures | Parallel tests sharing cwd / env / global state | Per-test `tempfile::TempDir`; `serial_test` for shared-state tests (§8.1) |

---

## 8. Small concrete repo rules added by this task

### 8.1 Test isolation tooling

The testing strategy (`docs/testing.md`) states the principle ("tests run
concurrently and must not share mutable cwd, environment, caches"); this file
names the tools that make it actionable (research item 6, [INFERENCE],
[SP-LEX]):

- `tempfile` — auto-deleted temp directories (`TempDir` cleans up on drop); the
  standard way to give each test its own isolated filesystem.
- `serial_test` — for the rare tests that must mutate global process state,
  annotate `#[serial]` so they run one at a time and restore state after.

Use these at format boundaries and in the M1+ persistence/parsers/integration
suites (see `docs/testing.md`).

### 8.2 Deferred test-runner adoption

The repository contains `.config/nextest.toml`, but the current CI/full gate runs native `cargo test`; `cargo-nextest` is absent locally and is not installed by CI. The bootstrap audit deferred it because native Cargo tests suffice for the current product suite. Revisit adoption only if the suite demonstrates a measurable need for per-test isolation, retries, timeouts, or runtime gains. The profile file is inert until Nextest is wired into the gate; do not treat it as a current prerequisite.

### 8.3 Existing rules that need no change

The following are already correct on this repo and need no edit (research
items 1, 3, 8, 16):

- Workspace layout: root `Cargo.toml` `[workspace]`, `[workspace.package]`
  (edition 2024, rust-version 1.99.0, license MIT), `[workspace.dependencies]`
  (oxidj-core), `[workspace.lints]` (rust + clippy). `default-members` restricts
  bare `cargo build`/`test` to product crates; `xtask` is invoked via the
  explicit alias (research item 1).
- Toolchain pin: `rust-toolchain.toml` (`channel = "1.99.0"`, profile minimal,
  components rustfmt/clippy/rust-analyzer/rust-src) + `clippy.toml`
  (`msrv = "1.99.0"`) + `deny.toml` (MIT allowlist). Reproducible on a fresh
  clone (research item 1).
- Build perf: no `sccache`/`mold` — correctly skipped; OxiDJ's 2-crate
  workspace has no demonstrated compile-time bottleneck ([DEPOT], research item 3).
- `cargo xtask` task runner and `cargo xtask ci` gate (layers → fmt → clippy →
  check) are already the repo-owned entry point (research item 1).
- The handoff checklist's evidence format and the
  "task that has not run the gate is not complete" principle already implement
  Proof-or-Stop on this repo (research item 8, [PROOF-OR-STOP]).
- Card acceptance criteria and `parents=[...]` dependency gating are already the
  committed Kanban discipline (research item 16, [HERMES-KANBAN]).

---

## 9. Changes deferred to owning tasks

These were recommended by the research but target files owned by other tasks
(or a live profile config), so they are recorded here rather than edited, to
avoid collisions:

- **AGENTS.md** (items 12, 13, 14 — worktree-isolation, verification-discipline,
  and context-anchor rules). AGENTS.md is owned by another task and was not
  edited by this task. Each rule is codified above in this file (§3.1, §4, §6)
  and is ready to be copied into AGENTS.md by its owner.
- **docs/testing.md** (item 6 — `tempfile` / `serial_test` named as the standard
  crates). `docs/testing.md` is owned by M0-08; editing it risks collision. The
  substance is codified in §8.1 above. The owning task may paste §8.1 in.
- **docs/agent-model-policy.md** (item 9 — add an explicit freshness clause to
  the escalation rule: "if any code changes after the gate is run, re-run the
  gate; evidence is void after the last edit"). Codified as a rule in §4.2; the
  one-sentence addition to the policy file is recorded for its owner.
- **Agent `max_turns`** (item 11 — lower from 150). **Already applied:** the lead
  lowered this profile's `agent.max_turns` from 150 to **75** in the config round
  (commit `f6d55de`). The setting is a live Hermes profile config, not a
  versioned repo file, so workers on other profiles must apply the same bound
  themselves.

---

## Research-item mapping (items 1–16 → realisation)

| Item | Confidence | Realisation |
|------|-----------|-------------|
| 1  | FACT | No change. Verified workspace + toolchain pin + lint/deny config. See §8.3. |
| 2  | INFERENCE | Apply `{ workspace = true }` for internal deps when the first cross-crate dep is added (oxidj-core already listed). §2.2 / deferred. |
| 3  | OPINION | No sccache/mold — already skipped. §8.3. |
| 4  | FACT | New `docs/worktree-isolation.md`. §3.2. |
| 5  | INFERENCE | Per-worktree `target/` default is already isolated; rule documented in `docs/worktree-isolation.md`. |
| 6  | INFERENCE | `tempfile` + `serial_test` named here (§8.1); editing `docs/testing.md` deferred to its owner (§9). |
| 7  | OPINION | New `.config/nextest.toml` CI profile (§8.2); nextest install + xtask wiring deferred. |
| 8  | FACT | Handoff checklist already correct. §4.1 / §8.3. |
| 9  | INFERENCE | Freshness rule codified here (§4.2); addition to `docs/agent-model-policy.md` deferred (§9). |
| 10 | OPINION | `scripts/gate.sh` deferred — human-readable paste is sufficient at this scale. No script built (see report). |
| 11 | INFERENCE | `max_turns` lowered 150 → 75 on this profile (commit `f6d55de`); guidance codified here (§6). |
| 12 | FACT | Worktree rule in AGENTS.md requested but AGENTS.md owned by another task → codified here §3.1 (§9). |
| 13 | INFERENCE | Verification-discipline rule in AGENTS.md → codified here §4 (§9). |
| 14 | OPINION | Context-anchor rule in AGENTS.md → codified here §6 (§9). |
| 15 | INFERENCE | Card granularity + shared-file hotspots. §2.1 / §3.3. |
| 16 | FACT | Acceptance criteria + dependency gating already correct. §2.2 / §8.3. |

---

## Primary sources for this document

- M0-D4 research: `docs/research/agentic-rust-kanban.md` (items 1–16, §4.2 failure
  table).
- Tiered quality gates: `docs/research/rust-quality-gates.md` (M0-10).
- Quality gate + commands: `docs/handoff-checklist.md`, `.cargo/config.toml`,
  `rust-toolchain.toml`, `clippy.toml`, `deny.toml`, `Cargo.toml`.
- Worktree model: `docs/layers-policy.md`.
- Toolchain baseline: `docs/development.md` (M0-02).
