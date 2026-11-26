# 159: The Calendar History window, with "Undo" on each line

**What to build:** The view of the Calendar History (US-83, US-155) and the way to undo any one Snapshot from it, on the core's Undo (ticket 147). It takes the place of the Change Journal dialog that ticket 147 removed.

- **The core lists Snapshots** of all Calendars together, newest first, or of one Calendar, or only the user's own (ticket 147's rule): each with its time, its origin (an action of the user, an import, a Backup restore, an Incoming Sync Snapshot, shown as "changes from the server", naming inside it the Events "replaced by the server's version" and those "combined with a change on the server" (ticket 168), "Stop waiting", an Undo naming what it took back), its description, its Calendars, how many Events it changed in each, read from the Event Versions that begin and end with it, and whether Undos in effect have taken it back, wholly or for some of its Events. The list is read in pages, as 90 days of Polls can be long.
- **The menus:** the ⋮ main menu offers "History…", which opens a dialog with the list of all Calendars; a Calendar's menu in the sidebar (`src/lib/Sidebar.svelte`) offers "History", for Local Calendars too, which opens it filtered to that Calendar. The filters are "All Calendars" or one, and "Only my changes". The dialog is read again on "Occurrences changed".
- **Each line** shows the time (`src/lib/when.ts`), what made it, the number of Events ("Change on the server · 3 Events", "Import · 120 Events", "You · 1 Event") and the Calendars. It offers "Undo", except on a Calendar's first Snapshot, on a Snapshot an Undo in effect has taken back, and on one spanning a Calendar marked read-only, as its Undo could only be refused (ADR 0020, "Read-only Calendars"). A line taken back shows it.
- **"Undo this Event"**, on an Event of a line, makes the Undo of ticket 147 limited to that Event, in every Calendar the Snapshot spans; for an Event in Conflict it is the restore (ticket 134).
- **"Undo"** makes the Undo of ticket 147 at once, whole, in all Calendars the Snapshot spans, a change on the server too, with ticket 158's toasts: no question first, "Redo" in the toast, and a refusal with "Undo both changes".
- **Not here:** an Event's History (ticket 160).

Spec: US-83, US-84, US-155; "Calendar History"; "Toasts". ADRs 0018, 0020.

**Blocked by:** 148, 158

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists the Snapshots on the user's side of all Calendars or of one, or only the user's own, with time, origin, description, number of Events per Calendar and whether it is taken back, in pages
- [ ] The main menu offers "History…" and a Calendar's menu opens it filtered; both filters work
- [ ] "Undo" on a line undoes that Snapshot whole, also a change on the server, and the toast offers "Redo"; a refusal shows as with Ctrl+Z
- [ ] "Undo this Event" undoes the Snapshot for that Event alone
- [ ] No "Undo" on a first Snapshot, a taken-back one, or one spanning a read-only Calendar
- [ ] The dialog follows "Occurrences changed"
- [ ] Core tests through the core interface, with the Store Check after each; component tests on the fake core

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19.
- 2026-11-16: Chosen here, not in an ADR, for review: at a merge the list shows the Snapshots of both sides, as both are what the Calendar descends from, ordered by time; "before" a Snapshot of the server's side is its parent on the server's side.
- 2026-11-16: Decided by the user: a "Go back" is taken back both by Ctrl+Shift+Z and by "Go forward to this" in the dialog. If ticket 158 is built first, its redo already has to reach a Snapshot beyond one Calendar's User's Pointer; whichever of the two tickets comes second checks it.
- 2026-11-21: The window names an Incoming Sync "change on the server" (grilling).
- 2026-11-21: Rewritten from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten): one list of all Calendars with filters, and "Undo" on any Snapshot, whole, in place of "Go back to before this" and "Go forward to this" for one Calendar. The comments above are history.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the origins are Incoming Sync Snapshots and Server Event Replacements instead of Incoming Syncs, Joins and merges; counts are read from `begin` and `end`. The spec's grouping of one Incoming Sync's Incoming Sync Snapshot and Server Event Replacements into one line that opens is to be built here.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): the Automatic Resolution is a new origin. Decided by the user: a Snapshot spanning a read-only Calendar keeps showing no "Undo", as its Undo could only be refused (ADR 0020 says so now).
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADR 0019): the Server Event Merge replaces the Automatic Resolution as an origin.
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): the Server Event Replacement and Merge are no longer origins but shown inside their Incoming Sync Snapshot's line; "Undo this Event" offers ADR 0020's limited Undo for any Snapshot.
