# 152: The Reference Snapshot and the comparison of the two sides; Propagate goes

**What to build:** The one piece of code that compares a Calendar's two sides (ADR 0019), with the Reference Snapshot it compares them with. An Incoming Sync follows the server or merges through it, and a Conflict is held from here on: the Server's Pointer stays off the Reference Snapshot and nothing is sent. Propagate comes out. Reporting Conflicts to the window and ending them by the user's choice are tickets 133 and 134.

- **The Reference Snapshot** is a column of the Calendar, set for every Calendar with a server from its First Sync on: the server's Snapshot the user's side last took in. It moves to the Server's Pointer when a comparison leaves no Conflict and when an Outgoing Sync records its outcome, and nowhere else.
- **The comparison** takes the server's Event Versions, the user's and those of the Reference Snapshot, and decides per Occurrence, at the grain Propagate has today (ticket 21 refines it for Recurring Events):
  - only the server changed it: it follows the server;
  - only the user changed it: it stays to be sent;
  - both changed it to the same result: they agree;
  - both changed it differently: a Conflict.
  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. The managed fields and the Unimplemented Fields are two parts: for the Unimplemented Fields, if only one side changed them that side's are taken, and if both did the server's win and no Conflict is raised.
- **An Incoming Sync** does two steps in one Database Transaction, and no code may split them:
  1. if the server's Events changed, the Server's Pointer moves to a new Snapshot holding the server's state (ticket 151); the Reference Snapshot stays;
  2. if the Server's Pointer is off the Reference Snapshot, the comparison runs. With a Conflict left, nothing more happens. Without one, the User's Pointer follows to the server's Snapshot if the user's side was on the Reference Snapshot, or else a merge Snapshot joins the two; then the Reference Snapshot moves to the Server's Pointer.
  The second step runs on every Incoming Sync while the Server's Pointer is off the Reference Snapshot, also when the server changed nothing, so a Conflict the two sides no longer have ends with the next Incoming Sync. Running it after each move of the User's Pointer is ticket 138.
- **An Outgoing Sync** needs the Server's Pointer on the Reference Snapshot and sends nothing otherwise. In ticket 151's glue, a send that records its outcome moves the Reference Snapshot to the Server's Pointer.
- **Unsent Changes.** Glue until ticket 153, named: after the second step, the Calendar's Unsent Changes are set to the Combined Event Keys whose Event Version with the user's mark differs from the one with the server's, keeping the time and the reason of those that stay.
- **What comes out:** Propagate and its special cases for Unsent Changes (`core/src/sync/`, `core/src/journal/mod.rs`'s successor), with the `GLOSSARY.md` entry Propagate a Server Object.
- **Until ticket 133,** a Calendar with a Conflict shows nothing: it only stops sending and merging. That is acceptable between tickets, as no real Local Store exists.
- **The Store Check** gains: the Reference Snapshot is the Server's Pointer's Snapshot or one it descends from; it is another one exactly while the comparison finds a Conflict.
- **Tests,** through the core interface against Radicale and as unit tests of the comparison, one per line of this table and more: each of the four outcomes, for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both; a change to a Recurring Event against a change to one Occurrence; the undo not yet sent followed by another change on the server (ADR 0019's reason for the Reference Snapshot: the undo must not be lost without a word); a merge not yet sent followed by a second change by someone else to the same Event (no Conflict); a Conflict that ends because the server came to agree.

Spec: US-73 to US-76; "Sync engine"; "Calendar History". ADRs 0017 ("Part of the version"), 0019 ("Whether the server changed", "Tracking and the Reference Snapshot", "An Incoming Sync", "The invariant", "Conflicts").

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] The Reference Snapshot on the Calendar, set from the First Sync on, moved only by the comparison and by an Outgoing Sync's outcome
- [ ] One function compares the two sides; no other code does
- [ ] An Incoming Sync's two steps run in one Database Transaction: follow, merge, or hold a Conflict
- [ ] While the Server's Pointer is off the Reference Snapshot, nothing is sent and Incoming Syncs keep moving the Server's Pointer
- [ ] The tests listed above
- [ ] Propagate is gone from the code and `GLOSSARY.md`
- [ ] The Store Check's two new queries, each with a test that breaks it by hand

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of four parts (see ticket 151). Every Conflict ticket (133 to 138, 21) builds on this one.
