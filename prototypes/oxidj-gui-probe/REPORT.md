# ADR-001 isolated egui probe — evidence report

Status: **NOT PASSED / NOT QUALIFYING**. This probe is isolated; no GUI dependency was added to the root workspace or production manifests. No personal media was opened.

## Environment and build

- Host: Linux `viper`, x86_64, kernel `7.2.5-3-omarchy`; Wayland session, `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`.
- Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Probe: `prototypes/oxidj-gui-probe`, standalone Cargo workspace, eframe `=0.32.3`, explicit `accesskit` feature.
- Build/run command: `cargo run --manifest-path prototypes/oxidj-gui-probe/Cargo.toml -- 200000` (500k: replace final argument with `500000`). First invocation exceeded the tool's 420-second ceiling during dependency build; the follow-up offline invocation reported `Finished dev profile` and launched the GUI. It was then terminated by the 90-second foreground timeout (124), so successful interactive completion is not established.
- Rows are deterministic synthetic labels rendered using `ScrollArea::show_rows` with 24-pixel row height. Frame rate displayed is update-callback count divided by monotonic wall-clock elapsed time; this includes time outside active rendering and is not a benchmark of presented/compositor frames.

## Results and gates

| Gate | Result | Evidence / remaining work |
|---|---|---|
| 200k virtualized list | NOT PASSED | Probe source and run command exist; no verified visual/interactivity result captured. |
| 500k rows at 60fps | NOT PASSED | Not run to completion; no measured FPS result. UI callback estimate alone would not prove displayed 60fps. |
| AccessKit tree | NOT PASSED | Feature enabled in dependency; no tree dump/inspection performed. |
| Table navigation/focus, keyboard multi-select, drag-drop | NOT PASSED | Not implemented or exercised in this probe. |
| Real screen-reader checks | NOT RUN | Linux Wayland is the only available session inspected. `orca`, AT-SPI Python bindings (`pyatspi`, `gi`) were not found. No macOS or Windows device/session was available. Exact remaining gate: verify accessible roles, row/cell labels, focus, table navigation, keyboard multi-select and drag-drop with a real screen reader on Linux/Orca, macOS/VoiceOver, and Windows/NVDA on supported devices; record each actual device and outcome. |

This is a scaffold/evidence of dependency isolation and the intended measurement method, not evidence that ADR-001's production gates pass. Do not admit the framework to production from this report.
