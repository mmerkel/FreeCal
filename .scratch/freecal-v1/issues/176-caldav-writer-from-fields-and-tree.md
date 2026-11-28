# 176: The CalDAV writer builds a resource from managed fields and the tree

**What to build:** Old ticket 144 as written (`git show design/snapshot-line:.scratch/freecal-v1/issues/144-caldav-writer-from-fields-and-tree.md`): `core/src/ical/write.rs` builds the iCalendar text of one CalDAV resource from a UID's single Events, each as managed fields plus 175's tree, reading no server text (ADR 0017). Pure code: the Sender switches to it in 187; the patching writer (`sync/caldav/resource/`) stays until then.

- Managed properties are written as today's writer writes a new `VEVENT` (`sync/caldav/resource/write.rs`), with the unknown parameters the tree holds under their name. Tree entries go back as properties, parameters and components; calendar-level ones outside the `VEVENT`s. Entries whose prefix isn't `ical:` aren't written.
- Fresh on every write: `DTSTAMP`, `LAST-MODIFIED`, the VTIMEZONEs the written times need (from `resource/write.rs`), `VERSION` and `PRODID`. `SEQUENCE` is written back from the tree unchanged; a new Event has none (ticket 126 decides who raises it).
- The writer fixes the order of output: the same input gives the same text.
- **Round trip:** for any managed fields and tree, 175's reader reads the written text back to the same fields and an equal tree.
- **Carried over from `resource/tests.rs`,** which 187 deletes: hostile Event Rows and tree entries (line breaks, `END:VEVENT`, control characters in names and values) can't change the structure of the text; long lines folded between characters; CRLF line ends; the VTIMEZONEs the times need (a flight gets each end's, a UTC time none, offset changes from the year of the first time on).

Spec: "Local Store"; "Untrusted content". ADR 0017. Layout: "The target module tree" (`ical/`); "Today's code, in brief" (Patching writer); "By phase", P1, 176.

**Blocked by:** 175

**Status:** done

**Effort:** M

- [x] The writer builds a resource for a single Event, a Recurring Event with Exceptions, and Exceptions without their Recurring Event
- [x] Round trip from fields and tree, over every kind of property value, parameter and component that 175's tests cover
- [x] Round trip from the server texts of `resource/tests.rs`: read, write, read again gives equal fields and trees
- [x] The cases carried over from `resource/tests.rs` listed above, as tests in `ical/write.rs`
- [x] A `SEQUENCE` in the tree is written back unchanged; none for an Event without one
- [x] The same input gives the same text

## Comments

- 2026-11-28: From the review of 175: the tree's values are as written, escapes and all (`a\,b,c`), so the writer writes them back as they are, without escaping them again; an `ical:DURATION` parameters entry can come from an end read from `DURATION`.
- 2026-11-28: Started from commit efde2e5.
- 2026-11-28: Built `ical/write/mod.rs` (`write_resource`) with its tests in `write/tests.rs`; the patching writer's VTIMEZONE moved there (`vtimezone_text`), and its VTIMEZONE test with it. No switched-off tests; no new terms. Decisions, for review, each with its strongest con:
  1. **decision, for review:** ~~an end read from `DURATION` whose tree keeps an `ical:DURATION` entry of parameters is written as `DURATION` again~~ Changed with the user, 2026-11-28: `DURATION` is fully managed. The reader keeps no `ical:DURATION` entry (not its parameters, not a second one, one it can't read, or one beside a `DTEND`, which stays an RFC violation in the Log), and the writer always writes `DTEND`. Con: an exception to the rule that a managed property's unknown parameters are kept; the user expects more such exceptions.
  2. **decision, for review:** a managed text that is empty but has parameters in the tree is written as an empty property (`SUMMARY;LANGUAGE=de:`), so the parameters read back. Con: an empty line the user didn't write.
  3. **decision, for review:** a `TZID` in the tree's properties that names a zone FreeCal knows gets a `VTIMEZONE` under that `TZID`'s text, covering from the year of the Event's start. Con: a `TZID` FreeCal doesn't know still loses its definition (the reader drops every `VTIMEZONE`), and the coverage follows the Event's start, not that property's time. Extended with the user, 2026-11-28: the VTIMEZONE covers from the earliest of the Event's start and the property's own times; the reader keeps the VTIMEZONE of a TZID FreeCal doesn't know, which the Event's tree uses, as an `ical:VTIMEZONE` entry on each such Event, and the writer writes it once at the calendar level (option B; option C, matching an unknown VTIMEZONE to a known zone, may come later).
  4. **decision, for review:** the calendar level is written from the `ical:VCALENDAR` entries of the Event the reader puts them on (the first in the writer's order); several such entries (a text with several `VCALENDAR`s) go into the one `VCALENDAR` and read back as one entry. Con: the round trip is not exact for those texts. Kept by the user, 2026-11-28: a CalDAV resource holds one VCALENDAR (RFC 4791), and the case needs a single-UID `.ics` file split over several.
  5. **decision, for review:** what the tree can't hold safely isn't written: a name other than letters, digits and dashes, a property `BEGIN` or `END`, a calendar-level `VEVENT` or `VTIMEZONE`, and the tree's `DTSTAMP`, `LAST-MODIFIED`, `VERSION` and `PRODID`; parameter values lose `"` and control characters, values their control characters. Con: such an entry is lost without a trace (the reader never makes one). Changed with the user, 2026-11-28: nothing is lost silently. The reader leaves out control characters (but tab and line ends) and what has a name that isn't one, and reports each as an RFC violation (`ControlCharacter`, `NotAName`), so the tree holds only what can be written. The writer keeps its checks, as the Local Store is untrusted, and returns what it leaves out (`Written::left_out`, `LeftOut`) for ticket 187 to write to the Log.
  6. **decision, for review:** the round trip compares recurrence IDs by Occurrence (ADR 0012): a floating one of a timed Recurring Event is written in its Event Time Zone, as the patching writer does. `zone_pinned` is not compared, as no text holds it. Con: "the same fields" is by Occurrence there, not as obtained.
  7. **decision, for review:** several `ical:EXDATE` entries of parameters go on the one `EXDATE` line and read back as one entry. Con: not exact for that rare text.
  - Trade-off: the patching writer keeps its own `Group::write` until 187 rather than calling the new writer's, which shares only the `VTIMEZONE`; sharing more would rework code 187 deletes.
