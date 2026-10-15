# Coding worker route validation

Date: 2026-10-15

## Outcome

No alternate coding or research worker is validated or selected. Keep product work on the existing lead route and stop safely on errors. The requested five-task Rust evaluation is incomplete.

## Environment and availability

- Hermes Agent `0.22.0 (2026.9.24)`; stable installation commit `e06df9c2eae6` (worker report).
- Rust stable was installed user-locally during the worker run. Lead independently verified `$HOME/.cargo/bin/cargo --version` returned `cargo 1.99.0 (5f94df478 2026-08-27)`; Rust was not yet on this session's `PATH`.
- Worker reported Portal OAuth logged in and inference endpoint `https://inference-api.nousresearch.com/v1`; this does not establish a tested Nous coding route.
- Worker reported live OpenRouter catalog HTTP 200 and free/tool-support labels for `nvidia/nemotron-3.5-lightning:free` and `google/gemma-4-31b-it:free`. Catalog labels do not prove service reliability or tool execution.
- No local inference runtime was found on the worker's PATH; this is not an exhaustive local-service audit.

## Bounded execution probes and independent readback

| Candidate | Worker-reported result | Lead readback and limits |
|---|---|---|
| `nvidia/nemotron-3.5-lightning:free` | Asked to create a small `oxidj-bpm` Rust crate and tests. First call exited 1 after 30.53 seconds; second claimed 12 tests passed, but worker's intended isolated benchmark directory was empty. | **The crate was actually written to the canonical repository root**, not to the intended isolated directory: `Cargo.toml`, `Cargo.lock`, and `src/lib.rs` appeared on `master`, with `.gitignore` modified by Cargo. Lead ran `$HOME/.cargo/bin/cargo test --manifest-path Cargo.toml` in that root: 12 unit tests and zero doctests passed. This is a workspace-isolation/tool-cwd failure, not proof of a safe worker. The worker's no-files conclusion was only true for its probed benchmark directory. |
| `google/gemma-4-31b-it:free` | Simple `pwd`/write/readback tool probe received HTTP 429 on three attempts, with 2.1 and 4.7 second backoffs; about 9 seconds end-to-end. | No useful tool execution or Rust coding benchmark was produced. |

The lead preserved the stray crate as benchmark evidence outside the product checkout and removed it from the repository. No product source was accepted. The first probe shows that verifying only the expected work directory can miss writes elsewhere; future trials must check the entire repository/worktree status and explicit working directory before and after each run.

## Working-directory root cause and Sol route check

The active Hermes profile had `terminal.cwd = /home/senci/dev/oxidj`. Separate `hermes chat --in <isolated scratch dir>` calls on `gpt-6.1-sol` repeatedly returned `pwd = /home/senci/dev/oxidj`, even when launched from scratch and with `TERMINAL_CWD` set; `--ignore-user-config` was not sufficient in those probes. The profile's configured terminal working directory overrode the intended isolation. Using the supported `hermes config set terminal.cwd ''` cleared the profile pin. A new `hermes chat --in <scratch dir> -m gpt-6.1-sol --provider openai-codex` probe then returned the correct scratch cwd. A second probe wrote `route-proof.txt` there, read back `SOL_ROUTE_OK`, and independent shell readback found the same content. `git status` of the product repository had no new benchmark changes. This validates a bounded tool-capable Sol route and working-directory isolation for those probes, not a free-worker route or independent quota fallback.

## Route and recovery

- Existing lead: `openai-codex` / `gpt-6-luna`. Existing isolated decision reviewer: `openai-codex` / `gpt-6.1-sol`, high reasoning. The reviewer shares the Codex allowance and is not a quota fallback.
- `hermes fallback list` returned no configured fallback providers. The Gemma HTTP 429 probe stopped after three retries without a route switch. This establishes fail-safe stop on this probe, not failover success.
- Do not enable PAYG, approve a coding worker, or infer uptime from a `:free` catalog label. If the lead route becomes unavailable, stop and preserve state.

## Required next validation — COMPLETED 2026-10-15 (`M0-01b`)

The five-task validation specified above was run and passed. Raw evidence is preserved at `docs/research/m0-01b-benchmark-evidence.md`, the patch-review record at `docs/research/m0-01b-t5-patch-review.md`, and the harness at `scripts/m0-01b-benchmark.sh`.

**Worker under test:** `poolside/laguna-s-2.1:free` via `nous` (Nous Portal), cargo/rustc 1.99.0. All work ran in `/home/senci/.hermes/profiles/oxidj-2/cache/scratch/m0-01b-benchmark`, verified outside the git tree.

| Task | Worker result | Lead verification |
|---|---|---|
| 1 — multi-module comprehension | 4 tests passed | claimed, not re-run |
| 2 — compiler/type error (`E0308`, `E0658`) | diagnostics surfaced and fixed | claimed, not re-run |
| 3 — typed API from acceptance criteria | 2 passed, clippy 0 warnings | **independently re-run: 2 passed; `cargo clippy --all-targets -- -D warnings` clean** |
| 4 — write/fix tests | 4 unit + 3 integration + 1 doctest = 8 passed | **independently re-run: identical counts, 8 passed** |
| 5 — patch review | buggy patch failed as expected; 1 passed after revert | claimed, record preserved |

**Isolation:** `git status` of the canonical checkout was clean before and after, and HEAD remained `8ce8c6a` with no stray writes. The failure mode that disqualified the earlier Nemotron probe — a healthy-looking run writing outside its workspace — did not recur.

**Provider probes:** primary `meituan/longcat-2.5-preview:free` returned `NOUS_CLI_OK` (~16 s) and fallback `poolside/laguna-s-2.1:free` returned `NOUS_FALLBACK_TOOL_OK` (~6 s agent), each executing a real `terminal` tool call. A forced-429 failover test was deliberately skipped to conserve allowance; the genuine Codex-429 → Nous switch was already observed and recorded under `M0-01`.

**Verdict:** `poolside/laguna-s-2.1:free` (Nous) is validated for **small bounded tasks**. This approves a worker, not an architecture: it does not approve the model for hard Rust reasoning, and the Nemotron isolation failure stands as a standing warning that a free route can look healthy and still be unsafe to trust.

**Caveats that remain:** free Nous routes depend on upstream capacity and can be rate-limited or retired without notice — `laguna-s-2.1:free` was announced as expiring 2026-10-31. A `:free` label still proves nothing about uptime or tool reliability, and no failover was forced in this round.
