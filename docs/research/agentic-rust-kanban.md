# Rust best practices for agentic coding in Kanban tasks

Status: research complete (shallow, one bounded pass). Owner: `sova` (`nous:poolside/laguna-s-2.1:free`). Scope: OxiDJ — a Rust project built largely by agent workers dispatched from a Kanban board into isolated Git worktrees, integrated by a lead. Research only: no code was written and no repo file was modified except this document.

This document answers the four research questions from task `t_a00ec423`, cites every claim, and ends with a concrete **Implement these** section. Each claim is labelled:

- **[FACT]** — stated in a cited primary source (official docs, Cargo/rustc book, the tool's own README, an arxiv paper, or a real-world AGENTS.md).
- **[INFERENCE]** — my reasoning from cited facts (not directly stated by a source).
- **[OPINION]** — judgement call / best-judgement recommendation, not proven.

OxiDJ already implements many of these practices (the workspace, `cargo xtask`, the handoff checklist, the layer gate, the Tier 1/2/3 policy). This document records what the sources say about each topic, flags where OxiDJ already complies, and identifies the gaps. Existing OxiDJ documents are cited as `[LOCAL]` alongside external primary sources.

---

## 1. Rust-specific agentic practices

### 1.1 Cargo workspace layout

The Cargo Book defines a workspace as a collection of packages managed together: one `Cargo.lock` at the root, a shared `target/` directory, and shared metadata via `[workspace.package]` and `[workspace.dependencies]` [FACT] [C-BOOK]. The Microsoft Rust Guidelines add two rules relevant to agentic work: (1) every crate produced by the project should be listed as a workspace member with its version in `[workspace.dependencies]` so intra-workspace deps resolve to a single canonical version; (2) all crates should live in a flat subdirectory (e.g. `crates/`) rather than nested inside other crates [FACT] [RUST-GUIDE]. The standard-rust-project-layout guide adds that a new crate's manifest should be "almost entirely inheritance" — anything it states for itself is a claim that it differs from the rest, and that claim should be visible in review [FACT] [MARTEL].

**OxiDJ status:** already compliant — the root `Cargo.toml` declares `[workspace]`, `[workspace.package]` (edition, version, rust-version, license), `[workspace.dependencies]`, and `[workspace.lints]`; members are `crates/oxidj-core`, `crates/oxidj-cli`, `xtask` [FACT] [LOCAL]. The `default-members` field restricts bare `cargo build`/`test` to the product crates, with `xtask` invoked explicitly via its alias [FACT] [LOCAL]. The layer gate (`cargo xtask layers`) enforces that new crates are classified, keeping the workspace meaningful as crates are added [FACT] [LOCAL].

### 1.2 `cargo xtask` — the repo-owned task runner

The cargo-xtask pattern (by matklad, the same author as rust-analyzer) is a "way to add free-form automation to a Rust project, a-la `make`, `npm run` or bespoke bash scripts." Its two distinguishing features: it requires no binaries besides `cargo` and `rustc` (it fully bootstraps from them), and unlike bash it is cross-platform because it doesn't use the shell [FACT] [XTASK]. The pattern is not an officially recommended workflow but is "a somewhat common pattern across the ecosystem" — notably, Cargo itself uses xtasks [FACT] [XTASK].

The mechanism has three pieces, all committed to version control: (1) a `.cargo/config.toml` alias `xtask = "run --package xtask --"`; (2) an `xtask/` package containing a Rust binary that dispatches subcommands via `argv[1]`; (3) a workspace `Cargo.toml` listing `xtask` as a member [FACT] [XTASK]. The workspace-member form is preferred because it shares the dependency cache with the rest of the project and works from any subdirectory; the standalone form (`--manifest-path`) only works from the project root [FACT] [XTASK]. Best practices from the README: use the `$CARGO` environment variable (not a hardcoded `"cargo"`) when spawning sub-invocations, commit the `xtask` lockfile, minimize xtask compile time, and define tasks as subcommands of the single `xtask` binary [FACT] [XTASK].

**Why this matters for agents:** a single repo-owned entry point (`cargo xtask ci`) means a worker never has to guess which commands to run, in which order, with which flags. The gate is self-documenting: one command on a clean clone reproduces the result. This directly serves the "agent-friendly diagnostics" and "verification evidence" goals (§3).

**OxiDJ status:** already implemented — `.cargo/config.toml` defines the `xtask` alias; `xtask/src/main.rs` implements `ci`, `ci-full`, `layers`, `fmt`, `clippy`, `check`, `test`, `deny`, and `help` subcommands, each invoking the correct cargo subcommand with the right flags and streaming exit codes [FACT] [LOCAL]. The `ci` subcommand runs `layers → fmt → clippy → check` in order with `stop_on_fail`, and the persistent `[build] warnings = "deny"` in `.cargo/config.toml` makes the lint gate self-contained on a fresh clone [FACT] [LOCAL].

### 1.3 Compiler-driven iteration

Rust's fast feedback loop is built on layered commands of increasing cost: `cargo check` (type-check only, no codegen) is the inner loop; `cargo clippy` adds lints; `cargo build` / `cargo run` does full codegen; `cargo test` builds and runs tests [FACT] [RUST-TRAIN] [SME]. The rustc dev guide's suggested workflows describe the same layering and note that `rust-analyzer` runs `cargo check` and `rustfmt` on save by default, with override support to run a subset (e.g. `x check compiler --json-output`) [FACT] [RUSTC-DEV]. Incremental compilation (stable via Cargo's default behavior, with `-Zincremental` for finer control) avoids redoing work when recompiling a crate, "ultimately lead[ing] to a much faster edit-compile-debug cycle" [FACT] [RUST-INC].

`cargo watch -x check` re-runs a command on every source-file change, and `cargo watch -x 'clippy --workspace --all-targets' -x 'test --workspace --lib'` runs clippy + tests on change [FACT] [RUST-TRAIN]. `kache` is a content-addressed `RUSTC_WRAPPER` that caches compiled dependencies globally across projects, surviving `cargo clean`, branch switches, and fresh worktrees; its `adaptive_incremental` mode (since 0.27.0) keeps the last-edited crate on the incremental path while dependencies stay cached [FACT] [TENEQ].

**OxiDJ status:** the handoff checklist's "Fast focused loops" section already gives the narrow-gate-then-widen pattern (`cargo fmt --check` → `cargo clippy -p oxidj-core` → `cargo test -p oxidj-core track_id -- --exact` → `cargo xtask ci`) [FACT] [LOCAL]. `cargo watch` and `kache` are absent — correctly deferred in M0-02 because there was no demonstrated compile-time bottleneck [FACT] [LOCAL].

### 1.4 rustfmt, Clippy, and cargo-nextest

- **rustfmt** (`cargo fmt --all -- --check`) is the only Rust formatter. On stable, the machine-readable signal is the exit code (0 = clean, 1 = would reformat); `--message-format json` is nightly-only [FACT] [RUSTFMT] [CARGOFMT-SRC] (cited in `docs/research/rust-quality-gates.md`).
- **Clippy** (`cargo clippy`) provides 800+ lint groups beyond rustc. `CARGO_BUILD_WARNINGS=deny` (stable since Cargo 1.97) raises all rustc + clippy warn-level lints to hard errors; the persistent `.cargo/config.toml` form (`[build] warnings = "deny"`) is the committed, reproducible version [FACT] [C-CONFIG] [CL-USAGE] (cited in `docs/research/rust-quality-gates.md`). `--keep-going` ensures the worker sees all lints, not just the first error [FACT] [CL-USAGE].
- **cargo-nextest** is a test runner whose defining choice is that every test runs in its own process, giving true isolation (one test cannot corrupt another's global state), better parallelism, and the ability to report a crashing test rather than losing the whole run [FACT] [NEXTEST] [NEXTEST-OCX]. It provides per-test timeouts (`slow-timeout` + `terminate-after`), retries for flaky tests (a test that passes on retry is marked FLAKY, and `--flaky-result fail` can treat flaky as a failure), and machine-readable output via `--message-format json` and JUnit XML [FACT] [NEXTEST-RET] [NEXTEST-LIST].

**Why nextest matters for agents:** process-per-test isolation means an agent running the full suite is not misbled by cross-test contamination; flaky-test detection distinguishes a nondeterministic test from a real failure; JSON output is machine-parseable for the evidence step in §3.

**OxiDJ status:** `cargo fmt --check` and `cargo clippy` (with `CARGO_BUILD_WARNINGS=deny`) are in the Tier 1 gate via `cargo xtask ci` [FACT] [LOCAL]. `cargo test` (not nextest) is the Tier 2 runner; `cargo nextest` is listed as the recommended runner in `docs/research/rust-quality-gates.md` but not yet installed [FACT] [LOCAL]. A `.config/nextest.toml` with retry/flakiness policy does not exist [FACT] [LOCAL].

### 1.5 Deterministic fixtures and parallel-test safety

Rust's test framework runs tests in parallel by default; environment variables, cwd, singleton registries, and global loggers can make tests flaky [FACT] [SP-LEX] [INDEED]. The `tempfile` crate provides auto-deleted temp directories (`TempDir` cleans up on drop, fitting Rust's ownership model) and is the standard way to give each test its own isolated filesystem [FACT] [CPP-RUST] [INDEED]. The Microsoft Rust Guidelines note that fixture files committed with the tests are reproducible and need no runtime path discovery; for large data sets, load from disk using paths based on `CARGO_MANIFEST_DIR` [FACT] [SP-LEX]. If a test must mutate global process state, isolate it with `-- --test-threads=1` or `serial_test`, and restore state after the test [FACT] [SP-LEX].

The `induscode` project (a real Rust CLI agent) enforces three determinism rules: no network (replay recorded responses via `wiremock`), no real home (`TempWorkspace` backed by `tempfile::TempDir`), no PTY (draw into an in-memory `TestBackend`); each test gets its own removed-on-drop workspace so runs are parallel-safe [FACT] [INDUSAGI].

**OxiDJ status:** `docs/testing.md` states the principle ("tests run concurrently and must not share mutable cwd, environment, caches or a user's real library") and requires real per-test temporary files/databases, but does not name `tempfile` or `serial_test` explicitly, and no committed test uses them yet (the M0 suite is minimal) [FACT] [LOCAL].

### 1.6 Property testing, fuzzing, and coverage

- **Property testing:** the `doctrine` Rust testing guide says projects **SHOULD** state invariants as property tests where a round trip, an ordering, or an algebraic law can be expressed, and names `proptest` as the tool [FACT] [DOCTRINE]. The radkit AGENTS.md adds: "Add property or fuzz tests (`proptest`, `arbitrary`, `cargo fuzz`) for parsing, serialization, and protocol logic" [FACT] [RADKIT].
- **Fuzzing:** projects handling untrusted input **SHOULD** use `cargo-fuzz`, which drives libFuzzer. Platform requirements: x86-64/AArch64, Unix-like only (not Windows), nightly toolchain (the flags are unstable), a C++ compiler [FACT] [DOCTRINE]. In CI, the nightly toolchain must be pinned by date, the corpus persisted between runs, and crash artefacts uploaded [FACT] [DOCTRINE].
- **Coverage:** `cargo-llvm-cov` drives the compiler's own source-based instrumentation (`-C instrument-cov`) and is the recommended default; it does not prove correctness but reliably reveals blind spots [FACT] [DOCTRINE] [MS-RUST]. `cargo-mutants` measures whether the test suite detects changed behaviour — a missed mutant is a concrete, addressable gap; it requires non-flaky tests [FACT] [DOCTRINE] (cited in `docs/research/rust-quality-gates.md`).

**OxiDJ status:** `docs/testing.md` already says to justify fuzzing and property testing once parsers and invariants exist (not as bootstrap dependencies), and that property tests should fuzz hostile/malformed input at format boundaries asserting the error is reported [FACT] [LOCAL]. This matches the doctrine guide's SHOULD. The quality-gates research classifies `cargo-llvm-cov` as a Tier-3 candidate, but no scheduled Tier-3 workflow exists in `.github/workflows`; `cargo-mutants` remains deferred until test-suite stability is demonstrated [FACT] [LOCAL].

### 1.7 Dependency auditing

`cargo-audit` checks `Cargo.lock` against the RustSec advisory DB (fast, offline-friendly with cached DB); `cargo-deny` provides a fuller supply-chain policy (licenses, bans, sources, advisories from RustSec + GitHub Advisory DB) [FACT] [AUDIT] [DENY] (cited in `docs/research/rust-quality-gates.md`). `cargo-machete` detects unused declared dependencies (fast, graph-based, `--json` output) [FACT] [MCH]. `cargo-vet` (Mozilla-backed) checks that crates.io deps are covered by trusted-org audits/imports [FACT] [VET].

**OxiDJ status:** `cargo audit` is in Tier 2, `cargo deny check licenses` is in the CI gate (`cargo xtask ci-full`) per ADR-009's MIT-only policy, and `cargo machete --json` is Tier 2 [FACT] [LOCAL]. `deny.toml` allowlists MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2016, Zlib; all copyleft families are denied [FACT] [LOCAL].

### 1.8 Build caching and linker speed

Long compile times are the #1 Rust developer pain point; `sccache` (compiler cache) + `mold`/`lld` (fast linkers) can cut iteration time by 50–80% [FACT] [RUST-TRAIN]. `mold` is 3–5× faster than `lld` and 10–20× faster than the default GNU `ld` [FACT] [RUST-TRAIN]. The configuration is committed in `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

[FACT] [RUST-TRAIN] [BIGIRON].

**Evidence quality note:** the 50–80% figure comes from a Microsoft training guide (secondary/vendor source). Depot.dev's real-world test on the Zed codebase found `mold` showed "negligible results" for that project, and `sccache` provides meaningful speedups mainly with a warm cache [FACT] [DEPOT]. So the benefit is real but project-dependent — **[INFERENCE]** for OxiDJ's small 2-crate workspace, the compile-time bottleneck is unlikely to justify adding `sccache`/`mold` until a real measurement shows one.

**OxiDJ status:** `.cargo/config.toml` contains only `[build] warnings = "deny"` and the `xtask` alias; no linker or wrapper configuration [FACT] [LOCAL]. M0-02 correctly skipped `sccache` absent a demonstrated bottleneck [FACT] [LOCAL].

### 1.9 Worktrees and isolated build directories

This is the most agent-specific Rust practice and the one most likely to bite. **Multiple Cargo instances cannot safely share a build cache.** The per-target-directory lock serializes builds and race conditions corrupt artifacts (`.rmeta`/`.rlib` files overwritten while another `rustc` is reading them). The only safe workaround is per-worktree isolation via `CARGO_TARGET_DIR` — which eliminates sharing entirely [FACT] [CARGO-16804].

The Cargo issue documents the scale of this in agent-driven development: across ~40 Rust projects over 4,400 sessions, there were **457 disk-exhaustion events**, **3,703 compilation failures from artifact corruption**, and **22,958 instances where agents had to manually isolate `target/` directories** to avoid concurrent access failures. The multiplier: `N worktrees × M toolchains × K profiles × (2–10 GB per instance)` — for 6 worktrees × 2 toolchains × 2 profiles that is 48–240 GB of build artifacts [FACT] [CARGO-16804].

The steelbrain/reims-vgpu project's AGENTS.md is a real-world model for agent-heavy Rust repos: "Each agent works in a worktree of its own (`git worktree add`), from its first edit, docs included, on a branch of its own. The main checkout is the owner's: agents do not edit it." Each worktree gets its own `CARGO_TARGET_DIR` under `~/.cache`; caches never go under `/tmp` [FACT] [STEELBRAIN]. The concurrency is capped (`max_in_progress: 2`, `max_in_progress_per_profile: 1`) so at most two Cargo instances run at once [FACT] [LOCAL].

**OxiDJ status:** the Kanban protocol sets `HERMES_KANBAN_WORKSPACE` and workers `cd` there first; `docs/layers-policy.md` describes the worktree-per-agent model [FACT] [LOCAL]. However, there is no committed guidance on `CARGO_TARGET_DIR` isolation, and the default `target/` inside each worktree means each worktree accumulates its own full copy of dependencies — safe but disk-hungry [FACT] [CARGO-16804] [INFERENCE].

### 1.10 Agent-friendly diagnostics

`cargo clippy --message-format=json` and `cargo check --message-format=json` emit compiler/clippy diagnostics as JSON Lines (`reason=compiler-message` with `spans`/`suggestions`), which is the stable machine-readable path [FACT] [C-EXTERNAL]. rust-analyzer provides LSP diagnostics and code actions (many clippy/rustc diagnostics carry machine-applicable suggestions exposed as code actions), but the rust-quality-gates research explicitly deferred the LSP-direct path because no study confirms it reduces agent iterations; the JSON path is the starting point [FACT] [LOCAL]. `rust-analyzer analysis-stats .` and `rust-analyzer diagnostics .` are verified CLI invocations that give a fast whole-workspace type-check summary and structured line-oriented diagnostics on stable 1.99.0 [FACT] [LOCAL].

**OxiDJ status:** the Tier 1 gate uses `--message-format=json` in the evidence-collection example in `docs/research/rust-quality-gates.md` §3, and `docs/rust-analyzer-cli.md` documents the CLI invocations [FACT] [LOCAL].

---

## 2. Agentic coding in Kanban tasks

### 2.1 Task granularity: agent-sized work

Roger Stringer's "Shipping Software When Agents Write the Code" distils the core rule: "every task should be small enough to implement, test, and verify in a single focused session. If I can't picture the agent finishing it and me checking it in one sitting, it's too big and gets split" [FACT] [ROGER]. Each piece should be specific (the agent knows what "done" looks like), achievable with the tools it actually has, and ordered (later pieces build on earlier ones instead of colliding) [FACT] [ROGER].

The agentic-coding principles guide adds the counterweight: "overly granular decomposition can lead to loss of overall context or consistency. It's crucial to find the appropriate level of decomposition based on task complexity and AI capabilities" [FACT] [AGENTIC-CODING]. The Hermes Kanban decomposer uses "2–6 tasks for normal work. Don't create 20 tiny tasks. Don't cram everything into 1 task" [FACT] [KANBAN-DECOMP].

**[INFERENCE]** For OxiDJ's bounded worker model, the sweet spot is a card that one worker can finish in 15–50 tool-call turns: large enough to be meaningful (a real crate, a real gate), small enough to verify in one sitting. The existing M0-D4-style research cards and the M0-08-style scaffolding cards are about the right size; a card like "implement the full persistence layer" would be too large.

### 2.2 Acceptance criteria an agent can test

Stringer's test: "could you write an automated check that returns pass or fail against this sentence? If not, it isn't acceptance criteria yet, it's a vibe" [FACT] [ROGER]. He distinguishes two layers: per-task acceptance criteria (what "done" means for this specific piece, changing every task) and a standing Definition of Done (tests pass, no linter errors, no secrets committed, docs updated if the public interface moved) [FACT] [ROGER].

The Agentic Agile-V framework's SCOPE-V loop names the Specify and Constrain phases: the task brief carries objective, scope, non-goals, affected modules, dependencies, acceptance criteria, and required evidence; constraints include "no unrelated files, no new dependencies without justification, no broad refactor during a bug fix" [FACT] [AGILE-V]. The agentic-coding-playbook's Module 06 adds: "The more concrete the verification criteria, the better the agent's self-check. Vague criteria like 'make sure it works' give the agent nothing useful to verify against" [FACT] [PLAYBOOK-06].

**OxiDJ status:** Kanban cards already carry acceptance criteria (e.g. this task's are explicit) and the model policy's escalation rule requires "correct worktree, actual compiler/tests where applicable, diff review, and whole-repository status" [FACT] [LOCAL]. The handoff checklist is the standing Definition of Done [FACT] [LOCAL].

### 2.3 Dependency gating and fan-out

The Hermes Kanban model expresses dependencies as `parents=[...]` on `kanban_create`; a child stays `todo` until every parent is `done`, then auto-promotes to `ready` [FACT] [HERMES-KANBAN] [KANBAN-DECOMP]. The decomposer is instructed to prefer parallelism: "If two tasks can be done independently, give them no parents so the dispatcher fans them out at once" [FACT] [KANBAN-DECOMP].

Stringer frames the same idea as a dependency graph: "Some tasks have to happen before others. Some don't depend on each other at all and can run in parallel." The critical path is the chain; independent tasks are parallel branches [FACT] [ROGER]. For tasks touching messy code, add an explicit scope line: "these files only, touch nothing else" [FACT] [ROGER].

**OxiDJ status:** the task graph already uses parent dependencies (this task's parent is `t_74a03928`; its child is `t_20f1ee12`), and the model policy caps concurrency at two independent items with independent worktrees [FACT] [LOCAL].

### 2.4 Worktree isolation and avoiding write conflicts

The steelbrain/reims-vgpu AGENTS.md is the clearest real-world statement: "Each agent works in a worktree of its own (`git worktree add`), from its first edit, docs included, on a branch of its own. The main checkout is the owner's: agents do not edit it." Finished work is merged on an integration worktree, and the owner's branch is fast-forwarded once the merge's checks pass [FACT] [STEELBRAIN].

The OxiDJ model policy records a real incident: "A Nemotron probe created a compilable Rust test crate in the canonical repository root, not the intended isolated benchmark directory" — an isolation failure that disqualified the run despite passing tests [FACT] [LOCAL]. The background-systems guidance says to record the intended absolute cwd and Git status in both the worktree and canonical checkout before, then inspect both after [FACT] [LOCAL-HB].

The Cargo concurrency issue (§1.9) adds the build-directory dimension: even with separate worktrees, two agents sharing a `target/` will corrupt artifacts; `CARGO_TARGET_DIR` per worktree is the only safe option [FACT] [CARGO-16804].

**[INFERENCE]** The collision hotspots in OxiDJ are the shared files that multiple research/scaffolding cards touch: `Cargo.toml`, `AGENTS.md`, `docs/research/`, `.cargo/config.toml`, `clippy.toml`. The Kanban protocol already tells the orchestrator to flag these with a `hotspot:` comment; the steelbrain model of one-branch-per-agent plus per-worktree `CARGO_TARGET_DIR` is the structural prevention.

### 2.5 Verification evidence and handoff discipline

The Proof-or-Stop framework (arXiv:2607.14890) is the most rigorous statement of the principle: "Treat an agent's lifecycle states as unproven claims until fresh, source-bound, mechanically verifiable evidence clears the relevant gate." The agent may emit a claim, but the claim never moves the state; the evidence does [FACT] [PROOF-OR-STOP]. The gate requires the evidence to be fresh (source-state hashes match the current tree — evidence gathered before the last edit is rejected), complete, integrity-verified, execution-attested (command, arguments, working directory, exit code, and output digest recorded), and tied to the specific claim [FACT] [PROOF-OR-STOP].

The paper's evaluation supports the practice: in a 9,240-cell ablation, the gated loop cut "visible-pass/hidden-fail" amplified artifacts to 2 per 1,800 (0.11%) against 31 per 1,800 (1.72%) for a compute-budgeted naive-retry baseline — a 1.6 percentage-point improvement whose confidence interval excludes zero. The unattended-loop engine passed 10 of 10 scenarios with zero false-DONE [FACT] [PROOF-OR-STOP].

The mokevin agentic-coding patterns guide states the same idea more simply: "The evidence has to come from the environment, not from the agent's account… A retelling of the result in the agent's final message does not count as evidence" [FACT] [MOKEVIN].

**OxiDJ status:** `docs/handoff-checklist.md` is the committed implementation: "A task that has not run the gate is not complete, however finished the code looks," with the required evidence format (paste real output, never a paraphrase; do not paste output you did not actually run) [FACT] [LOCAL]. The model policy's escalation rule — "the model's completion claim alone is not evidence" [FACT] [LOCAL] — is the same principle.

---

## 3. Verification & quality gates

### 3.1 "A task that has not run the gate is not complete"

This principle, stated in OxiDJ's handoff checklist, is the local instance of the Proof-or-Stop freshness gate [FACT] [LOCAL] [PROOF-OR-STOP]. The agentic-coding-playbook calls it "the most important quality gate in agentic coding, and the one that gets skipped most often" — the human review step [FACT] [PLAYBOOK-06]. The reproduce-before-report gate (AgentPatterns.ai) applies the same idea to reviewer findings: "A reviewer can hallucinate a defect with no external grounding. A verifier that must produce a reproduction cannot: it either runs or it does not" [FACT] [AP-REPRO].

The layered model from the playbook: (1) during generation — hooks that run on each edit, giving immediate feedback; (2) pre-commit — checks on staged files; (3) pre-merge — analysis comparing the feature branch to main, catching cumulative quality drift [FACT] [PLAYBOOK-06]. The three tiers should run cheapest-first and stop at the first failure [FACT] [STEELBRAIN].

### 3.2 Machine-readable output

The Proof-or-Stop receipt records "command, arguments, working directory, exit code, and output digest" — the minimum set for a reproducible claim [FACT] [PROOF-OR-STOP]. The existing OxiDJ handoff checklist asks for exactly this in human-readable form: the per-step `PASS (exit 0)` lines from `cargo xtask ci`, the final `test result: ok` summary lines, the JSON summary from `cargo machete --json`, the exit code + `0` lines from `typos --format json` [FACT] [LOCAL].

cargo-nextest adds `--message-format json` for machine-parseable test structure and JUnit XML for CI [FACT] [NEXTEST-LIST]. The nextest-metadata library provides a `TestListSummary` struct for parsing in Rust, always in sync with nextest's own output [FACT] [NEXTEST-LIST].

### 3.3 Freshness and reproduction of numeric claims

Proof-or-Stop's "freshness" condition is the key insight: a receipt binds to `materialHash` (a SHA256 of the canonical tracked source tree), `headHash` (commit identity), and `storyFilesHash`. An agent that edits code after the receipt was signed cannot reach `done` on it — the stale `materialHash` fails the freshness check, and a new full run is required [FACT] [PROOF-OR-STOP].

For OxiDJ, the practical implication is **[INFERENCE]**: a worker that runs the gate, then edits code (even a comment), then claims the gate evidence is still valid is wrong. The gate must be re-run after the last edit. The handoff checklist's "paste real output" rule is the human enforcement; a future machine-checkable form could bind the evidence to the commit hash.

### 3.4 Diff/scope checks

The Agentic Agile-V Constrain phase includes "no unrelated files" and "no new dependencies without justification" [FACT] [AGILE-V]. The playbook's human review step: "Check for scope creep. Did the agent only change what you asked for?" [FACT] [PLAYBOOK-06]. The reproduce-before-report gate adds independence: "A verifier that shares context with the reviewer inherits its reasoning errors and confirms its own false positives" — a fresh-context verifier is the property that does the work [FACT] [AP-REPRO].

**OxiDJ status:** the model policy already requires "diff review, and whole-repository status" as part of the escalation rule [FACT] [LOCAL]. The Kanban protocol instructs workers to confirm `pwd` before writing and never write outside `$HERMES_KANBAN_WORKSPACE` [FACT] [LOCAL].

---

## 4. Context & failure modes

### 4.1 Context management across long tasks

**Context rot** (arXiv:2607.17937v2) is defined as "the loss of an agent's ability to use instructions and evidence reliably as its working context grows or accumulates competing state." The study's key finding: requirements are lost while being compiled into the active task, drift during execution, or fail to constrain verification and completion. A 50-point strict-success loss was observed with retention of only 37.5% of clean success while retaining 93.3–94.9% of clean requirement coverage [FACT] [CONTEXT-ROT].

The behavioral drivers study (arXiv:2604.02547v1) found that the widely reported correlation between trajectory length and failure reverses once task difficulty is controlled — instead, trajectory *structure* discriminates: "agents that gather context before editing and invest in validation succeed more often" [FACT] [BEHAV].

The long-context strategies guide proposes a three-tier system: hot memory (constraints, plan, current state — always in the prompt), warm memory (running summaries and decision logs), and cold memory (full artifacts retrieved on demand). Failure modes unique to long context: constraint decay (early instruction ignored after many turns), plan drift (agent starts doing "interesting" work unrelated to goal), evidence conflicts, and context overflow [FACT] [ARUN].

CodeMem (arXiv:2512.15813) documents a concrete failure: without external state, an agent "forgets" which items it has already processed and enters a failure loop, re-fetching the first item indefinitely. The `write_todos` anchor prevents this by tracking complex plans in a structured list [FACT] [CODEMEM].

**OxiDJ status:** the Hermes dual compression layer (threshold 50%, tail_mode lean, protect_last_n 20) plus the escalating compaction failure cooldown handles the automatic side [FACT] [LOCAL]. The Kanban card body itself is the "hot memory" — it carries the acceptance criteria and scope. The model policy's `agent.max_turns: 150` is high for bounded tasks; loop guides recommend 15–25 for most tasks [FACT] [LOCAL] [M-EH].

### 4.2 Common failure modes of agentic Rust work

Drawing on the sources and OxiDJ's own incident record:

| Failure mode | Evidence | Prevention |
|---|---|---|
| **Drift between worktree and canonical** | Nemotron probe wrote to canonical root instead of isolated directory [FACT] [LOCAL] | Record cwd + Git status before/after in both trees; never write outside `$HERMES_KANBAN_WORKSPACE` [FACT] [LOCAL-HB] [STEELBRAIN] |
| **Silently-ignored env vars** | A worker sets `CARGO_BUILD_WARNINGS=deny` that the runner doesn't read, then reports success anyway [FACT] [LOCAL] | Prefer committed `.cargo/config.toml` over env-var-only gates; falsification-test env vars [FACT] [LOCAL] |
| **Self-reported success without reproduced evidence** | Worker claims "tests pass" without a run [FACT] [LOCAL] | Paste real exit codes / summary lines; Proof-or-Stop freshness gate [FACT] [LOCAL] [PROOF-OR-STOP] |
| **Provider rate-limit loops** | Sub-agent retried a failed command verbatim for ~300+ identical tool calls / ~4.6 hours with no iteration cap [FACT] [V-INC] | `max_turns` / `max_iterations` caps; duplicate-action detection; exponential backoff with jitter [FACT] [M-EH] [A-ER] |
| **Lost scratch artifacts** | A scratch file that a worker needs later is cleaned up before use [FACT] [LOCAL-HB] | Write durable deliverables to the task workspace, not `/tmp`; attach real artifacts via `kanban_attach` [FACT] [LOCAL] |
| **Build cache corruption across worktrees** | Concurrent Cargo instances sharing `target/` corrupt `.rmeta`/`.rlib` [FACT] [CARGO-16804] | Per-worktree `CARGO_TARGET_DIR` [FACT] [STEELBRAIN] [CARGO-16804] |
| **Context rot in long tasks** | Requirements lost as context grows; 50-point strict-success loss observed [FACT] [CONTEXT-ROT] | Hot/warm/cold memory; `write_todos` anchor; bounded `max_turns` [FACT] [ARUN] [CODEMEM] |
| **Premature completion under budget pressure** | Agents declare done when the gate hasn't run [FACT] [PROOF-OR-STOP] | Evidence-gated lifecycle; "a task that has not run the gate is not complete" [FACT] [LOCAL] [PROOF-OR-STOP] |
| **Nondeterministic test failures** | Parallel tests sharing cwd/env/global state [FACT] [SP-LEX] | `tempfile::TempDir` per test; `serial_test` for shared-state tests [FACT] [SP-LEX] |

### 4.3 Prevention patterns

The sources converge on a small set of structural preventions:

1. **Bounded iteration**: hard caps (`max_turns`, `max_iterations`, `max_concurrent_children`) plus duplicate-action detection and circuit breakers [FACT] [M-EH] [A-ER].
2. **Evidence-gated completion**: the gate, not the claim, moves the task to done [FACT] [PROOF-OR-STOP].
3. **Isolated execution**: one worktree + one `CARGO_TARGET_DIR` per agent; the canonical checkout is the owner's, not the agent's [FACT] [STEELBRAIN].
4. **Deterministic fixtures**: `tempfile` per test; no shared cwd/env; `serial_test` when global state is unavoidable [FACT] [SP-LEX].
5. **Context anchoring**: constraints and plan in the hot memory (card body); durable state via `write_todos` / task workspace, not conversation history [FACT] [CODEMEM] [ARUN].

---

## 5. Implement these

Concrete, actionable practices for the follow-up task to turn into repository rules and tooling. Each item is labelled by confidence and maps to a file that exists (or is explicitly to-be-created).

### Cargo workspace & toolchain

1. **[FACT]** The workspace layout, `[workspace.lints]`, `[workspace.package]`, `[workspace.dependencies]`, and the `default-members` restriction are already correct. **No change.** The `cargo xtask` alias, `rust-toolchain.toml` pin, `.cargo/config.toml` (`warnings = "deny"`), `clippy.toml` (`msrv`), and `deny.toml` (MIT allowlist) are all committed and reproducible. [LOCAL]
2. **[INFERENCE]** Add `[workspace.dependencies]` entries for internal crates (already has `oxidj-core`) so future internal deps use `{ workspace = true }` inheritance rather than duplicating versions. Currently only `oxidj-core` is listed; when `oxidj-cli` or others depend on each other, add them. *Low priority; do when the first cross-crate dependency is added.*

### Build performance

3. **[OPINION]** Do **not** add `sccache` or `mold`/`lld` yet. The Depot.dev real-world test showed negligible benefit for some codebases; OxiDJ's 2-crate workspace has no demonstrated compile-time bottleneck. M0-02 correctly skipped these. Revisit only if a measurement shows linking or compilation is the bottleneck. [LOCAL] [DEPOT] [RUST-TRAIN]

### Worktree & build-directory isolation

4. **[FACT]** Add a committed rule to `docs/layers-policy.md` (or a new `docs/worktree-isolation.md`) documenting that each agent worktree must use its own `CARGO_TARGET_DIR` (e.g. `CARGO_TARGET_DIR=target` inside the worktree, which is the default) and must **never** share a `target/` with another worktree or the canonical checkout. The Cargo issue documents that sharing causes artifact corruption. [CARGO-16804]
5. **[INFERENCE]** The Kanban protocol should set `CARGO_TARGET_DIR` explicitly in the worker environment (or document that the default per-worktree `target/` is already isolated). With the current model (one worktree per agent, `max_in_progress: 2`), the default is already safe; the rule is documentation, not code. *Low priority; the default is already correct, but the rule should be written down.*

### Testing & fixtures

6. **[INFERENCE]** Add to `docs/testing.md`: name `tempfile` as the standard crate for filesystem tests and `serial_test` for tests that must mutate global state. The principle is already stated ("tests run concurrently and must not share mutable cwd, environment, caches"); naming the tools makes it actionable for agents. [LOCAL] [SP-LEX]
7. **[OPINION]** Add a `.config/nextest.toml` with a CI profile (`retries = 2`, `fail-fast = true`, `failure-output = "immediate-final"`) and install `cargo-nextest` for Tier 2. The handoff checklist already lists nextest as the recommended runner; the config file makes the flakiness policy explicit and machine-readable. [LOCAL] [NEXTEST-RET]

### Verification & quality gates

8. **[FACT]** The handoff checklist's evidence format and the "task that has not run the gate is not complete" principle are already the correct implementation of Proof-or-Stop freshness. **No change needed** — but see item 9 for a strengthening. [LOCAL] [PROOF-OR-STOP]
9. **[INFERENCE]** Strengthen the model policy's escalation rule with an explicit freshness clause: "If any code changes after the gate is run, the gate must be re-run; gate evidence is void after the last edit." This is the human-enforceable form of the Proof-or-Stop `materialHash` check. [PROOF-OR-STOP]
10. **[OPINION]** Consider a committed `scripts/gate.sh` (or `cargo xtask evidence`) that runs the Tier-1 commands, captures their output, and writes a machine-readable evidence file (command, exit code, output digest) as a step toward Proof-or-Stop's execution-attested receipt. *Deferred; the current human-readable paste is sufficient for the current scale.*

### Context & failure modes

11. **[INFERENCE]** Lower `agent.max_turns` from 150 to a bounded-worker value (e.g. 50). Loop guides recommend 15–25 for most tasks; 150 is high for bounded research/coding tasks and widens the blast radius of a runaway loop. This was already recommended in `docs/research/agent-config-best-practices.md` §6(a) item 1. [LOCAL] [M-EH]
12. **[FACT]** Add to `AGENTS.md` an explicit worktree-isolation rule: "Never write outside `$HERMES_KANBAN_WORKSPACE`; confirm `pwd` before writing." The model policy records a real canonical-root incident; the rule makes it a policy-level guard. [LOCAL]
13. **[INFERENCE]** Add to `AGENTS.md` a verification-discipline rule: "Paste real exit codes / summary lines as evidence; a completion claim alone is not evidence." This is the highest-signal addition for agent reliability. [LOCAL] [PROOF-OR-STOP]
14. **[OPINION]** Add to `AGENTS.md` a context-anchor rule: "For long tasks, maintain a `write_todos` list of remaining steps; do not rely on conversation history for state." This directly prevents the CodeMem failure loop. [CODEMEM]

### Kanban task design

15. **[INFERENCE]** Keep card granularity at "finish and verify in one sitting" (Stringer's rule). The existing M0-D-card and M0-08-style cards are the right size. Avoid cards that touch the same shared files (`Cargo.toml`, `AGENTS.md`, `docs/research/`) in parallel — flag them with a `hotspot:` comment on the card. [ROGER] [CARGO-16804]
16. **[FACT]** The existing acceptance-criteria format on cards (explicit, checkable) and the parent-dependency gating are already correct. **No change.** [LOCAL] [HERMES-KANBAN]

---

## Sources

**Rust toolchain & workspace (primary):**
- [C-BOOK] The Cargo Book — Workspaces. https://doc.rust-lang.org/cargo/reference/workspaces.html
- [RUST-GUIDEL] Microsoft Rust Guidelines — Project. https://microsoft.github.io/rust-guidelines/guidelines/project/index.html
- [MARTEL] Miguel Martinez — standard-rust-project-layout. https://github.com/miguelmartens/standard-rust-project-layout
- [C-CONFIG] Cargo config — `build.warnings`. https://doc.rust-lang.org/cargo/reference/config.html#buildwarnings
- [CL-USAGE] Clippy usage (CI, `CARGO_BUILD_WARNINGS=deny`). https://doc.rust-lang.org/clippy/usage.html
- [RUSTFMT] rustfmt README. https://github.com/rust-lang/rustfmt
- [CARGOFMT-SRC] `cargo fmt --message-format json` is nightly-only. https://doc.rust-lang.org/stable/nightly-rustc/src/cargo_fmt/main.rs.html

**cargo xtask (primary):**
- [XTASK] matklad — cargo-xtask (README, pattern spec, limitations). https://github.com/matklad/cargo-xtask

**Build performance & caching (secondary/vendor, flagged as such):**
- [RUST-TRAIN] Microsoft RustTraining — Compile-Time and Developer Tools (sccache, mold, cargo-nextest). https://microsoft.github.io/RustTraining/engineering-book/ch08-compile-time-and-developer-tools.html
- [DEPOT] Depot.dev — Guide to faster Rust builds in CI (mold/sccache real-world). https://depot.dev/blog/guide-to-faster-rust-builds-in-ci
- [BIGIRON] Bigiron — Rust incremental cache strategies. https://www.bigiron.cc/guides/rust-incremental-cache-strategies-on-a-self-hosted-runner
- [TENEQ] tenequm/skills — dev-environment (kache, cargo-watch, rust-analyzer). https://github.com/tenequm/skills/blob/main/skills/rust-dev/references/dev-environment.md

**Compiler-driven iteration (primary):**
- [RUSTC-DEV] rustc-dev-guide — Suggested workflows, incremental compilation. https://rustc-dev-guide.rust-lang.org/building/suggested.html
- [RUST-INC] Rust Blog — Incremental compilation. https://blog.rust-lang.org/2016/09/08/incremental/

**Worktree & build-directory isolation (primary):**
- [CARGO-16804] rust-lang/cargo#16804 — No safe concurrent build cache (agent-driven development impact). https://github.com/rust-lang/cargo/issues/16804
- [STEELBRAIN] steelbrain/reims-vgpu — AGENTS.md (worktree + CARGO_TARGET_DIR patterns). https://github.com/steelbrain/reims-vgpu/blob/master/AGENTS.md

**Testing, coverage, fuzzing (primary + secondary):**
- [DOCTRINE] agh/doctrine — Rust Testing Tools (cargo-llvm-cov, cargo-fuzz, proptest, cargo-mutants). https://github.com/agh/doctrine/blob/main/guides/languages/rust/testing.md
- [MS-RUST] Microsoft RustTraining — Code Coverage (cargo-llvm-cov, proptest). https://microsoft.github.io/RustTraining/engineering-book/ch04-code-coverage-seeing-what-tests-miss.html
- [RADKIT] radkit — AGENTS.md (property/fuzz/coverage). https://github.com/agents-sh/radkit/blob/main/AGENTS.md
- [RUST-FUZZ] Rust Fuzz Book — cargo-fuzz coverage. https://rust-fuzz.github.io/book/cargo-fuzz/coverage.html
- [NEXTEST] nexte.st — Running tests (termination mode). https://nexte.st/docs/running/
- [NEXTEST-RET] nexte.st — Retries and flaky tests. https://nexte.st/docs/features/retries/
- [NEXTEST-LIST] nexte.st — Machine-readable listings (`--message-format json`, JUnit). https://nexte.st/docs/machine-readable/list/
- [NEXTEST-OCX] OCX Index — cargo-nextest (process-per-test, isolation). https://index.ocx.sh/nextest-rs/cargo-nextest
- [SP-LEX] softwarepatternslexicon — Test Organization and Management in Rust. https://softwarepatternslexicon.com/rust/testing-and-quality-assurance/test-organization-and-management/
- [CPP-RUST] Microsoft RustTraining — Testing Patterns for C/C++ Programmers (tempfile). https://microsoft.github.io/RustTraining/c-cpp-book/ch08-1-testing-patterns.html
- [INDUSAGI] Indusagi — Rust CLI Reference / Testing (determinism rules). https://www.indusagi.com/rust-cli/reference/testing

**Kanban & agentic task design (primary + secondary):**
- [HERMES-KANBAN] Hermes Agent — Kanban tutorial. https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban-tutorial
- [KANBAN-DECOMP] Hermes Agent — kanban_decompose.py (decomposer logic). https://github.com/nousresearch/hermes-agent
- [ROGER] Roger Stringer — Shipping Software When Agents Write the Code (decomposition, acceptance criteria). https://rogerstringer.com/guides/ai-project-management
- [AGENTIC-CODING] agentic-coding.github.io — Agentic Coding Principles & Practices. https://agentic-coding.github.io/
- [AGILE-V] Koch, C. — Agentic Agile-V: From Vibe Coding to Verified Engineering (arXiv:2605.20456). https://arxiv.org/html/2605.20456v1

**Verification & quality gates (primary):**
- [PROOF-OR-STOP] Huang, J. et al. — Proof-or-Stop: Don't Trust the Agent, Trust the Evidence (arXiv:2607.14890). https://arxiv.org/abs/2607.14890
- [AP-REPRO] AgentPatterns.ai — Reproduce-Before-Report Verification Gate. https://www.agentpatterns.ai/code-review/reproduce-before-report-verification-gate/
- [MOKEVIN] mokevin — agentic-coding-design-patterns: Feedback Loop. https://mokevnin.github.io/agentic-coding-design-patterns/en/give-agent-a-way-to-verify.html
- [PLAYBOOK-06] syv-ai/agentic-coding-playbook — Module 06: Verification and Quality Gates. https://github.com/syv-ai/agentic-coding-playbook/blob/main/docs/06-verification-and-quality.md

**Context & failure modes (primary + secondary):**
- [CONTEXT-ROT] arXiv:2607.17937v2 — When and How Context Rot Appears in Coding Agents. https://arxiv.org/pdf/2607.17937v2
- [BEHAV] arXiv:2604.02547v1 — Behavioral Drivers of Coding Agent Success and Failure. https://arxiv.org/html/2604.02547v1
- [ARUN] Arun Baby — Long-Context Agent Strategies. https://www.arunbaby.com/ai-agents/0052-long-context-agent-strategies/
- [CODEMEM] arXiv:2512.15813 — CodeMem: Architecturing Reproducible Agents. https://arxiv.org/html/2512.15813v1
- [M-EH] "AI Agent Error Handling: Retries, Timeouts, Fallbacks". https://www.misar.blog/@synor/articles/ai-agent-error-handling
- [A-ER] "AI Agent Error Recovery: Retry, Timeout, Backoff, and Circuit-Breaker Patterns". https://apidog.com/blog/ai-agent-error-recovery/
- [V-INC] "Claude — Runaway agent cost loop incidents". https://github.com/vkorost/state-of-ai-observability-2026/blob/main/reports/Claude%20-%20Runaway%20agent%20cost%20loop%20incidents.md

**Local (verified this session against the live repo/profile):**
- [LOCAL] `Cargo.toml`, `.cargo/config.toml`, `clippy.toml`, `rust-toolchain.toml`, `deny.toml`, `AGENTS.md`, `xtask/src/main.rs`, `docs/handoff-checklist.md`, `docs/layers-policy.md`, `docs/testing.md`, `docs/research/rust-quality-gates.md`, `docs/research/agent-config-best-practices.md`, `docs/research/rust-agent-workflow.md`, `docs/agent-model-policy.md`, `docs/agent-knowledge-policy.md`.
- [LOCAL-HB] Hermes Agent — Background systems (delegation / kanban / curator), hermes-agent skill `references/background-systems.md`.

---

*Research only. No code written; no repo file modified except this document. Fact / inference / opinion are labelled per claim; anything unverifiable has been flagged as opinion or inference rather than asserted as fact.*
