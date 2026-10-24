# ADR-001 isolated egui probe — evidence report

Status: **NOT PASSED / NOT QUALIFYING**. This probe is isolated; no GUI dependency was added to the root workspace or production manifests. No personal media was opened.

## Environment and build

- Host: Linux `viper`, x86_64, kernel `7.2.5-3-omarchy`; Wayland session, `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`.
- Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Probe: `prototypes/oxidj-gui-probe`, standalone Cargo workspace, eframe `=0.32.3`, explicit `accesskit` feature.
- Build/run command: `cargo run --offline --manifest-path prototypes/oxidj-gui-probe/Cargo.toml -- 200000` (500k: replace final argument with `500000`). GUI runs exited 0 after the probe closed its window. Hyprland reported the probe window title while active in earlier runs. Focused monitor was 3840×2160 at 30Hz.
- Rows are deterministic synthetic labels rendered using `ScrollArea::show_rows` with 24-pixel row height. Latest 200k run exited 0 and reported 453 update callbacks / 15,026ms = 30.1 callback/s, `keyboard_events=0`, `scroll_offset=0.0`; this was stationary and produced no scroll workload. Prior live runs' keyboard results are separately reported below. No presented-frame measurement was captured. The desktop capture route failed with X11 `GetImage` error 8 despite the session being Wayland, and `computer_use list_windows` returned no windows; therefore no trustworthy per-presented-frame sampling or live scroll run could be obtained through the available capture tool. Hyprland reported active mode 3840×2160@30Hz; 1920×1080@60Hz and 2560×1440@59.95Hz are available but were not activated. 60fps performance remains unmeasured.

## Results and gates

| Gate | Result | Evidence / remaining work |
|---|---|---|
| 200k virtualized list | NOT PASSED | Focused live GUI accepted 12 of 80 injected Down events and logged 30.2 update callbacks/s during the 15-second run. This is not a continuous-scroll or presented-frame measurement; no visual capture was taken. |
| 500k rows at 60fps | NOT RUN / NOT QUALIFIED | Earlier run had 30.2 update callbacks/s while stationary at 30Hz. No 60Hz mode was activated, no scrolling workload was applied, and no compositor/presented-frame measurement exists. Available 60Hz modes are not proof of measured performance. |
| AccessKit tree | PASS (headless tree; live tree inspected) | `accesskit_update_contains_named_track_controls` checks row label `Track 000000 — Synthetic Artist — Synthetic Album`, role `Button`. The live AT-SPI bus exposed app root `oxidj-gui-probe`; its first content node had 65 child nodes during the 500k run. No screen reader was attached. |
| Table navigation/focus and keyboard multi-select | FAIL / NOT PASSED | Focused live run sent `End`, `Ctrl+Space`, `Shift+Down`; output: `keyboard_events=2 focused_row=199999 widget_focused_row=None scroll_offset=5398321.0 selected_rows=[]`. A second run sent 80 Down keys; output: 12 received, `focused_row=12 widget_focused_row=None scroll_offset=324.0 selected_rows=[12]`. The logged `widget_focused_row=None` from these runs is not valid evidence about actual focus: the probe reset/logged the field before rendering, so the instrumentation guaranteed `None`. A new headless update/render regression test verifies that the corrected update path records the currently focused off-screen row (`Some(199)`) after rendering. No corrected live run has established native widget/AccessKit focus or keyboard multi-select; those gates remain NOT PASSED. State tests cover intended transitions, not these failed native interactions. |
| Drag-drop | PASS (egui pointer-event test only) | `drag_source_released_over_another_row_records_drop` sends press, move and release events through the same row widget used by the virtualized list; result `(0, 1)`. No physical-pointer drag was captured. |
| Real screen-reader checks | NOT RUN | Linux Wayland is the only available session inspected. `orca` and AT-SPI Python bindings (`pyatspi`, `gi`) are unavailable; the AT-SPI Registry started on demand for tree inspection. No macOS or Windows device/session was available. Exact remaining gate: verify accessible roles, row/cell labels, focus, table navigation, keyboard multi-select and drag-drop with a real screen reader on Linux/Orca, macOS/VoiceOver, and Windows/NVDA on supported devices; record each actual device and outcome. |

This is isolated probe evidence, not evidence that ADR-001's production gates pass. 500k/60Hz presented-frame performance and real screen-reader/platform checks remain unqualified. Do not admit the framework to production from this report.
