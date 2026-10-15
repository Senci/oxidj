# Handoff checklist — required evidence format for coding tasks

M0-08. Every coding task handed back **must** run the repo-owned gate and paste
the real output as evidence before it is marked `complete`. A task that has not
run the gate is not complete, however finished the code looks.

This checklist is the required evidence format. Fill it in verbatim; paste real
output, never a paraphrase. Tier policy comes from
`docs/research/rust-quality-gates.md` (M0-10); commands are the repo-owned
`cargo xtask` invocations, so a fresh clone + one command reproduces the result.

> Toolchain: **stable Rust/Cargo 1.99.0** (pinned by `rust-toolchain.toml`).
> Tier-1 commands use only rustup components already installed on the host.
> Tier-2/3 tools (`cargo-deny`, `cargo-audit`, `cargo-machete`, `typos`, and the
> `+nightly` Tier-3 set) are provisioned by CI / as justified.

---

## Tier 1 — mandatory evidence on **every** handoff (stable, offline, fast)

Run `cargo xtask ci` and paste its output. It runs, in order:
`layers` → `fmt` → `clippy` → `check`. All four must report `PASS`.

- [ ] `cargo xtask ci` → every step `PASS (exit 0)`.
      Paste: the full `cargo xtask ci` output (or at least the per-step
      `==> [layers] PASS`, `==> [fmt] PASS`, `==> [clippy] PASS`,
      `==> [check] PASS` lines and the `=> ci summary: ... [all pass]` line).

## Tier 2 — required before requesting review (CI / pre-merge)

- [ ] `cargo xtask test` → all tests pass (unit + integration + doctests).
      Paste: the final `test result: ok` summary lines.
- [ ] `cargo machete --json` → no unused declared deps. (advisory; review
      output, do not auto-fix blindly — it is imprecise). Paste: the JSON
      summary line.
- [ ] `typos --format json` → no findings, exit `0`. Paste: exit code + `0` lines.
- [ ] `cargo audit` → no advisories (or only pre-approved, auditable ignores
      with `reason`+`expires`). Paste: the summary line.

## Tier 3 — reviewer / schedule responsibility (not every handoff)

- [ ] `cargo deny check licenses` (ADR-009: MIT; GPL/LGPL/AGPL/SSPL denied).
      This **is** in the CI gate (`cargo xtask ci-full`). Paste the summary.
- [ ] Nightly suite (scheduled, pinned nightly — NOT on the handoff host):
      `cargo +nightly miri test`, `cargo +nightly udeps`,
      `cargo +nightly semver-checks check-release`. Paste job link + exit codes.
- [ ] Mutation score (scheduled): `cargo mutants` → `mutants.out/outcomes.json`
      score ≥ target. Paste the `Score:` line.

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
