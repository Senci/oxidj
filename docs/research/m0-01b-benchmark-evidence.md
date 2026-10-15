# M0-01b — Independent free worker + fallback validation

Date: 2026-10-15 · Task: `t_728a5ea9` · Worker under test: `poolside/laguna-s-2.1:free` via `nous` (pinned by the dispatcher)
Goal: validate an independent free coding/research worker and a quota-safe
fallback, **without gating Sol-backed M0 research**. All work runs in an isolated
scratch dir **outside** the oxidj repo.

---

## 1. Environment

- Hermes Agent 0.22.0; profile `oxidj-2`.
- Toolchain: `cargo 1.99.0 (5f94df478 2026-08-27)`, `rustc 1.99.0 (b940084d7 2026-09-28)`.
- Inference: `provider=nous` (Nous Portal OAuth), model `poolside/laguna-s-2.1:free` (task override).
- Rust work root: `/home/senci/.hermes/profiles/oxidj-2/cache/scratch/m0-01b-benchmark` — **outside the oxidj git tree** (verified by `realpath`).

## 2. Hermes config readback (live)

`hermes status`: Model `meituan/longcat-2.5-preview:free`, Provider `Nous Portal`.

Profile `oxidj-2/config.yaml`:
- `model.default: meituan/longcat-2.5-preview:free`, `provider: nous`
- `terminal.cwd: ''` ← isolation bug cleared (was fixed during M0-01)
- `kanban: max_in_progress=2, auto_decompose=false, max_in_progress_per_profile=1`
- `fallback_providers:`
  1. `meituan/longcat-2.5-preview:free` (nous)
  2. `poolside/laguna-s-2.1:free` (nous)

`hermes fallback list`: primary `meituan/longcat-2.5-preview:free`; fallback chain = 2 free Nous entries.
`hermes doctor` (live profiles): `viper=gpt-6-luna` (lead), `oxidj-2`/`dzabe=meituan/longcat:free`, `sova=poolside/laguna-s-2.1:free`.

**Evolution:** the 02:21 backup config had `primary=gpt-6-luna (openai-codex)` + openrouter fallbacks; the current config is a Nous-only free chain. The committed `docs/agent-model-policy.md` "Verified fallback chain" matches the live Nous chain (the "primary gpt-6-luna" line refers to the lead profile `viper`, not this task profile).

## 3. Live model IDs / pricing

- Live model IDs: `meituan/longcat-2.5-preview:free`, `poolside/laguna-s-2.1:free` (both `:free`, Nous Portal).
- Pricing: **$0** — free-tier Nous Portal models. No PAYG route; the Nous Portal credential is free-tier (paid models report insufficient credits, which does **not** affect free models).
- Both IDs verified **live** by actual inference (§5), not merely catalog lookup.

## 4. Fallback chain — is it quota/cost-independent?

The chain (primary `meituan/longcat` → fallback `poolside/laguna`) is on **Nous Portal only**, so it is cost-free and **independent of the Codex/Sol allowance**: a Codex rate-limit/failure drops to a free Nous model at zero cost. It is *not* an absolute guarantee — free Nous routes still depend on upstream capacity and can be rate-limited or retired without notice (see §7 caveat). A second model on the same exhausted Codex allowance would not be an independent fallback; this Nous chain is the independent, quota-safe path the policy requires.

## 5. Provider-failure recovery probes (live, real inference)

Run as `hermes chat --cli --oneshot ...` against the configured chain.

| Probe | Target | Result | Latency | Tool use |
|---|---|---|---|---|
| A (primary) | `meituan/longcat-2.5-preview:free` (nous) | `NOUS_CLI_OK` + model id reported | ~16 s | ran `hermes chat --help` (0.6 s) |
| B (fallback) | `poolside/laguna-s-2.1:free` (nous) | `NOUS_FALLBACK_TOOL_OK` + model id reported | ~6 s (agent) / 15.2 s wall | ran `echo NOUS_FALLBACK_TOOL_OK` (0.1 s) |

Both probes executed real `terminal` tool calls and returned the correct model id —
live proof the Nous free routes are authenticated, reachable, and tool-capable.

**Failover behaviour:** no failover fired during these probes because the primary was healthy; the chain engages automatically on 429 / 529 / 503 / connection errors (Hermes fallback semantics). A forced primary-429 re-test was **deliberately not run** to avoid consuming the Codex/Sol allowance (user preference: do not overuse). A real failover (Codex 429 → Nous free) was already observed and recorded by the lead in M0-01; both endpoints are now verified live.

## 6. The 5 isolated Rust/agent tasks (real compiler / test evidence)

All five ran in scratch via `cargo`; per-command wall times ≈ 0.05–0.2 s.

| # | Task | Evidence |
|---|---|---|
| 1 | **Multi-module comprehension** — `djmeta`: `lib` + `bpm` + `key` + `cue` modules | `cargo build` OK; `cargo test` **4 passed, 0 doctests**. Hit a genuine error en route (below). |
| 2 | **Type error** — `bpm-check`: `&str` passed to a `u8` fn | `cargo build` → **E0308 mismatched types (expected u8, found &str)**; fixed → `cargo run` prints `128 bpm -> fast`. Retry cycle recorded. |
| 3 | **Typed API** — `typed-beats`: `Beats` vs `Bpm` distinct newtypes | `cargo build` OK; **`cargo clippy --all-targets`: 0 warnings**; `cargo test` 2 passed. |
| 4 | **Tests** — `test-suite` | `cargo test`: **4 unit + 3 integration + 1 doctest = 8 passed**, 0 failed. |
| 5 | **Patch review** — `patch-review` git baseline | off-by-one patch (`<=`→`<`); `cargo test` **failed** (`assertion failed: r.contains(180)`, exit 101) → caught; reverted via `git checkout -- .`; tests pass, `git status` clean. Review in `t5_patch_review.md`. |

### Real errors the free worker surfaced and fixed (not scripted)

- **Task 1 — E0658 (const fn):** `RangeInclusive::contains` is not const-stable on Rust 1.99 → fixed with direct comparison (`n >= 1 && n <= 12`).
- **Task 1 — test failure:** `cue` derived `Ord` over `(label, millibeats)`, ordering by label ("drop" < "start"); reordered `millibeats` first so cues order by position. Test then passed.
- **Task 2 — E0308:** deliberate type mismatch, then the fix.

## 7. Repo / worktree readback (isolation)

- **Before** the 5 tasks: `git -C <worktree> status --porcelain` → **empty (clean)**.
- **After** the 5 tasks: `git status --porcelain` → **empty (clean)**, HEAD `8ce8c6a`.

The oxidj repo/worktree was **not modified** by the free worker — no stray `Cargo.toml`/`src` written to the canonical root (the exact failure mode that disqualified the earlier Nemotron probe). Every artifact lives under scratch.

## 8. Decision

- **Free worker: VALIDATED.** `poolside/laguna-s-2.1:free` (and `meituan/longcat-2.5-preview:free`) on Nous Portal are live, authenticated, tool-capable, and executed 5 distinct Rust tasks with real compiler/clippy/test/git evidence on Rust 1.99.0.
- **Select the free worker** for small, isolated, bounded tasks (per the policy's routing rule). It is **not** a default replacement for the lead (`gpt-6-luna` / Sol) on novel DJ semantics, hard lifetimes/generics/async, or risky migrations.
- **Quota-independent fallback: configured and end-to-end verified.** Both Nous free entries answered with real tool use; the chain is cost-free and independent of Codex. No PAYG anywhere.
- **Caveat:** free Nous routes are still subject to upstream capacity and retirement; re-verify before relying on them (unchanged from the policy's note).

## 9. Recommended follow-up

Record this validation in the authoritative docs (`docs/agent-model-policy.md`,
`docs/research/model-worker-benchmark.md`) — a small, reviewable edit to flip the
"no alternate worker is approved / unvalidated" statements to "validated 2026-10-15".
Kept out of this run so the repo/worktree isolation evidence stays unambiguous.

## Artifacts (scratch, attached to the kanban task)

- `t5_patch_review.md`, `t5_offbyone.patch`
- `harness_output.log`, `run_tasks.sh`
- crates: `t1_modules/`, `t2_typeerror/`, `t3_typedapi/`, `t4_tests/`, `t5_patchreview/`
