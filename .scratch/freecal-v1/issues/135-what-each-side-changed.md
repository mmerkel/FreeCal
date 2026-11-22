# 135: A Conflict shows what each side changed

**What to build:** Each Conflict the core lists (ticket 133) also says what each side changed against the Last Agreed Version: "you changed the time, they changed the title".

- **Computed on reading, never stored:** the User's Version and the Server Version are each compared with the Last Agreed Version, which ticket 133 reads from the Server Event Replacement's origin. An empty one means both sides created the UID (below).
- **Changed fields:** each side's changes are listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description and recurrence. Only managed fields are listed. Changes to different field groups were combined in a Server Event Merge (ticket 168) and aren't Conflicts. A listed Conflict has the same group changed on both sides, or the times on one side against another managed field on the other, or a whole Recurring Event against one Occurrence; where only the Unimplemented Fields changed on both sides, each side says "changed in details FreeCal doesn't show".
- **A side that deleted the Event** says only that. **An empty Last Agreed Version** (both sides created the UID) makes each side say that it created the Event, with no fields listed.
- The words belong to the frontend (ticket 137). The core returns the changed fields as values; regenerate the TypeScript types.

Spec: US-73; "Core application interface"; "Untrusted content". ADR 0019 ("Conflicts").

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says which fields each side changed against the Last Agreed Version, computed on reading
- [ ] A deleted side says it deleted the Event; with an empty Last Agreed Version each side says it created it
- [ ] Written test-first

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): ticket 20 said "the common version from the Change Journal". It is now the Last Agreed Version (new in `GLOSSARY.md`), kept with the Conflict from the Mirror (ADR 0014's amendment of 2026-11-09), as the Change Journal keeps only 90 days and undos blur where the user's changes started.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The Last Agreed Version is read from the Reference Snapshot, not kept in `conflict_event`. The case "without a Last Agreed Version, every field counts as changed" is gone; its place is taken by the UID both sides created. Blocked by 133, which now has the list.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0019 rewritten): the Last Agreed Version is the one the Server Event Replacement names.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a change to the Unimplemented Fields alone is a Conflict now, but one an Automatic Resolution ends at once, so the listed ones are unchanged.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADR 0019): the Automatic Resolution goes; a Conflict now has the same field group changed on both sides, the times against another group, or a Recurring Event against one Occurrence, so a side may list no managed field.
