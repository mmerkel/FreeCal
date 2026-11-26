# 135: A Conflict shows what each side changed

**What to build:** Each Conflict the core lists (ticket 133) also says, per field group, what each side changed against the Last Agreed Version kept in its Conflict Events (ADR 0019, "The Conflict tables"). Where there is no Last Agreed Version, it says where the two sides differ.

- **Computed on reading, never stored:** per Conflict Event, that is per Event of the UID (the Recurring Event and each Exception, by recurrence ID), the User's Version and the Server Version are each compared with the Last Agreed Version, by ticket 167's field groups.
- **What each side changed:** listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description and recurrence. Only managed fields are listed; a change to the Unimplemented Fields alone reads "changed details FreeCal doesn't show". A side may list nothing for an Event, where the other side's change met a change of its own elsewhere in the UID.
- **Without a Last Agreed Version** (both sides created the UID, or the base was untrusted, ticket 171), or for an Event on one side only: the two versions are compared with each other, and the fields where they differ are listed, without saying which side changed them.
- **A side without the Event** (deleted there) says only that.
- The words belong to the frontend (ticket 137). The core returns the changed fields as values; regenerate the TypeScript types.

Spec: US-73; "Core application interface"; "Untrusted content". ADR 0019 ("The Conflict tables", "Conflicts").

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says, per Event of the UID and per field group, what each side changed against the Last Agreed Version, computed on reading
- [ ] Without a Last Agreed Version, it says where the two sides differ
- [ ] A side without the Event says it was deleted there; changes only to Unimplemented Fields are named as such
- [ ] Written test-first

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): ticket 20 said "the common version from the Change Journal". It is now the Last Agreed Version (new in `GLOSSARY.md`), kept with the Conflict from the Mirror (ADR 0014's amendment of 2026-11-09), as the Change Journal keeps only 90 days and undos blur where the user's changes started.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The Last Agreed Version is read from the Reference Snapshot, not kept in `conflict_event`. The case "without a Last Agreed Version, every field counts as changed" is gone; its place is taken by the UID both sides created. Blocked by 133, which now has the list.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0019 rewritten): the Last Agreed Version is the one the Server Event Replacement names.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a change to the Unimplemented Fields alone is a Conflict now, but one an Automatic Resolution ends at once, so the listed ones are unchanged.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADR 0019): the Automatic Resolution goes; a Conflict now has the same field group changed on both sides, the times against another group, or a Recurring Event against one Occurrence, so a side may list no managed field.
- 2026-11-22: Reworded from point 4 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0019): the Server Event Replacement no longer names a Last Agreed Version, which for a Recurring Event is several Event Versions one column can't name, and which only this display used. The Conflict shows where the two versions differ instead of what each side changed. Retitled; the file name is kept for the links.
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): the Last Agreed Version is kept again, per Conflict Event (ticket 133), so the Conflict shows what each side changed, per field group; where the two sides differ is the fallback without one.
