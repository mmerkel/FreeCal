# 148: Undo for one Calendar, and Snapshots dropped after 90 days

**What to build:** The last part of the Calendar History on the user's side (ADRs 0018 and 0020): the core's undo and redo for one Calendar, and the dropping of old Snapshots. No window work: the Calendar's History in the sidebar menu, which uses this, has its own ticket.

- **Undo for one Calendar** moves only that Calendar's User's Pointer to the parent of the Snapshot it is on; redo for one Calendar moves it forward. It asks and sends as Ctrl+Z does (ticket 147).
- **A Snapshot spanning several Calendars, undone in one:** the other Calendars stay as they are. After undoing one Calendar's part of a move the Event is in both Calendars or in neither, and after one part of an import the rest of the import stays. The core's answer says so (which other Calendars the Snapshot spans and still holds it), so that the window can tell the user.
- **Ctrl+Z on such a Snapshot afterwards:** it is undone in the Calendars whose User's Pointer is still on it. ADR 0020 doesn't state this case; it follows from "the newest Snapshot a User's Pointer is on". The answer names the Calendars left as they are.
- **The redo path** after an undo in one Calendar ends by ticket 147's rule, per Calendar: a Snapshot spanning Calendars is kept while any pointer reaches it or it is beyond a User's Pointer.
- **Dropping:** Snapshots older than 90 days are dropped, except the last common ancestor of any Snapshot younger than 90 days that only one side has reached (with the user's side alone this exception has no case; ticket 151 adds its test). The oldest Snapshot kept in a Calendar becomes its first: its Snapshot Transition in that Calendar loses its parent, and its links say that every Event Version it holds begins with it, and undo stops there. An Event Version goes when no kept Snapshot refers to it and it has no Current-Version Mark. A Snapshot spanning Calendars is dropped per Calendar.
- **When:** at start and once a day while FreeCal runs, in one Database Transaction per Calendar, where the Change Journal's 90-day removal ran before ticket 147.
- **The Store Check** passes after a drop: every Event Version is still referred to or marked, and every pointer names a kept Snapshot.

Spec: US-83, US-84; "Calendar History". ADRs 0018 ("Keeping and dropping"), 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** M

- [ ] The core undoes and redoes one Calendar; other Calendars' pointers don't move
- [ ] Undoing one Calendar's part of a move and of an import: the answer names the other Calendars; tests for the Event then being in both Calendars and in neither
- [ ] Ctrl+Z on a Snapshot one Calendar has already left undoes it in the others
- [ ] Snapshots older than 90 days are dropped with the Event Versions only they held; the oldest kept one is the Calendar's first and undo stops at it
- [ ] An Event unchanged for more than 90 days is still there after a drop, and so is the Calendar as the user sees it (test with the clock set forward)
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Fourth of four parts (see ticket 145).
- 2026-11-21: Decided in a grilling: links belong to a Snapshot Transition, not to a Snapshot; a Calendar's first Snapshot has a Snapshot Transition without a parent; the origin stays on the Snapshot (ADR 0018, "Snapshots").
- 2026-11-21: Imports no longer create Calendars (ADR 0018), so the case of a Calendar whose first Snapshot is the import's is gone.
