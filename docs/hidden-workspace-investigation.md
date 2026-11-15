# Hidden-workspace ANR investigation

Task: `t_1fb1e40a`. Evidence collected 2026-11-15 on local master
`b02af2f`. **Investigation progress, not a demonstrated fix of the owner's
warning.** The warning was not reproduced in the isolated sessions below.
No owner process was attached to, stopped, or restarted; no owner catalog or
owner workspace was used. No installation was performed.

## Executable identity and hypothesis

The installed `oxidj-library-ui` is still the old executable:

- SHA-256: `96cbf7b42f4b9cdcb8c5219997d01deeee77750666fee7da1d2626035fa5d676`.
- ELF build ID: `5ebfb80e7a5aaf03efff2f991082e231117c77db`.
- Prior embedded dependency-path inspection identifies eframe 0.32.3.
  The package manifest's 0.1.0 is not an exact executable source revision;
  the installed source commit remains unknown.

A fresh `cargo build --locked --offline --manifest-path
prototypes/oxidj-library-ui/Cargo.toml` at `b02af2f` exited 0. Its debug executable
SHA-256 is `07fb416462418335577052dee0a7316cfefb1b43428b24f7978794f03c697f01`.
This build resolves eframe 0.37.0 with the default WGPU renderer.

[Upstream egui change #8631](https://github.com/emilk/egui/pull/8631)
addresses a matching Hyprland hidden-workspace failure: frame callbacks stop,
blocking a vsynchronized swap and preventing event-loop ping responses. Eframe
0.37 includes its Wayland pre-present/swap changes and overdue-redraw logic-only
fallback. These are upstream evidence and a plausible hypothesis, **not proof
that the owner's old process blocked there**. A poll-wait snapshot and little
catalog I/O do not distinguish swap blocking from event-loop idle or UI work.
The dependency migration is already on master; this investigation does not add
another speculative rendering workaround.

## Actual runtime checks

All app launches used explicit `--db <new synthetic.sqlite>`, a private working
directory, HOME/config/cache/data/runtime, and native Wayland. The synthetic
catalog was empty: no private music, playback, analysis, or import workload.

1. Sway 1.12 / wlroots 0.20.2, headless Pixman: both executables completed three
   workspace hide/restore cycles. A diagnostic compositor-only hook requested
   real `xdg_wm_base` pings on hidden-window resize, which the apps answered.
   **Neither version reproduced the warning.**
2. Hyprland 0.57.1 (`acc1aa0bfd4bd575dcb0f76c35ccf12b660a1ab9`),
   Aquamarine 0.16.0, nested under a private Weston 15.0.1 GL/headless parent:
   both executables completed actual workspace 1 → 2 → 1 transitions, with the
   app remaining on workspace 1, including resizes while hidden. Preliminary
   runs used three 15-second hidden intervals per executable.
3. The committed assertive regression below then ran separately against each
   executable in fresh sessions/catalogs:

   | Executable | Hidden intervals | Automatic ping/pong | Restore | Negative control |
   |---|---|---|---|---|
   | Fresh master, app PID 402885 / compositor PID 402794 | 3 × 12 s | 8/8 each interval | surface commit each time | stopped owned child rejected; recovery verified |
   | Installed old, app PID 404696 / compositor PID 404644 | 3 × 12 s | 8/8 each interval | surface commit each time | stopped owned child rejected; recovery verified |

   Both commands exited 0 and printed:

   ```text
   PASS: real hidden-workspace ping/restore regression (not original-cause proof)
   ```

The isolated Hyprland setup required a **parent-connection-only diagnostic
shim**, never loaded into OxiDJ. Aquamarine hard-codes registry bind versions
newer than the parent advertises (Weston compositor v5/seat v7); the disposable
compositor's parent connection was clamped to those advertised versions.
Source: Aquamarine `src/backend/Wayland.cpp`, registry-global binding callback.
Without it, startup fails on an invalid protocol version, before any OxiDJ test.
This setup difference and nested-versus-physical presentation limit the result;
it is not a native DRM/owner-session qualification. The shim and bootstrap are
retained with the task's diagnostic artifacts, not shipped as application code.

## Reusable regression

`scripts/verify-hidden-workspace.py` operates only against a **private Hyprland
owned by the same launcher**. It verifies the compositor's parent PID and Unix
IPC peer PID, requires an empty compositor on workspace 1, refuses an existing
output directory, and launches only its own app with new synthetic data. It
uses the current Hyprland Lua dispatch API; older dispatch syntax is not tested.
The runtime socket is explicit; inherited owner display/IPC/DB overrides are
removed. It never attaches to an existing app.

A Python launcher that has already started its private compositor invokes it
as a sibling subprocess (not through a shell that changes the parent identity):

```python
subprocess.run([
    "python3", "scripts/verify-hidden-workspace.py",
    "--binary", str(ui_binary.resolve()),
    "--compositor-pid", str(private_hyprland.pid),
    "--ipc", str(private_ipc_socket),
    "--runtime", str(private_runtime),
    "--display", private_wayland_socket_name,
    "--output", str(new_evidence_directory),
], check=True)
```

The launcher owns compositor creation/teardown and must not use the owner's
session. The script asserts actual workspace transitions, matched ping/pong
progress while hidden and surface commits after restore. It then SIGSTOPs
**only the child it spawned**, requires the same responsiveness assertion to
reject that child, resumes it and checks restored commits. `wayland.log` and
`result.json` record synthetic protocol evidence and executable identity.
The process-presence check alone cannot pass this regression. This opt-in
Linux/compositor test is not part of the portable Rust gate; syntax-only
checks do not count as runtime evidence.

## Remaining acceptance

- **PASS:** actual isolated hidden-workspace regression, restore, and a
  stopped-child negative control for both exact executables above.
- **NOT REPRODUCED:** the original Application Not Responding warning.
- **UNPROVEN:** original root cause and whether the already-merged upstream
  correction fixes that particular warning. Passing both old and new is not
  a differential fix result.
- **NOT RUN:** original active-workload reconstruction, physical-output
  reproduction, and post-review installation. Do not interrupt the running
  owner app to obtain them.

Next discriminating evidence is the active workload/view and timing associated
with the warning, or a synthetic failing case on an independently isolated
physical/virtual-display Hyprland session. Do not keep retrying identical empty
catalog hide/show cycles or add a speculative `vsync=false` workaround.
Independent review must retain this unresolved acceptance; reviewing this
regression is not approval to close the original root-cause claim.
