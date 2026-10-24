# ADR-001 isolated egui probe — evidence report

Status: **NOT PASSED / NOT QUALIFYING**. This probe is isolated; no GUI dependency was added to the root workspace or production manifests. No personal media was opened.

## Environment and build

- Host: Linux `viper`, x86_64, kernel `7.2.5-3-omarchy`; Wayland session, `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`.
- Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Probe: `prototypes/oxidj-gui-probe`, standalone Cargo workspace, eframe `=0.32.3`, explicit `accesskit` feature.
- Build/run command: `cargo run --offline --manifest-path prototypes/oxidj-gui-probe/Cargo.toml -- 200000` (500k: replace final argument with `500000`). GUI runs exited 0 after the probe closed its window. Hyprland reported the probe window title while active. Focused monitor was 3840×2160 at 30Hz.
- Rows are deterministic synthetic labels rendered using `ScrollArea::show_rows` with 24-pixel row height. Final 200k run: 453 update callbacks in 15,002ms = 30.2 callbacks/s. Final 500k run: 453 callbacks in 15,019ms = 30.2 callbacks/s. The display refresh was 30Hz, so this does not establish whether the app can reach 60 presented frames/s on a 60Hz display. No frame-presentation/compositor measurement was captured.

## Results and gates

| Gate | Result | Evidence / remaining work |
|---|---|---|
| 200k virtualized list | NOT PASSED | 15-second GUI run exited 0 and Hyprland listed the probe window. No visual scroll/interaction smoke was captured. |
| 500k rows at 60fps | NOT PASSED | GUI run exited 0; 30.2 update callbacks/s on a 30Hz display. No 60Hz display or frame-presentation measurement was available. |
| AccessKit tree | PASS (headless tree; live tree inspected) | `accesskit_update_contains_named_track_controls` checks row label `Track 000000 — Synthetic Artist — Synthetic Album`, role `Button`. The live AT-SPI bus exposed app root `oxidj-gui-probe`; its first content node had 65 child nodes during the 500k run. No screen reader was attached. |
| Table navigation/focus and keyboard multi-select | PASS (focused GUI probe) | Sent Down, Ctrl+Space, Shift+Down to the focused 200k probe window. Exit output: `keyboard_events=3 focused_row=2 selected_rows=[1, 2]`. State tests cover click toggle, range extension, Home/End bounds and empty-list behavior. |
| Drag-drop | PASS (egui pointer-event test) | `drag_source_released_over_another_row_records_drop` sends press, move and release events through the same row widget used by the virtualized list; result `(0, 1)`. No physical-pointer drag was captured. |
| Real screen-reader checks | NOT RUN | Linux Wayland is the only available session inspected. `orca` and AT-SPI Python bindings (`pyatspi`, `gi`) are unavailable; the AT-SPI Registry started on demand for tree inspection. No macOS or Windows device/session was available. Exact remaining gate: verify accessible roles, row/cell labels, focus, table navigation, keyboard multi-select and drag-drop with a real screen reader on Linux/Orca, macOS/VoiceOver, and Windows/NVDA on supported devices; record each actual device and outcome. |

This is isolated probe evidence, not evidence that ADR-001's production gates pass. 500k/60Hz presented-frame performance and real screen-reader/platform checks remain unqualified. Do not admit the framework to production from this report.
