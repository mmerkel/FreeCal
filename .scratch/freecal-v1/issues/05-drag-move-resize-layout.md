# 05: Drag to move or resize, overlap layout, all-day area

**What to build:** The user can drag an Event to another time or day and drag its edge to change its duration. Overlapping Events appear side by side in day and week views, and all-day and multi-day Events appear in an all-day area.

**Blocked by:** 03

**Status:** done

- [x] Dragging an Event moves it; dragging its edge resizes it; both go through the core interface
- [x] Overlapping Events are laid out side by side in day and week views
- [x] All-day and multi-day Events are shown in the all-day area
- [x] Component tests check that drag actions produce the right core calls
- [ ] A "Date changed" Signal during a drag, a resize or a drag-to-create doesn't cancel it: the grid holds back the new today, and the App holds back the jump while tracking today, until the interaction ends. The same goes while the recurrence scope prompt for a drop is open (once ticket 08 adds it). Component tests cover a "Date changed" in the middle of each.

## Comments

- 2026-10-11: Since ticket 04, a new today rebuilds the FullCalendar instance (`CalendarGrid.svelte`), because FullCalendar reads `now` only when it is created. A rebuild in the middle of an interaction drops it without a word, and a `revert()` held from the old instance may do nothing. This matters most after waking from sleep: the shell checks the date every 10 seconds, so "Date changed" can arrive while the user has already started to drag. Today a drag-to-create is lost the same way (no data at risk).
- 2026-10-13: If ticket 56 (read-only CalDAV Calendars) is done first, dragging or resizing a read-only Event shows a short message explaining why; otherwise 56 adds it.
- 2026-10-13: Done, except the part of the last box that waits on ticket 08: holding back "Date changed" while the scope prompt for a drop is open. Ticket 08 has a box for it now. Notes:
  - A drag or resize goes through a new core call, `rescheduleEvent(id, when)`. It changes only the time, keeps the Event Time Zones (ADR 0007) and refuses read-only Events. Dragging a timed Event into the all-day row and back gives it the Display Time Zone, because by then it has no zone to keep.
  - "Multi-day" means timed Events of 24 hours or more, as in Google Calendar: in day and week views they show in the all-day area over the dates they touch, and a drag there keeps their times. A timed Event from 22:00 to 02:00 stays in the time grid.
  - jsdom can't hit-test FullCalendar drags (it measures rects), so the component tests stop where FullCalendar hands over: `gridDrag.test.ts` turns FullCalendar's drop and resize arguments into core calls, the App tests drive `onreschedule` through the stub grid, and the grid tests check that a press on a chip, a resize handle or a slot holds back a new today until the release. Move, resize, a move across days, side by side, into the all-day row and a long Event dragged in the all-day row were checked by hand in the running app.
  - A refused drag draws the Occurrences again rather than calling `revert()`, which may do nothing once the grid has been rebuilt.
