# 70: Ctrl+Z, redo and timed toasts

**What to build:** The user undoes their own latest actions with Ctrl+Z and redoes with Ctrl+Shift+Z, and gets an Undo toast after each change. Undos that are dangerous ask first. Spec: US-146, US-147 and US-150; "Change Journal" and "Toasts" under Implementation Decisions.

**Blocked by:** 72

**Status:** done

**Effort:** M

- [x] Ctrl+Z undoes the user's own actions of this session (since FreeCal started), newest first, skipping undos and never undoing a sync
- [x] Ctrl+Shift+Z and Ctrl+Y redo the most recent undo, back to front, until the user's next change
- [x] After an edit, move, delete or import, a toast says what happened with Undo ("Event deleted · Undo"); after an undo, "Undone: … · Redo". Delete no longer asks "Delete this Event?" first
- [x] A confirmation toast disappears after 5 seconds, paused while hovered or focused; a toast reporting a problem (skipped Events, a refused undo) stays until dismissed. Changes ticket 59's `Toast.svelte`, which stays until dismissed
- [x] A dangerous undo or redo (ticket 72's classification) asks first and names why: "This also undoes 2 later changes: …", "3 of these were changed on the server", "This removes 412 Events"; a benign one runs at once
- [x] Shortcuts and toasts work while no dialog is open; every string goes through `t()`, Event titles as text in bidi isolates
- [x] Component tests against the fake core

## Comments

- 2026-10-18: From grilling ticket 58's and 59's decisions (TODO.md). ADR 0009.
- 2026-10-18: Ticket 69 was split; the undo rules are now ticket 72, which this ticket is blocked by.
- 2026-11-14: Implemented, frontend only (the core's `undo(target, approved)` and the Journal Transaction each write returns were enough). Decisions, for review, each with its strongest con:
  - **Decision, for review:** Session Undo lives in the frontend (`SessionUndo`, `src/lib/sessionUndo.ts`), as two lists of Journal Transaction IDs: the user's changes and the undos of them. A redo is the undo of an undo's Journal Transaction, so Ctrl+Z after a redo undoes it again. Con: a reload of the webview forgets the session; the core doesn't enforce the stack.
  - **Decision, for review:** a new Event counts as a change too: "Event created · Undo", and Ctrl+Z removes it. An import is on Session Undo too. Con: Ctrl+Z can undo a whole import, but its count of Events then makes it a Dangerous Undo that asks.
  - **Decision, for review:** a change undone already elsewhere (core error `alreadyUndone`) is skipped and Ctrl+Z tries the one before; a toast's Undo of that very change says it was undone already. Con: Ctrl+Z can silently pass over a change the user expected.
  - **Decision, for review:** an undo that changed nothing (every Event skipped) leaves Session Undo, so the next Ctrl+Z goes on to the change before. Con: once the Calendar is writable again, only the History (ticket 19) can undo it.
  - **Decision, for review:** a failed undo (other than "undone already") stays on Session Undo, and its toast stays, with Undo (or Redo) to try again.
  - **Decision, for review:** Ctrl+Z with nothing to undo says "Nothing to undo." in a toast (likewise for redo). Con: one more toast; silence would be the other common choice.
  - **Decision, for review:** the toasts name the change as a phrase: "Undone: deleting “Lunch”.", "Redone: the import of “a.ics”.", "Nothing of moving “Lunch” was undone." A drag and a resize both say "Event moved". Con: the phrases (`undo.action.*`) are grammar a translation has to fit into "Undone: {action}."; a resize is called a move.
  - **Decision, for review:** the question before a Dangerous Undo says "This changes 412 events." rather than the spec's "This removes 412 Events", as the core says how many it changes, not how many it removes. The later changes are listed by the titles of their Events, without their time. Con: less specific than the spec's example for an import.
  - **Decision, for review:** the import toast's Undo now asks before every Dangerous Undo, replacing ticket 72's interim rule (run one dangerous only by count, refuse the rest). Con: undoing a big import that the user just made asks once more.
  - **Decision, for review:** a confirmation toast's 5 seconds start again from the beginning once the pointer leaves and the focus goes, rather than going on from where they paused. Con: a toast pointed at often lasts longer than 5 seconds.
  - **Decision, for review:** Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y don't act while typing in a text field, which has its own undo; Ctrl shortcuts now match their letter regardless of Caps Lock, and Shift must match. Con: Ctrl+Z in the sidebar's text fields never undoes an Event change.
  - **Decision, for review:** Delete of an Event that repeats still asks whether to delete the Occurrence alone or all (now titled "Delete a repeating event"); only the plain "Delete this event?" is gone. The spec's "Event details" paragraph says so now.
  - **Decision, for review:** an undo of an import still shows "Working…" (and a redo "Redoing the import …"); other undos run without it.
  - New terms: **Session Undo** (new term): the user's own changes of this session, which Ctrl+Z undoes newest first, and the undos of them, which Ctrl+Shift+Z redoes until the user's next change (`SessionUndo`). `UserAction` names a change as the toasts do, after the Transaction Origin "user action". The hostile fixture's `deleteQuestion` became `undoneDelete`.
  - Tests: `src/lib/windowDialogs.test.ts` (`describe('Session Undo')`, the import flow), `src/lib/Toast.test.ts`, `src/lib/UndoDialog.test.ts`, `src/lib/shortcuts.test.ts`, `src/App.events.test.ts` (delete, Ctrl+Z, the hostile title), `src/App.import.test.ts`.

**2026-11-14, after /code-review:** fixed the review's findings. Refused drags now stay as problem toasts. A second Ctrl+Z while an undo runs keeps the toast. A change made during an undo leaves that undo's toast without Redo. Closing what is open cancels an unanswered Dangerous Undo question. A failed undo's retry targets the same change. A toast that appears under a resting pointer waits. The hostile fixture covers the question's later-change titles and the toast after undoing an import. Boxes 3 and 5 are ticked with the decisions above, for review: a repeating Event still asks which Occurrences (not "Delete this Event?"), and the question says "This changes 412 events".
- 2026-11-14: Superseded in part by ticket 19: undo now reaches beyond a session. Session Undo becomes the Undo History, which the core works out from the Change Journal and a restart doesn't reset; "of this session" in the first box no longer holds. An undo that changed nothing is marked in the Local Store, and the kind of each user action is stored with its Journal Transaction, so the toasts keep their labels after a restart.
