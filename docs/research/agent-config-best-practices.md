# Agent + repository configuration best practices (loops, unwanted patterns)

Status: research complete (shallow, one bounded pass). Owner: `sova` (`nous:poolside/laguna-s-2.1:free`).
Scope: OxiDJ — a project built largely by agent workers dispatched from a Kanban board into isolated Git worktrees, integrated by a lead. Research only: no code was written and no repo file was modified except this document.

This document answers the five research questions, cites every claim, and ends with a concrete **Recommended OxiDJ changes** section. Each claim is labelled:

- **[FACT]** — stated in a cited primary source.
- **[INFERENCE]** — my reasoning from cited facts (not directly stated).
- **[OPINION]** — judgement call / best-judgement recommendation, not proven.

Source URLs are listed inline and collected in §7.

---

## 1. Hermes / agent-runner configuration

### 1.1 What actually matters (verified against the live `sova` config)

Hermes resolves settings in this precedence order (highest first): CLI arguments → `config.yaml` → `.env` → built-in defaults. `config.yaml` wins for non-secret settings; secrets (API keys, tokens) belong in `.env`. [FACT] [H-CONFIG]

The config settings that matter for reliable operation, with the values the `sova` profile actually holds (verified this session by reading `~/.hermes/profiles/sova/config.yaml`):

| Setting | `sova` value | Default per docs | Notes |
|---|---|---|---|
| `model.default` | `poolside/laguna-s-2.1:free` | — | Primary route. |
| `model.fallback` / `fallback_provider` | `meituan/longcat-2.5-preview:free` / `nous` | — | Failover. |
| `agent.max_turns` | `150` | `90` [H-CONFIG] | See §1.4 — this is high for bounded tasks. |
| `agent.reasoning_effort` | `medium` | — | |
| `delegation.max_concurrent_children` | `2` | `3` [H-BG] | Conservative; matches the model policy's "at most two concurrent items". |
| `delegation.max_iterations` | `6` | `50` [H-CONFIG] | Tightly capped; good for bounded delegates. |
| `delegation.provider`/`model` | `openai-codex` / `gpt-6.1-sol` | — | Sol lane. |
| `kanban.max_in_progress` / `..._per_profile` | `2` / `1` | — | Conservative concurrency. |
| `memory.provider` | `hindsight` | — | Operational memory. |
| `terminal.cwd` | `''` (cleared) | `'.'` | Cleared deliberately; prevents a stale default cwd. |
| `compression.*` | not set → defaults | `threshold 0.50`, `tail_mode lean`, `protect_last_n 20` [H-COMPRESS] | Sensible defaults. |
| `approvals.mode` | `"off"` | `smart`/`manual`/`off` | No approval gate; see §5. |

### 1.2 Context compression (thresholds, tail protection, micro-compaction)

Hermes runs **two independent compression layers** [FACT] [H-COMPRESS]:

1. **Gateway session hygiene** — fires at a fixed **85%** of context, before the agent sees a message. A safety net for sessions that grew between turns (e.g. overnight accumulation). Fires only when `len(history) >= 4` and compression is enabled.
2. **Agent `ContextCompressor`** — fires at **`compression.threshold` (default 50%)** inside the tool loop, using real API-reported token counts (the "usage anchor"). This is the primary mechanism.

Key parameters and pitfalls [FACT] [H-COMPRESS]:

- **`threshold`** is a fraction of the **main** model's context window, never the auxiliary/summary model's. On a 262K-token model at `0.50` the trigger is 131,072 tokens.
- **`model_thresholds`** (per-model overrides) is useful when swapping between very different window sizes; keys are substring-matched, longest wins, and a **small-context floor of 0.75** still applies on top (raise-only) for models under 512K.
- **`tail_mode: lean`** (default) keeps a clamped tail (2.5% × window, 10K floor / 25K cap) and carries continuity in the summary + a mechanically-extracted anchor index + a `session_search` recovery pointer. `legacy` keeps a `target_ratio`-sized verbatim tail (~100K+ tokens on big windows) — far more expensive.
- **`protect_last_n` (default 20)** minimum recent messages always preserved; **`min_tail_user_messages` (default 1)** guarantees at least one real user turn survives. Raising this to e.g. 3 keeps bulky tool outputs from crowding out real instructions.
- **Summary-model context must be ≥ the main model's window.** If it is smaller, the summary call returns a context-length error and the compressor **silently drops the middle without a summary** — "the most common cause of degraded compaction quality." [FACT] [H-COMPRESS]
- **Failure cooldown**: a failed/stalled summary arms an escalating per-session cooldown (60s → 300s → 900s, persisted in `state.db`) so a broken summary backend does not re-fire every turn. This is the anti-thrash guard for compaction. [FACT] [H-COMPRESS]
- **`in_place: true`** (default) rewrites the live message list on the same session id (no rotation), which eliminated a session-rotation bug cluster. [FACT] [H-COMPRESS]

**Prompt caching** [FACT] [H-COMPRESS]: Hermes uses Anthropic `cache_control` breakpoints ("system_and_3" strategy) and reports up to ~75% input-token reduction on multi-turn conversations. Two pitfalls: (1) the system prompt must stay stable — mutating it mid-conversation breaks the cache; (2) **model identity is part of the cache key**, so any mid-conversation model change (explicit `/model`, primary-model fallback, or credential-pool rotation) means zero cache hits and a full-price re-read. The docs explicitly warn: don't silently swap the model or credentials mid-session.

### 1.3 Fallback / retry chains

The agent loop handles primary-model failure (429 rate limit, 5xx, 401/403) by walking the `fallback_providers` list in order, attempting credential refresh on 401/403 before failing over. [FACT] [H-LOOP] Auxiliary tasks (vision, compression, web extraction) each have their own independent fallback chain under `auxiliary.*`. [FACT] [H-LOOP]

The `sova` profile configures a verified free chain: primary `poolside/laguna-s-2.1:free` (Nous) → fallback `meituan/longcat-2.5-preview:free` (Nous). This is documented and evidence-backed in `docs/agent-model-policy.md`. [FACT] [LOCAL]

Pitfalls [FACT] [H-COMPRESS] [INFERENCE]:
- A fallback that silently changes the model **breaks prompt caching** (§1.2) and can quietly degrade output quality. Log which rung served each request.
- A second model on the **same exhausted quota** is not an independent fallback (the model policy explicitly calls this out for Codex). [FACT] [LOCAL]

### 1.4 Delegation and concurrency caps

- `delegation.max_concurrent_children` (sova: `2`) caps parallel subagents; `delegation.max_iterations` (sova: `6`) caps each child's turn budget. [FACT] [H-CONFIG] [H-BG]
- Subagents get **independent** iteration budgets capped at `delegation.max_iterations` (default 50); total iterations across parent + children can exceed the parent's cap. [FACT] [H-LOOP]
- `delegate_task` children share the parent's container/backend; concurrent `cd`/writes to the same path collide. [FACT] [H-BG]
- `kanban.max_in_progress` / `max_in_progress_per_profile` (sova: `2` / `1`) bound how many cards run at once. The model policy caps concurrency at two independent items with independent worktrees. [FACT] [LOCAL]

**`agent.max_turns` is the most consequential and most-mis-set value.** The configuration reference default is `90` [FACT] [H-CONFIG]; the loop-guides recommend a hard cap of **15–25 tool-call turns for most tasks**, with long research agents justifying 50+ but "there must be a number." [FACT] [M-EH] The `sova` profile sets **150** [FACT] [LOCAL] — high for bounded research/coding tasks. A real incident report documents a sub-agent that retried a failed `npm install` verbatim for **~27M tokens / 300+ identical tool calls / ~4.6 hours** specifically because there was **no max-iteration limit on the sub-agent** and no consecutive-failure detector. [FACT] [V-INC] **[INFERENCE]** For OxiDJ's bounded worker tasks, 150 turns is a wide loop budget; a lower cap (e.g. 25–50) plus the duplicate-action/circuit-breaker guards in §2 would bound blast radius.

### 1.5 Profile isolation, skill loading, memory

- Profiles live under `~/.hermes/profiles/<name>/` with the same layout as the home dir; the active profile resolves the real home from `$HERMES_HOME`. [FACT] [H-CONFIG]
- `terminal.home_mode` (default `auto`) keeps the real OS-user `HOME` for host CLIs; `profile` gives strict per-profile CLI config isolation but hides `~/.ssh`, `~/.gitconfig`, etc. unless initialized inside the profile home. [FACT] [H-CONFIG]
- **Skills** are the progressive-disclosure mechanism: load one via `skill_view(name)` only when relevant, don't preload general process skills. [FACT] [H-SKILLS]
- **Memory**: `memory.provider: hindsight` is set. The background-systems reference stresses a critical distinction — a *configured* provider, an *installed* plugin, an *available* status, and a *successful* memory operation are four distinct claims; verify the exact operation requested. A `queued` retain is not proof of persistence. [FACT] [H-BG]
- `hermes config check` reports sections missing from an older config after updates. [FACT] [H-CONFIG]

---

## 2. Loop and retry prevention

### 2.1 The five loop classes and where each guard lives

| Loop class | Primary guard | Where it lives |
|---|---|---|
| Quota / rate-limit retry storm | Honour `Retry-After`; exponential backoff + jitter; attempt cap | Config + provider SDK [FACT] [A-ER] |
| Card re-claim cycling | `kanban.failure_limit` (default 2) auto-blocks after consecutive spawn failures | Config [FACT] [H-BG] |
| Context-compaction thrashing | Escalating failure cooldown (60→300→900s) | Config + code [FACT] [H-COMPRESS] |
| Infinite delegation | `max_iterations` + `max_concurrent_children` + `max_spawn_depth` | Config [FACT] [H-CONFIG] |
| Tool-call retry loop | Duplicate-action detection + circuit breaker | Policy + code [FACT] [M-EH] |

### 2.2 Rate-limit / retry storms

The consensus pattern across sources is **exponential backoff with full jitter**, capped in both delay and attempt count [FACT] [M-EH] [A-ER] [G-FT]:

- Base delay ~1s, cap ~60s, **4–5 max retries for LLM calls, 2–3 for tools**. [FACT] [M-EH]
- **Always honour `Retry-After`** when the provider sends it (Anthropic/OpenAI do on 429s). Retrying immediately re-triggers the limit. [FACT] [A-ER] [M-EH]
- **Never retry permanent errors** (400, 401, 413) — they fail identically every time and delay alerting. [FACT] [M-EH]
- Without jitter, concurrent agents retry at the same instant and re-trigger the limit ("thundering herd"). [FACT] [M-EH]
- Classify tools as **idempotent** (read-only, safe to retry) vs **non-idempotent** (side effects). Non-idempotent calls need a stable **idempotency key**; without one, route ambiguous timeouts to a human/reconciliation queue instead of auto-retrying. [FACT] [M-EH] [A-ER]
- Bound the retry budget in **wall-clock time**, not just attempt count, so a chain of slow-but-legal calls can't outlive the caller's SLA. [FACT] [C-REL]

### 2.3 Circuit breakers and loop guards

A **circuit breaker** tracks the rolling error rate per provider/tool and, past a threshold (commonly **50% errors over 30 seconds**), "opens" — failing fast or rerouting for a cooldown (30–60s) before a single probe. [FACT] [M-EH] [A-ER] Wire it **per dependency**, not globally, so a dead search API doesn't stop a healthy billing one. [FACT] [A-ER]

**Loop guards** [FACT] [M-EH] [G-EH]:
- **Max iterations**: hard cap of 15–25 tool-call turns for most tasks; long research agents may justify 50+, but there must be a number. [FACT] [M-EH]
- **Duplicate-action detection**: same tool + same args 2–3 times → break the loop, force a re-plan or escalate. [FACT] [M-EH]
- **Cost caps**: track cumulative tokens × price per task; kill at a dollar threshold ($0.50–$5 typical). [FACT] [M-EH]

**Pre-request telemetry trip signals** [FACT] [C-REL]: repeated identical tool calls (≥N times, hashed on tool-name + args — "the strongest loop signal, cheaply computed"), span depth above threshold, token-rate per session > 2× baseline, cumulative tokens above a per-session hard budget, p99 latency spikes, tool-error rate above threshold, response-quality degradation, and **scope violations** (unauthorized tool used). The missing primitive is a pre-request circuit breaker that consumes these signals and intervenes. [FACT] [C-REL]

### 2.4 Where the guard should live (config vs code vs policy)

**[INFERENCE]** The sources converge on a three-layer placement:
- **Config** for the cheap, universal knobs: `max_turns`, `max_iterations`, `max_concurrent_children`, `compression.threshold`, `failure_limit`, `Retry-After` handling, backoff caps. These need no code and are per-profile.
- **Code** for the stateful guards that need memory across calls: the compaction failure cooldown (Hermes implements this), circuit breakers, duplicate-action detection, idempotency keys. These must live in the runner/framework.
- **Policy** (AGENTS.md / SOUL.md / model policy) for the behavioural rules an agent can follow but that need human judgment: "escalate rather than silently switching to a paid model", "verify evidence, don't accept a completion claim", "use isolated worktrees". Policy is necessary but not sufficient — see §4 on why agents ignore rules.

---

## 3. Agent persona / system-prompt ("soul") best practices

### 3.1 What makes an effective instruction set

Anthropic's "Building Effective Agents" distils three core principles [FACT] [A-BEA]:
1. **Simplicity** — find the simplest solution that works; add complexity only when it demonstrably improves outcomes.
2. **Transparency** — explicitly show the agent's planning steps (so failures are debuggable).
3. **Careful agent-computer interface (ACI)** — invest in tool documentation and testing as much as prompt engineering.

For the persona/system-prompt specifically, the effective ingredients are: a clear **role**, a **mission**, **constraints**, **escalation rules**, and **verification discipline**. The `sova` SOUL.md currently has role + mission only (7 lines) [FACT] [LOCAL] — it lacks explicit constraints, escalation, and verification rules.

### 3.2 What harms it

- **Contradictions / conflicting instructions** — create ambiguity about expected behaviour; measured at 28% prevalence in real AGENTS.md files and frequently co-occurring with Context Bloat. [FACT] [ARX-SMELLS]
- **Vagueness** — "be careful" or "do a good job" is not actionable. The AGENTS.md guidance is "commands, not prose": `Run pnpm test` beats "we have a comprehensive testing strategy." [FACT] [T-AGENTS]
- **Over-length / Context Bloat** — the file exceeds the size at which agents reliably honour its content (heuristic: ≥200 lines); 42% prevalence. [FACT] [ARX-SMELLS]
- **Redundant boilerplate** — Lint Leakage (restating rules already enforced by linters/formatters) is the **most common smell at 62%**; it consumes context and tokens without improving outcomes. [FACT] [ARX-SMELLS]
- **Silent instability** — mutating the system prompt or model mid-conversation breaks prompt caching and session continuity. [FACT] [H-COMPRESS]

### 3.3 How to keep it maintainable

- **Centralize prompts in config**, not scattered through code. [FACT] [A-BEA]
- **Modular design**: prompts in centralized config files, tools as discrete reusable modules, agents defined with only the tools they need. [FACT] [A-BEA]
- **Poka-yoke the interface**: design tools/params so the model can't make mistakes (e.g. require absolute filepaths after observing the agent drift out of the root). [FACT] [A-BEA]
- **Grow from evidence**: "When you notice an agent making the same mistake repeatedly, add an instruction to address it." [FACT] [O-AGENTS] [T-AGENTS]
- **Keep it tight**: aim ≤150 lines; long files bury important information and waste context tokens. [FACT] [O-AGENTS] [R-AGENTS]
- **Separate identity from project rules**: in Hermes, `SOUL.md` (in `$HERMES_HOME`) is identity (slot #1 in the system prompt) and is always loaded; `AGENTS.md` (cwd-only) is portable project rules. Don't put project rules in `~/.hermes/AGENTS.md`. [FACT] [H-PROJ]

---

## 4. Repository agent-rules (`AGENTS.md`) best practices

### 4.1 What belongs, what does not

**Belongs** [FACT] [R-AGENTS] [T-AGENTS] [O-AGENTS] [S-AGENTS]:
- Exact **build / lint / test / run commands** (with flags, not just tool names).
- **Project map** — where things live, which directories not to touch.
- **Small invariants** — "things that fail silently", non-obvious conventions, guardrails.
- **Hard "never do this" rules** — the highest-signal files pair exact commands with a few hard rules. [FACT] [S-AGENTS]
- **Branching / commit / PR conventions.**
- **Reference links** to architecture docs / ADRs — but each reference must explain its purpose or scope (see Blind References below). [FACT] [ARX-SMELLS]

**Does not belong** [FACT] [ARX-SMELLS] [R-AGENTS]:
- **Lint Leakage** — rules already enforced by linters/formatters (naming conventions, formatting, import ordering, max line length, generic style-guide recommendations). 62% prevalence. These belong in `clippy.toml`/`rustfmt`, not the agent context.
- **Context Bloat** — restating README content, low-priority detail, examples. ≥200 lines. 42% prevalence.
- **Skill Leakage** — specific, rarely-used, or highly context-dependent instructions that belong in on-demand skill files, not the always-loaded file. 35% prevalence.
- **Blind References** — references to external documents/files/directories with no explanation of purpose or scope. 16% prevalence.
- **Init Fossilization** — a file generated by `/init` (or equivalent) and never reviewed since (detected via single-commit history). 24% prevalence.
- **Secrets** — never credentials or keys (the paper explicitly discarded this as a smell because it's not coding-agent-specific, but it's an obvious hard rule). [FACT] [ARX-SMELLS]

### 4.2 The right granularity and non-redundancy

- **Aim for the litmus test**: "Would removing this line cause the agent to make a mistake it wouldn't otherwise make?" If not, delete it. [FACT] [R-AGENTS]
- **Commands over prose.** [FACT] [T-AGENTS]
- **One topic per file**; offload detail into referenced files and keep AGENTS.md as a map. [FACT] [R-AGENTS] [O-AGENTS]
- **Don't duplicate across config layers**: lint rules live in `clippy.toml` (already done — `clippy.toml` sets `msrv` and `disallowed-methods`); the licence policy lives in `deny.toml`; the toolchain pin lives in `rust-toolchain.toml`. AGENTS.md should *point to* these, not restate them. [FACT] [LOCAL]
- **Nest for monorepos**: agents read the nearest AGENTS.md, so the closest file takes precedence. (OxiDJ is not yet a monorepo, but the pattern matters as crates are added.) [FACT] [T-AGENTS]

### 4.3 Making rules an agent actually follows

**[INFERENCE]** The sources agree rules are followed when they are: (1) **specific and actionable** (commands, not prose), (2) **non-contradictory**, (3) **short enough to stay salient**, and (4) **evidence-backed** (added in response to an observed repeated failure). The single biggest lever is removing everything that fails the litmus test — a short, command-first, hard-rules-only file is far more likely to be honoured than a comprehensive one. [FACT] [S-AGENTS] [R-AGENTS] [ARX-SMELLS]

### 4.4 The OxiDJ AGENTS.md today

The current `AGENTS.md` is 10 lines and mostly rule-shaped (good — no bloat, no lint leakage) [FACT] [LOCAL]. Gaps against §4.1:
- **No exact commands.** It says "Verify behavior and tests" but never states `cargo xtask ci` / `cargo nextest run` / `cargo deny check licenses`. The highest-signal files lead with commands. [FACT] [S-AGENTS] [T-AGENTS]
- **No worktree / isolation rule** beyond "use isolated Git worktrees" — no explicit "never write to the canonical checkout" or "confirm cwd before writing". (See §5.)
- **No escalation / verification discipline** — no "evidence, not completion claims" rule.
- **References without scope**: it links `docs/agent-knowledge-policy.md` and `docs/agent-model-policy.md` (both exist) but doesn't say when to read them — a mild Blind Reference. [FACT] [ARX-SMELLS]

---

## 5. Verification and anti-patterns

The unwanted patterns OxiDJ has actually hit, and the configuration/policy that prevents each:

### 5.1 Silently-ignored environment variables

- **Pattern**: a worker sets an env var (e.g. `CARGO_BUILD_WARNINGS=deny`) that the live runner doesn't actually read, then reports success anyway. The model policy records a real falsification test: a planted `unused_variables` warning exited `0` under plain `cargo clippy` but exited `101` under `CARGO_BUILD_WARNINGS=deny` — proving the env var *is* read, but only because it was tested. [FACT] [LOCAL]
- **Prevention**: the persistent `.cargo/config.toml` form (`[build] warnings = "deny"`) makes the gate self-contained on a fresh clone without any per-shell env var. [FACT] [LOCAL] **[INFERENCE]** Prefer committed config over env-var-only gates; where an env var is the only mechanism, add a falsification test (plant a warning, confirm non-zero exit).

### 5.2 Self-reported success instead of reproduced evidence

- **Pattern**: a worker marks a task done claiming "tests pass" without a reproduced run; the lead must independently verify.
- **Prevention**: the model policy's escalation rule — "the model's completion claim alone is not evidence"; require "correct worktree, actual compiler/tests where applicable, diff review, and whole-repository status". [FACT] [LOCAL] The Anthropic guide adds that coding agents are powerful *because* code is verifiable via tests, and that "human review remains crucial for ensuring solutions align with broader system requirements." [FACT] [A-BEA] **[INFERENCE]** Require the worker to paste the actual exit code + summary line (as the rust-quality-gates handoff checklist does), not a prose claim.

### 5.3 Stale running-config vs written-config divergence

- **Pattern**: the file on disk says one thing; the live runtime uses another (e.g. a provider pin that only applies on the next dispatch).
- **Prevention**: the model policy notes "a pin only applies on the next dispatch, so a worker already mid-flight keeps the route it started on" — i.e. don't assume a config edit changes in-flight workers. [FACT] [LOCAL] Hermes resolves config at startup from CLI → config.yaml → env → defaults, so a mid-session edit to `config.yaml` may not take effect until restart. [FACT] [H-CONFIG] **[INFERENCE]** Verify the *resolved* config (`hermes config get <key>`), not just the file, before claiming a setting is active.

### 5.4 Workers escaping their worktree

- **Pattern**: a worker writes to the canonical checkout instead of its isolated worktree. The model policy records a real case: "A Nemotron probe created a compilable Rust test crate in the canonical repository root, not the intended isolated benchmark directory." [FACT] [LOCAL]
- **Prevention**: the Kanban protocol sets `HERMES_KANBAN_WORKSPACE` and the worker must `cd` there first; the background-systems reference says to record the intended absolute cwd and Git status in both the worktree and canonical checkout *before*, then inspect both *after* — "checking only that directory or accepting a reported test count misses an isolation failure." [FACT] [H-BG] **[INFERENCE]** An explicit AGENTS.md rule ("never write outside `$HERMES_KANBAN_WORKSPACE`; confirm `pwd` before writing") makes this a policy-level guard, not only a convention.

### 5.5 Context that grows without bound

- **Pattern**: long sessions accumulate context until compaction thrashes or the window overflows.
- **Prevention**: Hermes' dual compression (§1.2) + gateway hygiene at 85% + the escalating failure cooldown. [FACT] [H-COMPRESS] `compression.in_place: true` keeps one durable session id. **[INFERENCE]** For workers, the bigger risk is *tool-output* bloat; Hermes caps terminal output (`tool_output.max_bytes` 50K default), read_file lines (`max_lines` 2000), and spills oversized results to disk. [FACT] [H-CONFIG]

### 5.6 The verification checklist (synthesis)

**[INFERENCE]** Drawing the sources together, a reliable worker handoff needs: (1) a committed, reproducible gate (not env-var-only); (2) pasted exit codes / summary lines, not prose claims; (3) independent reproduction by the lead before integration; (4) explicit worktree-isolation confirmation; (5) a resolved-config check (`hermes config get`), not just a file read; (6) a hard loop budget (`max_turns` / `max_iterations`) + duplicate-action detection. These map directly to the OxiDJ anti-patterns above.

---

## 6. Recommended OxiDJ changes

Concrete, actionable, each mapping to a file that exists (or is explicitly to-be-created). Labelled by confidence.

### (a) Hermes config — `~/.hermes/profiles/sova/config.yaml`

1. **[OPINION]** Lower `agent.max_turns` from `150` to a bounded-worker value (e.g. `50`). 150 is high for bounded research/coding tasks; loop guides recommend 15–25 for most tasks, 50+ only for long research. This bounds the blast radius of a runaway loop. *Verified that the current value is 150 [FACT]; the target value is judgement.*
2. **[OPINION]** Keep `compression` on defaults (threshold 0.50, `tail_mode: lean`, `protect_last_n: 20`) — these are already correct. No change needed; recorded so a future reader knows it was considered.
3. **[INFERENCE]** Set `compression.min_tail_user_messages: 2` (from default `1`) so bulky tool outputs can't crowd out the most recent real instruction — cheap insurance for long worker turns.
4. **[OPINION]** `approvals.mode: "off"` is appropriate for a trusted, isolated, free-route research worker, but **[INFERENCE]** consider `smart` if the worker ever touches shared/external systems. Flagged, not changed.
5. **[FACT]** The fallback chain and `terminal.cwd: ''` are already correct and verified — no change.

### (b) Agent persona / system prompt — `~/.hermes/profiles/sova/SOUL.md`

6. **[OPINION]** Expand the 7-line SOUL.md to add, in the same terse style: a **verification discipline** line ("paste real exit codes / summary lines; never claim success without reproduced evidence"), an **escalation** line ("escalate on genuine ambiguity; don't guess"), and a **worktree-isolation** line ("never write outside `$HERMES_KANBAN_WORKSPACE`; confirm `pwd` first"). These directly target the OxiDJ anti-patterns in §5. *The current file has role + mission only [FACT]; the additions are best-judgement.*
7. **[INFERENCE]** Keep SOUL.md short (≤~25 lines) and identity-only; project rules belong in AGENTS.md, not SOUL.md (per the Hermes separation). Don't let SOUL.md become a second AGENTS.md.

### (c) `AGENTS.md` — repository root

8. **[OPINION]** Add an exact **commands** block at the top (commands-over-prose is the single highest-signal improvement):
   ```sh
   cargo fmt --all -- --check
   CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features --keep-going
   cargo check --all-targets --all-features
   cargo deny check licenses
   ```
   *These are the stable Tier-1 commands verified in `docs/research/rust-quality-gates.md` [FACT].*
9. **[OPINION]** Add two hard rules targeting observed failures: "Never write outside `$HERMES_KANBAN_WORKSPACE` — confirm `pwd` before writing" and "Paste real exit codes / summary lines as evidence; a completion claim alone is not evidence."
10. **[OPINION]** Add a one-line **scope note** to each existing link (e.g. "Read `docs/agent-model-policy.md` before changing model routing or cost controls") to remove the mild Blind Reference smell.
11. **[FACT]** Do **not** restate lint rules, the licence policy, or the toolchain pin in AGENTS.md — they already live in `clippy.toml`, `deny.toml`, and `rust-toolchain.toml` (avoiding Lint Leakage, the most common smell). Keep AGENTS.md under ~50 lines.

### (d) Repository config — `Cargo.toml`, `.cargo/config.toml`, `docs/`

12. **[FACT]** **Written-config vs repo divergence**: `Cargo.toml` references `cargo xtask ci`, `cargo xtask layers`, `docs/handoff-checklist.md`, and `docs/layers-policy.md`, but the `xtask/` crate and both docs **do not exist** (verified this session). The `.cargo/config.toml` alias `xtask = "run --package xtask --"` would therefore fail. **[INFERENCE]** Either create the `xtask` crate + referenced docs, or remove the dead references. This is a real "stale written-config" defect and a textbook Blind Reference.
13. **[INFERENCE]** Add a `docs/handoff-checklist.md` (referenced but missing) capturing the Tier-1 evidence block from `docs/research/rust-quality-gates.md` — this makes "evidence, not claims" a committed, reproducible gate rather than a convention.
14. **[OPINION]** Consider a committed `scripts/gate.sh` that runs the Tier-1 commands and exits non-zero on any failure, so the gate is one command and self-contained (reduces reliance on a worker remembering each command).

### Explicitly out of scope (researched, no change recommended)

- The `clippy.toml` / `deny.toml` / `rust-toolchain.toml` split is already correct and non-redundant. [FACT]
- The Kanban concurrency caps (`max_in_progress: 2`, `max_in_progress_per_profile: 1`) and delegation caps (`max_concurrent_children: 2`, `max_iterations: 6`) are already conservative and match the model policy. [FACT]

---

## 7. Sources

**Hermes Agent (primary, authoritative):**
- [H-CONFIG] Configuration reference — https://hermes-agent.nousresearch.com/docs/user-guide/configuration
- [H-COMPRESS] Context Compression and Caching — https://hermes-agent.nousresearch.com/docs/developer-guide/context-compression-and-caching
- [H-LOOP] Agent Loop Internals — https://hermes-agent.nousresearch.com/docs/developer-guide/agent-loop
- [H-PROMPT] Prompt Assembly — https://hermes-agent.nousresearch.com/docs/developer-guide/prompt-assembly
- [H-BG] Background systems (delegation / kanban / curator) — hermes-agent skill, `references/background-systems.md`
- [H-PROJ] Project context files — hermes-agent skill, `references/project-context-files.md`
- [H-SKILLS] Skills system — https://hermes-agent.nousresearch.com/docs/user-guide/features/skills

**Agent design / reliability:**
- [A-BEA] Anthropic, "Building Effective Agents" — https://www.anthropic.com/engineering/building-effective-agents
- [M-EH] "AI Agent Error Handling: Retries, Timeouts, Fallbacks" — https://www.misar.blog/@synor/articles/ai-agent-error-handling
- [A-ER] "AI Agent Error Recovery: Retry, Timeout, Backoff, and Circuit-Breaker Patterns" — https://apidog.com/blog/ai-agent-error-recovery/
- [G-FT] "LLM agent fault tolerance patterns" — https://runguard.dev/seo/llm-agent-fault-tolerance-patterns.html
- [G-EH] "AI Agent Guidebook — error handling" — https://github.com/ai-infra-curriculum/ai-agent-guidebook/blob/main/best-practices/error-handling.md
- [C-REL] "Agent Guardrails and the AI Agent Reliability Playbook" — https://changegamer.ai/articles/agent-reliability-in-production
- [V-INC] "Claude — Runaway agent cost loop incidents" — https://github.com/vkorost/state-of-ai-observability-2026/blob/main/reports/Claude%20-%20Runaway%20agent%20cost%20loop%20incidents.md

**AGENTS.md / repository rules:**
- [ARX-SMELLS] Santos et al., "Configuration Smells in AGENTS.md Files" (arXiv:2606.15828) — https://arxiv.org/abs/2606.15828
- [R-AGENTS] Red Hat, "Standardize project context with AGENTS.md and agent skills" — https://developers.redhat.com/articles/2026/07/27/standardize-project-context-agentsmd-and-agent-skills
- [T-AGENTS] Tembo, "What Is AGENTS.md?" — https://www.tembo.io/blog/agents-md
- [O-AGENTS] Addy Osmani, "AGENTS.md — giving agents project context" — https://addyosmani.com/agents/15-agents-md/
- [S-AGENTS] Security Boulevard, "6 AGENTS.md Examples From Real Production Repos" — https://securityboulevard.com/2026/06/6-agents-md-examples-from-real-production-repos/
- [CC-MEM] Claude Code, "How Claude remembers your project" — https://code.claude.com/docs/en/memory

**Local (verified this session against the live repo/profile):**
- [LOCAL] `~/.hermes/profiles/sova/config.yaml`, `~/.hermes/profiles/sova/SOUL.md`, `AGENTS.md`, `Cargo.toml`, `.cargo/config.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`, `docs/agent-model-policy.md`, `docs/agent-knowledge-policy.md`, `docs/research/rust-quality-gates.md`; and the absence of `xtask/`, `docs/handoff-checklist.md`, `docs/layers-policy.md`.

---

*Research only. No code written; no repo file modified except this document. Fact / inference / opinion are labelled per claim; anything unverifiable has been flagged as opinion or inference rather than asserted as fact.*
