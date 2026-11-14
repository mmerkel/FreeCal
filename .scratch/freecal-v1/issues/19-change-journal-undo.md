# 19: Undo through the Change Journal

**What to build:** The user can see every change in the Change Journal as one timeline, undo any one of them, or undo everything in a Calendar since a given time, and can see and undo an Event's History from its details. Undo follows ADR 0009 and the rules of tickets 69 and 72.

**Blocked by:** 70, 72, 125

**Status:** ready-for-agent

**Effort:** M

- [ ] The Change Journal list: one timeline, newest first, of every Journal Transaction (the user's changes, imports including those into Local Calendars, syncs as "Changed on the server: 3 Events in Work", Conflict resolutions, Snapshot restores, undos)
- [ ] A Journal Transaction expands into its Events, each with Undo; the Journal Transaction has "Undo all" for the Events still not undone
- [ ] Undo one Journal Event Modification: the previous Event Version is written back under the same identity (original ID, all details), later changes to the same Events with it
- [ ] Undo everything for a Calendar since a given time
- [ ] A History button in the Event's details lists the Event's Journal Event Modifications, each with Undo
- [ ] Dangerous Undos ask first as in ticket 70; the confirmation is the Mass-Change Guard's approval, so the next sync doesn't pause the Calendar
- [ ] Undo sends "Occurrences changed"
- [ ] What Ctrl+Z undoes and Ctrl+Shift+Z redoes comes from the core, worked out from the Change Journal (the user's own Journal Transactions since FreeCal started, not undone, newest first; redo: the undos made since the user's last change), instead of the list the frontend keeps (`SessionUndo`, `src/lib/sessionUndo.ts`), so that an undo from the timeline or an Event's History counts at once

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009). The 90-day removal moved to ticket 69.
- 2026-10-18: Ticket 69 was split; the per-Event undo rules and the 90-day removal are now ticket 72.
- 2026-11-07: Re-sized from L to M. Ticket 72 now holds the per-Event undo rules and the Dangerous Undo classification, and ticket 70 the confirmation; what's left here is the Change Journal list, the History in the details, and undoing a Calendar since a given time.
- 2026-11-14: Added the box for Ctrl+Z's list. Ticket 70 keeps that list in the frontend: Journal Transaction IDs with labels for the toasts. It can't disagree with the Change Journal while Ctrl+Z is the only way to undo. Once this ticket lets the user undo from the timeline or History, it can. Moving it into the core leaves one source of truth. Strongest con: two more calls on the core interface, and the redo rule has to be stated in Change Journal terms. Its name ("Session Undo", proposed in ticket 70) is decided here.
