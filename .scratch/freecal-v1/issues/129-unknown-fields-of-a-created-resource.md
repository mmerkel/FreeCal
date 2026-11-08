# 129: A resource FreeCal creates keeps what FreeCal doesn't show

**What to build:** Found by ticket 125's tests against Radicale. What FreeCal doesn't show (custom and `X-` properties, alarms, attachments, parameters) lives only in the Mirror's text of a UID's Server Object. Whenever the CalDAV Sender creates a resource, it builds the text from the Event Rows alone (ticket 122's `resource_to_send` without a Server Object), so those fields are lost on the server in three cases:

- **A move to another Calendar:** the Event is created in the target from its Event Rows and deleted in the source. The spec allows dropping details only "when the Event has Provider-specific details that the target can't hold" ("Moves across Accounts"), so a move between CalDAV Calendars should keep them.
- **Undoing a sent DELETE:** the Mirror forgot the Server Object, so the undo creates the Event anew, at the name FreeCal gives the UID (`<UID>.ics`), not at its original href, and without its unknown fields. Ticket 125 asked for an undo "under the original identity (same UID, same href)"; the UID holds, the href doesn't.
- **"This and following events":** the new Recurring Event is a new UID, created from its Event Rows, so it doesn't take the original's alarms or other unknown properties along, as Google and other clients do when they split a series.

Decide where those fields live once no Server Object holds them, name the trade-off, and record it in an ADR. Options to weigh, among others:

- The Event Version holds them (the `VEVENT`'s lines FreeCal doesn't read), so the Change Journal keeps them, an undo puts them back and a move carries them. A migration, a new `HASH_FORMAT`, and against ADR 0011's line that Event Rows hold no server data.
- The Sender finds them elsewhere when it creates: the UID's Server Object in the source Calendar for a move (gone if the DELETE was sent first), the Change Journal for an undo. No schema change, but one rule per case.
- Accept the loss for some cases (a split is a new Event) and say so in the spec.

Spec: "Untrusted content"; "Moves across Accounts"; "Change Journal". ADRs 0010, 0011, 0015.

**Blocked by:** 127

**Status:** needs-triage

**Effort:** L

- [ ] A decision on where unknown fields live without a Server Object, in an ADR
- [ ] An Event moved to another CalDAV Calendar keeps what FreeCal doesn't show on the server
- [ ] Undoing a sent DELETE puts the Event back with what FreeCal doesn't show, and at its href if the decision keeps it
- [ ] The new Recurring Event of "this and following events" is decided: takes the original's unknown properties along, or not
- [ ] Tested against Radicale

## Comments
- 2026-11-08 (/implement 125): the tests to extend are in `core/tests/caldav_send.rs`: `an_event_moved_to_another_calendar_moves_on_the_server` and `undo::undoing_a_sent_delete_puts_the_event_back_under_its_uid` (which asserts the `<UID>.ics` name today).
