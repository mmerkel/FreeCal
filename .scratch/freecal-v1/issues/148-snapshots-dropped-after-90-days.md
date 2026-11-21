# 148: Snapshots dropped after 90 days

**What to build:** The last part of the Calendar History on the user's side (ADRs 0018 and 0020): the dropping of old Snapshots. No window work.

- **Dropping:** Snapshots older than 90 days are dropped, except the last common ancestor of any Snapshot younger than 90 days that only one side has reached (with the user's side alone this exception has no case; ticket 151 adds its test). The oldest Snapshot kept in a Calendar becomes its first: its Snapshot Transition in that Calendar loses its parent, and its links say that every Event Version it holds begins with it, and no Undo takes it back. An Event Version goes when no kept Snapshot refers to it and it has no Current-Version Mark. A Snapshot spanning Calendars is dropped per Calendar. An Undo whose taken-back Snapshot is dropped keeps its description of what it took back and loses the reference, which is then empty; it is no longer redone, as redo needs the Undo's parent, which is older still. Decision, for review: ADR 0020 says the reference is written once and doesn't state this case.
- **When:** at start and once a day while FreeCal runs, in one Database Transaction per Calendar, where the Change Journal's 90-day removal ran before ticket 147.
- **The Store Check** passes after a drop: every Event Version is still referred to or marked, and every pointer names a kept Snapshot.

Spec: US-83, US-84; "Calendar History". ADRs 0018 ("Keeping and dropping"), 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** M

- [ ] Snapshots older than 90 days are dropped with the Event Versions only they held; the oldest kept one is the Calendar's first and no Undo takes it back; an Undo of a dropped Snapshot keeps its description
- [ ] An Event unchanged for more than 90 days is still there after a drop, and so is the Calendar as the user sees it (test with the clock set forward)
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Fourth of four parts (see ticket 145).
- 2026-11-21: Decided in a grilling: links belong to a Snapshot Transition, not to a Snapshot; a Calendar's first Snapshot has a Snapshot Transition without a parent; the origin stays on the Snapshot (ADR 0018, "Snapshots").
- 2026-11-21: Imports no longer create Calendars (ADR 0018), so the case of a Calendar whose first Snapshot is the import's is gone.
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo for one Calendar goes (the Calendar History undoes one Snapshot in all the Calendars it spans); this ticket keeps only the dropping. Effort stays M for the rewrite of first Snapshots and the Undo reference.
