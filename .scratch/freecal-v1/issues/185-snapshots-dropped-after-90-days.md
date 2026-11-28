# 185: Snapshots dropped after 90 days

**What to build:** The dropping of old Snapshots (ADR 0018, "Keeping and dropping"), with the roots and edges that exist so far. Old ticket 148 (`git show design/snapshot-line:.scratch/freecal-v1/issues/148-snapshots-dropped-after-90-days.md`) as written, plus "since before <date>" in An Event's History (from old 160). No window work beyond that line's text.

- **One rule** (`history/drop.rs`): a Snapshot is kept while a root reaches it; the drop finds every Snapshot no root reaches and deletes them in one Database Transaction. Roots now: the Snapshots of the last 90 days. Edges now: an Undo's `undoes` to the Snapshot it takes back (180), a deferred foreign key, so a drop that misses it is refused at commit. Written so that a root or an edge is one entry each: 188 adds the server's mark (the Snapshot a version carrying it ended with), 190 the open Conflicts and the edge to their Last Agreed Versions.
- **What goes with a Snapshot:** each Event Version that ended with it (`end` `ON DELETE CASCADE`, 178); `begin` empties itself (`ON DELETE SET NULL`), meaning the version began before the oldest kept Snapshot. A current Event Version has no `end` and is never dropped.
- **When:** at start and once a day while FreeCal runs, in a Database Transaction of its own, replacing the Change Journal's 90-day removal that 182 took out. Decided here (the layout left it open): not at the start of each write, so a write never pays for a drop.
- **Ctrl+Z and redo** reach only kept Snapshots; after a drop the oldest kept one is the last Ctrl+Z reaches.
- **An Event's History** (184): where the oldest line's version began before the oldest kept Snapshot and isn't the Event's first, it reads "since before <date>", the date of the oldest kept Snapshot.
- **The Store Check** passes after a drop: `end` is after `begin` where both are set, every Undo's `undoes` names a kept Snapshot.

Spec: US-83, US-84, US-148; "Calendar History". ADRs 0018 ("Keeping and dropping", "An Event's History"), 0020. Layout: "Tables (end state)" (`event_version`'s `begin` and `end`); "Seams for `/tdd`", 1 (`history/drop.rs`); "By phase", P2, 185.

**Blocked by:** 182

**Status:** ready-for-agent

**Effort:** M

- [ ] Snapshots no root reaches are dropped in one Database Transaction, each with the Event Versions that ended with it; a Snapshot that a kept Undo takes back stays, with those Event Versions
- [ ] A drop that leaves an Undo's reference dangling is refused at commit (a test that breaks the rule by hand; ADR 0018 rejects generated histories)
- [ ] An Event unchanged for more than 90 days is still there after a drop, and so is every Calendar as the user sees it (the clock set forward)
- [ ] The drop runs at start and once a day
- [ ] An Event's History reads "since before <date>" after a drop
- [ ] Roots and edges are each one entry, ready for 188 and 190; the Store Check passes after every scenario
