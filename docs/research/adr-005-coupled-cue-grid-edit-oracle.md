# ADR-005 draft fixture oracle: coupled cue and beat-grid edit

Status: draft specification only. This document proposes one fixture-oracle case; it does not accept a schema, settle relationship semantics, implement an adapter, provide fixture bytes, or claim fixture execution, qualification, or readiness. M1 remains NOT READY / NO-GO; ADR-005 remains Proposed.

## Scope and source boundary

This case makes the ADR-003 field-relationship risk concrete: changing linked metadata may preserve bytes while corrupting meaning. ADR-003 requires independent identity, per-field provenance, preservation of constant versus variable-tempo grids and point cues versus loop ranges, and pre-write loss reporting (ADR-003:134-142). Its required fixture validation checks semantic relationships, provenance, and reported loss, not only parseability or byte retention (ADR-003:134-142).

Current trunk at reviewed commit `1dc92940697913447fe145aac19ada13a686c304` contains ADR-005 and its companion fixture contract: `docs/adr/005-metadata-schema.md` (SHA-256 `7aa8fb87c801759263a4f316a2c1bdd5f19ac5023379aa30dfeb02767b448e98`) and `docs/adr/005-relationship-fixtures.md` (SHA-256 `588970b1578ab8a10990bb150be49c4df4b48a77ef0197d12bf8eba597323fad`). Every file/line citation in this draft refers to blobs at that exact commit—not a moving `origin/master` and not the older assigned worktree checkout. The ADR proposes, but does not accept, the source-native record / neutral projection / edit-overlay direction; it defines `logical-v1` as under review and `safe-v1` as the proposed pre-write export/loss contract (ADR-005:32-68, 145-149, 315-341). The companion defines the archive, export plan/report, and vendor-visible output as distinct surfaces and supplies F-04 and F-07–F-11 cases relevant here (relationship-fixtures.md:32-53, 187-207, 244-328). This draft is supplemental to those existing contracts, not a replacement. The readiness note still says NOT READY (m1-sqlite-readiness-note.md:10-24, 157-180, 302-330); relationship-aware fixtures are a gate, not authorization to qualify or change readiness.

## Proposed fixed fixture scenario

Fixture ID (proposed): `rekordbox-xml-coupled-cue-grid-v1`.

Use one small, deterministic, synthetic Rekordbox XML track record as the eventual fixture source, with a stable fixture-local media identity and a separately recorded source TrackID/Location. Its independently authored expected semantic record contains:

- A variable-tempo grid with two ordered tempo anchors: 120 BPM at 0 seconds and 90 BPM at 60 seconds. Preserve the anchors as a variable-tempo representation; do not replace them with an average BPM or constant-tempo grid.
- One point hotcue at 30 seconds, with its source identifier, type, and imported provenance recorded independently.
- One loop range from 48 to 56 seconds, represented with distinct start and end values and a loop type; it is not a point cue.
- One opaque vendor extension associated with the source record, with its exact original bytes and provenance captured separately from the semantic expectations.
- A source identifier and media location retained as provenance, while OxiDJ's internal identity remains independent of both.

The numeric values above define a proposed test vector, not observed vendor behavior or an accepted product contract. Fixture bytes, source application/version, field encodings, and any source-specific relationship between the vendor extension and grid/cues must be established and independently reviewed before this becomes executable evidence. Rekordbox XML documents multiple TEMPO entries and POSITION_MARK entries with cue/loop types and start/end values, but leaves interpolation, precision, and several cue semantics unspecified (interoperability.md:12-15).

## Coupled edit and proposed oracle

On an imported baseline, apply both edits as one explicit user operation:

1. Move the point hotcue from 30 seconds to 32 seconds.
2. Change the second grid anchor from 90 BPM to 96 BPM, retaining its 60-second position and the first anchor at 120 BPM / 0 seconds.

Supplemental proposed operation vector, not a second oracle framework: combine this case's simultaneous cue-position move (30→32 seconds) and variable-tempo edit (second anchor 90→96 BPM) as one user operation. Map its requirements to existing F-04 (USER edits, revisions, archive/history), F-07 (tempo edit leaves the bounded absolute XML mark unchanged), F-08 (unknown position-encoding loss handling), F-09 (independently authored and separately checked artifacts), F-10 (ordered variable-tempo points without invented interpolation), and F-11 (point/range/absence distinctions). It adds no vendor dependency and does not override or duplicate those cases. Whether this combined vector is suitable as a supplemental subcase remains proposed.

Expected archive assertions for the proposed vector (separate from expected export/report/output):

- The edited point cue is at 32 seconds and remains a point cue; its user-edit provenance and baseline value (30 seconds) remain distinguishable.
- The grid remains variable-tempo with two anchors at the stated positions and BPM values; no average-BPM flattening occurs.
- The loop remains a distinct range from 48 to 56 seconds, not a point cue. This is a bounded proposed expectation for this vector; it does not promise that every loop in every format remains unchanged, nor does it leave this vector's loop behavior as an alternative. No broader loop rule is asserted.
- Internal track identity does not become the vendor TrackID or path; source identifiers remain source-scoped provenance.
- Immutable source bytes/baseline, overlays, operation history, and USER provenance remain in the archive. A fresh vendor-only import is not expected to reconstruct history or provenance.

Freeze `expected-archive.json`, `expected-plan.json`, `expected-report.json`, and `expected-visible.json` as separate expected outputs: the plan and report are distinct, and neither is folded into archive assertions or vendor-visible output. Compare each independently under the companion contract. A fresh local re-import is only an additional integration check against `expected-visible.json`, never the independent checker or an archive/history assertion. Fresh local IDs are compared through the declared one-to-one correspondence, not by equality or history recovery. These remain proposals for a future independently checked fixture, not accepted behavior. The selected bounded XML absolute mark is independent of the tempo edit per F-07; do not claim beat-relative conversion, tempo coupling, or interpolation. Any additional source extension relationship requires evidence and qualification, not assumption.

## Dependency handling and export outcome

Apply only the reviewed ADR-005 `safe-v1` target, unit, and report contract; do not invent a report code, an XML extension qualification path, or a new policy allowance. For a known edited position with unknown alternate-encoding synchronization, the current safe-v1 contract withholds the whole affected native cue and reports `UNSUPPORTED_POSITION_EDIT` (ADR-005:315-341; relationship-fixtures.md F-08b). For unknown native records, unchanged passthrough is allowed only through a qualified exact-preservation path; otherwise `UNSUPPORTED_NATIVE_EXPORT` blocks that export. If no independent safe export unit or qualified exact-preservation path exists for this proposed vector, specify `BLOCK_EXPORT` with no target mutation. Do not authorize a new lossy export or broaden the reviewed report semantics. Any more general dependency-group rule remains proposal-only and requires its own review.

For a representable target, use only the reviewed safe-v1 expected plan/report and independently inspect vendor-visible output. If this vector encounters unknown native scope without a qualified exact-preservation path or independently safe unit, the proposed outcome is `BLOCK_EXPORT` and no target mutation—not a guessed group-level lossy export. Keep any broader scope expansion or loss-policy change proposal-only; never infer that a successful archive check authorizes export.

## Evidence and limits

- `idea.md:22-25, 34` requires vendor adapters, a neutral domain model, no silent metadata loss, reproducible import/export round trips, and safe use of real libraries.
- `docs/adr/003-metadata-representation.md:134-142` supplies the binding identity, provenance, non-destructive export, relationship-risk, and decision-bearing fixture constraints.
- `docs/adr/005-metadata-schema.md:32-68, 145-149, 315-341` defines the proposed direction, under-review `logical-v1`, and `safe-v1`; `docs/adr/005-relationship-fixtures.md:32-53, 57-70, 187-207, 244-328` defines the distinct comparison surfaces and relevant companion cases.
- `docs/research/interoperability.md:12-18, 22-30` bounds what public Rekordbox XML evidence establishes and requires conservative, non-destructive fixture-based interchange; it does not establish OneLibrary writing or undocumented-field semantics.
- `docs/research/m1-sqlite-readiness-note.md:10-24, 157-180, 302-330` records that the relationship-aware semantic oracle remains NOT READY and the current gate checklist.
- Kanban handoff `t_7956b0ec` and routing note `t_9641e861` define the proposed starting direction and minimum oracle conditions; neither supplies fixture execution evidence.

No fixture bytes were generated or executed for this specification. No schema, adapter, persistence dependency, benchmark, fault campaign, or qualification is authorized by it. M1 remains NOT READY / NO-GO; ADR-005 remains Proposed; runner remains NOT PASSED; A1 remains OPEN / NOT QUALIFYING.
