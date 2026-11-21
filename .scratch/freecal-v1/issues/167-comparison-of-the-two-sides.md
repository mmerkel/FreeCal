# 167: The comparison of the two sides, as one function

**What to build:** The one function that compares two sides with what both are compared with, per Occurrence (ADR 0019, "Whether the server changed"). An Undo needs it before sync does: an Undo is that comparison with the Snapshot taken back as what both are compared with (ADR 0020). Ticket 147 uses it for Undo and redo, ticket 152 for sync. It moved here from ticket 152, so that both use one function rather than two that agree (decided 2026-11-21: one rule built earlier, in place of an Undo with a comparison of its own that sync would replace later).

- **Input:** three sets of Event Versions of one Calendar: one side, the other side, and what both are compared with. Optionally, the side that wins wherever both changed an Occurrence (for "Undo both changes" and for a user's choices in a Conflict, ticket 134). **Output,** per Occurrence: unchanged, changed on one side only (with that side's Event Version), changed on both to the same result, or a Conflict. Nothing is written; the caller makes the Snapshot.
- **The rules,** at the grain Propagate has today (ticket 21 refines it for Recurring Events):
  - only one side changed it: that side's version;
  - both changed it to the same result: they agree, and the result names both rows, so that the caller keeps one (in sync the user's side takes over the server's row, ADR 0018, "Event Versions");
  - both changed it differently: a Conflict, unless a winning side is given.

  A change to the Recurring Event itself counts as a change to every Occurrence. A deletion is a change. "Changed" compares content with what both are compared with: the same row is unchanged, different rows are compared by canonical form. The managed fields and the Unimplemented Fields are two parts: for the Unimplemented Fields, if only one side changed them that side's are taken, and if both did, the side the caller names wins and no Conflict is raised: the server's for sync (ticket 152), the User's Pointer's for an Undo, so that the later change stays.
- **Tests,** as unit tests of the function, one per line above and more: each outcome for an edit, a creation and a deletion on either side; the Unimplemented Fields changed on one side and on both; a change to a Recurring Event against a change to one Occurrence; a winning side given.

Spec: "Calendar History"; "Sync engine". ADRs 0017 ("Part of the version"), 0019 ("Whether the server changed"), 0020.

**Blocked by:** 145

**Status:** ready-for-agent

**Effort:** S

- [ ] One function compares two sides with a third set per Occurrence and writes nothing
- [ ] A winning side, when given, decides every Occurrence changed on both sides
- [ ] The unit tests listed above

## Comments

- 2026-11-21: Written from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`): Undo is a Snapshot made by this comparison (ADR 0020), so the comparison comes before ticket 147, not with ticket 152.
