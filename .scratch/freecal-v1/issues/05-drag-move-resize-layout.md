# 05: Drag to move or resize, overlap layout, all-day area

**What to build:** The user can drag an Event to another time or day and drag its edge to change its duration. Overlapping Events appear side by side in day and week views, and all-day and multi-day Events appear in an all-day area.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] Dragging an Event moves it; dragging its edge resizes it; both go through the core interface
- [ ] Overlapping Events are laid out side by side in day and week views
- [ ] All-day and multi-day Events are shown in the all-day area
- [ ] Component tests check that drag actions produce the right core calls
- [ ] A "Date changed" Signal during a drag, a resize or a drag-to-create doesn't cancel it: the grid holds back the new today, and the App holds back the jump while tracking today, until the interaction ends. The same goes while the recurrence scope prompt for a drop is open (once ticket 08 adds it). Component tests cover a "Date changed" in the middle of each.

## Comments

- 2026-10-11: Since ticket 04, a new today rebuilds the FullCalendar instance (`CalendarGrid.svelte`), because FullCalendar reads `now` only when it is created. A rebuild in the middle of an interaction drops it without a word, and a `revert()` held from the old instance may do nothing. This matters most after waking from sleep: the shell checks the date every 10 seconds, so "Date changed" can arrive while the user has already started to drag. Today a drag-to-create is lost the same way (no data at risk).
- 2026-10-13: If ticket 56 (read-only CalDAV Calendars) is done first, dragging or resizing a read-only Event shows a short message explaining why; otherwise 56 adds it.
