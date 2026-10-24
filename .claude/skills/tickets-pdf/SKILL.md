---
name: tickets-pdf
description: Render the local tickets as a PDF overview with a dependency graph, effort, the critical path and what's ready to start. Use when asked for a ticket overview, a dependency graph of the tickets, remaining effort, or tickets.pdf, and after any change to a local ticket's Status, Blocked by or Effort, or to the set of tickets.
---

`tickets_pdf.py` reads `.scratch/<feature>/issues/*.md` (format in `docs/agents/issue-tracker.md`) and writes the PDF: a summary page with the tickets ready to start, an A3 dependency graph and a table of every ticket. It needs Graphviz (`dot`) and Chrome or Chromium. Run from the repo root.

1. Build it:

   ```bash
   python3 .claude/skills/tickets-pdf/tickets_pdf.py [.scratch/<feature>] [out.pdf]   # defaults: .scratch/freecal-v1, tickets.pdf
   ```

   It refuses to build while a ticket's header lines are wrong (a Status outside the triage labels and `done`, a `Blocked by` naming a missing ticket, a cycle; the pre-commit hook runs the same check with `--check`) and lists them; fix those. For a ticket that lacks an `**Effort:**` line, size it from its scope, using the sizes defined in `docs/agents/issue-tracker.md`, and write the line after its Status line. Then build again. Done when the script prints its `Wrote …` summary. Don't open or render the PDF afterwards; the script's output has everything the report needs.

2. Report the PDF's path, the summary line, the ready tickets it lists after that line (the three that unblock the most), and every Effort line you added in step 1, so the user can correct those sizes.
