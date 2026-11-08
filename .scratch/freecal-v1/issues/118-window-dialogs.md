# 118: Window Dialogs take what is open over the grid out of App.svelte

**What to build:** The third part of ticket 113 (read it for the why). Move what is open over the grid out of `src/App.svelte` into **Window Dialogs** (`src/lib/windowDialogs.svelte.ts`), a class, `new WindowDialogs(core, model)`: what is open, one at a time, and the steps between them. No change to what the user sees or to the core interface.

- all of today's `Open` (Event details, the editor for a new or an existing Event, notices, import, settings, About), so the one-at-a-time rule covers the import dialog too;
- the scope prompt, no longer a promise whose resolver sits in component state; while a drop's prompt is open it holds the Window Model's "Date changed";
- saving, rescheduling a drag and the refused drag, deleting, and the toast;
- the Event whose details are open, which the Window Model reads (ticket 117).

It depends on the Window Model, which never knows about it. Modules write through the core. The import flow (asking for the file, running the import with "Working…", the toast's Undo and its retry) stays in `App.svelte`, opening and closing its dialog through Window Dialogs; ticket 119 moves it.

`App.svelte` keeps the layout, the wiring into its child modules, the keyboard shortcut table, and the core calls that hold no state (sync now, quit, open a link). A shortcut is ignored while Window Dialogs has something open, and, until the Sidebar's own dialogs join it, while any `dialog[open]` is in the page, as now. Svelte 5 runes only; never destructure the class fields.

Spec: "Testing Decisions" (Seam 2); US-68.

**Blocked by:** 117

**Status:** ready-for-agent

**Effort:** M

- [ ] `App.svelte` holds no `Open` state, no scope prompt and no failure state of the dialogs; only the import flow's state is left of what Window Dialogs will hold
- [ ] Window Dialogs' tests, on `RecordingCore`, cover: the scope prompt's choices and Cancel for an edit, a changed repeat and a drop; one dialog at a time
- [ ] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged, on the old `FakeCore`
