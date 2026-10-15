# Rust development baseline

M0-02, verified 2026-10-15. This guide establishes tooling and workflow only; M0-08 owns the Cargo workspace, executable/CLI, xtask and CI. There is intentionally no product Cargo manifest in this task.

## Setup

Use user-local rustup, not a parallel system Rust installation. On a fresh machine, obtain the installer from [rustup's official site](https://rustup.rs/), review it, then run it without sudo:

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://sh.rustup.rs -o rustup-init.sh
sh rustup-init.sh -y --profile minimal --default-toolchain stable
. "$HOME/.cargo/env"
rustup component add --toolchain stable rustfmt clippy rust-analyzer rust-src
```

If rustup is already installed, do not reinstall it or silently replace the user's default toolchain:

```sh
rustup toolchain install stable --profile minimal
rustup component add --toolchain stable rustfmt clippy rust-analyzer rust-src
rustup show
rustup component list --toolchain stable --installed
rustc +stable -Vv
cargo +stable -V
cargo +stable fmt --version
cargo +stable clippy --version
rustup run stable rust-analyzer --version
bash scripts/rust-toolchain-smoke.sh
```

Stable is a moving channel, not reproducible by itself. The tested host toolchain is Rust/Cargo 1.99.0, x86_64-unknown-linux-gnu. To reproduce that compiler explicitly: `rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy --component rust-analyzer --component rust-src`, then run `RUSTUP_TOOLCHAIN=1.99.0 bash scripts/rust-toolchain-smoke.sh`. M0-08 should commit a `rust-toolchain.toml` with `channel = "1.99.0"`, `profile = "minimal"` and these components; refresh the pin intentionally with CI evidence. This is a compiler pin, not a selected MSRV.

A native linker/C build toolchain is also needed; `cc --version` succeeded on this host. Platform-specific linker/SDK setup must be verified when adding Windows/macOS CI; this Linux smoke test is not cross-platform validation. No sudo, shell-profile changes, additional targets, nightly or global Cargo helpers were needed here.

The smoke script creates and removes a dependency-free crate under `$TMPDIR` (fallback: `$HOME/.cache/oxidj`), isolates its target directory, and needs no crate registry access. Intentional compiler, formatter, lint and regression failures precede the final PASS. It is a toolchain probe, not a product test suite. An existing rustup toolchain override in `RUSTUP_TOOLCHAIN` is respected; default is stable. The installer path is documented, but reinstalling rustup on a pristine host was not exercised in this audit.

## Compiler-driven agent loop

1. Read intent, plan and applicable policies. Verify cwd, branch, manifest and working-tree state. Trace definitions/usages before calling an API. Read diagnostics instead of guessing a replacement dependency or symbol.
2. For a bug, first reproduce the behavior and add a deterministic test asserting the correct result. Observe a relevant failure on the original code. Fix the implementation, not the expectation. If a test cannot represent the failure (hardware, external format, nondeterminism), preserve the minimal reproducer and record the exception before claiming a fix.
3. Change one bounded concern. Use focused package/target checks and tests, then broaden the gate. Fix root diagnostics first; cascading errors are not independent design requirements.
4. Use rust-analyzer for navigation and early feedback, but require Cargo output. `cargo check` skips code generation/linking; it is not a build or runtime proof. Review compiler/lint suggestions against behavior; do not blanket-allow warnings to turn a gate green.
5. After about three unsuccessful attempts on the same diagnostic cluster, report commands, errors and attempted fixes for escalation. Preserve real output and current diff; do not replace a failed run with plausible output.

Once M0-08 provides a manifest, use commands shaped like these, with actual package/target names from the repository:

```sh
cargo metadata --format-version 1 --no-deps
cargo check -p <actual-package> --all-targets
cargo test -p <actual-package> <regression-filter> -- --exact
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
```

The exact filter must be the fully qualified test name; verify that at least one intended test ran. Bare `cargo test` follows default-members, and `--all-targets` does not replace doctests. Feature combinations must be chosen from the actual manifest: do not assume `--all-features` is a valid combination. `--locked` requires the application's committed, current lockfile. Use `--offline` only for dependency-free probes or known cached dependencies, not to mask normal fresh-clone fetch requirements. M0-08 should make its repo-owned gate authoritative rather than duplicate command lists in CI.

For machine-readable diagnostics, use `cargo check ... --message-format=json`, retain stdout and stderr separately, and inspect exit status. Parse newline records by `reason`; compiler-message has code/spans/children/rendered text. Tolerate unknown fields and non-JSON subprocess output. `rustc --explain <actual-code>` gives detailed compiler guidance. Use `RUST_BACKTRACE=1 cargo test ... -- --nocapture` when needed; sanitize logs before sharing. JSON is an optional troubleshooting interface, not a new custom agent framework.

## Bug reproduction invariant and fixtures

A bugfix handoff includes: revision/diff, compiler version, feature/target flags, exact command, smallest lawful fixture, expected versus actual result, RED output before the fix, GREEN output after it, and broader gate results. A failing build or missing fixture is not the required behavioral RED. If the regression is flaky, fix/isolate nondeterminism before treating one passing run as proof. Do not delete/ignore a failing test or weaken assertions without an explicit reviewed behavior change.

For future M1–M4 tests:

- Unit tests cover pure rules; integration tests cover persistence/adapters/public API; doctests keep promised usage executable.
- Store small redistributable or synthetic fixtures with provenance, license/permission, format/version, expected semantics, and any known loss. Do not commit private music, credentials or personal library identifiers. Preserve immutable raw evidence separately from expected results.
- Use per-test temporary files/databases. Tests run concurrently; do not share mutable cwd, environment, caches or a user's real library. Resolve fixture paths relative to the package manifest, not a presumed repo-root cwd.
- Roundtrip checks compare independent semantic expectations for cues/loops/grid/key/BPM, unknown metadata and ordering rules, not just bytes or an importer and exporter that can share the same mistake. Record intentional loss explicitly.
- Persistence tests eventually cover reopen/migrations/transactions and interrupted writes; malformed-input tests belong at format boundaries. Fuzzing/property/performance tools are justified once parsers, invariants and measured hot paths exist, not as bootstrap dependencies.

These are test requirements for later implementation, not a claim that product fixture/roundtrip tests already exist.

## Worktree and agent isolation

Use one task per worktree/branch and one worker per profile; preserve the project model policy and fail-safe empty fallback. A worktree does not sandbox globally installed tools, shell state, profile configuration, or shared wiki content.

Before/after execution, run `pwd`, `git rev-parse --show-toplevel`, `git branch --show-current`, `git status --short`, and independently `git -C <canonical-checkout> status --short`. Compare with the recorded starting state. Use explicit absolute workspace paths when invoking a child CLI. A fixed Hermes terminal.cwd can override intent: inspect supported configuration and verify real file readback in the intended directory before dispatch. Avoid shared mutable `CARGO_TARGET_DIR` between workers; worktree-local defaults are adequate initially.

Completion does not integrate a branch or uncommitted files. Lead integration must place approved parent changes into the downstream workspace before running its acceptance gate. Do not commit/push automatically. Shared policy/plan/wiki navigation files are collision hotspots; report them and serialize edits instead of silently overwriting a sibling's changes.

See [the sourced tooling/skills audit](research/rust-agent-workflow.md) and the canonical agent knowledge/model policies. External research is compiled in the existing Hermes `/llm-wiki`; no separate wiki engine or session-memory feature is introduced.
