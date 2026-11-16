# 160: An Event's History in its details

**What to build:** The view of one Event's versions (US-148), read through the Calendar History, without an undo of its own (ADR 0018, "An Event's History"). It brings back the History button that ticket 147 took out of the Event's details.

- **The core reads an Event's History** by Combined Event Key: the Event Versions that this Event had in the Snapshots its Calendar's User's Pointer is on or descends from, newest first, each with the time and origin of the Snapshot it began with. A Snapshot in which the Event didn't exist (deleted, then brought back by an undo or a sync) shows as a line "deleted". The history ends at the oldest kept Snapshot, whose version reads "since before <date>" when it isn't the Event's first.
- **For an Occurrence** of a Recurring Event the history is the Recurring Event's, and for an Exception its own, followed by the Recurring Event's from before the Exception existed.
- **Each line** says when, what made it (as the Calendar's History names origins, ticket 159) and which managed fields differ from the version before, with the old and the new value. A version that differs in the Unimplemented Fields alone reads "changed in fields FreeCal doesn't show" (ADR 0017); their content is not shown.
- **The window:** the Event's details (`src/lib/EventDetails.svelte`) get the History button back, for read-only Events too. No line has an Undo or a "restore" (ADR 0020, "Consequences").
- **Untrusted content:** old titles, descriptions and locations are stored fields and are shown as text only.

Spec: US-148; "Calendar History"; "Untrusted content". ADRs 0017 ("Part of the version"), 0018, 0020.

**Blocked by:** 149

**Status:** ready-for-agent

**Effort:** S

- [ ] The core returns an Event's versions with time and origin, for a single Event, a Recurring Event and an Exception
- [ ] A delete followed by an undo shows as "deleted" and then the version again
- [ ] A change in the Unimplemented Fields alone reads "changed in fields FreeCal doesn't show"
- [ ] The details show the History for writable and read-only Events, with no way to undo from it
- [ ] After Snapshots were dropped (ticket 148), the oldest line reads "since before <date>"
- [ ] Core tests through the core interface; a component test on the fake core

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19.
- 2026-11-16: Chosen here, for review: the history follows the user's side (what FreeCal showed), so a version the server had that never reached the user's side isn't listed; and the "deleted" line.
