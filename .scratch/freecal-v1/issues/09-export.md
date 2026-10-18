# 09: Export to .ics and .zip

**What to build:** The user can export any Calendar to an `.ics` file, or export everything to a `.zip` with one `.ics` per Calendar organised by Account. While the export runs, a modal "Working…" dialog blocks the window.

**Blocked by:** 03

**Status:** ready-for-agent

**Effort:** M

- [ ] Export one Calendar to a single `.ics`
- [ ] Export everything to a `.zip`, one `.ics` per Calendar, organised by Account
- [ ] The file dialog goes through a portal-friendly API
- [ ] Long-running commands show a modal "Working…" dialog that blocks the window until they finish
- [ ] Exported data round-trips: re-importing yields the same Events (verified once 57 exists, or by parsing in the test)
- [ ] The `.ics` is written through a library writer, never built from strings; the hostile test Event round-trips without injecting properties (`\r\nATTENDEE:…` stays inside its field)
- [ ] `.zip` entry names made from Account and Calendar names are sanitized: a Calendar named `../../.bashrc` stays in its folder
- 2026-10-17 (from ticket 59): The modal "Working…" dialog exists: `src/lib/WorkingDialog.svelte`, shown by `App.svelte`'s `working` state. The file dialog goes through tauri-plugin-dialog's xdg-portal backend, opened by the shell so the window never sees a path (see `pick_import_file` in `main.rs`).

## Comments
- 2026-10-18 (from ticket 07): A kept rule can have an `UNTIL` in the other form after an Event switches between all-day and timed (the core reads both); write it in the form RFC 5545 asks for the Event as exported. Exceptions are Events with the UID of their Recurring Event and a `recurrence_id`; cancelled Occurrences are `exdates`.

