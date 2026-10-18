# 55: Add a CalDAV Account and manage its Calendars in the window

**What to build:** The user adds a CalDAV Account with a server URL, username and password, chooses which of its Available Calendars to add, and sees them in the sidebar under the Account. The Account's line offers "Manage Calendars…" to add or remove Calendars later, and the user can remove the whole Account. Spec: US-15. Split from ticket 14.

**Blocked by:** 79, 80

**Status:** ready-for-agent

**Effort:** M

- [ ] An "Add account…" flow: server URL, username and password; errors from the core (wrong credentials, no keyring, unreachable server) shown translated, with server text only as text
- [ ] After adding, the user chooses which Available Calendars to add
- [ ] Calendars are grouped by Account in the sidebar
- [ ] "Manage Calendars…" on the Account line fetches the server's current list; the user can add Available Calendars or remove Calendars ("remove from FreeCal", which never touches the server)
- [ ] The user can remove the whole Account, after a confirmation
- [ ] The sidebar shows "Loading for the first time…" on a Calendar during its first sync
- [ ] Calendar names from the server are shown only as text; the hostile test Event pulled from Radicale is shown without rendering anything
- [ ] Every string goes through `t()`
- [ ] Component tests against the fake core; checked by hand in the real app with `run-freecal` against a local Radicale

## Comments

- 2026-10-13: Split from ticket 14.
- 2026-10-18 (from 78): the core has `add_calendars(account_id, urls)` (several at once, all or none) and `remove_calendar(id)` for server Calendars; `delete_calendar` now refuses them (`notLocalCalendar`), so the sidebar must offer "Remove from FreeCal" instead of Delete for them. Neither is a shell command yet. `calendarName` from the hostile fixture still needs its `expected` entry here.
