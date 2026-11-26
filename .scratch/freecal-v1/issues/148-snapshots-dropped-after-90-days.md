# 148: Snapshots dropped after 90 days

**What to build:** The last part of the Calendar History on the user's side (ADRs 0018 and 0020): the dropping of old Snapshots. No window work.

- **Dropping** (ADR 0018, "Keeping and dropping"): one rule for Snapshots. A Snapshot is kept while a root reaches it; the drop finds every Snapshot no root reaches and deletes them in one Database Transaction. This ticket's root is the Snapshots of the last 90 days and its edge an Undo's reference to the Snapshot it takes back (ticket 147), a deferred foreign key (`DEFERRABLE INITIALLY DEFERRED`), so a drop that misses it is refused at commit. The other roots and edges come with the tickets that add their columns: the Snapshot with which a version carrying the server's mark ended, and the trigger guarding that version (ticket 151); the Incoming Sync Snapshot of an open Conflict, the edge through its Conflict Events to the Snapshots with which their Last Agreed Versions ended, and the trigger guarding an open Conflict (ticket 133). An Event Version goes with the Snapshot it ended with (`end` with `ON DELETE CASCADE`, ticket 146); `begin` empties itself (`ON DELETE SET NULL`), meaning the version began before the oldest kept Snapshot. A current Event Version has no `end` and is never dropped.
- **When:** at start and once a day while FreeCal runs, in one Database Transaction, where the Change Journal's 90-day removal ran before ticket 147.
- **Tests:** one scenario per root and per edge, and one in which a missed edge is refused at commit (ADR 0018 rejects generated histories). **The Store Check** passes after a drop: `end` is after `begin`, every Undo's reference names a kept Snapshot, and every open Conflict still has its User's Version.

Spec: US-83, US-84; "Calendar History". ADRs 0018 ("Keeping and dropping"), 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** M

- [ ] Snapshots no root reaches are dropped in one Database Transaction, each with the Event Versions that ended with it; a Snapshot that a kept Undo takes back stays, with those Event Versions
- [ ] A drop that leaves an Undo's reference dangling is refused at commit
- [ ] An Event unchanged for more than 90 days is still there after a drop, and so is the Calendar as the user sees it (test with the clock set forward)
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Fourth of four parts (see ticket 145).
- 2026-11-21: Decided in a grilling: links belong to a Snapshot Transition, not to a Snapshot; a Calendar's first Snapshot has a Snapshot Transition without a parent; the origin stays on the Snapshot (ADR 0018, "Snapshots").
- 2026-11-21: Imports no longer create Calendars (ADR 0018), so the case of a Calendar whose first Snapshot is the import's is gone.
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo for one Calendar goes (the Calendar History undoes one Snapshot in all the Calendars it spans); this ticket keeps only the dropping. Effort stays M for the rewrite of first Snapshots and the Undo reference.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0018 rewritten): one line for all Calendars, `begin` and `end` instead of links, so dropping is two deletes in one Database Transaction; the last common ancestor of the two sides is gone with the pointers.
- 2026-11-22: Agreed by the user: an Undo of a dropped Snapshot loses its reference and can't be redone.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a Snapshot that a kept Undo takes back is no longer dropped (`ON DELETE RESTRICT`), reversing the agreement of 2026-11-22 above that such an Undo loses its reference; the Event Versions of an open Conflict are kept. Tests for open Conflicts come with ticket 133.
- 2026-11-22: Rewritten from point 4 of `.scratch/freecal-v1/critiques-2.md` (ADR 0018, "Keeping and dropping"): retention is one rule of roots and edges over Snapshots, an Event Version going with its `end` Snapshot, in place of the conditions on Event Versions and on Snapshots. The references are deferred foreign keys, not `ON DELETE RESTRICT`. Each later ticket adds its own root or edge. Tested by scenarios, not generated histories.
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): the edge from a Server Event Merge or Replacement to its Incoming Sync Snapshot and the Resolution's edge go; ticket 133 adds the Conflict's root and edge.
