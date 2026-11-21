# 159: The Calendar History window, with "Undo" on each line

**What to build:** The view of the Calendar History (US-83, US-155) and the way to undo any one Snapshot from it, on the core's Undo (ticket 147). It takes the place of the Change Journal dialog that ticket 147 removed.

- **The core lists Snapshots** on the user's side of all Calendars together, newest first, or of one Calendar, or only the user's own (ticket 147's rule): each with its time, its origin (an action of the user, an import, a Backup restore, an Incoming Sync or a Join, shown as "change on the server", a merge, "Stop waiting", an Undo naming what it took back), its description, its Calendars, how many Events it changed in each, read from its links, and whether an Undo in effect has taken it back. The list is read in pages, as 90 days of Polls can be long.
- **The menus:** the ⋮ main menu offers "History…", which opens a dialog with the list of all Calendars; a Calendar's menu in the sidebar (`src/lib/Sidebar.svelte`) offers "History", for Local Calendars too, which opens it filtered to that Calendar. The filters are "All Calendars" or one, and "Only my changes". The dialog is read again on "Occurrences changed".
- **Each line** shows the time (`src/lib/when.ts`), what made it, the number of Events ("Change on the server · 3 Events", "Import · 120 Events", "You · 1 Event") and the Calendars. It offers "Undo", except on a Calendar's first Snapshot, on a Snapshot an Undo in effect has taken back, and where it spans a Calendar marked read-only. A line taken back shows it.
- **"Undo"** makes the Undo of ticket 147 at once, whole, in all Calendars the Snapshot spans, a change on the server too, with ticket 158's toasts: no question first, "Redo" in the toast, and a refusal with "Undo both changes".
- **Not here:** an Event's History (ticket 160).

Spec: US-83, US-84, US-155; "Calendar History"; "Toasts". ADRs 0018, 0020.

**Blocked by:** 148, 158

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists the Snapshots on the user's side of all Calendars or of one, or only the user's own, with time, origin, description, number of Events per Calendar and whether it is taken back, in pages
- [ ] The main menu offers "History…" and a Calendar's menu opens it filtered; both filters work
- [ ] "Undo" on a line undoes that Snapshot whole, also a change on the server, and the toast offers "Redo"; a refusal shows as with Ctrl+Z
- [ ] No "Undo" on a first Snapshot, a taken-back one, or one spanning a read-only Calendar
- [ ] The dialog follows "Occurrences changed"
- [ ] Core tests through the core interface, with the Store Check after each; component tests on the fake core

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19.
- 2026-11-16: Chosen here, not in an ADR, for review: at a merge the list shows the Snapshots of both sides, as both are what the Calendar descends from, ordered by time; "before" a Snapshot of the server's side is its parent on the server's side.
- 2026-11-16: Decided by the user: a "Go back" is taken back both by Ctrl+Shift+Z and by "Go forward to this" in the dialog. If ticket 158 is built first, its redo already has to reach a Snapshot beyond one Calendar's User's Pointer; whichever of the two tickets comes second checks it.
- 2026-11-21: The window names an Incoming Sync "change on the server" (grilling).
- 2026-11-21: Rewritten from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten): one list of all Calendars with filters, and "Undo" on any Snapshot, whole, in place of "Go back to before this" and "Go forward to this" for one Calendar. The comments above are history.
