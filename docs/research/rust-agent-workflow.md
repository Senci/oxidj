# M0-02 Rust toolchain and agent workflow audit

Verified 2026-10-15 in `/home/senci/dev/oxidj/.worktrees/t_40fbea5b`, branch `oxidj/t_40fbea5b-m0-02-rust-toolchain-agent-workflow-and`. Scope: lean tooling/setup/research, not product code, crate boundaries or CI implementation. See [developer guide](../development.md).

## Audit and actions

The card's initial “no rustup/cargo in PATH” observation was stale: prerequisite probes found user-local rustup and stable Rust. `command -v` alone was misleading for rustfmt/Clippy/rust-analyzer: rustup proxies existed, but execution failed because the components were absent. Version execution and installed-component listing are the readiness checks.

| Tool | Decision | Verified state / reason |
| --- | --- | --- |
| rustup | KEEP | 1.29.1, existing user-local installation; no reinstall/default change |
| rustc / Cargo / rust-std | KEEP | stable 1.99.0, x86_64-unknown-linux-gnu |
| rustfmt | INSTALL (done) | 1.10.0-stable; formatting is a low-cost deterministic gate |
| Clippy | INSTALL (done) | 0.1.99; default lints plus warning denial; no blanket restriction/pedantic |
| rust-analyzer | INSTALL (done) | 1.99.0; navigation/diagnostics, not a substitute for Cargo verification |
| rust-src | INSTALL (done) | standard-library navigation required by rust-analyzer's installation guidance |
| Git | KEEP | 2.55.0; existing linked worktree verified |
| C compiler/linker | KEEP | `cc --version` reports GCC 16.2.1; actual Cargo test binaries linked and ran |
| uv / Bun | KEEP | existing 0.13.0 / 1.4.3; project-selected Python/TypeScript managers, not Rust build prerequisites |
| cargo-nextest | SKIP for bootstrap | absent; native cargo test is sufficient for the current empty product suite. Reconsider when suite isolation/runtime gives a measurable benefit; retain doctest coverage separately |
| cargo-deny / cargo-audit | SKIP for this card | absent; no product dependency graph yet. M0-08 should choose one justified security/license gate with reviewed policy when a manifest exists, rather than install overlapping tools now |
| cargo-machete | SKIP for this card | no user-installed Cargo binaries; unused-dependency helper is premature without product dependencies |
| cargo-insta | SKIP | absent; no snapshot suite. Prefer explicit semantic assertions for metadata first |
| sccache / watch wrappers | SKIP | sccache absent; no demonstrated compile-time bottleneck; focused Cargo commands need no watcher |
| nightly / Miri / fuzz / coverage / benchmarks | SKIP for bootstrap | no justification to add another toolchain or helpers before unsafe code/parsers/measured hot paths exist |
| competing system Rust / duplicate managers | REMOVE candidate only | none encountered on the probed PATH; removed nothing, and did not inspect/delete unrelated user packages |

Executed installation: `rustup component add --toolchain stable rustfmt clippy rust-analyzer`, then `rustup component add --toolchain stable rust-src`. Installed component readback contains Cargo, rustc, rust-std and those four components. `cargo install --list` returned no user-installed Cargo tools. Helper decisions above are project scope decisions, not claims about all software installed elsewhere on the host.

## Hermes skill inventory

Audited the active `oxidj-2` skill directory and session catalog. The following is a relevance/routing audit based on installed names/descriptions, not an execution or security certification of every skill. Loaded `hermes-agent` and `llm-wiki` because their current commands and knowledge boundaries matter here. General process skills were not loaded merely to repeat existing engineering practice.

| Skills | Decision | Rationale / constraints |
| --- | --- | --- |
| hermes-agent; llm-wiki | KEEP | required for supported runtime operations and sourced research; use existing wiki, not another compiler/service |
| grounded-citations; blocked-page-recovery | KEEP | useful for provenance/retrieval failures; no need to load when direct HTTPS suffices |
| test-driven-development; systematic-debugging | KEEP | selective use for specialist work; enforce behavioral RED/GREEN in repository rules regardless of skill loading |
| spike; requesting-code-review; sdlc-review | KEEP | scoped experiments and review handoff; do not let generic automation install tools or change completion contracts implicitly |
| github; codebase-inspection | KEEP | relevant when publishing/reviewing or surveying actual code; no publishing in this task |
| codex; claude-code; opencode | KEEP optional, SKIP alternate execution routes | installed skills do not validate provider/model/quota. Keep project-approved Sol route; no PAYG or fallback changes |
| simplify-code | SKIP for M0 | multi-agent cleanup is not needed for a tooling card and must not bypass conservative concurrency |
| hermes-agent-skill-authoring | SKIP | no skill creation/editing in this one-shot run; durable rules go in repository docs |
| desktop/UI/computer-use, JS/Python debugging skills | SKIP for this task | later UI/debugging needs may justify them; shell/Cargo tools suffice now |
| media/creative/productivity/email/Apple/system-admin skills | SKIP for Rust baseline | unrelated, left intact; installed presence is not a project dependency |
| new Rust-specific or package-manager skills | INSTALL: none | compiler, primary Rust docs and repository conventions supply this bounded workflow; no justified external skill install |
| existing skills | REMOVE: none | no duplicate/problematic skill demonstrated; do not mass-delete unrelated capabilities |

No Hermes config, skill files, provider credentials, model fallback or shared memory configuration was modified. The toolchain is shared user state and was changed only by the components explicitly justified above.

## Primary-source research and wiki promotion

Queried the existing project's `.wiki/SCHEMA.md`, `index.md`, `log.md` and all three indexed Cargo concepts first. Added nine complete primary-source text captures through the existing Hermes `/llm-wiki` workflow, with source URL, ingestion date and body SHA-256 under `.wiki/raw/articles/m0-02-*.md`. No new wiki implementation or session-memory capture was introduced.

Synthesis: `.wiki/concepts/compiler-driven-rust-agent-workflow.md`. Navigation/log updated; existing `workspace-build-and-test.md` was corrected with evidence: bare cargo test is controlled by default-members, not universally all members. This is a research correction, not a product decision. The shared ignored wiki is in the canonical checkout; tracked deliverables remain solely in this task worktree. Wiki edits are the card's explicit research exception to workspace-only writes.

| Primary evidence | Finding promoted |
| --- | --- |
| [rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html) | minimal + explicit components; avoid complete profile |
| [rustup overrides](https://rust-lang.github.io/rustup/overrides.html) | versioned toolchain file; stable floats; explicit pins improve repeatability |
| [cargo check](https://doc.rust-lang.org/cargo/commands/cargo-check.html) | fast type checking does not prove codegen/link/runtime; explicit package/target coverage |
| [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html) | unit/integration/doc tests, filters, package-root cwd, default-member semantics |
| [Clippy usage](https://doc.rust-lang.org/clippy/usage.html) | -D warnings gate; restriction group is not intended to be enabled wholesale |
| [rust-analyzer installation](https://rust-analyzer.github.io/book/installation.html) | binary + editor/LSP + standard-library source; rust-src justified |
| [Cargo external tools](https://doc.rust-lang.org/cargo/reference/external-tools.html) | metadata format version and newline-delimited build diagnostics |
| [rustc JSON diagnostics](https://doc.rust-lang.org/rustc/json.html) | codes/spans/children/applicability; tolerate format additions |
| [Git worktree](https://git-scm.com/docs/git-worktree) | separate checkout/index/HEAD, shared repository state; not an environment sandbox |

The extraction service reached HTTP 429 on two pages. Direct HTTPS retrieval via curl/urllib succeeded for primary-source captures. Recommendations about fixture provenance, semantic roundtrips, regression discipline and agent iteration limits are engineering synthesis from documented execution semantics and OxiDJ's existing no-loss principles, not purported compiler guarantees or empirical agent-model performance claims.

## Real verification

`bash -n scripts/rust-toolchain-smoke.sh` and `bash scripts/rust-toolchain-smoke.sh` exited 0 on the installed stable toolchain. The probe created a disposable dependency-free library outside the repository, with a private target directory. Actual output showed:

- cargo check emitted structured E0308 mismatched-types diagnostics and failed intentionally.
- cargo fmt --check detected intentional formatting drift; cargo fmt repaired it.
- cargo clippy failed intentionally on clippy::eq_op.
- Regression `tests::answer_is_42` failed behaviorally with actual 41 versus expected 42 before correction.
- Final fmt/check/Clippy passed; one unit, one integration, and one doctest passed after correction.

Full observed transcript is preserved in `docs/research/rust-toolchain-smoke.log`; it is generated execution evidence, not example output. The probe crate itself is removed on exit. This verifies local compiler/formatter/linter and RED/GREEN behavior, not the future product or clean-host installer, cross-platform SDKs, dependency fetches or CI. Repeat with the checked-in script; M0-08 owns the fresh-clone product path and xtask verification.

Provenance checks passed all nine M0-02 body hashes and the four currently indexed concepts' metadata, tag, source, link, orphan and index-count checks. A whole-wiki hash check exposed a legacy serialization difference in the original Cargo capture: its hash excludes the blank frontmatter separator line and matches that original convention. It was left immutable and noted in the wiki log. Concurrent interoperability captures were not part of this card's ingestion. ShellCheck was not installed; shell syntax was verified with bash -n rather than claiming a ShellCheck run.

## Downstream handoff

For M0-08: use a reviewed exact compiler pin (tested 1.99.0); baseline fmt + default Clippy + explicit workspace build/unit/integration/doctests. Add helpers only when their policy and dependency graph justify them. Keep tests semantically independent and lawful fixtures deterministic. Do not create M1 models here.

No commits or pushes were made. Parent completion does not merge these uncommitted worktree files; lead integration must copy/apply the approved docs and script into M0-08's workspace. Canonical model-policy/status edits from M0-01 remain untouched; do not replace them with this worktree's older versions.
