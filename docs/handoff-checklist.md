# Handoff checklist — required evidence format for coding tasks

M0-08. Every coding task handed back **must** run the repo-owned gate and paste
the real output as evidence before it is marked `complete`. A task that has not
run the gate is not complete, however finished the code looks.

This checklist is the required evidence format. Fill it in verbatim; paste real
output, never a paraphrase. Current tier policy is defined by `AGENTS.md`, the
repo-owned `cargo xtask` gates, and `.github/workflows/ci.yml`; the research gate
report is background, not the current implementation spec.

> Toolchain: **stable Rust/Cargo 1.99.0** (pinned by `rust-toolchain.toml`).
> Tier 1 (`cargo xtask ci`) uses the pinned stable components.
> CI's `cargo xtask ci-full` includes tests and the mandatory `cargo-deny`
> licence gate (pinned at 0.20.2). `cargo audit`, `cargo machete`, and `typos`
> are best-effort CI advisories (`continue-on-error`); they may be absent locally.
> The current gate uses native `cargo test`, not `cargo-nextest`.

---

## Tier 1 — mandatory evidence on **every** handoff (stable, fast; no advisory scans)

Run `cargo xtask ci` and paste its output. It runs, in order:
`layers` → `fmt` → `clippy` → `check`. All four must report `PASS`. The gate
avoids remote advisory-database scans, but Cargo may fetch locked dependencies
when the local cache is cold.

- [ ] `cargo xtask ci` → every step `PASS (exit 0)`.
      Paste: the full `cargo xtask ci` output (or at least the per-step
      `==> [layers] PASS`, `==> [fmt] PASS`, `==> [clippy] PASS`,
      `==> [check] PASS` lines and the `=> ci summary: ... [all pass]` line).

## Tier 2 — required CI / pre-merge gate

- [ ] `cargo xtask ci-full` → green CI run including workspace tests and the mandatory `cargo deny check licenses` gate. Paste the run link and final summary.
- [ ] Optional advisory jobs (`cargo audit`, `cargo machete --json`, `typos --format json`) → if run, paste their output. These CI steps use `continue-on-error`; missing local binaries are `SKIPPED`, not a failing handoff.
- `cargo-nextest` is not part of the current gate; CI uses native `cargo test`.

## Tier 3 — deferred analyses (not a handoff prerequisite)

No in-repository scheduled Tier-3 workflow is currently configured. Do not claim
these checks ran, and do not block handoffs on them. If a pinned nightly/release
workflow is added later, retain its job link, toolchain pin, exit codes, and
summary for:

- `cargo +nightly miri test`, `cargo +nightly udeps`, and
  `cargo +nightly semver-checks check-release`.
- `cargo mutants` → `mutants.out/outcomes.json`; report the `Score:` line.

---

## Bug-reproduction invariant (for any bugfix handoff)

A bugfix additionally includes: revision/diff, compiler version, feature/target
flags, exact command, smallest lawful fixture, **expected** vs **actual** result,
**RED** output before the fix, **GREEN** output after it, and the broader gate
results. A failing build or a missing fixture is not the required behavioral RED.
Do not delete/ignore a failing test or weaken an assertion without an explicit,
reviewed behavior change.

## Fresh clone (one command)

A fresh clone reproduces the worker's result with a single command — no manual
toolchain or dependency setup:

```sh
git clone <repo> && cd oxidj
cargo xtask ci        # Tier 1 gate: layers + fmt + clippy + check
```

- `rust-toolchain.toml` pins `channel = "1.99.0"` (profile minimal, components
  rustfmt / clippy / rust-analyzer / rust-src), so rustup installs the exact
  tested toolchain automatically on the first `cargo` invocation. This is a
  compiler pin, not an MSRV assertion.
- `cargo xtask ci-full` additionally runs the tests and the licence gate
  (`cargo deny check licenses`). It needs network the first time to fetch the
  tiny `serde_json` build-dependency and the licence metadata — a normal
  fresh-clone fetch, not an offline bypass.
- The gate is self-documenting: one command on a clean clone yields the same
  green result a worker pastes as handoff evidence (verified by the M0-08
  fresh-clone probe).

## Fast focused loops

For a bounded change, run the narrowest gate that covers it, then widen:

```sh
cargo fmt --all -- --check                       # style (instant)
cargo clippy -p oxidj-core --all-targets         # one crate's lints
cargo test  -p oxidj-core track_id -- --exact    # one regression (fully qualified name)
cargo xtask ci                                   # full Tier 1 before a handoff
```

Never substitute a passing narrow check for the full `cargo xtask ci` on a
handoff, and never use `--offline` to mask a missing dependency fetch on a fresh
clone (it is only for known-cached / dependency-free probes).

## Do not

- Do not mark a task complete without pasting real gate output.
- Do not paste output you did not actually run (no fabricated / plausible output).
- Do not blanket-allow warnings to turn a gate green.
- Do not bypass `cargo xtask layers` to hide a forbidden dependency edge.
