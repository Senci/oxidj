---
id: 003
title: Adopt neutral internal domain model with provenance
status: Provisional
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/research/mixxx-semantics.md, docs/research/interoperability.md, docs/research/soundcraft.md, docs/semantic-mapping.md]
sol_review: required
review_status: completed 2026-10-15 (gpt-6.1-sol, reasoning medium) — Option 1 confirmed, with a live counter-risk recorded
review_confidence: high
provisional_reason: The direction is confirmed by review, but the reviewer recorded a counter-risk (field-level provenance does not capture relationships between fields) that must be resolved by import-edit-export validation before this is accepted.
---

# ADR-003: Metadata Representation

> **Provisional (M0-07).** Evidence-backed recommendation, not yet accepted architecture. The required independent Sol/high review has not been performed and is suspended under the Nous-only free-model routing policy (see M0-07 comment, 2026-10-15). This is the most difficult-to-reverse M0 decision; do not treat it as accepted until the review gate is restored and completed.

> **Difficult-to-reverse.** This decision establishes the internal metadata representation. Once data enters OxiDJ's model, changing how identity, provenance, or vendor-field preservation works requires migration of every user's library. This is the most consequential M0 architectural decision.

## Context

OxiDJ's MVP is: "Import a real DJ library, manage it safely, export it again, and continue using compatible DJ software/hardware without unexpected metadata loss." Three ecosystems (Mixxx, Rekordbox XML, OneLibrary) represent the same concepts differently — and some concepts not at all. The internal representation must be neutral, lossless-by-default, and provenance-aware.

## Decision

Adopt a **neutral internal domain model** with these binding principles:

1. **Internal identity is independent of vendor IDs and file paths.** Resolve against normalized path plus content evidence (file hash or acoustic fingerprint). Retain all source IDs/URIs as provenance. (semantic-mapping.md §1)

2. **Vendor metadata is preserved, not flattened.** Unknown or unrepresentable fields are stored as opaque vendor payloads, never silently discarded. (interoperability.md findings table "Vendor passthrough" row and §Interchange strategy point 5, soundcraft.md REJECT "silent load-time repair")

3. **Provenance is attached per-field.** Every metadata field records its source: `ANALYZER` | `FILE_METADATA` | `USER` | vendor import (Rekordbox XML, Serato ANLZ, etc.). User edits never silently overwrite analyzed or imported values. (mixxx-semantics.md findings 4; interoperability.md findings table "Vendor passthrough" row)

4. **Representation is preserved, not normalized.** Constant-tempo vs variable-tempo beat grids are distinct internal forms; do not flatten a variable map to average BPM. Point cues remain distinct from loop ranges. (mixxx-semantics.md "Invariants and edge cases"; semantic-mapping.md §§3-5)

5. **Import/export is non-destructive and reports loss.** Unsupported/lost fields are reported to the user/adapter before writing. Original media and export artifacts are kept until the user approves a staged result. (interoperability.md §Interchange strategy, §Fixture plan)

This decision sets the **constraint map** for the metadata schema (ADR-005, deferred to M1). It does not define the database schema, the Rust types, or the serialization format — those are M1+ implementation decisions.

## Evidence

- `idea.md` principles: "vendor formats behind adapters", "neutral internal domain model", "no silent metadata loss", "reproducible import/export and round-trip tests."
- `interoperability.md` findings table ("XML track identity and paths" and "Vendor passthrough" rows) and §Interchange strategy (points 1-5): "Internal identity must be independent of vendor IDs and file paths"; "Do not decode/re-encode-and-discard unrecognized fields"; "Import/export must be non-destructive and report unsupported/lost fields before writing."
- `mixxx-semantics.md` findings 4: "Distinguish analyzer/file/user provenance and avoid silent overwrite of edits."
- `mixxx-semantics.md` findings 1: "Mixxx distinguishes constant-tempo BeatGrid (BPM + first beat) from BeatMap (individual beat positions)" — the two representations are not interchangeable, so representation must be preserved (the wiki synthesis phrases this as "Do not assume a dense BeatMap is equivalent to a sparse authored tempo map").
- `mixxx-semantics.md` "Invariants and edge cases": "Do not clamp valid extrapolated beats to track bounds or flatten variable tempo without recording loss." (The wiki synthesis phrases the second clause as "Flattening a variable map to one average BPM is lossy.")
- `mixxx-semantics.md` findings 3: "Loops are ranges" (a loop has start and end, not a point); point cues remain distinct from loop ranges (derived policy).
- `soundcraft.md` REJECT (line 69): "Silent load-time repair as imported-metadata policy" — "OxiDJ must preserve original vendor fields/provenance and report interpretations/repairs; repairs must not silently overwrite evidence."
- `soundcraft.md` ADAPT (line 64): "Separation of stored metadata and decoded caches" — durable provenance separate from replaceable analysis.
- `soundcraft.md` ADAPT (line 62): "Separate read/write roots and isolated automation profiles"; ADOPT (line 59): "Errors rather than unsupported success; hostile-input regressions."
- `semantic-mapping.md` §8 Summary: explicit Open vs. Closed table derived from all three ecosystems.

## Options Considered

| Option | Decision | Reason |
|--------|----------|--------|
| Neutral domain model + per-field provenance | **Provisional** | Aligns with all three ecosystem behaviors; prevents silent loss |
| Mirror a single vendor (e.g., Rekordbox OneLibrary) | Rejected | Locks users into one vendor; contradicts "no silent metadata loss" |
| Flatten all beat grids to constant BPM | Rejected | Lossy; both Mixxx and Rekordbox explicitly support variable tempo |
| Normalize/merge cue types across vendors | Rejected | Loses type semantics; Rekordbox→Mixxx mapping already documented as lossy |
| Discard unknown fields on import | Rejected | Contradicts core principle; interoperability.md "vendor passthrough" requirement |

## Consequences

- Schema design (M1, ADR-005) must include provenance columns and opaque vendor-payload storage.
- Import adapters must implement field-level mapping with loss reporting, not bulk conversion.
- Round-trip tests must verify provenance retention and loss reporting for each vendor.
- Identity resolution logic is centralized, not duplicated in each adapter.
- Storage overhead: opaque vendor payloads add bytes but prevent data loss — acceptable trade for MVP.

## Open Questions / What Would Change This

- Content evidence mechanism: file hash (fast, content-based) vs. acoustic fingerprint (content-based, format-agnostic). (Open — fingerprint library selection deferred to M2.)
- How to represent Rekordbox XML `Num` hotcue convention vs. Mixxx hotcue index 0 vs. OneLibrary `kind` integer — the neutral type must accommodate all three. (Open — resolved in ADR-005 schema.)
- OneLibrary `Cue.kind` integer values beyond 0/3/4 — cannot be mapped until empirically validated. (Open — see interoperability.md §Fixture D.)
- Rekordbox XML color encoding (RGB? index? palette?) — pyrekordbox OneLibrary docs show only `Color.name`, no RGB. (Open.)

## Sol Review Packet

> **Review status: SUSPENDED.** The required independent Sol/high review has not been performed and is suspended under the Nous-only free-model routing policy (see M0-07 comment, 2026-10-15). This packet is prepared for when the review gate is restored; do not treat the recommendation as reviewed.

**DECISION:** Adopt a neutral internal domain model with per-field provenance, vendor-metadata preservation (no flattening, no silent discard), and non-destructive import/export with loss reporting. Internal identity is independent of vendor IDs and file paths.

**WHY NOW:** This is the most difficult-to-reverse M0 decision. Getting it wrong means migrating every user's library later. All four research notes converge on these principles.

**CONSTRAINTS:** Must preserve Mixxx variable-tempo beat grids (not flatten to average BPM). Must preserve Mixxx cue type semantics (point vs range, 38-hotcue limit with overflow reporting). Must preserve Rekordbox XML multi-tempo TEMPO entries. Must preserve unknown OneLibrary `Cue.kind` values and position encodings. No vendor database can be copied into OxiDJ. No M5 implementation yet.

**OPTIONS:** (1) Neutral model + provenance — provisional; (2) Mirror one vendor — rejected (vendor lock-in); (3) Flatten beat grids — rejected (lossy); (4) Discard unknown fields — rejected (silent loss).

**EVIDENCE:** idea.md principles (neutral model, no silent loss, vendor adapters); interoperability.md §Interchange strategy and findings table (identity independence, vendor passthrough, non-destructive import); mixxx-semantics.md (provenance, variable-tempo preservation, point-vs-range cues); soundcraft.md REJECT silent repair and ADAPT metadata/cache separation.

**CURRENT RECOMMENDATION:** Neutral model with per-field provenance. Confidence: high (all four research notes converge; core OxiDJ principle from idea.md). What changes it: finding that a single vendor's model is so dominant that the adapter cost exceeds the neutral-model cost — unlikely given Rekordbox/OneLibrary schema unknowns.

**OUTPUT CONTRACT:** This decision constrains ADR-005 (metadata schema, M1). Until then, document the constraint map in `docs/semantic-mapping.md`. Future adapters must reference these principles. No code — this is a policy decision.

## Independent Review Outcome (2026-10-15)

**Reviewer:** `gpt-6.1-sol` via `openai-codex`, reasoning **medium**, isolated delegate (no tools, no delegation, this decision only).

**Recommendation returned:** **Option 1 — confirmed.** Neutral internal model with per-field provenance, internal identity independent of vendor IDs and file paths, and opaque preservation of unsupported vendor metadata.

**Confidence: high** — explicitly *not* a claim that the proposed schema has been validated.

**Reasons accepted:** variable-tempo grids and point/range cues need explicit semantics that neither a single-BPM nor a vendor-shaped model can express; independent identity stops vendor IDs and file relocation from determining a track's identity; provenance separates imported assertions, user edits and analysis results without silently overwriting disagreements; unknown-field preservation keeps information recoverable while loss reporting handles hotcue overflow and destination limits; adapter boundaries prevent XML interchange support from being mistaken for OneLibrary writing or hardware-compatible export.

**Counterargument recorded as a live risk, not dismissed:** a neutral model can decay into a lowest-common-denominator abstraction surrounded by vendor exceptions. **Per-field provenance does not capture relationships *between* fields** — changing a cue or a beat grid can invalidate retained opaque metadata, so blind passthrough could preserve bytes while corrupting meaning. The reviewer's alternative is **source-native metadata records plus a neutral projection and explicit edit overlays**, which may preserve those relationships more reliably without freezing an expansive canonical schema too early.

**Consequence for the work:** the provenance direction is accepted, but it must not be treated as sufficient by itself. The import-edit-export round-trip validation is now **decision-bearing rather than a nicety** — representative fixtures showing vendor semantics that depend on inseparable structures would favour source-native records with neutral projections and edit overlays. Flattening and discarding remain rejected either way.

**Status remains `Provisional`.** `ADR-005` (metadata schema, M1) must carry the field-relationship risk explicitly instead of assuming per-field provenance solves it.
