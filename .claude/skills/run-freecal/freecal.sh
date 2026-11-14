#!/usr/bin/env bash
# Runs the FreeCal debug build on a private Xvfb display and drives it.
# Run from the repo root. See SKILL.md next to this file.
set -euo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN_DIR="${FREECAL_RUN_DIR:-/tmp/freecal-run}"
DISPLAY_NUMBER="${FREECAL_DISPLAY:-:97}"
APP=./target/debug/freecal
STORE="$RUN_DIR/data/io.github.mmerkel.FreeCal/freecal.sqlite3"
VENV="$RUN_DIR/venv"
BUS="$RUN_DIR/bus"

export DISPLAY="$DISPLAY_NUMBER"
# GTK prefers Wayland when the session has it, which would put the window on
# the user's desktop instead of Xvfb.
unset WAYLAND_DISPLAY
export GDK_BACKEND=x11
# No accessibility bridge: FreeCal runs on its own D-Bus session (see bus below),
# GTK would make that bus start a second at-spi-bus-launcher, which can take
# and later delete the socket of the user's desktop accessibility bus.
export NO_AT_BRIDGE=1 GTK_A11Y=none
mkdir -p "$RUN_DIR/shots" "$RUN_DIR/data"

usage() {
  cat <<EOF
usage: $0 COMMAND
  build              debug build of the frontend and the shell (no bundles)
  bus                start the run's private D-Bus session if needed and print its address
  start [--fresh] [--subpixel]
                     start Xvfb and FreeCal; --fresh empties the Local Store first;
                     --subpixel renders text as on the user's machine (SKILL.md)
  restart [--subpixel]
                     relaunch FreeCal on the same Local Store
  stop               stop FreeCal, Xvfb and the private D-Bus session
  shot NAME [GEOM]   screenshot to $RUN_DIR/shots/NAME.png; GEOM crops, e.g. 260x200+0+0
  click X Y | dblclick X Y | rightclick X Y | drag X1 Y1 X2 Y2 | down X Y | up X Y | move X Y | type TEXT | key NAME
  wheel X Y up|down [N]
                     turn the mouse wheel N notches (default 1) there
  cursor X Y         move there and print the cursor's name (pointer, default, text, ...)
  sql QUERY          query the Local Store with sqlite3
  log                the app's output
EOF
  exit 1
}

input() {
  if [ ! -x "$VENV/bin/python" ]; then
    python3 -m venv "$VENV"
    "$VENV/bin/pip" -q --disable-pip-version-check install python-xlib
  fi
  "$VENV/bin/python" "$SKILL_DIR/xinput.py" "$@"
}

start_xvfb() {
  if ! xdpyinfo >/dev/null 2>&1; then
    Xvfb "$DISPLAY_NUMBER" -screen 0 1280x800x24 >"$RUN_DIR/xvfb.log" 2>&1 &
    timeout 10 bash -c 'until xdpyinfo >/dev/null 2>&1; do sleep 0.2; done'
  fi
}

# The run's own D-Bus session. Only one FreeCal runs per session (ADR 0005),
# so on the user's bus this FreeCal would hand off to theirs and exit.
start_bus() {
  if [ ! -s "$BUS.pid" ] || ! kill -0 "$(cat "$BUS.pid")" 2>/dev/null; then
    dbus-daemon --session --fork --print-address=1 --print-pid=3 \
      >"$BUS.address" 3>"$BUS.pid"
  fi
  cat "$BUS.address"
}

stop_bus() {
  if [ -s "$BUS.pid" ]; then kill "$(cat "$BUS.pid")" 2>/dev/null || true; fi
  rm -f "$BUS.pid" "$BUS.address"
}

launch() {
  [ -x "$APP" ] || { echo "no $APP: run '$0 build' first" >&2; exit 1; }
  # --subpixel replaces the CPU renderer rather than adding to it: the bug it
  # exists for only shows with GPU rendering (llvmpipe here).
  local rendering=(WEBKIT_DISABLE_DMABUF_RENDERER=1)
  if [ "$subpixel" = 1 ]; then
    local config="$RUN_DIR/subpixel-config"
    mkdir -p "$config/gtk-3.0"
    printf '%s\n' '[Settings]' gtk-xft-antialias=1 gtk-xft-hinting=1 \
      gtk-xft-hintstyle=hintslight gtk-xft-rgba=rgb >"$config/gtk-3.0/settings.ini"
    rendering=("XDG_CONFIG_HOME=$config" LIBGL_ALWAYS_SOFTWARE=1)
  fi
  env DBUS_SESSION_BUS_ADDRESS="$(start_bus)" XDG_DATA_HOME="$RUN_DIR/data" "${rendering[@]}" \
    nohup "$APP" >"$RUN_DIR/app.log" 2>&1 &
  # The window exists before the page has rendered; wait for both.
  timeout 30 bash -c 'until xwininfo -root -tree | grep -q "\"FreeCal\""; do sleep 0.2; done'
  sleep 3
  echo "FreeCal running on $DISPLAY_NUMBER; Local Store in $RUN_DIR/data"
}

stop_app() {
  # -x matches the process name only; `pkill -f target/debug/freecal`
  # would also match (and kill) the shell running this script.
  pkill -x freecal 2>/dev/null || true
  timeout 10 bash -c 'while pgrep -x freecal >/dev/null; do sleep 0.2; done'
}

command="${1:-}"
shift || true
fresh=0
subpixel=0
if [ "$command" = start ] || [ "$command" = restart ]; then
  for option in "$@"; do
    case "$option" in
      --fresh) [ "$command" = start ] || usage; fresh=1 ;;
      --subpixel) subpixel=1 ;;
      *) usage ;;
    esac
  done
fi
case "$command" in
  build) npx tauri build --debug --no-bundle ;;
  bus) start_bus ;;
  start)
    stop_app
    if [ "$fresh" = 1 ]; then rm -rf "$RUN_DIR/data"; mkdir -p "$RUN_DIR/data"; fi
    start_xvfb
    launch
    ;;
  restart) stop_app; launch ;;
  stop)
    stop_app
    pkill -f "Xvfb $DISPLAY_NUMBER" 2>/dev/null || true
    stop_bus
    ;;
  shot)
    [ $# -ge 1 ] || usage
    out="$RUN_DIR/shots/$1.png"
    if [ -n "${2:-}" ]; then import -window root -crop "$2" "$out"; else import -window root "$out"; fi
    echo "$out"
    ;;
  click | dblclick | rightclick | drag | down | up | move | wheel | type | key | cursor) input "$command" "$@" ;;
  sql) sqlite3 "$STORE" "$@" ;;
  log) grep -v '^libEGL warning' "$RUN_DIR/app.log" || true ;;
  *) usage ;;
esac
