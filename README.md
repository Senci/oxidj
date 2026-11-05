# OxiDJ

OxiDJ is a modern, open-source DJ library manager and, eventually, performance application written primarily in Rust. Its long-term goal is a credible Rekordbox alternative without locking users into one vendor ecosystem (idea.md). The bounded local reference catalog exists as an implementation candidate; import, analysis, playback, and GUI workflows remain absent.

## Status

M0 is verified complete. The named `local-catalog-alpha-v1` dependency exception is integrated by PR #13 (`671ae2b3df933ada97297bbba19ea1394657c14e`) for the reference-only, single-process, local-filesystem catalog capped at 10,000 tracks. A local implementation candidate now initializes and lists a versioned catalog, stores reference-only records through `oxidj-library`, and demonstrates fresh-process listing. Independent acceptance and full M1 remain incomplete (plan/STATUS.md). The separate representative 500k qualification remains NOT READY and unpassed.

**What exists today** (all local, Linux-first; synthetic and bounded real-library evidence only — see plan/STATUS.md):

- `oxidj-core` — neutral domain types, plus `WaveformPeaks` (3-band u8 peaks with a 2x mipmap) and a no-clobber file publish helper.
- `oxidj-analysis` — streaming 3-band waveform builder (fixed 2-pole crossover, fail-closed on bad input).
- `oxidj-library` — local SQLite catalog (versioned schema, 10,000-track cap), reference-only WAV/FLAC/MP3/AIFF/AIFF-C import (AIFF via a hand-rolled parser: 8/16/24/32-bit PCM, AIFF-C `NONE`/`sowt`/`fl32`/`fl64`; M4A/AAC, Ogg, ALAC and other AIFF-C codecs are refused as unsupported), persisted computed waveforms and separate vendor-derived waveforms.
- `oxidj-rekordbox` / `oxidj-onelibrary` — isolated adapters: Rekordbox XML preview, no-edit export and guarded edits; read-only DeviceSQL PDB, ANLZ (beat grid, cues, waveforms) and OneLibrary previews.
- `oxidj-application` / `oxidj-cli` — orchestration and commands, including `catalog import-library`, `catalog export-library-xml`, `catalog analyze-waveform` and `catalog analyze-tempo`.
- `prototypes/oxidj-library-ui` — provisional egui library UI: dense shell with a full-width track table and toggleable navigation / details / decks panels, guarded XML editing, and a smooth anti-aliased mono / 3-band / RGB waveform view with live per-band display gain and bar/beat lines from an imported library beat grid, or, when none matches, from an OxiDJ-computed grid (taktus; a suggestion, not Rekordbox parity).
- `xtask` — the repo-owned quality gate: `cargo xtask ci` (Tier 1) and `cargo xtask ci-full` (Tier 1 + tests + UI tests + licence gate, used by CI).
- Accepted ADRs and the architecture baseline (docs/adr/index.md, docs/architecture.md).

**Deferred** — do not expect these yet (idea.md MVP §Deferred; docs/architecture.md §11; plan/MVP.md):

- M1: bounded implementation candidate is under independent review; backup/restore, migration-failure, transaction-failure and complete M1 evidence remain open. Vendor DJ metadata schema remains proposed and unaccepted (ADR-005). Persistence dependencies for representative-scale qualification remain gated on the separate 500k-track route (ADR-002).
- M2: filesystem import (plan/MVP.md).
- M3: usable library UI — prototype direction only (egui/eframe, ADR-001), not committed to production.
- M4: complete Rekordbox/OneLibrary interoperability remains unqualified. XML preview, preflight, no-edit export, and guarded edits to selected tempo and position-mark fields cover a bounded synthetic subset only; no vendor application acceptance or undocumented-format compatibility is claimed (interoperability.md §Fixture D).
- M4+: realtime DJ functionality — decks, mixing, effects, stems, streaming, broad controller support (idea.md §Priorities/MVP).

## Install (Linux, no sudo)

`scripts/install-local.sh` builds a release and installs, for the current user only, `oxidj-cli`, `oxidj-library-ui`, an `oxidj-library` launcher (opens the library at `~/.local/share/oxidj/library.sqlite`, or `$OXIDJ_DB`; created when missing), a desktop entry and an icon under `~/.local`. Re-run it after any change; it never touches library data. Import music with `oxidj-cli catalog import --dir <folder> --db ~/.local/share/oxidj/library.sqlite`. Provisional prototype: see [Status](#status) for maturity.

## Export a USB library for Rekordbox on Windows

```sh
oxidj-cli catalog import-library --pdb /media/USB/PIONEER/rekordbox/export.pdb --catalog lib.sqlite
oxidj-cli catalog export-library-xml --catalog lib.sqlite --output rekordbox.xml --drive D
```

`--drive D` writes device-relative locations as `file://localhost/D:/Contents/…` (one letter A–Z, case-insensitive; other locations are unchanged; without it the export is unchanged). Opt-in `--computed-grids` gives tracks without a Rekordbox beat grid one constant `TEMPO` from OxiDJ's computed grid (run `catalog import --dir <copy of the stick> --db lib.sqlite` and `catalog analyze-tempo --db lib.sqlite` first; the local file must uniquely match the track's `/Contents/…` path and the analyser confidence must be at least 0.5); Rekordbox grids are never replaced, and the report prints counts only. Owner-verified in real Rekordbox on Windows (2026-11-04): the export loads all tracks with BPM, key and CUE markers. Steps:

1. Use the drive letter Windows gives the USB stick for `--drive`.
2. Copy the XML to a **local** drive — opening it from a network/UNC share fails with "unable to be read properly".
3. In Rekordbox, set Preferences > Advanced > rekordbox xml to that file and browse it under "rekordbox xml".

The owner has confirmed in Rekordbox that imported tracks, BPM, key, cues and beat grids (including tempo changes and a manual grid edit) display correctly; playlist import, playback and an edit-then-reimport round trip (two OxiDJ guarded edits) also work; hardware use (CDJ/XDJ) is not demonstrated (plan/STATUS.md M7).

The first major product proof — the MVP (idea.md) — is library-first: import a real DJ library, manage it safely in OxiDJ, export it again, and continue using compatible DJ software/hardware without unexpected metadata loss.

## Documentation

- [docs/index.md](docs/index.md) — curated guide to key project documents, grouped by purpose.
- [docs/glossary.md](docs/glossary.md) — DJ and domain vocabulary (beatgrid, cues, loops, provenance, vendor formats).
- [site/README.md](site/README.md) — the "FILE #0042" teaser site: what it is, and how to build and test it locally.
- [plan/STATUS.md](plan/STATUS.md) — current milestone, blockers, and next tasks.
- [AGENTS.md](AGENTS.md) — rules for agent contributors.

## Toolchain and quality gate

Toolchain: **Rust 1.99.0**, declared as the MSRV and pinned by `rust-toolchain.toml` (see docs/development.md §MSRV).

```sh
cargo xtask ci        # Tier 1: layers + fmt + clippy + check (every handoff must pass)
cargo xtask ci-full   # Tier 1 + tests + licence gate (what CI runs)
```

A fresh clone reproduces the gate with one command:

```sh
git clone <repo> && cd oxidj && cargo xtask ci
```

The gate is tiered: Tier 1 (stable, fast; no advisory scans) on every coding handoff; Tier 2 on CI/pre-merge; Tier 3 analyses are deferred and not yet wired to an in-repository schedule. Every handoff must paste real Tier-1 output (`docs/handoff-checklist.md`); current commands live in `cargo xtask` and `.github/workflows/ci.yml`.

## Licence

**MIT.** Copyright (c) 2026 Senad Ličina. See [LICENSE](LICENSE). The licence is decided by ADR-009 and enforced as a build gate: `cargo deny check licenses` with an allowlist of permissive licenses (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unicode-DFS-2016, Zlib) and GPL/LGPL/AGPL/SSPL denied by default (deny.toml).
