# 135: A Conflict shows what each side changed

**What to build:** The third part of ticket 20 (read it for the history). Each Conflict the core lists (ticket 134) also says what each side changed against the common version from the Change Journal: "you changed the time, they changed the title".

- **The common version:** the Event Version the user's changes started from. Decide which one, for example the previous Event Version of the first Journal Event Modification behind the Event's Unsent Changes, and record the choice. A First Sync isn't recorded, so the common version can be one the First Sync wrote: the previous version of the user's edit.
- **Changed fields:** each side's changes are listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description, recurrence, and deleted. A side that deleted the Event says only that.
- **Without a common version** (its Journal Event Modification is older than the Change Journal keeps, 90 days), the Conflict lists both versions without the changes.
- The words belong to the frontend (ticket 137). The core returns the changed fields as values.

Spec: "Change Journal"; "Untrusted content". ADRs 0009, 0014.

**Blocked by:** 134

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says which fields each side changed against the common version from the Change Journal
- [ ] A deleted side says it deleted the Event
- [ ] Without a common version, the Conflict lists both versions without the changes
- [ ] Written test-first

## Comments
- 2026-11-09: Split from ticket 20.
