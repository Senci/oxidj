# ADR-005 Relationship Fixtures

**Status:** Proposed — decision-bearing validation plan for ADR-005.
**Date:** 2026-10-15
**Owner:** OxiDJ lead
**Companion to:** `docs/adr/005-metadata-schema.md`

This document specifies concrete import→edit→export fixture cases that validate
the source-native records + neutral projection + edit overlays representation. Each
case is precise enough to implement as an automated test. The fixtures are
decision-bearing: if they expose vendor relationships that the proposed
representation cannot safely preserve, ADR-005 must be revised before schema
acceptance.

**No silent data loss.** Every fixture specifies exact expected outputs. If a
value cannot be preserved, the expected output is an explicit loss report or
conservative withholding — never a silent drop.

**No universal vendor claims.** Mixxx behavior is recorded as Mixxx behavior,
not as a universal standard. Rekordbox XML and OneLibrary behaviors are recorded
as their own. Where behaviors differ, the difference is documented.

---

## Conventions

### Source vendor formats

- **Mixxx**: `beats.proto` / `BeatMap` / `CueInfo` structures (constant-tempo
  BeatGrid with BPM + anchor; BeatMap with individual beat positions; CueInfo
  with type, start/end in ms, hotcue index, label, color, flags).
- **Rekordbox XML**: `POSITION_MARK` entries with `Type`, `Start`, `End`,
  `Num`, `Name`; `TEMPO` entries with `Inizio`, `BPM`, `Metro`, `Battito`;
  playlist tree with `TrackID` or `Location` keys.
- **OneLibrary**: `Cue` table rows with `kind`, `colorTableIndex`, `cueComment`,
  `isActiveLoop`, `beatLoopNumerator`, `beatLoopDenominator`, and multiple
  position encodings (`inUsec`, `outUsec`, `in150FramePerSec`, etc.).

### Provenance values

- `ANALYZED` — determined by OxiDJ analysis.
- `FILE_METADATA` — read from the audio file container.
- `USER` — explicitly set by the user via an edit overlay.
- `VENDOR:Mixxx` — imported from a Mixxx source-native record.
- `VENDOR:RekordboxXML` — imported from a Rekordbox XML source-native record.
- `VENDOR:OneLibrary` — imported from a OneLibrary source-native record.

### Relationship group identifiers

- `RG-BEATGRID-CUES` — beat grid and any cue/loop positions that are explicitly
  declared as beat-relative (e.g., beat-locked cues). In Mixxx, CueInfo positions
  are absolute milliseconds and do not reference the beat grid (mixxx-semantics.md
  lines 10–12).
- `RG-PLAYLIST-TRACK` — playlist membership and track identity.
- `RG-KEY-ANCHOR` — key notation and beat grid anchor.
- `RG-IDENTITY-LOCATION` — internal track identity and file location.

### Expected result markers

- **PASS** — the fixture demonstrates the specified behavior exactly.
- **FAIL** — the fixture demonstrates a violation of the specified behavior.
- **NOT DEMONSTRATED** — the fixture cannot determine the behavior from the
  available evidence; the gap is explicit and must be resolved before ADR-005
  acceptance.

---

## F-01: Stable track identity vs relocatable file location

**Source:** Mixxx
**Relationship group:** `RG-IDENTITY-LOCATION`

### Import fixture

Two Mixxx library entries pointing to the same audio file at different paths:

```
Track A: path = "/music/track1.flac", content_hash = "abc123..."
Track B: path = "/music/relocated/track1.flac", content_hash = "abc123..."
```

Both tracks have identical Mixxx metadata: BPM=128.0, anchor=0.5s, one hotcue
at 10.0s (index 0, label "Intro").

### Operation

1. Import Track A. Record internal identity `ID-1`.
2. Relocate the file on disk to `/music/relocated/track1.flac`.
3. Import Track B. The import adapter detects matching content evidence.

### Expected export output

```
Internal identity: ID-1 (resolved once, stable across relocation)
Source-native record A: retained with original path "/music/track1.flac"
Source-native record B: retained with original path "/music/relocated/track1.flac"
Edit overlay: none
Exported track: ID-1 with both source paths preserved as provenance
```

**PASS criteria:** The two imports resolve to one internal identity. Both
source-native records are retained with their original paths. The exported track
has one identity and both source paths visible in provenance. No data is lost.

**FAIL criteria:** Two separate internal identities are created, or one
source-native record overwrites the other, or the exported track loses one of
the source paths.

**NOT DEMONSTRATED:** Whether the content-hash resolution algorithm is correct
for near-duplicate files (same hash, different content) — this requires
acoustic fingerprint validation deferred to M2.

**Oracle:** Independent re-import of the exported data into a fresh OxiDJ
instance must produce the same single internal identity.

---

## F-02: Ordered playlist membership preservation

**Source:** Rekordbox XML
**Relationship group:** `RG-PLAYLIST-TRACK`

### Import fixture

A Rekordbox XML playlist tree:

```
Playlist "Sets" (folder)
  ├── Playlist "House"
  │     ├── TrackID=100 (Location="file://localhost/music/a.flac")
  │     ├── TrackID=200 (Location="file://localhost/music/b.flac")
  │     └── TrackID=300 (Location="file://localhost/music/c.flac")
  └── Playlist "Techno"
        ├── TrackID=200 (Location="file://localhost/music/b.flac")
        └── TrackID=400 (Location="file://localhost/music/d.flac")
```

### Operation

1. Import the XML. The import adapter stores the playlist tree as a
   source-native record preserving `sequenceNo` ordering.
2. Export back to Rekordbox XML format.

### Expected export output

```
Playlist "Sets" (folder)
  ├── Playlist "House"
  │     ├── TrackID=100 (sequenceNo=1)
  │     ├── TrackID=200 (sequenceNo=2)
  │     └── TrackID=300 (sequenceNo=3)
  └── Playlist "Techno"
        ├── TrackID=200 (sequenceNo=1)
        └── TrackID=400 (sequenceNo=2)
```

**PASS criteria:** Playlist ordering is preserved exactly. TrackID=200 appears
in both playlists at the correct positions. Folder hierarchy is intact.

**FAIL criteria:** Any track is missing, reordered, or duplicated. Folder
structure is flattened or altered.

**NOT DEMONSTRATED:** Whether Rekordbox itself re-imports this XML identically
— that requires the Rekordbox application and is outside OxiDJ's control. The
oracle is the independent re-import into OxiDJ, not Rekordbox acceptance.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
playlist tree with identical ordering.

---

## F-03: Cue/loop and beat-grid relationship preservation

**Source:** Mixxx
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Mixxx track with:

```
BeatGrid: BPM=128.0, anchor=2.0s (constant tempo)
CueInfo:
  - HotCue index 0: start=10.0s, label="Intro", color="red"
  - HotCue index 1: start=20.0s, label="Drop", color="blue"
  - Loop: start=15.0s, end=25.0s, label="Loop 1"
  - MainCue: start=5.0s
```

### Operation

1. Import the track. The import adapter stores the beat grid and all CueInfo
   entries as source-native records.
2. Export back to Mixxx format.

### Expected export output

```
BeatGrid: BPM=128.0, anchor=2.0s (constant tempo)
CueInfo:
  - HotCue index 0: start=10.0s, label="Intro", color="red"
  - HotCue index 1: start=20.0s, label="Drop", color="blue"
  - Loop: start=15.0s, end=25.0s, label="Loop 1"
  - MainCue: start=5.0s
```

**PASS criteria:** All cue positions, labels, colors, indices, and the beat grid
are preserved exactly. The loop remains a range (start + end), not a point.

**FAIL criteria:** Any cue position, label, color, or index is altered. The
loop is converted to a point. The beat grid BPM or anchor is changed.

**NOT DEMONSTRATED:** Whether Mixxx itself re-imports this identically —
requires the Mixxx application. The oracle is independent re-import into OxiDJ.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces identical
cue and beat-grid data.

---

## F-04: Field provenance preservation across edit

**Source:** Mixxx
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Mixxx track with:

```
BeatGrid: BPM=128.0, anchor=2.0s, provenance=ANALYZED
HotCue index 0: start=10.0s, label="Intro", provenance=VENDOR:Mixxx
```

### Operation

1. Import the track.
2. User edits: change hotcue label from "Intro" to "Opening" and BPM from
   128.0 to 130.0.
3. Export.

### Expected export output

```
BeatGrid: BPM=130.0, anchor=2.0s, provenance=USER (edited from ANALYZED 128.0)
HotCue index 0: start=10.0s, label="Opening", provenance=USER (edited from VENDOR:Mixxx "Intro")
```

The source-native records retain the original values:

```
Source-native BeatGrid: BPM=128.0, anchor=2.0s, provenance=ANALYZED
Source-native HotCue 0: start=10.0s, label="Intro", provenance=VENDOR:Mixxx
```

Edit overlays:

```
Overlay 1: source_path="/BeatGrid/BPM", old="128.0", new="130.0", provenance=USER
Overlay 2: source_path="/CueInfo/0/label", old="Intro", new="Opening", provenance=USER
```

**PASS criteria:** The exported values reflect the user edits. The
source-native records retain the original values. The edit overlays record the
exact source path, old value, new value, and USER provenance. Provenance is
never silently overwritten.

**FAIL criteria:** The source-native record is mutated in place (original value
lost). The overlay does not record the old value. Provenance is changed without
an overlay.

**NOT DEMONSTRATED:** Whether the BPM edit should cascade to cue positions —
in Mixxx, changing BPM shifts the beat grid but does not automatically move
cues (cues are in ms, not beats). The fixture assumes no cascade; if cascade
is desired, it must be an explicit user action recorded as additional overlays.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
exported values and the same source-native + overlay decomposition.

---

## F-05: Unknown vendor payload preservation (OneLibrary)

**Source:** OneLibrary
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A OneLibrary track with a cue row:

```
Cue row: kind=7, colorTableIndex=3, cueComment="custom", isActiveLoop=true,
         inUsec=10000000, outUsec=20000000, in150FramePerSec=1500000,
         out150FramePerSec=3000000, inMpegFrameNumber=..., inDecodingStartFramePosition=...,
         unknown_field_X="unrecognized value"
```

`kind=7` is not in the documented mapping (0=Cue, 3=Load, 4=Loop).
`unknown_field_X` has no documented semantics.

### Operation

1. Import the OneLibrary cue row as a source-native record. Store all fields
   verbatim, including `kind=7` and `unknown_field_X`.
2. The neutral projection maps known fields to the neutral cue model and marks
   unknown fields as opaque payloads.
3. Export back to OneLibrary format.

### Expected export output

```
Cue row: kind=7, colorTableIndex=3, cueComment="custom", isActiveLoop=true,
         inUsec=10000000, outUsec=20000000, in150FramePerSec=1500000,
         out150FramePerSec=3000000, inMpegFrameNumber=..., inDecodingStartFramePosition=...,
         unknown_field_X="unrecognized value"
```

The neutral projection:

```
Neutral cue: type=UNKNOWN(7), comment="custom", is_active_loop=true,
            start_ms=10000.0, end_ms=20000.0,
            opaque_payload={kind: 7, colorTableIndex: 3, unknown_field_X: "unrecognized value"}
```

**PASS criteria:** The exported OneLibrary cue row is byte-identical to the
imported row. The neutral projection preserves known fields and marks unknown
fields as opaque payloads. No data is silently dropped.

**FAIL criteria:** `kind=7` is mapped to a known type (e.g., "Cue") without
recording the mapping as an interpretation. `unknown_field_X` is discarded.
Any position encoding is altered.

**NOT DEMONSTRATED:** Whether `kind=7` has a real meaning in OneLibrary —
this requires empirical validation with official Rekordbox software (see
`docs/research/interoperability.md` fixture plan D). Opaque preservation is
the safe fallback.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
byte-identical OneLibrary cue row.

---

## F-06: Beat grid edit does not invalidate absolute-time cues

**Source:** Mixxx
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Mixxx track with a variable-tempo BeatMap and a hotcue at an absolute millisecond
position:

```
BeatMap: variable tempo
  - Beat 1: position=0.0s, BPM=120.0, source=ANALYZED
  - Beat 2: position=2.5s, BPM=128.0, source=ANALYZED
  - Beat 3: position=5.0s, BPM=130.0, source=ANALYZED
HotCue index 0: start=3.0s, label="Transition", color="green"
```

The hotcue is at 3.0s — an absolute CueInfo position in milliseconds. Per the Mixxx
source evidence, CueInfo interchange positions are milliseconds, and BPM and
anchor/phase are independent of cue positions: phase shifts move the grid, BPM
changes spacing, but cue positions in ms are not derived from beat-grid structure
(mixxx-semantics.md lines 10–12; semantic-mapping.md §5).

### Operation

1. Import the track. The BeatMap is stored as a source-native record with all
   beat positions and BPM values.
2. User edits: delete Beat 2 (the tempo transition at 2.5s).
3. The pre-write relationship validator checks whether the BeatMap edit affects
   any cue or loop position that depends on it. Per the Mixxx source evidence,
   CueInfo positions are absolute milliseconds and beat-grid information is
   separate, so the hotcue at 3.0s is not derived from the BeatMap structure
   (mixxx-semantics.md lines 10–12; semantic-mapping.md §5).

### Expected export output

The validator does NOT flag the hotcue as invalidated, because no inter-field
dependency exists between the absolute-time Mixxx cue and the BeatMap:
```
Exported HotCue 0: start=3.0s, label="Transition", color="green"  (unchanged)
Exported BeatMap: [Beat 1: 0.0s/120, Beat 3: 5.0s/130]  (Beat 2 removed per overlay)

Source-native BeatMap: [Beat 1: 0.0s/120, Beat 2: 2.5s/128, Beat 3: 5.0s/130]  (retained, unmutated)
Source-native HotCue 0: start=3.0s, label="Transition", color="green"  (retained, unmutated)

Edit overlay:
  Overlay 1: source_path="/BeatMap/2", action="delete", old="Beat(2.5s, 128BPM)", provenance=USER
```

No loss report is generated for the hotcue, because its absolute millisecond
position is independent of the BeatMap structure per Mixxx source evidence.

**PASS criteria:** The hotcue at 3.0s is retained at 3.0s in the export. The
BeatMap reflects the deletion. The source-native records are not mutated. The
edit overlay records the deletion. No spurious loss report flags the hotcue as
invalidated by the beat grid edit.

**FAIL criteria:** The system invalidates, relocates, or withholds the hotcue
at 3.0s because of the BeatMap deletion, treating an absolute-millisecond Mixxx
cue as beat-relative without an explicit relationship. The source-native record
is mutated to remove Beat 2. The overlay does not record the deletion.

**NOT DEMONSTRATED:** Whether Mixxx supports any beat-locked cue semantics — i.e.,
a cue whose position is defined relative to beat grid positions rather than
absolute milliseconds. No source evidence was found: CueInfo positions are
milliseconds and beat-grid information is separate (mixxx-semantics.md lines
10–12; semantic-mapping.md §5; semantic-mapping.md §8 Closed: point cues vs
loops). The "beat-locked cue" use case, if desired, must be an explicit
OxiDJ/user-declared relationship under proposal, not an inferred Mixxx behavior.
The coupled-field behavior of beat grid edits propagating to absolute-time cues
is NOT DEMONSTRATED as vendor behavior.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
retained hotcue position, the same source-native + overlay decomposition, and no
spurious loss report for the cue.

### F-06 (proposal): Beat-locked cue (explicit user-declared relationship)

If OxiDJ or the user explicitly declares a beat-locked relationship — e.g., "this
cue should track the beat grid so that its beat position is preserved across
tempo edits" — then the pre-write validator would treat the hotcue as dependent
on the BeatMap. Deleting Beat 2 would then produce a loss report:
```
LOSS REPORT (under proposal):
  Relationship group: RG-BEATGRID-CUES
  Declared relationship: beat-locked cue (explicit OxiDJ/user-declared)
  Invalidated by: edit deleting Beat 2
  Dependent fields affected:
    - HotCue 0: beat position referenced Beat 2 (2.5s, 128BPM), now absent
  Conservative action: WITHHOLD beat-locked HotCue 0 from export
  Alternative: export with explicit loss report flagging the affected field
```

This is **NOT DEMONSTRATED** as Mixxx behavior — no source evidence shows that
Mixxx expresses cue positions as beat-relative. It is a proposed OxiDJ extension
that would require its own fixture and explicit user declaration before ADR-005
acceptance.

**PASS criteria (proposal scenario):** The validator detects the beat-locked
relationship and generates the loss report or withholds the dependent cue.

**FAIL criteria (proposal scenario):** The validator ignores the beat-locked
relationship, or the source-native record is mutated to remove the cue, or the
edit overlay does not record the deletion.

**NOT DEMONSTRATED (proposal scenario):** Whether Mixxx supports any beat-locked
cue semantics — no source evidence was found; CueInfo positions are absolute
milliseconds, independent of beat-grid structure (mixxx-semantics.md lines 10–12).
The beat-locked relationship is an explicit OxiDJ/user-declared proposal, not
inferred vendor behavior.

**Oracle (proposal scenario):** Independent re-import into a fresh OxiDJ instance
produces the same loss report and the same source-native + overlay decomposition.

---

## F-07: Pre-write loss report / conservative withholding

**Source:** Rekordbox XML
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Rekordbox XML track with multiple tempo entries and a position mark:

```
TEMPO entries:
  - Inizio=0.0s, BPM=120.0, Metro=4, Battito=1
  - Inizio=10.0s, BPM=128.0, Metro=4, Battito=1
POSITION_MARK:
  - Type=Cue, Start=5.0s, End=, Num=0, Name="Intro"
```

### Operation

1. Import the XML. Store TEMPO entries and POSITION_MARK as source-native
   records.
2. User edits: change the first TEMPO BPM from 120.0 to 125.0.
3. The edit changes the tempo at position 0.0s, which affects the time
   mapping for the position mark at 5.0s (the beat at 5.0s changes).

### Expected export output

The pre-write validator detects the tempo edit affects the position mark's
time mapping:

```
LOSS REPORT:
  Relationship group: RG-BEATGRID-CUES
  Invalidated by: edit changing TEMPO[0].BPM from 120.0 to 125.0
  Dependent fields affected:
    - POSITION_MARK at 5.0s: time position assumed 120.0 BPM from 0.0s
  Conservative action: WITHHOLD TEMPO and POSITION_MARK from export
  Alternative: export with explicit loss report
```

**PASS criteria:** The pre-write validator detects the dependency. The export
either withholds or reports the inconsistency.

**FAIL criteria:** The TEMPO is exported with the new BPM and the
POSITION_MARK is exported at 5.0s with no warning, even though the time
mapping is now incorrect.

**NOT DEMONSTRATED:** Whether Rekordbox itself recalculates position marks
when tempo changes — this is undocumented. The conservative approach is to
withhold, not to assume.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
loss report.

---

## F-08: Unsupported relationship payload round-trip

**Source:** OneLibrary
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A OneLibrary track with a cue that has multiple position encodings, some of
which are not independently interpretable:

```
Cue row: kind=0, inUsec=10000000, outUsec=20000000,
         in150FramePerSec=1500000, out150FramePerSec=3000000,
         inMpegFrameNumber=750000, outMpegFrameNumber=1500000,
         inDecodingStartFramePosition=0, outDecodingStartFramePosition=0,
         inNumberOfSampleInBlock=480000, outNumberOfSampleInBlock=960000,
         inTimeCode=0, outTimeCode=0,
         unknown_encoding_X=0xDEADBEEF
```

### Operation

1. Import the cue row as a source-native record. Store all position encodings
   verbatim.
2. The neutral projection maps `inUsec`/`outUsec` to neutral start/end in ms.
   Other encodings are preserved as opaque payloads.
3. Export back to OneLibrary format.

### Expected export output

```
Cue row: kind=0, inUsec=10000000, outUsec=20000000,
         in150FramePerSec=1500000, out150FramePerSec=3000000,
         inMpegFrameNumber=750000, outMpegFrameNumber=1500000,
         inDecodingStartFramePosition=0, outDecodingStartFramePosition=0,
         inNumberOfSampleInBlock=480000, outNumberOfSampleInBlock=960000,
         inTimeCode=0, outTimeCode=0,
         unknown_encoding_X=0xDEADBEEF
```

The neutral projection:

```
Neutral cue: type=Cue, start_ms=10000.0, end_ms=20000.0,
            opaque_payload={in150FramePerSec: 1500000, out150FramePerSec: 3000000,
                            inMpegFrameNumber: 750000, outMpegFrameNumber: 1500000,
                            inDecodingStartFramePosition: 0, outDecodingStartFramePosition: 0,
                            inNumberOfSampleInBlock: 480000, outNumberOfSampleInBlock: 960000,
                            inTimeCode: 0, outTimeCode: 0,
                            unknown_encoding_X: 0xDEADBEEF}
```

**PASS criteria:** The exported cue row is byte-identical to the imported row.
The neutral projection preserves known fields and marks unknown encodings as
opaque payloads. No encoding is silently dropped or reinterpreted.

**FAIL criteria:** Any position encoding is altered, dropped, or silently
converted to a different encoding. `unknown_encoding_X` is discarded.

**NOT DEMONSTRATED:** Whether the position encodings are all in sync or
whether some are derived from others — this requires empirical validation
with official Rekordbox software (see `docs/research/interoperability.md`
fixture plan D). Opaque preservation of all encodings is the safe fallback.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
byte-identical cue row.

---

## F-09: Independent re-import oracle

**Source:** All vendors
**Relationship group:** All

### Fixture

For every fixture F-01 through F-08, after export:

### Operation

1. Export the edited library to the target vendor format.
2. Import the exported data into a **fresh** OxiDJ instance (independent
   re-import).
3. Compare the re-imported data against the expected export output.

### Expected result

```
Re-imported data == Expected export output (exact match)
```

**PASS criteria:** The re-imported data matches the expected export output
exactly. The neutral projection computed from the re-imported source-native
records matches the neutral projection from the original import.

**FAIL criteria:** Any field differs between the re-imported data and the
expected export output. The neutral projection from re-import differs from the
original projection.

**NOT DEMONSTRATED:** Whether the vendor's own application can import the
exported data — that requires the vendor application and is outside OxiDJ's
control. The oracle is independent re-import into OxiDJ.

**Oracle:** This fixture is itself the oracle for F-01 through F-08. It must
pass for all fixtures before ADR-005 can be accepted.

---

## F-10: Variable-tempo beat grid preservation

**Source:** Rekordbox XML
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Rekordbox XML track with multiple tempo entries (variable tempo):

```
TEMPO entries:
  - Inizio=0.0s, BPM=120.0, Metro=4, Battito=1
  - Inizio=8.0s, BPM=128.0, Metro=4, Battito=1
  - Inizio=16.0s, BPM=132.0, Metro=4, Battito=1
```

### Operation

1. Import the XML. Store all TEMPO entries as a source-native record.
2. The neutral projection represents this as a variable-tempo beat grid.
3. Export back to Rekordbox XML format.

### Expected export output

```
TEMPO entries:
  - Inizio=0.0s, BPM=120.0, Metro=4, Battito=1
  - Inizio=8.0s, BPM=128.0, Metro=4, Battito=1
  - Inizio=16.0s, BPM=132.0, Metro=4, Battito=1
```

The neutral projection:

```
Variable-tempo beat grid:
  - Position 0.0s: BPM=120.0
  - Position 8.0s: BPM=128.0
  - Position 16.0s: BPM=132.0
```

**PASS criteria:** All TEMPO entries are preserved exactly. The neutral
projection represents the variable tempo correctly (not flattened to average
BPM). The export reproduces the original TEMPO entries.

**FAIL criteria:** The TEMPO entries are flattened to a single average BPM
(e.g., 126.67). Any TEMPO entry is lost or altered. The neutral projection
shows a constant-tempo grid.

**NOT DEMONSTRATED:** Whether Rekordbox itself re-imports this identically —
requires the Rekordbox application. The oracle is independent re-import into
OxiDJ.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
TEMPO entries and the same variable-tempo neutral projection.

---

## F-11: Point cue vs loop range distinction

**Source:** Mixxx
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Mixxx track with a point cue and a loop:

```
CueInfo:
  - HotCue index 0: start=10.0s, label="Intro", color="red"  (point cue)
  - Loop: start=15.0s, end=25.0s, label="Loop 1"  (range)
```

### Operation

1. Import the track. Store CueInfo entries as source-native records.
2. The neutral projection maps the point cue to a neutral point cue (start
   only) and the loop to a neutral loop range (start + end).
3. Export back to Mixxx format.

### Expected export output

```
CueInfo:
  - HotCue index 0: start=10.0s, label="Intro", color="red"
  - Loop: start=15.0s, end=25.0s, label="Loop 1"
```

The neutral projection:

```
Neutral cues:
  - PointCue: start=10.0s, label="Intro", color="red"
  - LoopRange: start=15.0s, end=25.0s, label="Loop 1"
```

**PASS criteria:** The point cue is exported as a point (no end). The loop
is exported as a range (start + end). The neutral projection preserves the
distinction.

**FAIL criteria:** The point cue is exported with an end (e.g., start=10.0,
end=10.0). The loop is exported as a point. The neutral projection conflates
point cues and loop ranges.

**NOT DEMONSTRATED:** Whether Mixxx itself re-imports this identically —
requires the Mixxx application. The oracle is independent re-import into OxiDJ.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
point-cue vs. loop-range distinction.

---

## F-12: Hotcue capacity overflow reporting

**Source:** Mixxx
**Relationship group:** `RG-BEATGRID-CUES`

### Import fixture

A Mixxx track with 40 hotcues (exceeding the documented 38-hotcue limit):

```
CueInfo:
  - HotCue indices 0-37: valid hotcues with positions and labels
  - HotCue indices 38-39: additional hotcues (overflow)
```

### Operation

1. Import the track. The import adapter detects that hotcue indices 38-39
   exceed the Mixxx 38-hotcue capacity.
2. The import adapter stores all 40 hotcues as source-native records.
3. The neutral projection marks hotcues 38-39 as exceeding Mixxx capacity.
4. Export back to Mixxx format.

### Expected export output

```
LOSS REPORT:
  Relationship group: RG-BEATGRID-CUES
  Issue: hotcue capacity overflow
  Details:
    - Hotcues 0-37: within Mixxx capacity, exported normally
    - Hotcues 38-39: exceed Mixxx 38-hotcue limit
  Conservative action: WITHHOLD hotcues 38-39 from Mixxx export
  Alternative: export all 40 with explicit loss report flagging overflow

Source-native record: all 40 hotcues retained
Neutral projection: 40 hotcues, indices 38-39 marked as "exceeds Mixxx capacity"
```

**PASS criteria:** The import adapter detects the overflow. The export either
withholds the overflow hotcues or includes an explicit loss report. All 40
hotcues are retained in the source-native record.

**FAIL criteria:** Hotcues 38-39 are silently dropped. The export succeeds
with no warning. The source-native record is truncated to 38 hotcues.

**NOT DEMONSTRATED:** Whether Mixxx itself would reject the export or
silently drop the overflow — this requires the Mixxx application. The
conservative approach is to withhold with a loss report.

**Oracle:** Independent re-import into a fresh OxiDJ instance produces the same
overflow report and the same 40-hotcue source-native record.

---

## Fixture summary

| Fixture | Vendor | Relationship group | Key behavior | Result |
|---------|--------|-------------------|--------------|--------|
| F-01 | Mixxx | RG-IDENTITY-LOCATION | Identity stable across relocation | PASS/FAIL/NOT DEMONSTRATED |
| F-02 | Rekordbox XML | RG-PLAYLIST-TRACK | Ordered playlist membership | PASS/FAIL/NOT DEMONSTRATED |
| F-03 | Mixxx | RG-BEATGRID-CUES | Cue/loop + beat-grid preservation | PASS/FAIL/NOT DEMONSTRATED |
| F-04 | Mixxx | RG-BEATGRID-CUES | Field provenance across edit | PASS/FAIL/NOT DEMONSTRATED |
| F-05 | OneLibrary | RG-BEATGRID-CUES | Unknown vendor payload preservation | PASS/FAIL/NOT DEMONSTRATED |
| F-06 | Mixxx | RG-BEATGRID-CUES | Beat grid edit vs absolute-time cue independence | PASS/FAIL/NOT DEMONSTRATED |
| F-07 | Rekordbox XML | RG-BEATGRID-CUES | Pre-write loss report / withholding | PASS/FAIL/NOT DEMONSTRATED |
| F-08 | OneLibrary | RG-BEATGRID-CUES | Unsupported relationship payload round-trip | PASS/FAIL/NOT DEMONSTRATED |
| F-09 | All | All | Independent re-import oracle | PASS/FAIL/NOT DEMONSTRATED |
| F-10 | Rekordbox XML | RG-BEATGRID-CUES | Variable-tempo beat grid preservation | PASS/FAIL/NOT DEMONSTRATED |
| F-11 | Mixxx | RG-BEATGRID-CUES | Point cue vs loop range distinction | PASS/FAIL/NOT DEMONSTRATED |
| F-12 | Mixxx | RG-BEATGRID-CUES | Hotcue capacity overflow reporting | PASS/FAIL/NOT DEMONSTRATED |

All fixtures must PASS before ADR-005 can transition from Proposed. Any
NOT DEMONSTRATED result must be resolved before acceptance.

---

*This document is a companion to `docs/adr/005-metadata-schema.md`. It is a
decision-bearing validation plan, not a frozen schema. No code is required —
these fixtures are specified for future implementation as automated tests.*
