# 114: The frontend tests run on the recording fake core, and the simulating one goes

**What to build:** The fake core of the frontend tests (`FakeCore`, `src/test/fakeCore.ts`, about 940 lines) has grown into a partial second core: about 420 lines reimplement the Change Journal and its undo, the Occurrences of a Recurring Event, edits in a recurrence scope (without "this and following"), deleting by UID, import update / skip / copy, and Server Accounts. It can drift from the Rust core with no test noticing. The spec's Seam 2 asks only that user actions produce the right core calls and that the window follows the Signals. Move every frontend test to the recording fake core of ticket 113 and delete `FakeCore`. From the architecture review of 2026-11-07 (candidate 2).

- **One test file at a time**, each move a commit with every test passing: `App.navigation`, `App.settings`, `App.sidebar`, `App.sync`, `App.import`, `App.events`, `Sidebar`, `Sidebar.accounts`.
- **What stays at the App level:** each user flow keeps one test there, checking that the user's action makes the right core call, that a Signal changes what is shown, and that one dialog opens over another. Their variations live only in the Window Model's and Window Dialogs' tests (ticket 113): which read wins, the Date changed hold, the startup order, the scope prompt's choices and Cancel, the undo's retry.
- **Tests emit the Signals.** A test that checks the window after a change scripts the new answer and emits the Signal the core would send, such as "Occurrences changed".
- **What leaves the frontend tests:** an outcome that only the fake simulated, such as "a Recurring Event is on every date", "Undo undoes the import in one step" or "hiding a Calendar hides its Events". The window's part of each stays: the call made and the re-read on the Signal. Each outcome is dropped only once a test at the core interface (Seam 1) is found that covers it; one without is added there first, in Rust.
- **The special lists and knobs go:** `scopedChanges`, `sidebarSaves`, `settingsUpdates`, `openedLinks`, `fileDialogs`, `holdFirstSyncs`, `importFile`, `caldavServer`, `startupError` and the like become the recorded calls and scripted answers of `RecordingCore`.
- **The hostile test Event** (spec "Testing Decisions") still goes through the grid, its details, the editor, the delete question and the refused drag, as a scripted answer.
- **At the end** `FakeCore` is deleted and `RecordingCore` takes over its name and file (`FakeCore`, `src/test/fakeCore.ts`), the spec's "fake core interface".

Spec: "Testing Decisions" (Seam 2, the hostile test Event).

**Blocked by:** 113

**Status:** ready-for-agent

**Effort:** M

- [ ] No frontend test uses the old `FakeCore`; it is deleted, and the recording fake is `FakeCore` in `src/test/fakeCore.ts`
- [ ] Nothing in the fake decides what the core would do: no Change Journal, no Occurrences of a Recurring Event, no scope edits, no import rules
- [ ] Every outcome dropped from the frontend tests is covered by a named Seam 1 test, listed in this ticket's comments
- [ ] The hostile test Event still runs through every path it ran through before
- [ ] `npm test` and the pre-commit checks pass
