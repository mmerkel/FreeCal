# 148: Snapshots dropped after 90 days

**What to build:** The last part of the Calendar History on the user's side (ADRs 0018 and 0020): the dropping of old Snapshots. No window work.

- **Dropping** (ADR 0018, "Keeping and dropping"), in this order: first the Event Versions whose `end` is older than 90 days, that have no server's mark and that no Server Event Replacement names as its Last Agreed Version; then the Snapshots older than 90 days that no kept Event Version's `end` names. `begin` empties itself (`ON DELETE SET NULL`), meaning the version began before the oldest kept Snapshot; `end` refuses the deletion of its Snapshot (`ON DELETE RESTRICT`), so a version the server still holds keeps it. Until ticket 151 no version has the server's mark, and until ticket 152 no Server Event Replacement exists; their cases are tested there. An Undo whose taken-back Snapshot is dropped keeps its description of what it took back and loses the reference, which is then empty; it can no longer be redone. Decision, for review: ADR 0020 says the reference is written once and doesn't state this case.
- **When:** at start and once a day while FreeCal runs, in one Database Transaction, where the Change Journal's 90-day removal ran before ticket 147.
- **The Store Check** passes after a drop: `end` is after `begin` and every Undo's reference names a kept Snapshot or is empty.

Spec: US-83, US-84; "Calendar History". ADRs 0018 ("Keeping and dropping"), 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** M

- [ ] Old Event Versions and then old Snapshots are dropped in that order, the foreign keys refusing what is still needed; an Undo of a dropped Snapshot keeps its description
- [ ] An Event unchanged for more than 90 days is still there after a drop, and so is the Calendar as the user sees it (test with the clock set forward)
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Fourth of four parts (see ticket 145).
- 2026-11-21: Decided in a grilling: links belong to a Snapshot Transition, not to a Snapshot; a Calendar's first Snapshot has a Snapshot Transition without a parent; the origin stays on the Snapshot (ADR 0018, "Snapshots").
- 2026-11-21: Imports no longer create Calendars (ADR 0018), so the case of a Calendar whose first Snapshot is the import's is gone.
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo for one Calendar goes (the Calendar History undoes one Snapshot in all the Calendars it spans); this ticket keeps only the dropping. Effort stays M for the rewrite of first Snapshots and the Undo reference.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0018 rewritten): one line for all Calendars, `begin` and `end` instead of links, so dropping is two deletes in one Database Transaction; the last common ancestor of the two sides is gone with the pointers.
