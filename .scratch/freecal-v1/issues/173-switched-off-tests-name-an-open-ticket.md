# 173: Switched-off tests name an open ticket

**What to build:** A check in the pre-commit hook that keeps every switched-off test tied to the ticket that brings it back (decision 1 of `.scratch/freecal-v1/redesign-plan.md`). The redesign switches tests off on purpose while their feature is replaced; this check makes sure none is forgotten.

- A script beside the hook, listed in `.husky/pre-commit` with the other checks and run on every commit that stages code (whichever side), searches `core/`, `src-tauri/` and `src/` for `#[ignore` in Rust and `it.skip`, `describe.skip` and `test.skip` in the frontend.
- Each must carry the note `until ticket NN: <why>`: in Rust as `#[ignore = "until ticket NN: …"]`, in the frontend as the first words of the skipped test's name or of a comment on the line before it (the ticket settles one form and states it in the script's header).
- Refused, naming file and line: a bare `#[ignore]` or `.skip`; a note not of that form; an NN with no file `.scratch/freecal-v1/issues/NN-*.md`; an NN whose ticket's Status line says `done`.
- No test is switched off today; the script passes on the current tree.

Spec: none (tooling). Plan: decision 1. Layout: "By phase", P0, 173.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

- [ ] The script and its line in `.husky/pre-commit`
- [ ] It refuses each case above with file and line, and passes a well-formed note naming an open ticket (tests of the script on small fixture trees)
- [ ] The current tree passes
