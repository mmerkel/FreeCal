# 152: The comparison against the server's marks; Server Intakes and Server Event Replacements; Propagate goes

**What to build:** An Incoming Sync compares what it fetched with the user's current versions and the server's marks (ADR 0019, rewritten 2026-11-22). What is in no Conflict goes into the Server Intake; each Event in Conflict gets a Server Event Replacement, in which it takes the Server Version. No Calendar stops syncing. Propagate comes out. Reporting Conflicts and the Resolution are tickets 133 and 134.

- **The comparison** is ticket 167's function, called per Occurrence with the fetched versions, the user's current ones and the ones with the server's mark (what the server held when FreeCal last looked), the server's side winning in the Unimplemented Fields:
  - only the server changed it: it follows the server, in the Server Intake;
  - only the user changed it: it stays to be sent;
  - both changed it to the same result: they agree, and the server's mark moves to the user's row;
  - both changed it differently: a Conflict.
  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. "Changed" compares content: the same row is unchanged, different rows are compared by canonical form, so an edit back to what the server has is no change. For the Unimplemented Fields, if only one side changed them that side's are taken, and if both did the server's win and no Conflict is raised.
- **An Incoming Sync** writes in one Database Transaction, and no code may split it:
  1. a Server Intake with every change that is in no Conflict;
  2. a Server Event Replacement per Event (Calendar and UID) in Conflict, one Snapshot each, in which that Event's versions take the Server Version, an edit against a deletion on the server too. Its origin names the Event Version that had the server's mark before the fetch, the Last Agreed Version, as a foreign key; it is empty where the UID wasn't on the server before;
  3. the server's marks moved to the fetched versions, with their ETags.
  A Server Intake or Server Event Replacement that would change nothing isn't made. Nothing is sent for a replacement.
- **Dropping** (ticket 148) keeps an Event Version a Server Event Replacement names; its test comes here.
- **Unsent Changes.** Glue until ticket 153, named: after the Incoming Sync, the Calendar's Unsent Changes are set to the Combined Event Keys whose current Event Version differs from the one with the server's mark, keeping the time and the reason of those that stay.
- **What comes out:** Propagate and its special cases for Unsent Changes (`core/src/sync/`, `core/src/journal/mod.rs`'s successor), with the `GLOSSARY.md` entry Propagate a Server Object.
- **Until ticket 133,** a replacement shows nothing beyond the Server Version in the grid. That is acceptable between tickets, as no real Local Store exists.
- **Tests,** through the core interface against Radicale and as unit tests of the comparison: each of the four outcomes, for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both; a change to a Recurring Event against a change to one Occurrence; an Undo not yet sent followed by another change on the server to the same Event (a Conflict, as for any edit); a Server Intake followed by a second change on the server to the same Event (no Conflict); two Events in Conflict in one Incoming Sync make two Server Event Replacements beside one Server Intake; an edit against a delete on the server makes the Event go; an Undo of a replacement restores the User's Version and sends it; the Mass-Change Guard doesn't count a replacement.

Spec: US-73 to US-76; "Sync engine"; "Calendar History". ADRs 0017 ("Part of the version"), 0018, 0019 ("Whether the server changed", "An Incoming Sync", "The invariant", "Conflicts end at once").

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] One function compares; no other code does
- [ ] An Incoming Sync writes its Server Intake, one Server Event Replacement per Event in Conflict and the moved server's marks in one Database Transaction
- [ ] A Server Event Replacement names its Last Agreed Version by foreign key and sends nothing
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
