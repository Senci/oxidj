# Cross-Ecosystem Metadata Semantics Mapping

**Status:** M0-07 integration — evidence map, not a schema or format definition.
**Owner:** OxiDJ lead
**Date:** 2026-10-15
**Scope:** Compare Mixxx, neutral OxiDJ, and OneLibrary/Rekordbox semantics across identity, location, BPM/tempo/beat maps, bars, cues/loops, and ratings/colors/playlists. Unknown fields and contradictions are flagged explicitly rather than smoothed.

Sources: `docs/research/mixxx-semantics.md`, `docs/research/interoperability.md`, and the wiki synthesis pages `concepts/mixxx-beatgrid-and-cue-semantics.md`, `concepts/onelibrary-rekordbox-interchange.md`, `entities/soundcraft.md`. OxiDJ columns are derived policy from those sources, not from any single vendor.

---

## Legend

| Marker | Meaning |
|--------|---------|
| **Adopted** | OxiDJ policy derived from evidence |
| **Mirrors** | Vendor behavior matches OxiDJ intent |
| **Adapted** | OxiDJ normalizes/extends the vendor field |
| **Open** | Unknown or requires empirical validation |
| **Lossy** | Mapping loses information; report, do not discard |

---

## 1. Track Identity

### Mixxx
- Beat source provenance: `ANALYZER` | `FILE_METADATA` | `USER` (mixxx-semantics.md findings 1).
- No universal cross-vendor ID; the legacy `beats.proto` has no external ID field (mixxx-semantics.md findings 1).
- Mixxx track identity mechanism (file path vs. content fingerprint) is not established by the reviewed sources.

### OxiDJ (neutral)
- Internal identity is independent of vendor IDs and file paths (ADR-003).
- Resolve identity against normalized path plus content evidence (mechanism not specified by the reviewed sources; see interoperability.md "XML track identity and paths").
- Source IDs/URIs retained as provenance, not as sole global identity.

### Rekordbox / OneLibrary
- XML `Location` URI is required; playlist entries reference tracks by `TrackID` or `Location`.
- No public guarantee for stable `TrackID` across imports, ID collision handling, or URI normalization/escaping/case sensitivity.
- OneLibrary: schema unknown; identity/key semantics unverified. `Content.path` (VARCHAR 255, unique) mirrors Rekordbox's file-path identity but no public contract states whether it is stable across re-exports.
- Legacy ANLZ: PQTZ beat entries are referenced by file, not a global ID.

### Unknowns
- Rekordbox `TrackID` stability across XML re-imports.
- OneLibrary internal identity model (no public schema).
- Whether path normalization is consistent across Rekordbox XML, ANLZ, and OneLibrary.

**Evidence:** interoperability.md rows "XML track identity and paths", "OneLibrary identity" (Oct 2025 sources); mixxx-semantics.md findings 1 (beat source provenance; no external ID).

---

## 2. Location / File Path

### Mixxx
- Absolute filesystem path to audio file.
- Path changes tracked via file-system watcher; relocation is a user operation.

### OxiDJ (neutral)
- Path relocation requires explicit reconciliation, not silent reassignment (ADR-003).
- Preserve source URI/path as provenance; store both original and resolved location.
- Path normalization rules (case sensitivity, symlinks, spaces, non-ASCII) tested per platform.

### Rekordbox / OneLibrary
- XML expects `file://localhost/` URIs; location is required.
- `Content.path` and `Content.fileName` in OneLibrary are filesystem strings.
- No public guarantee for URI normalization, escaping, or case sensitivity.
- Same-level playlist/folder name uniqueness enforced by rekordbox XML import.

### Unknowns
- URI encoding behavior on import (what normalization rekordbox applies).
- Case-sensitivity rules across platforms for XML vs OneLibrary.
- Symlink resolution behavior.

**Evidence:** interoperability.md rows "XML track identity and paths", "XML interchange"; pyrekordbox OneLibrary docs (content table schema).

---

## 3. BPM, Tempo, Beat Maps

### Mixxx
- Two representations: `BeatGrid` (BPM + first beat) and `BeatMap` (individual beat positions with `enabled` + source flags).
- BPM and phase/anchor are independent: phase shifts move the grid, BPM changes spacing.
- Beat positions are audio frame positions; sample rate participates in calculations.
- Effective BPM includes playback-rate adjustment; key lock decouples pitch from tempo.
- Manual "assume constant tempo" = uniform beat spacing; disabling shows raw analyzer data.
- Legacy proto has no time-signature field.

### OxiDJ (neutral)
- Preserve representation: constant vs variable tempo as distinct forms (do not flatten a variable map to average BPM).
- Store BPM, anchor (first downbeat position), phase, sample rate.
- Distinguish analyzed BPM from user-edited BPM; preserve source/provenance per field.
- Effective BPM (with rate adjustment) is a playback-time parameter, not the stored grid.

### Rekordbox / OneLibrary
- XML `TEMPO` entries: `Inizio` (start position), BPM, `Metro` (meter), `Battito` (beat-in-bar).
- Multiple tempo entries per track permitted.
- Legacy ANLZ `PQTZ`: beat-in-measure, BPM x 100, time in ms.
- OneLibrary `Content.bpmx100`: BPM x 100 integer.
- XML docs do not define grid interpolation, phase origin, precision, or variable-tempo authoring semantics.

### Unknowns
- Grid interpolation algorithm (how tempo changes between points).
- Phase origin convention (first beat sample position vs. audio start).
- Precision of BPM storage and rounding behavior.
- Whether `Battito`/meter is used for bar numbering or just beat grouping.

**Evidence:** mixxx-semantics.md findings on constant/variable beatgrid, BPM/phase independence; interoperability.md row "Beat data"; pyrekordbox OneLibrary docs (content.bpmx100).

---

## 4. Bars, Time Signature, Downbeats

### Mixxx
- The 2.5.6 legacy `beats.proto` and `Beats` API have no time-signature field.
- Do not infer bar 1 from beat 1 or assume 4/4.
- Detailed bar/meter persistence behavior in Mixxx is unverified.
- Historical "Beat and Bar Edit Workflow" wiki is non-normative 2020 design context.

### OxiDJ (neutral)
- Meter/time-signature is an imported metadata field, kept separate until corroborated.
- Downbeat position is the anchor for a constant-tempo grid; store explicitly.
- Do not infer 4/4 or bar numbering from beat index alone.

### Rekordbox / OneLibrary
- XML `TEMPO` has `Metro` (meter) and `Battito` (beat-in-bar).
- OneLibrary: no public mapping from `Metro`/`Battito` to a bar-line model.
- XML documentation does not define how `Metro`/`Battito` drive bar numbering in player UI.

### Unknowns
- Mixxx persisted meter/time-signature (not established by 2.5.6 sources).
- Rekordbox OneLibrary bar-line derivation from `Metro`/`Battito`.
- Downbeat annotation convention (which beat is beat-1-of-bar in each vendor).

**Evidence:** mixxx-semantics.md findings 4 (no time signature); mixxx-semantics.md invariants (do not infer bar 1); interoperability.md row "Beat data" (meter/beat-in-bar but no interpolation/phase semantics).

---

## 5. Cues and Loops

### Mixxx
- Cue types: `Invalid`, `HotCue`, `MainCue`, `Loop`, `Jump`, `Intro`, `Outro`, `N60dBSound` (audible-range, not user-visible).
- `CueInfo`: optional start/end (ms), optional hotcue index (0 is valid), label, color, flags.
- One main cue, one loop, 38 hotcues (capacity enforced; overflow is lossy).
- Locked-loop flag used in Serato import but NOT saved in Mixxx's database.
- Loop is a range (start + end), not a point.
- Quantize is behavior/policy, not an inherent rewrite of stored positions.

### OxiDJ (neutral)
- Keep point cues distinct from loop ranges.
- Preserve imported flags only when evidence shows they exist; do not assume DB round-trip retains them.
- Report capacity overflow (e.g., >38 hotcues) before import/export.
- No silent clamping or rounding of positions.

### Rekordbox / OneLibrary
- XML `POSITION_MARK`: types `Cue` | `Fade-In` | `Fade-Out` | `Load` | `Loop`; start, end, name, `Num` convention (hotcue/memory).
- XML does not specify color, quantization, cue ordering/limits, or all newer cue attributes.
- OneLibrary `Cue` table: `kind` (Cue=0, Fade-In=0, Fade-Out=0, Load=3, Loop=4 — ambiguous), `colorTableIndex`, `cueComment`, `isActiveLoop`, `beatLoopNumerator`, `beatLoopDenominator`, and many position encodings (`inUsec`, `outUsec`, `in150FramePerSec`, `inMpegFrameNumber`, `inDecodingStartFramePosition`, `inNumberOfSampleInBlock`, etc.).
- Multiple position encodings for the same in/out — no public spec for which is authoritative or how they interrelate.
- Rekordbox cue ordering and limits differ from Mixxx; Rekordbox→Mixxx mapping "may be lossy."

### Unknowns
- OneLibrary `Cue.kind` integer values beyond 0/3/4.
- Whether OneLibrary stores all position encodings in sync or treats some as derived.
- XML color representation (RGB? index? undocumented).
- OneLibrary cue index/limit semantics.
- Loop activation semantics ("is active loop" flag persistence across round-trips).

**Evidence:** mixxx-semantics.md findings 3 (cue types, locked-loop not saved); interoperability.md row "Cues and loops"; pyrekordbox OneLibrary docs (cue table schema).

---

## 6. Ratings, Colors, Playlists

### Mixxx
- Playlist tree (folders + playlists) in the library.
- Ratings: stored in CueInfo flags; 0–5 stars.
- Colors: on cues (HotCue/MainCue) and potentially playlists; Mixxx color format is internal.

### OxiDJ (neutral)
- Ratings: 0–5 scale, stored as a normalized field with provenance.
- Colors: store vendor-native representation (index, RGB, or name) plus an OxiDJ-normalized form. Do not lose unnamed colors.
- Playlists: ordered lists; folders are tree nodes; same-level name uniqueness is a vendor constraint, not an OxiDJ invariant.

### Rekordbox / OneLibrary
- XML: descriptive tags, rating, counts/dates, tonality, color per track.
- OneLibrary:
  - `Content.rating` (0–5), `Content.color_id` → `Color.color_id` + `Color.name`.
  - `Playlist` table: `playlist_id`, `sequenceNo`, `name`, `image_id`, `attribute`, `playlist_id_parent` (tree).
  - `PlaylistContent`: `playlist_id`, `content_id`, `sequenceNo` (ordering).
  - `PlaylistType`: PLAYLIST=0, FOLDER=1, SMART_PLAYLIST=4.
  - XML prohibits duplicate playlist/folder names at the same tree level.
  - Colors are stored in a separate `Color` table with name strings only — no RGB/hex observed in pyrekordbox models.

### Unknowns
- Rekordbox XML color encoding (RGB? index? palette? not specified in format PDF).
- OneLibrary `Color` table — whether it stores RGB values or only names (pyrekordbox models show only `name`).
- OneLibrary `attribute` field on Playlist/Cue/MyTag — semantics undocumented.
- Serato color mapping (different palette system).
- Playlist smart-list criteria and sort-order persistence.

**Evidence:** interoperability.md row "Key and analysis" (rating/color fields); pyrekordbox OneLibrary docs (content, playlist, color, playlist_content tables); mixxx-semantics.md (ratings in CueInfo flags).

---

## 7. Key and Analysis Provenance

### Mixxx
- Analysis can produce BPM/grid, key, ReplayGain, waveform overview.
- Analyzed values are not proof of user validation.
- Distinction between analyzer/file metadata/user provenance is explicit in the beat format.

### OxiDJ (neutral)
- Analysis results carry the analyzer name, version, and timestamp.
- Distinguish analyzer/file metadata/user provenance per field (ADR-003).
- Waveform beat lines, cue/hotcue markers, loop boundaries in UI are presentation overlays, not semantic metadata.

### Rekordbox / OneLibrary
- XML `Tonality` string for key.
- OneLibrary `Key` table: `key_id`, `name` — no scale/mode/camelot notation observed.
- Marketing lists BPM, key, cue points, beatgrids, waveform data as OneLibrary export content — field list only, not a schema.
- `Content.analysisDataFilePath`, `analysedBits`, `analysisDataUpdateCount` suggest analysis file references, not embedded values.

### Unknowns
- Rekordbox key notation (Camelot? circle-of-fifths? open key?).
- OneLibrary key table fields beyond `name` (scale, mode, camelot).
- Analysis file format and version inside OneLibrary (`analysisDataFilePath`).
- Waveform encoding and analysis versioning.

**Evidence:** mixxx-semantics.md findings 4 (analysis provenance); interoperability.md row "Key and analysis"; pyrekordbox OneLibrary docs (key, content tables).

---

## 8. Summary: Open vs. Closed Questions

| Dimension | Closed (evidence-backed) | Open (requires validation) |
|-----------|-------------------------|---------------------------|
| Identity | Internal ID independent of vendor; path+content evidence | Rekordbox TrackID stability, OneLibrary internal ID |
| Location | Normalized path + provenance; explicit reconciliation | URI normalization, case sensitivity, symlinks |
| BPM/Tempo | Preserve constant vs variable; BPM + phase + anchor | Grid interpolation, phase origin, precision |
| Bars | Meter imported, kept separate; don't infer 4/4 | Mixxx meter persistence, OneLibrary bar derivation |
| Cues/Loops | Point cues vs loops; overflow reported; flags evidence-gated | OneLibrary kind values, position encoding sync |
| Ratings/Colors | 0–5 rating; vendor-native color + normalized | OneLibrary Color RGB, attribute fields |
| Playlists | Ordered; tree folders; same-level name is vendor constraint | Smart criteria, sort persistence, color encoding |
| Key | String field with provenance | Camelot/mode notation, OneLibrary key fields |

**Key principle:** OxiDJ's neutral model is defined by what all three ecosystems agree on (or where they disagree), not by mirroring any single vendor. Where evidence is absent, the policy is to preserve and report, never to invent.

---

*Generated by M0-07 integration. Sources cited inline; see `docs/research/` index for full references.*
