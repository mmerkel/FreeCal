# 144: The CalDAV writer builds a resource from managed fields and Unimplemented Fields

**What to build:** A writer that builds the iCalendar text of one CalDAV resource from Event Versions alone: the managed fields and the tree of ticket 143, for a UID's single Event, or its Recurring Event with its Exceptions. It reads no text from a server. Pure code: nothing calls it yet; the Sender switches to it in ticket 150, and until then the patching writer (`resource_to_send`, `update`, `rewrite` in `core/src/sync/caldav/resource/mod.rs`) stays as it is.

- **Input:** what a Loaded Event holds, with a tree per Event Version. **Output:** the resource's text and nothing else.
- **Managed properties** are written as today's writer writes a new `VEVENT` (`new_vevent` and the property groups in `core/src/sync/caldav/resource/write.rs`), with the unknown parameters the tree holds under that property's name.
- **Entries of the tree** are written back as properties, parameters and components; calendar-level entries go outside the `VEVENT`s. An entry whose prefix isn't `ical:` is not written (neutral names don't exist yet, and nothing is `caldav:`).
- **Set fresh on every write:** `DTSTAMP`, `LAST-MODIFIED`, the time zone definitions the written times need (`vtimezone`, as now), and at the calendar level `VERSION` and `PRODID`.
- **Order of output** is fixed by the writer, so the same input always gives the same text.
- **Round trip, the ticket's main test:** for any managed fields and tree, reading the written text with ticket 143's reader gives the same managed fields and an equal tree. For the corpus of server texts in `core/src/sync/caldav/resource/tests.rs`, reading and writing gives a text that reads back equal. ADR 0017's consequence is why: a property the pair fails to carry is lost on the next write of that Event.
- **Untrusted content:** values from the tree are escaped and folded by the writer like any other; an entry can't break out of its line or its component.

- **`SEQUENCE`** is an entry of the tree and is written back as it is; a new Event has none. FreeCal doesn't raise it: it changes no Event with guests (ticket 121), and RFC 5546 asks for a higher number only there. Ticket 126 decides who raises it. So the text FreeCal sends reads back as the Event Version it was built from.

Spec: "Local Store"; "Untrusted content". ADR 0017.

**Blocked by:** 143

**Status:** ready-for-agent

**Effort:** M

- [ ] A `SEQUENCE` in the tree is written back unchanged; none is written for an Event without one
- [ ] The writer builds a resource from managed fields and trees alone, for a single Event, a Recurring Event with Exceptions, and Exceptions without their Recurring Event
- [ ] Round trip from fields and tree: tests over every kind of property value, parameter and component that ticket 143's tests cover
- [ ] Round trip from the corpus of server texts
- [ ] A hostile entry (line breaks, `END:VEVENT`, control characters in names and values) can't change the structure of the text written
- [ ] The same input gives the same text

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second half of what was proposed as one ticket (L); the reader is ticket 143.
- 2026-11-16: `SEQUENCE` decided by the user: kept as an entry and written back unchanged until the attendee features (ticket 126); ADR 0017 has the line.
- 2026-11-22: Ticket 169 uses this writer at fetch time, in memory, to check that the reader lost nothing; ticket 170 runs the same check over a corpus of real servers' texts.
