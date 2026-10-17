# 44: Diagnose the lag when resizing the window

**What to build:** While the user drags the window's size, the content follows a little sluggishly, in both debug and release builds (X11, KDE). Find the cause by measuring, then fix it. Use the `diagnosing-bugs` skill. Spec: user story 143.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

- [ ] Reproduce and measure the delay between a size change and the redrawn content
- [ ] Try FullCalendar's `windowResizeDelay` (100 ms by default) at 0, and redrawing through a `ResizeObserver` on the grid's container
- [ ] Compare against a blank WebKitGTK page in the same window, to separate WebKitGTK's own lag from FreeCal's
- [ ] Try WebKitGTK's compositing environment variables and note their effect
- [ ] Record the findings as a comment here; fix only what the measurements point to, and say plainly if the rest is WebKitGTK's own

Out of scope: the minimum window size (ticket 39).
