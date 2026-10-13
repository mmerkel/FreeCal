# 60: Resize an Event from its start

**What to build:** The user can drag the start edge of an Event as well as its end: the top edge of a timed Event in day and week views, the left edge of a multi-day or all-day bar. The end stays where it is. Follow-up to ticket 05.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] FullCalendar's `eventResizableFromStart` is on; a resize from the start goes through `rescheduleEvent` like one from the end (`rescheduleOfResize` already reads `startDelta`)
- [ ] Works for timed Events in the time grid, for multi-day bars in the all-day area and the month view, and for all-day Events
- [ ] An Event can't shrink past FullCalendar's minimum (one slot in the time grid, one day for bars) from either edge
- [ ] Read-only Events still offer no resize on either edge
- [ ] The resize cursor shows on the start edge too
- [ ] A `gridDrag` test covers a resize that moves only the start; checked by hand with `run-freecal` (jsdom can't hit-test FullCalendar drags)
