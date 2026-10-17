Human adds items here. Remove when a ticket was created or otherwise the work is finished.

- [ ] 

## Import behaviour to review (ticket 58)

Decisions made while building ticket 58 that may want changing:

- [ ] **Changed Events:** undo matches Events by ID and UID. So an imported Event the user moved to another Calendar is still deleted, an updated one deleted since comes back, and a new Event that reused a deleted one's ID is left alone.
- [ ] **Read-only:** undo is refused while any Calendar it touches is Read-only.
- [ ] **Editing time:** when the user changes an Event's time, the pinned-zone note goes away (ADR 0007 rule 4). The ticket didn't ask for this.
- [ ] **Rule 2:** an update keeps the earlier pinned zone only when both start and end of the new version have no zone. If only one end lacks a zone, it takes the other end's zone (rule 3).
- [ ] **Nothing changed:** an import that adds or updates nothing sends no Signal.
- [ ] **Recurring Events:** waits for ticket 07 (box added there). Until then, a recurring Event is imported as its first Occurrence only, with no warning.
