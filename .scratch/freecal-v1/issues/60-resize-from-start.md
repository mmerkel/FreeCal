# 60: Resize an Event from its start

**What to build:** The user can drag the start edge of an Event as well as its end: the top edge of a timed Event in day and week views, the left edge of a multi-day or all-day bar. The end stays where it is. Follow-up to ticket 05.

**Blocked by:** none

**Status:** done

- [x] FullCalendar's `eventResizableFromStart` is on; a resize from the start goes through `rescheduleEvent` like one from the end (`rescheduleOfResize` already reads `startDelta`)
- [x] Works for timed Events in the time grid, for multi-day bars in the all-day area and the month view, and for all-day Events
- [x] An Event can't shrink past FullCalendar's minimum (one slot in the time grid, one day for bars) from either edge
- [x] Read-only Events still offer no resize on either edge
- [x] The resize cursor shows on the start edge too
- [x] A `gridDrag` test covers a resize that moves only the start; checked by hand with `run-freecal` (jsdom can't hit-test FullCalendar drags)

## Comments

Checked by hand with `run-freecal`: the top edge of a timed Event shows `n-resize` and a bar's left edge `w-resize`. Resizing from the start moved only the start, for a timed Event in the week view and for an all-day bar in the week view's all-day area and in the month view. Dragging the start past the end holds the preview at one slot or one day, and a release there changes nothing. A continuation segment of a bar has no start handle. Read-only Events, timed ones and bars, are covered by the `CalendarGrid` test only, because a local store has no read-only Calendar.
