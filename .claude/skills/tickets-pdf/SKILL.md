---
name: tickets-pdf
description: Render the local tickets as a PDF overview with a dependency graph, effort, the critical path and what's ready to start. Use when asked for a ticket overview, a dependency graph of the tickets, remaining effort, or tickets.pdf.
---

`tickets_pdf.py` reads `.scratch/<feature>/issues/*.md` (format in `docs/agents/issue-tracker.md`) and writes the PDF: a summary page with the tickets ready to start, an A3 dependency graph and a table of every ticket. It needs Graphviz (`dot`) and Chrome or Chromium. Run from the repo root.

1. Build it:

   ```bash
   python3 .claude/skills/tickets-pdf/tickets_pdf.py [.scratch/<feature>] [out.pdf]   # defaults: .scratch/freecal-v1, tickets.pdf
   ```

   It refuses to build while a ticket lacks an `**Effort:**` line and lists those files. Size each one from its scope, using the sizes defined in `docs/agents/issue-tracker.md`, and write the line after its Status line. Then build again. Done when the script prints its `Wrote …` summary.

2. Check the graph page: `pdftoppm -r 90 -f 2 -l 2 -png <out.pdf> <scratchpad>/graph`, then view the image. Labels must be readable and the critical path marked in red.

3. Report the PDF's path, the summary line, the two or three open tickets that unblock the most, and every Effort line you added in step 1, so the user can correct those sizes.
