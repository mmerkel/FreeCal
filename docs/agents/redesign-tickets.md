# Redesign tickets: building and finishing

Rules shared by `/implement-redesign` and `/code-review-redesign`, for every change made under a redesign ticket (173 and later). The plan's "Decisions" and "Switched-off tests" (`.scratch/freecal-v1/redesign-plan.md`) bind them; the **layout plan** is `.scratch/freecal-v1/redesign-layout.md`.

## While building

- **Switched-off tests.** Switch off exactly the tests the ticket's boxes name, as `#[ignore = "until ticket NN: <why>"]` in Rust or, in the frontend, the comment `// until ticket NN: <why>` on the line right before `.skip`, in the form the pre-commit check of ticket 173 accepts, NN the ticket that turns each back on. Turn on or rewrite the tests that name this ticket (`grep -rn "until ticket <this>"`). A test of a feature that goes for good is deleted with the feature. Every other test stays green. Code exists for the target design only: a step whose sole purpose is keeping an old test passing is a switched-off test instead.
- **Departures.** Where the code takes a different module, type, table or seam than the layout plan, the layout plan is amended: edit its section to the new shape and add `(changed in ticket NN: <why>)`. Later tickets read the plan, not your diff.
- **Names.** Use `GLOSSARY.md`'s vocabulary, its "Internals" too, and the layout plan's names. A concept with neither gets a name marked "(new term)" with a one-line definition in the ticket comment; the user approves it later. `GLOSSARY.md` gets only what a ticket's box asks for.
- **Rules from changed decisions.** When the user turns a decision into a rule ("nothing lost silently"), grep for every place the rule applies and list them in the ticket comment, each with how it now follows the rule.
- **Context budget.** Past about 100k tokens of context, commit at the next green point, write what is left into the ticket comment, and stop.

## Finishing

Done when each line holds:

1. **Switched-off tests:** `grep -rn "until ticket <this>"` finds nothing the ticket should have turned on, and each test switched off in this session names the ticket that turns it back on.
2. **Layout plan:** every module, type, table or seam the session's diff changed matches its section, or the section carries the `(changed in ticket NN: …)` note.
3. **Boxes:** each ticked box was checked as done, ticked boxes the session's diff touched included (untick one the change undid). A box partly done or waiting on another ticket stays open with a comment naming what is left and which ticket finishes it. Boxes for `GLOSSARY.md` or `CODING_STANDARDS.md` entries are done in this ticket. `**Status:** done` once every box is ticked. After a change to any ticket's Status or Blocked by, or a new ticket, run the `tickets-pdf` skill.
4. **Conformance Breaches:** the ticket comment has a line `Conformance Breaches:` naming each one the diff adds or ends (`CODING_STANDARDS.md`, "Conformance"), or `none`, and `.scratch/freecal-v1/conformance.md` follows: a breach added gets a line, one ended is struck with the ticket's number. Look for them where the diff stores a field, reads one back, or writes iCalendar or another Provider's format.
5. **Commit** to the current branch once the pre-commit hook passes; a failing hook means the work isn't done. The title says which step it is:
   - `Ticket NN written: <title>` for a ticket file committed on its own;
   - `Ticket NN: <outcome>` for the implementation;
   - `Ticket NN review: <outcome>` for the review's fixes, follow-up tickets included.

   The body says what changed in `GLOSSARY.md` terms, and repeats the `Conformance Breaches:` line.
