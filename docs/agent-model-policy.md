# Agent Model Policy

## Routing

- The OxiDJ lead currently runs `gpt-6-luna` through `openai-codex`; use it for orchestration, difficult synthesis, integration, and escalation.
- Use Kanban for bounded coding workers. Prefer an explicitly validated free/low-cost worker for small, isolated tasks; escalate to Luna when the task is complex or the worker is insufficient. Do not assume Hindsight's OpenRouter model is validated for coding: its verification covered Hindsight extraction/structured output only.
- Use `delegate_task` as the isolated independent reviewer for consequential or ambiguous decisions, not for routine coding. The verified default route is `openai-codex` / `gpt-6.1-sol` with high reasoning; the installed account-scoped Codex catalog exposed that exact model ID. The lead remains responsible for the final decision.
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
- The current Hermes fallback list is empty. Keep it empty unless an explicitly authorized, cost-safe fallback is verified.
- Hindsight's `nvidia/nemotron-3-super-120b-a12b:free` setting is a cluster-wide memory-extraction configuration, not a coding-worker recommendation; leave it unchanged absent a separate authorization.

## Concurrency

Keep Kanban and delegated work conservative: at most two concurrent items, with independent scopes and worktrees for concurrent code changes. Current profile limits are set to two for both Kanban in-progress work and `delegate_task` children.
