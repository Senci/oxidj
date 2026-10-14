# Agent Model Policy

## Routing

- The OxiDJ lead currently runs `gpt-6-luna` through `openai-codex`; use it for orchestration, difficult synthesis, integration, and escalation.
- Use Kanban for bounded coding workers. Prefer an explicitly validated free/low-cost worker for small, isolated tasks; escalate to Luna when the task is complex or the worker is insufficient. Do not assume Hindsight's OpenRouter model is validated for coding: its verification covered Hindsight extraction/structured output only.
- Use `delegate_task` for a tiny isolated decision review. The requested `Sol/high` route must be verified against the installed model/provider before use; do not invent an alias or infer availability from a credential alone.
- Use Astra only if its exact model/provider is available, suitable, and justified for the task.

## Cost and quota controls

- Check provider, model, quota, and fallback behavior before dispatch. No surprise pay-as-you-go usage: do not select paid API-key routes or add paid fallbacks without explicit authorization.
- Prefer validated free tiers for bounded work, keep prompts and worker scope small, and escalate rather than silently switching to a paid model.
- The current Hermes fallback list is empty. Keep it empty unless an explicitly authorized, cost-safe fallback is verified.
- Hindsight's `nvidia/nemotron-3-super-120b-a12b:free` setting is a cluster-wide memory-extraction configuration, not a coding-worker recommendation; leave it unchanged absent a separate authorization.

## Concurrency

Keep Kanban and delegated work conservative: at most two concurrent items, with independent scopes and worktrees for concurrent code changes. Current profile limits are set to two for both Kanban in-progress work and `delegate_task` children.
