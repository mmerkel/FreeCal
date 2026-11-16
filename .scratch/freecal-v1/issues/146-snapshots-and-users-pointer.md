# 146: Snapshots, the User's Pointer and the Store Check

**What to build:** The second part of the Calendar History (ADR 0018): every write to a Calendar's Events makes a Snapshot and moves the Calendar's User's Pointer to it. The Change Journal still records beside it, and undo is still per Event, until ticket 147. The server's side comes with ticket 151.

- **Snapshots:** a table holding, for each Snapshot, when it was made and its origin: an action of the user, an import, the restore of a Backup, an Incoming Sync, an Outgoing Sync that couldn't send everything, or a merge. In a Calendar a Snapshot has one parent, or two when it is a merge (the user's side and the server's); the schema allows both now, though merges arise only from ticket 151.
- **Links:** for each step from a parent to a Snapshot in a Calendar, which Event Versions begin and which end with it. An end is a link, not a column of the Event Version.
- **A Snapshot spanning several Calendars:** a step that changes several Calendars together (a move to another Calendar, an import into several) is one Snapshot with a parent in each.
- **The User's Pointer** is a column of the Calendar, never empty. The user's Current-Version Marks are derived from it: one function moves a pointer and rewrites the marks, in one Database Transaction, and nothing else writes either.
- **A Calendar enters the Local Store with its first Snapshot**, in one Database Transaction: an empty one for a Local Calendar. Glue until the ticket for the First Sync in memory, named: a server Calendar is still stored when it is added, with an empty first Snapshot, and while its First Sync is unfinished the First Sync's writes add to that first Snapshot instead of making new ones. So the first Snapshot is the First Sync's, as ADR 0018 says, and no undo can take a part of a First Sync back.
- **Every write a Snapshot:** each Journal Transaction makes exactly one Snapshot with the matching origin. An Incoming Sync makes one and moves the User's Pointer directly; there is no Server's Pointer yet. An undo, still the Change Journal's, is a write like any other and makes a Snapshot. A write that changes nothing makes none.
- **Old rows stay:** ticket 145's deletion of unmarked rows goes. An Event Version is kept while a Snapshot refers to it. Nothing is dropped until ticket 148.
- **The Calendar History's module** (`core/src/calendar_history/`): the functions that write Event Versions, Snapshots, links, pointers and marks are visible only to it. The Write Permissions stay until their own ticket; the Change Journal's code calls into the module.
- **The Store Check** (ADR 0018, "Where the invariants are recorded"): a function beside the schema in which each invariant is a named query with one line stating the rule and its ADR, reporting every violation. The test support (`core/tests/support`) runs it after each scenario of every test through the core interface. Its queries now:
  - the user's Current-Version Marks are exactly the Event Versions of the Snapshot the User's Pointer names;
  - every Event Version is referred to by a Snapshot or has a mark;
  - every Snapshot is an ancestor of a pointer or beyond the User's Pointer.
  It states that it doesn't cover ADR 0019's invariant on the Server's Pointer. Foreign keys and indexes carry a comment naming their rule.
- Deleting a Calendar removes its Snapshots, links and Event Versions; a Snapshot spanning other Calendars stays for those.
- No change to the core interface or to what the user sees.

Spec: US-81; "Calendar History"; "Local Store". ADR 0018.

**Blocked by:** 145

**Status:** ready-for-agent

**Effort:** M

- [ ] Tables for Snapshots, parents and links; the User's Pointer on the Calendar, never empty
- [ ] One function moves the User's Pointer and rewrites the marks; no other code writes a pointer or a mark
- [ ] Every Calendar is created with its first Snapshot; a server Calendar's unfinished First Sync adds to it
- [ ] Each kind of write makes one Snapshot with its origin: a user's change, an import, an Incoming Sync, an undo; a move and an import into several Calendars make one Snapshot spanning them
- [ ] A test walks a Calendar's Snapshots back through the links and gets the Calendar's Events as they were at each
- [ ] The Store Check with its three queries, run after each scenario of the tests through the core interface; a test per query breaks the invariant by hand and sees it reported
- [ ] The write functions are private to the Calendar History's module

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of four parts (see ticket 145).
