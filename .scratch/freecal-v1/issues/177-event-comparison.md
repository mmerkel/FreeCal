# 177: The comparison of the two sides, as one function

**What to build:** The Event Comparison: one pure function that compares two sides of one UID with what both are compared with, per Occurrence and by Field Group, and gives the outcome for the UID (ADR 0019, "Whether the server changed"; ADR 0020, "Where both changed"). Undo (180) and the Incoming Sync (189) both call it. Old ticket 167 (`git show design/snapshot-line:.scratch/freecal-v1/issues/167-comparison-of-the-two-sides.md`) plus old 21's rule per Occurrence for Recurring Events. Nothing is written; nothing calls it yet.

- **Where and what:** `core/src/comparison.rs`: `EventVersionContent` (managed fields and 175's tree, without row ID, Calendar or Snapshots; compared through the recurrence ID's `NormalForm` and the tree's canonical form, never by `derive`), `compare_event(base, ours, theirs, on_conflict) -> UidOutcome` over every single Event of the UID, keyed by recurrence ID. `OnConflict` is `Report` or `Win(Side)` ("… all over the later changes", ADR 0020). 177 settles `UidOutcome`'s variants.
- **Per Occurrence:** changed on one side only, that side's version; both to the same result, they agree (the outcome names both, so the caller keeps one row); both differently, compared by Field Group (the times with the time zone and repeat rule, the title, the Location, the description, the Unimplemented Fields): each group from the side that changed it where no group changed on both and the times didn't change against any group but the Unimplemented Fields; otherwise a Conflict, unless `Win` decides. "Changed" compares canonical forms. A deletion is a change; a deletion against any change is a Conflict.
- **Recurring Events** (from old 21): a change to the Recurring Event itself (any managed field, its rule and excluded dates included), its creation or deletion counts for every Occurrence; an Exception changed, created or removed counts for its own. A change to the Recurring Event against a change to one Occurrence on the other side is a Conflict on the whole UID, never combined: groups combine within one Event Version only. A Conflict on any Occurrence is a Conflict of the whole UID.
- **The groups** come from one exhaustive function over every managed field, destructured without `..`, so a managed field added later doesn't compile until it has a group.
- A helper builds the three maps for one single Event, for the simplest tests.

Spec: "Calendar History"; "Sync engine". ADRs 0017 ("Part of the version"), 0019 ("Whether the server changed", "A Server Event Merge"), 0020 ("Where both changed"). Layout: "New terms" (Event Version Content, Event Comparison, Sides); "Interfaces designed twice", "The comparison: `compare_event`"; "By phase", P1, 177.

**Blocked by:** 175

**Status:** ready-for-agent

**Effort:** M

- [ ] `EventVersionContent` and `compare_event` in `comparison.rs`, pure and unit-tested
- [ ] `Win(side)` decides every Occurrence changed on both sides
- [ ] Each outcome for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both
- [ ] Pairs of groups: the title against the Location combined; the times against the Unimplemented Fields combined; the times against the title, the same group on both, the Unimplemented Fields on both: Conflicts
- [ ] Recurring Events: changes to different Exceptions both kept; the Recurring Event changed on one side alone followed for every Occurrence; the Recurring Event against one Occurrence a Conflict of the UID; an Exception removed on one side and edited on the other; an excluded date against an Exception; an Exception's title against an alarm added to the same Exception combined
- [ ] The group function fails to compile when a managed field has no group (a comment shows how; no compile-fail test needed)
