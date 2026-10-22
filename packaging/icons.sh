#!/usr/bin/env bash
# Regenerate every app and tray icon from the BD mark in docs/brand/, rendered exactly as
# supplied. The mark is a BD trademark, NOT MIT/Apache: docs/brand/LICENSE-brand.txt.
#
# - App icon (Dock, taskbar, Start menu, Linux menus, installers): the mark in white on the
#   brand plate, from docs/brand/bd-mark-icon.svg.
# - Tray: the black mark as the macOS menu-bar template; the blue mark on Windows, Linux and BSD.
#
# Needs: resvg (brew install resvg / cargo install resvg). The .ico and .icns are packed by
# `cargo xtask ico` / `cargo xtask icns`, so this runs on any OS. The outputs are committed, so
# packaging (and the Rust build, which include_bytes!s some of them) never needs these tools.
#
#   packaging/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/assets/app-icon"
BRAND="$ROOT/docs/brand"
ICON="$BRAND/bd-mark-icon.svg"
ID="com.bdtecnologia.bd-launcher"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# The icon is already a square canvas with the mark centred on Apple's icon grid, so the same
# artwork goes to every platform.
render() { resvg -w "$2" -h "$2" "$1" "$3" </dev/null; }

# The Linux icon theme.
for s in 16 24 32 48 64 128 256 512; do
  mkdir -p "$DIR/hicolor/${s}x${s}/apps"
  render "$ICON" "$s" "$DIR/hicolor/${s}x${s}/apps/$ID.png"
done
mkdir -p "$DIR/hicolor/scalable/apps"
cp "$ICON" "$DIR/hicolor/scalable/apps/$ID.svg"
cp "$ICON" "$DIR/bd-launcher.svg"

# Windows .ico.
ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  render "$ICON" "$s" "$TMP/ico-$s.png"
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/bd-launcher.ico" "${ICO_PNGS[@]}")

# macOS .icns.
ICNS_PNGS=()
for s in 16 32 64 128 256 512 1024; do
  render "$ICON" "$s" "$TMP/icns-$s.png"
  ICNS_PNGS+=("$TMP/icns-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- icns "$DIR/bd-launcher.icns" "${ICNS_PNGS[@]}")

# The 1024 px master, used by the macOS bundle and the Linux bundles.
render "$ICON" 1024 "$DIR/bd-launcher-1024.png"

# Tray icons: the black mark is the macOS menu-bar template (44 px = 22 pt @2x; macOS tints it);
# the blue one is for the Windows notification area and Linux/BSD status notifiers.
render "$BRAND/bd-mark-black.svg" 44 "$BRAND/bd-mark-black-44.png"
render "$BRAND/bd-mark-blue.svg" 64 "$BRAND/bd-mark-64.png"
echo "icons written to $DIR"
