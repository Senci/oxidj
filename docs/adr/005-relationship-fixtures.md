# ADR-005 Relationship Fixtures

**Status:** Proposed logical-v1 contract — A1 synthetic XML fixture tests pass; all other fixture results and overall promotion remain unqualified.
**Date:** 2026-10-16
**Owner:** OxiDJ lead
**Companion:** [ADR-005](005-metadata-schema.md)

This replaces the earlier contradictory expected-state plan in response to Sol
advisory `t_4d01d430`, comment 129. It specifies bounded logical tests and required
adapter artifacts; it does not claim those artifacts or adapters already exist.
M1 remains **NOT READY**, runner **NOT PASSED**, ADR-005 **Proposed**. No benchmark,
fault campaign or persistence-dependency admission is authorized. Product media
copy/reference/retention and vendor-application/hardware acceptance are out of scope.

## 1. Evidence and boundaries

- Mixxx evidence is pinned to 2.5.6, commit
  `3ebac449e7e5fe2a0186596657696e87ce8b0e56`:
  [beats.proto](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/proto/beats.proto)
  defines BeatMap as beat positions with enabled/source, **not BPM per beat**;
  [cueinfo.h](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/cueinfo.h)
  defines optional endpoints in milliseconds, index zero, label, optional RGB and
  flags. DTO flag retention is not Mixxx database retention.
- [Rekordbox XML format list](https://cdn.rekordbox.com/files/20200410160904/xml_format_list.pdf)
  defines XML playlist sharing, TEMPO and second-based POSITION_MARK fields.
  XML sibling order is not the OneLibrary `sequenceNo` column. XML does not
  establish tempo-to-absolute-cue invalidation or beat interpolation.
- [Interoperability research](../research/interoperability.md) and
  [semantic mapping](../semantic-mapping.md) bound OneLibrary knowledge. A
  third-party table-field description is not a OneLibrary wire format.

Three different surfaces are tested; none implies the others:

1. **Archive:** immutable source artifacts, record/element IDs, revisions,
   source assertions, overlays, analysis candidates and effective state. Restore
   or inspection of this state can prove internal history retention.
2. **Export plan/report:** effective values, omissions and blocking decisions
   under ADR-005 `safe-v1`, checked before writing. Reports are separate artifacts.
3. **Vendor-visible output:** only representable post-edit/post-loss values.
   Fresh vendor-only import has new import provenance and local IDs. It cannot
   reconstruct source history, reports or withheld fields. No preservation sidecar
   is defined. Archival restore is not vendor re-import.

### Relationship registry

- `RG-IDENTITY-LOCATION`: explicit reconciliation and retained source assertions.
- `RG-PLAYLIST-TRACK`: endpoint integrity, hierarchy and membership order.
- `RG-BEATGRID-CUES`: only explicitly declared beat locks; absolute Mixxx cues
  and XML marks are independent of tempo edits in the bounded fixtures.
- `RG-OPAQUE-POSITION`: OxiDJ conservative policy for unqualified alternate
  position encodings, not an empirically demonstrated synchronization rule.
- `RG-ASSET-STUB`: test-only live reference and boundary-manifest equality.
- Key↔anchor coupling is **NOT DEMONSTRATED**; no `RG-KEY-ANCHOR` validator is enabled.

## 2. Exact logical artifacts and equality

For each case/subcase the future fixture directory must contain:

| Artifact | Exact role |
|---|---|
| `case.json` | `contract_version` = `logical-v1`, case ID, source kind/version, ordered record/element IDs and source assertions, target format and `safe-v1` |
| `source.bin` | Exact raw source bytes; for logical-only cases canonical JSON bytes defined below, explicitly not vendor bytes |
| `operations.json` | Ordered mutations with operation ID, target address, expected revision, typed old/new values and expected result |
| `expected-archive.json` | Hand-reviewed initial/final native records, history, revisions, provenance and effective values, including rejected-operation invariance |
| `expected-visible.json` | Explicit target-visible post-edit/post-policy values, order and omissions; null when export is blocked |
| `expected-plan.json` | Exact pre-write decision, selected/withheld units, report, and whether a target write is permitted or must not occur |
| `expected-report.json` | Exact ordered `safe-v1` entries; empty list when no loss |
| `provenance.json` | Artifact paths/lengths/SHA-256, source version/citation, logical vs real-format classification, checker identity/version |

These are artifact requirements, **not files produced by this documentation card**.
No ellipses, placeholder hashes or host-dependent paths may enter executable goldens.
Missing files/real-format bindings are NOT DEMONSTRATED, never implicitly generated
by the production importer. Future writers must freeze complete JSON records under
the rules below, review their exact bytes and record digests before implementation
results count. The recipes here define all fixture-specific semantic values; actual
adapter envelopes still require the bindings in §5.

Logical JSON uses UTF-8, no BOM, lexicographically sorted object keys, compact
separators and one LF. Arrays preserve order. BPM is an exact decimal string;
times, frames, indices and raw synthetic integer fields are exact JSON integers,
never converted through host floats. Units are explicit in field names/context.
The displayed objects/recipes are source payloads, not complete archive envelopes.
IDs use case-local
ASCII names: track `t1`, native record `r1` at source revision `s1`, cue elements
`c0`, `c1`, etc.; grid root `g1`, beats `b0`, `b1`, etc. Different named native
structures get different record IDs (`r-grid`, `r-cues`); they share track `t1`.
Every operation names the actual record/address, not `/CueInfo/0`. Unless stated
otherwise source namespace is the case ID, imported origin is its VENDOR label
(or `SYNTHETIC` for invented records), effective revision is 0, overlay and
analysis lists are empty. Unmentioned optional values are **absent**, not zero/null.
Each sequential accepted operation expects the immediately previous revision and
increments it once; rejected operations do not increment it. Subcases reset state.

- Times in logical goldens are integer microseconds; XML seconds × 1,000,000 and
  CueInfo milliseconds × 1,000 must be exact for these chosen values. BeatMap uses
  integer audio frames with explicit 48,000 frames/s. Preserve raw source values
  as well as exact converted values. No sample rounding, epsilon, interpolation,
  inferred bar number or inclusive/exclusive loop-end claim.
- BPM uses exact decimal strings (`128`, not a rounded average); XML decimal
  lexical variants may compare by exact rational value, but raw archive bytes
  still compare byte-for-byte. No tolerance is granted by these fixtures.
- Text compares Unicode scalar sequences without normalization or case folding;
  optional color in logical CueInfo is RGB bytes red=(255,0,0), blue=(0,0,255),
  green=(0,255,0). No claim that these are vendor palette IDs.
- Native artifact equality means byte length and SHA-256 plus byte comparison.
  Semantic XML equality compares required names/attributes/values, ordered
  child lists and absence; attribute order/whitespace are not semantic equality.
  Unknown XML/native payloads must remain archived even if no writer qualifies.
- Archive IDs/revisions are exact. Fresh-import IDs use an explicit bijection
  fixed by expected locations and case-local external keys, not the importer’s
  own reconciliation. A missing/extra/duplicate row or edge fails equality.

### Deterministic report equality

Every entry has the exact keys specified by ADR-005. For these cases `field` is
`identity`, `position`, `anchor` or `hotcue_index`; target format is the case's
logical target (`mixxx-library-logical` for F-01, `mixxx-cueinfo-logical` for other
Mixxx-derived cases, `rekordbox-xml`, or `onelibrary-synthetic`). A separate blocked
real OneLibrary attempt uses target `onelibrary`, never the synthetic label. WITHHOLD is `action=WITHHOLD`; blocking is
`action=BLOCK_EXPORT`. IDs refer to pre-export archive IDs, never fresh-import IDs.
Details are exactly: overflow `{index:i,maximum_index:37}`; position edit
`{old_usec:10000000,new_usec:11000000}`; missing anchor `{anchor_element_id:"b1"}`;
identity conflict `{reason:reason}`; unsupported native export
`{reason:"unqualified-wire-format"}`. Reasons for F-01 are specified there.
Entries sort as ADR-005 specifies; no extra diagnostic text participates in equality.

## 3. F-01–F-12 expected-state cases

### F-01: Identity, relocation, conflicts and provenance

Logical Mixxx-derived input A (`r1/s1`) has path `/music/track1.flac`, external ID
`100`, grid 128 BPM/anchor 500000 usec, cue c0 at 10000000 usec, index 0,
label `Intro`. B (`r2/s1`) has the same values and new path
`/music/relocated/track1.flac`. Both content witnesses reference the exact UTF-8
bytes `oxidj-f01-content-v1\n` (synthetic evidence, **not audio**).

Operations: import A as t1; import B as unresolved candidate; explicit user-confirmed
reconcile B to t1 with matching byte evidence and expected revision 0. Then select
B as the current location/source assertion at revision 1. Final revision 2, one
track, both immutable records, reconciliation and selection history, no USER
metadata edits. Export-visible state has **one current location**, B, and the
unchanged grid/cue; report `[]`. Archive retains A and B. Fresh vendor-only import
contains one track corresponding to t1; it need not have the same ID or old path.

**Observed logical-only result (same-content relocation success subcase only):** `xtask/tests/fixtures/f01-relocation-logical/` freezes canonical source bytes with the exact shared synthetic content witness, both immutable source assertions, operation history, archive, visible location/grid/cue, safe-v1 plan/report, vendor-only re-import and provenance. `xtask/tests/f01_relocation_logical.rs` independently checks reconciliation at revision 0, source selection at revision 1, final revision 2, exact artifact digests and the one-track re-import; mutation controls reject changed history/revision/current location, cue movement, re-import count and report changes. Only this logical success subcase is PASS. F-01 conflict subcases remain NOT DEMONSTRATED; no Mixxx transport, A2, collision, near-duplicate or URI claim is made.

**Observed logical-only result (same-namespace/source-ID content-conflict subcase):** `xtask/tests/fixtures/f01-conflict-logical/` freezes synthetic A at r1 and candidate B at r2 with namespace and external ID `100`, differing exact content witness `oxidj-f01-different-v1\n`, unchanged original A, retained r2 candidate, blocked safe-v1 export and exact `IDENTITY_CONFLICT` report. `xtask/tests/f01_conflict_logical.rs` independently checks source, operations, archive, visible, plan, report, provenance and exact artifact set; mutation controls reject merge, source mutation, missing candidate and report/plan changes. This subcase is logical-only PASS; other F-01 conflicts, Mixxx A2 and ADR-005 promotion remain NOT DEMONSTRATED/OPEN.

`oxidj-f01-different-v1\n` → reason `source-id-content-disagreement`;
(b) different namespaces sharing ID 100, without confirmed reconciliation →
`unconfirmed-identity`; (c) same path but different content → `path-content-disagreement`.

**Observed logical-only result (same-path/different-content conflict subcase):** `xtask/tests/fixtures/f01-same-path-logical/` freezes r1/r2 at the exact same path with distinct literal namespaces and external IDs and different synthetic content witnesses. No merge occurs: r1 remains current and r2 unresolved; safe-v1 export is blocked with `IDENTITY_CONFLICT`, reason `path-content-disagreement`. `xtask/tests/f01_same_path_logical.rs` independently checks source, operations, archive, visible, plan, report, provenance and exact artifact set; mutation controls reject path/content drift, merge, missing candidate and report/plan changes. This subcase is logical-only PASS; other F-01 conflicts, Mixxx A2 and ADR-005 promotion remain NOT DEMONSTRATED/OPEN.

**Observed logical-only result (unconfirmed cross-namespace identity subcase):**
`xtask/tests/fixtures/f01-unconfirmed-logical/` freezes distinct literal source
namespaces with shared external ID 100 and the exact same synthetic content witness.
No reconciliation occurs: r1 remains the selected original and r2 an unresolved
candidate; safe-v1 export is blocked with the exact `IDENTITY_CONFLICT` report,
reason `unconfirmed-identity`. `xtask/tests/f01_unconfirmed_logical.rs` independently
checks source, operations, archive, visible, plan, report, provenance and exact
artifact set; mutation controls reject silent merge, namespace/ID/content drift,
added confirmation and report/plan changes. Logical-only PASS; other F-01 conflicts,
Mixxx A2 and ADR-005 promotion remain open.

No silent merge, original unchanged, export blocked with `IDENTITY_CONFLICT`,
record `r2`, null element/cause, field `identity`, exact reason above. A fourth
subcase confirms relocation but B says BPM 129: retain both assertions, project
BPM conflict until explicit source selection; its export is blocked with the same
r2/root `IDENTITY_CONFLICT` shape and reason `metadata-source-disagreement`.
Selecting B resolves effective BPM to129 while A remains128 in history. Do not
infer that equal content makes metadata equal. Content-hash collision safety, acoustic near-duplicate
matching and platform URI normalization remain NOT DEMONSTRATED.

**Observed logical-only result (metadata-source disagreement subcase):**
`xtask/tests/fixtures/f01-metadata-conflict-logical/` freezes the same-content
relocation assertions (A=128 BPM, B=129 BPM), reconciliation and source-selection
history, retained A/B archive assertions, selected B location and effective BPM.
The preselection archive state records the exact blocked safe-v1 `IDENTITY_CONFLICT`
for r2 (`field=identity`, null element/cause, reason
`metadata-source-disagreement`). `xtask/tests/f01_metadata_conflict_logical.rs`
independently checks canonical source, operations, archive, visible, plan, report,
provenance and exact artifact set; mutation controls reject dropped assertions,
wrong effective BPM, unblocked export and report drift. This is a logical-only
fixture, not A2/Mixxx transport evidence; ADR-005 remains Proposed.

### F-02: XML order and cross-record endpoints

XML-format artifact binding must encode four COLLECTION tracks with TrackIDs
100/200/300/400 and Locations `file://localhost/music/a.flac` through `d.flac`.
Folder `Sets` contains playlists `House` then `Techno`; House's XML TRACK children
reference keys `[100,200,300]`, Techno `[200,400]` in precisely that order.
Use XML playlist key-type TrackID as documented, not a fabricated `sequenceNo`
attribute. Full envelope/numeric enum binding remains A1 in §5.

No edits: expected ordered trees exactly as above, five distinct membership IDs,
track 200 shared, four tracks; report `[]`. Logical mutation subcases reorder
House to `[300,100,200]` without changing membership IDs; reject a dangling track
reference, deleting track 200 without its memberships, and a folder cycle with
no state change. Explicit removals of both memberships plus deletion are one
atomic logical mutation. Vendor application acceptance remains NOT DEMONSTRATED.

**Observed synthetic XML edit:** `reorder-playlist` exports House `[100,200,300]`
to `[300,100,200]` using a playlist ordinal, exact old-order guard and unique
permutation; it writes to a new path. Adapter tests reject stale and invalid
orders. A duplicate-name/identical-membership regression proves only the
selected NODE changes. An independent ElementTree decode confirmed the F-02 CLI
output, unchanged Techno membership, four collection tracks and declared counts.
Sol approved commits `812a414` and `a50001d`; `cargo xtask ci-full` passed.
This remains synthetic-only; vendor application acceptance is NOT DEMONSTRATED.

**Observed synthetic XML playlist-member removal (F-02):** TrackID 200 was removed only from House after guarding playlist ordinal 0 and its exact ordered membership `[100,200,300]`. The collection remains `[100,200,300,400]`; House becomes `[100,300]`; Techno remains `[200,400]`; direct counts are `4/2/2`. An independent Python ElementTree decode of the exported file confirmed those values and the source file remained byte-identical. This is synthetic behavior only; it does not demonstrate Rekordbox application acceptance, ADR-005 promotion, or qualification.

**Observed synthetic XML playlist-member append (F-02):** TrackID 100 was appended to Techno after guarding playlist ordinal 1 and exact old order `[200,400]`. Techno becomes `[200,400,100]`; House remains `[100,200,300]`; collection remains `[100,200,300,400]`; direct counts are `4/3/3`. Adapter and CLI regressions cover the ordered change, stale guard, duplicate/unknown target refusal and source immutability. This is synthetic behavior only; it does not demonstrate Rekordbox application acceptance, ADR-005 promotion, or qualification.

An additional in-memory F-02 case appends TrackID 100 to an empty self-closing Techno NODE with an exact empty-order guard. The exporter expands only that NODE, producing Techno `[100]` and direct counts `4/3/1`; adapter and CLI tests also reject a stale nonempty guard and confirm the input bytes stay unchanged. This remains synthetic-only evidence.

**Observed synthetic XML deletion:** F-02 TrackID 200 is deleted from the collection and both guarded memberships in House and Techno in one new-file export. The request requires exact old ordered memberships for every playlist containing the target; stale/incomplete guards and missing/duplicate target references are rejected. The adapter updates `COLLECTION Entries` and the affected playlist `NODE Entries` while retaining all unrelated source bytes. Focused adapter/CLI tests exercise export, refusal and source immutability; independent ElementTree decode observed collection `[100,300,400]`, House `[100,300]`, Techno `[400]`, and counts `3/2/1`. This is synthetic F-02 behavior only; Rekordbox application acceptance, ADR-005 promotion and qualification remain NOT DEMONSTRATED.

**Observed synthetic XML playlist clear (F-02):** House was selected by ordinal 0 and exact old order `[100,200,300]`; one new-file export clears its memberships and sets its direct `Entries` to 0. The collection remains `[100,200,300,400]`, Techno remains `[200,400]`, and counts are `4/0/2`. Adapter and CLI tests cover stale order, malformed source count and source immutability. This is synthetic XML behavior only; Rekordbox application acceptance and qualification remain NOT DEMONSTRATED.

**Observed synthetic XML playlist-member insertion (F-02):** TrackID 100 was inserted at zero-based position 1 in Techno after guarding `[200,400]`, yielding `[200,100,400]`. House and collection remain unchanged; only Techno membership and its `Entries` count change. Adapter/CLI tests reject stale order, an out-of-range position, already-present and unknown targets. This remains synthetic XML-only behavior; Rekordbox app/hardware acceptance and qualification are NOT DEMONSTRATED.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist-member reorder:** `xtask/tests/fixtures/f02-playlist-member-reorder/source.xml` freezes the synthetic source. `xtask/tests/f02_playlist_member_reorder_oracle.rs` applies the existing guarded adapter and independently decodes the result with test-only `quick-xml`, not the production importer. House changes from `[100,200,300]` to `[300,100,200]`; Techno remains `[200,400]`; playlist/folder order and Counts, collection tracks/metadata/Entries, and source bytes remain exact. Wrong membership order, Entries, collection, and Count mutations are rejected; stale and non-permutation adapter requests refuse. This is one synthetic case only; global F-09 remains **NOT DEMONSTRATED**, and no vendor compatibility or ADR-005 qualification is claimed.

**Observed synthetic XML empty-folder creation (F-02):** A paired non-root Type=0 parent is selected by folder ordinal and exact Name; creation adds one empty Count=0 child with XML-escaped Name, rejects duplicate sibling names, and increments only the selected parent's direct-child Count. Adapter and CLI regressions verify source immutability, output preflight, unchanged tracks/playlists, escaping, collision/stale/root/invalid-text/malformed-input refusals, and no output on refusal. Local implementation evidence only; it does not demonstrate Rekordbox/OneLibrary compatibility, ADR-005 promotion, or qualification.

**Observed synthetic XML empty-playlist creation (F-02):** A paired non-root Type=0 parent is selected by folder ordinal and exact Name; creation adds one empty Type=1 child with XML-escaped Name, `KeyType=0` (TrackID), and `Entries=0`, rejecting duplicate sibling folder/playlist names and incrementing only the selected parent's direct-child Count. Adapter and CLI regressions cover source immutability, new-path export preflight, unchanged collection tracks and prior playlists/order, escaping, collision/stale/root/invalid-text/malformed-input refusals, and no output on refusal. Synthetic local implementation only; no Rekordbox/OneLibrary compatibility claim, ADR-005 promotion, or qualification.

**Observed synthetic XML empty-folder deletion (F-02):** A non-root Type=0 folder was selected by ordinal and exact Name; deletion required `Count=0` and no child elements, and decremented only the immediate parent folder's `Count`. Adapter tests cover paired and self-closing empty nodes plus root, nonempty, stale, out-of-range, and bad-count refusal; a CLI test confirms new-file export and unchanged source bytes. Sol approved commit `02588b3` and independently passed `cargo xtask ci-full`. Independent post-edit evidence for one case is frozen in `xtask/tests/fixtures/f02-empty-folder-delete/source.xml`; `xtask/tests/f02_empty_folder_delete_oracle.rs` runs the guarded adapter and decodes output with test-only `quick-xml`, not the production importer. It asserts the selected Empty folder alone disappears, Sets Count decrements, remaining folders/playlists/order/memberships and collection remain unchanged, and input bytes remain intact; wrong-node, Count and semantic mutations are rejected. Focused test passed. This bounded synthetic case does not demonstrate Rekordbox/OneLibrary compatibility, ADR-005 promotion, or qualification.

**Observed synthetic XML sibling-playlist reorder (F-02):** Direct Type=1 playlist nodes are reordered within an ordinal/name-guarded paired non-root folder, with exact current-order and unique-permutation guards. Only playlist NODE slots move; an interleaved child folder, counts, memberships, collection tracks and source bytes remain unchanged. Adapter and CLI regressions cover stale/duplicate/invalid/root/stale-parent/malformed refusal, new-file export and source immutability. Sol approved commit `3896976` and independently passed `cargo xtask ci-full` (exit 0; `ci=ok test=ok deny=ok`). This synthetic XML behavior does not demonstrate Rekordbox/OneLibrary compatibility, ADR-005 promotion, or qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 sibling-playlist reorder:** `xtask/tests/fixtures/f02-sibling-playlist-reorder/source.xml` freezes a source with an interleaved child folder. `xtask/tests/f02_sibling_playlist_reorder_oracle.rs` runs the existing guarded reorder and decodes its output with test-only `quick-xml`, not the production importer. It asserts the requested direct playlist slots move around the unchanged folder slot, exact House/Techno ordered memberships and Entries, unchanged collection tracks/Entries and Sets/folder Counts, and frozen source-byte immutability. Mutated output with playlist order, Count, Entries, or membership drift is rejected. This is one synthetic F-02 case only; global F-09 remains **NOT DEMONSTRATED**, and no Rekordbox/OneLibrary compatibility or ADR-005 qualification is claimed.

**Observed F-09 post-edit independent evidence for one synthetic F-02 sibling-folder reorder:** `xtask/tests/fixtures/f02-sibling-folder-reorder/source.xml` freezes a synthetic source with two sibling folders, an interleaved playlist, and nested child playlists/folder. `xtask/tests/f02_sibling_folder_reorder_oracle.rs` runs the existing guarded reorder and independently decodes output using test-only `quick-xml`, not the production importer. It asserts direct folder slots/order and Counts, nested folder/playlist paths, exact ordered membership and Entries, collection tracks/count, and frozen source immutability; complete-folder-node reorder, direct/nested Count, membership, and Entries mutations are rejected. This is one synthetic case only; global F-09 remains **NOT DEMONSTRATED**, and Rekordbox/OneLibrary compatibility and ADR-005 qualification are not claimed.

**Observed F-09 post-edit independent evidence for one synthetic F-02 empty-playlist creation:** `xtask/tests/fixtures/f02-empty-playlist-create/source.xml` freezes a synthetic collection and ordered folder/playlist tree. `xtask/tests/f02_empty_playlist_create_oracle.rs` runs the existing guarded creation adapter, then independently decodes output with test-only `quick-xml`, not the production importer. It asserts the new Type=1 playlist is appended under the selected parent with KeyType=0 and Entries=0, preceding playlists/order and memberships plus the collection remain unchanged, only the selected parent Count increments, and frozen source bytes stay intact. Count, KeyType, Entries, prior-field, and child-order mutations are rejected. This is one synthetic case only; global F-09 remains **NOT DEMONSTRATED**, and no Rekordbox/OneLibrary compatibility or ADR-005 qualification is claimed.

**Observed synthetic XML cross-parent folder move (F-02):** One ordinal/name-guarded non-root Type=0 subtree moves to a different non-root parent after exact typed-child-order guards. Same-parent and descendant/cycle targets are refused. Only the source and target direct-child Counts change; subtree bytes, collection tracks and playlist memberships remain unchanged, and a self-closing empty destination expands. Adapter and CLI regressions pass in commit `1b1676f`; `cargo xtask ci-full` passes (exit 0; `ci=ok test=ok deny=ok`). Independent post-edit F-09 evidence is added by `xtask/tests/f02_folder_move_oracle.rs` over the frozen synthetic input `xtask/tests/fixtures/f02-folder-move/source.xml`: the guarded adapter output is decoded with test-only `quick-xml`, not the production importer. It asserts source/destination folder order, full nested subtree path, unchanged ordered playlist memberships and collection tracks, direct Counts, and source immutability; order, direct Count, nested subtree Count and membership mutations are rejected. Focused test passed. This is one bounded synthetic F-02/F-09 case; global F-09 remains NOT DEMONSTRATED. No Rekordbox/OneLibrary compatibility, ADR-005 promotion, or qualification is claimed.

**Observed F-09 post-edit independent evidence for one synthetic F-02 Type=1 playlist deletion:** frozen source `xtask/tests/fixtures/f02-playlist-delete/source.xml` passes through the existing guarded deletion adapter; `xtask/tests/f02_playlist_delete_oracle.rs` decodes output with test-only `quick-xml`, not the production importer. It verifies exact selected playlist removal, selected-parent Count decrement, remaining folder/playlist order and memberships, collection identity, and source-byte immutability; wrong-node, Count, membership, and child-order mutations are rejected. One synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist clear:** frozen source `xtask/tests/fixtures/f02-playlist-clear/source.xml` passes through the existing guarded clear adapter; `xtask/tests/f02_playlist_clear_oracle.rs` decodes output with test-only `quick-xml`, not the production importer. It verifies the selected House playlist has `Entries=0` and no TRACK children, Techno and all playlist/folder order and memberships remain unchanged, collection TrackIDs/Entries and Counts are correct, and source bytes are unchanged. Wrong-playlist, Entries, membership and child-order mutations are rejected. One synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist rename:** frozen source `xtask/tests/fixtures/f02-playlist-rename/source.xml` passes through the existing guarded rename adapter; `xtask/tests/f02_playlist_rename_oracle.rs` decodes output with test-only `quick-xml`, not the production importer. The independently constructed expected tree changes only the selected playlist Name; ordered playlists, paths, other names, KeyType/Entries/memberships, folder Counts, collection tracks/locations and source bytes remain exact. Wrong-node/name, membership, playlist-order and Count mutations are rejected. This is one synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 empty-folder creation:** frozen source `xtask/tests/fixtures/f02-empty-folder-create/source.xml` passes through the existing guarded create adapter; `xtask/tests/f02_empty_folder_create_oracle.rs` decodes output with test-only `quick-xml`, not the production importer. It verifies the new `Practice` folder under `ROOT/Sets`, empty with Count=0, selected-parent Count increment, unchanged prior playlist order/memberships and collection, and source-byte immutability. Parent Count, new-folder name/Count, child-order, and membership mutations are rejected. One synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 folder rename:** frozen `xtask/tests/fixtures/f02-folder-rename/source.xml` passes through the existing guarded rename adapter; `xtask/tests/f02_folder_rename_oracle.rs` decodes output with test-only `quick-xml`, not the production importer. Only the selected folder Name/path changes; descendants, playlist paths/order/memberships, sibling order, Counts, collection tracks/locations and source bytes remain exact. Wrong-folder/name, descendant/path, child-order, membership and Count mutations are rejected. This is one synthetic case only: global F-09 remains **NOT DEMONSTRATED**, and vendor compatibility/ADR-005 qualification remain unclaimed.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist-member removal:** frozen source `xtask/tests/fixtures/f02-playlist-member-delete/source.xml` passes through the existing guarded member-delete adapter; `xtask/tests/f02_playlist_member_delete_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer. It asserts only House loses TrackID 200, Techno retains `[200,400]`, House order is `[100,300]`, both `Entries` values remain correct, playlist/folder order and Counts remain exact, the collection's four tracks/metadata are unchanged, and source bytes remain immutable. Wrong-playlist, membership, Entries, folder Count/collection, and playlist-order mutations are rejected. One synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 shared-track deletion:** frozen `xtask/tests/fixtures/f02-track-delete/source.xml` passes through the existing guarded `prepare_track_delete` adapter; `xtask/tests/f02_track_delete_oracle.rs` independently decodes its result with test-only `quick-xml`, not the production importer. TrackID 200 and its House/Techno references disappear; remaining collection records, metadata and order, playlist memberships/order/Entries and folder Counts remain exact, as do source bytes. Membership, collection, metadata and playlist-order mutations are rejected, and stale membership guards refuse. This is one synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist-member append:** frozen source `xtask/tests/fixtures/f02-playlist-member-append/source.xml` passes through the existing guarded append adapter; `xtask/tests/f02_playlist_member_append_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer. It asserts only Techno gains TrackID 100 in order `[200,400,100]`, House remains `[100,200,300]`, playlist paths/order and Entries, folder order/Counts, collection tracks/metadata, and source bytes remain exact. Wrong-playlist, stale membership, Entries, collection, and playlist order mutations are rejected. This is one synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 playlist-member insertion:** frozen source `xtask/tests/fixtures/f02-playlist-member-insert/source.xml` passes through the existing guarded insertion adapter; `xtask/tests/f02_playlist_member_insert_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer. It asserts Techno becomes `[200,100,400]`, while House, playlist/folder order and Counts, collection tracks/metadata, and source bytes remain exact. Wrong-playlist, position/order, Entries, collection and stale-membership controls are rejected. This is one synthetic case only: global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox/OneLibrary compatibility claim or ADR-005 qualification.

### F-03: Constant grid, cues and range unchanged

Mixxx-derived **logical CueInfo DTO**, not a claimed Mixxx library transport.
Grid: BPM 128, first beat 96000 frames at 48000 frames/s (2000000 usec), source
`ANALYZER`. Cues: c0 HotCue/index0/start10000000/label `Intro`/red;
c1 HotCue/index1/start20000000/label `Drop`/blue;
c2 Loop/start15000000/end25000000/label `Loop 1`;
c3 MainCue/start5000000. Hotcue/maincue ends and unlisted indices/colors are absent;
flags are 0. No edit. Archive and visible values unchanged, report `[]`.
The logical export target retains the native ANALYZER grid-source token, not an
OxiDJ analysis origin or a claim about any eventual library transport.
Fail on index-zero loss, end fabrication, range collapse or changed grid.
Real Mixxx transport/DB fidelity remains A2, not implied by DTO equality.

**Observed logical-only result:** `xtask/tests/fixtures/f03-logical/` freezes canonical
source bytes, case, operations, archive, visible, plan, report and provenance;
`xtask/tests/f03_logical.rs` independently checks the literal contract and rejects
index-zero loss, seconds or milliseconds substituted for microseconds, fabricated
point end, collapsed loop range and changed grid. These unit-swap controls add
F-09.1 evidence for this synthetic F-03 checker only; global F-09 and Mixxx A2
remain NOT DEMONSTRATED.
This is not A2 evidence or a real Mixxx transport claim.

### F-04: Edits, composition, conflicts and analysis precedence

Start with F-03 grid and only c0. Imported origin is `VENDOR:Mixxx` with retained
native beat-source token `ANALYZER`, not a claim of OxiDJ analysis. Operation e1
sets c0.label `Intro`→`Opening` at revision 0; e2 sets grid BPM 128→130 at revision 1.
Final revision 2; source bytes unchanged; effective c0 start10000000/label `Opening`,
grid BPM130/anchor2000000; effective changed fields USER; report `[]`.

Continuation: e3 label `Opening`→`Opening 2` at revision 2 succeeds (revision 3).
Stale e4 (`Intro`→`Wrong`, expected 0) returns Conflict, no change. Replay e3
identically makes no change; reuse e3 with different payload errors. Separate reset subcase: reorder the F-03 cue collection to c1,c0,c2,c3 at revision 0, then at revision 1 edit c0.label `Intro`→`Opening`; final revision 2, with c1 still `Drop`. Preserve source bytes and every other cue field/order; archive retains originals/history, visible has effective order only, report `[]`. A new source revision does not inherit
these overlays automatically.

Analysis a1, current source/content, analyzer `m1-synthetic`/`1`, sequence 1,
BPM129 at revision 3, appends candidate and advances to revision 4; USER130 remains
effective. Explicit clear-user at revision 4 reveals 129 (revision 5). Stale-source
analysis is rejected without change. Archive contains originals, all accepted
operations and candidate; visible values contain only effective values, no history.
Fresh vendor-only re-import must **not** be required to reconstruct that archive.

**Observed logical-only result (initial accepted-edit composition subcase):** `xtask/tests/fixtures/f04-compose-logical/` freezes the canonical one-cue F-03-derived source, ordered e1/e2 revisions, archive originals and USER overlays, effective visible grid/cue, safe-v1 plan, empty report and provenance with exact artifact digests. The independent literal checker `xtask/tests/f04_compose_logical.rs` verifies exact source values/bytes, operation order/revisions, original/effective values, provenance, report and artifact set; mutation controls reject revision/order/source/effective/archive-history/report/provenance drift. Checker identity is frozen externally in `checker-identity.json` (source path, version, SHA-256), verified against actual source bytes and included in exact artifact/provenance validation; identity and artifact-set mutation controls reject drift. This adds ADR-005 F-09.1 evidence for this synthetic subcase only; global F-09, Mixxx A2 and overall F-04 remain NOT DEMONSTRATED.

The continuation controls in `xtask/tests/fixtures/f04-replay-logical/` freeze e3 at revision 2→3, stale e4 Conflict without revision/state change, identical replay NoChange, and altered e3 payload IdReuseError; the independent literal checker and mutation controls cover result/revision/payload changes, replay changes, history loss, visible/report/provenance drift and exact artifact digests. Checker identity is frozen externally in `checker-identity.json` (source path, version, SHA-256), checked against actual checker bytes, and included in exact artifact/provenance validation with identity/artifact-set mutation controls. This adds F-09.1 evidence for this logical continuation only; global F-09, Mixxx A2 and overall F-04 remain NOT DEMONSTRATED.

**Observed logical-only result (analysis-precedence subcase):** `xtask/tests/fixtures/f04-analysis-logical/` freezes the exact e1/e2/e3 state at revision 3, current-source `a1` from `m1-synthetic` version 1/sequence 1 proposing BPM 129, USER 130 precedence at revision 4, and explicit clear-user at revision 4 revealing analysis BPM 129 at revision 5. Archive retains source BPM 128, USER edit/clear history, analysis candidate and ordered operations; visible state contains only effective BPM 129 and cue `Opening 2`. The independent literal checker and mutation controls reject source/revision/analyzer/sequence/BPM drift, USER precedence errors, lost history, wrong clear result and report/provenance drift; canonical artifacts, empty report and exact artifact set are digest-checked. Synthetic logical-only evidence. Stale-source analysis rejection has its own logical-only result below. Mixxx A2 and overall F-04 remain NOT DEMONSTRATED.

**Observed logical-only result (cue-order/edit-targeting subcase):** `xtask/tests/fixtures/f04-cue-order-logical/` freezes the exact four-cue F-03 source, revision-0 reorder and revision-1 c0 label edit, revision-2 archive with originals/history, effective ordered visible cues, safe-v1 plan, empty report and exact provenance/artifact set. Independent literal checker `xtask/tests/f04_cue_order_logical.rs` has its path, version and SHA-256 frozen outside the checker in `checker-identity.json`; the test hashes the actual source bytes and includes the identity file in provenance and exact artifact-set checks. Identity and artifact-set mutation controls pass. This adds F-09.1 evidence for this synthetic subcase only; global F-09, Mixxx A2 and overall F-04 remain NOT DEMONSTRATED.
**Observed logical-only result (stale-source analysis rejection subcase):** `xtask/tests/f04_analysis_stale_logical.rs` freezes source revision `s1`/content revision 3 and candidate `a-stale` from `s0`/content revision 2, expected revision 3. The literal result is `Conflict` / `stale-source-or-content-revision` with `state_changed:false`; unchanged before/after state includes revision 5, ordered operations, archive, intermediate analysis state and visible state from the analysis-precedence fixture. The checker pins its plan, report and provenance, plus the exact combined artifact set. Mutation controls reject source/candidate revision and identity drift, revision/state changes, archive/intermediate/visible drift, plan/report/provenance/artifact-set changes and result mutation. This is synthetic logical-only evidence; F-04 overall and Rekordbox compatibility remain NOT DEMONSTRATED.
### F-05: Unknown kind and opaque bytes — synthetic only

This is `onelibrary-synthetic`, **not real OneLibrary bytes/row serialization**.
Exact canonical logical source object:

    {"colorTableIndex":3,"cueComment":"custom","inUsec":10000000,"isActiveLoop":true,"kind":7,"opaque_hex":"00deadbeefff","outUsec":20000000}

No edit. Archive preserves exact bytes; logical projection is `UNKNOWN(7)` with
raw comment/flags and unit-tagged in/out observations, **not a Loop**, even when
outUsec/isActiveLoop are present. These observations do not select an authoritative
vendor position. Synthetic preservation channel echoes exactly source.bin;
report `[]`. Real OneLibrary export without qualified binding is blocked with
`UNSUPPORTED_NATIVE_EXPORT`, r1/root, field `position`, null cause and reason
`unqualified-wire-format`. Unknown kind meaning and empirical real-format
passthrough stay NOT DEMONSTRATED (A3), even if the synthetic test later passes.

**Observed logical-only result:** `xtask/tests/fixtures/f05-logical/` freezes the exact canonical source bytes/object, raw opaque bytes/comment/flags, unit-tagged position observations and `UNKNOWN(7)` projection, archive, byte-identical synthetic echo, empty report, separate real-target blocked plan/report and provenance. `xtask/tests/f05_logical.rs` independently checks the frozen contracts and artifact digests; mutation controls reject inferred Loop semantics, altered source bytes, and a missing blocked-report entry. This is synthetic logical evidence only; A3 remains unqualified.

### F-06: BeatMap deletion and explicit declared dependency

Logical mapping of pinned Mixxx legacy BeatMap: b0 frame0, b1 frame120000,
b2 frame240000, each enabled=true/source=ANALYZER; sample rate48000. No per-beat
BPM is invented. Cue c0 absolute start3000000 usec/index0/label `Transition`/green,
end absent. e1 deletes **b1 by stable ID**, expected revision0, old full beat value.
Archive retains original three beats and deletion; visible beat frames `[0,240000]`,
cue unchanged at3000000; report `[]`. No inferred interpolation or spurious cue
invalidation. Actual Mixxx encoding/envelope remains A2.

F-06b resets and explicitly declares an **OxiDJ-only** relationship c0→b1,
offset500000 usec in its initial OxiDJ-only snapshot at revision0. This declaration
is not imported vendor behavior; e1 remains the first mutation.
After deleting b1, archive retains c0 and unresolved relationship. `safe-v1`
withholds c0; expected-visible contains no cue; one report `MISSING_ANCHOR`,
r-cues/c0, field `anchor`, cause e1, details `{anchor_element_id:"b1"}`.
Vendor-only re-import has no c0 or relationship, not a resurrected cue/report.
The archive checker verifies both. No alternate export-with-warning branch.

### F-07: XML tempo edit preserves absolute mark

XML-bound logical values: TEMPO t0 `(Inizio=0,BPM=120,Metro="4/4",Battito=1)`
and t1 `(10,128,"4/4",1)`; mark c0 Cue/Start5 seconds/Num0/Name `Intro`, End absent.
The full XML artifact must use documented numeric Type and exact required envelope
(A1); `Cue` is a logical tag here, not an asserted wire attribute value.
e1 changes t0.BPM120→125, expected revision0. Expected-visible TEMPO values are
125 and128 with all other fields unchanged; c0 stays at5000000 usec. Report `[]`.

This reverses the earlier unsupported claim that an altered beat coordinate makes
an absolute Start invalid. Fail if this bounded case moves/withholds c0 or invents
a dependency warning. No claim about Rekordbox UI recalculation. Loss behavior is
exercised by F-06b, F-08b and F-12, not fabricated tempo coupling.

**Observed F-09 post-edit independent evidence for one synthetic F-07 absolute POSITION_MARK Start edit:** Frozen `xtask/tests/fixtures/f07-position-mark-start-edit/source.xml` contains two marks on TrackID 700. The guarded adapter changes ordinal 0 Start from 5 to 6 under exact old-value guard; `xtask/tests/f07_position_mark_start_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the complete decoded XML tree to the frozen tree with only that Start changed. End, second mark raw fields, TEMPO tuples/order, TrackID/metadata, playlist tree and source bytes remain exact. Stale Start/wrong ordinal and changed Start, End, second mark, TEMPO and collection mutations are rejected. This is absolute-time semantics only, not beat lock or vendor compatibility; one synthetic case, global F-09 and qualification remain **NOT DEMONSTRATED**.

**Observed F-09 post-edit independent evidence for one synthetic F-07 absolute POSITION_MARK Name edit:** Frozen `xtask/tests/fixtures/f07-position-mark-start-edit/source.xml` contains two marks on TrackID 700. The guarded adapter changes ordinal 0 Name from `Delete me` to `Edited cue` under exact old-value guard; `xtask/tests/f07_position_mark_name_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the complete decoded tree with only target Name changed. Type/Start/Num/End, second mark including raw End=0, TEMPO tuples/order, track/playlist tree, and source bytes remain exact. Wrong ordinal, stale/empty Name, and Start/End/other-mark/tempo/collection mutations are rejected. One synthetic case only; no Rekordbox compatibility claim, global F-09 remains NOT DEMONSTRATED, and ADR-005 qualification remains unclaimed.

**Observed F-09 post-edit independent evidence for one synthetic F-07 POSITION_MARK deletion:** Frozen `xtask/tests/fixtures/f07-position-mark-delete/source.xml` contains two marks on TrackID 700. The existing adapter deletes ordinal 0 under exact raw Type/Start/Name/End guards; `xtask/tests/f07_position_mark_delete_oracle.rs` decodes with test-only `quick-xml`, not the production importer. It verifies only the selected mark is removed, retained mark including raw `End="0"`, TEMPO fields/order, TrackID/metadata, playlist tree and source bytes remain exact, and rejects restored/deleted-wrong mark plus retained-mark, tempo and collection mutations. This is one synthetic case only; global F-09 and vendor qualification remain **NOT DEMONSTRATED**.

**Observed F-09 post-edit independent evidence for one synthetic F-07 absolute POSITION_MARK End edit:** Reuses the frozen two-mark fixture `xtask/tests/fixtures/f07-position-mark-start-edit/source.xml` (TrackID 700). The guarded adapter edits ordinal 0 End from 10 to 12.5 under an exact old-value guard; `xtask/tests/f07_position_mark_end_edit_oracle.rs` independently decodes with test-only `quick-xml`, not the production importer, and compares the complete decoded XML tree with only the target End changed. Start, Type/Num/Name, second mark including raw End=0, TEMPO tuples/order, collection/playlist tree, and source bytes remain exact. Wrong ordinal, stale or absent End, unsafe range, and target/other mark, tempo, and collection mutations are rejected. This bounded absolute-time case is synthetic only; global F-09 and vendor qualification remain **NOT DEMONSTRATED**.

**Observed F-09 post-edit independent evidence for one synthetic A1 F-07 TEMPO BPM edit:** Frozen `xtask/tests/fixtures/rekordbox-a1/F-07/source.bin` follows the accepted A1 contract: TrackID 700 has ordered TEMPO tuples `(0,120,4/4,1)` and `(10,128,4/4,1)`, and one `POSITION_MARK` with Type=0, Start=5, Name=Intro, Num=0, and End absent. The guarded adapter changes tempo ordinal 0 Bpm 120→125 under the exact old-value guard; `xtask/tests/f07_tempo_bpm_edit_oracle.rs` independently decodes output with test-only `quick-xml` (not the production importer), compares the complete decoded tree against the frozen A1 source with only target Bpm changed, and confirms source bytes remain unchanged. The second tempo tuple/order, cue fields/End absence, track/playlist tree and other attributes remain exact. Stale BPM/wrong ordinal and target/other-tempo, cue, order, track and collection mutations are rejected. No UI recalculation or Rekordbox compatibility claim; global F-09 and ADR-005 qualification remain **NOT DEMONSTRATED**.

**Observed F-09 post-edit independent evidence for one synthetic A1 F-07 TEMPO Battito edit:** Frozen `xtask/tests/fixtures/rekordbox-a1/F-07/source.bin` is passed through the guarded TrackID 700 tempo ordinal 0 edit `Battito` 1→2 under the exact old-value guard. `xtask/tests/f07_tempo_battito_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the complete tree with only that attribute changed. The second tempo tuple/order, cue fields, track/playlist tree and source bytes remain exact; stale guards, wrong ordinal and target/tempo/cue/track/collection mutations are rejected. One synthetic case only; no Rekordbox compatibility claim, global F-09 remains NOT DEMONSTRATED, and ADR-005 qualification remains unclaimed.

**Observed F-09 post-edit independent evidence for one synthetic A1 F-07 TEMPO Metro edit:** Frozen `xtask/tests/fixtures/rekordbox-a1/F-07/source.bin` is passed through the existing guarded TrackID 700 tempo ordinal 0 edit `Metro` `4/4`→`3/4` under the exact old-value guard. `xtask/tests/f07_tempo_metro_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the whole tree with only the target Metro changed. It asserts the source fixture bytes remain unchanged; stale guards and wrong ordinal refuse or fail the oracle, and target/other-tempo, cue, track, collection and tempo-order mutations are rejected. No meter/beat coupling, interpretation, normalization, or Rekordbox compatibility is claimed. One synthetic case only; global F-09 remains NOT DEMONSTRATED and ADR-005 qualification remains unclaimed.

**Observed F-09 post-edit independent evidence for one synthetic A1 F-07 TEMPO Inizio edit:** Frozen `xtask/tests/fixtures/rekordbox-a1/F-07/source.bin` passes through the existing guarded adapter for TrackID 700 tempo ordinal 1 `Inizio` `10`→`11` under the exact old-value guard. `xtask/tests/f07_tempo_inizio_edit_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer, and compares the complete tree with only that attribute changed. It verifies frozen source bytes remain unchanged and rejects stale/wrong ordinal requests plus target, first-tempo, tempo-field, cue, track, collection, and tempo-order mutations. This is raw coordinate editing only; no beat-grid interpolation/coupling or Rekordbox compatibility claim. One synthetic case only; global F-09 remains **NOT DEMONSTRATED** and ADR-005 qualification remains unclaimed.

**Observed F-09 post-edit independent evidence for one synthetic A1 F-02 Track Name edit:** Frozen accepted source `xtask/tests/fixtures/rekordbox-a1/F-02/source.bin` is passed through the existing guarded adapter for TrackID 100 Name `A`→`New < title` under the exact old-value guard. `xtask/tests/f02_track_name_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the whole decoded tree with only the selected Name changed. It confirms frozen source bytes remain unchanged and rejects stale/wrong TrackID guards plus target/other-track, identity, location, playlist-entry and membership mutations. This is one bounded synthetic A1 case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility or ADR-005 qualification claim.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Artist edit:** Frozen `xtask/tests/fixtures/f02-track-artist-edit/source.xml` has TrackID 100 Artist `Old & exact` and TrackID 200 with Artist absent. The guarded adapter changes only TrackID 100 Artist to `New Artist` under the exact old-value guard; `xtask/tests/f02_track_artist_edit_oracle.rs` independently decodes with test-only `quick-xml` (not the production importer) and compares the complete decoded tree against the frozen source with only the selected Artist changed. It preserves source bytes and the absent Artist state; stale/wrong-target guards and target, identity, location, playlist-entry, membership, and absent-state mutations are rejected. This is one bounded synthetic F-02 case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Comments edit:** Frozen source `xtask/tests/fixtures/f02-track-comments-edit/source.xml` has TrackID 100 Comments `old & note` and TrackID 200 with Comments absent. The existing exact-old guarded adapter changes only TrackID 100 Comments to `new < note`; `xtask/tests/f02_track_comments_edit_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer, and compares the complete decoded tree with only the target Comments changed. It checks source-byte immutability and preservation of absent Comments on TrackID 200; stale/wrong-target guards and target, identity, location, playlist-entry, membership, and absent-state mutations are rejected. This is one bounded synthetic case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Album edit:** Frozen source `xtask/tests/fixtures/f02-track-album-edit/source.xml` has TrackID 100 Album `old & record` and TrackID 200 with Album absent. The existing exact-old guarded adapter changes only TrackID 100 Album to `new < record`; `xtask/tests/f02_track_album_edit_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer, and compares the complete decoded tree with only target Album changed. It verifies source-byte immutability and absent Album on TrackID 200; stale/wrong-target guards and target, identity, location, playlist-entry, membership, and absent-state mutations are rejected. One bounded synthetic case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Genre edit:** Frozen source `xtask/tests/fixtures/f02-track-genre-edit/source.xml` has TrackID 100 Genre `old & style` and TrackID 200 with Genre absent. The existing exact-old guarded adapter changes only TrackID 100 Genre to `new < style`; `xtask/tests/f02_track_genre_edit_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer, and compares the complete decoded tree with only target Genre changed. It verifies source-byte immutability; stale/wrong-target guards and target, identity, location, playlist-entry, membership, and absent-state mutations are rejected. One bounded synthetic case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility claim or ADR-005 qualification.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Label edit:** Frozen source `xtask/tests/fixtures/f02-track-label-edit/source.xml` has TrackID 100 Label `old & imprint` and TrackID 200 with Label absent. The existing exact-old guarded adapter changes only TrackID 100 Label to `new < imprint`; `xtask/tests/f02_track_label_edit_oracle.rs` independently decodes output with test-only `quick-xml`, not the production importer, and compares the complete decoded tree with only target Label changed. It verifies source-byte immutability and absent Label on TrackID 200; stale/wrong-target guards and target, identity, location, playlist-entry, membership, and absent-state mutations are rejected. One bounded synthetic case only; global F-09 remains **NOT DEMONSTRATED**, with no Rekordbox compatibility claim or ADR-005 qualification.

### F-08: Alternate encodings and unsupported position edit

Synthetic object as F-05, replacing kind with0 and opaque_hex with `deadbeef`,
isActiveLoop with false, and adding exactly these integer fields:
in150FramePerSec=1500000, out150FramePerSec=3000000,
inMpegFrameNumber=750000, outMpegFrameNumber=1500000,
inDecodingStartFramePosition=0, outDecodingStartFramePosition=0,
inNumberOfSampleInBlock=480000, outNumberOfSampleInBlock=960000,
inTimeCode=0, outTimeCode=0. Values are deliberately uninterpreted opaque
observations, **not validated conversions**. Kind0 is retained raw; no range
semantics inferred from outUsec. No edit: byte-identical synthetic echo, report `[]`.
The inUsec/outUsec projection is a labeled observation, not vendor authority.

**Observed logical-only result:** `xtask/tests/fixtures/f08-logical/` freezes the
canonical F-05-derived source object, all ten alternate integer observations,
archive raw-byte witness, visible object, no-edit plan/report and provenance.
`xtask/tests/f08_logical.rs` independently checks the frozen values and hashes;
provenance claims, the complete artifact set, and artifact hashes; mutation
controls reject changed opaque bytes, invented conversion/kind semantics, and
invented provenance claims. This is synthetic logical evidence only; A3 remains
unqualified.

F-08b: e1 sets the observed inUsec10000000→11000000 at revision0. Revision1
retains the new observation as USER intent, marks the position unsafe for export,
and preserves all originals; encodings are not selectively rewritten. Synthetic
export plan withholds whole r1 cue, visible cue set empty; report exactly
`UNSUPPORTED_POSITION_EDIT`, r1/root, field `position`, cause e1 and details
`{old_usec:10000000,new_usec:11000000}`. Frozen logical-only artifacts and an
independent archive/visible/plan/report/provenance checker are in
`xtask/tests/fixtures/f08b-logical/` and `xtask/tests/f08b_logical.rs`; mutation
controls reject changed reports, accidental visibility, and original-byte/value
mutation. This is synthetic logical evidence only. Real OneLibrary export remains
blocked as F-05 until A3; byte retention alone establishes neither semantic
compatibility nor synchronization.

**Observed F-09 post-edit independent evidence for one synthetic F-02 guarded Track Tonality edit:** Frozen source `xtask/tests/fixtures/f02-track-tonality-edit/source.xml` contains Tonality `old & key` on TrackID 100 and no Tonality on TrackID 200. The existing exact-old guarded adapter changes only TrackID 100 Tonality to `new < key`; `xtask/tests/f02_track_tonality_edit_oracle.rs` independently decodes the output with test-only `quick-xml` (not the production importer) and compares the complete decoded tree with only that target attribute changed. Frozen source bytes and absent state remain exact; stale/wrong-target requests and target Tonality, identity, location, playlist-entry, membership, and absent-state mutations are rejected. This is one bounded synthetic case only; key notation/detection/normalization semantics, Rekordbox compatibility, global F-09, and ADR-005 qualification remain **NOT DEMONSTRATED**.

### F-09: Independent, post-edit and loss-aware oracle

Applies to **all F-01–F-12 subcases**, including F-09's own checker controls:

1. Before running production import/export, independently author/review the
   expected artifacts above from the input recipes and approved operations.
   They must never be generated by the production parser, projection, exporter
   or its ID reconciliation/helper code. Record checker source/version/digest.
2. Compare imported and post-operation archive snapshots against those goldens:
   immutable bytes, exact effective values, IDs/revisions, history, conflicts,
   order and relationships. This is a storage/archival inspection, not re-import.
3. Compare the pre-write report and export plan separately to expected artifacts.
   A blocked plan must write no target. Otherwise check actual output directly
   with a separate decoder using independent extraction rules: e.g. general XML
   tree reading plus explicit attributes/ordered children for A1, not OxiDJ's
   importer. For synthetic JSON compare canonical bytes plus direct JSON fields.
   Real Mixxx/OneLibrary checker bindings must be qualified under A2/A3.
4. Fresh OxiDJ vendor-only re-import is a useful **additional** integration check,
   not checker independence. Compare it to expected-visible, not original import
   or expected-archive. Normalize only local IDs with the declared bijection and
   explicitly transportable fields; absence of history is expected.
5. Checker self-tests must reject independently tampered disposable outputs:
   missing index0, ms/seconds unit swap, reordered House memberships, original
   `Intro` after F-04, omitted F-10 tempo point, restored withheld F-12 c38, a
   changed opaque byte, and an absent required report entry. A production parser
   shared with its checker cannot satisfy this requirement even in a fresh process.

No checker or self-test has run on this card. If independent expected states or
bindings are missing, result is NOT DEMONSTRATED. Do not bootstrap a golden by
accepting matching output from two paths that share the same bug.

### F-10: Variable tempo without invented interpolation

XML-bound ordered TEMPO tuples `(Inizio seconds,BPM,Metro,Battito)`:
`(0,120,"4/4",1)`, `(8,128,"4/4",1)`, `(16,132,"4/4",1)`.
No operation. Visible and archived tuples identical; neutral projection retains
three ordered points, units and raw meter/beat fields; report `[]`. No average BPM,
bar numbering, phase or interpolation is inferred. Numeric/type/envelope binding
A1 and application acceptance remain NOT DEMONSTRATED.

### F-11: Absent end differs from range and zero

Logical Mixxx DTO c0 HotCue/index0/start10000000/label `Intro`/red with absent end;
c1 Loop/start15000000/end25000000/label `Loop 1`, index/color absent. Flags0.
No operation: preserve exact optionality and range, report `[]`. Reset c2 point
start0 with absent end (valid zero). Archive preserves all distinctions; zero is
not treated as absent. Unresolved c3 with absent start/end is **deferred**: its
logical export-plan/report outcome is not specified, it is not in the executable
fixture set, and it is not represented as a valid positioned cue. Real Mixxx DTO
transport and absent-position policy remain A2. Loop endpoint inclusivity unclaimed.

The executable logical-only fixture is `xtask/tests/fixtures/f11-logical/` with
frozen source/archive/visible/plan/report/provenance bytes. Its separate checker
and disposable mutation controls are in `xtask/tests/f11_logical.rs`; they prove
only the specified logical contract, not a Mixxx library transport or application.


### F-12: Deterministic hotcue overflow and transport loss

Logical DTO contains exactly 40 HotCue elements c0..c39, ordered by integer i.
For i=0..39: index=i, start_usec=(i+1)*1000000, label=`H` plus two-digit decimal i
(`H00`..`H39`), end/color absent, flags0. These are explicit logical stress inputs,
not a claim that a real Mixxx library accepts indices38/39. No edits.

Archive retains all40; projection marks indices38/39 out of the fixture target's
0..37 capacity. `safe-v1` exports exactly c0..c37 without renumbering. Report has
two entries `HOTCUE_CAPACITY`, r-cues/c38 and r-cues/c39, field `hotcue_index`,
null cause, WITHHOLD, details `{index:38,maximum_index:37}` and
`{index:39,maximum_index:37}`. No export-all alternative.
Fresh vendor-only re-import expects **38**, not40, cues and no recovered overflow
history/report. Independent archive/report checks prove retention and disclosure.
Real Mixxx target transport and application handling remain NOT DEMONSTRATED.

**Observed logical-only result:** `xtask/tests/fixtures/f12-logical/` freezes the 40-cue source, archive, 38-cue visible output, exact two-entry capacity report, safe-v1 plan, provenance, and vendor-only re-import witness. `xtask/tests/f12_logical.rs` independently checks the contract and artifact digests; mutation controls reject archive truncation, visible renumbering, restored overflow cues, and missing/incorrect loss entries. Logical-only evidence; real Mixxx transport/application handling and A2 remain NOT DEMONSTRATED.

## 4. B1/B2 reference oracle (test-only, not a vendor fixture)

Use ADR-005's exact `RG-ASSET-STUB` bindings and protocol §1 payload/manifest bytes.
B1 expected live references: ref-1→stub-a, ref-2→stub-b. After the one committed
reference transition B2: ref-1→stub-b, ref-2→stub-c. Track endpoints stay fixed.
The archive oracle checks the transition history; bundle-set equality checks only
live references. No historical a reference forces a into B2's manifest. Other
tracks retain no sentinel references. Concrete ID/physical binding remains C6.

Independently frozen snapshots for each boundary must include all persisted logical
records, live references and operation history, plus protocol's independent witness
and digests. Restored B1 must match B1, not latest B2/live state, and vice versa.
No runtime regeneration, live-assets fallback or other-bundle dependency is allowed.
Missing file, changed payload byte and manifest tampering must fail on disposable
copies under the separate authorized recovery gate. This card performs none of
those controls or restores and defines no product media/retention policy.

## 5. Unresolved artifact and qualification gates

- **A1 XML binding — synthetic fixture set frozen:**
  [`xtask/tests/fixtures/rekordbox-a1/`](../../xtask/tests/fixtures/rekordbox-a1/)
  contains complete synthetic XML byte documents (`source.bin`) for F-02/F-07/F-10,
  independent semantic goldens, provenance/digests, and an independent Rust XML
  checker in `xtask/tests/rekordbox_xml_a1.rs`. The pinned Rekordbox XML format
  list (URL and digest in each `provenance.json`) specifies version 1.0.0,
  playlist `Type` 0/1, `KeyType` 0 for TrackID, and `POSITION_MARK` Cue=0; optional
  mark `End` is absent in the cue fixture. Checker path/version/SHA-256 is frozen
  externally in `checker-identity.json`; the test checks live checker bytes,
  provenance identity, exact fixture sets and artifact digests. Tests reject
  mutations, omissions, dangling endpoints and reordered membership/tempo points.
  This is synthetic fixture evidence only: no Rekordbox application acceptance
  or product adapter is claimed.
- **A2 Mixxx binding:** select the exact adapter transport/envelope and pin how
  BeatMap/BeatGrid proto, sample-rate context and CueInfo DTO optionality/colors
  are carried. A CueInfo C++ DTO is not a library-file format. Bind raw bytes,
  real target equality and the absent-position policy; no GPL implementation
  copying or new dependency is authorized. Logical tests do not prove DB flags
  persist or an application accepts overflow.
- **A3 OneLibrary binding:** obtain qualified real source artifacts/version and
  lawful exact-record/container extraction/preservation/checker path. Synthetic
  objects in F-05/F-08 have no OneLibrary container claim. Unknown kind and position
  synchronization stay NOT DEMONSTRATED; do not manufacture empirical rows.
- **C1–C6:** C1 remains OPEN under the lead-ratified ceiling interpretation: unchanged per-cell `B` values are assigned payload-budget ceilings, while `A` is independently justified realized eligible logical byte-token count with `0 <= A <= B` once counting rules exist. Preserve all nine initial and scan-offer cell counts and the fixed seed/order; existing totals are sums of assigned `B`, not realized `A`, measurements, or storage. No lower bound or occupancy threshold is approved. C1 cannot close until eligible-field/type/encoding/ownership/exclusion rules, an independently justified inventory/allocation, measurable meaningful-coverage acceptance, and C2/C3 compatibility are specified and independently reviewed. The ceiling alone cannot qualify tiny/empty payloads. C2 endpoint/ownership inclusion, sharing/order, required-case distribution/cohorts, timed manual/analysis schedules/payloads, and concrete sentinel ID/reference binding remain open as enumerated in ADR-005. No full qualification harness can be implemented honestly without resolving them.
- Freeze reviewed exact artifact bytes/expected states and implement the independent
  checker/self-tests before results count. No result table below is an execution.

| Case | Required specification behavior | Observed result |
|---|---|---|
| F-01 | Explicit reconciliation/conflicts, archive vs current location | Same-content relocation success PASS; same-namespace/source-ID content conflict, unconfirmed cross-namespace identity, same-path/different-content conflict, and metadata-source disagreement PASS — logical-only fixtures/checkers; other conflict cases and A2 NOT DEMONSTRATED |
| F-02 | XML child order, shared endpoints | PASS — synthetic A1 checker/tests; no application acceptance |
| F-03 | Constant grid/cue/range preservation | PASS — logical-only fixture/checker and mutation controls; A2 NOT DEMONSTRATED |
| F-04 | Revisioned edits, stale conflicts, analysis precedence | Initial accepted-edit composition, stale/replay/ID-reuse controls, cue reorder/edit-targeting, analysis precedence and stale-source analysis rejection PASS — logical-only fixtures/checkers/mutation controls; Mixxx A2 NOT DEMONSTRATED; overall F-04 NOT DEMONSTRATED |
| F-05 | Synthetic unknown-kind bytes; real vendor gap | PASS — synthetic logical fixture/checker and mutation controls; A3 NOT DEMONSTRATED |
| F-06a | Absolute cue independence, logical-only | PASS — synthetic logical fixture/checker and mutation controls; A2 NOT DEMONSTRATED |
| F-06b | Declared anchor loss, logical-only | PASS — synthetic OxiDJ-only declaration/deletion, exact safe-v1 withholding report, vendor-only re-import and mutation controls; A2 NOT DEMONSTRATED |
| F-06 grouped | Absolute independence / declared anchor loss | PASS — F-06a and F-06b logical-only evidence only; A2 NOT QUALIFIED |
| F-07 | XML tempo edit without false absolute-cue dependency | PASS — synthetic A1 checker/tests; no UI claim |
| F-08 / F-08b | Synthetic encoding retention / unsafe edit withholding | F-08 PASS — logical-only no-edit fixture/checker; F-08b PASS — logical-only position-edit archive/withholding fixture/checker and mutation controls; A3 NOT DEMONSTRATED |
| F-09 | Independent post-edit/loss-aware checker and controls | NOT DEMONSTRATED |
| F-10 | Ordered variable tempo/meter preservation | PASS — synthetic A1 checker/tests |
| F-11 | Point/range/optional-end distinctions | PASS — logical-only fixture/checker and mutation controls; A2 NOT DEMONSTRATED |
| F-12 | 40 archived, 38 visible, exact two-entry loss report | PASS — synthetic logical-only fixture/checker and mutation controls; A2 NOT DEMONSTRATED |
| B1/B2 | Atomic test-only reference transition and boundary snapshots | NOT DEMONSTRATED |

PASS means observed equality on every required surface and checker control, with
real-format subclaims bounded by their qualified artifacts. FAIL is a demonstrated
mismatch; absent/unqualified evidence is NOT DEMONSTRATED. All decision-bearing
fixtures must actually PASS before ADR-005 leaves Proposed under its existing rule.
Synthetic-only PASS never satisfies an unresolved real-format claim. Any narrowing
of the transition scope needs explicit reviewed agreement, not silent removal of
A1–A3. Neither this draft nor its later specification review is fixture execution.
