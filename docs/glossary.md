# Glossary — DJ and domain vocabulary

Terms OxiDJ depends on. Each entry is one or two sentences, citing the OxiDJ
document where the term matters. Where the ecosystems disagree about a term,
the disagreement is noted — it is the reason the neutral model exists
(semantic-mapping.md §Legend).

---

## Rhythmic structure

**beat** — the basic pulse of a track; the unit to which tempo (BPM) and the
beat grid are referenced. Beat positions in OxiDJ tie to audio-frame positions
and the source sample rate (semantic-mapping.md §3; mixxx-semantics.md findings
1–2).

**beatgrid** — a mapping of beats to time/sample positions in a track. OxiDJ
preserves the representation: a constant-tempo grid (uniform spacing from one
BPM + anchor) and a variable-tempo map (individual, independently-spaced beats)
are distinct and never flattened to an average BPM (semantic-mapping.md §3;
ADR-003 §Consequences; mixxx-semantics.md finding 1).

**constant tempo** — a beat grid in which all beats are spaced uniformly from a
single BPM and an anchor downbeat. Mixxx’s legacy `BeatGrid` representation.
OxiDJ stores it as a distinct form from variable tempo (semantic-mapping.md §3
OxiDJ; ADR-003 §Consequences; mixxx-semantics.md finding 1).

**variable tempo** — a beat grid in which beat positions are individual and may
vary in spacing. Mixxx’s `BeatMap` representation. Flattening a variable map to
a single average BPM is lossy and must be reported, not done silently
(semantic-mapping.md §3; ADR-003 §Consequences/Open Questions;
mixxx-semantics.md invariants).

**downbeat** — the first beat of a bar. OxiDJ stores the downbeat position as the
explicit anchor for a constant-tempo grid rather than inferring it from beat
index (semantic-mapping.md §4; ADR-003 §Consequences).

**bar** — a group of beats defined by the time signature (meter). OxiDJ keeps
meter/time-signature as an imported field and does not infer 4/4 or bar
numbering from beat index alone (semantic-mapping.md §4; ADR-003
§Open Questions; mixxx-semantics.md finding 2/invariants).

**phrase** — a higher-level musical grouping, conventionally 8 or 16 bars (32 or
64 beats at 4/4), used by DJs to align track sections for mixing. OxiDJ does
**not** model phrases yet: the neutral model covers beats, bars, and downbeats
but not phrase boundaries (semantic-mapping.md §4; ADR-003). Mixxx’s 2.5.6 proto
and CueInfo expose no phrase field (mixxx-semantics.md finding 1/§Legend), and
the ecosystems disagree on whether phrases are stored at all — OxiDJ’s policy is
to preserve and report, not to invent.

**BPM** — beats per minute; the tempo. OxiDJ distinguishes analyzed BPM from
user-edited BPM and preserves source/provenance per field; effective BPM
(including playback-rate adjustment) is a playback-time parameter, not the
stored grid (semantic-mapping.md §3; §7; ADR-003; mixxx-semantics.md finding 2).

---

## Cue and loop markers

**cue point** — a marked single position in a track (a point, not a range).
Rekordbox XML represents these as `POSITION_MARK` entries; OneLibrary marketing
lists “cue points” as an export field only, not a schema. OxiDJ keeps point cues
distinct from loops (semantic-mapping.md §5; interoperability.md “Cues and
loops”).

**hot cue** — a user-defined, indexed cue point for quick jumping. Rekordbox
XML uses the `POSITION_MARK` `Num` convention for hot-cue indices; OneLibrary’s
`Cue` table carries `colorTableIndex` and multiple position encodings. Mixxx
documents 38 hotcues with capacity enforced — overflow is lossy
(semantic-mapping.md §5; mixxx-semantics.md finding 3; ADR-003
§Consequences).

**loop** — a range with start and end points that repeats. OxiDJ keeps loop
ranges distinct from point cues. **Locked-loop caveat:** Mixxx’s `CueInfo` carries
a locked-loop flag used in Serato import/round-trip tests, but it is **not saved
in Mixxx’s database** — so imported flags are never assumed to survive a
round-trip (semantic-mapping.md §5; mixxx-semantics.md finding 3; ADR-003
§Open Questions).

**intro/outro markers** — boundary markers for a track’s intro (beginning) and
outro (end) sections. Mixxx’s cue-type list includes `Intro` and `Outro`;
Rekordbox XML `POSITION_MARK` instead uses `Fade-In` / `Fade-Out` — the
ecosystems disagree on the label (mixxx-semantics.md finding 3;
semantic-mapping.md §5).

---

## Musical key

**key** — the musical key of a track (e.g., C major). Rekordbox XML stores a
`Tonality` string; OneLibrary has a `Key` table (`key_id`, `name`). The exact
notation convention is **unverified** (semantic-mapping.md §7;
interoperability.md “Key and analysis”).

**Camelot notation** — the circle-of-fifths wheel (e.g., 8B for A minor) used
for harmonic mixing. OneLibrary’s `Key` table exposes only `name` — no scale,
mode, or Camelot fields were observed — and the Rekordbox key notation
(Camelot vs. circle-of-fifths vs. open key) is explicitly **unknown**
(semantic-mapping.md §7 §Unknowns; ADR-003 §Open Questions).

---

## Audio representation

**waveform** — a visual representation of a track’s amplitude over time. OxiDJ
generates waveform data as `Vec<f32>` (min/max per pixel) offline; the GUI never
touches CPAL and the realtime engine never decodes files (architecture.md §8;
ADR-004 §Consequences; audio-platform-evaluation.md §5).

**analysis** — automated extraction of BPM, beatgrid, waveform, and key.
Analysis results carry analyzer name, version, and timestamp; analyzed values
are not proof of user validation (semantic-mapping.md §7; architecture.md §6;
ADR-006, pending M1; audio-platform-evaluation.md §4).

---

## Metadata model

**provenance** — the recorded source of each metadata field
(`ANALYZER` | `FILE_METADATA` | `USER`, or a vendor import). Every field in the
neutral model records its source so origins are never lost silently
(ADR-003 §Consequences/Evidence; semantic-mapping.md §1, §3, §5; architecture.md
§4).

**vendor passthrough** — preserving unknown vendor metadata (fields, container
sections) through import and export without decoding or discarding them. Where no
documented passthrough exists, OxiDJ preserves the complete source artifact and
marks the field unsupported (interoperability.md “Vendor passthrough”, §3/5;
semantic-mapping.md Legend “Lossy” and §Legend; ADR-003 §Consequences;
architecture.md §2 principle 6).

**loss reporting** — reporting metadata that cannot be preserved during
import/export **before** writing, rather than silently discarding it.
OxiDJ’s principle is “no silent metadata loss”: unsupported or lost fields are
reported and original media is preserved until user approval
(idea.md §Principles; architecture.md §2 principle 6; ADR-003
§Consequences; interoperability.md §Interchange strategy 5; semantic-mapping.md
Legend “Lossy”).

---

## Vendor ecosystems

**OneLibrary** — AlphaTheta’s USB/SD library format for DJ performance data,
evolved from Device Library Plus with software/hardware partners (Algoriddim,
Native Instruments). No public wire-format or schema spec exists; OxiDJ treats
it as a compatibility target requiring empirical validation, not a documented
contract (interoperability.md “OneLibrary identity”, §Fixture D;
semantic-mapping.md §1, §3).

**Rekordbox** — Pioneer/AlphaTheta DJ software. Interoperability surface for
OxiDJ: the documented XML playlist/track interchange (via `Location` + `TrackID`)
and the reverse-engineered legacy `ANLZ`/`PDB` analysis files. OneLibrary is
Rekordbox’s successor format (interoperability.md; semantic-mapping.md §1, §3;
ADR-009).

**ANLZ/PDB (legacy)** — legacy Rekordbox analysis files: `anlz` (analysis data)
and `PDB` (device-library database). These are reverse-engineered formats (Deep
Symmetry / Crate Digger), **not** vendor-documented contracts; OxiDJ treats them
as a separate reverse-engineered adapter, not as OneLibrary. Legacy ANLZ `PQTZ`
beat entries are referenced by file, not a global ID
(interoperability.md “Legacy USB export internals”, “Legacy ANLZ”;
semantic-mapping.md §1).

**Mixxx** — an open-source DJ application (GPL-2.0-or-later). Interoperability
source for OxiDJ: the reviewed 2.5.6 semantics distinguish `BeatGrid` vs
`BeatMap`, expose cue types (`HotCue`, `Loop`, `Intro`, `Outro`, …), and carry
explicit per-field source provenance (`ANALYZER` | `FILE_METADATA` | `USER`)
(mixxx-semantics.md; semantic-mapping.md §1–6).

**Serato** — a DJ software/format. OxiDJ’s interoperability with Serato is
**not in M0 research** — its adapter scope is deferred to M1+
(architecture.md §10 “Open Decisions”; ADR-004 §Options). Mixxx’s locked-loop
flag is used in Serato import/round-trip tests, and Serato uses a different
colour palette system than OneLibrary (mixxx-semantics.md finding 3;
semantic-mapping.md §6 §Unknowns).

---

*OxiDJ distinguishes what all ecosystems agree on (or where they disagree) from
what no single vendor guarantees. Where evidence is absent, the policy is to
preserve and report, never to invent (semantic-mapping.md §8, §Summary).*
