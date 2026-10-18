# 70: Ctrl+Z, redo and timed toasts

**What to build:** The user undoes their own latest actions with Ctrl+Z and redoes with Ctrl+Shift+Z, and gets an Undo toast after each change. Undos that are dangerous ask first. Spec: US-146, US-147 and US-150; "Change Journal" and "Toasts" under Implementation Decisions.

**Blocked by:** 72

**Status:** ready-for-agent

**Effort:** M

- [ ] Ctrl+Z undoes the user's own actions of this session (since FreeCal started), newest first, skipping undos and never undoing a sync
- [ ] Ctrl+Shift+Z and Ctrl+Y redo the most recent undo, back to front, until the user's next change
- [ ] After an edit, move, delete or import, a toast says what happened with Undo ("Event deleted · Undo"); after an undo, "Undone: … · Redo". Delete no longer asks "Delete this Event?" first
- [ ] A confirmation toast disappears after 5 seconds, paused while hovered or focused; a toast reporting a problem (skipped Events, a refused undo) stays until dismissed. Changes ticket 59's `Toast.svelte`, which stays until dismissed
- [ ] A dangerous undo or redo (ticket 72's classification) asks first and names why: "This also undoes 2 later changes: …", "3 of these were changed on the server", "This removes 412 Events"; a benign one runs at once
- [ ] Shortcuts and toasts work while no dialog is open; every string goes through `t()`, Event titles as text in bidi isolates
- [ ] Component tests against the fake core

## Comments

- 2026-10-18: From grilling ticket 58's and 59's decisions (TODO.md). ADR 0009.
- 2026-10-18: Ticket 69 was split; the undo rules are now ticket 72, which this ticket is blocked by.
