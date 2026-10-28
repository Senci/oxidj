# Pre-alpha 53 — signed POSITION_MARK Num boundary

These two synthetic-only milestone cards record the signed-i32 parser/preview acceptance and exact no-edit export retention for `POSITION_MARK Num="-1"`. The preview regression covers i32 minimum/maximum, `-1`, and rejection of empty and out-of-range values; the export regression compares complete output and source bytes.

The cited official XML format list documents Num as signed 32-bit and identifies Memory Cue as `-1`. This evidence does not interpret cue indices, renumber cues, validate behavior in rekordbox/OneLibrary, or qualify ADR-005. Vendor compatibility remains unproven; ADR-005 remains Proposed.
