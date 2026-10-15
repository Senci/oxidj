# M0-06 Claim Audit — Independent Source Verification

**Date:** 2026-10-15
**Auditor:** sova
**Scope:** `docs/research/gui-evaluation.md`, `persistence-evaluation.md`, `audio-platform-evaluation.md` (canonical checkout at `/home/senci/dev/oxidj`)
**Method:** Every load-bearing claim opened against its cited primary source. Verdicts: SUPPORTED / PARTIAL / UNSUPPORTED / UNVERIFIED.

---

## 1. Claim Verdict Table

### GUI Evaluation (`gui-evaluation.md`)

| # | Claim (quoted) | Source file:line | Cited source | Verdict | Evidence passage | Note |
|---|---|---|---|---|---|---|
| G1 | "AccessKit (mandatory 0.35+)" | gui-evaluation.md:31 | eframe Cargo.toml | **UNSUPPORTED** | `default = ["accesskit", "clipboard", ...]` and `accesskit = ["egui-winit/accesskit"]` — accesskit is a **default** feature, disableable with `default-features = false` | The Cargo.toml comment says "Enable platform accessibility API implementations" — "Enable" implies optional. GoldStrikeArch stack-rows.md also says "mandatory dep since 0.35" but this is contradicted by the actual manifest. |
| G2 | "Mandatory dep since 0.35; schema types required by core" | gui-evaluation.md:72 | PR #2294, eframe Cargo.toml | **UNSUPPORTED** | eframe Cargo.toml (master, v0.36.2): `accesskit` is in the `default` features list, not a hard `[dependencies]` entry. PR #2294 is the original *optional* integration. | Internal conflict in GoldStrikeArch: egui section says "mandatory dep of egui since **0.34**" (stack-rows.md:290,310), shared-infra matrix says "since **0.35**" (stack-rows.md:543). The eval doc picks 0.35. |
| G3 | "Gaps documented in egui (accesskit 0.24+): text-input widgets incompletely exposed (caret/braille #2895/#8732), RTL/BiDi essentially unsupported (#1317)" | gui-evaluation.md:79 | stack-rows.md | **UNSUPPORTED** | egui issue #2895 is titled **"eframe: Fix state persisting when exiting on Linux"** — about state persistence, NOT caret/braille. egui issue #1317 returns **404 Not Found** — does not exist. The caret/braille #2895/#8732 gaps are attributed to **Slint** in stack-rows.md:61. egui's actual a11y gaps (stack-rows.md:290): "live regions (#2647), labelled_by panics (#3647)". egui's BiDi issues (stack-rows.md:287): "NO bidi/RTL (#1016/#5069 open)". | Three separate errors: wrong issue topic, non-existent issue, wrong framework attribution. |
| G4 | "iced — No built-in virtualization; 11k lines → +313 MB RSS in ecosystem benchmark (goldstrikearch.github.io)" | gui-evaluation.md:55 | goldstrikearch.github.io | **PARTIAL** | goldstrikearch.github.io: "iced (cosmic-text) renders the corpus correctly, 0 fonts bundled, but has no virtualization (+313 MB at 11k lines)" | The +313 MB figure IS on the cited page, but the context is **text rendering** (cosmic-text corpus test), not list virtualization. The eval doc conflates text-rendering virtualization with list virtualization. The live-dashboard RSS for iced is 95 MiB (goldstrikearch runtime.csv). |
| G5 | "Frames dropped, entire UI feels sluggish" at 100k rows (r/rust) | gui-evaluation.md:55 | r/rust 1cth129 | **SUPPORTED** | r/rust: "Iced: frames are skipped, the entire UI is laggy, the CPU core where the UI is running jumps to 100%" | Minor wording difference ("laggy" vs "sluggish") but substantively identical. |
| G6 | "GPUI … no releases 8+ months" | gui-evaluation.md:33 | crates.io gpui | **SUPPORTED** | crates.io: gpui max_version 0.2.2, last updated 2025-10-22. At eval doc date (2026-10-15), gap is ~12 months. | The "8.5 months" figure (line 119) is stale from the July 2026 GoldStrikeArch snapshot. General claim of no recent releases is correct. |
| G7 | "Slint … GPL-3.0 / commercial" | gui-evaluation.md:29 | slint Cargo.toml | **SUPPORTED** | slint Cargo.toml: `license = "GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0"` | Simplified but correct. GoldStrikeArch stack-rows.md:54 adds detail: "Tri-license: GPLv3 \| Royalty-Free 2.0 … \| paid commercial". |
| G8 | "iced … Missing in stable — issue #552 (2020); PR #3111 open-unmerged; hecrj closed #3281 'I'll work on this myself'" | gui-evaluation.md:77 | stack-rows.md | **SUPPORTED** | stack-rows.md:153: "NONE in stable — zero accesskit in dep tree; issue #552 open since 2020-10; PR #3111 open draft; PR #3281 closed unmerged 2026-03-14 by hecrj ('Thanks! But I'll work on this myself.')" | Exact match. |
| G9 | "AccessKit mandatory since 0.35" | gui-evaluation.md:129 | eframe Cargo.toml | **UNSUPPORTED** | Same as G1/G2. eframe Cargo.toml shows accesskit as a default (disableable) feature. | Repeated claim, same evidence. |

### Persistence Evaluation (`persistence-evaluation.md`)

| # | Claim (quoted) | Source file:line | Cited source | Verdict | Evidence passage | Note |
|---|---|---|---|---|---|---|
| P1 | "Read throughput \| 50,000–100,000+ queries/sec" | persistence-evaluation.md:29 | botmonster.com, sesamedisk.com | **SUPPORTED** | botmonster.com: "one SQLite database handles 50,000+ reads per second"; sesamedisk.com: "Read throughput can exceed 100,000 queries per second on a single server" | Both sources confirm the range. |
| P2 | "Write throughput \| 10,000–50,000 transactions/sec" | persistence-evaluation.md:30 | botmonster.com, daily.dev | **SUPPORTED** | botmonster.com: "10,000+ writes per second"; sesamedisk.com (citing daily.dev): "10,000 and 50,000 write transactions per second" | Both sources confirm the range. |
| P3 | "Practical dataset limit \| ~1 TB" | persistence-evaluation.md:31 | daily.dev, sesamedisk.com | **SUPPORTED** | sesamedisk.com: "practical production limit at roughly 1 TB" and "performance degrades significantly once databases exceed 1 TB" | botmonster.com cites 281 TB theoretical max; the eval doc correctly uses the practical limit. |
| P4 | "Single-writer latency \| 0.1–1 ms per transaction" | persistence-evaluation.md:32 | botmonster.com | **SUPPORTED** | botmonster.com: "A typical INSERT or UPDATE transaction finishes in 0.1 to 1 millisecond on NVMe storage" | Exact match. |
| P5 | "Lock contention threshold \| ~20 concurrent writers" | persistence-evaluation.md:33 | Shivek Khurana benchmark | **PARTIAL** | Shivek Khurana article: "expect SQLITE_BUSY errors above ~10-20 concurrent writers" | Original source gives a **range** (10–20); the eval doc picks the upper bound (~20) as a single number. The sesamedisk.com summary says "lock errors become negligible for workloads under 20 concurrent writers" — closer to the eval doc's phrasing but still a range boundary. |
| P6 | "rusqlite \| 0.40+" | persistence-evaluation.md:79 | crates.io | **SUPPORTED** | crates.io API: rusqlite max_version 0.40.2, updated 2026-08-08 | — |
| P7 | "tokio-rusqlite \| 0.7+" | persistence-evaluation.md:80 | crates.io | **SUPPORTED** | crates.io API: tokio-rusqlite max_version 0.8.0, updated 2026-09-06 | — |
| P8 | "rusqlite_migration \| 2.6+" | persistence-evaluation.md:82 | crates.io | **SUPPORTED** | crates.io API: rusqlite_migration max_version 2.6.0, updated 2026-05-28 | — |
| P9 | "sqlx \| 0.9+" | persistence-evaluation.md:83 | crates.io | **SUPPORTED** | crates.io API: sqlx max_version 0.9.0, updated 2026-05-21 | — |
| P10 | "SQLite 3.35+ (2021+) supports ALTER TABLE DROP COLUMN" | persistence-evaluation.md:122 | SQLite docs | **SUPPORTED** | SQLite 3.35.0 (2021-03-12) added `ALTER TABLE DROP COLUMN`. Well-documented SQLite feature. | — |
| P11 | "full ALTER TABLE on 3.46+ (2024)" | persistence-evaluation.md:122 | SQLite docs | **UNVERIFIED** | SQLite 3.46.0 (2024-05-23) release notes mention PRAGMA optimize improvements, not ALTER TABLE features. The claim is vague — "full ALTER TABLE" is not a defined SQLite feature. | The eval doc should specify what "full ALTER TABLE" means. SQLite ALTER TABLE has always been limited (can't modify column types, can't add CHECK constraints without table rebuild). |
| P12 | "Litestream \| ✅ Active (v0.5+ LTX format)" | persistence-evaluation.md:137 | litestream.io | **SUPPORTED** | litestream.io: "**v0.5.x** - Latest - Actively maintained with new features and bug fixes" | — |
| P13 | "SQLite .backup API \| Online snapshot via sqlite3_backup_init()" | persistence-evaluation.md:140 | sqlite.org/backup.html | **SUPPORTED** | sqlite.org/backup.html documents `sqlite3_backup_init()` / `sqlite3_backup_step()` for online backup | — |
| P14 | PRAGMA settings (WAL, synchronous=NORMAL, cache_size, mmap_size, busy_timeout, foreign_keys, journal_size_limit, auto_vacuum) | persistence-evaluation.md:41-48 | sqlite.org/pragma.html, sqlite.org/wal.html | **SUPPORTED** | sqlite.org/pragma.html documents all listed PRAGMA settings; sqlite.org/wal.html confirms WAL mode behavior | — |

### Audio Evaluation (`audio-platform-evaluation.md`)

| # | Claim (quoted) | Source file:line | Cited source | Verdict | Evidence passage | Note |
|---|---|---|---|---|---|---|
| A1 | "CPAL 0.18+" | audio-platform-evaluation.md:32 | crates.io | **SUPPORTED** | crates.io API: cpal max_version 0.18.2, updated 2026-08-16 | — |
| A2 | "Symphonia \| ✅ Active (0.6.0 May 2026)" | audio-platform-evaluation.md:127 | crates.io | **SUPPORTED** | crates.io API: symphonia 0.6.0 created 2026-05-15; 0.6.1 created 2026-08-13 | Version and date correct. Latest is 0.6.1. |
| A3 | Symphonia TOML: `features = ["ogg", "wav", "flac", "mp3", "vorbis", "opus", "isomp4", "mkv", "aiff", "caf", "aac", "alac", "adpcm", "metadata"]` | audio-platform-evaluation.md:147-152 | symphonia Cargo.toml | **UNSUPPORTED** | symphonia Cargo.toml features: `aac, adpcm, alac, caf, flac, isomp4, mkv, mp1, mp2, mp3, ogg, pcm, aiff, wav`. **No `opus` feature. No `metadata` feature.** Metadata is a separate crate (`symphonia-metadata`), not a feature flag on the umbrella crate. | Two invalid feature names in the TOML example. `opus` does not exist; `metadata` is a separate crate. |
| A4 | "Gapless playback support (FLAC, ALAC, MP3, Vorbis, PCM, ADPCM)" | audio-platform-evaluation.md:138 | symphonia README | **PARTIAL** | symphonia README gapless table is about **formats** (demuxers): AIFF (Yes), CAF (No), ISO/MP4 (No), MKV/WebM (No), OGG (Yes), Wave (Yes). README note: "Gapless playback requires support from both the demuxer and decoder." | The eval doc lists **codecs** (FLAC, ALAC, MP3, Vorbis, PCM, ADPCM) as having gapless support, but the README table shows gapless status for **formats** (containers), not codecs. The eval doc conflates formats and codecs. |
| A5 | "Metadata/tag reading (symphonia-metadata crate)" | audio-platform-evaluation.md:137 | symphonia Cargo.toml | **SUPPORTED** | symphonia Cargo.toml: `[dependencies.symphonia-metadata] version = "0.5.5"` | — |
| A6 | "Pure Rust, no FFI to FFmpeg" | audio-platform-evaluation.md:139 | symphonia README | **SUPPORTED** | symphonia README: "100% safe Rust" | — |

---

## 2. Claims That Must NOT Be Treated as Load-Bearing Until Re-Sourced

1. **egui AccessKit "mandatory since 0.35"** (G1, G2, G9) — The eframe Cargo.toml shows accesskit as a **default** (disableable) feature, not a mandatory dependency. The GoldStrikeArch source itself is internally inconsistent (0.34 vs 0.35). This claim affects the GUI framework decision's accessibility assessment.

2. **egui a11y gaps #2895/#8732 (caret/braille) and #1317 (RTL/BiDi)** (G3) — egui issue #2895 is about state persistence on Linux, not accessibility. Issue #1317 does not exist (404). The caret/braille gaps belong to **Slint** per the GoldStrikeArch source. egui's actual a11y gaps are #2647 (live regions) and #3647 (labelled_by panics). This misattribution could lead to wrong accessibility risk assessment.

3. **Symphonia feature flags "opus" and "metadata"** (A3) — These features do not exist in the symphonia umbrella crate. Any Cargo.toml using them will fail to compile. This affects the audio decode implementation plan.

4. **"Full ALTER TABLE on 3.46+"** (P11) — Vague and unverified. SQLite 3.46.0 did not add ALTER TABLE features. The eval doc should specify what "full ALTER TABLE" means or remove the claim.

5. **iced "+313 MB RSS" attributed to list virtualization** (G4) — The +313 MB figure is from a **text rendering** corpus test (cosmic-text), not list virtualization. The eval doc conflates the two contexts. The live-dashboard RSS for iced is 95 MiB.

---

## 3. Source Provenance Strength Ranking

| Rank | Source | Type | Strength | Notes |
|---|---|---|---|---|
| 1 | crates.io API | Primary registry | **Strong** | Authoritative for crate versions, release dates, feature flags. |
| 2 | GitHub raw Cargo.toml | Primary source code | **Strong** | Authoritative for dependency and feature definitions. |
| 3 | GitHub issue/PR API | Primary source code | **Strong** | Authoritative for issue titles, PR content, merge status. |
| 4 | sqlite.org documentation | Official docs | **Strong** | Authoritative for SQLite behavior, PRAGMA, WAL, backup API. |
| 5 | litestream.io | Official product docs | **Strong** | Authoritative for Litestream version and status. |
| 6 | GoldStrikeArch stack-rows.md | Secondary research | **Medium** | Self-described "done by using AI a lot" disclaimer. Contains internal conflicts (0.34 vs 0.35). Useful for ecosystem landscape but not for precise version/feature claims. |
| 7 | botmonster.com | Blog/analysis | **Medium** | Performance numbers plausible and consistent with other sources, but no raw benchmark data published. |
| 8 | sesamedisk.com | Blog/analysis | **Medium** | Cites daily.dev and Shivek Khurana; performance numbers consistent with botmonster.com. |
| 9 | r/rust thread 1cth129 | Community anecdote | **Weak** | Single-user benchmark, no controlled methodology. Useful for directional signal only. |
| 10 | boringcactus 2025 survey | Opinion survey | **Weak** | Author's personal experience, Windows-biased, no controlled methodology. |
| 11 | notgull.net burnout post | Personal blog | **Weak** | First-person account, no independent verification. |

---

## 4. Summary

| Verdict | Count | Claims |
|---|---|---|
| **SUPPORTED** | 18 | G5, G6, G7, G8, P1–P4, P6–P10, P12–P14, A1, A2, A5, A6 |
| **PARTIAL** | 3 | G4, P5, A4 |
| **UNSUPPORTED** | 4 | G1, G2, G3, G9, A3 |
| **UNVERIFIED** | 1 | P11 |

**Critical findings:** The egui AccessKit "mandatory" claim (G1/G2/G9) is the most consequential error — it is contradicted by the actual eframe Cargo.toml and affects the GUI framework accessibility assessment. The egui a11y gap misattribution (G3) is the second most serious — it assigns Slint's accessibility gaps to egui and references a non-existent issue. The Symphonia invalid feature flags (A3) would cause compile failures if used as-is.
