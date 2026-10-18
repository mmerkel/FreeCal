# 75: Changes to an Occurrence and all following ones (`RANGE=THISANDFUTURE`)

**What to build:** An Exception from a file whose `RECURRENCE-ID` has `RANGE=THISANDFUTURE` changes its Occurrence and every later one (RFC 5545, 3.2.13 and 3.8.4.4). Today it changes only its own Occurrence, so every later one shows the old title, place or time.

**Blocked by:** 08

**Status:** ready-for-agent

**Effort:** L

- [ ] Import keeps the range on the Exception, stored as its own column; the Change Journal's hash covers it (a new format, re-hashed by the migration as ticket 07 did). FreeCal keeps the Exception as it came instead of splitting the Recurring Event, so export (ticket 09) and CalDAV (tickets 54, 17) can write back what the file or server holds, under the same UID
- [ ] Each Occurrence from the Exception's on takes the Exception's fields (title, place, description); its start and end move by the same amount the Exception moved its own, in wall time in the Event Time Zone (ADR 0007)
- [ ] With several such Exceptions, each later Occurrence follows the latest one before it; an Exception without a range still changes only its own Occurrence, and wins over a range
- [ ] Cancelled Occurrences (`exdates`) stay cancelled after a range; an Occurrence a range moves keeps its recurrence ID, so it can still be deleted and changed one by one
- [ ] The derived bounds (`start_utc`/`end_utc`) of the Recurring Event include the moved Occurrences
- [ ] `RANGE=THISANDPRIOR` (RFC 2445 only) is read as no range, and so is any other value: the range is untrusted like every stored field
- [ ] The details of an Occurrence a range changed show its changed fields. Ticket 08's scopes work on it: "this event" changes it alone, "this and following" splits the Recurring Event there (ranges after the split go with the new Recurring Event, which gets their changes as its own fields and times), and "all" changes the Recurring Event and keeps its ranges where possible, as ticket 08 keeps Exceptions
- [ ] Core tests for every point above, including a range that moves Occurrences across a DST change

Out of scope: writing the range on export (ticket 09); the user's own "this and following" (ticket 08, which splits the Recurring Event).

## Comments

- 2026-10-18: Found while checking which parts of RFC 5545's recurrence FreeCal reads (after ticket 07). The range is kept rather than turned into a split, because a split gives the later Occurrences a new UID that the file or server doesn't know, so a later import update or pull would duplicate them.
- 2026-10-18: Blocked by 08 so that its scopes are decided once, together with Occurrences a range changed. Low priority: Google Calendar and Apple Calendar reportedly split a Recurring Event for "this and following" instead of writing a range (https://github.com/karora/aCal/issues/59), and until this ticket FreeCal shows a range as a change to its one Occurrence, as many apps do. Before starting, check whether the apps users sync with write ranges at all: make a "this and following" change in Google Calendar, Apple Calendar and Thunderbird and look for `RANGE=` in each export. If none does, shrink this ticket to keeping the range for export and CalDAV only.
- 2026-10-18: Splitting on import instead was weighed and rejected: it can't work for CalDAV without writing to the server during a pull, and for files a re-import would show the new part twice, because its UID isn't in the file. Cost is no reason to split: each range is one row, and applying the ranges while walking the starts adds little to a walk FreeCal already makes from the first start.
