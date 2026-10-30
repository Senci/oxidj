# Pre-alpha 57: local-library empty state

Fresh read-only exact-window capture of the OxiDJ Local Library using only the synthetic `pre-alpha-56/empty.sqlite` catalog. The capture shows the catalog path, 0 tracks, the empty-library import guidance, and the unselected “Select a track to inspect its details.” hint. The actual displayed theme label is **Tokyo Night**, with its dark purple/blue palette; no theme control was clicked or changed.

The window was re-identified immediately before capture as `oxidj-library-ui` (PID 3749223, window 14680067). SQLite independently returned a track count of 0. See [`empty-state.png`](empty-state.png).

### Theme-control accessibility check (2026-10-30)

Rechecked the live synthetic empty-library app on the isolated Hermes Bot Screen. The running `oxidj-library-ui` PID `3749223`, X11 window `14680067` (`OxiDJ Local Library`), used the exact catalog `/home/senci/.hermes/profiles/oxidj-2/cache/scratch/pre-alpha-56/empty.sqlite`; SQLite reported `tracks` count `0`. The live screenshot showed the Tokyo Night label and dark palette. The process's `DBUS_SESSION_BUS_ADDRESS` resolved via `org.a11y.Bus.GetAddress` to `unix:path=/run/user/1000/at-spi/bus_0`; the AT-SPI desktop contained `oxidj-library-ui` with PID `3749223`.

The app's own session bus resolved to `unix:path=/run/user/1000/at-spi/bus_0`; the live app owner was `:1.287`. The theme combo was expanded once through its AT-SPI `click` action. Fresh traversal exposed named `Tokyo Night`, `Tokyo Day`, and `Custom` action buttons (frame siblings). Tokyo Day was re-resolved by name, its action verified as `click`, then invoked once; `Action.DoAction(0)` returned `(true,)`. A fresh exact-window capture showed the displayed **Tokyo Day** label, light palette, and the same synthetic empty catalog (0 tracks). The app-written `app.ron` contains `"ui_theme": "tokyo-day"`.

The genuine screenshot [`tokyo-day-pre-relaunch.png`](tokyo-day-pre-relaunch.png) was captured from the exact live app window (PID `3749223`, window `14680067`) on the Bot Screen with the selector open and is explicitly **pre-relaunch**. Same-XDG graceful close and relaunch, persistence restoration after relaunch, and a post-relaunch screenshot are **NOT RUN**: no supported normal-close action is available in this session. Do not infer graceful persistence from the current app.ron value.

**NOT DEMONSTRATED:** same-XDG relaunch restoration; production GUI or accessibility qualification.
