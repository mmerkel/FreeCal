# 135: A Conflict shows what each side changed

**What to build:** The third part of ticket 20 (read it for the history). Each Conflict the core lists (ticket 134) also says what each side changed against the Last Agreed Version: "you changed the time, they changed the title".

- **Computed on reading, never stored:** the User's Version (the Event Rows) and the Server Version (`conflict_event`) are each compared with the Last Agreed Version that ticket 133 keeps in `conflict_event`. So a later sync or local edit (ticket 138) needs nothing recomputed.
- **Changed fields:** each side's changes are listed as the user reads them, not as columns: time (start, end, all-day and time zone together), title, location, description and recurrence. A side that deleted the Event says only that.
- **Without a Last Agreed Version** (the previous Server Version couldn't be read), every field counts as changed on both sides, as ADR 0014 says for the server side. The window then shows both versions without the changes (ticket 137).
- The words belong to the frontend (ticket 137). The core returns the changed fields as values.

Spec: "Untrusted content". ADR 0014.

**Blocked by:** 134

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says which fields each side changed against the Last Agreed Version, computed on reading
- [ ] A deleted side says it deleted the Event
- [ ] Without a Last Agreed Version, the Conflict says so, and no changes are listed
- [ ] Written test-first

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): ticket 20 said "the common version from the Change Journal". It is now the Last Agreed Version (new in `GLOSSARY.md`), kept with the Conflict from the Mirror (ADR 0014's amendment of 2026-11-09), as the Change Journal keeps only 90 days and undos blur where the user's changes started.
