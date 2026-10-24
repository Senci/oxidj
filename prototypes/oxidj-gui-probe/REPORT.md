# ADR-001 isolated egui probe — evidence report

Status: **NOT PASSED / NOT QUALIFYING**. This probe is isolated; no GUI dependency was added to the root workspace or production manifests. No personal media was opened.

## Environment and build

- Host: Linux `viper`, x86_64, kernel `7.2.5-3-omarchy`; Wayland session, `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-1`.
- Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`.
- Probe: `prototypes/oxidj-gui-probe`, standalone Cargo workspace, eframe `=0.32.3`, explicit `accesskit` feature.
- Build/run command: `cargo run --offline --manifest-path prototypes/oxidj-gui-probe/Cargo.toml -- 200000` (500k: replace final argument with `500000`). Both runs exited 0 after the probe closed its window at 15 seconds. Hyprland reported the probe window title while the 200k run was active. Focused monitor was 3840×2160 at 30Hz.
- Rows are deterministic synthetic labels rendered using `ScrollArea::show_rows` with 24-pixel row height. Final 200k run: 453 update callbacks in 15,007ms = 30.2 callbacks/s. Final 500k run: 454 callbacks in 15,029ms = 30.2 callbacks/s. The display refresh was 30Hz, so this does not establish whether the app can reach 60 presented frames/s on a 60Hz display. No frame-presentation/compositor measurement was captured.

## Results and gates

| Gate | Result | Evidence / remaining work |
|---|---|---|
| 200k virtualized list | NOT PASSED | 15-second GUI run exited 0 and Hyprland listed the probe window. No visual scroll/interaction smoke was captured. |
| 500k rows at 60fps | NOT PASSED | GUI run exited 0; 30.2 update callbacks/s on a 30Hz display. No 60Hz display or frame-presentation measurement was available. |
| AccessKit tree | NOT PASSED | Cargo feature is enabled; during the visible probe run, the session exposed `org.a11y.Bus` but no `org.a11y.atspi.Registry` service or accessible tree dump. |
| Table navigation/focus, keyboard multi-select, drag-drop | NOT PASSED | Not implemented or exercised in this probe. |
| Real screen-reader checks | NOT RUN | Linux Wayland is the only available session inspected. `orca`, AT-SPI Python bindings (`pyatspi`, `gi`), and the AT-SPI Registry were unavailable. No macOS or Windows device/session was available. Exact remaining gate: verify accessible roles, row/cell labels, focus, table navigation, keyboard multi-select and drag-drop with a real screen reader on Linux/Orca, macOS/VoiceOver, and Windows/NVDA on supported devices; record each actual device and outcome. |

This is a scaffold/evidence of dependency isolation and the intended measurement method, not evidence that ADR-001's production gates pass. Do not admit the framework to production from this report.
