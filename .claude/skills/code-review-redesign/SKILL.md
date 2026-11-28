---
name: code-review-redesign
description: "Review one implemented redesign ticket (173 and later) with /code-review, fix the findings and commit."
disable-model-invocation: true
---

Review the one ticket the user names, from `.scratch/freecal-v1/issues/`, after `/implement-redesign` built it, and act on every finding without asking. The redesign's files are those `/implement-redesign` names: the ticket, the **layout plan** `.scratch/freecal-v1/redesign-layout.md`, `.scratch/freecal-v1/redesign-plan.md`, and `docs/agents/redesign-tickets.md`, whose rules bind every fix.

## Steps

1. **Pin the start.** The fixed point is the commit in the ticket's first `Started from commit <sha>` comment. Without one, ask the user.
2. **Review.** Run the `code-review` skill from that commit, with the ticket as the spec. Give the Spec sub-agent the layout plan's sections the ticket cites and the ticket's comments, so a recorded departure or decision reads as intended; give the Standards sub-agent `CODING_STANDARDS.md` and `docs/agents/redesign-tickets.md`. Leave `GLOSSARY.md` as it is: new terms stay marked "(new term)" for the user.
3. **Settle every finding** as exactly one of:
   - **fixed**: hard violations, missing or partial boxes, and code that looks wrong are always fixed;
   - **declined**: a judgement call or scope-creep finding you weighed and rejected, with the reason;
   - **for review**: a fix would change an ADR, the spec, or the ticket's outcome; leave the code, record it marked "decision, for review" with its strongest con;
   - **follow-up**: work outside the ticket; write it as a new ticket (below).

   Judgement calls and scope creep are yours to decide: fix them or decline them, naming the strongest con of the choice. Done when every finding of both axes has its outcome.
4. **Fix**, following "While building" in `docs/agents/redesign-tickets.md`, test first with /tdd where a fix changes behaviour.
5. **Follow-up tickets.** Number from the highest in `.scratch/freecal-v1/issues/` plus one, in the format of the redesign's tickets (title, **What to build**, a `Spec: …. ADRs …. Layout: …` line, Blocked by, `**Status:** needs-triage`, Effort, boxes) and a first comment `From the review of NN.` with the strongest con. Add a line to the plan's `## Tickets added during implementation` (create it after "Batches"): number, title, from which review, Blocked by. The user promotes it to `ready-for-agent`.
6. **Comment** on the ticket: `<date>: Review of <start>..<HEAD before fixes>.`, then one bullet per fix, decline, decision for review and follow-up, and the new terms.
7. **Finish** per "Finishing" in `docs/agents/redesign-tickets.md`, with the title `Ticket NN review: <outcome>`.
8. **Report**, for the user to read before the next ticket:
   - every finding with its outcome;
   - the decisions for review, this review's and the ticket's open ones from `/implement-redesign`;
   - the new terms, to approve or rename;
   - the follow-up tickets, to promote;
   - a finding whose kind an earlier redesign ticket's review comment already records: name both, and suggest `/retro` on them;
   - the next command. When this ticket closes a phase in `redesign-plan.md`: the phase's `/simplify` (and, after P3 and P6, `/improve-codebase-architecture`) from the phase's start commit, given the layout plan. Otherwise `/implement-redesign` with the next ticket that is `ready-for-agent` and unblocked.
