# 158: Ctrl+Z and redo in the window: the toasts, refusals and their buttons

**What to build:** The window's part of undo (ADR 0020). The core has made Undos since ticket 147 and names the next Ctrl+Z's Snapshot and the next redo's Undo; until now the window showed a refusal in plain words. This ticket words every toast and adds their buttons. No undo asks a question.

- **What the core reports** for an Undo and a refusal (ticket 147): the Snapshot's origin, time, Calendars and description, and for a refusal the Occurrences in the way, each with the Snapshot that changed it later. This ticket adds what the toasts need and the core doesn't report yet: how many Events the step changes in each Calendar.
- **After an Undo:** "Undone: <what>" with "Redo", for 5 seconds; for a change on the server it names the Calendar, the time and the number of Events and adds "also on the server" for a Calendar with a server. The window names an Incoming Sync "change on the server", never "remote" or "sync". "Redo" redoes that Undo if it is still the newest in effect and is gone otherwise.
- **After a redo:** "Redone: <what>", without a button.
- **A refused undo:** a toast that stays until dismissed, naming the later change: “Couldn't undo "Move Standup": Standup was changed on the server since” or “… by "Resize Standup"” for the user's own, with "Open Standup" (several Events: the first, and the toast says how many) and "Undo both changes", which asks the core for the Undo with "both". Where the later change is an Incoming Sync Snapshot that replaced the Event: “Couldn't undo "Move Standup": Standup was replaced by the server's version”, with "Restore my version", the Undo of that Incoming Sync Snapshot limited to the Event (ticket 134), instead of "Undo both changes". A refused redo offers "Redo over the server's change" instead. A Snapshot spanning a Calendar marked read-only: “Couldn't undo "Move Standup": “Home” is read-only”, without a button.
- **When nothing is left:** "Nothing to undo" and "Nothing to redo".
- **The existing confirmation** (`src/lib/UndoDialog.svelte`, with `src/lib/undo.ts` and the undo and redo of the Window Dialogs, `src/lib/windowDialogs.svelte.ts`) goes; what is left of the Dangerous Undo's types and texts in the window and in `src/i18n/` goes with it.
- **A second Ctrl+Z while an undo runs** waits for it; a Snapshot arriving meanwhile (an Incoming Sync) doesn't change which Snapshot the running undo takes back.

Spec: US-146, US-150; "Calendar History"; "Toasts". ADR 0020.

**Blocked by:** 147

**Status:** ready-for-agent

**Effort:** S

- [ ] The core reports the number of Events changed per Calendar for an Undo
- [ ] Ctrl+Z undoes at once; the toast names what was undone, a change on the server with Calendar, time and number of Events, and offers "Redo" for exactly that Undo
- [ ] The redo toast has no button
- [ ] A refused undo names the later change, the server's or the user's own, with "Open" and "Undo both changes"; a refused redo offers "Redo over the server's change"; a read-only refusal has no button
- [ ] Nothing of the Dangerous Undo is left in the window or its texts
- [ ] Component tests on the fake core for each toast; core tests for the counts

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19 (with 147, 148, 159, 160). Blocked by 151 only: checked against tickets 152 to 154, which change what is compared and sent but not the choices an undo or a redo has.
- 2026-11-16: Chosen here, not in the spec: the wording of the two sides at a merge, and "Nothing to undo". For review.
- 2026-11-21: Merges now come only from ending a Conflict; a Join is undone with the question for a sync, with no choice of side (grilling, ADR 0019).
- 2026-11-21: Decided in a grilling: no question before undoing a Snapshot from a server; a toast after every undo, with "Redo", and after every redo, without a button. The window says "change on the server" for an Incoming Sync. Redo at a fork onto the server's own line after a Join asks as any fork (ADR 0020, unchanged).
- 2026-11-21: Imports no longer create Calendars (ADR 0018); the toast that stays no longer covers an import that created Calendars.
- 2026-11-21: Decided in a grilling on read-only Calendars (ADR 0020, "Read-only Calendars"): Ctrl+Z leaves out Calendars marked read-only. Where a Snapshot spans one, such as a move into it, Ctrl+Z asks before undoing the other Calendars' part alone ("“Home” is read-only. Undo the move in “Work” only? The Event will then be in both Calendars."); cancelling leaves it, and the next Ctrl+Z asks again. A Calendar whose writes are refused but that isn't marked is undone like any other.
- 2026-11-21: Rewritten from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten): an undo is an Undo Snapshot, so the questions at a merge, at a redo fork and between Calendars, and before undoing past a read-only Calendar, all go; refusals and their buttons come instead. Blocked by 147 alone, re-sized to S. The comments above are history.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): every server Calendar sends as made, so "also on the server" no longer depends on Tracking; the refusal by a Server Event Replacement offers "Restore my version" (ADR 0020).
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): "Restore my version" is the Undo of the Incoming Sync Snapshot limited to the Event.
