# Pre-alpha 57: local-library empty state

Fresh read-only exact-window capture of the OxiDJ Local Library using only the synthetic `pre-alpha-56/empty.sqlite` catalog. The capture shows the catalog path, 0 tracks, the empty-library import guidance, and the unselected “Select a track to inspect its details.” hint. The actual displayed theme label is **Tokyo Night**, with its dark purple/blue palette; no theme control was clicked or changed.

The window was re-identified immediately before capture as `oxidj-library-ui` (PID 3749223, window 14680067). SQLite independently returned a track count of 0. See [`empty-state.png`](empty-state.png).

### Theme-control accessibility check (2026-10-30)

Rechecked the live synthetic empty-library app on the isolated Hermes Bot Screen. The running `oxidj-library-ui` PID `3749223`, X11 window `14680067` (`OxiDJ Local Library`), used the exact catalog `/home/senci/.hermes/profiles/oxidj-2/cache/scratch/pre-alpha-56/empty.sqlite`; SQLite reported `tracks` count `0`. The live screenshot showed the Tokyo Night label and dark palette. The process's `DBUS_SESSION_BUS_ADDRESS` resolved via `org.a11y.Bus.GetAddress` to `unix:path=/run/user/1000/at-spi/bus_0`; the AT-SPI desktop contained `oxidj-library-ui` with PID `3749223`.

The app's live tree exposed a `Theme (Ctrl+Alt+1/2/3)` label and an unnamed `combo box`. That combo box had no children/options and only a generic `click` action; the tree did not expose a named Theme ComboBox with Tokyo Night/Day/Custom options. Therefore AT-SPI selection was **NOT RUN**; no action was invoked, and no app configuration or theme state was changed. Tokyo Day app-written `ui_theme` persistence and same-XDG relaunch restoration were **NOT RUN**. No Tokyo Day post-relaunch screenshot was captured. The AX capture surfaced an AT-SPI cache warning (`Unknown object '/org/a11y/atspi/cache'`), but the application and its 19-child frame were enumerated; the blocking evidence is specifically the unnamed combo and absent options, not failure to find the app.

**NOT DEMONSTRATED:** Tokyo Day selection or persistence; production GUI qualification; production accessibility qualification.
