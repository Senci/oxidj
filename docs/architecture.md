# OxiDJ Architecture Baseline

**Status:** ACCEPTED — M0 architecture baseline, after the M0-07 Sol reviews completed and the architecture ADRs (ADR-001–004, ADR-008, ADR-009) transitioned to `Accepted` (see `docs/adr/index.md`). ~~Previously marked `PROVISIONAL (M0-07) — pending independent review`.~~
**Lead:** OxiDJ (oxidj-2)
**Date:** 2026-10-15
**MSRV:** Rust 1.99.0 — declared as `rust-version` in `Cargo.toml` and `msrv` in `clippy.toml`; reproduced by the `rust-toolchain.toml` pin. ~~Previously "pinned, not MSRV"~~ (M0-10 §7.4: declaring the MSRV is what makes Clippy's `msrv` lint and `[lints]` edition groups meaningful). See `docs/development.md`.

~~This document records the **provisional** M0 architectural direction. Per the M0-07 integration gate and the Nous-only free-model routing policy, all directions were evidence-backed recommendations pending independent review; no decision was Accepted architecture until its review gate was restored and completed.~~ **This document records the M0 architecture baseline** now that the M0-07 independent Sol reviews are complete and the architecture ADRs are `Accepted`. The baseline still distinguishes *accepted architectural direction* (the ADRs) from *implementation preconditions* (e.g. ADR-002's 500k-track prototype gate before the persistence dependency enters the workspace) and *deferred choices* (listed in [Open Decisions](#open-decisions), unresolved until M1+). Decisions are captured in [ADRs](adr/).

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

> **ADR-004 review outcome:** the fixed crate inventory was rejected. M0 contains only `oxidj-core` and `oxidj-cli` (plus the `xtask` build tool); the durable deliverable is the **dependency-rule gate** (`cargo xtask layers`), which fails on a forbidden edge and stays meaningful as crates are added in M1+. Empty scaffolds enforce no boundary — create a crate only when its implemented responsibilities warrant isolation.

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
- **Backups:** ~~Litestream (WAL streaming to S3/GCS) + `.backup` API for CI.~~ **Corrected by the ADR-002 review:** Litestream is *optional* asynchronous replication (RPO ~1s), **not** the backup strategy — the `SQLite .backup` API covers ad-hoc/CI. Verified restores and separate protection of external assets (audio files, artwork) are still required.
- **Abstraction:** ~~SQL behind a trait to enable future PostgreSQL/libSQL migration.~~ **Corrected by the ADR-002 review:** **domain-facing repository interfaces** rather than a generic SQL-portability trait; add broader portability only when a second backend creates a concrete requirement.
- **Dependency admission:** the selected dependencies (`rusqlite`, `rusqlite_migration`) must **not** enter the workspace until the 500k-track prototype gate passes — acceptance of the decision is separate from dependency admission.

**Dataset scale:** 500k tracks ≈ 50–100 MB — well within SQLite's comfort zone. Migration threshold: >1TB + 2+ other signals (write contention, multi-process writes, >10K TPS).

---

## 6. Audio Architecture

Established by ADR-004 (crate isolation — audio/analysis separate from the realtime engine) and ADR-008 (the M0 **data boundary**). ADR-008 was accepted as a **candidate foundation, not a production qualification** — CPAL + Signalsmith Stretch at medium confidence, because *«CPAL is a reasonable Rust-first default, not proof of DJ-grade performance»*. Rubber Band is excluded on two independent grounds (ADR-008 §Licensing resolution; ADR-009): GPL-2.0+/commercial licence **and** only 208 lifetime Rust-binding downloads.

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

- **Decode/analysis (M0-M3):** Symphonia (pure Rust, all formats, metadata). No CPAL dependency in the analysis crate.
- **Realtime I/O (M4+):** CPAL with platform backends (ALSA/PipeWire/JACK on Linux, WASAPI/ASIO on Windows, CoreAudio on macOS).
- **Realtime DSP (M4+):** a replaceable time-stretch trait (Signalsmith Stretch first) backed by `dasp`/`fundsp`/`rubato` — RT-safe, zero-allocation.
- **Engine never decodes files.** The library provides pre-decoded f32 buffers + analysis.

**M0 audio boundary (ADR-008):** the actionable output of the review — what M0 must define so later engine work need not rework the library/analysis crates:

1. Separate library/analysis data from engine/device types — stable track identities, source-frame positions, versioned beat-grid and analysis units, and an explicit conversion to a shared engine-frame clock.
2. Bounded, timestamped commands between application and engine.
3. Preallocated PCM transfer, channel routing, and defined underrun behaviour.
4. Off-callback decoding and device recovery — never in the audio thread.
5. Sample-accurate deck scheduling on the shared clock, accounting for stretch latency.
6. Keep both I/O and stretch replaceable.
7. Document Bluetooth output as unsupported.

Signalsmith Stretch (MIT) is the first implementation behind the replaceable trait, but its transient/scratching quality and worst-case callback cost are **unmeasured** on DJ material. **No realtime-audio implementation in M0.** This is boundary definition only; latency/xrun targets, multichannel routing and device-recovery behaviour remain unmeasured and are carried forward as open risks (ADR-008 §Open risks).

---

## 7. GUI

Established by ADR-001. **Prototype-first, evaluate-second**:

1. Prototype in `egui` / `eframe` — immediate mode gives natural virtualization for 100k–500k track lists. ~~AccessKit mandatory since 0.35.~~ **Corrected (M0-06 claim audit, recorded in ADR-001 §13):** `accesskit` is a *default, disableable* feature of `eframe`, not a hard dependency — accessibility is available and must be explicitly enabled and verified, not assumed. MIT (ADR-009) fired ADR-001's conditional constraint, so **egui/eframe is the prototype framework**.
2. ~~Evaluate `Slint`~~ *(commercially licensed fallback only)*: declarative, built-in `ListView` virtualization, AccessKit default. Slint's GPL-3.0/commercial terms are now decisive under MIT — it is **excluded at the free tier** and retained only as a fallback conditional on explicit purchase approval.
3. Defer: `Qt` (deferred on `cxx-qt` API-stability grounds, **not** licensing disqualification — Qt/LGPL is theoretically compatible with MIT), `Tauri 2` (unless web-tech team), `iced` (no stable a11y — disqualified if a11y is a hard requirement), `Xilem` (zero production users).

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
| [ADR-001](adr/001-gui-framework.md) | GUI framework — prototype-first (egui), Slint commercial-only, Qt deferred | **Accepted** | Lead (review low) |
| [ADR-002](adr/002-persistence.md) | SQLite + WAL persistence | **Accepted** | Lead (review medium) |
| [ADR-003](adr/003-metadata-representation.md) | Neutral domain model + provenance | **Accepted** | Lead (review high) |
| [ADR-004](adr/004-crate-boundaries.md) | Crate layer separation | **Accepted** | Lead (review high) |
| [ADR-008](adr/008-audio-engine.md) | Audio engine foundation — CPAL I/O, replaceable stretch | **Accepted** | Lead (review low) |
| [ADR-009](adr/009-licence-mit.md) | Project licence — MIT | **Accepted** | Project owner |

> ~~Previously listed as `Provisional`, pending independent review.~~ Superseded: the M0-07 Sol reviews are complete; the architecture ADR set (001–004, 008, 009) is `Accepted` and each carries tracked preconditions (see `docs/adr/index.md`).

**Reserved for M1+:** ADR-005 (metadata schema), ADR-006 (analysis pipeline), ADR-007 (vendor adapter scope). ADR-005/006/007 are intentionally reserved — the numbering gap between ADR-004 and ADR-008 is deliberate: 005/006/007 cover the M1+ schema, analysis-pipeline and adapter decisions M0 defers, while 008/009 were settled during the M0 review round.

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
- **No realtime-audio implementation** — boundary definition only (ADR-004 crate isolation + ADR-008 data boundary; architecture §6).
- **No GUI wiring** — prototype direction only (ADR-001; egui first).
- **No vendor database writing** — Rekordbox XML is a documented import path; OneLibrary/ANLZ writing requires empirical validation (interoperability.md §Fixture D/E).
- ~~No committed product Cargo manifest~~ **Superseded:** M0-08 committed the workspace manifest and the `cargo xtask` gate (`Cargo.toml` + `crates/oxidj-core` + `crates/oxidj-cli` + `xtask`); see `docs/development.md` §Quality gate.

---

## 12. Project Licence (ADR-009)

**Accepted.** OxiDJ is licensed under the **MIT License** (copyright Senad Ličina), decided by the project owner (ADR-009). Recorded as an architecture constraint because it settles the conditional constraints in ADR-001 (GUI) and ADR-008 (audio):

- **Permissive-only dependency policy, enforced.** `cargo deny check licenses` (allow-list: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2016, Zlib) runs **pre-merge** as a Tier-2 gate (rust-quality-gates.md §7.10); GPL/LGPL/AGPL/SSPL and every unlisted licence are denied by default via `deny.toml`. An accidental copyleft dependency would silently re-license the distributed binary, so this is a build gate, not a convention.
- **GUI framework.** Slint's GPL-3.0/commercial terms are now decisive — MIT rules out GPL-compatible distribution with no commercial licence purchased — so Slint is **excluded at the free tier** and egui/eframe becomes the prototype framework (ADR-001).
- **Time-stretch engine.** Rubber Band is excluded on **two independent grounds** — its GPL-2.0+/commercial licence is incompatible with MIT distribution **and** its Rust bindings (`rubberband` 0.1.5) have only 208 lifetime downloads, so the Rust path is unproven regardless of licence. Signalsmith Stretch (MIT) remains the first implementation behind ADR-008's replaceable trait.
- **Qt / cxx-qt.** Remains theoretically compatible with MIT (LGPL linking), but stays deferred on `cxx-qt` API-stability grounds, not licensing.

The only open option (ADR-009 §Open question): dual `MIT OR Apache-2.0` (Apache adds a patent grant) is a one-line change now and would need contributor agreement once outside contributions exist.

---

*Generated by M0-07 integration; project licence integrated per ADR-009 (2026-10-15). Sources cited inline; see `docs/research/` for full evidence.*
