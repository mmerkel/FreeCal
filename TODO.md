Human adds items here. Remove when a ticket was created or otherwise the work is finished.

- [ ] hand test import
  - [ ] seems some timed events are not imported or don't show up

## Import behaviour to review (ticket 58)

Decisions made while building ticket 58 that may want changing:

- [ ] **Changed Events:** undo matches Events by ID and UID. So an imported Event the user moved to another Calendar is still deleted, an updated one deleted since comes back, and a new Event that reused a deleted one's ID is left alone.
- [ ] **Read-only:** undo is refused while any Calendar it touches is Read-only.
- [ ] **Editing time:** when the user changes an Event's time, the pinned-zone note goes away (ADR 0007 rule 4). The ticket didn't ask for this.
- [ ] **Rule 2:** an update keeps the earlier pinned zone only when both start and end of the new version have no zone. If only one end lacks a zone, it takes the other end's zone (rule 3).
- [ ] **Nothing changed:** an import that adds or updates nothing sends no Signal.
- [ ] **Recurring Events:** waits for ticket 07 (box added there). Until then, a recurring Event is imported as its first Occurrence only, with no warning.

## Import flow to review (ticket 59)

- [ ] **Default choice:** "Update them" is chosen when some Events already exist.
- [ ] **Undo message:** stays until dismissed; once dismissed, the import can't be undone until ticket 19's list.
- [ ] **Not under "Working…":** picking and previewing a file (they change nothing), so a large file shows "Reading the file…" in the dialog instead.
- [ ] **One Calendar per import:** a Takeout `.zip` of several Calendars goes into one Calendar.
