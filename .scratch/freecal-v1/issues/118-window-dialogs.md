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

**Status:** done

**Effort:** M

- [x] `App.svelte` holds no `Open` state, no scope prompt and no failure state of the dialogs; only the import flow's state is left of what Window Dialogs will hold
- [x] Window Dialogs' tests, on `RecordingCore`, cover: the scope prompt's choices and Cancel for an edit, a changed repeat and a drop; one dialog at a time
- [x] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged, on the old `FakeCore`

## Comments

- 2026-11-08: Done in `src/lib/windowDialogs.svelte.ts`, with its tests in `src/lib/windowDialogs.test.ts` (on `RecordingCore` and a started Window Model, without rendering). `App.svelte` went from about 670 to about 380 lines.
  - **Scope Prompt:** the promise's resolver now sits in Window Dialogs; the Scope Dialog answers with `dialogs.chooseScope(choice)`. While a drop's prompt is open it holds the Window Model's "Date changed" (`model.hold()`), from before the counts load.
  - **Details actions read what is open:** `startEditing()`, `save(draft)`, `cancelEditing()`, `remove()` and `removeOccurrence()` take the Event and Occurrence from what is open rather than from App's arguments.
  - **Deviation, open a link:** the ticket keeps it in App, but its failure is the open details' or About's failure, which the first box forbids in App. It moved into Window Dialogs (`dialogs.openLink`).
  - **Behaviour changes:** a Scope Prompt still waiting for its counts now counts as open. So shortcuts are ignored then too, and a file picked while a drop's prompt is open is let go (`openImport` returns false) instead of opening the import dialog beside it.
  - **Import flow:** stays in App, as asked, through `defaultCalendar()`, `showNotice`, `openImport`, `close`, `showToast` and `dismissToast`; ticket 119 moves it in.
  - **Left for later (review judgement calls):** `(id, recurrenceId)` travels together through many methods (pre-existing); "which Calendars are writable" could live on the Window Model's Directory; the Occurrence lookup by Event and recurrence ID repeats in `openedOccurrence` and `refuseDrag`.
  - **New terms**, for approval: **Scope Prompt** (`ScopePrompt`), the open question which Occurrences of a Recurring Event a change changes, with the Exception counts once read; **Toast** (`Toast`), the message at the bottom of the window with its Undo if it has one; **Open** (`Open`), what is open over the grid apart from the Scope Prompt; **Notice**, an OK-only dialog with a title and a body; **Default Calendar** (`defaultCalendar()`), the first shown writable Calendar, where new things go; **Editor Session** (`editorSession`), counts editors opened, so each starts from its own draft; **Drop Held** (`dropHeld`), the grid holding back a new today while a drop's Scope Prompt is open.
