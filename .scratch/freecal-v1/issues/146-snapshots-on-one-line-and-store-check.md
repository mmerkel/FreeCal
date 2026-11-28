# 146: Snapshots on one line, `begin` and `end`, and the Store Check

**What to build:** The second part of the Calendar History (ADR 0018, rewritten 2026-11-22): every write to Events makes a Snapshot on one line for all Calendars, and each Event Version records the Snapshot it began with and the one it ended with. The Change Journal still records beside it, and undo is still per Event, until ticket 147. The server's Current-Version Mark comes with ticket 151.

- **Snapshots:** a table holding, for each Snapshot, when it was made and its origin: an action of the user, an import, the restore of a Backup, an Incoming Sync (an Incoming Sync Snapshot), or an undo. No table of parents: a Snapshot's parent is the one before it, by ID.
- **`begin` and `end`** on the Event Version: `begin` refers to the Snapshot it began with (`ON DELETE SET NULL`), set at insert; `end` to the one it ended with (`ON DELETE CASCADE`: an Event Version goes with the Snapshot it ended with), empty while current. They replace ticket 145's user's Current-Version Mark: the user's current version is the row whose `end` is empty, one per Calendar, UID and recurrence ID by a partial unique index, and `event` becomes a view over those rows. The trigger refuses every update but `end` from empty to a Snapshot and `begin` to empty.
- **A Snapshot spanning several Calendars:** a step that changes several Calendars together (a move to another Calendar, an import into several) is one Snapshot; which Calendars it spans is read from the Event Versions that begin or end with it.
- **A Calendar enters the Local Store** with no Event Versions for a Local Calendar. Glue until ticket 155 makes the First Sync a Full Fetch in one Database Transaction: a server Calendar is still stored when it is added, and while its First Sync is unfinished, its writes add to one Snapshot instead of making new ones. So the First Sync is one Snapshot, as ADR 0018 says, and no undo can take a part of it back.
- **Every write a Snapshot:** each Journal Transaction makes exactly one Snapshot with the matching origin, and its Event Versions begin and end with it. An undo, still the Change Journal's, writes copies of the versions it brings back (ADR 0018). A write that changes nothing makes none.
- **Old rows stay:** ticket 145's deletion of unmarked rows goes. Nothing is dropped until ticket 148.
- **The Calendar History's module** (`core/src/calendar_history/`): the functions that write Event Versions, Snapshots and marks are visible only to it. The Write Permissions stay until their own ticket; the Change Journal's code calls into the module.
- **The Store Check** (ADR 0018, "Where the invariants are recorded"): a function beside the schema in which each invariant is a named query with one line stating the rule and its ADR, reporting every violation. The test support (`core/tests/support`) runs it after each scenario of every test through the core interface. Its queries now: `end` is after `begin`; every Snapshot has an Event Version beginning or ending with it. It states that it doesn't cover ADR 0019's invariant on the Incoming Sync. Foreign keys and indexes carry a comment naming their rule.
- Deleting a Calendar removes its Event Versions; a Snapshot left with none is removed with them (ADR 0018, "Removing a Calendar").
- An action of the user records which it was: create, edit, delete or move (ADR 0018, "Snapshots"), as an Undo needs to know a move after one of its Calendars was removed (ticket 147).
- No change to the core interface or to what the user sees.

Spec: US-81; "Calendar History"; "Local Store". ADR 0018.

**Blocked by:** 145

**Status:** ready-for-agent

**Effort:** M

- [ ] The Snapshot table; `begin` and `end` on the Event Version with their foreign keys; the partial unique index where `end` is empty; `event` a view over those rows; the trigger allowing only the two late changes
- [ ] Every Calendar's First Sync is one Snapshot; an unfinished one adds to it
- [ ] Each kind of write makes one Snapshot with its origin: a user's change, an import, an Incoming Sync, an undo; a move and an import into several Calendars make one Snapshot
- [ ] A test reads the Events of a Calendar as they were at each Snapshot (`begin ≤ N < end`)
- [ ] The Store Check with its queries, run after each scenario of the tests through the core interface; a test per query breaks the invariant by hand and sees it reported
- [ ] Debug builds run the Store Check after every Database Transaction that writes the Calendar History and panic on a violation; release builds don't run it
- [ ] The write functions are private to the Calendar History's module

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of four parts (see ticket 145).
- 2026-11-22: Debug builds run the Store Check too (point 5 of `critiques.md`); running it in release builds before an Outgoing Sync was rejected (ADR 0018, Considered Options).
- 2026-11-21: Decided in a grilling: links belong to a Snapshot Transition, not to a Snapshot; a Calendar's first Snapshot has a Snapshot Transition without a parent; the origin stays on the Snapshot (ADR 0018, "Snapshots").
- 2026-11-22: Rewritten after the grilling on critique 2: one line for all Calendars, so no Snapshot Transitions, no link table and no User's Pointer; `begin` and `end` on the Event Version replace them and the user's mark. The ticket's file name keeps its old title.
- 2026-11-22: Reworded from the grilling on point 4 of `.scratch/freecal-v1/critiques.md` (ADR 0018): ticket 155 no longer holds the First Sync in memory; a server Calendar stays stored when added, and only the glue for an unfinished First Sync goes with 155.
- 2026-11-22: `end` is `ON DELETE CASCADE`, not `RESTRICT` (point 4 of `.scratch/freecal-v1/critiques-2.md`, ADR 0018 "Keeping and dropping"): what is kept is decided for Snapshots, and a kept Snapshot keeps the Event Versions ending with it.
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): Server Event Replacements are no longer Snapshots of their own.
- 2026-11-28: Reworded from the grilling on point 5 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0020): the Snapshot of an action of the user records which it was, a move among them.
