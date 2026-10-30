# Pre-alpha 57: local-library empty state

Fresh read-only exact-window capture of the OxiDJ Local Library using only the synthetic `pre-alpha-56/empty.sqlite` catalog. The capture shows the catalog path, 0 tracks, the empty-library import guidance, and the unselected “Select a track to inspect its details.” hint. The actual displayed theme label is **Tokyo Night**, with its dark purple/blue palette; no theme control was clicked or changed.

The window was re-identified immediately before capture as `oxidj-library-ui` (PID 3749223, window 14680067). SQLite independently returned a track count of 0. See [`empty-state.png`](empty-state.png).

**NOT DEMONSTRATED:** Tokyo Day selection or persistence; production GUI qualification; production accessibility qualification.