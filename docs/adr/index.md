# ADR Index

> M0 decision packets. All six architecture ADRs and the licence ADR are now **Accepted**, each after its independent Sol review ran and its conditions were resolved. `ADR-009` (licence) was decided directly by the project owner; the other five were accepted on the strength of their reviews, with the recorded conditions carried forward as tracked preconditions.

## Accepted Decisions

| ADR | Title | Status | Decided by |
|-----|-------|--------|------------|
| [ADR-001](001-gui-framework.md) | GUI framework — prototype-first (egui), Slint commercial fallback, Qt deferred | **Accepted** | Lead (review low) |
| [ADR-002](002-persistence.md) | SQLite + WAL for local-first persistence | **Accepted** | Lead (review medium) |
| [ADR-003](003-metadata-representation.md) | Neutral internal domain model with provenance | **Accepted** | Lead (review high) |
| [ADR-004](004-crate-boundaries.md) | Crate layer separation, realtime engine isolated | **Accepted** | Lead (review high) |
| [ADR-008](008-audio-engine.md) | Audio engine foundation — CPAL I/O, replaceable stretch engine | **Accepted** | Lead (review low) |
| [ADR-009](009-licence-mit.md) | Licence OxiDJ under MIT | **Accepted** | Project owner |

`ADR-009` was made by the project owner rather than the lead. It is recorded because it *constrains* other decisions: MIT excludes Slint's free tier and Rubber Band outright, and it obliges a permissive-only dependency policy enforced by `cargo deny check licenses` in the build gate.

**Accepted ≠ unconditional.** Each acceptance carries tracked preconditions:

- **ADR-001** — final framework commitment gated on a 200k-row prototype, then 500k rows at 60fps plus accessibility/interaction evidence.
- **ADR-002** — persistence dependencies stay out of the workspace until a representative 500k-track prototype passes.
- **ADR-003** — field-relationship risk must be carried into ADR-005; import-edit-export validation is decision-bearing.
- **ADR-004** — M0 = `core` + `cli` only; a `cargo xtask layers` gate must fail on a forbidden edge.
- **ADR-008** — the foundation is not production-qualified; a bounded qualification spike is required before full engine implementation.

## What the reviews changed

All reviews ran on the Sol lane, one decision per packet, no tools and no delegation. Several changed the recorded position:

- **ADR-002 — the ADR's own "high confidence" was rejected.** The reviewer returned **medium**, on the grounds that the performance and dependency figures are third-party benchmark claims rather than acceptance evidence, and amended the decision: Litestream is *optional* rather than the backup strategy, a generic SQL-portability trait is replaced by domain-facing repository interfaces, and a 500k-track prototype is a precondition before the dependency enters the workspace. The lead verified the load-bearing claims against <https://sqlite.org/wal.html> and <https://litestream.io/how-it-works/>; WAL gives reader/writer overlap but **not parallel writers**, and Litestream is asynchronous replication, not verified-restore backup.
- **ADR-003 — direction confirmed, but with a counter-risk that must be carried forward.** Per-field provenance does **not** capture relationships *between* fields, so changing a cue or beat grid can invalidate retained opaque metadata — bytes preserved, meaning corrupted. Source-native records plus a neutral projection and edit overlays is the named alternative. This makes import-edit-export validation decision-bearing, and it must be carried into ADR-005.
- **ADR-004 — the fixed crate inventory was rejected.** The reviewer approved the dependency **rules** but not a six- or seven-crate scaffold: empty crates enforce no boundary while adding public interface and coordination cost. M0 therefore contains `oxidj-core` + `oxidj-cli` only, and the durable deliverable is a `cargo xtask layers` gate that fails on a forbidden edge. Two clarifications worth keeping: "zero dependencies" for core means zero *infrastructure* dependencies rather than zero external crates, and **offline decoding is not realtime audio I/O** — analysis may own decoding without device access, so `oxidj-engine` is not pulled forward by analysis work.
- **ADR-008 (audio) — a candidate, not a qualification.** CPAL + Signalsmith Stretch, at **medium** confidence: *"CPAL is a reasonable Rust-first default, not proof of DJ-grade performance."* The decision's real output is the M0 data boundary — stable identities, source-frame positions, versioned analysis units, a shared engine-frame clock, bounded timestamped commands, preallocated PCM transfer, off-callback decoding and device recovery. The `rubberband` crate was ruled out on two independent grounds: GPL-2.0+/commercial **and** only 208 lifetime downloads, meaning the Rust path is effectively unproven.
- **ADR-001 (GUI) — prototype confirmed, but accessibility is the gating risk.** egui first, Slint compared, **Qt deferred rather than disqualified** (Qt's maturity does not transfer to `cxx-qt`, whose own maintainers call it pre-stable). Slint's GPL/commercial terms are a *conditional* constraint, not a veto — it is retained only as a commercially licensed fallback. Critically, the prototype gate is **not a scrolling benchmark**: screen-reader table navigation, keyboard multi-selection and drag-and-drop must pass too.

**Lead correction:** the first draft marked ADR-002 and ADR-004 "Sol Review: Not required". Standing policy lists persistence architecture and difficult-to-reverse crate boundaries as review-mandatory, so all are Required. ADR-001 is genuinely deferred: its decision is *not to choose a framework yet*, which is cheap to reverse, and the review belongs to the selection that follows the prototype.

## Format

See [`000-template.md`](000-template.md) for the decision packet format. Each ADR follows:
Context → Decision → Evidence → Options → Consequences → Open Questions → Sol Review Packet.

## Pending M1+

- ADR-005: Metadata schema (tables, Rust types)
- ADR-006: Analysis pipeline (BPM, beatgrid, waveform, key)
- ADR-007: Vendor adapter scope (Serato, Engine DJ, etc.)
