# Agent Knowledge Policy

## Responsibilities

- **Repository:** canonical product intent, architecture, accepted decisions, ADRs, engineering rules, plans, code, and tests.
- **Hindsight:** operational continuity—prior attempts, failures, debugging discoveries, environment facts, workflow lessons, and transient implementation context. Do not use it as canonical documentation or a task tracker, and do not copy whole wiki pages into it.
- **`/llm-wiki`:** sourced external research, provenance, synthesis, comparisons, and contradictions. Do not use it as a coding-transcript archive or enable overlapping session-memory capture.
- **Kanban:** active tasks, dependencies, owners, acceptance criteria, status, and review state.

## Precedence and promotion

Resolve conflicts in this order:

1. Current explicit repository decision
2. Verified current primary evidence
3. `/llm-wiki` synthesis
4. Hindsight memory

Promote external knowledge as `source → /llm-wiki → verified finding → repository ADR/docs when accepted`. Promote experience as `coding/session experience → Hindsight → verified durable lesson → repository docs/ADR when appropriate`.

## Hindsight bank strategy

The Hermes Hindsight provider uses the supported `bank_id_template = "{project}"`. The repository-name placeholder resolves to `oxidj` for the main checkout and its linked worktrees, while other Git repositories resolve to their own repository-name banks. The existing `oxidj` bank remains the canonical OxiDJ bank; do not duplicate its memories. The static `oxidj` bank ID remains only as the fallback outside a project repository.
