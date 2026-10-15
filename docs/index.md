# Documentation index

This map lists every document in the repository, grouped by purpose. Use it when
you need to find your way around OxiDJ.

**Evidence vs. decisions.** The `docs/research/` directory, `docs/semantic-mapping.md`,
and `docs/architecture.md` are **research evidence** — findings, comparisons, and
open questions, not commitments. The `docs/adr/` documents are **architecture decisions**.
`docs/adr/index.md` records all six architecture ADRs and the licence ADR as
`Accepted` after their independent Sol reviews, but stresses that *“Accepted ≠
unconditional”* — each carries tracked preconditions that gate implementation.
`docs/architecture.md` labels the M0 baseline `PROVISIONAL (M0-07)` and
`plan/STATUS.md` describes the ADRs as Provisional, not Accepted. Check those
sources for the full precondition list before treating any direction as settled.

A `[evidence]` tag marks research/analysis documents; a `[decision]` tag marks
ADRs. Policy and planning documents are neither.

---

## Start here

Read these first.

- [idea.md](../idea.md) — project vision, priorities, principles, and the
  library-first MVP definition. Read before anything else.
- [AGENTS.md](../AGENTS.md) — rules for agent contributors: worktrees, tooling,
  and conventions. Read before editing any file.
- [docs/architecture.md](architecture.md) — the M0 architecture baseline
  (crate layout, domain model, persistence, audio, GUI). Read for the current
  design direction; labelled PROVISIONAL (M0-07) pending review.
- [docs/development.md](development.md) — developer setup: toolchain install,
  the compiler-driven agent loop, and worktree isolation. Read to set up a dev
  environment or write code.
- [docs/glossary.md](glossary.md) — DJ and domain vocabulary (beatgrid, cues,
  loops, provenance, vendor formats). Read whenever a term is unintelligible.
- [docs/index.md](index.md) — this map. Read to navigate the repository.

## Architecture decisions `[decision]`

Each ADR follows the packet: Context → Decision → Evidence → Options →
Consequences → Open Questions → Sol Review Packet (docs/adr/000-template.md).

- [docs/adr/index.md](adr/index.md) — index of all ADRs with status, review
  confidence, and tracked preconditions. Start here for decisions.
- [docs/adr/000-template.md](adr/000-template.md) — the decision-packet format
  template for writing new ADRs.
- [docs/adr/001-gui-framework.md](adr/001-gui-framework.md) — GUI: prototype-first
  in egui/eframe (Slint is a commercial fallback only under MIT — ADR-009),
  Qt deferred. Final framework commitment gated on a 200k-row virtualized list +
  accessibility evidence.
- [docs/adr/002-persistence.md](adr/002-persistence.md) — SQLite + WAL with
  `rusqlite` (bundled) and `rusqlite_migration`. Dependencies stay out of the
  workspace until a 500k-track prototype passes.
- [docs/adr/003-metadata-representation.md](adr/003-metadata-representation.md) —
  neutral internal domain model with per-field provenance; preserve constant vs
  variable tempo, point cues vs loop ranges.
- [docs/adr/004-crate-boundaries.md](adr/004-crate-boundaries.md) — layered crate
  separation (core → cli → library/storage/analysis/audio → UI); realtime engine
  deferred to M4. Enforced by `cargo xtask layers`.
- [docs/adr/008-audio-engine.md](adr/008-audio-engine.md) — audio foundation:
  CPAL for device I/O, replaceable time-stretch engine (Signalsmith first). Not
  production-qualified.
- [docs/adr/009-licence-mit.md](adr/009-licence-mit.md) — MIT licence
  (decided by the project owner). Excludes Slint’s free tier and Rubber Band;
  mandates a permissive-only dependency gate.

## Research evidence `[evidence]`

Findings, comparisons, and claim audits — not commitments. All primary sources
cited with URLs and dates.

- [docs/research/README.md](research/README.md) — M0-06 research index: the
  GUI, persistence, and audio/platform evaluations.
- [docs/research/agent-config-best-practices.md](research/agent-config-best-practices.md) —
  agent + repository configuration best practices (loops, unwanted patterns).
- [docs/research/audio-platform-evaluation.md](research/audio-platform-evaluation.md) —
  audio/platform boundaries: CPAL backend matrix, Symphonia decode, RT-safe DSP
  candidates, Linux/macOS/Windows considerations.
- [docs/research/gui-evaluation.md](research/gui-evaluation.md) — GUI framework
  evaluation (virtualized lists, AccessKit accessibility, platform support,
  maintenance risk).
- [docs/research/interoperability.md](research/interoperability.md) — OneLibrary
  and Rekordbox interoperability research; fixture and experiment plan.
- [docs/research/m0-06-claim-audit.md](research/m0-06-claim-audit.md) —
  independent claim audit of the three M0-06 evaluation docs (by sova).
- [docs/research/m0-01b-benchmark-evidence.md](research/m0-01b-benchmark-evidence.md) —
  free worker (`poolside/laguna-s-2.1:free`) + fallback validation evidence.
- [docs/research/m0-01b-t5-patch-review.md](research/m0-01b-t5-patch-review.md) —
  patch review: off-by-one in `BpmRange::contains` found by a free worker.
- [docs/research/model-worker-benchmark.md](research/model-worker-benchmark.md) —
  coding worker route validation (Codex free fallback chain).
- [docs/research/persistence-evaluation.md](research/persistence-evaluation.md) —
  SQLite vs alternatives for 100k–500k tracks; PRAGMA settings, migrations,
  backup strategy.
- [docs/research/rust-agent-workflow.md](research/rust-agent-workflow.md) —
  M0-02 Rust toolchain and agent workflow audit (tooling, citations, smoke test).
- [docs/research/rust-quality-gates.md](research/rust-quality-gates.md) — M0-10
  Rust static-analysis & quality-gate best practices; 21-tool table, tiered gate,
  lead decisions (§7). The authoritative gate definition.
- [docs/research/rust-toolchain-smoke.log](research/rust-toolchain-smoke.log) —
  observed transcript of the toolchain smoke test (formatter, linter, RED/GREEN).
- [docs/research/soundcraft.md](research/soundcraft.md) — SoundCraft primary-source
  architecture review (M0-03); layer separation, command-first operations, hostile
  input handling.
- [docs/research/soundcraft-verification.txt](research/soundcraft-verification.txt) —
  SoundCraft provenance and verification transcript (commit hash, SHA-256, source
  ranges).

## Policy

Engineering rules and conventions, committed in-repo for reproducibility.

- [docs/agent-knowledge-policy.md](agent-knowledge-policy.md) — knowledge
  precedence: repository decisions → verified evidence → wiki synthesis →
  Hindsight; promotion path for external knowledge.
- [docs/agent-model-policy.md](agent-model-policy.md) — model routing, Sol
  delegation lane (medium reasoning for important work), verified free fallback
  chain, worker validation.
- [docs/agentic-workflow.md](agentic-workflow.md) — the agentic-Rust workflow:
  what a worker must do and show on every card, mapped to the M0-D4 research.
- [docs/worktree-isolation.md](worktree-isolation.md) — one worktree and one
  `target/` per agent; the `CARGO_TARGET_DIR` rule and the worker checklist.
- [docs/layers-policy.md](layers-policy.md) — the dependency-layer model and the
  `cargo xtask layers` guard (ADR-004); how the gate stays meaningful as crates
  are added.
- [docs/diagrams.md](diagrams.md) — diagram conventions: Mermaid type selection,
  dark/light profiles, media tags, and guardrails.
- [docs/handoff-checklist.md](handoff-checklist.md) — required evidence format
  for coding task handoffs; Tier 1/2/3 checklist and fresh-clone reproduction.
- [docs/testing.md](testing.md) — test strategy for M0 (unit/integration/doctest
  of `TrackId`) and requirements for M1+ (semantic round-trips, fixtures, fuzzing).
- [docs/rust-analyzer-cli.md](rust-analyzer-cli.md) — using `rust-analyzer` as a
  command-line analyser on stable 1.99.0 (`analysis-stats`, `diagnostics`).
- [docs/semantic-mapping.md](semantic-mapping.md) — cross-ecosystem metadata
  semantics mapping (Mixxx, neutral OxiDJ, Rekordbox/OneLibrary). Evidence map,
  not a schema. Read alongside the ADRs.

## Planning

- [plan/MVP.md](../plan/MVP.md) — milestone roadmap M0 through M7.
- [plan/STATUS.md](../plan/STATUS.md) — current milestone (M0), blockers, recent
  decisions, and next tasks. Read for the live project state.

---

*Generated for the documentation entry-points task. Links are relative to this
file; every target above resolves within the repository tree.*
