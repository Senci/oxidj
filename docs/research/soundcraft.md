# SoundCraft primary-source architecture review

Status: M0-03 research recommendations, not accepted architecture or an ADR.
Reviewed: 2026-10-15. Scope: patterns useful to OxiDJ's library-first MVP.

## Identity and reproducibility

The [official product page](https://getartcraft.com/apps/soundcraft) links to
[storytold/soundcraft](https://github.com/storytold/soundcraft); the repository README
links back to that page and `Cargo.toml` declares the same repository URL.
This is the ArtCraft Rust DAW, not a similarly named mixer product.

Inspected commit: `3a28a657692c1b98b28c276d8f61df9471074f0f`
(`Release 0.7.0`, committed 2026-10-14T09:33:59-04:00).
External checkout: `/home/senci/.hermes/profiles/oxidj-2/cache/scratch/soundcraft-primary`.
Origin: `https://github.com/storytold/soundcraft.git`; clean at inspection.
No SoundCraft implementation or assets were copied into OxiDJ.

Version discrepancy: the official website advertises 0.5.0; this source declares
0.7.0. Findings concern the pinned source, not the installed/downloaded release.
Neither website claims nor source presence prove production readiness.

Research interface: Hermes `/llm-wiki`; synthesis is at
`/home/senci/dev/oxidj/.wiki/entities/soundcraft.md`. Immutable captures are
`raw/articles/m0-03-soundcraft-{official-site,identity,architecture,execution,tests-and-distribution}.md`,
with body SHA-256 hashes and commit-qualified source URLs. The wiki is ignored,
external research storage; this repository note is the portable handoff.

## Evidence map

All source links below resolve against the inspected commit, not moving `main`.

| ID | Primary evidence | What was inspected |
|---|---|---|
| E1 | [Cargo.toml:1-59](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/Cargo.toml#L1-L59) | Globbed workspace members, central dependencies, lints, Rust 1.90, pinned shared UI git dependency. |
| E2 | [AGENTS.md:12-69](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/AGENTS.md#L12-L69) | Agent rules, assets, layering, commands, isolation, quality gates and release policy. Rules are intent, not proof. |
| E3 | [xtask/src/main.rs:19-169](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/xtask/src/main.rs#L19-L169) | Cargo metadata dependency guard; attribution coverage; explicit fmt/Clippy/test/assets/layers/WASM gate sequence. |
| E4 | [crates/engine/src/cmd/mod.rs:45-103](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/src/cmd/mod.rs#L45-L103) and [engine/src/lib.rs:298-378](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/src/lib.rs#L298-L378) | Discoverable commands, JSON boundary, enablement, snapshot undo/rollback, gesture merging. |
| E5 | [crates/automation/src/lib.rs:20-79](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/automation/src/lib.rs#L20-L79) and [docs/mcp.md:20-85](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/docs/mcp.md#L20-L85) | Headless dispatch, inspection, separate read/write authority, explicit bridge/confinement limits. |
| E6 | [crates/model/src/source.rs:9-81](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/model/src/source.rs#L9-L81), [session.rs:529-585](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/model/src/session.rs#L529-L585), [docs/architecture.md:27-59](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/docs/architecture.md#L27-L59) | Serde document, nonserialized decoded pool, session JSON, load-time repair and in-memory media. |
| E7 | [crates/engine/src/io.rs:895-1011](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/src/io.rs#L895-L1011) and [file_access.rs:1-21](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/src/file_access.rs#L1-L21) | Multi-file saves, temporary replacement, sync behavior and documented path-check race. |
| E8 | [hostile_params.rs:86-107](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/tests/hostile_params.rs#L86-L107), [hostile_sessions.rs](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/engine/tests/hostile_sessions.rs), [automation/tests/agent_tasks.rs](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/automation/tests/agent_tasks.rs) | Property tests and realistic public-protocol workflows; not all were executed here. |
| E9 | [crates/playback/src/lib.rs:208-233](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/playback/src/lib.rs#L208-L233), [docs/architecture.md:45-69](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/docs/architecture.md#L45-L69) | Shared render engine, snapshots and off-thread host work; callback transition allocations require scrutiny. |
| E10 | [release.yml](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/.github/workflows/release.yml), [packaging-lint.yml](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/.github/workflows/packaging-lint.yml), [Flatpak manifest](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/packaging/linux/flatpak/ai.storyteller.soundcraft.yml) | External reusable workflows, platform packaging and file/audio sandbox permissions. |
| E11 | [NOTICE](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/NOTICE), [ATTRIBUTION.md](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/ATTRIBUTION.md) | MIT OR Apache-2.0 code; separate proprietary brand terms and asset/font attribution. |
| E12 | [crates/ui-egui/Cargo.toml:1-34](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/crates/ui-egui/Cargo.toml#L1-L34), [docs/control-protocol.md:15-58](https://github.com/storytold/soundcraft/blob/3a28a657692c1b98b28c276d8f61df9471074f0f/docs/control-protocol.md#L15-L58) | UI Rust 1.95, test harness, isolated profiles, synthetic input and screenshot caveats. |

## Classification for OxiDJ

ADOPT means a compatible engineering pattern worth using, not approval of a new
crate/dependency/API. ADAPT needs OxiDJ-specific design. DEFER is outside the current
milestone. REJECT means do not transfer this particular pattern, not reject SoundCraft.

| Class | Pattern and evidence | OxiDJ application and boundary |
|---|---|---|
| ADOPT | Executable layering checks (E1-E3) | Keep UI dependencies out of the library/domain and adapters. Validate actual manifests once crate boundaries are accepted. Do not recreate all seven DAW layers now. |
| ADOPT | One application-operation path for UI and headless callers (E4-E5) | Import, edits and export must have the same validation and semantics regardless of caller; programmatic operations never open dialogs. Queries expose results for verification. |
| ADOPT | Explicit tooling gates and asset provenance (E2-E3, E11) | Run scoped tests plus deliberate broader checks; track asset origin/license. Attribution matching checks coverage, not legal correctness. Existing OxiDJ tooling policy still wins. |
| ADOPT | Errors rather than unsupported success; hostile-input regressions (E2, E4, E8) | Treat vendor files and automation inputs as untrusted. Return useful failures and loss reports. A caught panic is still a defect, not an ordinary successful validation result. |
| ADAPT | Command registry and JSON control boundary (E4) | Prefer typed internal requests/results and stable machine-readable errors; put JSON at external adapters. Keep command identity distinct from UI menu placement, shortcuts and incumbent menu catalogs. API shape remains for M0-07. |
| ADAPT | Undo/rollback and safe replacement (E4, E7) | Bound history by library size; use transaction semantics for persistent edits. Document external side effects: restoring an Arc does not undo filesystem writes, other engine fields, crashes or native plugin failures. |
| ADAPT | Separate read/write roots and isolated automation profiles (E5, E7, E12) | Make permissions explicit, test path/links/Windows behavior and prevent accidental personal-library access. Roots are a convenience boundary, not a hostile-process sandbox; do not expose a desktop bridge assuming confinement carries over. |
| ADAPT | End-to-end agent workflows and inspection (E8, E12) | Test import → inspect → edit → export → independent re-import against semantic fixtures. Screenshots/synthetic input complement, not replace, native accessibility/keyboard acceptance. |
| ADAPT | Separation of stored metadata and decoded caches (E6) | Preserve durable track/location/vendor provenance separately from replaceable waveforms/analysis/audio buffers. Do not equate a session SourceId/path with cross-vendor track identity. |
| ADAPT | Platform packaging and explicit permissions (E10, E12) | Later test real external/removable library folders and file portals; xdg-music alone is not enough evidence for a DJ workflow. Declare effective per-crate/toolchain requirements, not just root MSRV. |
| DEFER | MCP server, desktop TCP bridge and screenshot API (E5, E12) | Command-testability is useful now; a public server/protocol is optional later. Localhost binding alone does not establish authentication or authorization. |
| DEFER | Realtime render graph, devices, plugin hosts, WASM/FreeBSD and full installer matrix (E9-E10) | No decks, plugin ABI, DAW routing or broad release matrix in M0/M1. Keep future audio isolated; choose GUI/platform/storage in their separate evaluations. |
| REJECT | Whole-session JSON plus eagerly decoded media as the library storage design (E6-E7) | A large DJ collection is not a DAW session. No blanket media copying, resampling, eager full decode, or whole-library snapshots on each edit. This does not preselect a database. |
| REJECT | Silent load-time repair as imported-metadata policy (E6) | The source sanitizes names, drops nonpositive clips and repairs values. OxiDJ must preserve original vendor fields/provenance and report interpretations/repairs; repairs must not silently overwrite evidence. |
| REJECT | Incumbent-menu parity counts as OxiDJ acceptance (E2-E4) | M7 success is metadata-safe interoperability, not matching a menu tree. SoundCraft's own feature counts are not an independent quality assessment. |
| REJECT | Floating remote CI and external agent-rule dependencies as reproducibility defaults (E2, E10) | SoundCraft delegates release/lint workflows to another repo at @main and references external craftrules/local ignored plans. Keep required OxiDJ rules in-repo and pin/review external automation before adopting it. |

## Important limits and uncertainties

- The source is a DAW. Its playlists, memory locations and tempo maps are not
  evidence for Rekordbox/OneLibrary cue, beatgrid, identity or round-trip semantics.
  Those mappings belong to the independent interoperability and Mixxx research.
- The layer gate checks declared direct dependencies, a fixed UI-name denylist and
  registered internal crates. Dev dependencies/apps/xtask are exempt; same-layer
  dependencies are allowed. Passing it is useful, not a proof of complete separation.
- `Engine::execute` restores the document on errors/panics, with a narrower zoom-only
  restoration for view commands. It is not a database transaction or a guarantee
  against process abort, OOM, native faults or previously completed external writes.
- Session saves write audio before replacing the session file. Temporary replacement
  syncs the file, but directory sync errors are ignored and the temp name is fixed.
  We did not prove all-files atomicity, concurrent-writer safety, power-loss recovery
  or Windows overwrite behavior. Do not copy this as a durability contract.
- The callback rules are aspirations with nuanced scope: playback transition code
  can rebuild buffers/engine state and uses std channels. No allocation, latency,
  deadline or dropout audit was performed. Device/DSP recommendations are deferred.
- The root declares Rust 1.90 but the UI declares 1.95; neither older compiler was
  exercised. Tests ran with the local toolchain, not a cross-platform/MSRV matrix.
- Release/lint workflows call remote @main workflows; these transitive workflows
  and signing/installers were not audited or executed. The advertised platform list
  is not independently verified support. No third-party dependency license audit.
- SoundCraft code is dual licensed, but ArtCraft logos are explicitly excluded.
  Researching design is not authorization to copy branding, proprietary assets or
  vendor internals. This review copies no implementation into OxiDJ.

## Verification and review

Executed in the external pinned checkout:

- `cargo xtask layers` → `layers: ok`.
- `cargo xtask assets` → `assets: ok`.
- `cargo test --locked -p soundcraft-time -p soundcraft-model` → model unit tests:
  35 passed; time unit tests: 29 passed; meter-grid integration tests: 3 passed;
  both doc-test targets: 0 tests, successful. No failures.

These runs establish only the named gates/tests. Engine property tests, MCP workflow
acceptance, desktop UI, audio, installers and the full `cargo xtask ci` were inspected
or referenced, not executed. No product implementation was required or added.

The repository note was self-reviewed against the pinned excerpts: observed behavior
is separated from source policy, recommendations and unverified claims; all requested
areas are covered; no GUI, persistence, crate layout or metadata schema was selected.
Provenance/link/hash validation and `git diff --check` are recorded in
`soundcraft-verification.txt`. M0-07 owns independent decision review and promotion
into ADRs; this note must not silently become accepted architecture.
