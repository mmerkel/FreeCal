# 19: Undo through the Change Journal

**What to build:** The user can see every change in the Change Journal as one timeline, undo any one of them, or undo everything in a Calendar since a given time, and can see and undo an Event's History from its details. Undo follows ADR 0009 and ticket 69's rules.

**Blocked by:** 17, 69, 70

**Status:** ready-for-agent

**Effort:** L

- [ ] The Change Journal list: one timeline, newest first, of every unit (the user's changes, imports including those into Local Calendars, syncs as "Changed on the server: 3 Events in Work", Conflict resolutions, Snapshot restores, undos)
- [ ] A unit expands into its Events, each with Undo; the unit has "Undo all" for the Events still not undone
- [ ] Undo one entry: the previous version is written back under the same identity (original ID, all details), later changes to the same Events with it
- [ ] Undo everything for a Calendar since a given time
- [ ] A History button in the Event's details lists the Event's entries, each with Undo
- [ ] Dangerous undos ask first as in ticket 70; the confirmation is the Mass-Change Guard's approval, so the next sync doesn't pause the Calendar
- [ ] Undo sends "Occurrences changed"

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009). The 90-day removal moved to ticket 69.
