#!/bin/sh
# Renders FreeCal's icons from the SVGs in src/ (see ICONS.md):
#   src/app.svg           -> app/<size>x<size>.png, listed in bundle.icon
#   src/tray-<name>.svg   -> tray/<name>.png, built into the binary
# Renders with `tauri icon` from @tauri-apps/cli, whose version package-lock.json
# pins, so the same SVG gives the same bytes on every machine.
# With --check, writes nothing and fails if a PNG differs from its SVG's render.
set -eu
cd "$(dirname "$0")"

APP_SIZES="32 48 64 128 256 512"
TRAY_SIZE=64

check=false
[ "${1:-}" = "--check" ] && check=true

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
mkdir "$out/app" "$out/tray"

render() { # svg size dest
  dir=$(mktemp -d -p "$out")
  npx --no-install tauri icon "$1" -o "$dir" --png "$2" >/dev/null 2>&1 ||
    { echo "Can't render $1" >&2; exit 1; }
  mv "$dir/${2}x${2}.png" "$3"
}

for size in $APP_SIZES; do
  render src/app.svg "$size" "$out/app/${size}x${size}.png"
done
for svg in src/tray-*.svg; do
  [ -e "$svg" ] || continue
  name=$(basename "$svg" .svg)
  render "$svg" "$TRAY_SIZE" "$out/tray/${name#tray-}.png"
done

if $check; then
  for dir in app tray; do
    if ! diff -r -x .gitkeep "$out/$dir" "$dir" >/dev/null; then
      echo "src-tauri/icons/$dir/ doesn't match src/; run src-tauri/icons/generate.sh" >&2
      exit 1
    fi
  done
else
  for dir in app tray; do
    find "$dir" -name '*.png' -delete
    cp "$out/$dir"/*.png "$dir"/ 2>/dev/null || true
  done
fi
