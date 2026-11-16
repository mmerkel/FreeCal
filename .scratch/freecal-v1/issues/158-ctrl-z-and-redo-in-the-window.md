# 158: Ctrl+Z and redo in the window: the questions before a sync, at a merge and at a redo fork, and the toasts' wording

**What to build:** The window's part of undo as a stack (ADR 0020). The core has undone and redone by moving the User's Pointer since ticket 147, and has named the choices at a merge and at a fork since ticket 151; until now the window went back through the user's side and forward the way it came. This ticket makes the window ask, and rewords what it says.

- **What the core reports** for the next undo and the next redo (tickets 147 and 151): the Snapshot's origin, its time, its Calendars and the choices it has. This ticket adds what the questions need and the core doesn't report yet: how many Events the step changes in each Calendar, and for a merge or a fork the same for each way.
- **Before undoing a Snapshot from a server** (an Incoming Sync, or a merge), Ctrl+Z asks: "Undo the sync of “Work” from 14:02? 3 Events go back to how they were, on the server too." For a Calendar that isn't Tracking, or a Local or Server read-only one, the sentence leaves out "on the server too". Nothing else asks: a user's action, an import and a Backup restore are undone at once.
- **At a merge,** the question also asks which side to go back through: "Before the server's changes" (the user's side) or "Before my changes" (the server's side), each with its number of Events. One dialog, not two.
- **Redo** (Ctrl+Shift+Z and Ctrl+Y) redoes at once when exactly one Snapshot is next beyond the User's Pointers of all Calendars. Otherwise it asks which, in one dialog that lists every choice by Calendar, origin, time and number of Events:
  - between Calendars that each have a Snapshot to bring back ("Redo in “Work” (your change from 10:05) or in “Home” (your change from 10:10)?"); the ones not chosen stay and can be redone next. This also happens after undoing one change in each of two Calendars (ADR 0020, "Consequences");
  - which way where two Snapshots arose from the one a pointer is at, with the note that the other way is then gone for good.

  A Snapshot spanning several Calendars that is next in only some of them is offered for those, and the question says so. It also brings back what an undo for one Calendar took back ("Go back to before this", ticket 159), one Snapshot per press.
- **Read-only Calendars are left out:** their User's Pointer follows their Server's Pointer, so Ctrl+Z never takes their Snapshots. If the core's choice of the newest Snapshot (ticket 147) doesn't leave them out yet, it does here.
- **Toasts:** "Undone: <what>" and "Redone: <what>", for 5 seconds, naming the step by its origin (as the question does for a sync). No toast offers Undo or Redo. When nothing is left: "Nothing to undo" and "Nothing to redo".
- **A toast that stays until dismissed** when the step left something behind (ticket 148's answer): a Snapshot spanning Calendars that one Calendar had already left, and an import that created Calendars, which is taken back in the Calendars that existed and stays in the ones it created. It names the Calendars left as they are.
- **The existing confirmation** (`src/lib/UndoDialog.svelte`, with `src/lib/undo.ts` and the undo and redo of the Window Dialogs, `src/lib/windowDialogs.svelte.ts`) is rewritten to these questions; what is left of the Dangerous Undo's types and texts in the window and in `src/i18n/` goes.
- **While a question shows,** a second Ctrl+Z does nothing; and if a Snapshot arrives while it shows (an Incoming Sync), the answer applies to the step that was asked about or the undo is dropped with a toast, never to another step.
- **No Conflict rules here:** while a Calendar has a Conflict, undo and redo move its User's Pointer and send nothing, and the comparison runs again (ticket 138).

Spec: US-146, US-150; "Calendar History"; "Toasts". ADR 0020.

**Blocked by:** 151

**Status:** ready-for-agent

**Effort:** M

- [ ] The core reports, for the next undo and redo, the number of Events changed per Calendar and per way
- [ ] Ctrl+Z on a user's action, an import or a Backup restore undoes at once; on an Incoming Sync it asks first, with the Calendar, the time and the number of Events
- [ ] At a merge, the question offers both sides, and the undo goes back through the one chosen
- [ ] Redo with one Snapshot next redoes at once; with one next in each of two Calendars it asks which Calendar, and the other can be redone afterwards
- [ ] Redo at a fork asks which way, and says that the other way is dropped
- [ ] Read-only Calendars' Snapshots are never undone by Ctrl+Z
- [ ] Toasts say "Undone: …" and "Redone: …" without a button; a step that left Calendars as they are names them in a toast that stays
- [ ] A Snapshot arriving while the question shows doesn't make the answer undo another step
- [ ] Nothing of the Dangerous Undo is left in the window or its texts
- [ ] Component tests on the fake core for each question; core tests for the counts

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. One of the successors of ticket 19 (with 147, 148, 159, 160). Blocked by 151 only: checked against tickets 152 to 154, which change what is compared and sent but not the choices an undo or a redo has.
- 2026-11-16: Chosen here, not in the spec: the wording of the two sides at a merge, and "Nothing to undo". For review.
