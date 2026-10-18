# 78: Add and remove a CalDAV Account's Calendars in the core

**What to build:** The core can add an Available Calendar of a CalDAV Account as a Calendar, and remove it again. Removing deletes only the local copy and makes it an Available Calendar again. Pulling the added Calendar's Events is ticket 54. Spec: US-15. Split from ticket 53.

**Blocked by:** 77

**Status:** ready-for-agent

**Effort:** S

- [ ] Adding an Available Calendar makes it a Calendar of the Account; it no longer lists as Available
- [ ] Removing a Calendar deletes only its local copy, never touches the server, and makes it an Available Calendar again
- [ ] `delete_calendar` gets a Provider check for server Calendars like the one in `create_calendar` (ticket 02's note)

Out of scope: pulling Events (ticket 54).

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 77): `Core::available_calendars` returns every Calendar on the server; leaving out the ones already added is this ticket's. An `AvailableCalendar` is named by its `url`.
