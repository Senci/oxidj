# OxiDJ Agent Rules

These rules apply to every agent working in this repository. They are deliberately
short: detail lives in the linked policy documents, and the repository — not an
agent's memory — owns accepted decisions and live status.

## Before you write

- Read `idea.md` and the relevant plan before making project changes.
- Repository decisions are canonical. Do not silently override an accepted
  decision, and do not discard metadata you do not understand.
- Research uncertain or expensive architectural choices before committing to
  them; a choice that is expensive to reverse needs a scoped review first.
- **Confirm `pwd` before writing, and never write outside your assigned
  workspace.** When dispatched into a worktree, write only inside it. A fixed
  `terminal.cwd` can override the worktree and send files to the canonical
  checkout, so check where your writes actually landed rather than where you
  intended them. See [worktree isolation](docs/worktree-isolation.md).
- Keep task scopes independent. Do not run parallel cards that touch the same
  shared files (`Cargo.toml`, `AGENTS.md`, `docs/research/`); flag the collision
  on the card with a `hotspot:` comment instead.

## Evidence, not claims

- **A completion claim is not evidence.** Paste the real exit codes and summary
  lines from the commands you ran. "Tests pass" is a claim; a gate summary line
  is evidence.
- Every coding handoff runs the repo-owned gate — `cargo xtask ci` locally, and
  CI runs `cargo xtask ci-full`. A task that has not run the gate is not
  complete, however finished the code looks. See
  [handoff checklist](docs/handoff-checklist.md).
- **Gate evidence goes in the commit body.** Every commit includes the gate
  summary line (exit code + result) in its message — not just in the Kanban
  card. A commit without a gate line is not a complete handoff. When an agent
  produces the change, add `Co-authored-by: @<agent-name>` as a trailer.
- **Gate evidence is void after the last edit.** If any code changes after the
  gate ran, re-run it.
- Never replace a failed run with plausible-looking output. If a result cannot
  be produced, say so; a reported blocker is always better than an invented
  result.

## Bound your loop

- After about three unsuccessful attempts at the same problem, stop and report
  the commands, errors and attempted fixes instead of retrying.
- For long tasks, maintain a todo list of remaining steps; do not rely on
  conversation history for state.
- Write durable state at sub-goal boundaries — into `plan/STATUS.md` and the
  Kanban card — rather than only into the chat.

## Knowledge and tooling

- Use Hindsight for concise operational memory and `/llm-wiki` for substantive,
  sourced external research; use Kanban for active work.
- Use `uv` for Python environments and packages, and Bun for TypeScript
  dependencies and scripts.
- Never commit credentials or secrets.

## Policy

- [Knowledge policy](docs/agent-knowledge-policy.md) — what counts as evidence,
  and how external knowledge is promoted into the repository.
- [Model policy](docs/agent-model-policy.md) — model routing, the scoped
  decision-review lane, the verified fallback chain, and GPT-6 API efficiency
  guidelines (model selection, reasoning-effort ladder, caching, compaction,
  async tools, cost/latency controls).
- [Agentic workflow](docs/agentic-workflow.md) — the full workflow these rules
  summarise, including the gate tiers and card design.
- [Layers policy](docs/layers-policy.md) — crate boundaries and the
  `cargo xtask layers` guard (ADR-004).
