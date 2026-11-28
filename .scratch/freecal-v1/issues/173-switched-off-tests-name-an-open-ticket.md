# 173: Switched-off tests name an open ticket

**What to build:** A check in the pre-commit hook that keeps every switched-off test tied to the ticket that brings it back (decision 1 of `.scratch/freecal-v1/redesign-plan.md`). The redesign switches tests off on purpose while their feature is replaced; this check makes sure none is forgotten.

- A script beside the hook, listed in `.husky/pre-commit` with the other checks and run on every commit that stages code (whichever side), searches `core/`, `src-tauri/` and `src/` for `#[ignore` in Rust and `it.skip`, `describe.skip` and `test.skip` in the frontend.
- Each must carry the note `until ticket NN: <why>`: in Rust as `#[ignore = "until ticket NN: …"]`, in the frontend as the first words of the skipped test's name or of a comment on the line before it (the ticket settles one form and states it in the script's header).
- Refused, naming file and line: a bare `#[ignore]` or `.skip`; a note not of that form; an NN with no file `.scratch/freecal-v1/issues/NN-*.md`; an NN whose ticket's Status line says `done`.
- No test is switched off today; the script passes on the current tree.

Spec: none (tooling). Plan: decision 1. Layout: "By phase", P0, 173.

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] The script and its line in `.husky/pre-commit`
- [x] It refuses each case above with file and line, and passes a well-formed note naming an open ticket (tests of the script on small fixture trees)
- [x] The current tree passes

## Comments

- 2026-11-28: Started at commit `366db47`.
- 2026-11-28: Done. `.husky/switched_off_tests.py`, its tests in `.husky/test_switched_off_tests.py` (the hook runs them when `.husky/` is staged).
  - **Decision, for review: a second note, `by hand: <why>`.** The tree had one ignored test, the keyring test of `src-tauri/src/keyring.rs`, which needs a desktop session and is run by hand; it isn't switched off and no ticket brings it back. It now reads `#[ignore = "by hand: needs a desktop session's keyring"]`, and the script accepts that form (also as a frontend comment). Strongest con: an escape hatch from the ticket rule; an agent could mark a switched-off test `by hand:` to dodge it, and only `/code-review` would notice. Alternative rejected: gating the test on an environment variable, which makes it pass silently when not run.
  - **Decision, for review: the frontend note is a comment on the line right before the skip** (`// until ticket NN: <why>`), not the first words of the test's name. Switching the test back on deletes the comment and `.skip` and leaves the name as it was; it matches the Rust form, where the note sits apart from the name. Strongest con: a blank or other line slipped between comment and skip makes the note fail the check, though the check then names the line.
  - **Decision, for review: the check runs on every commit**, not only on those staging code: a commit that sets a ticket's Status to done touches only `.scratch/` and can leave a test naming a done ticket. Strongest con: about 0.1 s on doc-only commits.
  - Departures recorded in the layout plan, P0. No new terms.
- 2026-11-28: After `/code-review`, the user decided:
  - **No `by hand:` note.** Tests a person runs wait until the redesign is done. The keyring test now reads `#[ignore = "until ticket 28: needs a desktop session's keyring"]`: ticket 28 (packaging) checks the keyring in a real desktop session, so finishing 28 forces a decision on this test.
  - **A `wontfix` ticket counts as closed**, as in the tickets overview: a test naming one is refused.
  - **The frontend note stays a comment on the line before the skip**; plan decision 1 now says so. A note with no skip on the next line is refused, so one left over after a test is switched back on fails the check.
