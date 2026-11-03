# Pre-alpha 01 milestone screenshots

This archive contains four genuine captures from two separate OxiDJ milestones.
No personal library data, audio, or source identifiers are included.

## M3 local-library UI smoke — 2026-10-27

Captured from the live OxiDJ Local Library app on isolated Bot Screen `:20`;
the populated view used only the repository's synthetic `tone.flac` fixture.

- [01-local-library-startup.png](01-local-library-startup.png) — one synthetic
  track appears with duration, format facts, and Available status. Proves this
  rendered persisted-catalog startup view; does not prove production admission,
  accessibility, platform support, or performance qualification.
- [02-empty-library.png](02-empty-library.png) — empty catalog renders the
  import prompt. Proves this rendered empty state; does not prove import behavior
  or any production qualification gate.

## ADR-001 isolated GUI-probe navigation — 2026-10-25

Captured from the live synthetic 200,000-row `oxidj-gui-probe` on the isolated
Hermes Bot Screen (`DISPLAY=:20`, 1440×900). The probe uses deterministic
synthetic track labels; no personal media was opened. These are visual readbacks
of navigation boundary states, not a performance or production qualification.

- [03-gui-probe-home-boundary.png](03-gui-probe-home-boundary.png) — Home
  boundary shows focus at row 0 and the viewport beginning at row 0. Proves the
  captured rendered boundary state only; does not establish production GUI
  admission, accessibility/screen-reader behavior, platform support, or FPS.
- [04-gui-probe-end-boundary.png](04-gui-probe-end-boundary.png) — End
  boundary shows focus at the final row (199,999) with the final viewport rows
  visible. Proves the captured rendered boundary state only; does not establish
  production GUI admission, accessibility/screen-reader behavior, platform
  support, or FPS.

These screenshots are visual milestone records, not Rekordbox/OneLibrary
compatibility evidence. M4 vendor behavior remains synthetic-only and
unqualified; ADR-001 and deferred performance qualification gates remain open.
