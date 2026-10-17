# 61: Show a dragged bar where it would land

**What to build:** While the user drags a multi-day or all-day Event in the month view or the all-day area of the week view, the bars show where the Event would land, split across rows as they would be after the drop: an Event over 10 days dragged in the month view shows its two bars with their right lengths, as in Google Calendar. Follow-up to ticket 05.

**Blocked by:** none

**Status:** needs-triage

**Effort:** M

## What FullCalendar does now

FullCalendar 7 draws the result of a resize in the day grid, but not of a drag: during a drag it only shades the target cells and moves a small copy of the chip with the pointer (`getMirrorSegs` in the DayGrid code returns only resize segments). The time grid already draws a dragged Event in place. So this needs FreeCal's own code, not an option.

A likely approach: read the target range from FullCalendar's `eventAllow` callback, which gets it on every move, draw a temporary preview Event with that range so FullCalendar splits it into its row bars, and hide the shading, the small copy and the original chip. A `/prototype` may be needed to see whether FullCalendar takes a changed event list in the middle of a drag.

## Grilling session

Before this ticket is ready, run a `/grill-me` session on the questions below and record the answers under `## Answers`, then turn them into checkboxes and set the status to `ready-for-agent`.

1. What happens to the original bars during the drag: stay in place faded, as in Google Calendar, or disappear?
2. Does the preview look like the Event (its Calendar colour and title) or like a neutral outline?
3. Does the cell shading stay, or does the preview replace it? And the small copy under the pointer?
4. Does the week view's all-day area get the same preview, and the day view's?
5. A timed Event dragged from the time grid into the all-day area, or a long Event dragged out of it: what does the preview show while crossing?
6. A preview that runs past the visible range: cut off at the edge, as the bars are now?
7. Read-only Events offer no drag, so no preview; anything else to show there (ticket 56's message)?
8. A drop that the core refuses: the preview disappears and the bars return; any message?
9. A "Date changed" rebuild in the middle of a drag is already held back (ticket 05); does the preview need anything more?
10. Does a resize, which FullCalendar already previews, need to look the same as the new drag preview?
11. How is it checked: by hand with `run-freecal` only, or can the preview's range be tested in jsdom through `eventAllow`?
