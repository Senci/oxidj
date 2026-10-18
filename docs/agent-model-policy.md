# Agent Model Policy

## Routing

- `oxidj-2` is the default runner profile for OxiDJ work. Route ordinary implementation, integration, and review tasks through this profile. Use a separate profile only for genuine, tightly scoped research or summarization.
- GPT-6 Luna (`gpt-6-luna`) via `openai-codex` is the default execution model. Use GPT-6.1 Sol (`gpt-6.1-sol`) at **medium** for important/complex work and independent sanity checks. Use **high** only rarely, after medium proves insufficient.
- Do not use free models for OxiDJ implementation or independent review by default. If Luna/Sol is unavailable or quota is insufficient, stop and surface the blocker; do not silently switch routes or enable PAYG.
- Per-card model/provider/profile overrides take precedence over profile defaults. When creating or updating a card, explicitly set and read back its assignee, model, and provider. A later pin does not change a task that is already running.
- Keep the lead responsible for the final decision and require real compiler/test/gate evidence for coding handoffs. Historical worker-validation and fallback records below document past evidence/configuration; they do not authorize a current default or override the routing rules above.

## Bounded autonomy and owner questions

- **Default to a safe technical choice and progress.** For bounded, reversible details within accepted requirements, the lead chooses the best-supported option, records material assumptions and alternatives, and proceeds. Do not ask the owner to supply internal implementation details, sampling formulas, or other technical parameters the lead can research and recommend.
- **Respect the owner boundary.** Ask the owner for product intent or preference, changes to an accepted requirement, spending, destructive/external actions, security or safety decisions, or a decision explicitly reserved to the owner. For consequential technical questions, use the scoped Sol lane when required; Sol advises and the lead closes the decision unless the decision is owner-only.
- **Localize stops.** If one acceptance or qualification gate is unresolved, stop only work that depends on that gate. Continue safe, independent work and label provisional/non-qualifying output honestly. Never bypass a required review, repository gate, benchmark authorization, or M1 qualification gate.
- **Question contract.** When owner input is genuinely necessary, ask one plain-language question, state the lead's recommended default and its consequence, and offer at most two realistic alternatives. Do not ask the owner to design a technical solution from scratch.

## Decision closure and review count

- Use one packet for one consequential decision. The packet requests one recommendation, short reasons, the strongest counterargument, confidence, and evidence that would change the view.
- The lead verifies load-bearing evidence and records the decision. Do not create a second Sol “approval” card just to approve the same Sol advice; a second review is justified only by new decisive evidence, a changed decision question, or an explicit independent gate.
- For one bounded deliverable, keep source mapping, correction, and candidate preparation in one implementation card when they share an owner and acceptance contract. Use one independent final review when required; allow one focused correction and re-review if needed. Split cards only for a real ownership, dependency, deliverable, or acceptance boundary.
- Do not double-review routine patches. These efficiency rules never waive review required by an accepted ADR, safety/security policy, or M1 gate.

## Sol delegation lane — scope and effort (user authorization, 2026-10-15)

The user authorized using **tightly scoped Sol delegates at medium reasoning** for **important decisions and important coding work**, with **high used rarely**. This is a deliberate widening of the earlier decision-review-only rule, and it comes with a deliberate ceiling:

- **Default effort is `medium`.** `delegation.reasoning_effort = medium`, `delegation.max_iterations = 6`. Raise to `high` only for a named, specific difficulty that medium actually failed on — never as a first move and never as a substitute for a clearer task.
- **Scope stays tight.** One decision or one coding objective per child, a small packet, explicit exclusions, and the evidence the child needs — not the project history. A delegate that has to ask what the project is was mis-scoped.
- **Sol is for important/complex work, not the ordinary default.** Luna remains the default; historical free-route validation below is evidence only, not authorization to route OxiDJ implementation or independent review to free models.
- **Importance is the admission test.** "Important" means: expensive to reverse, affects data integrity or interoperability, sets a crate/API boundary others will build on, or is difficult Rust reasoning (ownership/lifetimes/generics, async/Send/Sync, unsafe/realtime). A task that is merely tedious does not qualify.
- **Important coding work follows the same discipline as review work**: give the child the acceptance criteria and require real compiler/test/gate evidence back, not a completion claim.
- Historical note: the earlier "free by default, Sol/medium for important work" rule is superseded by the current Routing rules above. PAYG routes still require explicit approval and must not be enabled under the current instruction.
- Astra is not an authorized OxiDJ route under the current Luna/Sol-only policy. Any earlier Astra references are superseded historical context; do not dispatch to Astra unless the owner explicitly changes the policy.

## Dedicated decision delegation

- Keep each decision packet to about 300–800 words, preferably shorter, with only: **DECISION, WHY NOW, CONSTRAINTS, OPTIONS, EVIDENCE, CURRENT RECOMMENDATION (optional), OUTPUT CONTRACT**.
- Ask for exactly: recommendation; 3–5 short reasons; strongest counterargument; confidence; and what would change the decision (if relevant). No essay, implementation, backlog, or redesign.
- Give the reviewer no tools by default. Grant read-only file access or web access only when the decision genuinely requires that evidence; never grant write/terminal access merely because it is available.
- The reviewer must not re-delegate, implement, expand scope, modify Kanban, or write Hindsight. Use one consequential decision per child. Configure `delegation.max_iterations = 6`; raise it only when a required verification step needs more.
- If Sol strongly disagrees, verify the disputed evidence. If disagreement remains, the lead chooses and documents a provisional default when that is within accepted requirements; escalate only when the decision is owner-reserved or changes product intent/authorization. Do not treat disagreement alone as a reason to make the owner arbitrate technical advice.

## Cost and quota controls

- Check provider, model, quota, and fallback behavior before dispatch. No surprise pay-as-you-go usage: do not select paid API-key routes or add paid fallbacks without explicit authorization.
- Keep prompts and worker scope small. Do not switch to a free or paid route when Luna/Sol is unavailable; surface the blocker. PAYG still requires explicit approval.
- A historical profile fallback-chain configuration is documented below for audit only; it is not an authorized OxiDJ task fallback. Do not alter fallback configuration under this policy. Add a paid entry only with explicit user authorization.
- Hindsight's `nvidia/nemotron-3-super-120b-a12b:free` setting is a cluster-wide memory-extraction configuration, not a coding-worker recommendation; leave it unchanged absent a separate authorization.

## Concurrency

Keep Kanban and delegated work conservative: at most two concurrent items, with independent scopes and worktrees for concurrent code changes. Current profile limits are set to two for both Kanban in-progress work and `delegate_task` children.

## Historical worker validation and recovery evidence (2026-10-15)

The records in this section document earlier experiments and configuration only. They are not current dispatch instructions; follow the Routing and Cost and quota controls sections above.

- **Approved worker (2026-10-15, `M0-01b`):** `poolside/laguna-s-2.1:free` via **Nous Portal** is validated for **small bounded tasks** — five isolated Rust tasks with real compiler/test evidence, and the lead independently reproduced the typed-API (2 passed, clippy clean under `-D warnings`) and test-suite (8 passed) results. Raw evidence: `docs/research/m0-01b-benchmark-evidence.md`; harness: `scripts/m0-01b-benchmark.sh`. This approves a worker for bounded work only — it does **not** approve the model for hard Rust reasoning, and `laguna-s-2.1:free` was announced as expiring 2026-10-31, so re-validate after that date.
- Earlier candidates were **not** approved. The live OpenRouter catalog listed `nvidia/nemotron-3.5-lightning:free` and `google/gemma-4-31b-it:free` with tool support and zero prices, but catalog lookup is not a coding-worker test. See `docs/research/model-worker-benchmark.md`.
- A Nemotron probe created a compilable Rust test crate in the **canonical repository root**, not the intended isolated benchmark directory. Twelve unit tests passed on independent readback, but the unexpected working-directory write disqualifies this as an isolated agentic worker result. The Gemma probe encountered HTTP 429 on all three retries before tool execution.
- Nous Portal authentication was reported, but no coding task on its inference route was verified; no local inference route was validated. Do not treat either as fallback merely because credentials or binaries exist.
- An empty fallback chain was correct while no independent route was verified: an earlier HTTP 429 probe stopped after retries without provider switching. A second model on the same exhausted Codex allowance is not an independent fallback; no PAYG route may be enabled without explicit user approval.
- Re-evaluate pinned candidates only when service recovers: five isolated Rust tasks, explicit working-directory and whole-repository status readback, compiler/tests, tool evidence, retries, and latency. Review before enabling any worker or fallback.
- Historical authorization and probe record: `openai-codex` / `gpt-6.1-sol` was used for tightly scoped important work at **medium** reasoning. The earlier routine-free-route direction in this record is superseded by Routing above. Sol shares Codex allowance and is not a quota-independent fallback. A prior scratch probe verified worktree isolation.

## Historical fallback-chain verification (2026-10-15)

This configuration was verified at the time; it is not an authorized OxiDJ task route under the current Routing rules.

Configured through the Hermes config API (not by hand-editing YAML); the pre-change file was backed up alongside the profile config.

```
primary:  gpt-6-luna            (via openai-codex)
fallback: 1. poolside/laguna-s-2.1:free            (via nous / Nous Portal)
          2. meituan/longcat-2.5-preview:free      (via nous / Nous Portal)
```

- **Why Nous Portal, not OpenRouter:** OpenRouter is the unstable route here. `poolside/laguna-s-2.1:free` returned HTTP 429 through OpenRouter while the same model served normally through Nous Portal, and the profile's OpenRouter credential carried a recorded `rate_limit` failure. Both Nous entries answered through the profile's Portal OAuth credential with a `hermes chat -q` probe. Prefer Portal for free routes; treat OpenRouter free endpoints as opportunistic only.
- **Evidence — provider-failure recovery:** with the primary genuinely rate-limited (Codex HTTP 429), a `hermes chat -q` probe run under `--in <scratch>` answered `NOUS_FALLBACK_OK` and reported: *"Model fallback: gpt-6-luna via openai-codex unavailable (rate limit); using poolside/laguna-s-2.1:free via nous."* The canonical repository showed no new writes afterwards. This is a real switch, not a catalog claim.
- Both entries are zero-price routes, so no PAYG authorization is consumed. The Portal credential is marked as having insufficient credits for **paid** models; that must not be read as a limit on free models, which serve normally. Free routes remain subject to upstream capacity and to retirement without notice — re-verify before relying on either.
- Historical Kanban cards were pinned to `nous:poolside/laguna-s-2.1:free`; a pin only applies on the next dispatch, so a worker already mid-flight keeps the route it started on. This does not authorize that route for current OxiDJ work.
- This established recovery behavior for the historical M0 gate, not a current OxiDJ recovery route. It did **not** approve either model as the default coding worker; that selection required the five-task isolated Rust benchmark in `docs/research/model-worker-benchmark.md` (`M0-01b`).

## Task-level escalation rule

- Luna default: bounded source inventory, reproducible tests, mechanical/refactor/CRUD changes, and narrow APIs with explicit acceptance criteria. Require correct worktree, actual compiler/tests where applicable, diff review, and whole-repository status; the model's completion claim alone is not evidence.
- Escalate to Sol for novel DJ semantics; difficult traits, ownership/lifetimes/generics, async or Send/Sync interactions; risky migration/data integrity; unsafe/realtime concurrency; ambiguous cross-crate boundaries; or repeated Luna failure after a minimal reproducer. Supply exact diagnostics/interfaces and one precise question, not the whole project history.
- Require a **separate, independent Sol review** for expensive-to-reverse decisions identified above. This is a mandatory gate, not a discretionary check based on whether the implementer thinks it adds value; keep the review separate from implementation and use one small packet and one decision per child. For routine work outside this consequential-decision class, independent sanity checks are optional. The lead verifies evidence and owns the result; do not double-process ordinary patches.
- Do not route routine bounded work to free models. Use Luna by default and Sol/**medium** for important/complex work or independent sanity checks; go to Sol/high only rarely, after evidence that medium proved insufficient. Compare pass rate, regressions, retries, isolation failures, latency and allowance use before proposing a route change. These are heuristics, not proof that either model will pass a given task.

Primary references checked 2026-10-15: [Luna model positioning](https://platform.openai.com/docs/models/gpt-6-luna), [Sol model positioning](https://platform.openai.com/docs/models/gpt-6.1-sol), [reasoning effort](https://platform.openai.com/docs/guides/reasoning), [model selection and representative evaluations](https://developers.openai.com/api/docs/guides/model-selection), [Cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [Cargo Clippy](https://doc.rust-lang.org/cargo/commands/cargo-clippy.html). API pricing on those model page does not by itself establish the user's Codex subscription charges or limits.

## Efficiency guidelines

Full GPT-6 API efficiency guidance (model selection, reasoning-effort ladder, prompt
caching, compaction, async tools, mid-turn steering, cost/latency controls) is maintained
as a shared skill: `openai-api-efficiency`. Load it on every prompt when using OpenAI
models via `skill_view(name='openai-api-efficiency')`, or read the canonical copy at
`~/.hermes/skills/openai-api-efficiency/SKILL.md`.

**Quick reference for this repo's routes:**
- **Default:** GPT-6 Luna at medium reasoning — for bounded/repeated work
- **Escalation:** GPT-6.1 Sol at medium — for important/complex decisions
- **Last resort:** Sol at high — only when medium demonstrably failed
- **Never:** Astra without explicit approval (costs $10/$50 per 1M tokens)

Apply the official OpenAI checklist before changing routes or effort:
1. Measure task success, latency, and cost per successful task
2. Compare on representative evals — don't assume higher effort is always better
3. Use prompt caching for stable prefixes (up to 95% cheaper input tokens)
4. Compact long conversations to control context size (expect lower cache reuse after)
5. Parallelize independent tools; serialize only on dependencies
6. Use Fast/Ultrafast mode when latency matters and the premium justifies it
