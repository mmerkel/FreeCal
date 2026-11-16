# 156: The modal dialog that adds server Calendars and shows their First Sync

**What to build:** The window's part of ADR 0018's First Sync: adding Available Calendars opens a dialog that follows each chosen Calendar's First Sync through the "First Sync progress" Signal of ticket 155.

- **When it opens:** when the user adds Available Calendars, from the list after adding an Account (`src/lib/AddAccountDialog.svelte`) and from "Manage Calendars…" (`src/lib/ManageCalendarsDialog.svelte`, `src/lib/Sidebar.svelte`). It replaces the wait on the whole operation that ticket 155 left.
- **What it shows:** one line per chosen Calendar with its name and colour (untrusted text from the server), and its state: waiting, loading with its progress (the numbers the Provider counts, as a bar and as text), finished, cancelled, or its Sync Error's message with "Retry".
- **A finished Calendar** appears in the sidebar at once, through "Calendars changed", behind the dialog, while the others still load.
- **Cancel** stops those still running and keeps the finished ones; the dialog says which were added. When all have finished it offers "Close" instead, and closes by itself if none had a Sync Error.
- **Retry** adds that Calendar again; its line starts from the beginning.
- **It locks the window:** it is a modal `Dialog` like the others, without a ✕ while any First Sync runs, so that Cancel is the one way out; Escape does what Cancel does. The Window Dialogs' rule for which dialog may open over which covers it: a waiting import and the dialog for a Calendar that stopped Tracking (ticket 162) wait until it closes.
- **Closing the window to the tray** keeps the First Syncs running, and the dialog is there when the window is shown again, as the window is hidden and not destroyed (ADR 0016). Quit from the tray ends them without a question: nothing but loading time is lost, and the Calendars are Available Calendars again.
- **The Window Dialogs** (`src/lib/windowDialogs.svelte.ts`) hold the dialog's state; the fake core of the frontend tests gains the Signal.

Spec: US-5, US-6, US-152; "Adding server Calendars" under the frontend; "Signals". ADR 0018 ("The pointers").

**Blocked by:** 155

**Status:** ready-for-agent

**Effort:** S

- [ ] Adding Available Calendars opens the dialog, from both places
- [ ] Each line follows "First Sync progress": waiting, loading with progress, finished, cancelled, Sync Error with "Retry"
- [ ] A finished Calendar is in the sidebar while others still load
- [ ] Cancel keeps the finished Calendars and stops the rest; "Retry" starts one again
- [ ] The dialog is modal: nothing behind it can be used, and no other dialog opens over it
- [ ] Hiding the window to the tray and showing it again finds the dialog in its current state
- [ ] Names from the server are shown as text only (see "Untrusted content")
- [ ] Component tests on the fake core cover the states, Cancel and Retry; one run in the real window against Radicale (`run-freecal`)

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of two parts (see ticket 155).
- 2026-11-16: Decided by the user: the dialog is modal and locks the window (ADR 0018 and the spec changed with it), so the ticket is S. Its con, accepted: a large Calendar on a slow server locks the window for as long as it loads. No read of the running First Syncs is needed, as the window that started them is the one that shows them.
