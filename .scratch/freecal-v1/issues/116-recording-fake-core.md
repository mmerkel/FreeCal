# 116: A recording fake core for the Window Model's and Window Dialogs' tests

**What to build:** The first part of ticket 113. The tests of the Window Model (ticket 117) and Window Dialogs (tickets 118, 119) run on a new fake core, `RecordingCore` (new term, temporary: it takes over `FakeCore`'s name in ticket 114), in `src/test/`. It reimplements nothing of the core:

- a call nobody scripted answers with a default built from the generated samples (`src/core/samples.generated.ts`);
- every method's answer is scripted the same way, `core.answer(method, value or error code)` or `core.pending(method)` for an answer the test gives later; the method names are typed against `CoreApi`;
- every call is recorded with its arguments and read the same way for every method;
- it sends no Signal of its own; tests emit them.

Nothing else changes: `FakeCore` and the tests on it stay as they are.

Spec: "Testing Decisions" (Seam 2).

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `RecordingCore` implements `CoreApi` and answers, records and emits as above; its own tests show each
- [x] Nothing in it decides what the core would do
- [x] A method added to `CoreApi` fails the typecheck until `RecordingCore` has it, or is covered by its generic answering without a change

## Comments

- 2026-11-08: Done in `src/test/recordingCore.ts`, with its tests in `src/test/recordingCore.test.ts`.
  - **Answering an error code:** `core.answer(method, failure({ code }))`. A bare error code is taken as an ordinary value, because `startupFailure` answers with one. `pending(method)` returns `resolve(value?)` and `fail(error)`. Pending answers go to the calls of their method in the order they come, ahead of a scripted answer. `resolve()` without a value gives the default answer.
  - **Default answers:** the first sample of the type. A list gets every sample. Where the answer can be nothing (`startupFailure`, `pickImportFile`, the `void` methods), it is nothing. `today` is the date of the sample "Date changed". `version` is `0.0.0`, since a version has no sample type.
  - **Typecheck:** the default answers are one table typed against `CoreApi`, and the methods are made from it. Adding a method to `CoreApi` was tried, and the typecheck failed at that table.
  - **Copies:** recorded arguments and every answer are copies, so later changes to the window's reactive state don't rewrite the history.
  - **Signals:** a subscriber is added by `subscribe`'s default answer, so `emit` reaches it only once `subscribe` has answered.
  - **New terms**, for `/code-review`: **RecordingCore** (from the ticket), **Default Answer**, **Scripted Answer**, **Pending Answer** and **Failure** (the code names a test scripts with), **Call** (one recorded call with its arguments).
