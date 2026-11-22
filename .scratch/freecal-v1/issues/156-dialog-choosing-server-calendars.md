# 156: The dialog that chooses server Calendars to add

**What to build:** The window's part of ADR 0018's adding of Calendars: a dialog lists the Available Calendars, the user ticks the ones to add, and "Add" stores them through ticket 155 and closes at once. Their loading is shown in the sidebar (ticket 161), not here.

- **When it opens:** from the list after adding an Account (`src/lib/AddAccountDialog.svelte`) and from "Manage Calendars…" (`src/lib/ManageCalendarsDialog.svelte`, `src/lib/Sidebar.svelte`).
- **What it shows:** one line per Available Calendar with its name and colour (untrusted text from the server) and a checkbox; "Add" and "Cancel".
- **"Add"** stores the chosen Calendars and closes the dialog without waiting for their First Syncs; they appear in the sidebar through "Calendars changed", empty, with the Syncing mark.
- **No progress, Cancel of a loading Calendar or Retry here:** progress is the Syncing mark's tooltip, a retry the Sync Error's own, and cancelling is "Remove from FreeCal" on the Calendar's line.
- **The Window Dialogs** (`src/lib/windowDialogs.svelte.ts`) hold the dialog's state as for the others.

Spec: US-5, US-6, US-152; "Adding server Calendars" under the frontend. ADR 0018 ("Calendars").

**Blocked by:** 155

**Status:** ready-for-agent

**Effort:** S

- [ ] The dialog opens from both places and lists the Available Calendars
- [ ] "Add" stores the chosen Calendars and closes at once; they show in the sidebar while loading
- [ ] Names from the server are shown as text only (see "Untrusted content")
- [ ] Component tests on the fake core; one run in the real window against Radicale (`run-freecal`)

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of two parts (see ticket 155).
- 2026-11-16: Decided by the user: the dialog is modal and locks the window (ADR 0018 and the spec changed with it), so the ticket is S. Its con, accepted: a large Calendar on a slow server locks the window for as long as it loads. No read of the running First Syncs is needed, as the window that started them is the one that shows them.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the dialog for a Calendar that stopped Tracking is gone (ticket 162).
- 2026-11-22: Rewritten and renamed (was `156-dialog-adding-server-calendars`) from the grilling on point 4 of `.scratch/freecal-v1/critiques.md` (ADR 0018): the dialog only chooses; the First Syncs run as ordinary Incoming Syncs and show in the sidebar, so the modal dialog, its progress, Cancel and Retry go. The comment of 2026-11-16 above is history.
