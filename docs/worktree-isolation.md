# Worktree and build-directory isolation

M0-D6, `sova`, `nous:poolside/laguna-s-2.1:free`. Licence: MIT (ADR-009).
Status: **committed rule** (research item 4, [FACT]).

This is the rule for agent worktrees in OxiDJ. It exists because multiple Cargo
instances that share a build cache corrupt each other's artifacts — the failure
mode is not theoretical and is amplified in agent-driven development
([CARGO-16804]).

> **Rule.** Each agent works in a worktree of its own (`git worktree add`), on a
> branch of its own, from its first edit. That worktree keeps its own `target/`
> directory. No two worktrees, toolchains, or profiles may share a `target/`,
> and no agent may write to the canonical checkout. ([STEELBRAIN], [CARGO-16804])

The canonical checkout belongs to the owner/lead. Agents do not edit it. Finished
work is merged on an integration worktree and fast-forwarded by the owner once
the merge's checks pass ([STEELBRAIN]).

---

## Why the default is already safe

- Cargo places `target/` in the directory tree that contains the manifest being
  built (by default, the worktree root). Running `cargo` from inside a worktree
  therefore produces a `target/` scoped to that worktree.
- With the OxiDJ Kanban model — one worktree per agent, `max_in_progress: 2` — at
  most two Cargo instances run at once, each in its own worktree with its own
  `target/`. The per-worktree default is already isolated ([CARGO-16804],
  research item 5).
- Caches must **not** live under `/tmp` (ephemeral/cleaned by the OS). Keep them
  under the worktree or under `~/.cache` ([STEELBRAIN]).

Because the default is already isolated, this rule is documentation: it makes
the invariant explicit and warns against the one thing that breaks it.

---

## What breaks the rule (do not do these)

- Setting `CARGO_TARGET_DIR` to a path shared with another worktree, toolchain
  profile, or the canonical checkout. This causes `.rmeta` / `.rlib` corruption
  that produces spurious compile failures ([CARGO-16804]).
- Pointing a build at the host's shared `~/.cargo` build cache while another
  worktree is building on the same toolchain.
- Editing files in the canonical checkout directly. Record `pwd`,
  `git rev-parse --show-toplevel`, and `git status --short` before and after, in
  both the worktree and the canonical tree, and compare them.

## On the large-workspace scaling note

The Cargo concurrency issue ([CARGO-16804]) reports a multiplier of
`N worktrees × M toolchains × K profiles × (2–10 GB per instance)` — e.g. ~48–240 GB
for 6 worktrees × 2 toolchains × 2 profiles. That is a capacity concern for many
concurrent agents, not a correctness one. The correctness fix is always per-
worktree `CARGO_TARGET_DIR`; the capacity fix is a scheduling/policy decision for
later (cap `max_in_progress`, prune stale worktree targets, consider a content-
addressed cache once a real compile-time bottleneck is measured — see
`docs/research/agentic-rust-kanban.md` §1.8 / research item 3).

---

## Worker checklist (before first edit, after last edit)

```sh
pwd                                              # must be inside $HERMES_KANBAN_WORKSPACE
git rev-parse --show-toplevel
git branch --show-current
git status --short
git -C "$(git rev-parse --show-toplevel)" status --short   # canonical checkout, read-only
echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"       # empty = per-worktree default (safe)
```

If `CARGO_TARGET_DIR` is set and shared outside this worktree, unset it or point
it at a path inside this worktree before building.

---

## Sources

- [CARGO-16804] rust-lang/cargo#16804 — No safe concurrent build cache (agent-driven
  development impact). https://github.com/rust-lang/cargo/issues/16804
- [STEELBRAIN] steelbrain/reims-vgpu — AGENTS.md (worktree + CARGO_TARGET_DIR
  patterns). https://github.com/steelbrain/reims-vgpu/blob/master/AGENTS.md
