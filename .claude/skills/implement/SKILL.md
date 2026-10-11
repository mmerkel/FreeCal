---
name: implement
description: "Implement a piece of work based on a spec or set of tickets."
disable-model-invocation: true
---

Implement the work described by the user in the spec or tickets.

Use /tdd where possible, at pre-agreed seams.

Run typechecking regularly, single test files regularly, and the full test suite once at the end.

Once done, use /code-review to review the work.

Close the ticket: tick each box only after checking it is done. A box that is partly done, or waits on another ticket, stays open, with a comment naming what is left and which ticket finishes it.

Commit your work to the current branch.
