---
name: implement
description: "Implement a piece of work based on a spec or set of tickets."
disable-model-invocation: true
---

Implement the work described by the user in the spec or tickets.

Use /tdd where possible, at pre-agreed seams.

Run typechecking regularly and single test files regularly. The commit's pre-commit hook runs the full suite; a failing hook means the work isn't done.

Once done, use /code-review to review the work.

Close the ticket: tick each box only after checking it is done. A box that is partly done, or waits on another ticket, stays open, with a comment naming what is left and which ticket finishes it. On the local tracker, set `**Status:** done` once every box is ticked, then run the `tickets-pdf` skill.

Commit your work to the current branch.
