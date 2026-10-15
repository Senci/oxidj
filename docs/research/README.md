# M0-06: GUI, Persistence & Audio/Platform — Research Index

**Task:** `t_6fd918a6` — M0-06 GUI, persistence and future platform evaluation
**Status:** Research complete — scoped Sol reviews completed; ADR-002 accepted; 500k-track admission gate still NOT READY
**Owner:** oxidj-2
**Date:** 2026-10-15

---

## Overview

This milestone evaluates three architectural boundaries before OxiDJ commits to implementation decisions in M3+. Each evaluation is sourced from primary 2025-2026 sources with explicit citations.

**No decision without scoped Sol review.**

---

## Deliverables

| Area | Document | Key Question |
|------|----------|--------------|
| **GUI** | [gui-evaluation.md](gui-evaluation.md) | Which cross-platform Rust GUI framework for dense virtualized lists + accessibility? |
| **Persistence** | [persistence-evaluation.md](persistence-evaluation.md) | SQLite vs alternatives for 100k–500k tracks with migrations, backups, provenance? |
| **Audio/Platform** | [audio-platform-evaluation.md](audio-platform-evaluation.md) | CPAL + decode/DSP boundaries; realtime isolation architecture? |

---

## Cross-Cutting Decisions

### 1. GUI ↔ Persistence
- GUI framework must work with `rusqlite` (sync) or `sqlx` (async) read pool
- **egui/Slint/Tauri** all compatible; no blocker
- Virtualized list → paginated SQL queries (`LIMIT/OFFSET` or keyset pagination)

### 2. GUI ↔ Audio
- Library UI (M3) shows waveforms, beatgrids, cues — **pre-computed by analysis crate**
- Realtime engine (M4+) consumes pre-decoded buffers — **GUI never touches CPAL**
- Waveform data: `Vec<f32>` (min/max per pixel) generated offline

### 3. Persistence ↔ Audio
- Track metadata + analysis results in SQLite
- Decoded audio **not stored in DB** — file references only
- Provenance tracks import source for every field (see persistence-evaluation.md)

---

## Recommendation Summary

### GUI: Prototype Order
1. **egui/eframe** — Immediate mode = natural virtualization; AccessKit mandatory since 0.35; fastest iteration
2. **Slint** — Declarative + design tool; built-in virtualization; AccessKit default
3. **Tauri 2** — If web tech team; system webview; excellent a11y via HTML

**Avoid:** iced (no stable a11y, no virtualization), GPUI (stalled releases), Xilem (zero production users)

### Persistence: SQLite, admission gated
- **Engine:** SQLite/WAL via bundled `rusqlite` + `rusqlite_migration`; `synchronous=FULL` on every writer with commit-success acknowledgement, conditional on the supported storage stack honoring sync.
- **Gate:** no persistence dependency enters the workspace until the executable 500k-track prototype passes.
- **Backups:** `.backup` API for snapshots; Litestream is optional replication, not backup strategy or guaranteed RPO. Verify restore and separately protect external assets.
- **Future sync:** revisit Turso/libSQL only when multi-device sync is required.

### Audio: CPAL + Symphonia, Engine Separated
- **Decode/Analysis (M0-M3):** Symphonia (pure Rust, all formats, metadata)
- **Realtime I/O (M4+):** CPAL with platform-appropriate backends
- **Realtime DSP (M4+):** `dasp`/`fundsp`/`rubato` — RT-safe, zero-allocation
- **Boundary:** Library crate provides pre-decoded f32 buffers + analysis; Engine consumes only

---

## Open Risks Requiring Sol Review

| Risk | Area | Severity | Review Gate |
|------|------|----------|-------------|
| Linux Wayland + NVIDIA WebKitGTK bugs (affects Tauri) | GUI | Medium | Prototype test |
| iced a11y timeline (6+ years stalled) | GUI | High if a11y required | Disqualify iced |
| GPUI release cadence (0 in 8.5 months) | GUI | Medium | Defer |
| WAL checkpoint lag under write bursts | Persistence | Low | Monitor + benchmark |
| Schema migration locking (single-writer) | Persistence | Low | Migration lock pattern |
| CPAL realtime priority on Linux (rtkit config) | Audio | Medium | Document + test |
| ASIO on Windows (proprietary SDK + LLVM) | Audio | Low | Opt-in feature only |
| Bluetooth latency (all platforms) | Audio | High for DJ | Document unsupported |

---

## Next Steps (Post-Review)

1. **Freeze** the executable SQLite 500k-track workload, fixtures, measurement accounting, semantic oracle and recovery matrix with the documented provisional budgets reviewed before run.
2. **Run** the prototype on the recorded Linux reference stack only after protocol readiness is verified; admit no dependency unless every criterion passes.
3. **Draft ADR-005** metadata schema after the gate, carrying ADR-003 field-relationship risk and import-edit-export evidence.
4. **Maintain** cross-platform compatibility as a separate verification requirement.

---

## Source Provenance

All evaluations cite primary sources with URLs and dates:
- **rust-gui-desktop-ecosystem-state** (2026-07-07 snapshot, reconciled 2026-07-10)
- **boringcactus 2025 survey** (2025-04-13)
- **Rust SME Cookbook** (2026)
- **botmonster.com / sesamedisk.com / daily.dev** SQLite production guides (2026)
- **rustfinity.com** Rust ORM comparison (2026-04-04)
- **CPAL / Symphonia / Rodio** official docs + DeepWiki (2026)
- **Litestream / Turso / LiteFS** official docs + Simon Willison (2025-2026)

---

## Kanban Context

- **Parent:** `t_7f6b85c3` (M0-01 routing) — completed
- **Child:** `t_da2deaf1` — waiting on this completion
- **Independent of:** M0-02 toolchain, M0-03 SoundCraft, M0-04 Mixxx, M0-05 OneLibrary

---

*Generated by oxidj-2 worker. All documents in `docs/research/`.*