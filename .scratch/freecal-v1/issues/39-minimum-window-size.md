# 39: Minimum window size

**What to build:** The window can't be made smaller than a size at which the sidebar at its default width and the week view still fit. Spec: user story 138, "Window size" under Frontend.

**Blocked by:** none

**Status:** done

- [x] `minWidth` and `minHeight` in `tauri.conf.json`, about 800×560; the exact values are measured in the real app with `run-freecal` so that the toolbar, the sidebar at 14rem and seven week columns with readable Event chips fit
- [x] At the minimum size the Event editor fits or scrolls inside its dialog (ticket 37's maximum height), with Save reachable
- [x] Checked by hand in the real app, including a screenshot at the minimum size

Out of scope: hiding the sidebar on narrow windows (ticket 31 keeps it from hiding on its own); remembering the window's size and position.

## Comments

- 2026-10-11: Done at 880×560 (`minWidth`, `minHeight` in `src-tauri/tauri.conf.json`). Measured with `run-freecal` by resizing the window through Xlib (Xvfb has no window manager, so the hints aren't enforced there) and, for the measurement only, switching the grid to `timeGridWeek`. At 800 px the week columns are 72 px wide and the chip's "13:00 – 14:00" is cut off; at 880 px they are 82 px and it fits, with the title on the next line ending in "…". In the month view, a timed chip shows its time and the start of its title ("13:00 De…"). At 560 px high the Event editor is a few pixels taller than the window; it scrolls inside the dialog through WebKit's default `max-height` for modal dialogs, with Save visible. Ticket 37 adds the explicit maximum height. The toolbar from ticket 04 will take some height from the grid; the week view scrolls and the month rows have room to spare.
- 2026-10-11: KWin on X11 ignores the minimum for quick-tiled windows: tiling to the left and then dragging the tile's edge shrank FreeCal to 406 px wide, while `xprop` still showed "program specified minimum size: 880 by 560". KDE does this on purpose and has made Wayland behave the same since Plasma 6.0.2 ([bug 434615](https://bugs.kde.org/show_bug.cgi?id=434615)), so the app has to cope. `.app` in `App.svelte` now has `min-width: 880px; min-height: 560px`: below that, the layout keeps its size and the page scrolls. Checked at 406×500 on Xvfb (with the config's minimum lowered for that build only, because GTK resizes an unmanaged window back to its minimum). A narrow layout instead of scrolling is ticket 45.
