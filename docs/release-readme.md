# OxiDJ pre-release archives

These experimental archives contain the provisional library UI and CLI; they are not production-qualified. Back up your library and test on copies. Archives are unsigned; macOS builds are not notarized. Your operating system may warn or refuse to open them. No installer is included.

Unpack the archive to a writable local directory. Run `oxidj-cli --help` for the CLI. Run `oxidj-library-ui --db PATH` to open or create a catalog at an explicit local path; without `--db`, the UI uses `oxidj-library.sqlite` in its current working directory. The UI opens an audio output when available.

Linux requires a compatible x86-64 glibc system (built on Ubuntu 22.04), a graphical session and runtime audio/display libraries including ALSA, XKB/XKB-X11, EGL and OpenGL/Mesa. On Ubuntu 22.04 install `libasound2 libxkbcommon-x11-0 libegl1 libgl1-mesa-dri`; JACK is optional and requires its runtime when selected. Windows x86-64 and macOS Apple Silicon native builds are provisional, not platform-qualified. Linux-only storage admission currently refuses some library-import destinations on other platforms. Test interoperability, audio devices, accessibility and performance are not qualified by the build.

Verify the archive using the `SHA256SUMS` file attached to the same GitHub release; checksums detect corruption but are not signatures.
