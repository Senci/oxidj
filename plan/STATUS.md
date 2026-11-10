# Project Status

Condensed 2026-11-02. Full prior text (per-increment evidence, M0-09 verification, decisions, risks, F-09 oracle notes): [docs/history/status-2026-11-02.md](../docs/history/status-2026-11-02.md). Milestone contract: [plan/MVP.md](MVP.md). Live work: Kanban; this file records stages and gates only.

## M8 deck and mixer layout v2 — implemented, reviewed and installed (2026-11-10)

The owner approved the direction in a static web mockup
([v19 spec](../docs/milestones/v19-deck-layout-v2/README.md)): Layout B default,
aligned deck content band with enlarged cover, Lucide icons, outside-knob mixer
with full-height faders and meters, crossfader under a separator line, compact
mixer, Settings preferences. Implementation was staged as three chained cards; deck panel (ec026cc), mixer (e99daa1, 6bd5f67) and
Layout B (e073714: detail + overviews on top, deck A | mixer | deck B below,
Deck view preferences, library keeps 120 px) each passed independent review with ci-full exit 0
and are installed. Playback-driven overview fitting is reviewed and approved (install pending). Layouts A, C and D are
selectable (Settings → Deck view → Deck arrangement, B default; commit e8fc45b, review and install pending; captures `app-layout-{a,c,d}.png`). The mockup is synthetic and not
evidence of anything shipped; native display, accessibility and performance
qualification stay NOT DEMONSTRATED.

## M8 deck album artwork — implementation pending independent review

`t_383435d2` adds read-only FLAC/MP3/AIFF embedded artwork extraction,
cover/folder-file fallback, a decoded SQLite cache and shared in-memory LRU.
Both decks load asynchronously, start with a deck-tinted vinyl placeholder and
swap in centered aspect-preserving images. Settings controls visibility and a
persisted 1–1024 MiB budget (default 100 MiB). Budget changes drop stale jobs and
textures; display images are capped at 512 px with smaller budget-based caps,
and decoding is serialized with pixel allocation limits. The budget accounts
for cached pixels and deck texture allowances, not total process RSS or metadata
parser/graphics-driver overhead. Synthetic regressions and final gates are
recorded on the card; independent acceptance and installation remain pending.
Personal catalogs/media were not used for this work.

## M8 headphone cue routing — implementation pending independent review

`t_dc62491a` completes the separate post-filter/pre-fader headphone bus and
native CUE switches, MIX and volume controls. The tap follows the colour
filter, and the per-deck cue gain, CUE volume and master volume ramp across
each chunk so CUE switches and knob moves fade instead of clicking. Main output
remains independent; multichannel outputs 1–2 carry main and 3–4 carry
headphones, with an explicit startup endpoint selector. Synthetic
production-renderer and actual-egui tests cover routing, gain, the post-filter
tap, click-free gain changes, selected state, keyboard/AccessKit/wheel/reset
and focus. The real CPAL → temporary PipeWire four-channel sink → monitor
check passed with independent main/cue frequencies, 0 reported xruns/errors
and no default sink change. [Runbook and limits](../docs/headphone-cue.md).
Physical interfaces, ASIO/macOS and latency remain NOT DEMONSTRATED. Review,
install and the requested publication remain pending; preceding artwork WIP is
not accepted by this card.

## Requested Settings theme presets — implementation pending independent review

`t_f12bcc02` adds Catppuccin Mocha, Dracula, Ayu Dark, Dainty Dark,
GitHub Dark Classic, Atom One Dark, Houston, Night Owl and Matcha alongside
Tokyo Night, Tokyo Day and Custom. One picker reports the actual selected
preset; existing storage keys, custom colors, settings and shortcuts are kept.
Upstream sources and MIT notices are embedded in Settings. Synthetic
actual-egui pointer selection, AccessKit selected-state, storage restoration,
semantic-token and contrast regressions cover all presets. Independent review
and installation remain required; native-display and full accessibility
qualification are not claimed. The standing follow-on objective remains Lucide
SVG icons, club-standard mixer/faders, distinct trim/filter/master knobs,
per-deck headphone cue routing with cue mix/volume, and deck artwork; this card
does not complete that objective.

## M6 bounded step-tempo inspection — reviewed and installed (2026-11-08)

`inspect-step-tempo --file PATH` provides an explicit-file, inspection-only
`ComputedStepTempoV1` constant/variable/refusal result under the synthetic
100–160 BPM, 8–180 s, at-most-two-change contract in the
[ADR-004 B2 addendum](../docs/adr/004-b2-step-tempo.md). Implementation landed
in `7cb9bfe` and `fc117c5`; Sol audit/fix-all corrected unsupported-rate decoder
admission in `9065518`. `oxidj-codex` independently approved the full diff
`7e5fad7..9065518`; `cargo xtask ci-full` exited 0 with `ci=ok`,
`test=ok`, `ui-test=ok`, `deny=ok`. After approval,
`bash scripts/install-local.sh` exited 0 at
`9065518`; installed CLI and UI binaries byte-match the release artifacts, and
CLI help exposes the command. Normal `catalog analyze-tempo`, format-v1 computed
storage, imported precedence, computed-grid export and SYNC remain unchanged.
No private data, push, persistence/export/SYNC admission, real-music accuracy,
or vendor/hardware claim. M6 vendor accuracy, native display, accessibility and
GPU/presented FPS remain NOT DEMONSTRATED.

## M8 full-width playlist interaction — implementation pending independent review

`t_811767eb` extends the reviewed playlist browser with the full central metadata
table (ordered duplicates, search, selection/Details), row drag/drop from both
collection and playlists to either deck, and an opt-in persisted Simple DJ keys
mode. Context menus and deck Load buttons retain exact-location resolution and
missing-audio refusal. Synthetic actual-egui regressions cover the new paths;
final gate output and commit are recorded in the card handoff. Independent
review and installation are not yet claimed. Rotary EQ/filter is a separate
queued follow-on (`t_b5986ae6`); no mixer/DSP change is included here.

## M8 imported playlist browsing/loading — reviewed, audited, installed (2026-11-07)

- `t_8b142e7c` audited `e6b9ad1` and `8eeeee7`; fixes committed as `8068744`: long playlist navigation scrolls, same-named folders have unique structural IDs, playlist selection uses structural identity rather than membership, and imported snapshot read errors stay visible instead of appearing empty.
- Production-egui regressions cover ordered duplicates, exact imported-location loading onto A/B (including creation of an unscanned audio reference), no filename fallback, refusal preserving deck state, independent same-named folders, scrolling to member 80, stopped playback, read failures, and unchanged imported snapshot.
- Sol 6.1 final approval (`t_4ca94599`) approved `8068744`. Focused production-egui tests exit 0 (4 passed); `cargo xtask ci` exit 0: `layers=ok fmt=ok clippy=ok check=ok [all pass]`; `cargo xtask ci-full` exit 0: `ci=ok test=ok ui-test=ok deny=ok [all pass]` (UI 238 passed, 2 ignored); `git diff --check` exit 0.
- Installed with `bash scripts/install-local.sh` on local `master`; installed `oxidj-cli` and `oxidj-library-ui` are byte-identical to the release artifacts; `oxidj-cli --version` reports `0.1.0`. No push, private-library/media/USB access, or vendor/hardware qualification performed.

## TL;DR (2026-11-05)

- M8 decks and playback landed on local `master` and pushed (`771f631`..`d92932f`, W12 `2108b79`..`d1e6c21`, all Sol-reviewed): Settings (Ctrl+,), merged two-deck detail waveform (A warm above / B cool below one seam), separate overviews, collapsible sections, CPAL engine (`oxidj-audio`), hot cues/loops, tempo + key lock + SYNC, optional mixer, 3-band deck waveforms with overview auto-gain. Owner listening check: "it works" (2026-11-05, no measurements). NOT DEMONSTRATED: club gear, CDJ/XDJ playback, latency guarantees (ADR-008: one 600 s PipeWire run, 1 xrun; key-lock fader transient < 5 ms for one beat).
- Real library seeded locally (1,323 audio tracks, 1,331 Rekordbox tracks; computed BPM within 2% of Rekordbox on 97% of tracks, counts only); `scripts/install-local.sh` installs the CLI, UI, launcher and desktop entry after each accepted change (the owner's Omarchy menu entry is a one-time local config); `export-library-xml --computed-grids` writes computed grids only for tracks without a Rekordbox grid (Rekordbox check of those 5 tracks pending).
- Open items at sign-off (2026-11-06, owner paused work): (1) owner Rekordbox check of the 5 computed-grid tracks (export in the VM shared folder); (2) CDJ/XDJ hardware playback and club-gear latency NOT DEMONSTRATED; (3) ADR-005/A3 native OneLibrary probe parked in triage (needs owner approval); (4) local backup branch `backup/canonical-dirty-20261024` kept (3 local-only commits) pending owner decision; (5) Codex Sol quota: when the session window is under 4% reviews fall back to Opus 5.5 (done once for the track-listing card). Next candidates at sign-off: import music from the UI with background analysis (subsequently completed and reviewed in M6; `3b7423c`..`e8dd936`); loading the next track from a crate/playlist onto a deck; polish from owner feedback; tempo-change maps for variable-tempo grids.
- A3 follow-up (2026-11-08): owner authorized disposable external-reader installs for local validation. The pinned `v1vendi/onelibrary` candidates and `pyrekordbox` 0.5.0 did not yield schema/row results in the verified read-only sandbox; this is not incompatibility evidence. A separate pinned `chrisle/rbxport` 1.10.0 read-only path applies `cipher=sqlcipher`, then `legacy=4`, then key; its aggregate-only check exited 3 with a generic error and no schema/row result, despite a verified read-only namespace and byte-identical staged copies. OxiDJ's native schema exactly matches the community schema file, but this is structural alignment only. Prior OxiDJ counts remain local feasibility evidence; independent agreement, source/version provenance, and ADR-005/A3 qualification remain NOT DEMONSTRATED. Detailed attempts and sources: Kanban `t_4105b6eb`.
- Earlier history below (2026-11-03).

## TL;DR (2026-11-03)

- Sweep S1–S5b landed on local `master` (not pushed): defect fixes with failing-first regressions (S2), one table-driven `prepare_track_attribute_edit` replacing 26 per-field copies (S3), UI split into modules with one generic field-edit workflow and `cargo xtask ui-test` in `ci-full`/CI (S4), one table-driven F-02 TRACK attribute oracle plus a shared XML tree decoder with F-09 provenance still at 50 bindings / 57 artifacts (S5), and one `oxidj_core::publish::publish_no_clobber` for exports without hard links (S5b).
- Waveform view landed (W1–W4, Sol-approved): `WaveformPeaks` in core, new `oxidj-analysis` streaming 3-band builder, catalog persistence (`catalog analyze-waveform`), read-only ANLZ PWV3/PWV5 vendor waveforms kept separate, and one egui `waveform_view` (mono / 3-band / RGB, live per-band EQ gain). CPU render measured 0.04–0.09 ms/frame (release, 10 min @ 1440 px; excludes GPU/presented FPS). Real-music spot check: 17 local files analysed, 0 failed (~1.1 s), one track eyeballed in RGB; screenshot kept private.
- Provisional UI polish W5–W9 (local `master`): smooth anti-aliased vector (triangle-strip) waveform envelopes (W5b), bar/beat lines and a "Bar N" counter from the *imported* library beat grid (W6), a design audit with contrast/24 px target/focus-ring fixes (W7, `docs/milestones/archive/ui-design-audit/`), a dense shell — full-width track table, toggleable Nav/Details/Decks panels (Ctrl+B/I/D), one-row toolbar (W8, `docs/milestones/archive/ui-minimal/`) — and waveform-panel polish (wrapping captions, aligned "Low ━○ 1.00" slider rows, 96–140 px detail strip, truncated deck names; W9, `docs/milestones/archive/ui-minimal/after-w9/`). W11 visual polish (segmented panel toggles, right-aligned numeric columns, search with clear, compact details/SHA, filled Position slider, hairline separators; `docs/milestones/v12-ui-polish/`). Waveform CPU frame (run+tessellate, release, 1440 px) re-measured 0.07–0.63 ms. Still the provisional prototype; the gates below are unchanged.
- A1 AIFF decode (`t_7177ff1d`, local `master`): `pcm::decode_mono` and `catalog import`/`analyze-waveform` accept `.aiff/.aif/.aifc` (and FORM/AIFF|AIFC magic for decode) through a hand-rolled, fail-closed parser (no new dependency): 8/16/24/32-bit big-endian PCM, AIFF-C `NONE`/`sowt` (little-endian) and `fl32`/`fl64`; other AIFF-C codecs, non-8/16/24/32 sizes and fractional or >768 kHz rates are Unsupported. Still unsupported: M4A/AAC, ALAC, Ogg/Opus. Real-data: owner stick NOT mounted (library-wide count NOT RUN); 12 local AIFF files (all `AIFF`/`NONE`/stereo/16-bit) imported and analysed 12/12.
- Synthetic XML editing is broad enough. **Stop per-field/per-operation increments.**
- Next direction (idea.md MVP): **real-library import → manage → export proof** — import a real DJ library, manage it safely in OxiDJ, export it again, and keep it usable in compatible DJ software without unexpected metadata loss. Private data stays local; evidence is aggregate-only.

## Stages

| Stage | Status | Note |
|---|---|---|
| M0 Bootstrap/research/tooling | Complete | Foundation and repository workflow established; ADRs 001–004, 008, 009 Accepted. |
| M1 Library core | Complete for revised local contract | Local catalog scope is capped at 10,000 tracks; historical scale and qualification gates remain separate. |
| M2 Filesystem import | Complete for revised local contract | Reference-only local import acceptance is complete (WAV/MP3/FLAC declared matrix); AIFF/AIFF-C added by A1. |
| M3 Usable library UI | Complete for revised local contract | Persistent local-library UI acceptance is complete; production GUI/accessibility and 500k/60fps gates remain unpassed. |
| M4 OneLibrary/Rekordbox interoperability | In progress | Synthetic XML editing/export and read-only PDB/OneLibrary previews exist. Native/vendor compatibility and ADR-005 qualification remain NOT DEMONSTRATED. |
| M5 DJ metadata | Complete for listed local UI scope | Read-only imported cue/loop Details (`91d8417`), verbatim imported-key Details (`8be1dac`), and imported beat-grid display in reviewed W6 waveform UI. Focused actual-egui regressions cover cue labels, raw key, absent vs present-empty, selection changes, and unchanged catalog. Vendor/hardware qualification remains separate and NOT DEMONSTRATED. |
| M6 Analysis | In progress | First slice: waveform peaks + view (`7e8968d`, `9de5d19`, `22a5d85`/`cc1ebfd`, `8713512`/`107f656`); W5/W5b smooth 3-band (`5b115af`, `7052e2f`); W6 bar-aware detail window from an *imported* (vendor-derived) beat grid: 4 bars from a downbeat, beat/bar lines, "Bar N" counter, 8 s fallback without a grid (synthetic captures `docs/milestones/archive/waveform/07–09`; real-library match count 0). Zoomed deck detail peaks increased to 1,200/s (`078816a`); synthetic scrolling one-bar transient regression and storage/timing comparison passed; legacy low-resolution peaks can be upgraded with `catalog analyze-waveform --db DB --refresh [--track-id ID]` (`413a548`, `0cb829d`, `7136836`); omitting `--track-id` refreshes the full catalog, and refresh rereads audio and increases derived storage. Synthetic targeted/bulk upgrade and failure-preservation tests passed; UI callers preserve non-refresh behavior. Computed BPM + beat grid (B2): taktus 0.1.0 admitted with streaming `TempoBuilder` (`82dcee1`, `2390310`), per-track grids persisted (schema v7) with `catalog analyze-tempo` (`da0220b`), and the details panel shows the computed grid (bar lines every 4th beat from the first detected beat, own caption) plus a `BPM (computed)` fact when no imported grid exists; an imported grid always wins (`9a9b96c`, `docs/milestones/v13-computed-beat-grid/`). Maturity: computed suggestion; B1 evaluation: BPM within 2% of Rekordbox on 91% of 90 real tracks, F70 0.76; not Rekordbox parity, downbeat unknown, no tempo changes; exported to XML only opt-in (`--computed-grids`, `d295fc4`) for tracks without a Rekordbox grid. PDB-imported-library analysis bridge and CLI storage-failure regression reviewed and installed (`764f888`..`cd6a8fe`): synthetic PDB import → `catalog analyze-tempo` computes BPM/key, persists the expected beat grid and preserves vendor metadata; injected storage failure exits nonzero, with recovery/unavailable-audio controls. Key suggestion is displayed, not persisted. Imported-reference UI background waveform analysis reviewed/audited and installed (`3b7423c`, `c4b4ad9`, `16f3765`, `e8dd936`): queues only `Imported` rows, covers held-in-flight production frames, synthetic unreadable/conflict/unchanged/unsupported exclusion, truthful failure counts, waveform reload, and active-job receiver retention on repeated imports. End-to-end accuracy against vendor libraries remains NOT DEMONSTRATED. |
| M7 MVP interoperability proof | In progress | Slice 1 (`t_712e2b28`, `40b0631`): `catalog import-library` (PDB preview → new catalog, OneLibrary read-only aggregate cross-check) and `catalog export-library-xml` (no-edit, verified, new-file-only); private staged-copy run aggregate-only: 1,330 tracks / 20 playlists / 901 memberships, order preserved, sources byte-identical. Slice 2 (`t_ee487800`): PDB track metadata carried through catalog (schema v4) and XML — Genre, Label, Tonality, Comments, DateAdded, Year, Size, BitRate, SampleRate, TrackNumber, DiscNumber, Rating (stars×51); id 0 / zero / empty = absent (not written); unknown or duplicated lookup id, non-`YYYY-MM-DD` date, rating > 5 = `UnresolvedInSource` (not written, never invented); export re-parse verifies every carried field plus BPM/length. Private run: counts still 1,330/20/901, source byte-identical, unresolved 0 for every field, an independent Python DeviceSQL decode equals the XML for all 12 fields on 1,330/1,330 tracks. Slice 3 (`t_cf1269e6`): read-only ANLZ decoder (`PQTZ` beat grid, `PCOB`/`PCO2` cues; unknown/duplicate/malformed sections fail closed; `PPTH` must name the track) carries the beat grid as one `TEMPO` per stored beat (`Inizio`/`Bpm`/`Metro="4/4"`/`Battito`, no averaging or interpolation, F-10 tuple semantics) and cues as `POSITION_MARK` (`Type` 0/4, `Start`/`End`, `Num` hot-cue index or -1, hot-cue `Red`/`Green`/`Blue` from `PCO2`) through catalog schema v5; cues come from the `.EXT` `PCO2` lists only when every `.DAT` `PCOB` cue also appears there; beats outside 1..=4, zero tempo or non-increasing times = `UnresolvedInSource`. Private run: counts still 1,330/20/901, PDB and all analysis files byte-identical; TEMPO carried 1,322 / absent 8 (7 unresolved grids, 1 empty); POSITION_MARK carried 238 / absent 1,092 / unresolved 0; an independent Python ANLZ decode equals the exported TEMPO and POSITION_MARK on 1,330/1,330 tracks. The per-beat grid makes the XML large (tens of MB for this library). Still not carried (1,330 each): Composer, Grouping, Remixer, Mix, Kind, DateModified, LastPlayed, PlayCount, Colour. Slice 4 (`t_02ecec22`): `export-library-xml --drive X` writes device-relative `file://localhost/Contents/…` locations as `file://localhost/X:/Contents/…` (one ASCII letter, case-insensitive; other locations unchanged; no option = byte-identical to before). **Owner-verified in real Rekordbox on Windows (2026-11-04; screenshot not committed):** the full per-beat export loads 1,330 tracks with BPM, key and CUE markers once the XML is on a local drive and locations carry the stick's drive letter (per-beat TEMPO loads; no tempo collapsing needed). Owner also confirmed (2026-11-04) the imported beat grids and cues display correctly, including a tempo change (145→180→145 BPM) and a manual beat-grid edit in one track: tempo and grid data in the export are used by Rekordbox, not re-derived by its own analysis. Rekordbox still prompts to analyse some tracks (waveforms are not carried by the XML; 8 of 1,330 source tracks have no beat grid at all). **Edit-then-reimport round trip confirmed by owner (2026-11-04, Rekordbox on Windows):** two OxiDJ guarded edits (a track's Comments field and another track's title) appear in Rekordbox after importing the edited export; a playlist import and playback of a track from the stick also worked, with the beat grid correct while playing (screenshots not committed). Rekordbox's own analysis prompt on some tracks is expected (waveforms are not carried by the XML). Slice 5 (`t_cf19bac4`): opt-in `export-library-xml --computed-grids` writes one constant `TEMPO` (first computed beat, computed BPM, 4/4, beat 1; AverageBpm filled only when absent) for library tracks without a vendor grid whose local file matches uniquely by the `/Contents/` suffix and whose computed confidence is ≥ 0.5; vendor grids untouched; counts-only report; no flag = byte-identical. Rekordbox load of computed grids NOT YET OWNER-VERIFIED. Catalog playlist create/append/reorder (slices 6–8; `0bbdc96`, `ca518c5`, `a5f73f1`) are reviewed and installed, with synthetic export proof only. Real FAT/exFAT export NOT RUN. Hardware (CDJ/XDJ) acceptance NOT DEMONSTRATED. |

## M7 slice 6 — catalog playlist creation (2026-11-06)

`t_20e6f34b`: `catalog create-library-playlist --catalog DB --name NAME --track-ids 3,1,3` independently reviewed and approved through `0bbdc96`; installed from that commit. Appends one root playlist atomically using exported XML TrackIDs (imported position + 1, not audio/PDB IDs), preserves ordered duplicate memberships, and refuses invalid arguments or a same-name root node without logical change. The library owns the transaction; application validates/composes; CLI has no SQL. Synthetic CLI regression independently decodes fresh-process exports and compares baseline plus exactly one final playlist, including metadata/grids/cues/tree/order; source PDB/ANLZ bytes and no-clobber behavior are checked. Focused storage tests cover reopen, original node identity and injected mid-write rollback. No owner catalog/private data or hardware used. Rekordbox acceptance of this new operation, ADR-005/A3, hardware, accessibility/platform and scale remain unverified; M7 is not closed. Usage/ID selection: README.

## M7 slice 7 — append members to an existing catalog playlist (reviewed + installed)

`catalog append-library-playlist-members --catalog DB --playlist-name NAME --track-ids 3,1` appends exported XML TrackIDs (imported snapshot position + 1) in supplied order, duplicates retained, to one exact unique root playlist. The library layer validates the target and every member inside one immediate transaction before mutation; storage errors roll back. A read-only preflight refuses non-imported SQLite inputs without changing schema, `user_version`, or sentinel data. Application maps IDs and owns composition; CLI has no SQL and emits only membership counts. Synthetic CLI coverage imports the fixture, appends to an existing root playlist, reopens and exports from a fresh process, then independently decodes the XML and compares the full baseline plus only the requested membership append; source PDB/ANLZ bytes remain unchanged. README documents the ID namespace, exact root-only target and personal-metadata privacy. Sol approved round 2 on `ca518c5`: CLI 13 passed, library rollback regression 1 passed, `cargo xtask ci-full` exit 0 (UI 229 passed, 2 ignored). Post-approval `bash scripts/install-local.sh` exit 0; installed `oxidj-cli` exposes the command and matches the release binary. No private catalog/device data used. Synthetic evidence only: vendor behavior, hardware, ADR-005/A3, global F-09, accessibility/platform and scale remain NOT DEMONSTRATED. M7 remains open.

## M7 slice 8 — reorder catalog playlist members (reviewed + installed, 2026-11-08)

`catalog reorder-library-playlist-members` accepts a complete permutation of the existing root playlist's exported XML TrackIDs, preserving duplicate counts, in one immediate transaction. Target selection and exact multiset validation run under the write lock; injected mid-insert failure rolls back. Read-only preflight rejects unsupported/unrelated databases before migration-capable open. The Sol audit corrected the README's sequential example to retain all five members (including duplicates), pinned the CLI pre-reorder export to the existing hand-written fixture oracle, and added malformed/overflow/cardinality/folder/nested-target and missing/empty-catalog refusal coverage. Fresh-process import → create → append → reorder → export compares all decoded metadata, grids, cues and tree/memberships against that oracle plus only the intended member reorder; source PDB/ANLZ bytes are unchanged. The Sol 6.1 audit/fix-all corrected the README workflow and closed the independent-baseline/refusal test gaps; final review run 2077 approved `a5af0e0` + `a5f73f1`. Focused CLI suite: 15 passed; storage rollback/refusal regression: 1 passed; `cargo xtask ci-full` exit 0 (`ci=ok test=ok ui-test=ok deny=ok`, UI 238 passed / 2 ignored). Installed after approval with `bash scripts/install-local.sh`; `~/.local/bin/oxidj-cli --help` exposes the command. Synthetic-only evidence; vendor and hardware, ADR-005/A3, global F-09, accessibility/platform, and scale remain NOT DEMONSTRATED. M7 remains open for owner-side checks and qualification. |

## M7 catalog track selection / install (reviewed + installed)

`t_05b2713d` (review round 1): `catalog list-library-tracks --catalog DB` lists
exported XML TrackIDs, titles and optional artists in imported order, with
reversible escaped TSV. This is an explicit personal-metadata display: local
viewing only, never task logs or transmission. Existing current-schema catalogs
are opened read-only without migration; missing paths are refused and an empty
library is labelled. Synthetic CLI tests compare actual listing to independently
decoded actual XML, create a playlist using listed IDs, reopen/export and verify
ordered duplicate membership, source bytes, catalog snapshots and existing XML
preservation; missing/empty and escaped metadata (including missing versus empty
artists) are covered. No personal catalog was opened.

Review: approved (commit b48be001b0f83f005b1787665e7c33775fa4f913, upstream card
t_05b2713d, review round 1; local/orchestrator review, no push).

Install (post-approval standing install): `bash scripts/install-local.sh` succeeded
on local `master` (commit 8c0fd77); installed oxidj-cli,
oxidj-library-ui and oxidj-library under ~/.local/bin. Installed
`oxidj-cli --help` lists `catalog list-library-tracks`, and the command reads
only the --catalog path given on the command line, so no personal catalog is
opened. Done.

M7 remains open: ADR-005/A3, vendor/hardware, accessibility/platform and scale
qualification are NOT DEMONSTRATED by this synthetic workflow.

## M7 synthetic preservation proof (review corrections, 2026-11-06)

`t_4a3c5bb7`: the reproducible command is
`cargo test -p oxidj-cli --test library_import_export m7_proof_import_manage_export_large_library -- --exact`.
It generates a synthetic DeviceSQL/ANLZ library (1,330 tracks / 20 playlists /
901 memberships), invokes the actual CLI import/export and six guarded edits,
and calls `scripts/verify_m7_export.py` for the baseline and every edited output.
The oracle independently decodes the PDB with `scripts/decode_pdb.py` and compares
all exported track attributes, exact synthetic cue/grid attributes, complete
ordered folder/playlist attributes and every membership vector against baseline
plus only the intended edits. Controls reject unrelated title, cue, grid,
member replacement and order-only corruption (same counts and valid IDs).
Source PDB/ANLZ bytes remain unchanged; audited unresolved fields are zero.
This replaces the earlier spot-check preservation claim; Sol approved corrective
commit `8eb806f` in review round 2. The focused regression passed (1 test), and
`cargo xtask ci-full` passed (`ci=ok test=ok ui-test=ok deny=ok`). Synthetic
evidence only: no personal catalog opened, no native/vendor, hardware, ADR-005,
accessibility or scale qualification implied.

## M4 workstreams

| Workstream | Evidence (details in history file) | Status / next gate |
|---|---|---|
| Rekordbox XML preview and no-edit export | Read-only preview; bounded fail-closed no-edit subset export; byte-preserving retention incl. POSITION_MARK `End`/`Num`/colours; F-10 tempo tuple `(16,132,"4/4",1)` per ADR-005 §3 (`f5caa67`). | Synthetic-only. |
| Guarded XML edits (CLI + provisional UI) | Playlist: member reorder/remove/append/insert, clear, create, rename, delete, sibling reorder, cross-folder move; folders: create/delete/rename/sibling reorder/subtree move; shared-track delete; TRACK attributes (Name, Artist, Comments, Album, Genre, Label, Grouping, Composer, Remixer, Mix, Tonality, Kind, Year, Rating, Colour, DateAdded, DateModified, LastPlayed, PlayCount, AverageBpm, SampleRate, DiscNumber, …). Each: source-bound preparation, preview/cancel, stale-state refusal, new-file-only export, reopen, independent F-02 whole-tree oracle with mutation controls. Sol-reviewed per increment. Export publish is no-clobber (`61a5f71`). | Synthetic XML only; Bot Screen live interaction NOT RUN. **Frozen: no further per-field increments.** |
| F-09 provenance | 50 operation oracles bound by path/SHA-256 (incl. shared helpers) to 45 frozen inputs / 57 artifacts in `xtask/tests/fixtures/f09-operation-provenance.json`; B1/B2 logical snapshots and sentinel/manifest integrity controls are test-only. | Bounded F-09.1 provenance only; global F-09 NOT DEMONSTRATED. |
| DeviceSQL PDB | Read-only preview (`3e8d9cf`, Sol run 1628); bounded scratch-copy smoke: 1,330 tracks / 20 playlists / 901 memberships, source and scratch byte-identical. | Independent format slice; no hardware/application compatibility claim. |
| OneLibrary reader | Synthetic encrypted profile v1 (`fd93e29`) and provisional native profile (`49ed6cd`, Sol run 1653) through an isolated SQLCipher reader outside the main Cargo graph; scratch smoke: 22 tables / 4 indexes, 379 tracks / 4 playlists / 228 memberships; `playlist.attribute` uninterpreted. | Provisional structural evidence only; A3/ADR-005 remain NOT DEMONSTRATED. |
| A3 native evidence | Owner-authorized, read-only local profile-2 probe decoded aggregate structure: 379 tracks, 4 playlists, 228 ordered memberships. This establishes local decode feasibility only; private source details remain local. | Community/library sources describe OneLibrary as a Rekordbox USB/SD SQLCipher export and publish a passphrase they label verified against a real Rekordbox 7 export ([v1vendi OneLibrary spec](https://raw.githubusercontent.com/v1vendi/onelibrary/master/spec/ONELIBRARY.md), [pyrekordbox format docs](https://pyrekordbox.readthedocs.io/en/latest/formats/onelibrary.html)); these are not vendor attestations. The prior probe record does not identify its passphrase or establish that it used this public candidate: probe-to-source match NOT DEMONSTRATED. Staged-source provenance/version, independent-oracle agreement, lawful exact-record/container extraction and preservation, and vendor/hardware compatibility remain NOT DEMONSTRATED. A3/ADR-005 remains NOT DEMONSTRATED. |

## NOT DEMONSTRATED gates (none is implied by synthetic tests or bounded local smoke)

- **ADR-005 / A3:** ADR-005 Proposed; A1–A3 and C1–C6 OPEN / NOT QUALIFYING. Aggregate-only local profile-2 decode feasibility is demonstrated; staged-source provenance/version, probe-to-public-candidate match, independent-oracle agreement, lawful exact-record/container extraction and preservation, and vendor/hardware compatibility remain NOT DEMONSTRATED.
- **Vendor compatibility:** no Rekordbox/hardware/application compatibility claim; the PDB and OneLibrary previews are structural only.
- **Global F-09:** only listed synthetic operation oracles exist; global F-09 remains NOT DEMONSTRATED.
- **Production GUI/accessibility/platform (ADR-001):** the prototype is provisional; Bot Screen smoke for newer increments NOT RUN; screen-reader/platform checks NOT RUN; the normal standalone prototype production build is not claimed.
- **500k / 60fps:** representative 500k SQLite qualification is NOT READY / NO-GO and the runner NOT PASSED; no benchmark, fault campaign, fixture qualification or 500k qualification has run; 500k-row 60fps is deferred and unverified. The 10,000-track cap is a scope boundary, not capacity evidence. Process-crash/reopen/backup evidence is not OS/power-loss or representative-scale qualification.
- **Waveform:** native display, accessibility, real-library waveform quality beyond a single eyeballed track (no reference waveform), GPU upload and presented FPS, realtime/playback-synchronised rendering, Rekordbox visual parity. Vendor PWV strip is covered by a UI test only, not by a capture.
- **Real FAT/exFAT export:** the no-hard-link fallback is forced by a test, not a vfat device; NOT RUN on a real mount.

Revised local M1–M3 acceptance is complete; the named `local-catalog-alpha-v1` exception (PR #13, merge `671ae2b3df933ada97297bbba19ea1394657c14e`) covers that bounded slice only. Do not claim historical M1, 500k readiness, or qualification PASS. WAL/FULL (ADR-002) is binding. Full qualification detail: history file, "Qualification checkpoint" and "Blockers".

## Next actions

1. **M7 core import/manage/export workflow:** local software workflow implemented, reviewed, and installed through slices 1–8: staged-library import, metadata and playlist management (including create, append, and reorder), export, and owner-confirmed Rekordbox load/playback plus guarded edit-then-reimport; aggregate-only counts and independent-oracle checks are recorded above. Remaining owner-side checks are listed separately; no hardware claim.
2. **Computed grids in XML export:** done (`d295fc4`, opt-in `--computed-grids`, only tracks without a Rekordbox grid); the owner's Rekordbox check of those tracks is pending.
3. **Owner-side checks:** Rekordbox import of the export and an edited re-export are owner-confirmed (M7; per-beat `TEMPO` loads, no collapsing needed); still open: real FAT/exFAT export on the `PATRIOT` stick (NOT RUN) and hardware (CDJ/XDJ).
4. **A3 native evidence:** aggregate-only local profile-2 decode feasibility established (379 tracks / 4 playlists / 228 ordered memberships). Resume only when the applicable public source URL/revision or native container/format/version metadata is available with a verified read-only view. Do not repeat prior key/reader attempts without new applicable provenance or a materially different authorized reader path. Independent-oracle agreement and lawful extraction/preservation remain required; A3/ADR-005 qualification remains NOT DEMONSTRATED.
5. **Deferred, unplanned until a running product exists:** production GUI/accessibility/platform admission, 500k/60fps qualification, `cargo xtask ci-full` re-run after next product change, native-display check, push (optional backup).

Gate commands: `cargo xtask ci` (every handoff), `cargo xtask ci-full` (CI; includes `ui-test`). See `docs/handoff-checklist.md`.
