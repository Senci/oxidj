# ADR-005 A2: Mixxx transport binding proposal

**Status:** Bounded research proposal only. ADR-005 remains Proposed; A2 and qualification remain NOT DEMONSTRATED. No adapter, DB persistence, or application acceptance is claimed.

## Verified pinned facts

The reviewed Mixxx source is tag 2.5.6, peeled commit `3ebac449e7e5fe2a0186596657696e87ce8b0e56` (GPL-2.0-or-later). `src/proto/beats.proto` is proto2: `BeatMap` is repeated `Beat` values (`frame_position` optional int32, `enabled` default true, `source` default ANALYZER); `BeatGrid` contains optional `Bpm` and `first_beat`; `Bpm` contains optional double BPM and source. Proto source enum is ANALYZER=0, FILE_METADATA=1, USER=2. The proto defines neither sample rate nor time signature. `src/track/beats.h` names BeatGrid-1.0, BeatGrid-2.0, BeatMap-1.0; its `Beats` abstraction is constructed with sample rate, format/sub-version and frame positions. Its documented constant-tempo representation uses no transition markers and a final tempo marker at the first downbeat; variable tempo uses transition markers plus a final tempo marker.

`src/track/cueinfo.h` defines CueInfo as a DTO, not a file format: type, optional start/end milliseconds (`double`), optional hotcue index (`int`), label, optional RGB color, flags. Hotcue index begins at zero. `Locked` is used for Serato import/round-trip tests and explicitly is not saved in Mixxx's database. Cue types include HotCue, MainCue, Loop, Jump, Intro, Outro and an audible-range type. Existing project research reports loops as ranges, absolute cue positions in milliseconds, and no established tempo-to-cue dependency.

The existing OxiDJ relationship contract requires A2 to pin the adapter envelope, proto/sample-rate/CueInfo optionality and color transport, raw bytes, real-target equality, and absent-position policy; it explicitly warns DTO equality does not prove DB flags persist or application acceptance.

## Recommendation (not a decision or implementation)

Define one versioned **Mixxx beat transport envelope** per native beat record: `{ format_version, sub_version, sample_rate_hz, encoding, raw_bytes }`. `encoding` is exactly `BeatGrid-1.0`, `BeatGrid-2.0`, or `BeatMap-1.0`; `raw_bytes` is the exact serialized proto payload, not a re-encoding. Require positive finite sample rate as explicit context because frame positions have no intrinsic time unit in the proto. Preserve unknown version/sub-version and payload bytes opaquely; do not parse-and-reserialize for no-edit round trips. For semantic comparison, decode only under an explicitly supported encoding and compare the complete typed message including presence/default distinction as actually serialized, plus the exact sample-rate context and version tags. Do not equate differing proto encodings merely because they project to similar beats.

Carry CueInfo as a separate versioned DTO record, preserving each optional field's presence distinctly from zero/empty: type token, start/end milliseconds, hotcue index (including 0), label, optional RGB components, flags, and exact raw bytes. Retain raw bytes as authoritative source-native evidence; projection is derived. Compare DTO fields by exact optional presence and value, with floating-point representation/rounding policy unresolved; do not use C++ `operator==` as a transport or persistence oracle. Never infer that flags or colors survive Mixxx DB persistence from DTO round-trip equality.

When position is absent, retain absence in the source record and neutral projection; do not synthesize zero, a frame, or a beat lock. An explicitly present endpoint remains independent absolute milliseconds. Do not convert it using sample rate, infer an anchor relationship, or invalidate it after a grid edit absent separately declared OxiDJ beat-lock semantics. If an edit targets a position encoding whose synchronization is unqualified, preserve original bytes and withhold the affected export unit under `safe-v1`, reporting unsupported position edit; no selective rewriting of alternate encodings.

## Unresolved proof / limits

This proposal does not establish actual Mixxx library-file/DB serialization, whether the described proto payload is the exact persisted envelope, DB round-trip retention of optional fields/colors/flags, numeric precision and equality rules, absent-position UI behavior, target-version compatibility, or application acceptance (including cue-capacity overflow). These require version-pinned real artifacts and independent extraction/checking under A2. No GPL implementation was copied and no dependency is proposed.

A3 OneLibrary, global F-09, C1–C6, and ADR-005 qualification remain NOT DEMONSTRATED.

## Pinned primary sources

- Mixxx 2.5.6 `beats.proto`: https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/proto/beats.proto
- Mixxx 2.5.6 `beats.h`: https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/beats.h
- Mixxx 2.5.6 `cueinfo.h`: https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/cueinfo.h
- Mixxx 2.5.6 `cue.h`: https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/track/cue.h
- Mixxx 2.5.6 `cuedao.h`: https://github.com/mixxxdj/mixxx/blob/3ebac449e7e5fe2a0186596657696e87ce8b0e56/src/library/dao/cuedao.h
- OxiDJ [ADR-005](../adr/005-metadata-schema.md), [relationship fixtures §A2](../adr/005-relationship-fixtures.md#the-complete-binding-map), and [Mixxx semantics research](mixxx-semantics.md).
