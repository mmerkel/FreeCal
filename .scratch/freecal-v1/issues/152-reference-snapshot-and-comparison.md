# 152: The comparison against the server's marks; Incoming Sync Snapshots and Server Event Replacements; Propagate goes

**What to build:** An Incoming Sync compares what it fetched with the user's current versions and the server's marks (ADR 0019, rewritten 2026-11-22). Every version fetched begins with the Incoming Sync Snapshot; each Event in Conflict then gets a Server Event Replacement, in which it takes a copy of the Server Version. No Calendar stops syncing. Propagate comes out. Reporting Conflicts and the Resolution are tickets 133 and 134.

- **The comparison** is ticket 167's function, called per Occurrence with the fetched versions, the user's current ones and the ones with the server's mark (what the server held when FreeCal last looked):
  - only the server changed it: it follows the server, in the Incoming Sync Snapshot;
  - only the user changed it: it stays to be sent;
  - both changed it to the same result: they agree, and the server's mark moves to the user's row;
  - both changed it differently: a Conflict, or combinable by field group (ticket 167).
  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. "Changed" compares content: the same row is unchanged, different rows are compared by canonical form, so an edit back to what the server has is no change. A change to the Unimplemented Fields is a change like any other. Combining what can be combined is ticket 168's Server Event Merge; until then an Occurrence the comparison could combine is treated as a Conflict and gets a Server Event Replacement.
- **An Incoming Sync** writes in one Database Transaction, and no code may split it:
  1. an Incoming Sync Snapshot, with which every version fetched begins: those of Events in no Conflict stay current; those of Events in Conflict end with it at once, **zero-length Event Versions** (ADR 0018) carrying only the server's mark;
  2. a Server Event Replacement per Event (Calendar and UID) in Conflict, one Snapshot each, with which a copy of each of that Event's server versions begins on the user's side, an edit against a deletion on the server too; the server's mark moves to the copies. It refers to its Incoming Sync Snapshot in a column of the Snapshot table, a deferred foreign key and an edge of the drop's rule (ADR 0018), set exactly for a Server Event Replacement or a Server Event Merge (a CHECK). It doesn't name the Last Agreed Version: the comparison uses the version with the server's mark before it moves, and nothing reads it afterwards (ticket 135 compares the two sides);
  3. the server's marks moved to the fetched versions, with their ETags.
  An Incoming Sync Snapshot is made whenever a version fetched differs from the one with the server's mark, even where all of them then go to Server Event Replacements; a Server Event Replacement that would change nothing isn't made. Nothing is sent for a replacement.
- **Zero-length Event Versions in the schema:** an insert trigger allows `end` equal to `begin` only where that Snapshot is an Incoming Sync Snapshot, `end` after `begin` everywhere else, with a comment naming ADR 0018. Reading a Snapshot's state goes by `begin` at or before it and `end` after it or empty. The Store Check gains: every Server Event Replacement's Event has a zero-length Event Version at the Incoming Sync Snapshot it refers to.
- **Dropping** (ticket 148) keeps an Event Version a Server Event Replacement names, and an Incoming Sync Snapshot a kept Server Event Replacement refers to; their tests come here.
- **Unsent Changes.** Glue until ticket 153, named: after the Incoming Sync, the Calendar's Unsent Changes are set to the Combined Event Keys whose current Event Version differs from the one with the server's mark, keeping the time and the reason of those that stay.
- **What comes out:** Propagate and its special cases for Unsent Changes (`core/src/sync/`, `core/src/journal/mod.rs`'s successor), with the `GLOSSARY.md` entry Propagate a Server Object.
- **Until ticket 133,** a replacement shows nothing beyond the Server Version in the grid. That is acceptable between tickets, as no real Local Store exists.
- **Tests,** through the core interface against Radicale and as unit tests of the comparison: each of the four outcomes, for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side (followed or sent) and on both (a Conflict); a change to a Recurring Event against a change to one Occurrence; an Undo not yet sent followed by another change on the server to the same Event (a Conflict, as for any edit); an Incoming Sync Snapshot followed by a second change on the server to the same Event (no Conflict); two Events in Conflict in one Incoming Sync make two Server Event Replacements after one Incoming Sync Snapshot, which holds their zero-length Event Versions; an Incoming Sync whose only change is in Conflict still makes its Incoming Sync Snapshot; the trigger refuses a zero-length Event Version at any other Snapshot; an edit against a delete on the server makes the Event go; an Undo of a replacement restores the User's Version and sends it; the Mass-Change Guard doesn't count a replacement.

Spec: US-73 to US-76; "Sync engine"; "Calendar History". ADRs 0017 ("Part of the version"), 0018, 0019 ("Whether the server changed", "An Incoming Sync", "The invariant", "Conflicts end at once").

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] One function compares; no other code does
- [ ] An Incoming Sync writes its Incoming Sync Snapshot with every version fetched, zero-length for Events in Conflict, one Server Event Replacement per Event in Conflict with a copy of the Server Version, and the moved server's marks, in one Database Transaction
- [ ] A Server Event Replacement kept past 90 days (by an open Conflict or an Undo) keeps its Incoming Sync Snapshot with its zero-length Event Versions
- [ ] A Server Event Replacement refers to its Incoming Sync Snapshot by foreign key; the trigger allows zero-length Event Versions only at an Incoming Sync Snapshot
- [ ] A Server Event Replacement sends nothing
- [ ] After an Undo, an Incoming Sync takes the server's changes in at once, as after any change
- [ ] Ctrl+Z passes over Server Event Replacements; an Undo of one from the core restores the User's Version
- [ ] The tests listed above
- [ ] An edit back by hand (not undo) while the server changed the Event follows the server, with no Conflict
- [ ] Both sides changing an Event to the same content ends with the server's mark on the user's row
- [ ] An Event Version named by a Server Event Replacement survives dropping
- [ ] Propagate is gone from the code and `GLOSSARY.md`

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of four parts (see ticket 151). Every Conflict ticket (133 to 137, 21) builds on this one.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: A Join with one parent replaces the merge of an Incoming Sync. While the User's Pointer has a redo path, the second step waits (Held during Undo); the comparison still runs. Store Check rule widened (grilling, ADR 0019).
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). The comparison itself moved to ticket 167, which Undo needs first; the second step no longer waits for a redo path.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADR 0019 rewritten): the base of the comparison is the server's marks, not a Reference Snapshot; a Conflict ends at once with a Server Event Replacement instead of holding the Calendar; no Join. The comments above are history; the file name keeps the old title.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a Conflict is any Event changed differently on both sides, whatever the fields; the server's Unimplemented Fields no longer win inside the comparison. The Automatic Resolution is ticket 168.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0019): the Server Intake is now the Incoming Sync Snapshot and holds every version fetched; an Event in Conflict has a zero-length Event Version there, and its Server Event Replacement begins a copy and refers to the Incoming Sync Snapshot by foreign key. What can be combined becomes ticket 168's Server Event Merge.
- 2026-11-22: From point 4 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0019): the Server Event Replacement no longer names its Last Agreed Version, which for a Recurring Event is several Event Versions; the reference to the Incoming Sync Snapshot is a deferred foreign key, an edge of the drop's rule. A kept Incoming Sync Snapshot keeps its zero-length Event Versions, which the old conditions dropped once they had lost the server's mark.
