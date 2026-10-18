# 78: Add and remove a CalDAV Account's Calendars in the core

**What to build:** The core can add an Available Calendar of a CalDAV Account as a Calendar, and remove it again. Removing deletes only the local copy and makes it an Available Calendar again. Pulling the added Calendar's Events is ticket 54. Spec: US-15. Split from ticket 53.

**Blocked by:** 77

**Status:** done

**Effort:** S

- [x] Adding an Available Calendar makes it a Calendar of the Account; it no longer lists as Available
- [x] Removing a Calendar deletes only its local copy, never touches the server, and makes it an Available Calendar again
- [x] `delete_calendar` gets a Provider check for server Calendars like the one in `create_calendar` (ticket 02's note)

Out of scope: pulling Events (ticket 54).

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 77): `Core::available_calendars` returns every Calendar on the server; leaving out the ones already added is this ticket's. An `AvailableCalendar` is named by its `url`.
- 2026-10-18: Done. `Core::add_calendars(account_id, urls)` adds Available Calendars by their `url`, all or none, after asking the server for its current list, so only URLs the server offers can ever become Calendars (ticket 54 will send the password to them). Name and colour are the server's at that moment; a Calendar without a colour gets the default grey. `Core::remove_calendar(id)` deletes a server Calendar's local copy and Events like `delete_calendar` and never contacts the server. `available_calendars` now leaves out added Calendars. Choices:
  - Delete and remove are separate operations, as the spec's interface list has them: `delete_calendar` refuses a server Calendar with the new code `notLocalCalendar`, and `remove_calendar` refuses a Local one with `notServerCalendar`. Both decide with an exhaustive `match` on the Provider, so Google (ticket 25) has to choose.
  - A URL that the server doesn't offer, that is added already, or that comes twice is refused with `calendarNotAvailable`. So is any URL for the Local Account.
  - The Local Store records a server Calendar's `url` (new column, unique per Account). Local Calendars have none.
  - Added Calendars are writable for now; starting read-only before 1.0 is ticket 56's.
  - Tests: `core/tests/caldav_calendars.rs`, including the hostile `calendarName` stored unchanged.
