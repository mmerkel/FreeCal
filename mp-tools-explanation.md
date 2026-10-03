 ▐▛███▛█   Claude Code v2.1.288
▝▜██████▀  Opus 5.5 · Claude Pro
 ▝▝   ▝▝   ~/Code/FreeCal

  Get to finished work sooner with Opus 5.5. Switch anytime with /model.

❯ /reload-skills                                                                
  ⎿  Reloaded skills: 30 skills available (no changes)

● Unknown command: /ask-matt

❯ /model                                                                        
  ⎿  Set model to Opus 5.5 (default) and saved as your default for new sessions

❯ /ask-matt I want to create a new app from scratch. How do I proceed?          

  Ran 1 shell command (ctrl+o to expand)

● Your repo (FreeCal) has no commits yet, but it already has a package.json,
  src/ and the skills installed. Here's the order I'd go in:

  0. One-time setup: /setup-matt-pocock-skills
  This tells the other skills which issue tracker to use (GitHub Issues or local
  .scratch/ files), which triage labels exist, and where docs go. Run it before
  anything else.

  1. Choose a starting point based on how clear the app is in your head
  - You can describe what it does in one sitting (e.g. "a calendar app that does
    X, Y, Z for people like me") → start with /grill-with-docs. It interviews
    you until the idea is sharp, and it writes the agreed terms to GLOSSARY.md
    and the big decisions to ADRs.
  - It's big and unclear (you don't know the architecture, the scope or what to
    build first, and it won't fit in one session) → start with /wayfinder. It
    works through the open decisions one ticket at a time until the plan is
    clear, then joins the main flow at /to-spec. It's slower and more demanding,
    so only use it if the first option really doesn't fit.

  2. Detour if a question can't be settled by talking
  For example, if you need to see a UI or try a data model in code: /handoff out
  → /prototype in a fresh session → /handoff the answer back.

  3. Plan and build
  A new app is almost certainly more than one session's work:
  - /to-spec → /to-tickets. The tickets are small end-to-end slices, and each
    one lists which tickets block it.
  - Then build, either with /implement one ticket at a time (/clear between
    tickets) or with /implement-spec to build all of them in parallel on one
    integration branch.
  - Both use /tdd (test first) and /code-review automatically. /pr writes the PR
    description.

  4. /retro after the build
  It looks back at the session and suggests changes to your setup (checks,
  coding standards, tooling) so the next build goes more smoothly.

  Context tip: keep steps 1–3 in one session, without /clear or /compact, until
  /to-tickets is done. If the session gets close to about 150k tokens before
  then, /compact at the nearest break between steps.

  Next step: run /setup-matt-pocock-skills, then /grill-with-docs with a short
  description of what FreeCal should do. If FreeCal's scope is still really
  unclear, use /wayfinder instead.

✻ Churned for 14s · done 11:28