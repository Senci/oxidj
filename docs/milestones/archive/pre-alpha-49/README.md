# Pre-alpha 49 — fail-closed position-mark End boundary

The synthetic F-07 CLI preview regression confirms present-empty
`POSITION_MARK End` is rejected with `invalid position mark End value`. Existing
preview and no-edit tests keep absent End distinct from raw `End="0"`.

The two PNGs are screenshot-style milestone-card renders from adjacent SVG
sources, not captures of Rekordbox or a production OxiDJ UI. No private library
data was used.

This documents a fail-closed parser boundary, not vendor compatibility. Global
F-09, Rekordbox/OneLibrary compatibility, and ADR-005 qualification remain NOT
DEMONSTRATED.
