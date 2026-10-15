# rust-analyzer as a command-line analyser

M0-08. rust-analyzer ships as a rustup component (`rust-analyzer`), which is
normally an editor LSP server. Agents do not have an editor LSP client, so this
document records the **command-line** invocations that work on the pinned
**stable 1.99.0** toolchain. Every command below was executed on 1.99.0 and its
observed output is summarised — none is assumed.

The toolchain pin is `rust-toolchain.toml` (`channel = "1.99.0"`). rust-analyzer
1.99.0 was installed via `rustup component add rust-analyzer` (plus `rust-src`
so it can resolve std). `RUSTUP_TOOLCHAIN` is inherited, so it analyses under the
same 1.99.0 toolchain as the gate.

## Verified CLI invocations (stable 1.99.0)

### 1. Type-check / inference stats

```sh
rust-analyzer analysis-stats .
```

Batch-typechecks the workspace and prints summary statistics (crates, mods,
decls, bodies, ADTs, signatures, and per-phase timings). On the M0 workspace it
reports:

```
Total Statistics:
  crates: 4, mods: 7, decls: 74, bodies: 68, adts: 3, consts: 9, signatures: 88, variants: 3
Workspace:
  lines of code: 790, item trees: 4
  ...
  exprs: 2641, ??ty: 0 (0%), ?ty: 0 (0%), !ty: 0
  ...
Total: 3.65s, 38ginstr, 693mb
```

`??ty: 0 (0%), ?ty: 0 (0%), !ty: 0` means **zero** unresolved / unknown / never
types — the strongest signal that the workspace type-checks cleanly. This is a
fast, machine-parseable whole-workspace check an agent can run without an editor.

### 2. Structured diagnostics

```sh
rust-analyzer diagnostics .
```

Runs a diagnostic scan and prints one record per finding with **span + severity +
lint code**, e.g.:

```
at crate oxidj_cli, file .../crates/oxidj-cli/src/main.rs: WeakWarning RaLint("inactive_code", WeakWarning) from LineCol { line: 67, col: 0 } to LineCol { line: 96, col: 1 }: code is inactive due to #[cfg] directives: test is disabled

diagnostic scan complete
```

(The `test is disabled` WeakWarnings are expected: without `--enable-tests`,
rust-analyzer analyses in non-test mode, so `#[cfg(test)]` code is inactive. They
are not errors and the scan exits 0.) Records are line-oriented and parseable
(`at crate <crate>, file <path>: <Severity> <LintKind>(...)`).

## How to use them (and when not to)

- **Prefer `cargo xtask ci` as the gate.** rust-analyzer's `analysis-stats` /
  `diagnostics` are *supplementary* agent-facing analysis; the committed gate is
  `cargo clippy` (Tier 1) which already surfaces the same class of diagnostics with
  `--message-format=json`. rust-analyzer is for **navigation and early feedback**
  (goto-def, find-references, code actions via the LSP), and for agents that want a
  structured whole-workspace type-check summary without an editor.
- **rust-analyzer is not a substitute for Cargo.** `cargo check` skips
  codegen/linking; rust-analyzer `analysis-stats` does full type inference but is
  still an analyser, not a build or runtime proof. Require Cargo output before
  claiming the code compiles or works.
- For the machine-readable *build* diagnostics, use
  `cargo clippy --message-format=json` / `cargo check --message-format=json` (see
  `docs/handoff-checklist.md`). For machine-readable *test* structure,
  `cargo nextest list --message-format json` (Tier 2). rust-analyzer's own output
  is line-oriented, not JSON.

## Related

- Tier policy and the full tool table: `docs/research/rust-quality-gates.md` (M0-10).
- The repo-owned gate: `cargo xtask ci` / `cargo xtask ci-full`
  (see `docs/handoff-checklist.md`).
