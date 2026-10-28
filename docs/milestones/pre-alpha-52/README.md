# Pre-alpha 52 — POSITION_MARK Num no-edit byte retention

Two synthetic-only milestone cards document the accepted A1 F-07 CLI export regression for exact `POSITION_MARK Num` text retention. The test covers distinct accepted numeric spellings (`00`, `01`), byte-identical exported output, and unchanged source input.

The existing parser boundary is Rust `u32` parsing only; this evidence assigns no cue-index semantics, does not widen parsing, and makes no Rekordbox/OneLibrary compatibility, global F-09, or ADR-005 qualification claim. These are retrospective documentation cards, not vendor captures.
