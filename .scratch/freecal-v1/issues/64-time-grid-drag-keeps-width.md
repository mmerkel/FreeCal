# 64: A dragged Event keeps its width in the time grid

**What to build:** In the day and week views, an Event that shares its time with another is drawn narrower than its column, side by side with the other. While the user drags it, FullCalendar draws the dragged copy across the whole width of the column it is over, so it jumps wider when grabbed and covers the Events beside it. The copy should keep the width and the horizontal offset it had when grabbed, in whichever column it is over: an Event drawn in the right half of Tuesday stays a right-half-wide copy in the right half of Wednesday. After the drop, the layout is worked out as now. Follow-up to ticket 05; ticket 61 does the same kind of preview for bars in the month view and the all-day area.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] While a timed Event is dragged in the time grid, its copy has the width and offset (as fractions of the column) the Event had when the drag started, in any column and at any time
- [ ] An Event that had the whole column keeps the whole column, as now
- [ ] Dragged into the all-day row, it becomes a bar as now; resizing is unchanged
- [ ] The drop and the layout after it are unchanged
- [ ] Tests at the seam that computes the copy's placement, if there is one jsdom can reach; checked by hand with `run-freecal`, results noted below: two overlapping Events in the week view, each dragged to another day and time, keep their width and side while moving

Out of scope: showing the width the Event would get beside whatever it would overlap where it lands (as Google Calendar does). That could be a later step.

## Comments

- 2026-10-17: From `TODO.md` ("keep initial width of event during drag"), settled in a grilling session: keep both the width and the offset, so nothing jumps when grabbed.
