# 207: The Calendar History window, with "Undo" on each line

**What to build:** The view of the Calendar History (US-83, US-155) and the Undo of any one Snapshot from it, on 180 and 181's Undo and 183's toasts and dialog. Old ticket 159 (`git show design/snapshot-line:.scratch/freecal-v1/issues/159-calendar-history-window.md`) as written. It fills the place of the Change Journal's dialog that 182 deleted, with new tests; it isn't called "Undo History" (the glossary's Undo History is what Ctrl+Z steps through).

- **The core lists Snapshots** (`history/`, a reader of its own) of all Calendars, of one, or only the user's own (180's rule), newest first, in pages, as 90 days of Polls can be long. Each with its time, its origin (an action of the user, an import, a Backup restore, an Incoming Sync Snapshot, "Stop waiting", an Undo naming what it took back), its description, its Calendars with the number of Events it changed in each (read from the Event Versions that begin and end with it), and whether Undos in effect took it back, wholly or for some Events. Inside an Incoming Sync Snapshot it names the Events "replaced by the server's version" (190) and those "combined with a change on the server" (198's zero-length Event Versions; old 168's line, the last of 159, 160 and 168 to come). Interface types; regenerate the TypeScript types and give the fake core its answer.
- **The menus:** the ⋮ main menu's "History…" opens the dialog for all Calendars; a Calendar's menu in the sidebar (`src/lib/Sidebar.svelte`) offers "History", for Local Calendars too, filtered to it. Filters: "All Calendars" or one, and "Only my changes". Read again on "Occurrences changed".
- **Each line:** the time (`src/lib/when.ts`), what made it, the number of Events ("Change on the server · 3 Events", "Import · 120 Events", "You · 1 Event") and the Calendars; a line taken back shows it. "Undo", a First Sync's too (ADR 0018), except on a line taken back or one whose every Event in effect is in a Calendar marked read-only or removed (ADR 0020, "Read-only Calendars").
- **"Undo"** is 180's whole Undo in every Calendar the Snapshot spans, a change on the server too, with 183's toasts ("Redo") and dialog where the core asks first. **"Undo this Event"** on an Event of a line is 181's Undo limited to its UID; on an Event in Conflict it is the restore and resolves the Conflict (191).
- **206's merge toast** gains "Show" for several Server Event Merges, opening this window at that Incoming Sync.
- **Not here:** an Event's History (184). Titles and Calendar names are untrusted text.

Spec: US-83, US-84, US-155; "Calendar History"; "Toasts"; "Untrusted content". ADRs 0018, 0020. Layout: "By phase", "P5 and P6" (the Calendar History's listing in `history/`).

**Blocked by:** 183, 185, 191, 198

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists Snapshots of all Calendars, one, or only the user's own, in pages, with time, origin, description, Events per Calendar and whether taken back; Incoming Sync Snapshots name replaced and combined Events
- [ ] "History…" in the main menu and "History" in a Calendar's menu; both filters work; the dialog follows "Occurrences changed"
- [ ] "Undo" undoes a line whole, a change on the server and a First Sync too, with "Redo"; it asks first as Ctrl+Z does; none on a line taken back or left only in read-only or removed Calendars
- [ ] "Undo this Event" undoes that UID alone
- [ ] The merge toast's "Show" opens the window at its Incoming Sync
- [ ] Core tests through the interface, the Store Check after each, including old 134's "restore by 'Undo this Event' in the Calendar History resolves the Conflict"; component tests on the fake core written anew from US-83 and US-155, using the scenarios of the deleted `core/tests/change_journal_list.rs`, `src/App.journal.test.ts` and `windowDialogs.test.ts`'s "the Undo History" (`git show design/snapshot-line:<path>`)
