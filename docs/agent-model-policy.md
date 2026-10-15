# Agent Model Policy

## Routing

- The OxiDJ lead currently runs `gpt-6-luna` through `openai-codex`; use it for orchestration, difficult synthesis, integration, and escalation.
- Use Kanban for bounded coding workers. Prefer an explicitly validated free/low-cost worker for small, isolated tasks; escalate to Luna when the task is complex or the worker is insufficient. Do not assume Hindsight's OpenRouter model is validated for coding: its verification covered Hindsight extraction/structured output only.
- Use `delegate_task` as the isolated independent reviewer for consequential or ambiguous decisions, not for routine coding. The verified default route is `openai-codex` / `gpt-6.1-sol` at **medium** reasoning; the installed account-scoped Codex catalog exposed that exact model ID. The lead remains responsible for the final decision.

## Sol delegation lane — scope and effort (user authorization, 2026-10-15)

The user authorized using **tightly scoped Sol delegates at medium reasoning** for **important decisions and important coding work**, with **high used rarely**. This is a deliberate widening of the earlier decision-review-only rule, and it comes with a deliberate ceiling:

- **Default effort is `medium`.** `delegation.reasoning_effort = medium`, `delegation.max_iterations = 6`. Raise to `high` only for a named, specific difficulty that medium actually failed on — never as a first move and never as a substitute for a clearer task.
- **Scope stays tight.** One decision or one coding objective per child, a small packet, explicit exclusions, and the evidence the child needs — not the project history. A delegate that has to ask what the project is was mis-scoped.
- **Sol is the exception, not the default.** Routine research, mechanical changes, scaffolding and bounded tasks stay on the free Nous routes; Sol is spent where being wrong is expensive or the Rust reasoning is genuinely hard.
- **Importance is the admission test.** "Important" means: expensive to reverse, affects data integrity or interoperability, sets a crate/API boundary others will build on, or is difficult Rust reasoning (ownership/lifetimes/generics, async/Send/Sync, unsafe/realtime). A task that is merely tedious does not qualify.
- **Important coding work follows the same discipline as review work**: give the child the acceptance criteria and require real compiler/test/gate evidence back, not a completion claim.
- **The free-route restriction is superseded only to this extent.** Blanket "Nous free only" applied while Codex limits were exhausted; with limits cleared, the rule is now *free by default, Sol/medium for important work*. PAYG routes still require explicit approval.
- Use Astra only for an exceptional, high-impact decision after Sol remains uncertain, and only if the exact model is live-available through the user's Codex entitlement. Astra is not configured as a routine route.

## Dedicated decision delegation

- Keep each decision packet to about 300–800 words, preferably shorter, with only: **DECISION, WHY NOW, CONSTRAINTS, OPTIONS, EVIDENCE, CURRENT RECOMMENDATION (optional), OUTPUT CONTRACT**.
- Ask for exactly: recommendation; 3–5 short reasons; strongest counterargument; confidence; and what would change the decision (if relevant). No essay, implementation, backlog, or redesign.
- Give the reviewer no tools by default. Grant read-only file access or web access only when the decision genuinely requires that evidence; never grant write/terminal access merely because it is available.
- The reviewer must not re-delegate, implement, expand scope, modify Kanban, or write Hindsight. Use one consequential decision per child. Configure `delegation.max_iterations = 6`; raise it only when a required verification step needs more.
- If Sol strongly disagrees, verify the disputed evidence. Escalate an unresolved consequential disagreement to the user rather than silently choosing either view.

## Cost and quota controls

- Check provider, model, quota, and fallback behavior before dispatch. No surprise pay-as-you-go usage: do not select paid API-key routes or add paid fallbacks without explicit authorization.
- Prefer validated free tiers for bounded work, keep prompts and worker scope small, and escalate rather than silently switching to a paid model.
- The profile fallback chain is configured and cost-free (see "Verified fallback chain" below). Add a paid entry only with explicit user authorization.
- Hindsight's `nvidia/nemotron-3-super-120b-a12b:free` setting is a cluster-wide memory-extraction configuration, not a coding-worker recommendation; leave it unchanged absent a separate authorization.

## Concurrency

Keep Kanban and delegated work conservative: at most two concurrent items, with independent scopes and worktrees for concurrent code changes. Current profile limits are set to two for both Kanban in-progress work and `delegate_task` children.

## Worker validation and recovery (2026-10-15)

- **Approved worker (2026-10-15, `M0-01b`):** `poolside/laguna-s-2.1:free` via **Nous Portal** is validated for **small bounded tasks** — five isolated Rust tasks with real compiler/test evidence, and the lead independently reproduced the typed-API (2 passed, clippy clean under `-D warnings`) and test-suite (8 passed) results. Raw evidence: `docs/research/m0-01b-benchmark-evidence.md`; harness: `scripts/m0-01b-benchmark.sh`. This approves a worker for bounded work only — it does **not** approve the model for hard Rust reasoning, and `laguna-s-2.1:free` was announced as expiring 2026-10-31, so re-validate after that date.
- Earlier candidates were **not** approved. The live OpenRouter catalog listed `nvidia/nemotron-3.5-lightning:free` and `google/gemma-4-31b-it:free` with tool support and zero prices, but catalog lookup is not a coding-worker test. See `docs/research/model-worker-benchmark.md`.
- A Nemotron probe created a compilable Rust test crate in the **canonical repository root**, not the intended isolated benchmark directory. Twelve unit tests passed on independent readback, but the unexpected working-directory write disqualifies this as an isolated agentic worker result. The Gemma probe encountered HTTP 429 on all three retries before tool execution.
- Nous Portal authentication was reported, but no coding task on its inference route was verified; no local inference route was validated. Do not treat either as fallback merely because credentials or binaries exist.
- An empty fallback chain was correct while no independent route was verified: an earlier HTTP 429 probe stopped after retries without provider switching. A second model on the same exhausted Codex allowance is not an independent fallback; no PAYG route may be enabled without explicit user approval.
- Re-evaluate pinned candidates only when service recovers: five isolated Rust tasks, explicit working-directory and whole-repository status readback, compiler/tests, tool evidence, retries, and latency. Review before enabling any worker or fallback.
- **Worker route authorized by the user:** `openai-codex` / `gpt-6.1-sol` executes tightly scoped delegates for important decisions and important coding work at **medium** reasoning (see the Sol delegation lane section above); the user prefers not to overuse it. Routine research collection, tooling, scaffolding, straightforward Rust implementation and other bounded tasks go to the free Nous routes. This is task routing, not a quota-independent fallback — a Sol child still shares the Codex allowance and cannot serve as its own recovery path. The profile's fixed `terminal.cwd` was cleared through `hermes config set terminal.cwd ''`; a Sol CLI probe with `--in <scratch>` then verified `pwd`, file creation, tool readback, and independent readback in scratch without new repository writes.

## Verified fallback chain (2026-10-15)

Configured through the Hermes config API (not by hand-editing YAML); the pre-change file was backed up alongside the profile config.

```
primary:  gpt-6-luna            (via openai-codex)
fallback: 1. poolside/laguna-s-2.1:free            (via nous / Nous Portal)
          2. meituan/longcat-2.5-preview:free      (via nous / Nous Portal)
```

- **Why Nous Portal, not OpenRouter:** OpenRouter is the unstable route here. `poolside/laguna-s-2.1:free` returned HTTP 429 through OpenRouter while the same model served normally through Nous Portal, and the profile's OpenRouter credential carried a recorded `rate_limit` failure. Both Nous entries answered through the profile's Portal OAuth credential with a `hermes chat -q` probe. Prefer Portal for free routes; treat OpenRouter free endpoints as opportunistic only.
- **Evidence — provider-failure recovery:** with the primary genuinely rate-limited (Codex HTTP 429), a `hermes chat -q` probe run under `--in <scratch>` answered `NOUS_FALLBACK_OK` and reported: *"Model fallback: gpt-6-luna via openai-codex unavailable (rate limit); using poolside/laguna-s-2.1:free via nous."* The canonical repository showed no new writes afterwards. This is a real switch, not a catalog claim.
- Both entries are zero-price routes, so no PAYG authorization is consumed. The Portal credential is marked as having insufficient credits for **paid** models; that must not be read as a limit on free models, which serve normally. Free routes remain subject to upstream capacity and to retirement without notice — re-verify before relying on either.
- Pending Kanban cards are pinned to `nous:poolside/laguna-s-2.1:free`; a pin only applies on the next dispatch, so a worker already mid-flight keeps the route it started on.
- This establishes the recovery path required for the M0 gate. It does **not** approve either model as the default coding worker; that selection still requires the five-task isolated Rust benchmark in `docs/research/model-worker-benchmark.md` (`M0-01b`).

## Task-level escalation rule

- Luna default: bounded source inventory, reproducible tests, mechanical/refactor/CRUD changes, and narrow APIs with explicit acceptance criteria. Require correct worktree, actual compiler/tests where applicable, diff review, and whole-repository status; the model's completion claim alone is not evidence.
- Escalate to Sol for novel DJ semantics; difficult traits, ownership/lifetimes/generics, async or Send/Sync interactions; risky migration/data integrity; unsafe/realtime concurrency; ambiguous cross-crate boundaries; or repeated Luna failure after a minimal reproducer. Supply exact diagnostics/interfaces and one precise question, not the whole project history.
- Keep **independent Sol review** separate from implementation for expensive-to-reverse decisions listed above. One small packet and one decision per child; the lead verifies evidence and owns the result. Do not double-process every ordinary patch.
- Use low/medium reasoning on the free routes for routine bounded work, and Sol/**medium** for a consequential decision or a hard escalation; go to Sol/high only with evidence that medium genuinely failed. Compare task-lane pass rate, regressions, retries, isolation failures, latency and allowance use before changing the route. These are routing heuristics, not proof that either model will pass a given task.

Primary references checked 2026-10-15: [Luna model positioning](https://platform.openai.com/docs/models/gpt-6-luna), [Sol model positioning](https://platform.openai.com/docs/models/gpt-6.1-sol), [reasoning effort](https://platform.openai.com/docs/guides/reasoning), [model selection and representative evaluations](https://developers.openai.com/api/docs/guides/model-selection), [Cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [Cargo Clippy](https://doc.rust-lang.org/cargo/commands/cargo-clippy.html). API pricing on those model pages does not by itself establish the user's Codex subscription charges or limits.
