# 113: The Window Model and Window Dialogs take the window's state out of App.svelte

**What to build:** `src/App.svelte` (about 870 lines) holds most of what the window does in one component: startup, the reads of what it shows with four hand-written "newest read wins" counters, the Signal handling, today and the navigation with the Date changed hold, the remembered view, sidebar and settings, and one state machine (`Open`) for everything shown over the grid, including the scope prompt as a promise whose resolver sits in component state. Every ticket adds to it, and its behaviour can only be tested by rendering all of it. Split it into two modules behind small interfaces. No change to what the user sees or to the core interface. From the architecture review of 2026-11-07 (candidate 1).

- **Window Model** (`src/lib/windowModel.svelte.ts`), a class, `new WindowModel(core)`, whose `start()` returns the function that stops it. It holds what the window shows and keeps it current:
  - startup: the startup failure, subscribing before the first read, the settings before the window shows, then today, the last-used view, the sidebar, the Accounts and Calendars and the Sync Statuses;
  - the reads of the Accounts and Calendars, the Sync Statuses, the Occurrences of the visible range and the Event whose details are open, each with one shared "newest read wins" helper in place of the four counters, and their read failure;
  - which Signal re-reads what (today's `onSignal`);
  - today, the navigation and the Date changed hold: `hold()` returns its release, and a held "Date changed" moves the view once the last holder releases. The grid's press and a drop's scope prompt each take one;
  - the remembered view, sidebar and settings: shown at once, written in the background, and the settings put back to what the Local Store kept when the write fails;
  - the Sync Statuses as shown, with `SyncingDelay` applied, stopped with the model.
- **Window Dialogs** (`src/lib/windowDialogs.svelte.ts`), a class, `new WindowDialogs(core, model)`: what is open over the grid, one at a time, and the steps between them. That is all of today's `Open` (Event details, the editor for a new or an existing Event, notices, import, settings, About), the scope prompt, saving, rescheduling a drag and the refused drag, deleting, the import flow with "Working…", and the toast with its Undo and the undo's retry. It depends on the Window Model, which never knows about it. Waiting imports (spec) will open through it, as it knows whether a dialog of the window is open.
- **Writes:** modules write through the core, the Window Model only reads, and the Signals bring the result back. The remembered view, sidebar and settings are the exception, as they are the window's own state and send no Signal.
- **App.svelte** keeps the layout, the wiring of the two modules into its child modules, the keyboard shortcut table, and the core calls that hold no state (sync now, quit, open a link). A shortcut is ignored while Window Dialogs has something open, and, until the Sidebar's own dialogs join it, while any `dialog[open]` is in the page, as now. The one-second clock and the overall status's words stay with what renders them.
- **Recording fake core:** the new modules' tests use a new fake core, `RecordingCore` (new term, temporary: it takes over `FakeCore`'s name in ticket 114), in `src/test/`. It reimplements nothing of the core:
  - a call nobody scripted answers with a default built from the generated samples (`src/core/samples.generated.ts`);
  - every method's answer is scripted the same way, `core.answer(method, value or error code)` or `core.pending(method)` for an answer the test gives later; the method names are typed against `CoreApi`;
  - every call is recorded with its arguments and read the same way for every method;
  - it sends no Signal of its own; tests emit them.
- **Order:** one commit for the Window Model, then one for Window Dialogs. Each keeps the `App.*.test.ts` and `Sidebar*.test.ts` files unchanged and passing, on the old `FakeCore`; ticket 114 moves them.
- Svelte 5 runes only. Read the class fields as `model.x` in components and never destructure them, which loses their reactivity.

Spec: "Testing Decisions" (Seam 2); "Signals"; US-68.

**Blocked by:** none

**Status:** wontfix

**Split into:** 116, 117, 118, 119

- [ ] `App.svelte` holds no reads, no Signal handling, no `Open` state and no failure state of the reads, dialogs or import; it is about 250 lines
- [ ] The Window Model's tests, on `RecordingCore` without rendering, cover: subscribing before the first read; an older read never overwriting a newer one, for each kind of read; each Signal's re-reads; a "Date changed" held by a press and by a drop's scope prompt, and followed once the last hold is released; a failed settings write putting back the stored settings
- [ ] Window Dialogs' tests, on `RecordingCore`, cover: the scope prompt's choices and Cancel for an edit, a changed repeat and a drop; the import flow's "Working…" and the undo's retry; a file picked while something else opened being let go
- [ ] `RecordingCore` answers, records and emits as above, and nothing in it decides what the core would do
- [ ] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged after each of the two commits

## Comments

- 2026-11-08: Split into 116 (the recording fake core), 117 (the Window Model), 118 (Window Dialogs, without the import flow) and 119 (the import flow joins Window Dialogs), so that each part's `/implement` fits one context window. Each part is one commit, with the existing `App.*.test.ts` and `Sidebar*.test.ts` passing unchanged. Work from those; this file stays for its history.
