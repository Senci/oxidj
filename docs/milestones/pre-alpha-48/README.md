# Pre-alpha 48 — POSITION_MARK End optionality on no-edit export

The CLI regression derives a synthetic XML case from the frozen F-07 two-mark
fixture. The first mark has no `End`; the second retains raw `End="0"`. Export
must preserve both states byte-for-byte and leave the input unchanged.

Screenshot-style milestone-card renders: `01-end-optionality.png` and
`02-scope-and-proof.png`; `.svg` sources are kept beside them. They are not
captures of Rekordbox or a production OxiDJ UI.

This is synthetic F-07 preservation evidence only. It does not establish
Rekordbox/OneLibrary compatibility, global F-09, or ADR-005 qualification.
No private library data was used.
