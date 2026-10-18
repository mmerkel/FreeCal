# 55: Add a CalDAV Account and manage its Calendars in the window

**What to build:** The user adds a CalDAV Account with a server URL, username and password, chooses which of its Available Calendars to add, and sees them in the sidebar under the Account. The Account's line offers "Manage Calendars…" to add or remove Calendars later, and the user can remove the whole Account. Spec: US-15. Split from ticket 14.

**Blocked by:** 79, 80

**Status:** done

**Effort:** M

- [x] An "Add account…" flow: server URL, username and password; errors from the core (wrong credentials, no keyring, unreachable server) shown translated, with server text only as text
- [x] After adding, the user chooses which Available Calendars to add
- [x] Calendars are grouped by Account in the sidebar
- [x] "Manage Calendars…" on the Account line fetches the server's current list; the user can add Available Calendars or remove Calendars ("remove from FreeCal", which never touches the server)
- [x] The user can remove the whole Account, after a confirmation
- [x] The sidebar shows "Loading for the first time…" on a Calendar during its first sync
- [x] Calendar names from the server are shown only as text; the hostile test Event pulled from Radicale is shown without rendering anything
- [x] Every string goes through `t()`
- [x] Component tests against the fake core; checked by hand in the real app with `run-freecal` against a local Radicale

## Comments

- 2026-10-13: Split from ticket 14.
- 2026-10-18 (from 78): the core has `add_calendars(account_id, urls)` (several at once, all or none) and `remove_calendar(id)` for server Calendars; `delete_calendar` now refuses them (`notLocalCalendar`), so the sidebar must offer "Remove from FreeCal" instead of Delete for them. Neither is a shell command yet. `calendarName` from the hostile fixture still needs its `expected` entry here.
- 2026-10-18 (from 80): `add_calendars` runs each new Calendar's first sync before it returns, so its shell command must run off the UI thread (an async Tauri command) for the sidebar to show "Loading for the first time…" meanwhile. The marker is `calendar.firstSync`, re-read on "Calendars changed", sent as each first sync starts and ends; Events arrive with one "Occurrences changed" per page.
- 2026-10-18: Done. "Add account…" sits under the last Account in the sidebar; its dialog (`AddAccountDialog.svelte`) asks for server address, username and password and shows the core's error codes in words, can't be cancelled while the server is asked (the core may still add the Account). Choices:
  - One dialog, `ManageCalendarsDialog.svelte`, serves both the chooser after adding (what adding found, all checked) and "Manage calendars…" (asks the server each time): one checklist where checked means in FreeCal. Save removes the unchecked Calendars, closes, then adds the checked Available ones; the sidebar shows "Loading for the first time…" on each during its first sync and reports a failed add or first sync. A note says how many Calendars unchecking removes. When the server can't be reached, the Account's Calendars stay listed so they can still be removed.
  - Server Accounts get a ⋮ on their heading with "Manage calendars…" and "Remove account…" (confirmation). A server Calendar's own menu has "Remove from FreeCal" (confirmation) instead of Delete.
  - New shell commands: `add_caldav_account`, `available_calendars`, `add_calendars`, `remove_account` run off the UI thread (`off_the_ui_thread` in `main.rs`); `remove_calendar` is a plain command.
  - The hostile fixture's `calendarName` got its `expected.removeCalendarTitle`; Calendar names in new strings are bidi-isolated.
  - Checked by hand with `run-freecal` against a local Radicale and a private gnome-keyring on its own D-Bus (how to is now in the skill): wrong and right password, chooser with the hostile Calendar name, the hostile Event pulled from Radicale shown as text, add and remove via Manage, Remove from FreeCal, the loading marker (needs a Calendar of a few hundred Events to be visible), and removing the Account, which emptied its keyring entry. The loading text now truncates instead of wrapping; that last CSS fix wasn't re-checked in the window.
