# 148: Snapshots dropped after 90 days

**What to build:** The last part of the Calendar History on the user's side (ADRs 0018 and 0020): the dropping of old Snapshots. No window work.

- **Dropping** (ADR 0018, "Keeping and dropping"), in this order: first the Event Versions whose `end` is older than 90 days, that have no server's mark, that no Server Event Replacement names as its Last Agreed Version and whose `end` isn't a Server Event Replacement without a Resolution, so that an open Conflict is never dropped; then the Snapshots older than 90 days that no kept Event Version's `end` names and that no kept Undo takes back. `begin` empties itself (`ON DELETE SET NULL`), meaning the version began before the oldest kept Snapshot; `end` refuses the deletion of its Snapshot (`ON DELETE RESTRICT`), so a version the server still holds keeps it. An Undo refers to the Snapshot it takes back with `ON DELETE RESTRICT` (ticket 147), so that Snapshot is kept as long as the Undo; the reference is never empty. Until ticket 151 no version has the server's mark, and until tickets 152 and 133 no Server Event Replacement or Resolution exists; their cases are tested there.
- **When:** at start and once a day while FreeCal runs, in one Database Transaction, where the Change Journal's 90-day removal ran before ticket 147.
- **The Store Check** passes after a drop: `end` is after `begin`, every Undo's reference names a kept Snapshot, and every open Conflict still has its User's Version.

Spec: US-83, US-84; "Calendar History". ADRs 0018 ("Keeping and dropping"), 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** M

- [ ] Old Event Versions and then old Snapshots are dropped in that order, the foreign keys refusing what is still needed; a Snapshot that a kept Undo takes back stays
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
