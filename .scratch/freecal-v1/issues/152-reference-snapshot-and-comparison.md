# 152: The Reference Snapshot and the comparison of the two sides; Propagate goes

**What to build:** The one piece of code that compares a Calendar's two sides (ADR 0019), with the Reference Snapshot it compares them with. An Incoming Sync follows the server or takes its changes in by a Join, and a Conflict is held from here on: the Server's Pointer stays off the Reference Snapshot and nothing is sent. Propagate comes out. Reporting Conflicts to the window and ending them by the user's choice are tickets 133 and 134.

- **The Reference Snapshot** is a column of the Calendar, set for every Calendar with a server from its First Sync on: the server's Snapshot the user's side last took in. It moves to the Server's Pointer when a comparison leaves no Conflict and when an Outgoing Sync records its outcome, and nowhere else.
- **The comparison** is ticket 167's function, called with the user's Event Versions, the server's and those of the Reference Snapshot, the server's side winning in the Unimplemented Fields. Per Occurrence:
  - only the server changed it: it follows the server;
  - only the user changed it: it stays to be sent;
  - both changed it to the same result: they agree, and the Join ends the user's row on the user's side and takes over the server's, so the Event Version is one row with both marks;
  - both changed it differently: a Conflict.
  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. "Changed" compares content against the Reference Snapshot: the same row is unchanged, different rows are compared by canonical form, so an edit back to the Reference Snapshot's content is no change. The managed fields and the Unimplemented Fields are two parts: for the Unimplemented Fields, if only one side changed them that side's are taken, and if both did the server's win and no Conflict is raised.
- **An Incoming Sync** does two steps in one Database Transaction, and no code may split them:
  1. if the server's Events changed, the Server's Pointer moves to a new Snapshot holding the server's state (ticket 151); the Reference Snapshot stays;
  2. if the Server's Pointer is off the Reference Snapshot, the comparison runs. With a Conflict left, nothing more happens. Otherwise the User's Pointer follows to the server's Snapshot if the user's side was on the Reference Snapshot, or else a Join takes the server's changes in: a new Snapshot whose only parent is the User's Pointer's Snapshot; then the Reference Snapshot moves to the Server's Pointer.
  The second step runs on every Incoming Sync while the Server's Pointer is off the Reference Snapshot, also when the server changed nothing, so a Conflict the two sides no longer have ends with the next Incoming Sync. Running it after each move of the User's Pointer is ticket 138.
- **An Outgoing Sync** needs the Server's Pointer on the Reference Snapshot and sends nothing otherwise. In ticket 151's glue, a send that records its outcome moves the Reference Snapshot to the Server's Pointer.
- **Unsent Changes.** Glue until ticket 153, named: after the second step, the Calendar's Unsent Changes are set to the Combined Event Keys whose Event Version with the user's mark differs from the one with the server's, keeping the time and the reason of those that stay.
- **What comes out:** Propagate and its special cases for Unsent Changes (`core/src/sync/`, `core/src/journal/mod.rs`'s successor), with the `GLOSSARY.md` entry Propagate a Server Object.
- **Until ticket 133,** a Calendar with a Conflict shows nothing: it only stops sending and merging. That is acceptable between tickets, as no real Local Store exists.
- **The Store Check** gains: the Reference Snapshot is the Server's Pointer's Snapshot or one it descends from; it is another one exactly while the comparison finds a Conflict.
- **Tests,** through the core interface against Radicale and as unit tests of the comparison, one per line of this table and more: each of the four outcomes, for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both; a change to a Recurring Event against a change to one Occurrence; an Undo not yet sent followed by another change on the server to the same Event (a Conflict, as for any edit); a Join not yet sent followed by a second change on the server to the same Event (no Conflict: ADR 0019's reason for the Reference Snapshot); a merge not yet sent followed by a second change by someone else to the same Event (no Conflict); a Conflict that ends because the server came to agree.

Spec: US-73 to US-76; "Sync engine"; "Calendar History". ADRs 0017 ("Part of the version"), 0019 ("Whether the server changed", "Tracking and the Reference Snapshot", "An Incoming Sync", "The invariant", "Conflicts").

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] The Reference Snapshot on the Calendar, set from the First Sync on, moved only by the comparison and by an Outgoing Sync's outcome
- [ ] One function compares the two sides; no other code does
- [ ] An Incoming Sync's two steps run in one Database Transaction: follow, Join, or hold a Conflict
- [ ] After an Undo, an Incoming Sync takes the server's changes in at once, as after any change
- [ ] While the Server's Pointer is off the Reference Snapshot, nothing is sent and Incoming Syncs keep moving the Server's Pointer
- [ ] The tests listed above
- [ ] An edit back by hand (not undo) while the server changed the Event follows the server, with no Conflict
- [ ] Both sides changing an Event to the same content ends with one row carrying both marks
- [ ] Propagate is gone from the code and `GLOSSARY.md`
- [ ] The Store Check's two new queries, each with a test that breaks it by hand

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of four parts (see ticket 151). Every Conflict ticket (133 to 138, 21) builds on this one.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: A Join with one parent replaces the merge of an Incoming Sync. While the User's Pointer has a redo path, the second step waits (Held during Undo); the comparison still runs. Store Check rule widened (grilling, ADR 0019).
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). The comparison itself moved to ticket 167, which Undo needs first; the second step no longer waits for a redo path.
