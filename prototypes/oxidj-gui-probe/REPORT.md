# ADR-001 isolated egui probe — evidence report

Status: **NOT PASSED / NOT QUALIFYING**. This probe is isolated; no GUI dependency was added to the root workspace or production manifests. No personal media was opened.

## Environment and build

- Host: Linux `viper`, x86_64, kernel `7.2.5-3-omarchy`; Wayland session, `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`.
- Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Probe: `prototypes/oxidj-gui-probe`, standalone Cargo workspace, eframe `=0.32.3`, explicit `accesskit` feature.
- Build/run command: `WAYLAND_DISPLAY=wayland-2 cargo run --offline --manifest-path prototypes/oxidj-gui-probe/Cargo.toml -- 200000` (500k: replace final argument with `500000`). The probe reports after 15 seconds and leaves its window open. Rows are deterministic synthetic labels rendered using `ScrollArea::show_rows` with 24-pixel row height. In the separate nested Wayland session, 200k reported 341 update callbacks / 16,072ms = 21.2 callback/s after one End key; 500k reported 454 / 15,010ms = 30.2 callback/s after one End key. Both runs reached the last row, reported matching widget focus and nonzero scroll offsets. These are callback counts around a single navigation event, not sustained-scroll workload or presented-frame measurements. A screenshot showed the 500k focused row visibly highlighted; the 200k window remained open beside it. The earlier probe was not reopened after its auto-close; the user later authorized this separate nested Wayland session. Host workspace and display mode were untouched. The host Hyprland mode remains 3840×2160@30Hz; 1920×1080@60Hz and 2560×1440@59.95Hz are available but were not activated. 60fps performance remains unmeasured.

## Results and gates

| Gate | Result | Evidence / remaining work |
|---|---|---|
| 200k virtualized list | NOT PASSED | One live End event reached row 199999, `widget_focused_row=Some(199999)`, `scroll_offset=5399259.0`; 341 update callbacks / 16,072ms = 21.2 callback/s. No sustained scroll or presented-frame measurement. |
| 500k rows at 60fps | NOT PASSED / NOT QUALIFIED | One live End event reached row 499999, `widget_focused_row=Some(499999)`, `scroll_offset=13499678.0`; 454 update callbacks / 15,010ms = 30.2 callback/s. Screenshot showed last row visibly focused. No sustained scroll, 60Hz mode, or compositor/presented-frame measurement. Available host 60Hz modes are not proof of measured performance. |
| AccessKit tree | PASS (headless tree; live tree inspected) | `accesskit_update_contains_named_track_controls` checks row label `Track 000000 — Synthetic Artist — Synthetic Album`, role `Button`. The live AT-SPI bus exposed app root `oxidj-gui-probe`; its first content node had 65 child nodes during the 500k run. No screen reader was attached. |
| Table navigation/focus and keyboard multi-select | PARTIAL / NOT PASSED | Corrected live End events reached row 199999 and 499999 with matching widget focus and nonzero scroll; 500k screenshot showed focused row. Ordinary arrows, Shift range-selection, and Ctrl/Command toggling remain unverified live. Headless tests cover these behaviors but do not establish live interaction. Historical pre-fix `None` telemetry was an instrumentation artifact. |
| Drag-drop | PASS (egui pointer-event test only) | `drag_source_released_over_another_row_records_drop` sends press, move and release events through the same row widget used by the virtualized list; result `(0, 1)`. No physical-pointer drag was captured. |
| Real screen-reader checks | NOT RUN | Linux Wayland is the only available session inspected. `orca` and AT-SPI Python bindings (`pyatspi`, `gi`) are unavailable; the AT-SPI Registry started on demand for tree inspection. No macOS or Windows device/session was available. Exact remaining gate: verify accessible roles, row/cell labels, focus, table navigation, keyboard multi-select and drag-drop with a real screen reader on Linux/Orca, macOS/VoiceOver, and Windows/NVDA on supported devices; record each actual device and outcome. |

This is isolated probe evidence, not evidence that ADR-001's production gates pass. Sustained-scroll/presented-frame performance and real screen-reader/platform checks remain unqualified. Do not admit the framework to production from this report.
