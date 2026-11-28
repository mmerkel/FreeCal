# 176: The CalDAV writer builds a resource from managed fields and the tree

**What to build:** Old ticket 144 as written (`git show design/snapshot-line:.scratch/freecal-v1/issues/144-caldav-writer-from-fields-and-tree.md`): `core/src/ical/write.rs` builds the iCalendar text of one CalDAV resource from a UID's single Events, each as managed fields plus 175's tree, reading no server text (ADR 0017). Pure code: the Sender switches to it in 187; the patching writer (`sync/caldav/resource/`) stays until then.

- Managed properties are written as today's writer writes a new `VEVENT` (`sync/caldav/resource/write.rs`), with the unknown parameters the tree holds under their name. Tree entries go back as properties, parameters and components; calendar-level ones outside the `VEVENT`s. Entries whose prefix isn't `ical:` aren't written.
- Fresh on every write: `DTSTAMP`, `LAST-MODIFIED`, the VTIMEZONEs the written times need (from `resource/write.rs`), `VERSION` and `PRODID`. `SEQUENCE` is written back from the tree unchanged; a new Event has none (ticket 126 decides who raises it).
- The writer fixes the order of output: the same input gives the same text.
- **Round trip:** for any managed fields and tree, 175's reader reads the written text back to the same fields and an equal tree.
- **Carried over from `resource/tests.rs`,** which 187 deletes: hostile Event Rows and tree entries (line breaks, `END:VEVENT`, control characters in names and values) can't change the structure of the text; long lines folded between characters; CRLF line ends; the VTIMEZONEs the times need (a flight gets each end's, a UTC time none, offset changes from the year of the first time on).

Spec: "Local Store"; "Untrusted content". ADR 0017. Layout: "The target module tree" (`ical/`); "Today's code, in brief" (Patching writer); "By phase", P1, 176.

**Blocked by:** 175

**Status:** ready-for-agent

**Effort:** M

- [ ] The writer builds a resource for a single Event, a Recurring Event with Exceptions, and Exceptions without their Recurring Event
- [ ] Round trip from fields and tree, over every kind of property value, parameter and component that 175's tests cover
- [ ] Round trip from the server texts of `resource/tests.rs`: read, write, read again gives equal fields and trees
- [ ] The cases carried over from `resource/tests.rs` listed above, as tests in `ical/write.rs`
- [ ] A `SEQUENCE` in the tree is written back unchanged; none for an Event without one
- [ ] The same input gives the same text

## Comments

- 2026-11-28: From the review of 175: the tree's values are as written, escapes and all (`a\,b,c`), so the writer writes them back as they are, without escaping them again; an `ical:DURATION` parameters entry can come from an end read from `DURATION`.
