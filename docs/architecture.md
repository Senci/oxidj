# OxiDJ Architecture Baseline

**Status:** PROVISIONAL (M0-07) — minimal architecture baseline, pending independent review
**Lead:** OxiDJ (oxidj-2)
**Date:** 2026-10-15
**MSRV:** Rust 1.99.0 (pinned, not MSRV — see `docs/development.md` line 31)

This document records the **provisional** M0 architectural direction. Per the M0-07 integration gate and the Nous-only free-model routing policy, all directions are evidence-backed recommendations pending independent review; no decision is Accepted architecture until its review gate is restored and completed. Decisions are captured in [ADRs](adr/). Speculative or deferred choices are listed in [Open Decisions](#open-decisions) and remain unresolved until M1+.

---

## 1. Vision

OxiDJ is a modern, open-source DJ library and eventually performance application written primarily in Rust. Long-term goal: a credible Rekordbox alternative without vendor lock-in. See `idea.md`.

---

## 2. Architecture Principles

From `idea.md` and confirmed by M0-03/M0-06 research:

1. **Rust-first**, not Rust-at-all-costs.
2. **Local-first** — no mandatory cloud account.
3. **Linux-first** development, cross-platform architecture.
4. **Vendor formats behind adapters** — Rekordbox, Mixxx, Serato, etc. are adapter crates, not core.
5. **Neutral internal domain model** — see ADR-003.
6. **No silent metadata loss** — unknown fields preserved; loss reported, not discarded.
7. **Reproducible import/export and round-trip tests** — with semantic fixtures, not byte comparison alone.
8. **UI separated from business logic** — shared command path for GUI and CLI.
9. **Realtime audio isolated** from ordinary application code — separate crate, M4+.
10. **Mature libraries before reimplementation** — Symphonia for decode, SQLite for storage.

---

## 3. Crate Layout

Established by ADR-004. The dependency flow is one-directional:

```
oxidj-cli        (command-first entry point — M0 scaffold)
  └── oxidj-core (neutral domain model — M0 scaffold)

Future layers (M1+), each depending only on core:
  oxidj-library  — import/export, metadata pipeline
  oxidj-storage  — SQLite persistence, migrations
  oxidj-analysis — BPM, beatgrid, waveform, key
  oxidj-audio    — Symphonia decode (offline, M0-M3)
  oxidj-adapters — vendor format crates (oxidj-mixxx, oxidj-rekordbox, ...)
  oxidj-ui       — egui/Slint GUI (M3, see ADR-001)

Future engine (M4+, depends only on core):
  oxidj-engine  — CPAL I/O + RT-safe DSP (NOT in M0)
```

**M0 scaffolding scope:** only `oxidj-core` and `oxidj-cli`. No adapter, storage, analysis, audio, UI, or engine crates.

**Layer gate:** `cargo xtask layers` (adapted from SoundCraft, soundcraft.md lines 54-58) must pass — UI/persistence/audio dependencies must not leak into core.

---

## 4. Neutral Domain Model

Established by ADR-003. The core domain model is neutral and provenance-aware:

- **Internal identity** is independent of vendor IDs and file paths. Resolved via normalized path + content evidence (file hash or acoustic fingerprint — selection deferred to M2).
- **Vendor metadata is preserved.** Unknown fields stored as opaque payloads never silently discarded.
- **Per-field provenance:** every metadata field records its source (ANALYZER, FILE_METADATA, USER, or vendor import).
- **Representation preserved:** constant vs variable tempo beat grids are distinct; point cues vs loop ranges are distinct; never flattened to averages.
- **Non-destructive import/export:** unsupported/lost fields reported before writing; original media preserved until user approval.

The detailed semantic mapping across Mixxx, OneLibrary/Rekordbox, and OxiDJ is in [`docs/semantic-mapping.md`](semantic-mapping.md). The database schema (tables, columns, Rust types) is deferred to ADR-005 (M1).

---

## 5. Persistence

Established by ADR-002. SQLite with WAL mode is the default:

- **Engine:** SQLite, WAL journal mode, `synchronous=NORMAL`.
- **Rust binding:** `rusqlite` with `bundled` feature.
- **Migrations:** `rusqlite_migration` via `PRAGMA user_version`.
- **Connection model:** single writer + read pool, `BEGIN IMMEDIATE`.
- **Backups:** Litestream (WAL streaming to S3/GCS) + `.backup` API for CI.
- **Abstraction:** SQL behind a trait to enable future PostgreSQL/libSQL migration.

**Dataset scale:** 500k tracks ≈ 50–100 MB — well within SQLite's comfort zone. Migration threshold: >1TB + 2+ other signals (write contention, multi-process writes, >10K TPS).

---

## 6. Audio Architecture

Established by ADR-004 and `audio-platform-evaluation.md`:

```
M0-M3 (Library/Analysis)     M4+ (Realtime Engine)
┌────────────────────────┐   ┌──────────────────────────┐
│  oxidj-audio          │   │  oxidj-engine             │
│  (Symphonia decode)   │   │  (CPAL I/O + dasp/fundsp) │
│  waveform generation   │   │  resampler, mixer, effects│
│  BPM/beatgrid analysis │   │  RT-safe, zero-alloc     │
└────────────────────────┘   └──────────────────────────┘
        ↑                              ↑
        └── both depend on oxidj-core only
```

- **Decode/analysis (M0-M3):** Symphonia (pure Rust, all formats, metadata). No CPAL dependency.
- **Realtime I/O (M4+):** CPAL with platform backends (ALSA/PipeWire/JACK on Linux, WASAPI/ASIO on Windows, CoreAudio on macOS).
- **Realtime DSP (M4+):** dasp / fundsp / rubato — RT-safe, zero-allocation.
- **Engine never decodes files.** Library provides pre-decoded f32 buffers + analysis.

**No realtime-audio implementation in M0.** This is boundary definition only.

---

## 7. GUI

Established by ADR-001. **Prototype-first, evaluate-second**:

1. Prototype in `egui` / `eframe` — immediate mode gives natural virtualization for 100k–500k track lists; AccessKit mandatory since 0.35.
2. Evaluate `Slint` — declarative, built-in `ListView` virtualization, AccessKit default.
3. Defer: `Tauri 2` (unless web-tech team), `iced` (no stable a11y — disqualified if a11y is hard requirement), `GPUI` (stalled releases), `Xilem` (zero production users).

**Gates before final commitment:** 200k-row virtualized list at 60fps; AccessKit tree smoke test with NVDA/VoiceOver/Orca. Final framework choice deferred to M3.

---

## 8. Cross-Cutting Boundaries

From `gui-evaluation.md` §Cross-Cutting Decisions:

| Boundary | Rule |
|----------|------|
| GUI ↔ Persistence | GUI works with sync read pool; virtualized list → paginated SQL queries |
| GUI ↔ Audio | Library UI shows pre-computed waveforms/beatgrids; GUI never touches CPAL; waveform data = `Vec<f32>` (min/max per pixel) generated offline |
| Persistence ↔ Audio | Track metadata + analysis in SQLite; decoded audio NOT stored in DB — file references only; provenance tracks import source per field |

---

## 9. Decision Log

| ADR | Title | Status | Sol Review |
|-----|-------|--------|------------|
| [ADR-001](adr/001-gui-framework.md) | GUI framework (prototype-first) | Provisional | Required — suspended |
| [ADR-002](adr/002-persistence.md) | SQLite + WAL persistence | Provisional | Not required |
| [ADR-003](adr/003-metadata-representation.md) | Neutral domain model + provenance | Provisional | Required — suspended |
| [ADR-004](adr/004-crate-boundaries.md) | Crate layer separation | Provisional | Not required |

**Pending M1+:** ADR-005 (metadata schema), ADR-006 (analysis pipeline), ADR-007 (vendor adapters).

---

## 10. Open Decisions

These are deliberately **not** decided in M0. Recording them as open prevents premature commitment:

| Decision | Why Deferred | Evidence Gate |
|----------|-------------|---------------|
| Final GUI framework (egui vs Slint) | Needs 200k-row virtualization + a11y prototype test | ADR-001 gates |
| Metadata schema (Rust types, DB tables) | Needs neutral model + ecosystem mapping first | ADR-005 (M1) |
| Content evidence (hash vs fingerprint) | Library selection criteria not yet evaluated | M2 analysis spike |
| Workspace layout (workspace root, member paths) | M0-08 workspace task owns this | ADR-004 inherits |
| Engine crate name and exact DSP stack | M4+ scope | ADR-004 notes |
| OneLibrary writer compatibility | No public schema; empirical validation required | interoperability.md §Fixture D |
| Serato, Engine DJ adapter scope | Not in M0 research | vendor adapter planning |
| Bluetooth latency support | Unsuitable for DJ monitoring | Document as unsupported |
| ASIO on Windows | Proprietary SDK + LLVM | Opt-in feature only |

---

## 11. What Is NOT in Scope for M0

Per the acceptance criteria and `idea.md` MVP definition:

- **No M5 implementation** (DJ metadata schema) — this is architecture direction only.
- **No realtime-audio implementation** — boundary definition only (ADR-004, §6).
- **No GUI wiring** — prototype direction only (ADR-001).
- **No vendor database writing** — Rekordbox XML is a documented import path; OneLibrary/ANLZ writing requires empirical validation (interoperability.md §Fixture D/E).
- **No committed product Cargo manifest** — M0-08 workspace task owns the workspace bootstrap (development.md line 3).

---

*Generated by M0-07 integration. Sources cited inline; see `docs/research/` for full evidence.*
