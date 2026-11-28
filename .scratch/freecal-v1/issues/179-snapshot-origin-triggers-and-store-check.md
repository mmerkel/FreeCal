# 179: The Snapshot's origin, the line's triggers and the Store Check

**What to build:** The second part of the Calendar History (ADR 0018): every Snapshot records its origin, the schema refuses changes to the line, and the Store Check states the invariants the schema can't. Old ticket 146 (`git show design/snapshot-line:.scratch/freecal-v1/issues/146-snapshots-on-one-line-and-store-check.md`) without what 178 already did, and the core's part of old 172 (`git show design/snapshot-line:.scratch/freecal-v1/issues/172-removing-a-calendar-says-it-cant-be-undone.md`). The Change Journal still records beside it until 182.

- **Origin:** `snapshot.origin`: an action of the user with its `UserAction` (create, edit, delete, and the others today's `TransactionOrigin` names), an import, a Backup restore, an undo (the Change Journal's, until 180's Undo), an Incoming Sync. `TransactionOrigin` (`journal/mod.rs`) becomes the Snapshot's origin type in `history/mod.rs`. The `undoes` column is 180's, `undo_uid` 181's; `Rescheduled` and `MovedBetweenCalendars` are 180's.
- **A step changing several Calendars** (a move, an import into several) is one Snapshot; the Calendars it spans are read from its Event Versions.
- **The First Sync** until 194: each page it writes is an Incoming Sync Snapshot of its own, as every write makes a Snapshot. No special case, and no test asserts one Snapshot per First Sync before 194.
- **Triggers on `event_version`:** refuse every update but `end` from empty to a Snapshot and `begin` to empty; `end = begin` only with an Incoming Sync Snapshot. A comment at each names its rule and ADR.
- **The Store Check** (`store/check.rs`; ADR 0018, "Where the invariants are recorded"): each invariant a named query with one line stating the rule and its ADR, reporting every violation. Now: `end` is after `begin` (or equal, for an Incoming Sync Snapshot); every Snapshot has an Event Version beginning or ending with it; a key isn't begun twice in one Snapshot. Its doc comment says it doesn't cover ADR 0019's invariant on the Incoming Sync. `core/tests/support/Harness` runs it on drop and fails the test on a violation; debug builds run it after every Database Transaction that writes the Calendar History and panic on one.
- **Removing a Calendar** (ADR 0018, "Removing a Calendar"; `Core::remove_calendar`, `delete_local_copy`): its Event Versions go, and Snapshots left with none go with them, in one Database Transaction. Check `caldav_calendars`' `removing_an_account_removes_…_their_history` against this.
- No change to the core interface or to what the user sees.

Spec: US-81; "Calendar History"; "Local Store". ADR 0018 ("Snapshots", "Calendars", "Removing a Calendar", "Where the invariants are recorded"). Layout: "Tables (end state)" (`snapshot.origin`, `event_version`'s triggers); "Seams for `/tdd`", 2; "By phase", P2, 179.

**Blocked by:** 178

**Status:** ready-for-agent

**Effort:** M

- [ ] Each kind of write makes one Snapshot with its origin: each user action, an import, a Backup restore, an undo, an Incoming Sync; a move and an import into several Calendars make one Snapshot each
- [ ] The triggers refuse each forbidden update (a test per column) and `end = begin` outside an Incoming Sync Snapshot
- [ ] The Store Check with its queries; a test per query breaks the invariant by hand and sees it reported
- [ ] The Harness runs the Store Check after every test through the core interface; debug builds after every write of the Calendar History
- [ ] Removing a Calendar removes its Event Versions and the Snapshots left empty; a Snapshot spanning another Calendar stays
