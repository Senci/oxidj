# Mixxx beatgrid and cue semantics (research)

Research snapshot: Mixxx manual 2.5 and source tag 2.5.6, peeled commit `3ebac449e7e5fe2a0186596657696e87ce8b0e56`. This is interoperability research, not an OxiDJ schema decision and not a port of Mixxx C++.

The pinned Mixxx `LICENSE` states GPL-2.0-or-later. This note records observed semantics and links to upstream sources; it copies no implementation code. Any future code reuse requires separate license review.

## Findings

- Mixxx distinguishes constant-tempo BeatGrid (BPM + first beat in legacy `src/proto/beats.proto`) from BeatMap (individual beat positions). The legacy BeatMap beat carries `enabled` and source (ANALYZER, FILE_METADATA, USER). The current `src/track/beats.h` abstraction can express constant tempo as no transition markers plus a first-downbeat tempo marker, and variable tempo as transition markers plus a final tempo marker. It provides legacy BeatGrid-1.0/2.0 and BeatMap-1.0 adapters.
- Positions use audio frames plus sample rate in Beats; CueInfo interchange positions are milliseconds. BPM and anchor/phase are independent: phase shifts move the grid; BPM changes spacing. Manual distinguishes uniform offset error from tempo drift and notes small BPM error accumulates over long tracks.
- The reviewed 2.5.6 legacy proto has no time signature. Do not infer 4/4 or bar numbering from beat index. Detailed persisted bar/meter behavior remains unverified.
- Cue types include main cue, hotcue, loop, intro/outro, jump, and a non-user-visible audible range. CueInfo has optional endpoints, hotcue index, label, color, flags; hotcue index zero is valid. Its locked-loop flag is used in Serato import/round-trip tests but **not saved in Mixxx's database** (`cueinfo.h`); do not infer durable flag support from this DTO. Loops are ranges. Mixxx documents one main cue, one loop, and 38 hotcues. Rekordbox mapping can turn memory cues/loops into other cue types; conversion may be lossy.
- Analysis can produce BPM/grid, key, ReplayGain and waveform overview. Analyzed values are not proof of user validation. Distinguish analyzer/file/user provenance and avoid silent overwrite of edits. Effective BPM includes playback-rate adjustment; key lock decouples pitch from tempo changes. Quantization is behavior; beatloop falls back to seconds absent beat data.

## Invariants and edge cases

Preserve grid representation, anchor, source/provenance, enabled state, precision, sample rate and analysis context where available. Preserve loop endpoints and cue category/index/label/color; preserve imported flags only when evidence shows they exist, and do not assume a Mixxx database round-trip retains them. Do not clamp valid extrapolated beats to track bounds or flatten variable tempo without recording loss. Handle missing grids, absent optional endpoints, unknown cue types/flags and target-capacity overflow explicitly. Validate units and monotonic/finite values; exact thresholds, duplicate policy, cue rounding/tie rules, loop endpoint inclusivity, waveform storage, key normalization and meter persistence were not established by this review.

## Versioned primary sources

- [Mixxx 2.5.6 `beats.proto`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/proto/beats.proto)
- [Mixxx 2.5.6 `beats.h`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/beats.h), [`beats.cpp`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/beats.cpp)
- [Mixxx 2.5.6 `cueinfo.h`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/cueinfo.h), [`cue.h`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/cue.h), [`cuedao.h`](https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/library/dao/cuedao.h)
- [Mixxx manual 2.5 Beat Detection](https://manual.mixxx.org/2.5/en/chapters/preferences/beat_detection), [Library](https://manual.mixxx.org/2.5/en/chapters/library), [User Interface](https://manual.mixxx.org/2.5/en/chapters/user_interface), [Getting Started](https://manual.mixxx.org/2.5/en/chapters/getting_started)
- The [Beat and Bar Edit Workflow](https://github.com/mixxxdj/mixxx/wiki/Beat-and-Bar-Edit-Workflow) wiki page is historical 2020 design context, not a versioned contract.

Detailed evidence, source captures, provenance and open questions: `/home/senci/dev/oxidj/.wiki/concepts/mixxx-beatgrid-and-cue-semantics.md`. No Mixxx code was copied into OxiDJ.
