# ADR Index

> M0 decision packets. Architecture directions are **Provisional** — evidence-backed recommendations pending the conditions their reviews attached. Only `ADR-009` (licence) is **Accepted**, because the project owner decided it directly. No architecture ADR becomes Accepted until its review has run *and* its condition is met.

## Accepted Decisions

| ADR | Title | Status | Decided by |
|-----|-------|--------|------------|
| [ADR-009](009-licence-mit.md) | Licence OxiDJ under MIT | **Accepted** | Project owner |

`ADR-009` is the only Accepted decision on the board, and it was made by the project owner rather than the lead. It is recorded because it *constrains* other decisions rather than because it needed review: MIT excludes Slint's free tier and Rubber Band outright, and it obliges a permissive-only dependency policy enforced by `cargo deny check licenses` in the build gate.

## Provisional Decisions

| ADR | Title | Status | Independent review |
|-----|-------|--------|--------------------|
| [ADR-001](001-gui-framework.md) | GUI framework (prototype-first, evaluate-second) | Provisional | **Done** — egui prototype confirmed, Slint compared, Qt deferred; confidence **medium** |
| [ADR-002](002-persistence.md) | SQLite + WAL for local-first persistence | Provisional | **Done** — Option 1 amended; confidence **medium**; prototype precondition attached |
| [ADR-003](003-metadata-representation.md) | Neutral internal domain model with provenance | Provisional | **Done** — Option 1 confirmed; confidence **high**; field-relationship risk open |
| [ADR-004](004-crate-boundaries.md) | Crate layer separation, realtime engine isolated | Provisional | **Done** — Option 1 with incremental crate creation; confidence **high**; M0 = `core` + `cli` only |
| [ADR-008](008-audio-engine.md) | Audio engine foundation — CPAL I/O, replaceable stretch engine | Provisional | **Done** — Option 1 provisionally; confidence **medium**; DJ-grade latency unmeasured |

## What the reviews changed

All three reviews ran on the Sol/medium lane, one decision per packet, no tools and no delegation. Two of them changed the recorded position:

- **ADR-002 — the ADR's own "high confidence" was rejected.** The reviewer returned **medium**, on the grounds that the performance and dependency figures are third-party benchmark claims rather than acceptance evidence, and amended the decision: Litestream is *optional* rather than the backup strategy, a generic SQL-portability trait is replaced by domain-facing repository interfaces, and a 500k-track prototype is a precondition before the dependency enters the workspace. The lead verified the load-bearing claims against <https://sqlite.org/wal.html> and <https://litestream.io/how-it-works/>; WAL gives reader/writer overlap but **not parallel writers**, and Litestream is asynchronous replication, not verified-restore backup.
- **ADR-003 — direction confirmed, but with a counter-risk that must be carried forward.** Per-field provenance does **not** capture relationships *between* fields, so changing a cue or beat grid can invalidate retained opaque metadata — bytes preserved, meaning corrupted. Source-native records plus a neutral projection and edit overlays is the named alternative. This makes import-edit-export validation decision-bearing, and it must be carried into ADR-005.
- **ADR-004 — the fixed crate inventory was rejected.** The reviewer approved the dependency **rules** but not a six- or seven-crate scaffold: empty crates enforce no boundary while adding public interface and coordination cost. M0 therefore contains `oxidj-core` + `oxidj-cli` only, and the durable deliverable is a `cargo xtask layers` gate that fails on a forbidden edge. Two clarifications worth keeping: "zero dependencies" for core means zero *infrastructure* dependencies rather than zero external crates, and **offline decoding is not realtime audio I/O** — analysis may own decoding without device access, so `oxidj-engine` is not pulled forward by analysis work.
- **ADR-008 (audio) — a candidate, not a qualification.** CPAL + Signalsmith Stretch provisionally, at **medium** confidence: *"CPAL is a reasonable Rust-first default, not proof of DJ-grade performance."* The decision's real output is the M0 data boundary — stable identities, source-frame positions, versioned analysis units, a shared engine-frame clock, bounded timestamped commands, preallocated PCM transfer, off-callback decoding and device recovery. The `rubberband` crate was ruled out on two independent grounds: GPL-2.0+/commercial **and** only 208 lifetime downloads, meaning the Rust path is effectively unproven.
- **ADR-001 (GUI) — prototype confirmed, but accessibility is the gating risk.** egui first, Slint compared, **Qt deferred rather than disqualified** (Qt's maturity does not transfer to `cxx-qt`, whose own maintainers call it pre-stable). Slint's GPL/commercial terms are a *conditional* constraint, not a veto. Critically, the prototype gate is **not a scrolling benchmark**: screen-reader table navigation, keyboard multi-selection and drag-and-drop must pass too.

**No ADR is `Accepted`.** Every review attached a condition, and the persistence precondition (a real prototype) has not been met.

**Lead correction:** the first draft marked ADR-002 and ADR-004 "Sol Review: Not required". Standing policy lists persistence architecture and difficult-to-reverse crate boundaries as review-mandatory, so all three are Required. ADR-001 is genuinely deferred: its decision is *not to choose a framework yet*, which is cheap to reverse, and the review belongs to the selection that follows the prototype.

## Format

See [`000-template.md`](000-template.md) for the decision packet format. Each ADR follows:
Context → Decision → Evidence → Options → Consequences → Open Questions → Sol Review Packet.

## Pending M1+

- ADR-005: Metadata schema (tables, Rust types)
- ADR-006: Analysis pipeline (BPM, beatgrid, waveform, key)
- ADR-007: Vendor adapter scope (Serato, Engine DJ, etc.)
