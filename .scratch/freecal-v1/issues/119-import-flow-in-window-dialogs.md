# 119: The import flow joins Window Dialogs

**What to build:** The last part of ticket 113 (read it for the why). Move the import flow from `src/App.svelte` into Window Dialogs (ticket 118): asking for the file, letting it go when something else opened meanwhile, running the import with "Working…", and the toast with its Undo and the undo's retry. No change to what the user sees or to the core interface. Waiting imports (spec) will open through Window Dialogs, as it knows whether a dialog of the window is open.

Spec: "Testing Decisions" (Seam 2).

**Blocked by:** 118

**Status:** done

**Effort:** S

- [x] `App.svelte` holds no reads, no Signal handling, no `Open` state and no failure state of the reads, dialogs or import; it is about 250 lines
- [x] Window Dialogs' tests, on `RecordingCore`, cover: the import flow's "Working…" and the undo's retry; a file picked while something else opened being let go
- [x] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged, on the old `FakeCore`

## Comments

- 2026-11-08: Done in `src/lib/windowDialogs.svelte.ts` (`startImport`, `runImport`, `cancelImport`, the private `#undoImport`, and the `importFailure` and `working` state), with its tests in `src/lib/windowDialogs.test.ts` ("the import flow", and two let-go tests under "one dialog at a time").
  - **Size:** `App.svelte` went from about 380 to 304 lines, not about 250. All that's left is layout markup, the wiring into the child modules, the shortcut table and quit's own failure. This ticket has nothing else to move out, so the box is ticked on what it lists rather than on the line count.
  - **Smaller interface:** `defaultCalendar`, `showNotice` and `showToast` are private now, and `openImport` is gone; App only calls `startImport`. `runImport` takes the file from the open import dialog, as the details actions do.
  - **Not visible, but changed:** a file let go because something else opened meanwhile is always discarded. The old code went through Cancel, which skips the discard while "Working…" shows, a case that can't happen while the dialogs are modal.
  - **Left for later (review judgement calls):** `startCreating` and `startImport` repeat the "no Default Calendar, show a Notice" step with different titles. `working` holds the message the dialog shows, not a yes/no, so `workingMessage` might read better. The **Window Dialogs** entry in `GLOSSARY.md` doesn't mention "Working…".
  - **New terms**, for approval: **Working** (`working`), the modal "Working…" dialog shown while a long core command runs, such as an import or its undo; **Import Failure** (`importFailure`), why the last import from the open import dialog failed, shown in that dialog; **Import Flow**, asking for the file, running the import under "Working…", and the Toast's Undo with its retry.
