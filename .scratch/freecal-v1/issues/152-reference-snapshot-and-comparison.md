# 152: The comparison against the server's marks; Incoming Sync Snapshots and Server Event Replacements; Propagate goes

**What to build:** An Incoming Sync compares what it fetched with the user's current versions and the server's marks (ADR 0019) and writes one Incoming Sync Snapshot per Calendar. Every version fetched begins with it; for an Event in Conflict, the server's version becomes current at once and the User's Version ends, a **Server Event Replacement**. No Calendar stops syncing. Propagate comes out. Recording and reporting Conflicts and resolving them are tickets 133 and 134.

- **The comparison** is ticket 167's function, called per Occurrence with the fetched versions, the user's current ones and the ones with the server's mark (what the server held when FreeCal last looked):
  - only the server changed it: it follows the server, in the Incoming Sync Snapshot;
  - only the user changed it: it stays to be sent;
  - both changed it to the same result: they agree, and the server's mark moves to the user's row;
  - both changed it differently: a Conflict, or combinable by field group (ticket 167).
  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. "Changed" compares content: the same row is unchanged, different rows are compared by canonical form, so an edit back to what the server has is no change. A change to the Unimplemented Fields is a change like any other. Combining what can be combined is ticket 168's Server Event Merge; until then an Occurrence the comparison could combine is treated as a Conflict and replaced.
- **An Incoming Sync** writes in one Database Transaction, and no code may split it, one Incoming Sync Snapshot per Calendar with:
  1. for an Event the server alone changed: the server's versions begin with it and become current;
  2. for an Event (Calendar and UID) in Conflict, a Server Event Replacement: the server's versions begin with it and become current, carrying both marks, an edit against a deletion on the server too; the User's Version ends with it. No copy and no zero-length Event Version. The branch that decides a Conflict is where ticket 133 writes its row of `conflict`;
  3. the server's marks moved to the fetched versions, with their ETags.
  A Snapshot that would change nothing isn't made, the Incoming Sync Snapshot included. Nothing is sent for a replacement.
- **Unsent Changes.** Glue until ticket 153, named: after the Incoming Sync, the Calendar's Unsent Changes are set to the Combined Event Keys whose current Event Version differs from the one with the server's mark, keeping the time and the reason of those that stay.
- **What comes out:** Propagate and its special cases for Unsent Changes (`core/src/sync/`, `core/src/journal/mod.rs`'s successor), with the `GLOSSARY.md` entry Propagate a Server Object.
- **Until ticket 133,** a replacement shows nothing beyond the Server Version in the grid, and its User's Version is dropped after 90 days like any other. That is acceptable between tickets, as no real Local Store exists.
- **Tests,** through the core interface against Radicale and as unit tests of the comparison: each of the four outcomes, for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side (followed or sent) and on both (a Conflict); a change to a Recurring Event against a change to one Occurrence; an Undo not yet sent followed by another change on the server to the same Event (a Conflict, as for any edit); an Incoming Sync Snapshot followed by a second change on the server to the same Event (no Conflict); two Events in Conflict and one the server alone changed in one Incoming Sync, all in its one Incoming Sync Snapshot; an Incoming Sync whose only change is in Conflict makes its Incoming Sync Snapshot; an Incoming Sync that changes nothing makes none; an edit against a delete on the server makes the Event go; an Undo of the Incoming Sync Snapshot limited to a replaced Event restores the User's Version and sends it; the Mass-Change Guard doesn't count a replacement.

Spec: US-73 to US-76; "Sync engine"; "Calendar History". ADRs 0017 ("Part of the version"), 0018, 0019 ("Whether the server changed", "An Incoming Sync", "The invariant", "Conflicts end at once").

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] One function compares; no other code does
- [ ] An Incoming Sync writes one Incoming Sync Snapshot per Calendar with every version fetched, the server's version current for Events in Conflict and the User's Version ended, and the moved server's marks, in one Database Transaction
- [ ] An Incoming Sync that would change nothing makes no Snapshot
- [ ] A Server Event Replacement sends nothing
- [ ] After an Undo, an Incoming Sync takes the server's changes in at once, as after any change
- [ ] Ctrl+Z passes over Incoming Sync Snapshots; an Undo of one limited to a replaced Event restores the User's Version
- [ ] The tests listed above
- [ ] An edit back by hand (not undo) while the server changed the Event follows the server, with no Conflict
- [ ] Both sides changing an Event to the same content ends with the server's mark on the user's row
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
- 2026-11-26: Rewritten from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): a Server Event Replacement is what the Incoming Sync Snapshot does to an Event in Conflict, no longer a Snapshot of its own: the server's version is current at once, with no copy and no zero-length Event Version. The Snapshot column pointing to the Incoming Sync Snapshot, its CHECK, its edge and its Store Check rule go; zero-length Event Versions and their trigger move to ticket 168, the only one that needs them; writing the row of `conflict` is ticket 133's.
