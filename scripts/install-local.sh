#!/usr/bin/env bash
# Build OxiDJ in release mode and install it for the current user (no sudo).
#   scripts/install-local.sh            # install to ~/.local
#   PREFIX=/opt/oxidj scripts/install-local.sh
# Installs: oxidj-cli (CLI), oxidj-library-ui (egui app), oxidj-library (launcher that
# opens the default library), a desktop entry and an icon. Safe to re-run; it only
# overwrites those files and never touches your library data.
set -euo pipefail
cd "$(dirname "$0")/.."
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"; APPS="$PREFIX/share/applications"; ICONS="$PREFIX/share/icons/hicolor/scalable/apps"

cargo build --release --locked -p oxidj-cli
cargo build --release --locked --manifest-path prototypes/oxidj-library-ui/Cargo.toml

mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 0755 target/release/oxidj-cli "$BIN/oxidj-cli"
install -m 0755 prototypes/oxidj-library-ui/target/release/oxidj-library-ui "$BIN/oxidj-library-ui"
install -m 0644 assets/oxidj.svg "$ICONS/oxidj.svg"

# Launcher: default library lives in ~/.local/share/oxidj (override with OXIDJ_DB or --db).
cat >"$BIN/oxidj-library" <<'EOF'
#!/bin/sh
# Opens the OxiDJ library UI on the default (or $OXIDJ_DB) catalog; creates it when missing.
db="${OXIDJ_DB:-${XDG_DATA_HOME:-$HOME/.local/share}/oxidj/library.sqlite}"
if [ "${1:-}" = "--db" ]; then exec oxidj-library-ui "$@"; fi
mkdir -p "$(dirname "$db")"
exec oxidj-library-ui --db "$db" "$@"
EOF
chmod 0755 "$BIN/oxidj-library"

cat >"$APPS/oxidj-library.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OxiDJ
GenericName=DJ Library
Comment=Local-first DJ library: waveforms, beat grids, Rekordbox XML
Exec=$BIN/oxidj-library
Icon=oxidj
Terminal=false
Categories=AudioVideo;Audio;Music;
Keywords=dj;rekordbox;library;waveform;bpm;
EOF
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
echo "Installed: $BIN/oxidj-cli, $BIN/oxidj-library-ui, $BIN/oxidj-library"
echo "Commit: $(git rev-parse --short HEAD) ($(git status --porcelain --untracked-files=no | wc -l) uncommitted tracked changes)"
