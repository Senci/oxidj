# OxiDJ

Live project site: <https://senci.github.io/oxidj/>. Artist profile: [Senči on SoundCloud](https://soundcloud.com/s3nci).

OxiDJ is a modern, open-source DJ library manager and performance application written primarily in Rust. Its long-term goal is a credible Rekordbox alternative without locking users into one vendor ecosystem (idea.md). The local catalog, import, analysis, provisional library UI and two-deck playback are implemented; broader vendor, hardware and platform qualification remains open.

## Status

M0 and the bounded local M1–M3 scopes are complete; M4/M7 interoperability and M6 analysis remain in progress. The M8 two-deck UI and audio engine are provisional, not a production or hardware compatibility claim. See [project status](plan/STATUS.md) and [audio handoff](plan/AUDIO.md). The separate 500k-track/60fps qualification remains unpassed.

**What exists today** (all local, Linux-first; synthetic and bounded real-library evidence only — see plan/STATUS.md):

- `oxidj-core` — neutral domain types, plus `WaveformPeaks` (3-band u8 peaks with a 2x mipmap) and a no-clobber file publish helper.
- `oxidj-analysis` — streaming 3-band waveform builder (fixed 2-pole crossover, fail-closed on bad input).
- `oxidj-library` — local SQLite catalog (versioned schema, 10,000-track cap), reference-only WAV/FLAC/MP3/AIFF/AIFF-C import (AIFF via a hand-rolled parser: 8/16/24/32-bit PCM, AIFF-C `NONE`/`sowt`/`fl32`/`fl64`; M4A/AAC, Ogg, ALAC and other AIFF-C codecs are refused as unsupported), persisted computed waveforms and separate vendor-derived waveforms.
- `oxidj-rekordbox` / `oxidj-onelibrary` — isolated adapters: Rekordbox XML preview, no-edit export and guarded edits; read-only DeviceSQL PDB, ANLZ (beat grid, cues, waveforms) and OneLibrary previews.
- `oxidj-application` / `oxidj-cli` — orchestration and commands, including `catalog import-library`, `catalog export-library-xml`, `catalog analyze-waveform` and `catalog analyze-tempo`.
- `prototypes/oxidj-library-ui` — provisional egui library UI: dense shell with a full-width track table and toggleable navigation / details / decks panels, guarded XML editing, and a smooth anti-aliased mono / 3-band / RGB waveform view with live per-band display gain and bar/beat lines from an imported library beat grid, or, when none matches, from an OxiDJ-computed grid (taktus; a suggestion, not Rekordbox parity). Since M8 it also has two playable decks (see below).
- `oxidj-audio` — realtime CPAL engine with two decks, key lock, SYNC, optional mixer, output selection and recovery, limiter, JACK support, measured output status and headphone cue routing. A muted synthetic JACK run at 48 kHz/256 frames lasted 600.1 s with 0 reported xruns/errors; physical USB reconnect and four-channel cue, Windows ASIO and macOS audio remain untested. Club gear, CDJ/XDJ playback and a latency guarantee are NOT DEMONSTRATED (ADR-008, plan/AUDIO.md).
- `xtask` — the repo-owned quality gate: `cargo xtask ci` (Tier 1) and `cargo xtask ci-full` (Tier 1 + tests + UI tests + licence gate, used by CI).
- Accepted ADRs and the architecture baseline (docs/adr/index.md, docs/architecture.md).

**Deferred** — do not expect these yet (idea.md MVP §Deferred; docs/architecture.md §11; plan/MVP.md):

- M4/M7: complete Rekordbox/OneLibrary interoperability remains unqualified. An owner-run Rekordbox Windows round trip provides bounded evidence, not broad vendor or hardware compatibility.
- The egui library UI is provisional; production accessibility, native platforms and representative-scale performance remain open.
- Beyond M8: effects, stems, streaming, broad controller support and production-grade latency qualification remain future work.

## Install (Linux, no sudo)

`scripts/install-local.sh` builds a release and installs, for the current user only, `oxidj-cli`, `oxidj-library-ui`, an `oxidj-library` launcher (opens the library at `~/.local/share/oxidj/library.sqlite`, or `$OXIDJ_DB`; created when missing), a desktop entry and an icon under `~/.local`. Re-run it after any change; it never touches library data. Import music with `oxidj-cli catalog import --dir <folder> --db ~/.local/share/oxidj/library.sqlite`. Provisional prototype: see [Status](#status) for maturity.

## Seed a library from a Rekordbox USB stick

Local, read-only on the stick; counts and files stay on your machine. Order matters (the UI lists tracks from the audio import step):

The UI's **More → Import Rekordbox PDB…** imports into an explicitly named **new** catalog after review/confirmation; it never merges or switches the current catalog. Both UI and CLI require a destination on verified local, non-removable storage, outside the source device. Linux admission uses util-linux `findmnt` and `lsblk` (including backing-device ancestry); USB, removable/hotplug disks, network/FUSE/overlay storage, missing tools, unknown topology and unsupported platforms are refused before staging. Choose an internal disk directory (or local tmpfs); source layout does not determine destination safety. Real removable-filesystem/hardware qualification remains separate.

1. `oxidj-cli catalog import-library --pdb <stick>/PIONEER/rekordbox/export.pdb --catalog new.sqlite` (library tracks, playlists, beat grids, cues, vendor waveforms), then move it to `~/.local/share/oxidj/library.sqlite` (keep the old file).
2. `oxidj-cli catalog import --dir <copy of the stick's Contents> --db ~/.local/share/oxidj/library.sqlite` (audio files; library paths are device-relative, matched by `/Contents/` suffix).
3. `oxidj-cli catalog analyze-waveform --db …` then `analyze-tempo --db …` (about 1.5 h for ~1,300 tracks; AIFF, MP3, WAV, FLAC decode in-house). A Rekordbox grid always wins over a computed one.

Do not open the UI on a catalog while an analysis job writes to it. `catalog create-library-playlist` (IDs from `catalog list-library-tracks`) and `export-library-xml --computed-grids` are described under the export section.

## Keys (library UI)

`Ctrl+,` Settings · `Ctrl+B` / `Ctrl+I` / `Ctrl+D` navigation / details / deck area · `A` / `B` load the selected track to deck A / B · `Space` play/pause (active deck) · `Q` / `P` cue deck A / B · `1`–`8` hot cues · `Tab` switch active deck. Every deck-area section (detail waveform, overviews, control strip, mixer) collapses with its small icon; the mixer is hidden by default and bypassed bit-exactly while hidden. The Omarchy launcher entry is a user-side setup: `~/.config/omarchy/extensions/omarchy-menu.jsonc`, key `oxidj`, action `uwsm-app -- oxidj-library`.

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

## Create an ordered playlist in an imported catalog

List imported tracks locally and select the `ExportedTrackID` values for the new
playlist (no manual XML parsing is needed):

```sh
oxidj-cli catalog list-library-tracks --catalog lib.sqlite
oxidj-cli catalog create-library-playlist --catalog lib.sqlite --name 'New set' --track-ids 3,1,3
oxidj-cli catalog append-library-playlist-members --catalog lib.sqlite --playlist-name 'New set' --track-ids 2,1
oxidj-cli catalog reorder-library-playlist-members --catalog lib.sqlite --playlist-name 'New set' --track-ids 1,2,3,1,3
oxidj-cli catalog remove-library-playlist-member --catalog lib.sqlite --playlist-name 'New set' --expected-track-ids 1,2,3,1,3 --index 0 --expected-track-id 1
oxidj-cli catalog rename-library-playlist --catalog lib.sqlite --playlist-name 'New set' --new-name 'Friday set'
oxidj-cli catalog export-library-xml --catalog lib.sqlite --output with-playlist.xml --drive D
```

**Privacy:** `list-library-tracks` deliberately prints personal titles and artists
on stdout. View it only locally; do not log, upload, transmit or paste real output
into task threads. The command itself does not log metadata or send it anywhere.
It opens an existing current-schema catalog read-only (no creation or migrations)
and never edits the source PDB/ANLZ, catalog records or XML files.

Output is tab-separated with header `ExportedTrackID`, `Title`, `Artist`, in
imported order. Within fields, backslash, TAB, LF and CR are escaped as `\\`,
`\t`, `\n`, `\r`; quotes and Unicode remain literal. Missing artist is `\N`,
an empty artist is an empty field, and a literal `\N` is printed as `\\N`.
An empty imported library prints the header followed by `(no imported tracks)`.
Only imported library tracks are listed, not separately imported local audio.

IDs here are **this catalog's exported XML TrackIDs** (imported track position + 1),
not local-audio `TrackId`s, original PDB IDs, or IDs from another catalog/XML file.
The command commits one new root playlist after all existing root nodes; member
order and duplicates are retained. It refuses an empty membership, malformed,
zero or missing track IDs, blank/XML-invalid names, and any existing same-name
root folder or playlist (including an identical retry). Success prints only
`CreatedPlaylists: 1` and the membership count, after commit. The catalog must
already exist and contain imported tracks; a missing path is never initialized.

This edits **only the OxiDJ catalog**, not the source PDB/ANLZ, vendor database,
media files, or an existing XML. Export always requires a new output filename.
The synthetic import → create → fresh-process export regression verifies the
original library plus exactly one final playlist. This operation has not been
verified in Rekordbox or on hardware; the historical round trip above is not
qualification of playlist creation. ADR-005/A3, accessibility/platform and scale
gates remain unverified. GUI playlist management is not included.

`catalog remove-library-playlist-member` removes exactly one occurrence using a zero-based
index, its expected exported TrackID, and the complete current TrackID order. Any stale
order or mismatched selection is refused. Other occurrences and playlists are unchanged.

`catalog reorder-library-playlist-members` takes the complete desired sequence of
exported XML TrackIDs (imported catalog track position + 1), not vendor or audio
IDs. It accepts only a permutation of the existing root playlist members,
including duplicate counts, and changes their order atomically.

To append members to one existing root playlist, use its exact unique name with
`catalog append-library-playlist-members --catalog lib.sqlite --playlist-name
'New set' --track-ids 2,1`. IDs remain the exported XML TrackIDs shown by
`list-library-tracks`; membership appends in supplied order and retains
duplicates. Nested playlists are not targets. Refusals (missing catalog/import,
invalid IDs or flags, absent/ambiguous root target) leave catalog membership
unchanged. Output is counts-only; never paste personal track names or other
catalog metadata into task logs.

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
