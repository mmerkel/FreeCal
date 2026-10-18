# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- User stories in a spec carry permanent IDs, written as `- **US-N** As a …`: a new story takes the next unused number wherever it goes, and a number is never reused or renumbered. Tickets cite them by ID, with the spec's other sections by heading: `Spec: US-15; "Untrusted content".`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`, never a single combined tickets file
- Triage state is recorded as a `**Status:** <label>` line near the top of each issue file (see `triage-labels.md` for the role strings), and blockers as `**Blocked by:** NN, NN` (or `none`)
- Effort is recorded as an `**Effort:** S|M|L|XL` line after the Status line: S ≈ ½ day, M ≈ 1–2 days, L ≈ 3–5 days, XL ≈ 1–2 weeks of agent-driven work. Size it from the ticket's scope when writing the ticket; re-size it when a ticket is split or grows. A split ticket carries no Effort; its parts do
- A finished ticket gets `**Status:** done`
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- After changing any ticket's Status, Blocked by or Effort, or adding, splitting or removing a ticket, rebuild the overview with the `tickets-pdf` skill (pass the feature's directory if it isn't `.scratch/freecal-v1`). Once per batch of changes, not per edit. A comment or a ticked box alone doesn't need it

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` (the Notes / Decisions-so-far / Fog body).
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `**Type:**` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `**Status:**` line records `claimed`/`resolved`.
- **Blocking**: a `**Blocked by:** NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
