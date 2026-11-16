# 159: A Calendar's History in the sidebar menu, with "Go back to before this"

**What to build:** The view of one Calendar's History (US-83) and the way to take that Calendar back, on the core's undo for one Calendar (ticket 148). It takes the place of the Change Journal dialog that ticket 147 removed.

- **The core lists a Calendar's Snapshots:** every Snapshot the Calendar's User's Pointer is on or descends from, newest first, each with its time, its origin (an action of the user, an import, a Backup restore, an Incoming Sync, an Outgoing Sync that couldn't send everything, a merge), how many Events it changed in this Calendar, read from its links, and the other Calendars it spans. The Calendar's first Snapshot is the last line. The list is read in pages, as 90 days of Polls can be long.
- **The core takes one Calendar back to before a given Snapshot:** it moves that Calendar's User's Pointer to the given Snapshot's parent, in one Database Transaction, and reports what ticket 148's undo for one Calendar reports for each Snapshot left on the way. The Snapshot must be an ancestor of the User's Pointer or the one it is on, and not the Calendar's first; otherwise it is refused. For a merge the caller names the parent, the user's side or the server's. The redo path and what is sent afterwards follow tickets 147 and 153; the Mass-Change Guard checks the Outgoing Sync that follows (ticket 22).
- **The menu:** a Calendar's menu in the sidebar (`src/lib/Sidebar.svelte`) offers "History", for Local Calendars too. It opens a dialog with the list. The dialog is read again on "Occurrences changed" for its Calendar.
- **Each line** shows the time (`src/lib/when.ts`), what made it and the number of Events ("Sync · 3 Events", "Import · 120 Events", "You · 1 Event"), and for a Snapshot spanning several Calendars the others' names. It offers "Go back to before this", except on the first Snapshot and in a Read-only Calendar, whose User's Pointer follows its Server's Pointer.
- **"Go back to before this"** asks once: how many Snapshots go, how many of them came from the server, and, for a Calendar that is Tracking, that the server is brought to that state too. For a merge it asks which side, as Ctrl+Z does (ticket 158; until that ticket is built, this dialog has its own form of the question).
- **Afterwards** a toast says what was done. If a Snapshot spanning several Calendars was among those left, the toast stays until dismissed and names the other Calendars and what that means: a moved Event is now in both Calendars or in neither, an import is still in the others.
- **Taking a "Go back" back,** two ways (decided 2026-11-16):
  - Ctrl+Shift+Z reaches it: the redo of ticket 158 also takes a Snapshot beyond one Calendar's User's Pointer, one Snapshot per press. When another Calendar has a Snapshot to bring back too, redo asks which Calendar, as it always does then (ADR 0020, "Which Calendar").
  - The dialog lists the Snapshots beyond the Calendar's User's Pointer above the one it is on, set apart as "taken back", each with "Go forward to this", which moves this Calendar's User's Pointer forward to it in one Database Transaction, without a question unless a fork is on the way. The core's list marks which Snapshot the pointer is on and returns those beyond it; they go when the redo path ends (ticket 147), and the dialog then no longer lists them.
- **Not here:** an Event's History (ticket 160); taking back one Snapshot from the middle (ADR 0020, "Left open").

Spec: US-83, US-84; "Calendar History"; "Toasts". ADRs 0018, 0020.

**Blocked by:** 148

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists a Calendar's Snapshots with time, origin, number of Events and the other Calendars spanned, in pages
- [ ] The core moves one Calendar's User's Pointer to before a given Snapshot in one Database Transaction, and refuses the first Snapshot and one that isn't an ancestor
- [ ] The sidebar menu offers "History" and the dialog lists the Snapshots, newest first
- [ ] "Go back to before this" asks once, with the number of Snapshots, and then the Calendar shows the earlier state; other Calendars don't change
- [ ] After a "Go back", the Snapshots taken back are listed with "Go forward to this", and Ctrl+Shift+Z brings them back one at a time, asking when another Calendar has a redo too; a change by the user in this Calendar ends both
- [ ] A Read-only Calendar's History has no "Go back"
- [ ] Going back past a move says that the Event is now in both Calendars or in neither, in a toast that stays
- [ ] The dialog follows "Occurrences changed"
- [ ] Core tests through the core interface, with the Store Check after each; component tests on the fake core

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19.
- 2026-11-16: Chosen here, not in an ADR, for review: at a merge the list shows the Snapshots of both sides, as both are what the Calendar descends from, ordered by time; "before" a Snapshot of the server's side is its parent on the server's side.
- 2026-11-16: Decided by the user: a "Go back" is taken back both by Ctrl+Shift+Z and by "Go forward to this" in the dialog. If ticket 158 is built first, its redo already has to reach a Snapshot beyond one Calendar's User's Pointer; whichever of the two tickets comes second checks it.
