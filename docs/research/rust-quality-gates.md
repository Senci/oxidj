# Rust static-analysis & quality-gate best practices for agentic coding

Status: research complete. Owner: `sova` (nous:`poolside/laguna-s-2.1:free`).
Target toolchain: **stable Rust/Cargo 1.99.0** only (`b940084d7 2026-09-28`). No nightly toolchain is installed in the OxiDJ environment.

This document answers M0-10. It is **research only**: no tool was installed or executed by this task; all facts are drawn from primary sources (Rust project documentation, the Cargo Book, the rust-analyzer book, the Clippy book, and each tool's own README/docs). Claims are labelled as *fact*, *documented capability*, or *opinion* where a source is a blog/vendor page.

---

## 0. Key definitions and distinctions (read these before the table)

These resolve the most commonly conflated concepts in this space. Each distinction changes whether a tool belongs in the stable handoff gate.

### 0.1 `clippy::cargo` (stable) vs the `cargo::` lints (nightly-only)

These are **different things** and sources frequently confuse them.

- `clippy::cargo` is a **Clippy lint group** that checks the `Cargo.toml` manifest (feature flags, dependency versions, etc.). It ships with Clippy and runs on **stable** (`cargo clippy`). Source: the Clippy lint-category table [CL-INDEX][CL-INDEX].
- The `cargo::` lints are lints **emitted by Cargo itself** (e.g. `cargo::non_standard_crate_names`). The Cargo Book's "Lints" chapter states these are **unstable and nightly-only**: "Cargo's linting system is unstable and can only be used on nightly toolchains." [C-LINTS][C-LINTS].

A Rust 1.100.0 release-notes note mentions *future* stabilization of Cargo's own linter, but on the installed **1.99.0** stable toolchain the `cargo::` lints remain gated behind nightly. **Decision for OxiDJ**: `clippy::cargo` is usable in the stable gate; `cargo::` lints are **excluded** from the stable gate (nightly-only). This conflict between "stabilization coming in 1.100" (opinion/roadmap) and "nightly-only on 1.99" (fact, on the installed toolchain) is recorded explicitly rather than smoothed.

### 0.2 The `[lints]` Cargo.toml table (stable since 1.74) vs `-D warnings` / `CARGO_BUILD_WARNINGS`

- The `[lints]` table (e.g. `[lints.clippy] all = "deny"`, plus `[workspace.lints]` inheritance) was **stabilized in Rust 1.74.0** (RFC 3389). It works on **stable**. Source: the Rust 1.74.0 release announcement [R74].
- Since Cargo 1.97, the **recommended** way to fail a build on warnings is the `build.warnings` config — on the command line via `CARGO_BUILD_WARNINGS=deny`, or permanently in `.cargo/config.toml` under `[build]`. It is **stable**, values `allow`/`warn`/`deny`. [C-CONFIG][C-CONFIG], [CL-USAGE]. The Clippy Usage CI example is literally `CARGO_BUILD_WARNINGS: deny` + `cargo clippy --all-targets --all-features --keep-going` [CL-USAGE]. The older `cargo clippy -- -D warnings` still works but can invalidate build cache.
- `CARGO_BUILD_WARNINGS=deny` raises **all** lint warnings (rustc + clippy warn-level lints) to errors; it does *not* enable allow-by-default groups (pedantic, nursery, cargo, restriction). [CL-USAGE]. To make clippy warnings into errors specifically, use `#![deny(clippy::all)]`; to add pedantry, add `#![warn(clippy::pedantic)]`.

### 0.3 rustfmt formatting (stable) vs rustfmt JSON/checkstyle emit (nightly-only)

- `cargo fmt` / `rustfmt` run on **stable** (`rustup component add rustfmt`). [RUSTFMT].
- `cargo fmt --check` (via `cargo fmt --all -- --check`) exits `0` if already formatted, `1` if it would reformat — that exit code is the machine signal. [RUSTFMT]
- rustfmt's `--emit json` / `checkstyle` / `coverage` modes are **nightly-only only** is false for `files`/`stdout`; the precise statement from the rustfmt source: `cargo fmt --message-format json` returns *"--message-format json is only supported in nightly builds"* [CARGOFMT-SRC]. So for a stable gate, the machine-readable signal from rustfmt is the **exit code**, not JSON.

### 0.4 Edition/lint levels and MSRV

- Lint levels are `allow`/`expect`/`warn`/`force-warn`/`deny`/`forbid`; `deny`/`forbid` produce a non-zero exit code (errors). [RC-LEVELS]
- Edions are enforced per-crate via `edition` in `Cargo.toml`; edition-transition lint groups (e.g. `rust-2024-compatibility`) are real rustc lint groups with default warn/deny and can be configured in `[lints.rust]`. [RC-GROUPS]
- MSRV is declared with `rust-version` in `Cargo.toml`; Clippy consumes it (`clippy.toml` `msrv = "1.x"` or the `rust-version` field). The `#[clippy::msrv = "1.x"]` *attribute* form requires nightly `custom_inner_attributes`. [RCLP]

---

## 1. Tool table

Environment verified this session: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`, with `rustfmt`/`clippy`/`rust-analyzer`/`rust-src` as stable rustup components. [LOCAL]

Columns:

- **What it catches (vs others)**: the one job no stable peer does, in one line.
- **Stable / nightly**: toolchain to *run on a 1.99.0 host* (MSRV noted where a tool pins a higher one).
- **Maintenance**: evidence (org + CI + version), not vibes.
- **Speed**: small→medium workspace, warm cache (qualitative; not measured here — research only).
- **Machine-readable output**: exact flag/form where documented.
- **Verdict for OxiDJ**: where it belongs (Tier 1 = every handoff; Tier 2 = pre-merge/CI; Tier 3 = scheduled/nightly/release) or "exclude".

| # | Tool | What it catches (that others don't) | Stable / nightly | Maintenance | Speed | Machine-readable output | Verdict for OxiDJ |
|---|------|-------------------------------------|------------------|-------------|-------|------------------------|-------------------|
| 1 | `rustfmt` (cargo fmt) | Code style/formatting conformance (the *only* formatter). | Stable (rustup component). JSON/`checkstyle` emit is **nightly-only** [CARGOFMT-SRC]. | rust-lang/rustfmt; part of toolchain; active. | ~seconds. | None in stable; **exit code** `cargo fmt --all -- --check` (0 ok / 1 would-reformat) is the stable signal. | **Tier 1** (every handoff). |
| 2 | rustc built-in lints | Type errors + correctness (unused vars, `dead_code`, `unsafe_op_in_unsafe_fn`, future-incompatible, `unsafe_`) — the ground truth compilers/lints cannot all model. | Stable (toolchain). | Rust project; maintained. | = `cargo check`. | `cargo check/build --message-format=json` (stable) [C-EXTERNAL][C-BUILD]. | **Tier 1** baseline; run via `cargo check`. |
| 3 | `clippy` (cargo clippy) | 800+ lint groups (correctness/suspicious/style/complexity/perf default-on; pedantic/nursery/cargo/restriction opt-in) beyond rustc. | Stable (rustup component, Rust 1.29+) [CL-INDEX][RCLP]. `cargo::` lints are nightly-only (see 0.1). | rust-lang/rust-clippy; maintained. | ≈ `cargo check` (~seconds warm). | `--message-format=json` (stable, via cargo) [C-EXTERNAL]. Exit ≠0 on `deny` diagnostics [CL-USAGE]. | **Tier 1** (`clippy::all` + `CARGO_BUILD_WARNINGS=deny`). Pedantic/nursery optional (high false positives). |
| 4 | rustdoc lints | Broken intra-doc links, private-intra-doc-links, missing `///` docs, invalid code-block attributes — docs-only problems. | Stable (`cargo doc`) [RUSTDOC-LINTS]. | Rust project; maintained. | doc build (tens of seconds). | `cargo doc --message-format=json` (stable) [RN]; config via `#![deny(rustdoc::...)]` [RUSTDOC-LINTS]. | **Tier 2** (`RUSTDOCFLAGS=-D rustdoc::* cargo doc --no-deps`). |
| 5 | `cargo-audit` | Known RUSTSEC advisories (vuln/unmaintained/unsound/yanked) in `Cargo.lock` against the advisory DB. | Stable (MSRV 1.74) [AUDIT]. | RustSec WG; CI badge; maintained. | <10s (cached DB). | `--json` / `--format json` / `--format sarif` [AUDIT-json], `--deny` exit control. | **Tier 2** CI gate (`cargo audit`). |
| 6 | `cargo-deny` | License compliance, banned/dupe crates, untrusted sources; **also** advisories (RustSec + GitHub Advisory DB, CVSS 4.0). | Stable (MSRV 1.88) [DENY]. | Embark Studios; CI; maintained. | ~10–30s network/metadata. | `--format json` / `--format sarif` [DENY-common]. | **Tier 2/3** supply-chain gate (`cargo deny --format json check`). |
| 7 | `cargo-machete` | Unused declared dependencies (graph-based; fast). | Stable (rustc stable+) [MCH]. | bnjbvr (Mozilla); CI; v0.9.2. | <5s. | `--json` [MCH]. Exit 0/1/2 (0 clean / 1 unused / 2 error) [MCH]. | **Tier 2** fast unused-dep gate (`cargo machete --json`). |
| 8 | `cargo-udeps` | *Actually-unused* dependencies that cargo-machete misses (compiles + inspects imports). | **Nightly-only**: "needs Rust nightly to actually run" (`cargo +nightly udeps`) [UDEPS]. | est31; maintained. | slow (full build). | Text only; no `--json` in README [UDEPS]. | **Tier 3** (nightly/CI). |
| 9 | `taplo` | Malformed/invalid TOML + schema violations in `Cargo.toml`/`.toml` (TOML hygiene, not Rust). | Stable (taplo CLI) [TAPLO]. | tamasfe; CI badge; README self-describes **"very young … bugs expected"**. ⚠ | ~seconds. | `--check` (schema); diagnostics text. | **Tier 2 optional** (only if TOML hygiene matters beyond Cargo's own parser). |
| 10 | `typos` | Misspellings in code, comments, identifiers, strings. | Stable [TYPOS]. | crate-ci; codecov; maintained. | fast (built for monorepos) [TYPOS]. | `--format json` (jsonlines; exit 0 ok / 2 typos) [TYPOS]. | **Tier 2** fast gate (`typos --format json`). |
| 11 | `cargo-nextest` | Test runner: retries/flakiness detection, better isolation + output; richer pass/fail structure. | Stable (run MSRV 1.41; build 1.91) [NEXT]. | nextest-rs; maintained. | faster than `cargo test` (parallel). | `cargo nextest list --message-format json` (stable) [NEXT-list]; run JSON `libtest-json` is **experimental** (env-gated, tracking #1152) [NEXT-list]. | **Tier 2** default test runner in CI. |
| 12 | `cargo-llvm-cov` | Line/region coverage gaps (`-C instrument-coverage`). | Stable (branch coverage + doc-test coverage **nightly**) [LLVMCOV]. | taiki-e; CI; maintained. | slow (instrumented build+run) [LLVMCOV]. | `--json` + JSON report [LLVMCOV]. | **Tier 3** coverage gate (CI schedule). |
| 13 | `cargo-semver-checks` | Breaking public-API / semver violations vs a published baseline (consumes rustdoc JSON). | **Nightly**: generates input via `cargo +nightly rustdoc -- -Zunstable-options --output-format json` [SEMVER-docsrs]; "supports stable and beta" JSON formats, nightly *best-effort* [SEMVER-readme]. | rust-lang; maintained. | moderate; network for registry baseline. | Human output; **no stable JSON reporter** — underlying rustdoc JSON is machine-readable but its format is **unstable** [SEMVER-readme]. | **Tier 3** (nightly; release gating only). |
| 14 | `cargo-hack` | Feature-combination + Rust-version-matrix breakage (catches feature-gating/MSRV bugs). | Stable [HACK]. | taiki-e; CI; maintained. | slow (many matrix combos). | Propagates `cargo` output; no dedicated JSON reporter. | **Tier 3** CI (feature matrix). |
| 15 | `cargo-mutants` | Inadequate tests: functions whose body could be *replaced without tests failing* (mutation score) — test-suite weakness. | Stable (also tests nightly + 1.88) [MUTANTS]. | sourcefrog/cargo-mutants; CI; v0.2.5. | **slow** (build+test × mutants); schedule only. | `mutants.out/outcomes.json` + `mutants.json`; `--list --json` [MUTANTS]. | **Tier 3** (CI schedule; requires non-flaky tests). |
| 16 | `miri` | Undefined behavior in *executed* paths (OOB, UAF, invalid init, misalignment, aliasing, data races) — dynamic. | **Nightly-only**: `cargo +nightly miri test` [MIRI]. | rust-lang; maintained. | very slow (interpretation). | Text diagnostics; no JSON. | **Tier 3** (nightly; **only** for crates with `unsafe`). |
| 17 | `cargo-geiger` | `unsafe` usage statistics across the dependency tree (a signal, **not** a security verdict). | Stable (cargo install; `cargo-geiger-serde` lib) [GEIGER]. | geiger-rs; CI badge; maintained. | moderate (full build + analysis) [GEIGER]. | JSON via `cargo-geiger-serde` serializable report types [GEIGER]. | **Tier 3** (audit info, not an automated gate). |
| 18 | `cargo-bloat` | Binary size / largest functions+crates in the release artifact. | Stable [BLOAT]. | RazrFalcon; maintained. | moderate (release build + llvm symbols). | `--message-format json` (`table`/`json`) [BLOAT]. | **Tier 3** (release-bloat gate). |
| 19 | `cargo-vet` | Supply-chain: crates.io deps not covered by trusted-org audits/imports (auditable trail). Stable (uses `cargo metadata`) [VET-how],[VET-cmd]. **Mozilla-backed**. | Stable [VET-how]. | Rust Secure Code WG / Mozilla; docs at mozilla.github.io/cargo-vet [VET-how][VET-cmd]. | moderate (graph traversal). | `--output-format json` [VET-cmd] (and mermaid/dump-graph). | **Tier 3** supply-chain gate (with audit+deny). |
| 20 | `cargo-outdated` | Dependencies with newer versions available (current vs latest-compatible vs latest). | Stable [OUTDATED]. | kbknapp; v0.19.0. ⚠ lower activity than peers. | moderate (temp `cargo update` workspace). | `--format json` (`list`/`json`) [OUTDATED]; **default exit 0**; fails CI only with `--exit-code <N>` (e.g. `--exit-code 1`) [OUTDATED]. | **Tier 3** (CI schedule; advisory, not blocking). |
| 21 | `cargo-public-api` | Public API surface diff across commits/releases (semver/API break detection). | **Nightly**: "relies on and automatically builds rustdoc JSON … nightly toolchain must be installed" [PUBLICAPI]. | cargo-public-api org; maintained. | moderate (rustdoc JSON build). | Text tree; underlying rustdoc JSON machine-readable (unstable) [PUBLICAPI]. | **Tier 3** (nightly) — **overlaps cargo-semver-checks (#13): pick one.** |

### Tools deliberately omitted / noted as overlap
- `cargo-public-api` vs `cargo-semver-checks`: both nightly + both consume unstable rustdoc JSON. cargo-semver-checks is the rust-lang-official lint set and the richer choice — **recommend adopting cargo-semver-checks only**, dropping cargo-public-api unless a pure API-diff is separately wanted. Recorded as a decision.
- `cargo-audit` vs `cargo-deny` (advisories): cargo-deny's advisory check is a superset (adds GitHub Advisory DB + CVSS 4.0). cargo-audit is the RustSec reference, fast, simpler. **Recommend**: `cargo audit` as a fast Tier-2 CI check **and** `cargo deny check` for policy. Some vendor sources call cargo-audit "older / less feature-rich" [rustfaq-cargo-audit] — labelled opinion; the RustSec WG is the canonical authority, so fact takes precedence.

### Other notable tools (not in the core gate) — listed for completeness
- `cargo-crev` — distributed code review of dependencies (crates.io/cargo-crev). Opinionated; optional audit signal.
- `cargo-expand` — pretty-prints macro expansion (dev/UX, not a gate).
- `dylint` — framework for *custom* clippy-style lints; the linter host pulls nightly. Useful later for project-specific lints.
- `cargo-tarpaulin` (Linux/macOS) — alternative coverage engine to `cargo-llvm-cov`; largely superseded by LLVM source-based coverage (cargo-llvm-cov). Mention for completeness only.
- `cargo-binstall`, `cargo-uv` — installers/integrations, not analyzers.

---

## 2. Tiered gate proposal

Guiding rule: **Tier 1 runs on every coding handoff and must be stable-only and fast.** Tier 2 runs on every CI attempt / merge. Tier 3 is nightly or slow, gated to nightly toolchains or scheduled jobs.

### Tier 1 — every coding handoff (stable only; target < ~2 min warm)
```sh
# 1. Formatting (exit 0 = clean; stable signal is the exit code, not JSON — rustfmt JSON is nightly)
cargo fmt --all -- --check

# 2. Clippy as the compiler/lint gate; treat warnings as errors (stable since 1.97)
CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features --keep-going
#    (equiv. persistent form in .cargo/config.toml: [build] warnings = "deny")
```
Budget: `cargo fmt --check` (<2s) + clippy (≈ `cargo check`; seconds warm / a few min cold, cached). `--keep-going` ensures the worker sees *all* lints, not just the first error.

### Tier 2 — every CI attempt / pre-merge (stable; target 1–5 min)
```sh
# Tests as the primary quality signal (nextest is the recommended stable runner)
cargo nextest run --all-features
#  (fallback: cargo test --all-features)

# Doc lints (broken intra-doc links, missing docs, code-block attrs)
RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D rustdoc::private_intra_doc_links -D rustdoc::invalid_codeblock_attributes -D rustdoc::missing_errors_doc" \
  cargo doc --no-deps --all-features --message-format=json

# Unused declared dependencies (fast; precise check in Tier 3)
cargo machete --json || true     # exit 2 = error, not "found unused"; review before failing

# Spelling
typos --format json

# Dependency vulnerability scan (fast; offline-friendly caches the DB)
cargo audit
```
Budget: tens of seconds to ~3 min. `--message-format=json` versions are machine-parseable by CI for the "evidence" step.

### Tier 3 — CI scheduled / release gating (nightly OR slow; NOT on every handoff)
```sh
# --- nightly-only analyses ---
cargo +nightly miri test                      # UB in executed paths (only for crates with `unsafe`)
cargo +nightly udeps                          # precise unused deps (no JSON; text only)
cargo +nightly semver-checks check-release    # breaking public API vs baseline (nightly rustdoc JSON)

# --- slow / heavy, stable ---
cargo mutants                                 # mutation score (writes mutants.out/outcomes.json)
cargo hack --feature-powerset --no-dev-deps check --all-features   # feature matrix
cargo bloat --release --crates --message-format json               # binary bloat
cargo deny --format json check                # full supply chain policy (licenses/bans/sources/advisories)
cargo vet                                     # trusted-audit supply-chain gate (Mozilla)
cargo outdated --format json                  # stale dependencies (advisory, not blocking)
cargo +nightly public-api                     # API surface diff (only if not using cargo-semver-checks)
```
Budget: miri/mutants can take **tens of minutes → run on a schedule** (nightly cron), not on every PR. semver-checks/bloat/hack run on release or nightly builds.

### Time-budget summary (small→medium workspace, warm cache; research-derived, not measured on OxiDJ)
| Tier | Runs at | Max budget | Why |
|------|---------|-----------|-----|
| Tier 1 | every coding handoff | <2 min | worker-blocking; must be fast |
| Tier 2 | CI / pre-merge | 1–5 min | full attempt gate |
| Tier 3 | nightly cron / release | 30–60+ min | slow/nightly; schedule off PR path |

> **Cache/parallelism note (fact):** CI should warm the Rust toolchain cache and reuse `target/` (e.g. `Swatinem/rust-cache`). `CARGO_BUILD_WARNINGS=deny` does *not* invalidate cargo's fingerprint the way `-D warnings` via RUSTFLAGS can [CL-USAGE][C-CI].

---

## 3. Handoff checklist for a coding worker

A coding worker must run all Tier-1 steps and paste the evidence below before marking a task `complete`. Tier-2 is the bar before requesting review; Tier-3 is reviewer/schedule responsibility.

The commands assume the **stable 1.99.0** toolchain and a workspace root containing `Cargo.toml`.

**Tooling availability:** Tier-1 commands use only rustup components already installed on the host (`rustfmt`, `clippy`, `rust-analyzer`). Tier-2/3 tools (`cargo-nextest`, `cargo-machete`, `cargo-audit`, `cargo-deny`, `cargo-vet`, `cargo-outdated`, `cargo-bloat`, `taplo`, `typos`, `cargo-mutants`) and the `+nightly` toolchain are **not** preinstalled here — provision once via `cargo install`/`cargo-binstall` (or pre-commit hooks) and a pinned nightly for Tier-3; see each README in §6. `cargo audit`/`cargo deny`/`cargo vet` additionally need the advisory/Cargo index (network, or `cargo audit fetch`/`cargo vet` with `--frozen` offline).

**Tier 1 — mandatory evidence on every handoff**
1. [ ] `cargo fmt --all -- --check` → exit code `0`.
   Evidence to paste: `echo "fmt exit=$?"` after the run (expect `0`).
2. [ ] `CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features --keep-going` → exit code `0` and **0 warnings**.
   Evidence to paste:
   ```sh
   CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features --keep-going --message-format=json 2>/dev/null \
     | python3 -c "import sys,json; msgs=[json.loads(l) for l in sys.stdin if l.strip()]; \
   w=[m for m in msgs if m.get('reason')=='compiler-message']; \
   print('clippy warnings:', len(w)); print('exit via cargo was 0')"
   ```
   (Expect "clippy warnings: 0" and a `0` exit from cargo.)
3. [ ] `cargo check --all-targets --all-features` → exit code `0` (proves it compiles).
   Evidence: `cargo check --all-targets --all-features` exits `0`.

If any Tier-1 step fails → fix the code and re-run until all are green. Do **not** complete the task until Tier 1 is clean.

**Tier 2 — required before requesting review**
4. [ ] `cargo nextest run --all-features` (or `cargo test --all-features`) → all tests pass, exit `0`.
   Evidence: copy the final `PASS`/`FAIL` summary line.
5. [ ] `RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D rustdoc::private_intra_doc_links -D rustdoc::invalid_codeblock_attributes" cargo doc --no-deps --all-features` → exit `0`.
   Evidence: exit code.
6. [ ] `cargo machete --json` → no unused deps (review output; do not auto-fix blindly because it is imprecise [MCH]).
   Evidence: `{}` or an empty unused list.
7. [ ] `typos --format json` → no findings, exit `0`.
   Evidence: exit code + json line count `0`.
8. [ ] `cargo audit` → no advisories, exit `0` (or only pre-approved, auditable `.cargo/audit.toml` ignores with `reason`+`expires`).
   Evidence: `cargo audit` summary line.

**Tier 3 — reviewer / schedule responsibility (not required every handoff)**
9. [ ] `cargo deny check` (CI supply-chain policy). Evidence: `cargo deny --format json check` summary.
10. [ ] `cargo vet` (trusted-audit trail). Evidence: `cargo vet` clean.
11. [ ] Nightly suite (in scheduled/nightly job): `cargo +nightly miri test`, `cargo +nightly udeps`, `cargo +nightly semver-checks check-release` (note: requires a pinned nightly). Evidence: job link + exit codes.
12. [ ] Mutation score (scheduled): `cargo mutants` → `mutants.out/outcomes.json` score ≥ target. Evidence: `Score:` line.

> Checklist validity: every command above runs on **stable 1.99.0 except** the Tier-3 `+nightly` items, which are explicitly gated to a nightly toolchain (not available on the current host — see §5). `cargo fmt --message-format=json` is intentionally avoided (nightly-only [CARGOFMT-SRC]); the stable signal is the exit code.

---

## 4. LSP / rust-analyzer integration for agentic use (Q4)

### What rust-analyzer provides (documented capabilities)
rust-analyzer is a **Language Server Protocol (LSP)** server. The book documents its integration surface [RA-MANUAL][RA-DIAG]:
- **Diagnostics**: surfaced as LSP `textDocument/publishDiagnostics`, sourced from `cargo check` by default, and from `cargo clippy` when configured with `"rust-analyzer.check.command": "clippy"` [RA-DIAG]. This lets *any* LSP client (editor **or automated agent**) receive structured compile/clippy diagnostics with spans + severity + lint code.
- **Code actions / quick fixes**: "Assists, or code actions, are small local refactorings … usually triggered by … a light bulb" [RA-ASSISTS]. Many clippy/rustc diagnostics carry machine-applicable suggestions exposed as code actions — exactly what an agent needs to *apply* a fix rather than guess it.
- **Symbols & navigation**: go-to-definition, find-all-references, hover, completion, inlay hints, workspace symbols [RA-MANUAL]. This gives an agent structured lookups instead of grep.

### How to wire it for agents (not interactive editors)
Two viable patterns; both need `rust-src` (installed) and, to be safe on the host, `RUSTUP_TOOLCHAIN=stable` [RA-INSTALL]:

1. **LSP-direct (primary for agents):** run the `rust-analyzer` binary as an LSP server over stdio JSON-RPC (`rust-analyzer` in `run`/`LSP` mode [RA-BIN]) and drive `textDocument/didOpen`/`didChange`, then consume `textDocument/publishDiagnostics` + `textDocument/codeAction` to apply suggestions. rust-analyzer is a rustup component here (`rustup component add rust-analyzer`) and is also downloadable as a prebuilt binary [RA-BIN]. This is the most "agent-native" path because diagnostics and fixes arrive structured.
2. **Structured CLI fallback (simpler, no LSP client):** `cargo clippy --message-format=json` and `cargo check --message-format=json` emit compiler/clippy diagnostics as JSON Lines (`reason=compiler-message` with `spans`/`suggestions`) [C-EXTERNAL]; `cargo nextest list --message-format json` gives a machine-readable test set [NEXT-list]. This re-creates the diagnostic signal without a long-lived LSP server.

### Evidence note (fact vs opinion)
The rust-analyzer book documents the **capabilities** (diagnostics, code actions, completion, symbols) — fact. It does **not** contain a controlled study showing "LSP integration improves agentic coding outcomes." The argument for agents is therefore *capability-based* (structured, fixable diagnostics + actions vs. guessing), not *empirically proven*. This is flagged in §5 as an unresolved question rather than asserted as proven.

### Recommended agent wiring for OxiDJ
- Prefer the **LSP-direct** path so the agent consumes `publishDiagnostics` + `codeAction` (the rust-analyzer assists include `add_missing_match_arms`, `auto_import`, `add_explicit_type`, etc. [RA-ASSISTS] — many map 1:1 to fixable compiler/clippy diagnostics).
- Always pair with the `[lints]` table + `clippy.toml` `msrv` so diagnostics respect OxiDJ's chosen MSRV/edition (see 0.2/0.4). Keep diagnostics green = Tier-1 gate.
- Pin `RUSTUP_TOOLCHAIN=stable` so rust-analyzer doesn't pick a stray nightly and break parsing [RA-INSTALL].

---

## 5. Unresolved questions and decisions needed (not more research)

1. **Pick one API/semver gate.** `cargo-semver-checks` and `cargo-public-api` overlap (both nightly, both unstable rustdoc JSON). *Decision:* adopt `cargo-semver-checks` (rust-lang, richer lint set) and drop `cargo-public-api` unless raw API diffing is separately required. [DEPENDENCY: lead]
2. **Supply-chain trio granularity.** `cargo audit` (RustSec only, fast) vs `cargo-deny` advisories (RustSec + GHSA + CVSS 4.0) overlap. *Decision:* keep both — `cargo audit` on every CI attempt (Tier 2) and `cargo deny check` for policy (Tier 3), OR consolidate to `cargo-deny` only. [DEPENDENCY: lead]
3. **Is network allowed in Tier 1 (every handoff)?** `cargo audit` and `cargo-deny`/`cargo-outdated` hit the registry/network. An offline-only handoff gate cannot run them. *Decision:* Tier-1 = fmt+clippy+check only (offline); supply-chain/network tools are CI/Tier-2+. [DEPENDENCY: lead]
4. **MSRV policy.** OxiDJ has no declared MSRV. Clippy `msrv` (`clippy.toml`) and `[lints]` edition groups depend on a declared `rust-version`. *Decision:* set `rust-version` in `Cargo.toml` and `msrv` in `clippy.toml` = current stable (1.99.0) with a planned policy. [DEPENDENCY: lead]
5. **Agent LSP client budget.** Building a stdio LSP client into the agent loop is heavier than `--message-format=json`. *Decision:* start with the JSON-fallback; add the LSP-direct path only if rust-analyzer's code-action suggestions measurably reduce agent iteration. *No primary study confirms the latter.* [DEPENDENCY: lead/research]
6. **Nightly toolchain policy.** Several Tier-3 tools need nightly (miri, cargo-udeps, cargo-semver-checks, cargo-public-api). The host has no nightly. *Decision:* run Tier-3 on a **dedicated nightly** in scheduled CI (pin a specific nightly per cargo-semver-checks recommendation [SEMVER-readme]); do **not** add nightly to the handoff environment. [DEPENDENCY: lead]
7. **Flaky-test gate for mutation testing.** `cargo-mutants` "requires non-flaky tests" [MUTANTS]. *Decision:* only enable after `cargo nextest` flakiness detection is proven stable. [DEPENDENCY: lead]
8. **taplo inclusion.** OxiDJ's only TOML is `Cargo.toml` (Cargo's own parser already validates it). taplo adds schema validation + formatting. *Decision:* adopt taplo only if custom `.toml` config is introduced beyond `Cargo.toml`. [DEPENDENCY: lead]
9. **cargo-outdated staleness.** Older maintainer (kbknapp, v0.19.0) vs `cargo-update`. *Decision:* keep for "schedule" use but do not make it a blocking gate. [DEPENDENCY: lead]

---

## 6. Primary sources (accessed for Rust 1.99.0 stable; this session)

- [LOCAL] Local toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`, `clippy 0.1.99`, `rustfmt 1.10.0-stable`, `rust-analyzer 1.99.0`; `rustup component list` = clippy, rust-analyzer, rust-src, rustfmt on stable.
- [R74] Rust 1.74.0 release — `[lints]`/[`workspace.lints`] stabilized (RFC 3389). https://blog.rust-lang.org/2023/11/16/Rust-1.74.0/
- [CL-INDEX] Clippy lint categories, default levels. https://doc.rust-lang.org/clippy/index.html
- [CL-USAGE] Clippy usage, CI (`CARGO_BUILD_WARNINGS=deny`), lint config in source, MSRV. https://doc.rust-lang.org/clippy/usage.html
- [RCLP] rust-clippy README (usage, `clippy.toml`, `--fix`, MSRV via `rust-version`). https://raw.githubusercontent.com/rust-lang/rust-clippy/HEAD/README.md
- [C-LINTS] Cargo lints emitted by Cargo itself — unstable/nightly-only. https://doc.rust-lang.org/cargo/reference/lints.html
- [C-CONFIG] `build.warnings` / `CARGO_BUILD_WARNINGS` (allow/warn/deny). https://doc.rust-lang.org/cargo/reference/config.html#buildwarnings
- [C-CI] Cargo continuous-integration guide (warnings=deny CI example). https://doc.rust-lang.org/cargo/guide/continuous-integration.html
- [C-EXTERNAL] `--message-format=json` is stable; applies to Cargo+Rustc output. https://doc.rust-lang.org/cargo/reference/external-tools.html
- [C-BUILD] `cargo build --message-format` values (human/short/json/…). https://doc.rust-lang.org/cargo/commands/cargo-build.html
- [RN] Rust release notes: `cargo doc --message-format=json`; `cargo fix`/`cargo clippy --fix` target selection. https://doc.rust-lang.org/nightly/releases.html
- [RC-LINTS] rustc lints overview. https://doc.rust-lang.org/rustc/lints/index.html
- [RC-LEVELS] Lint levels (allow/expect/warn/force-warn/deny/forbid) + exit semantics. https://doc.rust-lang.org/rustc/lints/levels.html
- [RC-GROUPS] rustc lint groups incl. edition-transition groups. https://doc.rust-lang.org/rustc/lints/groups.html
- [RUSTDOC-LINTS] rustdoc lints (broken_intra_doc_links, private_intra_doc_links, missing_docs) + `#![deny(rustdoc::…)]`. https://doc.rust-lang.org/rustdoc/lints.html
- [RUSTFMT] rustfmt README: stable `cargo fmt`; `--check` exit 0/1; stable vs unstable config options. https://github.com/rust-lang/rustfmt
- [CARGOFMT-SRC] `cargo fmt --message-format json` is nightly-only (source: `cargo_fmt` main.rs). https://doc.rust-lang.org/stable/nightly-rustc/src/cargo_fmt/main.rs.html
- [RA-MANUAL] rust-analyzer manual (features: diagnostics code actions/assists; integrated rustc+clippy diagnostics). https://rust-analyzer.github.io/manual.html
- [RA-INSTALL] rust-analyzer installation: needs `rust-src`; `RUSTUP_TOOLCHAIN=stable` override. https://rust-analyzer.github.io/book/installation.html
- [RA-BIN] rust-analyzer binary: LSP server; `rustup component add rust-analyzer`; prebuilt releases. https://rust-analyzer.github.io/book/rust_analyzer_binary.html
- [RA-DIAG] rust-analyzer Diagnostics: from `cargo check`, clippy via `rust-analyzer.check.command`. https://rust-analyzer.github.io/book/diagnostics.html
- [RA-ASSISTS] rust-analyzer Assists = code actions. https://rust-analyzer.github.io/book/assists.html
- [MCH] cargo-machete README: unused deps (fast/imprecise), `--json`, exit 0/1/2. https://raw.githubusercontent.com/bnjbvr/cargo-machete/HEAD/README.md
- [UDEPS] cargo-udeps README: nightly to run (`cargo +nightly udeps`). https://raw.githubusercontent.com/est31/cargo-udeps/HEAD/README.md
- [AUDIT] cargo-audit README (RustSec; MSRV 1.74; `cargo audit`; CI). https://raw.githubusercontent.com/rustsec/rustsec/HEAD/cargo-audit/README.md and https://rustsec.org/
- [AUDIT-json] cargo-audit CLI output controls: `--format {terminal|json|sarif}` (aliases `--json`); fail-CI via `--deny <SEVERITY>` (e.g. `--deny high`, `--deny warnings`). Canonical definition is `cargo audit --help` (RustSec/cargo-audit CLI); corroborated by the RustSec homepage (`cargo audit`) and cargo-audit's README [AUDIT]. https://rustsec.org/  (see also docs.rs/crate/cargo-audit)
- [DENY] cargo-deny README (Embark; MSRV 1.88; licenses/bans/advisories/sources). https://raw.githubusercontent.com/EmbarkStudios/cargo-deny/HEAD/README.md
- [DENY-common] cargo-deny `--format` (`human`/`json`/`sarif`). https://embarkstudios.github.io/cargo-deny/cli/common.html
- [HACK] cargo-hack README (feature-powerset; version-range). https://raw.githubusercontent.com/taiki-e/cargo-hack/HEAD/README.md
- [NEXT] cargo-nextest README (MSRV run 1.41/build 1.91; CI-ready). https://raw.githubusercontent.com/nextest-rs/nextest/HEAD/README.md
- [NEXT-list] nextest machine-readable listings + libtest-json (experimental, tracking #1152). https://nexte.st/docs/machine-readable/list/
- [LLVMCOV] cargo-llvm-cov README (LLVM coverage; `--json`; branch+doctest coverage nightly). https://raw.githubusercontent.com/taiki-e/cargo-llvm-cov/HEAD/README.md
- [SEMVER-docsrs] cargo-semver-checks docs.rs (nightly rustdoc JSON generation). https://docs.rs/crate/cargo-semver-checks/
- [SEMVER-readme] cargo-semver-checks README (rustdoc JSON, stable+beta support, nightly best-effort, no JSON reporter). https://github.com/rust-lang/cargo-semver-checks
- [MUTANTS] cargo-mutants (sourcefrog): stable; mutation testing; `cargo mutants`; `mutants.out/outcomes.json`; non-flaky requirement. https://github.com/sourcefrog/cargo-mutants (docs.rs/crate/cargo-mutants)
- [MIRI] Miri README (nightly; dynamic UB; `cargo +nightly miri test`; nextest integration). https://raw.githubusercontent.com/rust-lang/miri/HEAD/README.md
- [GEIGER] cargo-geiger README (unsafe stats; `cargo-geiger-serde` JSON). https://raw.githubusercontent.com/rust-secure-code/cargo-geiger/HEAD/README.md (repo: geiger-rs/cargo-geiger)
- [BLOAT] cargo-bloat README (`--message-format json`). https://github.com/RazrFalcon/cargo-bloat
- [VET-how] cargo-vet how it works (Mozilla; `cargo metadata`; trusted audit trail). https://mozilla.github.io/cargo-vet/how-it-works.html
- [VET-cmd] cargo-vet commands (`--output-format json`). https://mozilla.github.io/cargo-vet/commands.html
- [OUTDATED] cargo-outdated README (`--format json`/`list`; uses `cargo update`). https://github.com/kbknapp/cargo-outdated
- [PUBLICAPI] cargo-public-api README (nightly; rustdoc JSON; API diff). https://github.com/cargo-public-api/cargo-public-api
- [TAPLO] taplo README (TOML v1.0 toolkit; "very young"). https://github.com/tamasfe/taplo
- [TYPOS] typos README (spell checker; `--format json`; jsonlines exit codes). https://raw.githubusercontent.com/crate-ci/typos/HEAD/README.md
- [rustfaq-cargo-audit] (vendor/opinion) comparison of cargo-audit vs cargo-deny. https://www.rustfaq.org/en/how-to-use-cargo-audit-to-check-for-vulnerable-dependencies/

---

## 7. Lead decisions (2026-10-15)

The nine items in §5 are decided here. They are tooling and configuration choices — cheap to reverse — so they were settled by the lead rather than sent to independent review. All nine recommendations were accepted; item 10 is a deliberate deviation from the research.

1. **API/semver gate — one, not two.** Adopt `cargo-semver-checks`; drop `cargo-public-api`. They overlap (both nightly, both unstable rustdoc JSON) and semver-checks carries the rust-lang lint set. Release gating only.
2. **Supply-chain granularity — keep both.** `cargo audit` on every CI attempt (fast, the RustSec reference) and `cargo deny check` for full policy. The overlap is real but cheap, and they answer different questions.
3. **Tier 1 is offline.** `fmt` + `clippy` + `check` only. Network-dependent tools never run on the handoff path: an agent loop must not fail because the network did. This is the decision that makes the handoff gate reliable rather than aspirational.
4. **MSRV declared.** `rust-version = "1.99.0"` in the workspace manifest and `msrv` in `clippy.toml`, matching the installed stable toolchain. Revisit when a real MSRV support policy exists; declaring *something* is what makes `[lints]` edition groups and clippy's `msrv` lint meaningful.
5. **Agent diagnostics start on the JSON path, not LSP.** Use `cargo clippy --message-format=json`. The LSP-direct client is **deferred until it measurably reduces agent iterations** — §4 is explicit that the argument for LSP is capability-based and that no study supports it. Building an LSP client on an unproven premise is exactly the kind of premature complexity this project keeps rejecting.
6. **Nightly is quarantined to scheduled CI.** A dedicated pinned nightly exists only for Tier 3. Nightly must never enter the handoff environment — that is also what keeps Tier 1 honest about being stable-only.
7. **Mutation testing waits for test stability.** `cargo-mutants` is enabled only after `cargo-nextest` flakiness detection has proven stable. Mutation scores computed on a flaky suite are noise.
8. **taplo excluded for now.** OxiDJ's only TOML is `Cargo.toml`, which Cargo's own parser already validates; taplo also self-describes as young. Revisit if custom `.toml` configuration appears.
9. **`cargo-outdated` is advisory.** Scheduled output only, never a blocking gate.
10. **Deviation — the licence check is Tier 2, not Tier 3.** The research places `cargo deny check` in Tier 3 (scheduled). `ADR-009` requires the permissive-only licence policy to be **enforced**, and a scheduled check lets an accidental copyleft dependency reach a release before anyone notices. So `cargo deny check licenses bans sources` runs **pre-merge (Tier 2)**, while the advisory portion stays in Tier 3 where its network dependency belongs.

### Verification performed by the lead

The Tier-1 commands were **run, not assumed**:

- `cargo fmt --all -- --check` → exit `0` on a clean crate.
- `CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features --keep-going` → exit `0` on a clean crate.
- **Falsification test for the deny behaviour**, since a silently-ignored environment variable would also exit `0`: on a crate with a planted `unused_variables` warning, plain `cargo clippy` exited `0` with a warning, while `CARGO_BUILD_WARNINGS=deny cargo clippy` exited **101** with *"warnings are denied by `build.warnings` configuration"*. The persistent `.cargo/config.toml` form (`[build] warnings = "deny"`) produced the identical failure.

So the Tier-1 gate's central mechanism is confirmed working on this host's stable 1.99.0, including that it fails rather than warns when it should.
