#!/usr/bin/env python3
"""Opt-in native-Wayland regression; requires a launcher-owned private Hyprland.

The compositor and this script MUST be sibling children of the same launcher.
No inherited desktop socket, catalog, or display is used. See
 docs/hidden-workspace-investigation.md for evidence and limitations.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import socket
import struct
import subprocess
import time


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def protocol(text, operation):
    return re.findall(r"xdg_wm_base#\d+\." + operation + r"\((\d+)\)", text)


def child_parent(pid):
    # Linux /proc stat: comm can contain spaces and closing parentheses.
    return int(Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[1])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--compositor-pid", type=int, required=True)
    parser.add_argument("--ipc", type=Path, required=True)
    parser.add_argument("--runtime", type=Path, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    runtime = args.runtime.resolve(strict=True)
    ipc = args.ipc.resolve(strict=True)
    require(ipc.is_relative_to(runtime), "IPC must belong to the supplied private runtime")
    require("/" not in args.display, "display must be a socket name, not an external path")
    require((runtime / args.display).is_socket(), "private Wayland socket missing")
    require(child_parent(args.compositor_pid) == os.getppid(),
            "refusing compositor not owned by this script's launcher")
    output = args.output.resolve()
    output.mkdir(mode=0o700)  # Refuse reuse: every run gets a new catalog and state.
    for name in ("home", "config", "cache", "data"):
        (output / name).mkdir(mode=0o700)

    ipc_fd = os.open(ipc.parent, os.O_RDONLY | os.O_DIRECTORY)
    runtime_fd = os.open(runtime, os.O_RDONLY | os.O_DIRECTORY)
    app = None
    try:
        def command(message):
            with socket.socket(socket.AF_UNIX) as connection:
                connection.settimeout(5)
                connection.connect(f"/proc/self/fd/{ipc_fd}/{ipc.name}")
                peer_pid, _, _ = struct.unpack("3i", connection.getsockopt(
                    socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
                require(peer_pid == args.compositor_pid, "IPC peer is not the owned compositor")
                connection.sendall(message.encode())
                chunks = []
                while chunk := connection.recv(65536):
                    chunks.append(chunk)
                return b"".join(chunks).decode()

        require(json.loads(command("j/activeworkspace"))["id"] == 1,
                "private compositor must start on workspace 1")
        require(json.loads(command("j/clients")) == [],
                "private compositor must have no existing app windows")
        env = os.environ.copy()
        for key in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "SWAYSOCK",
                    "HYPRLAND_INSTANCE_SIGNATURE", "DBUS_SESSION_BUS_ADDRESS",
                    "LD_PRELOAD", "OXIDJ_DB"):
            env.pop(key, None)
        env.update(HOME=str(output / "home"), XDG_CONFIG_HOME=str(output / "config"),
                   XDG_CACHE_HOME=str(output / "cache"), XDG_DATA_HOME=str(output / "data"),
                   XDG_RUNTIME_DIR=f"/proc/self/fd/{runtime_fd}",
                   WAYLAND_DISPLAY=args.display, XDG_SESSION_TYPE="wayland",
                   WINIT_UNIX_BACKEND="wayland", WAYLAND_DEBUG="client")
        log_path = output / "wayland.log"
        result = {"binary_sha256": hashlib.file_digest(binary.open("rb"), "sha256").hexdigest(),
                  "compositor_pid": args.compositor_pid, "cycles": []}
        with log_path.open("w") as log:
            app = subprocess.Popen([str(binary), "--db", str(output / "synthetic.sqlite")],
                                   cwd=output, env=env, pass_fds=(runtime_fd,),
                                   stdout=log, stderr=log)
            result["app_pid"] = app.pid
            deadline = time.monotonic() + 30
            clients = []
            while time.monotonic() < deadline:
                require(app.poll() is None, "probe app exited before mapping")
                clients = [c for c in json.loads(command("j/clients")) if c["pid"] == app.pid]
                if clients:
                    break
                time.sleep(0.1)
            require(clients and clients[0]["workspace"]["id"] == 1,
                    "probe window did not map on workspace 1")
            require(not clients[0]["xwayland"], "probe must use native Wayland")
            address = clients[0]["address"]

            def dispatch(expression):
                require(command("dispatch " + expression).strip() == "ok", "dispatch failed")

            def log_since(offset):
                with log_path.open() as source:
                    source.seek(offset)
                    return source.read()

            def hide():
                dispatch("hl.dsp.focus({workspace=2})")
                require(json.loads(command("j/activeworkspace"))["id"] == 2,
                        "workspace switch not observed")
                own = [c for c in json.loads(command("j/clients")) if c["pid"] == app.pid]
                require(len(own) == 1 and own[0]["workspace"]["id"] == 1,
                        "app did not remain on the hidden workspace")
                time.sleep(1)  # Let workspace transition finish before measuring.

            def responsive_hidden():
                offset = log_path.stat().st_size
                time.sleep(12)
                require(app.poll() is None, "probe app exited while hidden")
                text = log_since(offset)
                pings, pongs = protocol(text, "ping"), protocol(text, "pong")
                require(len(pings) >= 3 and pings == pongs,
                        f"hidden ping progress failed: ping={len(pings)}, pong={len(pongs)}")
                return {"pings": len(pings), "pongs": len(pongs)}

            def restore():
                offset = log_path.stat().st_size
                dispatch("hl.dsp.focus({workspace=1})")
                require(json.loads(command("j/activeworkspace"))["id"] == 1,
                        "workspace restore not observed")
                time.sleep(2)
                text = log_since(offset)
                require(re.search(r" -> wl_surface#\d+\.commit\(", text),
                        "no surface commit after restore")

            dispatch(f'hl.dsp.window.float({{action="on", window="address:{address}"}})')
            time.sleep(2)
            for cycle in range(3):
                hide()
                dispatch('hl.dsp.window.resize({' + f'x={900 + cycle * 20}, y={600 + cycle * 20}, '
                         + f'window="address:{address}"' + '})')
                sample = responsive_hidden()
                restore()
                result["cycles"].append(sample)
                print(f"cycle {cycle + 1}: hidden {sample}; restored surface commit", flush=True)

            # A live PID alone must not satisfy the monitor. Stop only our own child.
            hide()
            app.send_signal(signal.SIGSTOP)
            try:
                try:
                    responsive_hidden()
                except RuntimeError as error:
                    require("hidden ping progress failed" in str(error),
                            "negative control failed for an unrelated reason")
                    result["stopped_child_detected"] = True
                else:
                    raise RuntimeError("monitor falsely accepted a stopped app")
            finally:
                app.send_signal(signal.SIGCONT)
            time.sleep(2)
            restore()
            print("negative control: stopped owned child rejected; restored after SIGCONT", flush=True)
        (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print("PASS: real hidden-workspace ping/restore regression (not original-cause proof)")
    finally:
        if app is not None and app.poll() is None:
            app.send_signal(signal.SIGCONT)
            app.terminate()
            try:
                app.wait(timeout=10)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()
        os.close(runtime_fd)
        os.close(ipc_fd)


if __name__ == "__main__":
    main()
