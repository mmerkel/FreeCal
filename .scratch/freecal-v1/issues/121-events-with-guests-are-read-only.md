# 121: Events with guests are read-only

**What to build:** The second part of ticket 17 (read it for the history). Any Event with attendees other than the user is read-only, as an Event of a Read-only Calendar is. Attendee data is never shown and always preserved (spec: "Attendees").

- The core reads whether an Event has guests from the ATTENDEE properties of its Corresponding Server Object, and the core interface says so on the Event. The core refuses edits, moves and deletes of such an Event, as for a Read-only Calendar.
- Decide how the core knows the user's own address, so that an Event whose only attendee is the user stays writable (for CalDAV, for example, the principal's `calendar-user-address-set`), and record it in a comment. Google (ticket 26) will need the same answer.
- The Event's details say "has guests" with the hint to edit it on the server; the grid offers no move or resize for it, and a refused drag explains why (US-31, US-32).

Spec: US-31, US-32, US-46; "Attendees"; "Event details".

**Blocked by:** 56

**Status:** ready-for-agent

**Effort:** M

- [ ] Any Event with attendees other than the user is read-only; the core refuses changes to it
- [ ] Its details say "has guests" with the hint to edit it on the server; the grid offers no move or resize, and a refused drag explains why
- [ ] Core tests for who counts as a guest; component tests for the details, the grid and the refused drag

## Comments
