#!/bin/sh
# Renders FreeCal's icons from the SVGs in src/ (see ICONS.md):
#   src/app.svg           -> app/<size>x<size>.png, listed in bundle.icon
#   src/tray-base.svg + src/mark-<status>.svg
#                         -> tray/<status>.png, built into the binary: the
#                            Status Mark on the base's lower right, with a
#                            transparent gap around it
# Renders with `tauri icon` from @tauri-apps/cli, whose version package-lock.json
# pins, so the same SVG gives the same bytes on every machine.
# With --check, writes nothing and fails if a PNG differs from its SVG's render.
set -eu
cd "$(dirname "$0")"

APP_SIZES="32 48 64 128 256 512"
TRAY_SIZE=64
# The Status Mark's side and the gap around it, as fractions of the tray icon.
MARK_FRACTION=0.40
GAP_FRACTION=0.045

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
# The first number in an SVG root's attribute, e.g. width="512" -> 512.
attr() { # svg name
  tr '\n' ' ' <"$1" | sed -n "s/^.*<svg[^>]*[[:space:]]$2=\"\([0-9.]*\).*$/\1/p"
}

# Writes the tray icon for one Status Mark: the base in a canvas of its own
# width, then the mark on the lower right. A mask cuts the gap out of the base:
# the mark's shape, widened by feMorphology, hides the base under it.
compose() { # base mark
  canvas=$(attr "$1" width)
  mark_width=$(attr "$2" width)
  [ -n "$canvas" ] && [ -n "$mark_width" ] ||
    { echo "$1 and $2 need a plain width=\"<number>\" on <svg>" >&2; exit 1; }
  awk -v c="$canvas" -v w="$mark_width" -v m="$MARK_FRACTION" -v g="$GAP_FRACTION" 'BEGIN {
    printf "%s %s %s\n", c * (1 - m), c * m / w, c * g
  }' | {
    read -r offset scale gap
    echo "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"$canvas\" height=\"$canvas\" viewBox=\"0 0 $canvas $canvas\">"
    echo "<defs><filter id=\"status-mark-gap\"><feMorphology in=\"SourceAlpha\" operator=\"dilate\" radius=\"$gap\"/></filter>"
    echo "<mask id=\"status-mark-cut\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"$canvas\" height=\"$canvas\">"
    echo "<rect width=\"$canvas\" height=\"$canvas\" fill=\"#fff\"/><use xlink:href=\"#status-mark\" filter=\"url(#status-mark-gap)\"/></mask></defs>"
    echo "<g mask=\"url(#status-mark-cut)\">"
    sed '/^<?xml/d' "$1"
    echo "</g><g id=\"status-mark\" transform=\"translate($offset $offset) scale($scale)\">"
    sed '/^<?xml/d' "$2"
    echo "</g></svg>"
  }
}

for mark in src/mark-*.svg; do
  [ -e "$mark" ] || continue
  status=$(basename "$mark" .svg)
  status=${status#mark-}
  compose src/tray-base.svg "$mark" >"$out/tray-$status.svg"
  render "$out/tray-$status.svg" "$TRAY_SIZE" "$out/tray/$status.png"
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
