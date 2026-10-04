---
name: run-freecal
description: Build, run, and drive the FreeCal desktop app (Tauri + Svelte). Use when asked to start or launch FreeCal, check a change in the real window, click through the sidebar or calendar grid, take a screenshot of the app, or inspect its Local Store.
---

FreeCal is a Tauri app (Rust core, WebKitGTK webview, Svelte frontend). An agent drives the debug build on a private Xvfb display with `.claude/skills/run-freecal/freecal.sh`. It starts and stops the app, sends clicks and keys through XTEST, takes screenshots with ImageMagick and queries the SQLite Local Store. Nothing appears on the user's own desktop.

All paths are relative to the repo root. Run every command from there.

## Prerequisites

These were already installed on the machine this was verified on (Ubuntu 24.04): `xvfb`, `imagemagick` (`import`), `x11-utils` (`xdpyinfo`, `xwininfo`), `libwebkit2gtk-4.1-dev` (Tauri's build dependency), `sqlite3`, Node with `npm install` done, and a Rust toolchain. The driver needs `python3` with `venv`. The first input command creates `/tmp/freecal-run/venv` and installs `python-xlib` there; no system packages are needed for it.

## Build

```bash
.claude/skills/run-freecal/freecal.sh build      # npx tauri build --debug --no-bundle → target/debug/freecal
```

Rebuild after every frontend or Rust change; the binary embeds the compiled frontend.

## Run (agent path)

```bash
R=.claude/skills/run-freecal/freecal.sh
$R start --fresh                 # Xvfb on :97, FreeCal with an empty Local Store
$R shot start                    # → /tmp/freecal-run/shots/start.png — Read it and look
$R click 60 52                   # "New calendar"
$R type "Home" && $R key Return
$R move 600 700                  # park the mouse so no row shows its hover actions
$R shot sidebar 260x200+0+0      # crop to the sidebar
$R sql 'select id, name, colour, shown from calendar'
$R restart                       # relaunch on the same Local Store (persistence checks)
$R log                           # app output, minus the harmless libEGL warnings
$R stop                          # stops FreeCal and Xvfb
```

| command | what it does |
|---|---|
| `build` | debug build without bundles |
| `start [--fresh]` | start Xvfb (if needed) and FreeCal, wait for the window; `--fresh` deletes the Local Store first |
| `restart` | relaunch FreeCal, keeping the Local Store |
| `stop` | stop FreeCal and Xvfb |
| `shot NAME [GEOM]` | screenshot to `/tmp/freecal-run/shots/NAME.png`; `GEOM` crops (`WxH+X+Y`) |
| `click X Y`, `dblclick X Y`, `move X Y` | mouse, in screen pixels |
| `type TEXT` | types characters (upper case and `#:<>!…` use Shift) |
| `key NAME` | one key by X keysym: `Return`, `Escape`, `Tab`, `ctrl+a`, `shift+Tab` |
| `sql QUERY` | `sqlite3` on the Local Store |
| `log` | the app's stdout and stderr |

`FREECAL_RUN_DIR` (default `/tmp/freecal-run`) and `FREECAL_DISPLAY` (default `:97`) override the locations.

**Finding coordinates:** there is no DOM access, so take a full screenshot, read it, and click by pixel. The window sits at 0,0 and is 1200×800 on the 1280×800 screen. Layout as of ticket 02: the sidebar's account heading is at y≈21. Calendar rows start at y≈48 and are 28 px apart; the "New calendar" button is below the last row. Each row's colour swatch, ✎ (rename) and ✕ (delete) are at x≈187, 208 and 227. The grid starts at x≈260.

## Run (human path)

```bash
XDG_DATA_HOME=/tmp/freecal-run/data ./target/debug/freecal   # window on your desktop; close it to stop
```

Without `XDG_DATA_HOME` it uses the real Local Store in `~/.local/share/io.github.mmerkel.FreeCal`. The agent path never does.

## Test

```bash
cargo test --workspace && cargo clippy --workspace --all-targets && npm run check && npm test
```

## Gotchas

- **Row actions are invisible until hovered** (`opacity: 0`). They still take clicks, but `move` over the row first if a screenshot should show them, and park the mouse elsewhere (`move 600 700`) for clean shots.
- **`<input type="color">` opens a GTK colour chooser dialog**, not a web popup. It is centred over the window (Cancel ≈ 705,528, Select ≈ 794,528, swatch grid from ≈ 392,276 in 52 px steps). Screenshot it before clicking.
- **Rename fields open with the name selected**, so typing replaces it; `ctrl+a` is not needed.
- **Never stop the app with `pkill -f target/debug/freecal`**: the pattern also matches the shell running the command, which kills it (exit code 144). The driver uses `pkill -x freecal`.
- **python-xlib's `Display.sync()` fails on Xvfb** with `AttributeError: 'BadRRModeError' object has no attribute 'sequence_number'`. `xinput.py` uses `flush()` plus a short pause instead.
- **FullCalendar's labels are English** whatever the locale until ticket 06; that is expected.

## Troubleshooting

- **`libEGL warning: DRI3 error: Could not get DRI3 device`** in the log: harmless on Xvfb; the webview renders in software. `log` hides it.
- **`unknown key 'ctrl'`** (or keycode 0): modifier names are `ctrl`, `shift`, `alt`, joined with `+` (`ctrl+a`); other names are X keysyms (`Return`, not `Enter`).
- **`start` times out waiting for the window**: check `$R log`; usually the binary is missing or stale. Run `build` first.
