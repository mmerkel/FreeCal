# Redesign plan: tickets from 173 on

The plan for rewriting the tickets that implement ADRs 0017 to 0020 (Calendar History, Unimplemented Fields, sync from the server's marks, Undo as a Snapshot, Conflicts) and for implementing them. Each batch below runs in a fresh context window of at most 130k tokens; start one with "run batch Bn of `.scratch/freecal-v1/redesign-plan.md`". A batch ends by committing, ticking its line under "Batches" and writing anything the next batch needs into this file, never only into the conversation.

Start of the redesign: tag `design/snapshot-line` (commit `065a8e8`). Diffs and reviews of the redesign as a whole start there.

## Decisions (2026-11-28, agreed with the user)

1. **Switched-off tests, not glue code.** A ticket doesn't write code that a later ticket deletes only to keep an old test passing. A test whose feature is being replaced is switched off on purpose: `#[ignore = "until ticket NN: <why>"]` in Rust, `it.skip` with the same note in the frontend. Ticket NN switches it back on or rewrites it, and has a box for that. A test of something that goes for good (the Change Journal, Propagate, Unsent Changes) is deleted in the ticket that removes the feature. Every other test stays green, so "the checks pass" still means a ticket is done.
   - A test stays switched off for at most 3 to 4 tickets. During that window, the new code that replaces the feature is covered by tests at its own seam.
   - A check in the pre-commit hook makes sure every switched-off test names a ticket that isn't done (first ticket, 173).
   - The app may lack a feature between tickets (e.g. no undo between the new Event Versions and the new Undo): no real Local Store exists.
2. **Ticket size by tokens.** A ticket fits in one context window: about 20k for orientation, about 80k for the work, room left for the hook's output. In practice one layer (core, shell or frontend) and a diff of roughly 300 to 600 lines. The Effort line stays (S or M); the token budget is the binding limit.
3. **Short tickets.** A new ticket states the outcome, the boxes and the switched-off tests it turns on or off, and cites ADRs, spec sections and the layout plan instead of repeating them. No grilling history. Decisions that aren't in the ADRs or the spec go into the ticket text, not into comments.
4. **Numbering from 173.** The old tickets 133 and later are deleted (git keeps them), except 139, 140, 141 and 170 (ready-for-human, unrelated), which stay as they are. In scope as well: the Google tickets 163 and 164, and older tickets that build on the redesign (15, 21, 22 and whatever else B1 finds); these are rewritten as new tickets too, or only have their references fixed if their content still holds.
5. **A new skill, `implement-redesign`**, a variant of `.claude/skills/implement/SKILL.md`: reads the layout plan instead of mapping the code with an Explore subagent; knows the rules for switched-off tests; one ticket is one session in one layer, so no stopping between layers; records each departure from the layout plan in that plan, with the ticket's number. `/code-review` still follows every ticket, in a fresh session (the main safety net while tests are switched off).
6. **Sequential, on the current branch.** No `implement-spec` and no parallel worktrees: the tickets touch the same files.
7. **`/simplify`** at the end of every phase, on the phase's diff, given the layout plan so that it keeps structure that was chosen on purpose. **`/improve-codebase-architecture`** twice: after the sync phase (before the Conflict and window tickets build on it), and at the end. Not before (the layout plan does that job) and not in between (glue and switched-off tests would mislead it).
8. **A layout plan for the backend** (`.scratch/freecal-v1/redesign-layout.md`): modules and files, main types and tables, the seams where `/tdd` tests sit, and which tests through the core interface (`core/src/lib.rs`) survive the whole redesign. Central interfaces (the store of Event Versions, the comparison of the two sides, the Incoming Sync) designed with `/codebase-design`'s "design it twice". A target that tickets may amend, not a contract. New names are proposed as new terms for the user to approve (CLAUDE.md).

## Phases (draft; B1 settles the tickets)

| Phase | Content (old numbers) | After the phase |
|---|---|---|
| P0 | Check for switched-off tests in the pre-commit hook | — |
| P1 | Identity by Combined Event Key (142); Unimplemented Fields tree, CalDAV reader and writer (143, 144); the comparison of the two sides as a pure function (167) | `/simplify` |
| P2 | Calendar History: Event Versions, Snapshots, Undo, dropping (145 to 148); the Change Journal goes | `/simplify` |
| P3 | Sync from the server's marks (149 to 155, 157, 169, 171); Propagate and Unsent Changes go | `/simplify`, `/improve-codebase-architecture` |
| P4 | Conflicts in the core (133 to 135, 168, 166, 21) | `/simplify` |
| P5 | Window (158 to 162, 136, 137, 156, 172, 15, 22) | `/simplify` |
| P6 | Google (163, 164, and older Google tickets that build on these) | `/simplify`, `/improve-codebase-architecture` |

Record the commit at the start of each phase here when implementation begins, for the phase's `/simplify` and review.

## Batches

- [ ] **B1, the ticket plan.** Read `GLOSSARY.md`, ADRs 0017 to 0020, the spec's sections they touch, `CODING_STANDARDS.md`, and the old tickets 133 and later (skim; their "What to build" and boxes, not their comments). Find older tickets that name any of them in `Blocked by` or text (`grep -l`). Sort the existing tests (Rust and frontend) into survive / rewrite / delete-with-their-feature, at the level of files or test modules, via an Explore subagent asked for a list, not contents. Write into this file, under "Ticket plan": every new ticket (number from 173, title, a few sentences of scope, layer, phase, Blocked by, the old tickets it replaces), the table of switched-off tests (test, off in ticket, on in ticket; check the 3 to 4 ticket limit), and the test sorting. Each old ticket in scope maps to new tickets or to "dropped, because …". Don't write ticket files yet.
- [ ] **B2, the layout plan and the skill.** Map the backend through an Explore subagent (seams, modules, tables, `file:line` pointers, no contents). Write `.scratch/freecal-v1/redesign-layout.md` as in decision 8, ordered by phase, naming which new ticket creates or changes each module. Propose new terms to the user. Adjust the ticket plan where the layout shows a ticket too large or in the wrong order. Write `.claude/skills/implement-redesign/SKILL.md` per decision 5 (use `/writing-for-agents`).
- [ ] **B3 and later, the ticket files.** Write about 8 to 10 tickets per batch from the ticket plan and the layout plan, in the format of `docs/agents/issue-tracker.md` and decision 3, each citing the sections of `redesign-layout.md` it builds. Note here which numbers each batch wrote.
- [ ] **Last batch, the switch.** Delete the old ticket files 133 and later (except 139, 140, 141, 170) and the old tickets that new ones replace. Fix references to old ticket numbers in the remaining tickets, the spec, the ADRs, `GLOSSARY.md` and `TODO.md` (`grep -rn -E 'ticket[s]? (1[3-7][0-9])'` and the `Blocked by` lines). Run `tickets-pdf`. Commit.

## Ticket plan

(B1 writes here.)
