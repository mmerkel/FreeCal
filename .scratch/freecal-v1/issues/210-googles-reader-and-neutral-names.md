# 210: Google's reader, and the neutral names

**What to build:** The reading half of the Google Provider, as pure code with no network: from the JSON of one Google event resource to an Event Version Content, its managed fields and its Unimplemented Fields (ADR 0017). With a second format in the core, the neutral names are assigned here, as the ADR says. Old ticket 163 (`git show design/snapshot-line:.scratch/freecal-v1/issues/163-googles-reader-and-neutral-names.md`) as written, on 175's tree and reader, 176's writer and 186's prefixes per Provider.

- **Where:** a new `google/` beside `ical/` (`google/read.rs`); this ticket adds `google/` to `redesign-layout.md`'s module tree.
- **One reading** gives both the managed fields and the tree, so a field can't be in both or in neither, as in 175's reader; the result has the CalDAV reader's type. Every field of the JSON is untrusted.
- **Managed fields:** title, times with their zones (ADR 0007), all-day, location, description, the recurrence (`recurrence` holds iCalendar lines and goes through the code that reads them for CalDAV), guests (an Event with attendees is read-only, ticket 121). The UID is `iCalUID`; an Exception is a resource with `recurringEventId`, its recurrence ID `originalStartTime` as obtained. Google's HTML descriptions go through `core/src/structured_text.rs`.
- **Cancelled resources:** a cancelled Exception is a cancelled Occurrence of its Recurring Event; any other cancelled resource is deleted. Ticket 140 decides what FreeCal shows of a status.
- **The tree:** everything else under `google:` names; a structured item that doesn't convert without loss (reminders, attachments, conference data, extended properties) stays whole under its `google:` name. What Google derives or sets fresh on each write (`etag`, `htmlLink`, `kind`) isn't kept, as `DTSTAMP` isn't.
- **Neutral names:** decided here from ADR 0017's candidates (status, show-as, visibility, sequence, created and updated, `RDATE`) and listed in the ADR; a field is neutral only if its meaning and values are the same in both formats. Google's reader fills them, and 175's CalDAV reader and 176's writer read and write them in place of the `ical:` entry. No migration renames stored entries: no real Local Store exists (`redesign-layout.md`, "Tables (end state)").
- **Conformance** (`CODING_STANDARDS.md`, "Conformance"): a Google recurrence ID (`originalStartTime`) is stored in the iCalendar form, converted exactly (a date as a date, a time with `timeZone` as wall time and zone, a time with an offset only as UTC), so the recurrence ID column holds one form and 215's reading applies unchanged; 213's writer converts back. ADR 0012's "as obtained" gets a line saying so. Each neutral name has one value form, valid in both formats; a value not valid in one stays under its format's name. Control characters in managed text are left out as the CalDAV reader leaves them out.
- **Prefixes:** Google holds `google:`, added to 186's function that a move asks.
- **The writer** to Google's JSON is 213's.

Spec: "Local Store"; "Untrusted content"; "Provider interface". ADR 0017 ("The format"). Layout: "By phase", "P5 and P6".

**Blocked by:** 176, 186

**Status:** ready-for-agent

**Effort:** M

- [ ] Google's reader gives managed fields and Unimplemented Fields from one reading of an event resource
- [ ] Everything FreeCal doesn't manage is in the tree under `google:` names or neutral ones
- [ ] The neutral names are decided, listed in ADR 0017, and read and written by the CalDAV reader and writer
- [ ] A move to a Local or CalDAV Calendar drops `google:` entries and keeps neutral ones (186's unit test extended)
- [ ] A Google recurrence ID of each form (date, time with `timeZone`, time with an offset only) is stored in the iCalendar form and passes 216's Store Check; written back by 213 to the same instant
- [ ] Each neutral name is read from both formats and written to both, and a value valid in only one stays under that format's name (a test per neutral name)
- [ ] The hostile test Event and malformed JSON are read without running or rendering anything
- [ ] Unit tests, test-first, with a table of resources (single Event, Recurring Event, Exception, cancelled ones, all-day, each zone form, HTML description, guests, each structured item, unknown fields, the hostile test Event); CalDAV text with each neutral field reads into the tree and is written back as it was, so 187's tests against Radicale still pass
