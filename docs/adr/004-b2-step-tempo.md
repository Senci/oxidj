# ADR-004 B2 addendum: bounded computed step-tempo inspection

Implementation contract adopted from the attached proposal on `t_5e5411f0` by
authorized card `t_08409dc4`; independent audit/review pending. Existing B2
constant-grid behavior is unchanged.

### B2 extension: bounded computed step-tempo inspection (implementation contract; audit pending)

**Scope and boundary.** Add one opt-in, local, synthetic-only analysis/inspection operation. Its observable result is a constant suggestion, a piecewise-constant variable suggestion, or a refusal with a reason. The application composes decoded mono PCM from the library with analysis and presents the result through one CLI inspection path. The inspection path takes no catalog database and performs no catalog, media, or vendor writes. It is not part of normal import, background analysis, the existing `catalog analyze-tempo` persistence operation, or UI playback. Synthetic fixtures establish this contract only; they do not qualify real-music accuracy or vendor compatibility.

`oxidj-core` owns dependency-free validated result data; `oxidj-analysis` owns estimation and refusal; `oxidj-application` owns decode/composition and the typed inspection result; CLI only formats that result. Decoding remains in `oxidj-library`. The existing exact `taktus =0.1.0` admission, NOTICE obligations and dependency boundaries remain unchanged. This amendment admits no new dependency, feature or engine pin. The existing global-BPM `TempoBuilder` is not evidence of variable-tempo support: the new operation must satisfy the fixtures below before implementation acceptance.

**Separate representation.** Use a new neutral result, conceptually `ComputedStepTempoV1`, with explicit outcomes `Constant`, `Variable`, and `NoMap(reason)`. Do not extend, reinterpret, or silently convert the existing `TempoGrid`, its scalar BPM, or its detected-beat list into this map. Both successful outcomes use the new representation: one segment for `Constant`, two or three for `Variable`. There is no scalar average-BPM substitute for `Variable` and no implicit conversion to `TempoGrid` or `LibraryBeat`.

A successful result contains:

- representation identifier `computed-step-tempo-v1`, algorithm identifier/version (including the step estimator, not just the engine), fixed policy/version and analysis basis (sample rate, exact decoded frame count, fixed BPM band);
- first detected beat anchor, exclusive support end, and ordered segments, each containing its start anchor, finite positive BPM, ordinal of that anchor counted from the first detected beat, and segment evidence;
- segment evidence: finite taktus `sure` and `catch`, maximum matched-beat timing residual in milliseconds, supported predicted-beat fraction, and unexplained onset-energy fraction. Scores are diagnostics, not probabilities. Do not invent meter, bar indices, downbeat, or a calibrated accuracy confidence.

All time positions are nonnegative integer **milliseconds from decoded PCM frame zero**, not from the first onset, container timestamps, a cue, a vendor grid or playback time. Duration is derived from frame count/sample rate; support end is its floor in milliseconds. Convert estimated times by nearest-millisecond rounding once at the result boundary; estimate with unrounded times internally. Returned anchors must be strictly increasing and lie before support end. The first segment begins at the first detected beat, not necessarily zero; the result makes no beat claim before it or at/after support end. The first beat ordinal is zero and is not a downbeat assertion.

**Step and phase semantics.** Segment `i` owns `[start_i, start_(i+1))`; the last owns `[start_last, support_end)`. BPM is constant within an interval. There are no tempo ramps or interpolation of BPM between anchors. Predicted beats are spaced by `60000/BPM` milliseconds from the segment anchor. At a change anchor the incoming interval has completed an integer number of old-tempo beats; the outgoing interval uses the new period. The boundary beat exists exactly once, belongs to the new segment, and its ordinal continues the previous count. Consequently `start_next = start_i + (ordinal_next - ordinal_i) * 60000/BPM_i`, allowing at most 1 ms discrepancy from final time rounding. Do not reset phase, insert/delete a beat, or duplicate the boundary beat to make two incompatible fits join. Changes not supportable at a shared beat anchor are refused. This is an OxiDJ analysis convention, not a claim about imported formats.

**Admitted input and classification.** Version 1 deliberately supports only a fixed, inclusive **100–160 BPM** band, 8–180 seconds of mono finite PCM, integer sample rates 8000–48000 Hz, and at most two changes. This sub-octave band avoids deciding between both halves/doubles inside the admitted band; it is a limitation of this inspection experiment, not a new default for existing analysis. Do not octave-fold individual segments into this band. Outside-band evidence or unresolved rhythmic-level/phase ambiguity produces `NoMap`, not a fabricated in-band answer.

Every segment must span at least 8 seconds and contain at least 12 complete predicted inter-beat intervals. Every segment must have `sure >= 0.05`, finite `catch`, maximum timing residual <=30 ms, >=90% supported predicted beats, and <=10% unexplained onset energy. Support is a one-to-one match between predicted beats and detected onset peaks within 30 ms. Unexplained energy is the fraction of total nonnegative onset-envelope energy outside those matched peaks' +/-30 ms windows. Zero total energy cannot pass. These limits and peak-extraction policy belong to the estimator's versioned policy; peak extraction must be deterministic, documented and frozen before qualification, not tuned per fixture. Passing `sure` alone is insufficient.

For two positive BPMs define relative difference as `abs(a-b)/min(a,b)`. Treat <=1% differences as the same tempo and merge adjacent equivalent segments **only if the merged fit also passes all support/phase gates**. Try a whole-span constant fit first; when it passes unambiguously, report `Constant`, never jitter-derived changes. Otherwise report `Variable` only when every adjacent change is >=5%, all segments and joins pass, and the complete span is explained. Differences strictly between 1% and 5%, unmergeable near-equal segments, unsupported spans or an unresolved change are `NoMap`; do not average them away. Nonadjacent equal BPMs are not merged: an A–B–A sequence remains three segments. A failed step fit is never permission to return a coarse constant estimate that failed the whole-span gate.

Before returning a map, compare competing rhythmic/phase and segmentation hypotheses. For this ambiguity check, a candidate must meet the duration, beat-support, sure and residual requirements but need not yet pass the unexplained-energy gate (equal-strength conflicting pulse trains can fail that final gate for both phases). If another such candidate has absolute differences from the best such candidate of <=5 percentage points in supported-beat coverage, <=5 ms in maximum residual, and <=5 percentage points in unexplained energy, yet disagrees on tempo by >1%, phase by >30 ms, or change position by >30 ms, return `NoMap(Ambiguous)`. Compare phase modulo the local beat period, so shifting the beat index alone is not a competing phase. Prefer the fewest segments among otherwise equivalent fits. Reject continuous drift/ramp evidence rather than approximating it with steps. No claim is made to distinguish every possible real-world ambiguity from synthetic PCM.

**Refusal and resource bounds.** A refusal carries no usable or partial map. Distinguish at least silent/no-periodic-evidence, ambiguous, insufficient/short-segment, unsupported-rate/range/non-step, and non-finite-input outcomes. Decode/I/O errors and invalid options remain errors; a resource-cap violation is an explicit `ResourceLimit` error, not successful analysis of a truncated prefix. An empty stream is insufficient input. Never panic or emit NaN/infinite scores.

Count exact source frames with checked arithmetic: accept at most `180 * sample_rate` samples and reject the first excess sample regardless of chunking. Keep no decoded-audio history; retain at most 18000 onset-envelope values at 100 Hz plus fixed engine state and bounded scratch. Analysis-owned live heap is capped at 8 MiB, excluding caller-owned input and decoder buffers. Limit output to three segments, evaluate at most 4096 segment candidates, and never retry without a bound; candidate-budget exhaustion is `ResourceLimit`, not ambiguity or a partial result. No wall-clock performance promise is made by this contract; implementation must demonstrate the storage/work bounds with counters or focused cap tests before acceptance.

For identical sample rate, options and ordered PCM, changing chunk boundaries (including empty pushes) must produce **exactly equal** result data, evidence, classification and refusal/error reason in the same build. Frame counts, onset extraction, fitting windows and resource accounting are anchored to the stream, never to push calls. Floating-point results need not be bit-identical across platforms, but each platform must satisfy the same fixture tolerances. No early returned suggestion may conceal a later invalid sample, unsupported section or cap crossing.

**Synthetic acceptance fixed before implementation.** Reuse the kick/click renderer in `crates/oxidj-analysis/tests/tempo.rs` (55 Hz decaying kick at 0.6 plus 1.8 kHz decaying click at 0.3, 250 ms support) with truth beat times constructed independently of the analyzer. Let the first beat be 300 ms. Generate the cases below at both 8000 and 44100 Hz; add a positive 48000 Hz control. A change beat is rendered only once. Beat times are rounded directly to source frames, not accumulated through rounded periods.

| Fixture | PCM/truth | Required result |
|---|---|---|
| Constant | 40.3 s; 120 BPM from 0.3 s | `Constant`, one segment, no changes |
| Two-tempo step | 40.3 s; 120 BPM on [0.3,20.3), 150 on [20.3,40.3) | `Variable`, exactly two segments; change at 20.3 s |
| Return | 60.3 s; 120 on [0.3,20.3), 150 on [20.3,40.3), 120 on [40.3,60.3) | `Variable`, exactly three segments; changes at 20.3 and 40.3 s; no A–B–A flattening |
| Ambiguous phase | 40.3 s; equal-strength copies of the same 120 BPM pulse train offset by one third of its beat period, with no accent selecting either phase | `NoMap(Ambiguous)`; no arbitrary phase winner |
| Silent/nonperiodic | 30 s zeros; 30 s DC 0.4; 30 s xorshift noise from each existing tempo-test seed | `NoMap`, no constant or variable suggestion |
| Short | empty; one sample; 7.9 s of 120 BPM | `NoMap(Insufficient)` |
| Short middle segment | 43.5 s; 120 on [0.3,20.3), 150 on [20.3,23.5), 120 on [23.5,43.5) | `NoMap(ShortSegment)`, not constant or a dropped middle section |
| Unsupported model | 40.3 s linear tempo ramp 120 to 150 BPM with beats defined by integrating tempo; 40.3 s step 120 to 123 BPM at 20.3 s, beat-aligned | `NoMap`, respectively non-step/unsupported and ambiguous-change guard band |

For positive fixtures, each segment BPM is within **0.5 BPM** of truth; first anchor and each change anchor are within **30 ms** of truth. Generated map beats must match truth **one-to-one within 30 ms**, including the change beats, with no interior omissions/extras. Cover the truth beats from the first to last rendered beat, allowing at most one beat omitted at either outer edge, never at a change. These phase/coverage assertions are additional to BPM tolerances; BPM agreement alone does not pass. Check the analytic boundary-continuity equation above, segment count and classification explicitly. The linear ramp uses `BPM(t) = 120 + 30*(t-0.3)/40` on [0.3,40.3); pulse times solve the integral beat count, not a staircase generator.

For each fixture compare a whole-stream push with chunk sizes 1, 7 and 4096, and an irregular repeating partition [13,509,1,4096] with empty pushes inserted; require exact result equality. Test NaN and each infinity at the start, inside, and after an otherwise acceptable span; each must refuse without a map. Test sample rates 0, 7999 and 48001; outside-band 90 and 174 BPM; 180 s and one source frame over it; and candidate/memory/output caps without allocating an over-cap structure. The exactly-at-duration-cap control must not fail merely for duration (a content refusal remains possible); one frame over must report the cap violation. If multiple independent failures occur, validate options/rate first, then use ResourceLimit > NonFiniteInput > content refusal precedence, independent of chunk boundaries. Add an 80.3 s fixture alternating 120/150/120/150 BPM at 20.3, 40.3 and 60.3 s: exceeding the supported number of genuine changes must return NoMap(UnsupportedModel), not flatten or drop a section. Fewer than three changes in the returned map is not a passing outcome for that fixture.

Finally, on a temporary **synthetic** catalog, prove the inspection leaves existing constant computed rows, vendor grids/cues/metadata/playlists, imported precedence, source bytes and normal/`--computed-grids` export artifacts unchanged. Existing variable-grid SYNC refusal remains unchanged, and the new result has no path to decks, the constant exporter or `store_computed_beat_grid`. Run existing focused regressions plus the repository gates after implementation; synthetic qualification must report actual outcomes, not be inferred from this proposal.

**Persistence and exclusions.** This increment is inspection-only: no schema migration, new catalog table, cached map, automatic saved report, or rewrite of `track_tempo_grids` format v1. The representation identifier versions the inspection result, not the database. Persistence would require a separately reviewed explicit map representation/encoding and catalog schema migration, preservation of existing constant rows and imported payloads, source identity/staleness handling, transactional write-failure and fresh-process reopen tests; it must not relabel existing format-v1 rows or make variable maps reachable by the constant exporter. That is not accepted by this amendment.

Manual authoring, arbitrary maps/dense beat-map editing, ramps, meter/downbeat inference, vendor interpolation semantics, vendor export changes, variable-tempo SYNC, playback integration, UI expansion, private-library evaluation and accuracy/parity/hardware claims remain out of scope. Failure of the admitted engine/estimator to meet this bounded contract must be reported; do not relax tolerances, add dependencies, expand scope, or claim implementation acceptance silently.

## Frozen estimator policy v1

`oxidj-step-fit-v1 / taktus 0.1.0` with
`step-policy-v1:100-160;8-180s;2-changes` uses the **square of the nonnegative
taktus spectral-rise envelope** at 100 Hz as onset energy. Squaring concentrates
transients; this is not the raw envelope used by the unchanged B2 builder.
Peak extraction selects local maxima above 20% of the global maximum, with
80 ms deterministic left-to-right suppression. Times use the exact source-frame
time of the first emitted value, then 10 ms increments, minus the admitted
engine's 22 ms detection latency, plus 5 ms to locate the centre of the 10 ms
onset cell rather than its lower boundary. This prevents a consistently early
quantized anchor from predicting a spurious beat just before the exclusive end
of the positive fixtures. A fixed 80 ms lead would be wrong at 8 kHz.
This policy is common to every fixture and sample rate; none of the numeric
acceptance tolerances above are relaxed.

Whole-span ordinal least-squares fitting is tried first. On failure of its
residual limit, the worst run is split at the candidate beat minimizing the two
least-squares errors; both sides include the shared boundary. At most four
working ranges are retained. Adjacent ranges are re-fitted and merged when the
combined fit passes. Four genuine segments are refused, never truncated.
Segment fits must still pass every timing, support, energy, duration, change,
engine-score and join gate. A low residual alone does not accept a map.

Competing pulse phases/tempos are seeded from differences among the first eight
peaks at separations of up to four peaks. Hypothesis energy is measured against
matched peaks, not inferred from peak counts. Same-tempo competing phases with
similar full support/residuals are conservatively refused even when overlapping
pulse decays bias spectral onset energy toward one phase; this detector artefact
must not choose a rhythmic phase. The general ambiguity comparison retains the
5 percentage-point energy, 5 percentage-point support and 5 ms residual gates.
Neighboring beat-aligned segmentation hypotheses (up to two beats either side)
are challenged through the same per-segment evidence gates, before the 10%
unexplained-energy cutoff. This bounded search is not an exhaustive real-music
ambiguity solver. No partial or early suggestion is emitted.

Storage bounds: one fixed-capacity 18,000-element f32 envelope, at most 2,250
peak times and matched-peak scratch, 64 pulse hypotheses, four working ranges
and three output segments. The admitted taktus code uses linear envelope/sum
scratch, fixed-lag correlation scratch and bounded voting; a conservative
36 bytes per envelope bin plus peak/hypothesis/fixed overhead is below 1 MiB
(`StepTempoBuilder::heap_bound_bytes`), below the 8 MiB contract. Every slice or
pulse candidate consumes one shared 4,096-candidate budget; a dense-envelope
unit control exhausts it and returns `ResourceLimit` without a partial map.
The explicit-file decoder stops on a source-frame cap violation. Empty pushes
never advance source frames or the onset clock.

`ConstantStepMap` and `VariableStepMap` have private construction, selected only
from a validated `StepMap` by segment count. Public enum construction therefore
cannot relabel a one-segment map as variable. Neither wrapper converts to
`TempoGrid` or imported library beat types. The map exposes representation,
algorithm/policy identifiers, exact frame/rate basis and the fixed BPM band.

## Implementation status

Synthetic implementation evidence is recorded on `t_08409dc4`; independent
Sol audit/fix-all and final review are gated by `t_4a979b1b`. This addendum does
not claim review approval, installation, real-music accuracy, vendor compatibility,
persistence admission or variable-tempo SYNC support.
