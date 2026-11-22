# 160: An Event's History in its details

**What to build:** The view of one Event's versions (US-148), read through the Calendar History, without an undo of its own (ADR 0018, "An Event's History"). It brings back the History button that ticket 147 took out of the Event's details.

- **The core reads an Event's History** by Combined Event Key: the Event Versions of this Event, by `begin`, newest first, each with the time and origin of the Snapshot it began with. A Snapshot in which the Event didn't exist (deleted, then brought back by an undo or a sync) shows as a line "deleted". The history ends at the oldest kept Snapshot, whose version reads "since before <date>" when it isn't the Event's first.
- **For an Occurrence** of a Recurring Event the history is the Recurring Event's, and for an Exception its own, followed by the Recurring Event's from before the Exception existed.
- **Each line** says when, what made it (as the Calendar's History names origins, ticket 159) and which managed fields differ from the version before, with the old and the new value. A version that differs in the Unimplemented Fields alone reads "changed in fields FreeCal doesn't show" (ADR 0017); their content is not shown. An edit back by hand shows a line, and so does an Undo, whose copy is a new version (ADR 0018). A version a Server Intake took in shows "changes from the server"; one a Server Event Replacement brought shows "replaced by the server's version", and the replaced version before it stays in the list.
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
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: A version brought to the user's side by a merge shows the server's Snapshot it began with (grilling; origins stay on the Snapshot).
- 2026-11-21: A version taken in by a Join shows the Join; the merge rule stays for Conflict endings (grilling, ADR 0019).
- 2026-11-21: Origins are named as in ticket 159: an Incoming Sync or a Join reads "change on the server" (grilling).
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): an Event's History is read by `begin` on one line; merges, Snapshot Transitions and Joins are gone.
