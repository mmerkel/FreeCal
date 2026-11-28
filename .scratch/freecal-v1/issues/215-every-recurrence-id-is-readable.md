# 215: Every recurrence ID is readable

**What to build:** A recurrence ID FreeCal holds always names the original start of an Occurrence. Only untrusted input can hold one that doesn't read, so the core refuses it where it comes in: a stored field read from the Local Store, and the window's recurrence IDs and Combined Event Keys. This reverses ticket 105's "a stored recurrence ID that can't be read still pairs with nothing": an unreadable recurrence ID follows the rule every other stored time follows, an error and not a crash (`an_unreadable_stored_time_is_an_error_not_a_crash`, `core/tests/events.rs`). Nothing in the Local Store's schema changes.

- `RecurrenceId` (`core/src/event/recurrence_id.rs`) holds the start it names with no "can't be read" case. Reading one from text can fail; only the two edges below read text. `RecurrenceId::same_unreadable`, the last comparison of recurrence IDs as text, goes, and with it the branches where an unreadable one pairs with nothing (`Opened::load`, the Normal Form's `Option`).
- **The Local Store:** a recurrence ID that doesn't read, in an Event Row, a kept Unsent Change or a Journal Event Modification, makes the read fail with the same error as an unreadable stored time (`CoreError::UnreadableEventTime`), so the window needs no new message. Reading an Event or listing a range that holds such a row fails, as for an unreadable start or end; sending a Calendar with such an Unsent Change fails as a Sync Error until the row is fixed.
- **The window:** a Combined Event Key whose recurrence ID doesn't read is refused as a malformed key is today; a recurrence ID the window passes for an Occurrence that doesn't read answers as an unknown Occurrence does today.
- **Files and servers:** already give only readable recurrence IDs (the reader writes the text from the start it parsed, `RecurrenceId::of_original`; an unknown zone gives a time with no zone). Check what the reader does with a `RECURRENCE-ID` the iCalendar library can't parse at all: if `get_recurrence_id()` answers none, the Exception comes in as a second Event with its Recurring Event's UID. If so, such a `VEVENT` is skipped as an import skips other unreadable Events, and an Incoming Sync handles its resource as one it can't read (an Ignored Resource).

Spec: "Untrusted content". ADR 0012. Tickets 88, 105, 174 (its review: the text comparison of keys).

**Blocked by:** none

**Status:** needs-triage

**Effort:** M

- [ ] `RecurrenceId` can't hold a recurrence ID that doesn't read; `same_unreadable` is gone
- [ ] A stored recurrence ID that doesn't read is an error on reading the Event, listing its range and sending its Calendar; `an_exception_with_an_unreadable_recurrence_id_is_found_by_its_key` (`core/tests/recurrence_scopes.rs`) becomes a test of that
- [ ] The window's Combined Event Keys and recurrence IDs that don't read are refused, with tests
- [ ] A file or server `RECURRENCE-ID` that doesn't parse doesn't make a second Event of its UID, with a test

## Comments

- 2026-11-28: From the review of 174. Strongest con: one bad row fails a whole range listing, so the grid shows a read failure for those weeks instead of one stray Event; today such an Exception still shows, as a single Event.
