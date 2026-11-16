# 135: A Conflict shows what each side changed

**What to build:** Each Conflict the core lists (ticket 133) also says what each side changed against the Last Agreed Version: "you changed the time, they changed the title".

- **Computed on reading, never stored:** the User's Version and the Server Version are each compared with the Last Agreed Version, which ticket 133 reads from the Reference Snapshot. The Reference Snapshot is always set (ADR 0019), so there is always something to compare with.
- **Changed fields:** each side's changes are listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description and recurrence. Only managed fields are listed; a change to the Unimplemented Fields alone is no Conflict and isn't shown.
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
