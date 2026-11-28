# 167: The comparison of the two sides, as one function

**What to build:** The one function that compares two sides with what both are compared with, per Occurrence (ADR 0019, "Whether the server changed"). An Undo needs it before sync does: an Undo is that comparison with the Snapshot taken back as what both are compared with (ADR 0020). Ticket 147 uses it for Undo and redo, ticket 152 for sync. It moved here from ticket 152, so that both use one function rather than two that agree (decided 2026-11-21: one rule built earlier, in place of an Undo with a comparison of its own that sync would replace later).

- **Input:** three sets of Event Versions of one Calendar: one side, the other side, and what both are compared with. Optionally, the side that wins wherever both changed an Occurrence (for "Undo all over the later changes" and for a user's choices in a Conflict, ticket 134). **Output,** per Occurrence: unchanged, changed on one side only (with that side's Event Version), changed on both to the same result, or a Conflict, which says whether it can be combined and, if so, the combination. Nothing is written; the caller makes the Snapshot.
- **The rules,** at the grain Propagate has today (ticket 21 refines it for Recurring Events):
  - only one side changed it: that side's version;
  - both changed it to the same result: they agree, and the result names both rows, so that the caller keeps one (in sync the user's side takes over the server's row, ADR 0018, "Event Versions");
  - both changed it differently: compared by **field group** (ADR 0019): the times (with the time zone and the repeat rule), the title, the Location, the description and the Unimplemented Fields. Where each group changed on at most one side, it is combined, each group from the side that changed it, and the output gives the combination; where both changed the same group, or one side the times and the other any other group but the Unimplemented Fields, it is a Conflict; unless a winning side is given, which decides it. The same rule serves sync and Undo; no caller names a side for the Unimplemented Fields (ADRs 0017, 0019, 0020, decided 2026-11-22).

  A change to the Recurring Event itself counts as a change to every Occurrence, and against a change to one Occurrence on the other side it is a Conflict on the whole Recurring Event, never combined: groups are compared within one Event Version. A deletion is a change, and a deletion against any change is a Conflict. "Changed" compares content with what both are compared with: the same row is unchanged, different rows are compared by canonical form. **The groups** are defined in one function that destructures the Event Version's managed fields without `..`, so a managed field added later doesn't compile until it is given a group.
- **Tests,** as unit tests of the function, one per line above and more: each outcome for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both; each pair of groups changed on the two sides, combined or a Conflict (the title against the Location, the times against the title, the times against the Unimplemented Fields, the same group on both, the Unimplemented Fields on both); a change to a Recurring Event against a change to one Occurrence; a winning side given.

Spec: "Calendar History"; "Sync engine". ADRs 0017 ("Part of the version"), 0019 ("Whether the server changed"), 0020.

**Blocked by:** 145

**Status:** ready-for-agent

**Effort:** S

- [ ] One function compares two sides with a third set per Occurrence and writes nothing
- [ ] A winning side, when given, decides every Occurrence changed on both sides
- [ ] Changes to different field groups are combined, the times only with the Unimplemented Fields; the same group on both sides, or a Recurring Event against one Occurrence, is a Conflict
- [ ] The groups come from one function that fails to compile when a managed field has no group
- [ ] The unit tests listed above

## Comments

- 2026-11-21: Written from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`): Undo is a Snapshot made by this comparison (ADR 0020), so the comparison comes before ticket 147, not with ticket 152.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the base for sync is the server's marks (ticket 152); the User's Pointer is gone.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): both sides changing an Occurrence differently is a Conflict whatever the fields; the function says whether it can be combined and gives the combination, which the Automatic Resolution (ticket 168) and an Undo (ticket 147) use.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0017, 0019, 0020): both sides changing an Occurrence differently is compared by field group; changes to different groups are combined, the times kept apart, and nothing is combined across a Recurring Event and its Exceptions. One rule for sync and Undo: the caller no longer names a side for the Unimplemented Fields (point 3 of `critiques-2.md`).
- 2026-11-28: Reworded from the grilling on point 5 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0020): "Undo both changes" is now "Undo all over the later changes".
