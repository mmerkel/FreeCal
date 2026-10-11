# 39: Minimum window size

**What to build:** The window can't be made smaller than a size at which the sidebar at its default width and the week view still fit. Spec: user story 138, "Window size" under Frontend.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] `minWidth` and `minHeight` in `tauri.conf.json`, about 800×560; the exact values are measured in the real app with `run-freecal` so that the toolbar, the sidebar at 14rem and seven week columns with readable Event chips fit
- [ ] At the minimum size the Event editor fits or scrolls inside its dialog (ticket 37's maximum height), with Save reachable
- [ ] Checked by hand in the real app, including a screenshot at the minimum size

Out of scope: hiding the sidebar on narrow windows (ticket 31 keeps it from hiding on its own); remembering the window's size and position.
