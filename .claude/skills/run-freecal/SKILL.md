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
$R start --fresh                 # Xvfb on :97, FreeCal with a new Local Store
$R shot start                    # → /tmp/freecal-run/shots/start.png — Read it and look
$R click 200 27                  # "+" (New calendar) next to the account heading
$R type "Birthdays" && $R key Return
$R move 600 700                  # park the mouse so no row shows its hover actions
$R shot sidebar 260x200+0+0      # crop to the sidebar
$R cursor 236 30                 # → pointer: the cursor's name over Today
$R sql 'select id, name, colour, shown from calendar'
$R restart                       # relaunch on the same Local Store (persistence checks)
$R log                           # app output, minus the harmless libEGL warnings
$R stop                          # stops FreeCal and Xvfb
```

| command | what it does |
|---|---|
| `build` | debug build without bundles |
| `start [--fresh]` | start Xvfb (if needed) and FreeCal, wait for the window; `--fresh` deletes the Local Store first, so FreeCal starts on a new one with Work (shown) and Home (hidden) |
| `restart` | relaunch FreeCal, keeping the Local Store |
| `stop` | stop FreeCal and Xvfb |
| `shot NAME [GEOM]` | screenshot to `/tmp/freecal-run/shots/NAME.png`; `GEOM` crops (`WxH+X+Y`) |
| `click X Y`, `dblclick X Y`, `rightclick X Y`, `move X Y` | mouse, in screen pixels |
| `drag X1 Y1 X2 Y2` | press, move in steps, release: drag-to-create across days |
| `down X Y`, `up X Y` | press or release the left button there; with `move` and `shot` in between, a screenshot can be taken mid-drag |
| `cursor X Y` | move there and print the name of the cursor shown, as WebKitGTK names it after CSS: `pointer`, `default`, `text`, `col-resize`, …; after `down` it shows the cursor mid-drag |
| `type TEXT` | types printable ASCII, with Shift wherever the keyboard map needs it |
| `key NAME` | one key by X keysym: `Return`, `Escape`, `Tab`, `ctrl+a`, `shift+Tab` |
| `sql QUERY` | `sqlite3` on the Local Store |
| `log` | the app's stdout and stderr |

`FREECAL_RUN_DIR` (default `/tmp/freecal-run`) and `FREECAL_DISPLAY` (default `:97`) override the locations.

**Finding coordinates:** there is no DOM access, so take a full screenshot, read it, and click by pixel. The window sits at 0,0 and is 1200×800 on the 1280×800 screen. Layout as of ticket 47, with the sidebar at its default 12rem: the sidebar's account heading is at y≈27, with its "+" (New calendar) at x≈168. Calendar rows start at y≈56 and are 28 px apart; as of ticket 34 a new Local Store has Work at y≈56 and Home at y≈84, and with no Calendars the account shows a hint instead of rows; clicking a row shows or hides it. Each row's ⋮ button is at x≈170. As of ticket 48 its menu is a popover centred under the ⋮ (x≈81 to 259), above the row when there is no room below: from the first row (y≈56), the 24 swatches are in three rows at y≈108, 126 and 144 (x≈100 to 238, 20 px apart), Rename at y≈181, Show only this at y≈211 and Delete at y≈250. While a menu is open, the wheel scrolls neither the sidebar nor the grid. While a menu is open, a click or right-click anywhere outside it only closes it. Shift+F10 on a focused row opens the same menu. The delete dialog's Cancel and Delete are at about 660,447 and 740,447. As of ticket 47 the sidebar is 192 px wide (12rem) and the grid starts at x≈204.

As of ticket 04, a toolbar sits over the grid at y≈30. As of ticket 47 it starts with Today at x≈236 (pressed while tracking today), then previous and next at 297 and 335, the period title from x≈364, and Day, Week and Month at 992, 1048 and 1112. The shortcuts `d`, `w`, `m`, `j`, `k` and `t` are often easier than clicks. The view is stored in `app_state` (`sql 'select view from app_state'`), so `start --fresh` opens in month view and `restart` reopens the last one.

As of ticket 33, the ⋮ main menu is at 1170,30. As of ticket 48 it ends at the window's right edge; its items are Settings… at 1100,70, About FreeCal at 1100,99 and Quit at 1100,138; `key ctrl+comma` also opens Settings. In Settings, Appearance's System, Light and Dark are at 484, 600 and 716 on y≈403, and Close at 742,458. The appearance is stored in `app_state` too (`sql 'select appearance from app_state'`). The window starts hidden and appears once the frontend knows the appearance, so its X window only exists from then on.

In the month view of October 2026 (weeks start on Monday): each day column is 140 px wide from x≈204 and each week row is 117 px tall from y≈85, so 5 October is at about 272,260. In the week and day views the all-day row is at y≈103 and the time grid shows 06:00 at y≈136, 48 px per hour; week columns are 133 px wide from x≈259. Clicking a day number opens that day in day view. In day and week view, clicking a month name in the title opens that month in month view. Clicking a day or slot, or dragging across days (`drag 440 260 712 260`), opens the "New event" editor, and `key c` opens it at the next full hour. As of ticket 05, a `drag` that starts on a chip moves it instead (30-minute steps in the time grid; into the all-day row it becomes all-day), and one that starts on a chip's bottom edge resizes it (as of ticket 60 the top edge, or a bar's left edge, resizes it from the start; the handle is only a few px, so find it with `cursor` first); check the result with `sql 'select title, all_day, start_at, end_at from event'`. Timed Events of a day or more show in the all-day row. With no writable Calendar, either one shows the "No calendar for new events" hint (OK at 750,447). The editor's Title has focus, and Return there saves, like Save at about 745,628; Cancel is at 671,628. Clicking a chip opens the Event's details, which are centred and as tall as their content, so take a screenshot before clicking Delete (left), Edit or Close (right). A title with a line break or a description stored as HTML can't be entered through the window; to check those, insert an `event` row with `sqlite3` (fields from `fixtures/hostile-event.json`, `description_html = 1`) and `restart`.

As of ticket 47 the sidebar's right edge (x≈192 at the default 12rem) is a splitter with a tab on it at about 197,400. A click on the tab (or `key F9`) hides and shows the sidebar; a press that moves 4 px or more is a drag. A drag puts the edge where the pointer goes, keeping where it was grabbed: between 8rem and 20rem (128 and 320 px), and left of x≈64 (4rem) the sidebar hides, live. `drag 192 300 320 300` widens it to 20rem, `drag 192 300 40 300` hides it. Hidden, the tab is at about 5,400 and the grid and toolbar start at x≈12 (Today at about 44,30); `drag 5 400 245 400` pulls it out to 15rem, and a pull that stays left of x≈69 shows nothing. Its state is in `app_state` (`sql 'select sidebar_shown, sidebar_width from app_state'`, width in rem, NULL for the default). The other coordinates here assume the sidebar shown.

A ticket that moves the layout updates the coordinates here.

**Checking cursors:** screenshots don't show the pointer and jsdom has no cursors, so use `cursor`. It reads the name through XFixes. An open dialog's backdrop covers the whole window, so everywhere outside the dialog says `default`.

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

- **The ⋮ button is invisible until its row is hovered** (`opacity: 0`). It still takes clicks, but `move` over the row first if a screenshot should show them, and park the mouse elsewhere (`move 600 700`) for clean shots.
- **Dark mode:** choose Dark in Settings, or `sql "update app_state set appearance = 'dark'"` and `restart`. With Appearance on System, WebKitGTK takes `prefers-color-scheme` from the GTK theme: start with `GTK_THEME=Adwaita:dark $R restart`.
- **Another system font:** point `FONTCONFIG_FILE` at a config that prepends the family (e.g. DejaVu Sans, the widest common one) and restart, to check that nothing overflows.
- **Once the sidebar has more Calendars than fit, its overlay scrollbar takes clicks at x≈170** near its bar: a click on a ⋮ there scrolls the sidebar instead (to the end, low down). Click ⋮ at x≈164. The bar also shows over an open menu while the pointer is over the sidebar's area. There is no `wheel` command; send X buttons 4 and 5 through XTEST to scroll.
- **Rename fields open with the name selected**, so typing replaces it; `ctrl+a` is not needed.
- **Never stop the app with `pkill -f target/debug/freecal`**: the pattern also matches the shell running the command, which kills it (exit code 144). The driver uses `pkill -x freecal`.
- **python-xlib's `Display.sync()` fails on Xvfb** with `AttributeError: 'BadRRModeError' object has no attribute 'sequence_number'`. `xinput.py` uses `flush()` plus a short pause instead.
- **FullCalendar's labels are English** whatever the locale until ticket 06; that is expected.

## Troubleshooting

- **`libEGL warning: DRI3 error: Could not get DRI3 device`** in the log: harmless on Xvfb; the webview renders in software. `log` hides it.
- **`unknown key 'ctrl'`** (or keycode 0): modifier names are `ctrl`, `shift`, `alt`, joined with `+` (`ctrl+a`); other names are X keysyms (`Return`, not `Enter`).
- **`start` times out waiting for the window**: check `$R log`; usually the binary is missing or stale. Run `build` first.
