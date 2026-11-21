# 151: The Server's Pointer: an Incoming Sync records the server's state

**What to build:** The server's side of the Calendar History (ADRs 0018 and 0019): the server's Current-Version Mark and the Server's Pointer, so that the Local Store holds what a Calendar's server last had as Event Versions. Propagate still decides what the user's side takes; the comparison of the two sides replaces it in ticket 152, and the Sender still reads the Unsent Changes until ticket 153.

- **The server's Current-Version Mark** is a second column of `event_version`, with its own unique index per Calendar, UID and recurrence ID. The trigger allows updates of the two marks only. An Event Version both sides agree on is one row with both marks: a side that takes over the other's version takes over its row (ADR 0018, "Event Versions").
- **The Server's Pointer** is a column of the Calendar, set for every Calendar with a server and empty for a Local Calendar, which the schema or the Store Check enforces. The function that moves a pointer rewrites its side's marks.
- **An Incoming Sync**, in one Database Transaction: if the fetched Server Objects changed the server's Events, it makes a Snapshot holding the server's state and moves the Server's Pointer to it. Then the user's side is brought up to it, as ADR 0019's invariant asks:
  - the user's side has nothing of its own (no Unsent Change in the Calendar): the User's Pointer moves to the same Snapshot;
  - otherwise: Propagate decides per Occurrence as today, and its result is a Join: a new Snapshot whose only parent is the User's Pointer's Snapshot, with an Incoming Sync as its origin, which the User's Pointer moves to. The server's Snapshot is not its parent.
- **A First Sync** ends with both pointers on the Calendar's first Snapshot (still built as in ticket 146 until the ticket for the First Sync in memory).
- **An Outgoing Sync.** Glue until ticket 153, named: each send of a UID that succeeds records, in the Database Transaction that stores its new ETag, that the server now holds the user's Event Versions of that UID. If the Calendar then has no Unsent Change left, the Server's Pointer moves to the User's Pointer's Snapshot; otherwise a Snapshot on the server's side holds the server's state, with the origin "an Outgoing Sync that couldn't send everything". Ticket 153 replaces this with one closing Database Transaction per Outgoing Sync.
- **The stored ETag** of a kept resource is written in the same Database Transaction as the Event Versions with the server's mark it belongs to. A Server Object FreeCal can no longer read keeps its Event Versions and loses its ETag, as now.
- **The Store Check** gains: the server's Current-Version Marks are exactly the Event Versions of the Snapshot the Server's Pointer names; a Calendar has a Server's Pointer exactly when it has a server; every stored ETag belongs to a UID with Event Versions carrying the server's mark. Its query for kept Snapshots counts both pointers.
- **Dropping** (ticket 148) spares the last common ancestor of the two sides; its test comes here.
- No change to what the user sees.

Spec: US-81; "Calendar History"; "Sync engine". ADRs 0018 ("The pointers"), 0019 ("The invariant", "Server Objects"), 0020.

**Blocked by:** 148, 150

**Status:** ready-for-agent

**Effort:** M

- [ ] The server's Current-Version Mark with its unique index; the Server's Pointer on the Calendar
- [ ] After any Incoming Sync, the Event Versions with the server's mark read as the server's resources do (test against Radicale, also with Unsent Changes in the Calendar)
- [ ] An Incoming Sync into a Calendar with nothing of its own makes one Snapshot for both pointers; with Unsent Changes it makes the server's Snapshot and a Join on the user's side, in one Database Transaction
- [ ] After an Outgoing Sync that sent everything, both pointers are on one Snapshot; after one that didn't, the server's side holds what was sent
- [ ] Ctrl+Z passes over an Incoming Sync's Snapshot and a Join; an Undo of either from the core takes back the server's changes
- [ ] Old Snapshots are dropped without the last common ancestor of the two sides
- [ ] The Store Check's new queries, each with a test that breaks it by hand
- [ ] The existing sync, send and undo tests pass unchanged

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of four parts of what was proposed as one ticket (XL): 151 the Server's Pointer, 152 the Reference Snapshot and the comparison, 153 the Outgoing Sync from the marks, 154 one sync activity and the read-back.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: A Join with one parent replaces the merge of an Incoming Sync; merges with two parents come only from ending a Conflict (grilling, ADR 0019).
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo and redo at a merge moved to ticket 134, as merges only end Conflicts; there are no forks to redo through.
