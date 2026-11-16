# 163: Google's reader: an event resource becomes managed fields and Unimplemented Fields; neutral names are assigned

**What to build:** The reading half of the Google Provider, as pure code with no network: from the JSON of one Google event resource to the managed fields and the Unimplemented Fields of an Event Version (ADR 0017). With a second format in the core, the neutral names are assigned here, as the ADR says. First of three parts of ticket 25 (163 the reader, 164 the Account, 25 the Calendars and their sync).

- **The reader:** one reading of a resource gives both the managed fields and the tree, so a field can't be in both or in neither, as in ticket 143's reader. Its result has the type the CalDAV reader's has. Every field of the JSON is untrusted.
- **Managed fields:** title, times with their zones (ADR 0007), all-day, location, description, the recurrence (`recurrence` holds iCalendar lines and goes through the code that reads them for CalDAV), and guests (an Event with attendees is read-only, ticket 121). The UID is `iCalUID`; an Exception is a resource with `recurringEventId`, and its recurrence ID is `originalStartTime` as obtained.
- **Descriptions:** Google's HTML goes through the core's conversion to plain text and link pieces (`core/src/structured_text.rs`).
- **Cancelled resources** (chosen here, for review): a cancelled Exception is read as a cancelled Occurrence of its Recurring Event; any other cancelled resource as deleted. Ticket 140 decides what FreeCal shows of a status.
- **The tree:** everything else in the resource, under `google:` names. A structured item that doesn't convert without loss (reminders, attachments, conference data, extended properties) stays whole under its `google:` name. What Google derives or sets fresh on each write (`etag`, `htmlLink`, `kind`) isn't kept, as `DTSTAMP` isn't.
- **Neutral names:** this ticket decides which fields are neutral, starting from ADR 0017's candidates (status, show-as, visibility, sequence, created and updated, `RDATE`), and adds the list to the ADR. A field is neutral only if its meaning and its values are the same in both formats. For each neutral name:
  - Google's reader fills it;
  - the CalDAV reader and writer (tickets 143 and 144) read and write it instead of the `ical:` entry;
  - a migration, in Rust, renames the entries in every Event Version; what doesn't convert stays under its `ical:` name.
  The writer to Google's JSON is ticket 26.
- **Prefixes a Provider holds:** Google holds `google:`, added to the function of ticket 149 that a move asks.
- **Tests:** a table of resources (single Event, Recurring Event, Exception, cancelled ones, all-day, each zone form, HTML description, guests, each structured item, unknown fields, the hostile test Event); CalDAV text with each neutral field reads into the tree and is written back as it was, so ticket 150's tests against Radicale still pass; the migration on a Local Store holding `ical:` entries.

Spec: "Local Store"; "Untrusted content"; "Provider interface". ADR 0017 ("The format").

**Blocked by:** 144, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] Google's reader gives managed fields and Unimplemented Fields from one reading of an event resource
- [ ] Everything FreeCal doesn't manage is in the tree under `google:` names or neutral ones
- [ ] The neutral names are decided, listed in ADR 0017, and read and written by the CalDAV reader and writer; the migration renames stored entries
- [ ] The hostile test Event and malformed JSON are read without running or rendering anything
- [ ] Written test-first; `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` if an interface type changes

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Ticket 25 was re-sized to XL and split under the M cap into 163 (this reader), 164 (the Account) and 25 (the Calendars and their sync). The triage listed two parts; the Account's login and the Calendars' sync together were above M, so there are three.
