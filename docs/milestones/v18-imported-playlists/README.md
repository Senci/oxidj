# v0.18 — imported playlists

**Date:** 2026-11-07
**Source:** approved UI in commit `8068744`
**Capture environment:** isolated OxiDJ Bot Screen, 1440×900, Tokyo Night; installed `oxidj-library-ui` release. The installed app was launched with an isolated synthetic catalog, not the owner's default catalog.

`01-imported-playlist-load-controls.png` is a crop of a fresh, actual app-window capture (crop coordinates `(0, 198, 175, 450)` in the 1440×791 app capture). It shows the expanded nested playlist, selected `Inner First`, selected stored-order member `Song C`, and explicit `Load A` / `Load B` buttons. All visible catalog labels are synthetic repository-test data; no personal library, audio collection, device, or vendor data was used. Pixels were inspected before publication.

This is visual evidence only, not vendor compatibility, production GUI/accessibility/platform qualification, or hardware qualification. Selection is separate from loading; accepted behavior and regression evidence are in `plan/STATUS.md` M8 closeout and the `8068744` code/tests.
