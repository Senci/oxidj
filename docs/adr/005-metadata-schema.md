---
id: 005
title: Source-native metadata records with neutral projection and edit overlays
status: Proposed
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/adr/003-metadata-representation.md, docs/semantic-mapping.md, docs/research/mixxx-semantics.md, docs/research/interoperability.md, docs/research/soundcraft.md, docs/adr/005-relationship-fixtures.md]
sol_review: required
provisional_reason: "Sol advisory on t_4d01d430 recommends retaining Proposed; logical contract review and executed decision-bearing fixtures remain required"
review_status: "2026-10-16 advisory completed (t_4d01d430, comment 129); revised contract not independently reviewed or executed"
---

# ADR-005: Source-Native Metadata Records with Neutral Projection and Edit Overlays

> **Proposed.** This ADR carries the ADR-003 field-relationship risk and must pass
> decision-bearing fixture validation before it can transition to Provisional or
> Accepted. It is approved only for drafting and fixture validation, not as a frozen
> schema or permission to admit persistence dependencies.

> **Scope clarification (2026-10-23):** the revised, separately versioned `local-catalog-alpha-v1` schema is outside this proposed vendor-schema decision and does not accept ADR-005. This ADR remains Proposed; its historical vendor-schema fixtures and A1–A3/C1–C6 remain preserved, open, and unpassed. The local alpha does not admit those fixtures as evidence or change their qualification status.

## Context

ADR-003 accepted a neutral internal domain model with per-field provenance but
explicitly flagged that provenance does not capture relationships between fields:
an edit to a cue or beat grid can invalidate retained opaque vendor metadata while
bytes survive. The independent Sol/medium review identified source-native records
with a neutral projection and explicit edit overlays as the named alternative that
may preserve those relationships more reliably. This ADR proposes that
representation for decision-bearing fixture validation.

## Decision

Propose **source-native metadata records** with a **neutral projection** and
**explicit user-edit overlays** as the candidate M1 logical representation.
The rules below are proposed OxiDJ semantics, not a final SQL/physical schema,
vendor contract, implemented API, or permission to execute qualification.

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
   referencing specific source-native fields by stable revision-qualified address. An overlay carries
   `provenance=USER`, a revision-qualified record/element target, typed old/new
   values, expected effective revision, operation ID, and audit timestamp.
   Timestamps never resolve conflicts. Edits never mutate source-native records.

4. **Relationship groups.** Coupled fields are grouped only where evidence
   demonstrates an inter-field dependency. In Mixxx, CueInfo positions are absolute
   milliseconds and beat-grid information is separate (mixxx-semantics.md lines
   10–12; semantic-mapping.md §5), so a BeatMap edit does not by itself invalidate a
   Mixxx hotcue. A beat-locked cue — where a cue position is expressed relative to
   the beat grid — would require an explicit OxiDJ/user-declared relationship and
   is NOT DEMONSTRATED as vendor behavior (see F-06). Where a demonstrated
   relationship group is edited, all dependent members are checked for consistency.

5. **Deterministic pre-write policy.** Before export, validate known and explicitly
   declared relationships. Use `safe-v1` below: withhold the smallest unsafe
   export unit and report it, or block the export if a safe unit cannot be
   isolated. Reporting alone never authorizes exporting known-inconsistent data.
   Unknown dependencies are not evidence of a specific coupling.

If accepted, this logical contract would constrain M1 types and serialization;
DDL, indexes, physical encoding and transaction batching remain separate work.
The unresolved coverage and adapter bindings below still prevent qualification.

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
        CHK -->|No| WH[Withhold unsafe unit + report<br/>or block export]
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

## Proposed logical semantic contract (logical-v1)

Everything in this section is **OxiDJ-declared, under review** unless explicitly
attributed to vendor evidence. Names are logical identifiers, not SQL tables or
Rust APIs. No existing accepted ADR is amended by this draft.

### Identity, source revisions, and addresses

- `TrackId` is a library-local stable identity, never a vendor ID, hash, or path.
  New IDs are allocated once and never reused. A fresh vendor-only import may
  allocate different IDs; compare an explicit one-to-one correspondence, not ID
  equality across independent libraries.
- Every import has an immutable artifact, byte length/digest, format/version and
  source-library namespace. Each native record has an internal `record_id` and
  immutable `source_revision`. Re-import creates a new revision/snapshot; it
  never overwrites the previous one. Equal vendor IDs in different namespaces
  are not equal identities. Duplicate IDs within one snapshot are conflicts.
- Collection elements receive internal `element_id`s on first capture; identity
  is distinct from hotcue index, XML Num, array index, label and position. An
  address is `(record_id, source_revision, element_id-or-root, field_name)`.
  Original ordinal/order is retained separately. Reorder does not retarget edits;
  deletion tombstones an element; new elements never inherit a deleted ID.
  Cross-import element matching is NOT DEMONSTRATED: do not carry overlays to a
  new source revision automatically, even when vendor IDs or values agree.
- Source bytes, vendor IDs, raw locations and imported provenance remain facts.
  A resolved location is a separate assertion. Paths/URIs alone never prove
  identity; no platform-independent case folding or symlink equivalence is
  assumed. A hash match alone does not authorize automatic merging.
- Relocation is explicit: `reconcile(record_revision, TrackId, evidence,
  expected_track_revision)`. Fixture evidence can declare byte-equal content
  plus an explicit user confirmation. Preserve that decision and both source
  assertions. Competing candidates, same path with different content, conflicting
  IDs or conflicting field values produce an unresolved conflict, not a merge
  or last-import-wins projection. Metadata disagreement survives reconciliation;
  selection of an effective source must be explicit and revisioned.

### Effective state, composition, and provenance

- Each track has a monotonically increasing `effective_revision`, initially 0.
  A mutation supplies a unique operation ID, expected revision, target address,
  operation (`set`, `delete`, `insert`, `reorder`, `select-source`, `clear-user`),
  typed old/new values and provenance. Validate the entire mutation atomically.
  Accepted mutations advance the revision once; rejected mutations change nothing.
  Record a monotonic accepted-operation sequence. Audit time is not ordering.
- Require expected revision and old-value equality against current effective
  state (including absent vs null). A stale edit, deleted target, wrong source
  revision or ambiguous source returns `Conflict`; do not rebase silently.
  Two edits A→B then B→C compose in accepted order; a later A→D is stale.
  Repeating an operation ID with identical payload returns its recorded result
  without applying twice; reusing it with a different payload is an error.
- Source selection is explicit per logical field/structure. An unresolved
  disagreement projects as a conflict with all candidates, not an arbitrary value.
  Within a selected source, active USER overlays win over analysis candidates;
  analysis never changes a USER assertion. An imported native USER assertion is
  also protected from automatic analysis replacement; selecting analysis instead
  requires an explicit revisioned source-selection operation. `clear-user` is an explicit revisioned
  operation revealing the latest applicable analysis candidate, otherwise the
  selected imported value. Every transition remains in internal history.
- Analysis is an immutable candidate with analyzer name/version, input source
  revision, input content evidence, units and analysis sequence. Apply only to
  the same content/source revision; stale results return `Conflict`. Imported
  Mixxx `ANALYZER` remains its vendor token. OxiDJ origin is `ANALYZER`, qualified
  by analyzer identity; the old fixture word `ANALYZED` is not a new vendor token.
  Exported vendor data need not carry OxiDJ USER/history provenance.

### Legitimate metadata and workload realization

Logical values include optional UTF-8 title/artist/comment and raw key text;
constant grid `(BPM, anchor)` distinct from a variable ordered beat-position
sequence; point cues distinct from ranges; explicit ordered playlist memberships;
source assertions and opaque native payloads. Each value has provenance and units.
Do not infer time signature, key notation, beat interpolation or unknown cue kind.
Absent, null, empty string and zero are distinct when the source distinguishes them.

The frozen profile in [the protocol](../research/m1-sqlite-admission-protocol.md)
is **not** a semantic schema. All nine cells are accounted for below. A possible
legitimate allocation is descriptive UTF-8 text plus actually present native
metadata for budgets, and ordered playlist memberships for association slots.
Lead-ratified `coverage-v1` interpretation: each cell's unchanged `B` value is
its assigned payload-budget ceiling, not a required exact realization. `A` is the
independently justified realized eligible logical byte-token count; once C1 defines
the counting rules, require `0 <= A <= B`. The frozen totals are sums of assigned
`B` values only, never realized `A`, serialized/storage bytes or measurements. The
ceiling alone does not qualify empty or tiny payloads: a measurable meaningful-
coverage criterion remains OPEN, with no numerical occupancy floor approved. No C1
allocation is approved. Eligible fields/types/encoding/ownership/exclusions, an
independently justified inventory/allocation, and measurable meaningful-coverage
acceptance must be specified and independently reviewed before C1 can close; C2/C3
compatibility must also be established. No padding field, arbitrary cue count or
sentinel-per-track may fill a cell. Zero association slots need not mean no cues, no
provenance, or no other relationships.

| Budget tokens (bytes) | Abstract slots | Initial count | Semantic realization status |
|---|---|---|---|
| 256 | 0 | 180000 | OPEN C1: legitimate text/native allocation; no slot-derived edge |
| 256 | 1 | 90000 | OPEN C1/C2: allocation plus membership endpoint/order |
| 256 | 16 | 30000 | OPEN C1/C2: allocation plus justified shared endpoints/order |
| 1024 | 0 | 90000 | OPEN C1: legitimate text/native allocation; no slot-derived edge |
| 1024 | 1 | 45000 | OPEN C1/C2: allocation plus membership endpoint/order |
| 1024 | 16 | 15000 | OPEN C1/C2: allocation plus justified shared endpoints/order |
| 8192 | 0 | 30000 | OPEN C1: legitimate text/native allocation; no slot-derived edge |
| 8192 | 1 | 15000 | OPEN C1/C2: allocation plus membership endpoint/order |
| 8192 | 16 | 5000 | OPEN C1/C2: allocation plus justified shared endpoints/order |

C1 must define which legitimate fields consume tokens, encoding/count rules and
optionality without equating the budget to serialized storage. C2 must specify
endpoint inclusion/ownership, sharing/skew, repeated membership identity, order and
allocation of the protocol's fixed 1,000-member query playlist; endpoint inclusion
is OPEN, not excluded. C3 must assign exact Unicode,
duplicate/null, cue/range, variable-grid, provenance and opaque cases independently
of slots, and keep frozen search/page cohorts stable. The scan-offer cells inherit
these same blockers; this draft does not revise any frozen counts or generator seed.
If a meaningful-coverage criterion or honest eligible allocation cannot be justified,
propose a reviewed profile revision before qualification; do not force schema semantics
to match the abstract ceilings.

Proposed operations, implementable as logical tests but not frozen workload payloads:

- Manual edit: set a selected cue label `Intro`→`Opening`, preserving start/end,
  index, color and source bytes. F-04 also exercises grid BPM 128→130. Proposed
  timed workload alternative: toggle a track's USER comment between exact UTF-8
  `m1-edit-a` and `m1-edit-b`; neither is a search/sort predicate. Selection,
  initial values, offer-to-track schedule, revisions and byte accounting remain
  **OPEN C4** until C1/C3/cohorts are bound. This is not a protocol freeze.
- Analysis update: replace the OxiDJ analyzer's candidate constant-grid BPM,
  alternately exact decimal 128 and 129, keeping anchor fixed; analyzer identity
  `m1-synthetic`/`1`, never a vendor result or measured audio analysis. USER BPM
  remains effective. For variable-grid tracks this operation is inapplicable,
  not permission to flatten. Target cohort, source/content versions, sequence,
  initial candidate, payload bounds and scheduling remain **OPEN C5**. Preserve
  the protocol's 100 updates/s and don't quietly substitute no-op acknowledgements.

### Cross-record relationships and test-only assets

- A membership has its own stable ID, playlist endpoint, track endpoint and
  explicit order. Folders form an acyclic tree. Reject dangling endpoints,
  duplicate order positions and cycles in one mutation. Reorder preserves
  membership identity. Deleting a track with memberships is rejected unless
  explicit membership removals are in the same logical mutation; no silent cascade.
  Membership/order mutations validate expected revisions of every touched playlist
  and track aggregate, advancing each affected aggregate once atomically.
  These are OxiDJ invariants, not claims about OneLibrary foreign keys.
- Constant BPM and anchor are independent values. Mixxx absolute millisecond cues
  and XML second-based marks have no demonstrated dependency on tempo edits.
  A key↔grid anchor group is **NOT DEMONSTRATED** and is not enabled. Relationships
  between unknown encodings are unknown, not a license to invent conversions.
- Optional OxiDJ-declared beat lock names exact grid/beat element IDs and an exact
  offset. Deleting its anchor makes the relationship unresolved; no inferred
  interpolation or automatic relocation. Withhold that cue with a report under
  `safe-v1`. This extension has no vendor-behavior claim (F-06b).
- The test namespace `m1-asset-stubs-v1` alone contains logical test tracks
  `asset-track-1` and `asset-track-2`, resolved to deterministic noncolliding
  initial TrackIds by the later generator manifest. Bindings `ref-1` and `ref-2`
  are stable test-only relationships, not cues, artwork or product media fields.
  Asset IDs are exactly `stub-a`, `stub-b`, `stub-c`; payload/manifest bytes are
  exactly those already defined by protocol §1. No other asset is generated.
- B1 live bindings are `(ref-1, asset-track-1, stub-a)` and
  `(ref-2, asset-track-2, stub-b)`. One logical mutation, expected boundary B1,
  replaces ref-1's endpoint with `stub-b` and ref-2's with `stub-c`; either both
  changes commit or neither does. B2 live bindings are therefore b/c. Historical
  bindings remain audit history, not live asset references or bundle members.
  All other tracks have no sentinel references. This fixture is independent of
  abstract fan-out slots. Concrete TrackIds/physical reference representation
  remain **OPEN C6**, not invented DB row IDs.
- Reject a live reference to an absent test asset and removal of a referenced
  asset. At each stable boundary the manifest set equals the set of live referenced
  sentinel IDs. Restore checks exact live bindings plus historical state against
  the boundary snapshot; it does not reinterpret old history as live references.
  Backup publication/isolation/retention mechanisms remain outside this ADR.

### Export/loss contract: safe-v1

Export computes an immutable effective snapshot, validates relationships, produces
an export plan and a report **before writing**. Report entries have exactly
`code`, `target_format`, `record_id`, `element_id` (null for root), `field`,
`cause_operation_id` (null if imported), `action`, and typed `details`. Sort by
record ID, element ID (null first), field, code; no volatile time in equality.
The companion fixtures define expected entries. No loss means the exact empty list.

- Known safe representable values are exported from the post-edit state.
- Capacity overflow: deterministically withhold hotcue indices outside 0..37 for
  the Mixxx fixture target and report each, never renumber or choose arbitrarily.
- An edited position with unknown alternate-encoding synchronization: withhold
  the whole affected native cue; preserve its bytes internally and report
  `UNSUPPORTED_POSITION_EDIT`. Do not update only one encoding and pretend safety.
- Unresolved explicit beat lock: withhold the cue and report `MISSING_ANCHOR`.
- Unknown native records may be passed unchanged only through a qualified exact
  preservation path. Without one, report `UNSUPPORTED_NATIVE_EXPORT` and block
  that export (`action=BLOCK_EXPORT`); never label synthetic records vendor output.
- Unresolved identity/source conflict or dangling relationships block export
  (`IDENTITY_CONFLICT` or `DANGLING_REFERENCE`). No partially written target.

Internal immutable evidence, operation history, provenance, withheld values and
reports survive only in OxiDJ archival/storage snapshots. Vendor-only export carries
only target-representable effective values, not that archive. No sidecar transport
is specified or assumed. The oracle compares three separate surfaces: archive,
export report, and vendor-visible output; see [fixture contract](005-relationship-fixtures.md).

## Open Questions / What Would Change This

- C1–C6 above are explicit qualification blockers. The entire frozen workload is
  **not implementable without inventing unresolved semantics** yet. Bounded
  logical state/overlay tests can be implemented from this draft after review;
  physical schema and workload construction cannot be inferred from it.
- Exact adapter artifacts/encodings and independent checker bindings in the
  companion remain open, especially real OneLibrary records and Mixxx transport.
  Synthetic preservation success cannot settle empirical vendor meaning.
- Rekordbox TrackID stability, URI normalization and cross-import element matching
  remain unproven; explicit reconciliation is the bounded fallback.
- Complete enumeration of vendor dependencies remains NOT DEMONSTRATED. A total
  projection returns opaque/unsupported/conflict states rather than invented values.
- Sol advisory `t_4d01d430`, comment 129, recommends retaining Proposed. Independent
  review of this revised contract and actual decision-bearing fixture PASS evidence
  are still required before any transition. M1 **NOT READY**, runner **NOT PASSED**;
  no benchmark, fault campaign, dependency admission, commit or push is authorized
  by this draft. Product audio/artwork copy/reference/retention and vendor-application
  acceptance are not decided here.

## Sol Review Packet

**DECISION:** Adopt source-native metadata records with a neutral projection and
explicit user-edit overlays. Store each vendor's metadata structure intact;
compute the neutral domain model as a derived read projection; store user edits
as overlays that reference source-native fields by stable revision-qualified
address; validate known/declared relationship groups before export with deterministic withholding plus loss reports, or block
the export under `safe-v1`.

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


### Bounded raw TRACK.Kind post-edit evidence (M4)

One synthetic F-02 guarded TRACK.Kind edit has a test-only independent quick-xml whole-tree oracle in `xtask/tests/f02_track_kind_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-kind-edit/`. It checks exact-old editing on TrackID 100, absent Kind on TrackID 200, source-byte preservation, and stale/wrong-target and semantic mutation rejection. This is a bounded implementation evidence case only: Kind is raw text; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

### Bounded raw TRACK.Colour post-edit evidence (M4)

One synthetic F-02 guarded TRACK.Colour edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_colour_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-colour-edit/`. It checks exact-old `old & palette`→`new < palette` editing on TrackID 100, absent Colour on TrackID 200, source-byte preservation, and stale/wrong-target plus semantic mutation rejection. Colour remains uninterpreted raw text: no palette/index mapping. This is one bounded implementation evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

### Bounded raw TRACK.AverageBpm post-edit evidence (M4)

One synthetic F-02 guarded `TRACK.AverageBpm` edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_average_bpm_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-average-bpm-edit/`. The oracle checks exact-old `old & bpm`→`new < bpm` editing on TrackID 100, absent AverageBpm on TrackID 200, unchanged source bytes, and rejection of stale/wrong-target and semantic mutations. AverageBpm remains raw text: no numeric or unit interpretation/conversion. This is one bounded implementation evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.
One synthetic F-02 guarded TRACK.DiscNumber edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_disc_number_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-disc-number-edit/`. It checks exact-old `old & disc`→`new < disc` editing on TrackID 100, absent DiscNumber on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. The complete ordered decoded tree includes text and CDATA. DiscNumber remains uninterpreted raw text; no numeric or numbering semantics. This is one bounded implementation evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.TrackNumber edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_number_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-number-edit/`. It checks exact-old `old & track`→`new < track` editing on TrackID 100, absent TrackNumber on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. The complete ordered decoded tree includes text and CDATA. TrackNumber remains uninterpreted raw text; no numeric or ordering semantics. This is one bounded implementation evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.DateAdded edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_date_added_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-date_added-edit/`. It checks exact-old `old & stamp`→`new < stamp` editing on TrackID 100, absent DateAdded on TrackID 200, source-byte preservation, and stale/wrong-target plus semantic mutation rejection. DateAdded remains raw with no date/timezone conversion. This is one bounded implementation evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.Size edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_size_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-size-edit/`. It checks exact-old `old & size`→`new < size` editing on TrackID 100, absent Size on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. The complete ordered decoded tree includes text and CDATA. Size remains uninterpreted raw text; no numeric parsing, units, or conversion. This is one bounded evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.TotalTime edit has a test-only independent `quick-xml` whole-tree oracle in `xtask/tests/f02_track_total_time_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-total-time-edit/`. It checks exact-old `old & time`→`new < time` editing on TrackID 100, absent TotalTime on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. TotalTime remains uninterpreted raw text; no duration parsing, numeric/time interpretation, or conversion. This is one bounded evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.BitRate edit has a test-only independent `quick-xml` ordered decoded whole-tree oracle in `xtask/tests/f02_track_bit_rate_edit_oracle.rs`, with frozen input under `xtask/tests/fixtures/f02-track-bitrate-edit/`. It checks exact-old `old & rate`→`new < rate` editing on TrackID 100, absent BitRate on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. BitRate remains uninterpreted raw text; no numeric parsing, units, or conversion. This is one bounded evidence case only; global F-09 and ADR-005 qualification remain unpassed, with no Rekordbox compatibility claim.

One synthetic F-02 guarded TRACK.SampleRate edit has an independent test-only `quick-xml` ordered decoded whole-tree oracle in `xtask/tests/f02_track_sample_rate_edit_oracle.rs`, using the frozen fixture `xtask/tests/fixtures/f02-track-sample-rate-edit/source.xml`. It checks exact-old `old & rate`→`new < rate` editing on TrackID 100, absent SampleRate on TrackID 200, source-byte preservation, stale/wrong-target guards, and semantic/inserted-text mutation rejection. SampleRate remains uninterpreted raw text; no numeric parsing, Hz interpretation, or conversion. This one bounded evidence case does not establish global F-09, Rekordbox compatibility, or ADR-005 qualification.

The CLI no-edit export regression also preserves synthetic raw TRACK.SampleRate bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; it makes no Rekordbox compatibility or ADR-005 qualification claim.

The CLI no-edit export regression preserves synthetic raw TRACK.Size bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; Size remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.BitRate bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; BitRate remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.TrackNumber bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; TrackNumber remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.DiscNumber bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; DiscNumber remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.TotalTime bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; TotalTime remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.Year bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; Year remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.

The CLI no-edit export regression also preserves synthetic raw TRACK.AverageBpm bytes across present, empty, and absent states, including entity-escaped text. It checks exact output bytes and source immutability; AverageBpm remains uninterpreted raw text and no Rekordbox compatibility or ADR-005 qualification is claimed.
