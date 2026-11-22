# 135: A Conflict shows where the two sides differ

**What to build:** Each Conflict the core lists (ticket 133) also says in which fields the User's Version and the Server Version differ. It doesn't say which side changed what: no Last Agreed Version is kept after the Incoming Sync (ADR 0019), and the user knows which change was theirs.

- **Computed on reading, never stored:** the User's Version is compared with the Server Version, Event by Event of the UID (the Recurring Event and each Exception, matched by Combined Event Key).
- **Differing fields:** listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description and recurrence. Only managed fields are listed; where the two differ only in Unimplemented Fields, the Conflict says "differs in details FreeCal doesn't show".
- **A side without the Event** (deleted there) says only that. Where both sides created the UID, both versions are complete and their differing fields are listed as usual.
- The words belong to the frontend (ticket 137). The core returns the differing fields as values; regenerate the TypeScript types.

Spec: US-73; "Core application interface"; "Untrusted content". ADR 0019 ("Conflicts").

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says in which fields the two sides differ, per Event of the UID, computed on reading
- [ ] A side without the Event says it was deleted there; differences only in Unimplemented Fields are named as such
- [ ] Written test-first

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): ticket 20 said "the common version from the Change Journal". It is now the Last Agreed Version (new in `GLOSSARY.md`), kept with the Conflict from the Mirror (ADR 0014's amendment of 2026-11-09), as the Change Journal keeps only 90 days and undos blur where the user's changes started.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The Last Agreed Version is read from the Reference Snapshot, not kept in `conflict_event`. The case "without a Last Agreed Version, every field counts as changed" is gone; its place is taken by the UID both sides created. Blocked by 133, which now has the list.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0019 rewritten): the Last Agreed Version is the one the Server Event Replacement names.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a change to the Unimplemented Fields alone is a Conflict now, but one an Automatic Resolution ends at once, so the listed ones are unchanged.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADR 0019): the Automatic Resolution goes; a Conflict now has the same field group changed on both sides, the times against another group, or a Recurring Event against one Occurrence, so a side may list no managed field.
- 2026-11-22: Reworded from point 4 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018, 0019): the Server Event Replacement no longer names a Last Agreed Version, which for a Recurring Event is several Event Versions one column can't name, and which only this display used. The Conflict shows where the two versions differ instead of what each side changed. Retitled; the file name is kept for the links.
