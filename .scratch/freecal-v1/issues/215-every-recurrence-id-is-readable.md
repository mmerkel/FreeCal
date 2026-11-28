# 215: Every recurrence ID is readable

**What to build:** A recurrence ID FreeCal holds always names the original start of an Occurrence. Only untrusted input can hold one that doesn't read, so the core refuses it where it comes in: a stored field read from the Local Store, and the window's recurrence IDs and Combined Event Keys. This reverses ticket 105's "a stored recurrence ID that can't be read still pairs with nothing": an unreadable recurrence ID follows the rule every other stored time follows, an error and not a crash (`an_unreadable_stored_time_is_an_error_not_a_crash`, `core/tests/events.rs`). Nothing in the Local Store's schema changes.

- `RecurrenceId` (`core/src/event/recurrence_id.rs`) holds the start it names with no "can't be read" case. Reading one from text can fail; only the two edges below read text. `RecurrenceId::same_unreadable`, the last comparison of recurrence IDs as text, goes, and with it the branches where an unreadable one pairs with nothing (`Opened::load`, the Normal Form's `Option`).
- **The Local Store:** a recurrence ID that doesn't read, in an Event Row, a kept Unsent Change or a Journal Event Modification, makes the read fail with the same error as an unreadable stored time (`CoreError::UnreadableEventTime`), so the window needs no new message. Reading an Event or listing a range that holds such a row fails, as for an unreadable start or end; sending a Calendar with such an Unsent Change fails as a Sync Error until the row is fixed.
- **The window:** a Combined Event Key whose recurrence ID doesn't read is refused as a malformed key is today; a recurrence ID the window passes for an Occurrence that doesn't read answers as an unknown Occurrence does today.
- **Files and servers:** already give only readable recurrence IDs (the reader writes the text from the start it parsed, `RecurrenceId::of_original`; an unknown zone gives a time with no zone). Check what the reader does with a `RECURRENCE-ID` the iCalendar library can't parse at all: if `get_recurrence_id()` answers none, the Exception comes in as a second Event with its Recurring Event's UID. If so, such a `VEVENT` is skipped as an import skips other unreadable Events, and an Incoming Sync handles its resource as one it can't read (an Ignored Resource).

Spec: "Untrusted content". ADR 0012. Tickets 88, 105, 174 (its review: the text comparison of keys).

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `RecurrenceId` can't hold a recurrence ID that doesn't read; `same_unreadable` is gone
- [x] A stored recurrence ID that doesn't read is an error on reading the Event, listing its range and sending its Calendar; `an_exception_with_an_unreadable_recurrence_id_is_found_by_its_key` (`core/tests/recurrence_scopes.rs`) becomes a test of that
- [x] The window's Combined Event Keys and recurrence IDs that don't read are refused, with tests
- [x] A file or server `RECURRENCE-ID` that doesn't parse doesn't make a second Event of its UID, with a test

## Comments

- 2026-11-28: From the review of 174. Strongest con: one bad row fails a whole range listing, so the grid shows a read failure for those weeks instead of one stray Event; today such an Exception still shows, as a single Event.
- 2026-11-28: Started from commit 5d690b5.
  - **Decision, for review: a `RECURRENCE-ID` the iCalendar library can't parse makes the whole file or resource unreadable, as an unreadable `DTSTART` does, rather than skipping its `VEVENT`.** The ticket assumed an import skips unreadable Events; it doesn't: `read_events` (`core/src/import/mod.rs`) fails the file (`ImportNotCalendar`), and an Incoming Sync then handles the resource as one it can't read (logged, an Ignored Resource). Skipping just the `VEVENT` would also drop it from the Server Object when FreeCal writes the resource back. Strongest con: one bad Exception keeps a whole file out of an import.
  - **Decision, for review: sending fails on an Exception's Event Row whose recurrence ID doesn't read, not on the Unsent Change's.** The Sender works per UID and reads the Event Rows (`unsent_uids`), never an Unsent Change's recurrence ID; that one is read only for "Not yet synced", so it fails reading the Event and listing its range. Strongest con: a damaged Unsent Change alone doesn't stop a send, so the grid errors while sync stays green.
  - Built: `RecurrenceId` holds `original: Time` with no "can't be read" case; `RecurrenceId::read` (the window's and the Local Store's text) answers none for text that doesn't read, and `same_unreadable` is gone. The Local Store reads a recurrence ID through `FromSql` (`core/src/store/sql/mod.rs`), whose failure `UnreadableStoredTime` becomes `CoreError::UnreadableEventTime` in the one `From<rusqlite::Error>` (`core/src/error.rs`); this covers Event Rows, kept Unsent Changes and Journal Event Modifications alike. A Combined Event Key whose recurrence ID doesn't read fails to deserialize. The loaded Exceptions' Normal Form is no longer an `Option`, and `NormalForm::to_recurrence_id` fails with the unreadable-time error rather than storing text that doesn't read.
  - Tests: `an_exception_with_an_unreadable_recurrence_id_is_an_error_not_a_crash` (renamed from `..._is_found_by_its_key`) and `an_unreadable_recurrence_id_in_the_change_journal_is_an_error_not_a_crash` (`core/tests/recurrence_scopes.rs`); `an_exception_with_an_unreadable_recurrence_id_is_a_sync_error` and `an_unsent_change_with_an_unreadable_recurrence_id_is_an_error_not_a_crash` (`core/tests/caldav_send.rs`); `a_combined_event_key_whose_recurrence_id_does_not_read_is_refused` (`core/src/store/mod.rs`); an unknown zone added to `a_recurrence_id_the_recurring_event_doesnt_have_is_refused`; `a_file_with_a_recurrence_id_freecal_cannot_read_is_not_a_calendar` (`core/tests/import.rs`); `a_resource_with_a_recurrence_id_freecal_cannot_read_is_not_read` (`core/tests/caldav_sync.rs`).
  - New term: **Unreadable Stored Time** (`UnreadableStoredTime`), a stored time the Local Store fails to read a row on, which becomes `CoreError::UnreadableEventTime`.
