# Task 5 — Patch review: off-by-one in `BpmRange::contains`

Date: 2026-10-15 · Free worker: `poolside/laguna-s-2.1:free` via Nous Portal (M0-01b)

## The patch under review

```
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -16,7 +16,7 @@ impl BpmRange {
     /// `true` if `bpm` lies within the inclusive range [low, high].
     pub fn contains(&self, bpm: u16) -> bool {
-        bpm >= self.low && bpm <= self.high
+        bpm >= self.low && bpm <  self.high
     }
 }
```

## Review verdict

**REJECT — semantics-preserving-looking but incorrect boundary change.**

- **What it does:** changes the range check from *inclusive* (`<=`) to *exclusive* (`<`)
  on the upper bound. `contains` still compiles — no type/ownership error — so a
  compiler-only check passes this patch.
- **Why it is wrong:** the doc comment states the range is *inclusive*
  `[low, high]`, and the baseline contract is "the upper BPM is a valid BPM".
  With `< self.high`, the top of the valid range (e.g. 180 in `[60, 180]`) is
  silently dropped, rejecting a BPM that the range advertises as valid.
- **How it was caught:** `cargo test` fails on the existing boundary test
  `assert!(r.contains(180))` → `assertion failed: r.contains(180)`, exit 101.
  A textual diff review alone is tempting to approve here; the test suite is
  what exposes the defect.
- **Root cause / fix:** restore `bpm <= self.high`. The patch should be
  reverted to the inclusive baseline (`git checkout -- .`), which is exactly what
  was done — post-revert `cargo test` passes (1 passed) and `git status` is clean.

## Evidence

- `cargo test` with patch: **1 failed** (`tests::includes_both_boundaries`,
  `assertion failed: r.contains(180)`), exit 101.
- `cargo test` after revert: **1 passed**, exit 0; `git status --porcelain` clean.
- Patch saved to `t5_offbyone.patch`.

## Lesson recorded

A compile-clean patch is not a correct patch. Boundary conditions require the
existing behavioural tests (or new ones) to guard them. This is precisely the
class of defect a reviewer must catch that a compiler cannot.
