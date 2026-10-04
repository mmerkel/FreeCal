# 02: Local Calendars

**What to build:** The user can create, rename, recolour and delete Calendars in the Local Account, and show or hide each Calendar from a sidebar where Calendars are grouped by Account.

**Blocked by:** 01

**Status:** done

- [x] Create, rename, recolour and delete a Local Calendar through the core interface and the sidebar
- [x] Each Calendar has a colour
- [x] Calendars are grouped by Account in the sidebar and can be shown or hidden individually
- [x] The core sends "Calendars changed" on every add, remove, rename or recolour, and the sidebar refreshes on it
- [x] The frontend subscribes to Signals before reading the initial state

## Comments

Implemented. Notes for later tickets:

- Core: `list_calendars`, `create_calendar`, `rename_calendar`, `recolour_calendar`, `delete_calendar`, `set_calendar_shown`. Names are trimmed and must not be empty; colours are `#rrggbb` (`Colour::parse`), and a stored colour that isn't one reads back as the default grey. Tests: `core/tests/local_calendars.rs`.
- "Calendars changed" is also sent when a Calendar is shown or hidden (the spec now says so), so the frontend never changes its own copy: it calls the core and re-reads on the Signal. Ticket 03 can re-fetch Occurrences on the same Signal when the set of shown Calendars changes.
- Whether a Calendar is shown is stored in the Local Store (`calendar.shown`), so it survives a relaunch.
- `calendar.account_id` references `account` with `ON DELETE CASCADE`, and `foreign_keys` is on. Ticket 03's Events table should reference `calendar` the same way, so deleting a Calendar deletes its Events. Ticket 14 should add a Provider check to `delete_calendar` ("remove from FreeCal" for server Calendars) like the one in `create_calendar`.
- First launch creates no Calendar: the Local Account starts empty, with a "New calendar" button. Ticket 03 has to handle the case where no writable Calendar exists yet.
- Core errors still reach the UI as English text (`sidebar.changeFailed`, `app.readFailed`) until ticket 29 turns `CoreError` into codes.
- Checked by hand in the real window (debug build on Xvfb): create, rename, Escape, recolour through the GTK colour picker, hide, delete with Cancel and confirm, a blank name refused by the core, and everything kept across a relaunch.
