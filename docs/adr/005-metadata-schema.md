---
id: 005
title: Source-native metadata records with neutral projection and edit overlays
status: Proposed
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/adr/003-metadata-representation.md, docs/semantic-mapping.md, docs/research/mixxx-semantics.md, docs/research/interoperability.md, docs/research/soundcraft.md, docs/adr/005-relationship-fixtures.md]
sol_review: required
provisional_reason: "Sol review required but not yet completed; field-relationship risk carried from ADR-003 makes this decision-bearing"
---

# ADR-005: Source-Native Metadata Records with Neutral Projection and Edit Overlays

> **Proposed.** This ADR carries the ADR-003 field-relationship risk and must pass
> decision-bearing fixture validation before it can transition to Provisional or
> Accepted. It is approved only for drafting and fixture validation, not as a frozen
> schema or permission to admit persistence dependencies.

## Context

ADR-003 accepted a neutral internal domain model with per-field provenance but
explicitly flagged that provenance does not capture relationships between fields:
an edit to a cue or beat grid can invalidate retained opaque vendor metadata while
bytes survive. The independent Sol/medium review identified source-native records
with a neutral projection and explicit edit overlays as the named alternative that
may preserve those relationships more reliably. This ADR proposes that
representation for decision-bearing fixture validation.

## Decision

Adopt **source-native metadata records** with a **neutral projection** and
**explicit user-edit overlays** as the M1 metadata and relationship representation.

1. **Source-native records.** Each vendor import stores the original metadata
   structure intact — Mixxx cue list with indices and flags, Rekordbox XML
   position marks with `Num` convention, OneLibrary cue rows with all position
   encodings. Unknown or unrepresentable fields remain as opaque payloads within
   their native parent structure. No field-level mapping to a canonical schema at
   import time.

2. **Neutral projection.** A read-only projection maps source-native records to
   the neutral domain model (ADR-003) for UI and queries. The projection is
   derived, not stored; per-field provenance is computed at projection time from
   the source record and any applicable edit overlay.

3. **Explicit edit overlays.** User edits are stored as overlay records
   referencing specific source-native fields by stable path. An overlay carries
   `provenance=USER`, the source path, the old value, the new value, and a
   timestamp. Edits never mutate source-native records in place.

4. **Relationship groups.** Coupled fields are grouped only where evidence
   demonstrates an inter-field dependency. In Mixxx, CueInfo positions are absolute
   milliseconds and beat-grid information is separate (mixxx-semantics.md lines
   10–12; semantic-mapping.md §5), so a BeatMap edit does not by itself invalidate a
   Mixxx hotcue. A beat-locked cue — where a cue position is expressed relative to
   the beat grid — would require an explicit OxiDJ/user-declared relationship and
   is NOT DEMONSTRATED as vendor behavior (see F-06). Where a demonstrated
   relationship group is edited, all dependent members are checked for consistency.

5. **Pre-write loss report and conservative invalidation.** Before export, all
   relationship groups are validated. Inconsistent groups are either withheld
   from export (conservative) or exported with an explicit loss report
   identifying the invalidated fields and the edit that caused the inconsistency.

This decision constrains the M1 schema, Rust types, and serialization format; it
does not define them. Those remain implementation choices within this
representation.

> Relationship-aware import/edit/export flow (proposed; not yet implemented or
> fixture-validated):

```mermaid
flow LR
    subgraph "1. Import"
        V[VENDOR records] --> SN[Source-native records<br/>stored immutable]
    end
    subgraph "3. Read"
        SN --> NP[Neutral projection<br/>derived at read time]
    end
    subgraph "4. Edit"
        U[User edit] --> EO[Edit overlay<br/>non-mutating]
    end
    subgraph "5. Export — pre-write validation"
        SN & EO --> VAL[Validate relationship<br/>groups in overlay]
        VAL --> CHK{Group<br/>consistent?}
        CHK -->|Yes| OK[Export normally]
        CHK -->|No| WH[Withhold inconsistent<br/>fields<br/>or emit explicit<br/>loss report]
    end
```

## Evidence

- ADR-003 Sol review counter-risk (lines 105–123): per-field provenance does not
  capture inter-field relationships; source-native records plus neutral
  projection and edit overlays is the named alternative; import-edit-export
  validation is decision-bearing.
- `docs/semantic-mapping.md` section 8: Open vs. Closed table — identity,
  location, BPM/tempo, bars, cues/loops, ratings/colors/playlists, key across all
  three ecosystems.
- `docs/research/mixxx-semantics.md` findings 3: cue types, hotcue index 0 valid,
  locked-loop flag not saved in Mixxx DB, loops are ranges.
- `docs/research/mixxx-semantics.md` findings 4: analyzer/file/user provenance
  distinction.
- `docs/research/mixxx-semantics.md` lines 10–12: CueInfo interchange positions
  are milliseconds; BPM and anchor/phase are independent of cue positions;
  beat-grid information is separate from cue positions (Finding 2).
- `docs/research/interoperability.md`: "Do not decode/re-encode-and-discard
  unrecognized fields"; "Internal identity must be independent of vendor IDs and
  file paths"; fixture plan items A–E.
- `docs/research/soundcraft.md` REJECT (line 69): silent load-time repair
  rejected; ADAPT (line 62): separate read/write roots; ADAPT (line 64):
  separation of stored metadata and decoded caches.
- `docs/adr/005-relationship-fixtures.md`: concrete fixture cases with
  PASS/FAIL/NOT DEMONSTRATED expected outputs for import-edit-export.

## Options Considered

| Option | Decision | Reason |
|--------|----------|--------|
| Source-native records + neutral projection + edit overlays | **Proposed** | Preserves vendor relationships within native structures; avoids premature schema freeze; edit overlays prevent silent overwrite |
| Richer neutral schema with relationship columns | Rejected | Premature — schema would need to anticipate every vendor relationship; current evidence is insufficient to enumerate them all |
| Per-field provenance alone (ADR-003 as-is) | Rejected (insufficient) | Does not capture inter-field relationships; an edit to one field can invalidate dependent opaque data while bytes survive |
| Flatten vendor structures on import | Rejected | Lossy; contradicts ADR-003 principle 2 and mixxx-semantics.md invariants |

## Consequences

- Import adapters store source-native records; no field-level mapping to a
  canonical schema at import time.
- The neutral projection is computed at read time; it can be versioned and
  recomputed as understanding of vendor semantics improves.
- Edit overlays add storage but preserve the complete edit history and prevent
  silent data loss.
- Relationship-group validation adds pre-write overhead but prevents exporting
  inconsistent metadata.
- Export reads overlays, applies them to source-native records, projects to the
  target vendor format, and validates relationship groups before writing.
- Unknown or unrepresentable fields remain as opaque payloads within their
  source-native parent — preserved but not necessarily interpreted.

## Open Questions / What Would Change This

- Whether the neutral projection can be made deterministic and total — every
  source-native record projects without loss of interpretable meaning. Empirical
  bar: fixture validation in `docs/adr/005-relationship-fixtures.md`.
- How to handle vendor relationships that span multiple source-native records —
  e.g., OneLibrary cue table plus playlist content table referencing the same
  track. Not yet designed.
- Whether edit overlays can be composed — multiple edits to the same source
  path — without conflict. Requires versioning and timestamp resolution rules.
- Rekordbox `TrackID` stability across re-imports — unresolved. Affects whether
  identity reconciliation can rely on source IDs.
- OneLibrary `Cue.kind` values beyond 0/3/4 — cannot be mapped until empirically
  validated. Opaque preservation is the fallback.
- Whether pre-write relationship validation can be made complete — all
  relationship groups enumerated — or must remain best-effort with explicit
  NOT DEMONSTRATED gaps.
- Whether any vendor expresses cue positions as beat-relative rather than absolute
  time — no source evidence was found for Mixxx (CueInfo positions are milliseconds;
  see mixxx-semantics.md lines 10–12). The beat-locked cue use case, if desired,
  must be an explicit OxiDJ/user-declared relationship, not an inferred vendor
  behavior.

## Sol Review Packet

**DECISION:** Adopt source-native metadata records with a neutral projection and
explicit user-edit overlays. Store each vendor's metadata structure intact;
compute the neutral domain model as a derived read projection; store user edits
as overlays that reference source-native fields by stable path; validate coupled
relationship groups before export with conservative withholding or explicit loss
reports.

**WHY NOW:** ADR-003 established the neutral-model constraint map but flagged that
per-field provenance does not capture inter-field relationships. This is the most
difficult-to-reverse metadata decision after ADR-003: once data enters a
source-native plus overlay model, migrating to a different representation
requires transforming every library. The fixture validation must demonstrate that
the representation preserves vendor relationships across import-edit-export
before schema acceptance.

**CONSTRAINTS:** Must preserve Mixxx variable-tempo beat grids (not flatten to
average BPM). Must preserve Mixxx cue type semantics (point vs. range,
38-hotcue limit with overflow reporting). Must preserve Rekordbox XML multi-tempo
TEMPO entries. Must preserve unknown OneLibrary `Cue.kind` values and position
encodings. Must not silently drop unsupported relationship payloads. Must not
claim Mixxx behavior as a universal standard. Must not infer that absolute-time
Mixxx cue positions are invalidated by BeatMap edits without explicit source
evidence; CueInfo positions are milliseconds, independent of beat-grid structure
(mixxx-semantics.md lines 10–12). No vendor database can be copied into OxiDJ.
No persistence dependency.

**OPTIONS:** (1) Source-native records + neutral projection + edit overlays —
proposed; (2) Richer neutral schema with relationship columns — rejected
(premature); (3) Per-field provenance alone — rejected (insufficient for
relationships); (4) Flatten vendor structures — rejected (lossy).

**EVIDENCE:** ADR-003 Sol review counter-risk (lines 105–123);
`docs/semantic-mapping.md` section 8; `docs/research/mixxx-semantics.md`
findings 3–4; `docs/research/interoperability.md` fixture plan;
`docs/research/soundcraft.md` REJECT silent repair;
`docs/adr/005-relationship-fixtures.md`.

**CURRENT RECOMMENDATION:** Source-native records with neutral projection and
edit overlays. Confidence: medium — the direction is owner-approved for drafting
and fixture validation, but the fixture outcomes are not yet demonstrated. What
changes it: fixture failures showing that source-native records cannot safely
preserve vendor relationships, or that the projection loses interpretable
meaning.

**OUTPUT CONTRACT:** This decision constrains the M1 metadata schema. The
fixture validation plan in `docs/adr/005-relationship-fixtures.md` must pass
before ADR-005 transitions from Proposed. No code — this is a representation
decision. No benchmark, fault campaign, or persistence dependency.
