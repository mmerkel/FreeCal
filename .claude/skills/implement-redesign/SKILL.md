---
name: implement-redesign
description: "Implement one redesign ticket (173 and later) in one session, from the layout plan."
disable-model-invocation: true
---

Implement the one ticket the user names, from `.scratch/freecal-v1/issues/`. Four files carry the redesign: the ticket; `.scratch/freecal-v1/redesign-layout.md`, the **layout plan** (modules, types, tables, seams, by ticket number); `.scratch/freecal-v1/redesign-plan.md`, whose "Decisions" and "Switched-off tests" bind every ticket; and `docs/agents/redesign-tickets.md`, the rules for building and finishing.

## Steps

1. **Orient** (about 20k tokens). Read the ticket with its comments, the layout plan's sections it cites, every layout-plan line naming its number, and `docs/agents/redesign-tickets.md`. When a comment leaves a grilling open, hold it with /grilling; with the user away, decide each question yourself, name its strongest con, and list it first in the ticket comment and the report, marked "decision, for review". Done when you can name the files you will change and the seam each box is tested at.
2. **Locate**, from the layout plan's `file:line` pointers, with `grep -n` or LSP, and read only the lines you change. The layout plan replaces mapping the code with a subagent. Where a pointer has gone stale, find the code and correct the pointer in the layout plan.
3. **Record the start.** Write `Started from commit <sha>.`, the commit before your first change, into a ticket comment. If the ticket opens a phase in `redesign-plan.md`, write that commit under the phase table too.
4. **Build**, one layer as the ticket says, test first with /tdd at the seams the layout plan names, following "While building" in `docs/agents/redesign-tickets.md`. Run typechecking and single test files as you go.
5. **Finish** per "Finishing" in `docs/agents/redesign-tickets.md`, with the title `Ticket NN: <outcome>`.
6. **Report**: what was built, the switched-off tests turned off and on, the departures from the layout plan, the new terms, the decisions for review, and the next command: `/code-review-redesign NN` in a fresh session.
