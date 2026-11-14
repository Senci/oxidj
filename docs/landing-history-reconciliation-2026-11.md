# Landing history reconciliation — 2026-11-14

## Resolved history

- Local parent: `9d52a9f3cefc9229a651dbf08fee4e2bb13151dc`.
- Fetched published parent: `d518055d28e7956c6f6bfe6c1024724c60a28b37`,
  the merge of [PR #26](https://github.com/Senci/oxidj/pull/26).
- Common ancestor: `84ab00f916ecd5d08eef8dca383ee4feaa7a587a`.
- PR #26 published the combined landing changes as
  `47a68c2f6adb997c739688dec80356f7a5d90f52`; local history retained the
  original implementation and fixes (`9bd35ef`, `40c480a`, `9d52a9f`).
  These are different commit histories with **identical final `site/` trees**,
  not competing final designs. The published PR's required `cargo xtask ci-full`
  check succeeded before its merge; that does not qualify unpublished changes.

A normal two-parent merge of published master into local master resolved without
conflicts. Before this record was added, the complete merged tree was identical
to the local parent (`git diff --cached --exit-code 9d52a9f`). No site file,
asset, dependency implementation, lockfile or toolchain setting was replaced.
This record is the only new content introduced by the reconciliation.

## Preservation and remaining gates

- Dependency migration `625ad95471bb9b732c7ce7fc737e0192fd4b0938`, its
  preceding SHA updates and reviewed toolchain changes remain in local ancestry.
  Preservation is **not** dependency approval. Card `t_bf244c1b` remains open:
  independent code review and standalone prototype license admission are still
  required. The documented BSL-1.0, OFL-1.1 and Ubuntu-font-1.0 refusals are not
  waived by a green root license check; latest-version exceptions also remain.
- Column card `t_764eab32` remains separate and blocked pending its verification
  and review. Its branch at `41364e3b82f5d50df73899bc8aab41177888f5a1`
  and worktree were not modified or integrated. Additional approved sorting and
  quick-filter scope remains unmet; this merge does not finish track-table work.
- Existing backup/side refs, other worktrees, untracked `.codex/`,
  `.debug-compositor/`, `scripts/__pycache__/`, and ignored artifacts are retained.
  No reset, clean, deletion, stash, force-push, catalog access or install was used.
  Tracked cleanliness must not be confused with deleting unknown local artifacts.
- This is a **local-only** history reconciliation. Independent review of this
  merge is required. Publishing its ancestry would also publish the unapproved
  dependency migration: do not push/deploy/install this head until its separate
  gates and applicable repository protection checks are satisfied. Prior PR #26
  checks do not transfer to this integration head.

## Reproduction

Run the site's frozen install, typecheck, lint, format check, unit tests and
production build/verifier, then `cargo xtask ci` on the final tree. Exact observed
results belong in the merge commit body and the `t_23180fe3` review handoff.
No visual redesign was made here; previous rendered evidence is not claimed as a
fresh browser audit, nor is upstream Wayland inclusion a runtime verification.
