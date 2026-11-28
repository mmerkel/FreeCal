---
name: implement-redesign
description: "Implement one redesign ticket (173 and later) in one session, from the layout plan."
disable-model-invocation: true
---

Implement the one ticket the user names, from `.scratch/freecal-v1/issues/`. Three files carry the redesign: the ticket; `.scratch/freecal-v1/redesign-layout.md`, the **layout plan** (modules, types, tables, seams, by ticket number); and `.scratch/freecal-v1/redesign-plan.md`, whose "Decisions" and "Switched-off tests" bind every ticket.

## Steps

1. **Orient** (about 20k tokens). Read the ticket with its comments, the layout plan's sections it cites, and every layout-plan line naming its number. When a comment leaves a grilling open, hold it with /grilling; with the user away, decide each question yourself, name its strongest con, and list it first in the ticket comment and the report, marked "decision, for review". Done when you can name the files you will change and the seam each box is tested at.
2. **Locate**, from the layout plan's `file:line` pointers, with `grep -n` or LSP, and read only the lines you change. The layout plan replaces mapping the code with a subagent. Where a pointer has gone stale, find the code and correct the pointer in the layout plan.
3. **Record the start.** Write the commit before your first change into a ticket comment. If the ticket opens a phase in `redesign-plan.md`, write that commit under the phase table too.
4. **Build**, one layer as the ticket says, test first with /tdd at the seams the layout plan names. Run typechecking and single test files as you go.
   - **Switched-off tests.** Switch off exactly the tests the ticket's boxes name, as `#[ignore = "until ticket NN: <why>"]` in Rust or `it.skip` with the same note in the frontend, in the form the pre-commit check of ticket 173 accepts, NN the ticket that turns each back on. Turn on or rewrite the tests that name this ticket (`grep -rn "until ticket <this>"`). A test of a feature that goes for good is deleted with the feature. Every other test stays green. Code exists for the target design only: a step whose sole purpose is keeping an old test passing is a switched-off test instead.
   - **Departures.** Where the code takes a different module, type, table or seam than the layout plan, the layout plan is amended: edit its section to the new shape and add `(changed in ticket NN: <why>)`. Later tickets read the plan, not your diff.
   - **Names.** Use `GLOSSARY.md`'s vocabulary, its "Internals" too, and the layout plan's names. A concept with neither gets a name marked "(new term)" with a one-line definition in the ticket comment; the user approves it after /code-review.
5. **Check.** Commit when the pre-commit hook passes; a failing hook means the ticket isn't done. Past about 100k tokens of context, commit at the next green point, write what is left into the ticket comment, and stop.
6. **Close.** Tick each box only after checking it is done; a box partly done or waiting on another ticket stays open with a comment naming what is left and which ticket finishes it. Boxes for `GLOSSARY.md` or `CODING_STANDARDS.md` entries are done in this ticket. Set `**Status:** done` once every box is ticked, run the `tickets-pdf` skill, and commit to the current branch.
7. **Report**: what was built, the switched-off tests turned off and on, the departures from the layout plan, the new terms, the decisions for review, and the next command: `/code-review` in a fresh session from the recorded commit. When this ticket closes a phase, add the phase's `/simplify` (and, after P3 and P6, `/improve-codebase-architecture`) from the phase's start commit, given the layout plan.
