# OxiDJ Agent Rules

- Read `idea.md` and the relevant plan before making project changes.
- Repository decisions are canonical; do not silently override accepted decisions or discard metadata.
- Research uncertain or expensive architectural choices before committing to them.
- Use Hindsight for concise operational memory and `/llm-wiki` for substantive, sourced external research; use Kanban for active work.
- Use isolated Git worktrees for concurrent coding agents and keep task scopes independent.
- Use `uv` for Python environments/packages and Bun for TypeScript dependencies/scripts.
- Never commit credentials or secrets. Verify behavior and tests before marking work done.
- Follow [the knowledge policy](docs/agent-knowledge-policy.md) and [the model policy](docs/agent-model-policy.md).
