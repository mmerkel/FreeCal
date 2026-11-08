# 119: The import flow joins Window Dialogs

**What to build:** The last part of ticket 113 (read it for the why). Move the import flow from `src/App.svelte` into Window Dialogs (ticket 118): asking for the file, letting it go when something else opened meanwhile, running the import with "Working…", and the toast with its Undo and the undo's retry. No change to what the user sees or to the core interface. Waiting imports (spec) will open through Window Dialogs, as it knows whether a dialog of the window is open.

Spec: "Testing Decisions" (Seam 2).

**Blocked by:** 118

**Status:** ready-for-agent

**Effort:** S

- [ ] `App.svelte` holds no reads, no Signal handling, no `Open` state and no failure state of the reads, dialogs or import; it is about 250 lines
- [ ] Window Dialogs' tests, on `RecordingCore`, cover: the import flow's "Working…" and the undo's retry; a file picked while something else opened being let go
- [ ] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged, on the old `FakeCore`
