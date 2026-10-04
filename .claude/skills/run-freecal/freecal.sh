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

export DISPLAY="$DISPLAY_NUMBER"
mkdir -p "$RUN_DIR/shots" "$RUN_DIR/data"

usage() {
  cat <<EOF
usage: $0 COMMAND
  build              debug build of the frontend and the shell (no bundles)
  start [--fresh]    start Xvfb and FreeCal; --fresh empties the Local Store first
  restart            relaunch FreeCal on the same Local Store
  stop               stop FreeCal and Xvfb
  shot NAME [GEOM]   screenshot to $RUN_DIR/shots/NAME.png; GEOM crops, e.g. 260x200+0+0
  click X Y | dblclick X Y | rightclick X Y | drag X1 Y1 X2 Y2 | move X Y | type TEXT | key NAME
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

launch() {
  [ -x "$APP" ] || { echo "no $APP: run '$0 build' first" >&2; exit 1; }
  XDG_DATA_HOME="$RUN_DIR/data" WEBKIT_DISABLE_DMABUF_RENDERER=1 \
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
case "$command" in
  build) npx tauri build --debug --no-bundle ;;
  start)
    stop_app
    if [ "${1:-}" = "--fresh" ]; then rm -rf "$RUN_DIR/data"; mkdir -p "$RUN_DIR/data"; fi
    start_xvfb
    launch
    ;;
  restart) stop_app; launch ;;
  stop)
    stop_app
    pkill -f "Xvfb $DISPLAY_NUMBER" 2>/dev/null || true
    ;;
  shot)
    [ $# -ge 1 ] || usage
    out="$RUN_DIR/shots/$1.png"
    if [ -n "${2:-}" ]; then import -window root -crop "$2" "$out"; else import -window root "$out"; fi
    echo "$out"
    ;;
  click | dblclick | rightclick | drag | move | type | key) input "$command" "$@" ;;
  sql) sqlite3 "$STORE" "$@" ;;
  log) grep -v '^libEGL warning' "$RUN_DIR/app.log" || true ;;
  *) usage ;;
esac
