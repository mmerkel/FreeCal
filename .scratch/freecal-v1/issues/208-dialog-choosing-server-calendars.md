# 208: The dialog that chooses server Calendars to add

**What to build:** The window's part of ADR 0018's adding of Calendars: a dialog lists the Available Calendars, the user ticks the ones to add, and "Add" stores them through 194's `Core::add_calendars` and closes at once. Old ticket 156 (`git show design/snapshot-line:.scratch/freecal-v1/issues/156-dialog-choosing-server-calendars.md`) as written.

- **Opens from** the list after adding an Account (`src/lib/AddAccountDialog.svelte`) and from "Manage Calendars…" (`src/lib/ManageCalendarsDialog.svelte`, `src/lib/Sidebar.svelte`).
- **Shows** one line per Available Calendar with its name and colour (untrusted text from the server) and a checkbox; "Add" and "Cancel".
- **"Add"** stores the chosen Calendars and closes without waiting for their First Syncs; they appear in the sidebar through "Calendars changed", empty, with 202's Syncing mark and "Loading for the first time".
- **Not here:** progress (the Syncing mark's tooltip), a retry (the Sync Error's), cancelling (removing the Calendar).
- **The Window Dialogs** (`src/lib/windowDialogs.svelte.ts`) hold its state as for the others.

Spec: US-5, US-6, US-152; "Adding server Calendars" under the frontend; "Untrusted content". ADR 0018 ("Calendars"). Layout: "By phase", "P5 and P6".

**Blocked by:** 194

**Status:** ready-for-agent

**Effort:** S

- [ ] The dialog opens from both places and lists the Available Calendars
- [ ] "Add" stores the chosen Calendars and closes at once; they show in the sidebar while loading
- [ ] Names from the server are shown as text only
- [ ] Component tests on the fake core; one run in the real window against Radicale (`run-freecal`)
