# Redesign plan: tickets from 173 on

The plan for rewriting the tickets that implement ADRs 0017 to 0020 (Calendar History, Unimplemented Fields, sync from the server's marks, Undo as a Snapshot, Conflicts) and for implementing them. Each batch below runs in a fresh context window of at most 130k tokens; start one with "run batch Bn of `.scratch/freecal-v1/redesign-plan.md`". A batch ends by committing, ticking its line under "Batches" and writing anything the next batch needs into this file, never only into the conversation.

Start of the redesign: tag `design/snapshot-line` (commit `065a8e8`). Diffs and reviews of the redesign as a whole start there.

## Decisions (2026-11-28, agreed with the user)

1. **Switched-off tests, not glue code.** A ticket doesn't write code that a later ticket deletes only to keep an old test passing. A test whose feature is being replaced is switched off on purpose: `#[ignore = "until ticket NN: <why>"]` in Rust; in the frontend, the comment `// until ticket NN: <why>` on the line right before `.skip` (changed in ticket 173: switching the test back on leaves its name as it was). Ticket NN switches it back on or rewrites it, and has a box for that. A test of something that goes for good (the Change Journal, Propagate, Unsent Changes) is deleted in the ticket that removes the feature. Every other test stays green, so "the checks pass" still means a ticket is done.
   - A test stays switched off for at most 3 to 4 tickets. During that window, the new code that replaces the feature is covered by tests at its own seam.
   - A check in the pre-commit hook makes sure every switched-off test names a ticket that isn't done (first ticket, 173).
   - The app may lack a feature between tickets (e.g. no undo between the new Event Versions and the new Undo): no real Local Store exists.
2. **Ticket size by tokens.** A ticket fits in one context window: about 20k for orientation, about 80k for the work, room left for the hook's output. In practice one layer (core, shell or frontend) and a diff of roughly 300 to 600 lines. The Effort line stays (S or M); the token budget is the binding limit.
3. **Short tickets.** A new ticket states the outcome, the boxes and the switched-off tests it turns on or off, and cites ADRs, spec sections and the layout plan instead of repeating them. No grilling history. Decisions that aren't in the ADRs or the spec go into the ticket text, not into comments.
4. **Numbering from 173.** The old tickets 133 and later are deleted (git keeps them), except 139, 140, 141 and 170 (ready-for-human, unrelated), which stay as they are. In scope as well: the Google tickets 163 and 164, and older tickets that build on the redesign (15, 21, 22 and whatever else B1 finds); these are rewritten as new tickets too, or only have their references fixed if their content still holds.
5. **A new skill, `implement-redesign`**, a variant of `.claude/skills/implement/SKILL.md`: reads the layout plan instead of mapping the code with an Explore subagent; knows the rules for switched-off tests; one ticket is one session in one layer, so no stopping between layers; records each departure from the layout plan in that plan, with the ticket's number. `/code-review` still follows every ticket, in a fresh session (the main safety net while tests are switched off), through `/code-review-redesign`, which fixes the findings, decides the judgement calls itself, files work outside the ticket as `needs-triage` tickets and commits (changed 2026-11-28, agreed with the user). The rules both skills share are in `docs/agents/redesign-tickets.md`.
6. **Sequential, on the current branch.** No `implement-spec` and no parallel worktrees: the tickets touch the same files.
7. **`/simplify`** at the end of every phase, on the phase's diff, given the layout plan so that it keeps structure that was chosen on purpose. **`/improve-codebase-architecture`** twice: after the sync phase (before the Conflict and window tickets build on it), and at the end. Not before (the layout plan does that job) and not in between (glue and switched-off tests would mislead it).
8. **A layout plan for the backend** (`.scratch/freecal-v1/redesign-layout.md`): modules and files, main types and tables, the seams where `/tdd` tests sit, and which tests through the core interface (`core/src/lib.rs`) survive the whole redesign. Central interfaces (the store of Event Versions, the comparison of the two sides, the Incoming Sync) designed with `/codebase-design`'s "design it twice". A target that tickets may amend, not a contract. New names are proposed as new terms for the user to approve (CLAUDE.md).

## Phases (draft; the ticket plan below settles them and lists its changes)

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

- P0: started at `366db47` (ticket 173).
- P1: started at `219b694` (ticket 174).

## Batches

- [x] **B1, the ticket plan.** Read `GLOSSARY.md`, ADRs 0017 to 0020, the spec's sections they touch, `CODING_STANDARDS.md`, and the old tickets 133 and later (skim; their "What to build" and boxes, not their comments). Find older tickets that name any of them in `Blocked by` or text (`grep -l`). Sort the existing tests (Rust and frontend) into survive / rewrite / delete-with-their-feature, at the level of files or test modules, via an Explore subagent asked for a list, not contents. Write into this file, under "Ticket plan": every new ticket (number from 173, title, a few sentences of scope, layer, phase, Blocked by, the old tickets it replaces), the table of switched-off tests (test, off in ticket, on in ticket; check the 3 to 4 ticket limit), and the test sorting. Each old ticket in scope maps to new tickets or to "dropped, because …". Don't write ticket files yet.
- [x] **B2, the layout plan and the skill.** Map the backend through an Explore subagent (seams, modules, tables, `file:line` pointers, no contents). Write `.scratch/freecal-v1/redesign-layout.md` as in decision 8, ordered by phase, naming which new ticket creates or changes each module. Propose new terms to the user. Adjust the ticket plan where the layout shows a ticket too large or in the wrong order. Write `.claude/skills/implement-redesign/SKILL.md` per decision 5 (use `/writing-for-agents`).
- [x] **B3 and later, the ticket files.** Write about 8 to 10 tickets per batch (173 to 214) from the ticket plan and the layout plan, using the terms of `redesign-layout.md`'s "New terms" (approved 2026-11-28), in the format of `docs/agents/issue-tracker.md` and decision 3, each citing the sections of `redesign-layout.md` it builds. Note here which numbers each batch wrote.
  - [x] B3 wrote 173 to 182 (2026-11-28).
  - [x] B4 wrote 183 to 192 (2026-11-28).
  - [x] B5 wrote 193 to 202 (2026-11-28).
  - [x] B6 wrote 203 to 214 (2026-11-28).
  - **Conventions B3 set, for the later batches:** a ticket that takes over an old one cites it as `` `git show design/snapshot-line:.scratch/freecal-v1/issues/NNN-….md` `` (the old files are deleted in the last batch) and still states the outcome in a few bullets, so it reads without the old file. Order: title, **What to build** with bullets, one line `Spec: …. ADRs …. Layout: "<section>", …`, then Blocked by, Status, Effort, then **Switched-off tests** / **Deleted tests** where the plan's tables name some (with the exact `until ticket NN: <why>` note), then the boxes. A box "`GLOSSARY.md` follows" where the plan's glossary table lists the ticket. Never write the bold Status label in a ticket's body: the hook's ticket check reads the first one.
  - **Choices B3 made that later tickets rely on:** 178 keeps old 145's carried ID column (glue named in the ticket; 182 deletes it, as this plan's table says). Until 182 every write, 180's Undo included, goes through the `JournalTransaction`, which calls the `SnapshotWriter`, so Unsent Changes are still recorded and an Undo reaches the server. 180 refuses an Undo whole where it would leave out an Event (Conflict, read-only or removed Calendar); 181 replaces the refusal by the report. 181 rewrites `guests.rs`' switched-off test to ADR 0020's rule (Events with guests are undone like any). No ticket before 186 tests the Unimplemented Fields through an Undo (the Event Versions don't carry them yet). 177 is M (old 167 was S, plus old 21's rules). `tickets-pdf` wasn't run per writing batch while old and new ticket files overlapped; the last batch of writing ran it. That hold ended with the writing batches: implementing and reviewing a ticket runs `tickets-pdf` per `docs/agents/redesign-tickets.md`, "Finishing" 3.
  - **Choices B4 made that later tickets rely on:** 183 also adds to the core the number of Events an Undo changes per Calendar (old 158's core box; too small to reopen 180), so it is "window (+ the counts)". An Event's History line "replaced by the server's version" moves from 184 to 190, as it needs the Conflict. 185 drops at start and once a day (not at the start of each write), and writes roots and edges as one entry each for 188 and 190. 189 runs the First Sync, until 194, through the same plan and writer with an empty `Base`; it treats what `compare_event` could combine as a Conflict until 198. 190 is M (old 133 was S; it gains the window's sync display, the History line and old 21's Conflict Events). 190 adds the open-Conflict count to "Sync status changed"; 202 builds on it. 191's restore of a changed Event asks first (181), no longer a refusal. **For B5 and B6:** old 152's test "the Mass-Change Guard doesn't count a Server Event Replacement" belongs in 201; old 133's "untrusted base leaves the Last Agreed Version empty" test in 195; old 134's "restore by 'Undo this Event' in the Calendar History resolves the Conflict" in 207.
  - **Choices B5 made that later tickets rely on:** 193's server's form in place is one function of `history/` that 213 reuses for Google's answer. 194 ends 179's one Snapshot per First Sync page and 189's separate First Sync path; the window's "Loading…"/"Not loaded yet" texts come back in 202. 195 makes the untrusted base a case of the plan that is always a Server Event Replacement; 198's merge branch leaves that case alone (its test sits in 198). 198 adds the core's report of each Incoming Sync's Server Event Merges in the Signal and the Event's History line "on the server, before combining" (184 came first); 179 already has the zero-length trigger. **For B6:** 206's toast reads 198's Signal; 207 adds the Calendar History's line "combined with a change on the server" (old 168), the last of 159/160/168; 205 wires the Conflict mark's click, which 202 leaves doing nothing; 203 writes the words and actions for 200's "writes refused" (bar, "Stop waiting" with 200's count, the error when removing the mark fails) and 201's paused Sync Error ("Send", the unsent reason "paused"); 200 and 201 leave the window a plain fallback text for their new Sync Error kinds. 200 drops old 166's "Stop waiting ends a Conflict" test: ADR 0019 doesn't say so, and 191's rules end Conflicts.
  - **Choices B6 made, for the last batch and the implementers:** Blocked by differs from the table above in 205 (adds 202, whose Conflict mark it wires) and 207 (adds 191 and 198: "Undo this Event" on a Conflict is the restore, and it names merged Events); the table is updated. 204 adds a field to the Occurrence the core interface returns, whether its Event is in Conflict, read from 190's `conflict` table (no new storage; 190 sends "Occurrences changed" but leaves the Occurrence type as it is), so it is "window (+ the Occurrence's flag)". 203 writes the bar component that 206's Conflict bar reuses. 206's merge toast has no "Show" until 207 adds it. 210 writes no migration for neutral names (no real Local Store; old 163 asked for one). The glossary box for Google Server Object sits in 212, which creates `google_server_object`, not 210. 212 marks every Google Calendar read-only and refuses removing the mark until 213, which applies 200's rule. 214 refuses a re-login as another Google account (new, not in old 26). The last batch deletes old 22, 25, 26 and 172 too, as 201/203, 212, 213/214 and 209 replace them.
- [x] **Last batch, the switch.** Delete the old ticket files 133 and later (except 139, 140, 141, 170) and the old tickets that new ones replace. Fix references to old ticket numbers in the remaining tickets, the spec, the ADRs, `GLOSSARY.md` and `TODO.md` (`grep -rn -E 'ticket[s]? (1[3-7][0-9])'` and the `Blocked by` lines). Run `tickets-pdf`. Commit.
  - **What the switch did (2026-11-28):** deleted old 133 to 138, 142 to 169, 171, 172, 21, 22, 25 and 26. Renumbered the bodies and `Blocked by` lines of the open tickets 09, 11, 12, 15, 18, 23, 24, 27, 68, 71, 73, 74, 75, 126, 140, 170 and freecal-v2's 01 and 02, plus ADRs 0013, 0015, 0019, `TODO.md`, `docs/research/caldav-duplicate-uids.md` and four code comments; 20's "Split into" now names 190, 191, 199, 204, 205. Dated comments, done and wontfix tickets, the critiques and ADR history lines keep the old numbers (the table "Old tickets in scope" maps them); open tickets whose comments cite them got a comment saying so. Choices: 11 drops old 138's rule (the sides are compared at the next Incoming Sync) and is blocked by 182 for old 147; 24's one-sided Undo of a move cites 181; 71 is blocked by 181 (old 148 there meant the Undo); 27 is blocked by 211 (the login).

## Tickets added during implementation

Tickets that reviews found outside their ticket, with the review that found them. `/code-review-redesign` writes them as `needs-triage`; the user promotes them.

- 215, every recurrence ID is readable: from the review of 174. Blocked by none. Done.
- 216, the Store Check reads every Event Version; the writer reads its own text back: from the conformance discussion of 2026-11-28 (`conformance.md`). Blocked by 179; runs after 179 and before 180, out of number order. 186 is blocked by it.
- 217, an Exception created on both sides is compared with its Occurrence: from the build of 177 (with the user). Blocked by 198.
- 218, an edited description drops its HTML: from the review of 177. Blocked by 186, 187.

## Ticket plan

Written by B1 (2026-11-28). B2 may adjust sizes and order where the layout shows a need; record each change here.

### Changes to the draft phases

- **Conflicts move into P3.** The comparison ticket (189) ends the old Conflict detection, so the Conflict tests switched off there have to come back within 3 to 4 tickets: recording and resolving Conflicts (190, 191) follow it directly. P4 keeps the Server Event Merge, what each side changed, writes refused and the Mass-Change Guard.
- **An Event's History moves into P2** (184), right after the Change Journal goes, for the same reason. The Calendar History window stays in P5: the Change Journal dialog's tests are deleted with the dialog (182), and 207 writes new ones (see the table).
- **Old ticket 21 is split**, not rewritten: its rule per Occurrence for Recurring Events belongs in the comparison (177), its whole-UID Conflict in 190 and 191, and "what each side changed" per Occurrence in 199.
- **Old 147 is split** into the core's Undo (180, and asking first in 181), the removal of the Change Journal (182) and, as before, the window (183).
- **B2's changes** (from the layout plan, `redesign-layout.md`): old 180 split into 180 (the Undo of a whole Snapshot, Ctrl+Z and redo) and 181 (asking first, limits to one UID, moves), as the Undo and its test rewrites don't fit one context window; the server undo tests come back in 181, not with the Change Journal's removal, which shrinks 182. Tickets from the old 181 on are renumbered by one (now up to 214). `begin` and `end` move from 179 into 178, so 178 writes no placeholder for `end`. Old 147 now maps to 180, 181 and 182.
- **Layers.** Where a ticket changes the core interface's types, the window has to follow in the same ticket, because the generated types bind them (174, 182, 187, 190, 194). These tickets keep the window's part to what the type change forces.

### New tickets

| # | Title | Scope | Layer | Phase | Blocked by | Replaces |
|---|---|---|---|---|---|---|
| 173 | Switched-off tests name an open ticket | A pre-commit check: every `#[ignore = "until ticket NN: …"]` and every `it.skip` carries such a note, and ticket NN exists and isn't done. Bare `#[ignore]` and `it.skip` are refused. | tooling | P0 | none | — (decision 1) |
| 174 | An Event is identified by its Combined Event Key | 142 as written: the core interface and the window name Events by key; `EventId` leaves the generated types; the row ID stays inside the core. | core + window | P1 | none | 142 |
| 175 | Unimplemented Fields: the tree, its canonical form and the CalDAV reader | 143 as written. Pure code. | core | P1 | none | 143 |
| 176 | The CalDAV writer builds a resource from managed fields and the tree | 144 as written. Pure code. Takes over the ideas of the patching writer's tests worth keeping (hostile Event Rows, folding and CRLF, VTIMEZONE), as `resource/tests.rs` is deleted in 187. | core | P1 | 175 | 144 |
| 177 | The comparison of the two sides, as one function | 167, plus old 21's rule per Occurrence: a change to a Recurring Event counts for every Occurrence, an Exception's for its own; the Field Groups from one exhaustive function; a winning side for "… all over the later changes". Pure, unit-tested. | core | P1 | 175 | 167, part of 21 |
| 178 | Event Versions are rows that never change; `event` becomes a view | 145 as written, with a bare `snapshot` table (ID and time) so that `begin` and `end` refer to Snapshots from the first row (B2). The Change Journal keeps recording. | core | P2 | 174 | 145 |
| 179 | The Snapshot's origin, the line's triggers and the Store Check | 146: every write makes one Snapshot with its origin (an action of the user records which), each First Sync page an Incoming Sync Snapshot as any write is (no special case). Adds the core's part of removing a Calendar (ADR 0018, "Removing a Calendar"): its Event Versions go, Snapshots left empty go. | core | P2 | 178 | 146, core part of 172 |
| 180 | Undo and redo are Snapshots in the core | First part of 147: an Undo of a whole Snapshot through 177's comparison, made at once where every Event can be taken back and refused with the toast where it would change nothing; the next Ctrl+Z's Snapshot (in effect, passing over Incoming Sync Snapshots and Undos) and the next redo's Undo. New core functions and interface types beside the Change Journal's undo, which stays until 182; the window doesn't call them yet. Rewrites the undo tests of `recurrence_scopes.rs`, `recurring_events.rs`, `import_run.rs` and the reusable whole-Snapshot scenarios of `undo_per_event.rs`. | core | P2 | 177, 179 | 147 (part) |
| 181 | An Undo asks first about what it leaves out | Second part of 147: an Undo limited to one UID; Events left out with their reason (Conflict, read-only, removed Calendar) and nothing written until answered; moves with one side unwritable; "Undo the others" and "… all over the later changes" (177's winning side); Ctrl+Z passing over Snapshots of which nothing can be written. The core interface reports what was left out; the window only shows it in plain words (183). Rewrites the server undo tests switched off in 180. | core | P2 | 180 | 147 (part) |
| 182 | The Change Journal goes | Last part of 147: its tables, the hash and its migrations, undo per Event, the listings, the Dangerous Undo and the carried ID column, from the core, the interface and the window (the Change Journal's dialog, which today's code calls "Undo History", though the glossary's Undo History is now what Ctrl+Z steps through; and the History button). Keeps `mass_change_guard::exceeded` for 201. `GLOSSARY.md` and `CODING_STANDARDS.md` follow. | core + window | P2 | 181 | 147 (part) |
| 183 | Ctrl+Z and redo in the window: the toasts and the dialog asking first | 158 as written, on the interface of 180 and 181. | window (+ the counts, B4) | P2 | 182 | 158 |
| 184 | An Event's History in its details | 160 without its three lines that need later tickets: "changed in fields FreeCal doesn't show" moves to 186, "since before <date>" to 185, "replaced by the server's version" to 190 (B4). | core + window | P2 | 182 | 160 |
| 185 | Snapshots dropped after 90 days | 148, with the roots and edges known so far (90 days, Undo → taken back); later tickets add their root or edge (188, 190). An Event's History reads "since before <date>". | core | P2 | 182 | 148 |
| 186 | Event Versions carry Unimplemented Fields | 149 as written; an Event's History reads "changed in fields FreeCal doesn't show". | core | P2 | 175, 184 | 149 |
| 187 | The Sender builds every resource from Event Versions; the Mirror's text goes | 150 as written; the window loses the view of the server's original text. | core + window | P2 | 176, 186 | 150 |
| 188 | The server's Current-Version Mark | 151 as written; adds the drop's root for versions with the server's mark and the trigger refusing their delete. | core | P3 | 185, 187 | 151 |
| 189 | The comparison against the server's marks; Incoming Sync Snapshots and Server Event Replacements; Propagate goes | 152 as written, through 177. Every Event changed differently on both sides is replaced for now (the Server Event Merge is 198). Ends the old Conflict detection. | core | P3 | 177, 188 | 152 |
| 190 | The core records and reports Conflicts | 133 as written, with old 21's whole-UID Conflict (a Conflict Event per single Event of the UID) and old 15's last box (overall status Conflict); the window's sync display follows the new Sync Status. | core + window | P3 | 189 | 133, last box of 15, part of 21 |
| 191 | The user resolves a Conflict | 134 as written, the restore of a whole Recurring Event's UID included. | core | P3 | 190 | 134, part of 21 |
| 192 | An Outgoing Sync sends what differs between the Current-Version Marks; `unsent_change` goes | 153 as written. | core | P3 | 189 | 153 |
| 193 | One sync activity per Calendar; read-back; the server's form takes the sent one's place | 154 as written. | core | P3 | 192 | 154 |
| 194 | A server Calendar is stored empty when added; its First Sync is a Full Fetch | 155 as written; the window drops the First Sync flag. | core + window | P3 | 193 | 155 |
| 195 | Mark Reconciliation and the untrusted base | 171 as written. | core | P3 | 190, 194 | 171 |
| 196 | Write Permissions and Guards go | 157 as written. | core | P3 | 193, 194 | 157 |
| 197 | The round-trip check at fetch | 169 as written. | core (+ a note in the details) | P3 | 186, 187 | 169 |
| 198 | The Server Event Merge | 168's core part; its toast moves to 206. | core | P4 | 190 | 168 (core) |
| 199 | A Conflict shows what each side changed | 135, with old 21's Occurrences named per side for a Recurring Event. | core | P4 | 190 | 135, part of 21 |
| 200 | Writes refused: changes wait as when offline | 166's core part: the Sync Error "writes refused", privileges read on every Poll, the mark only by the user or when added, "Stop waiting" as a Snapshot of its own origin, removing the mark asks the server. | core | P4 | 180, 192 | 166 (core) |
| 201 | The Mass-Change Guard checks every Outgoing Sync | 22's core part, including the call "Send". | core | P4 | 192 | 22 (core) |
| 202 | Status Marks, the Unsent Mark and the progress of a long sync | 161 as written. | window (+ the Signal) | P5 | 192, 194 | 161 |
| 203 | The window's actions on Sync Errors | "Send" on a paused Calendar (22), the bar for "writes refused" with "Stop waiting and mark read-only" and removing the read-only mark (166). | window | P5 | 200, 201, 202 | 22 (window), 166 (window) |
| 204 | The Conflict badge on Event chips | 136 as written. | window (+ the Occurrence's flag, B6) | P5 | 190 | 136 |
| 205 | The Conflict list in the window | 137 as written. | window | P5 | 191, 199, 202, 204 (202 added by B6) | 137 |
| 206 | The Conflict bar and the toasts | 162, plus 168's toast after a Server Event Merge. | window | P5 | 198, 205 | 162, 168 (toast) |
| 207 | The Calendar History window | 159 as written, with new tests in place of the deleted Change Journal dialog's. | core + window | P5 | 183, 185, 191, 198 (191, 198 added by B6) | 159 |
| 208 | The dialog that chooses server Calendars to add | 156 as written. | window | P5 | 194 | 156 |
| 209 | Removing a Calendar says it can't be undone | 172 as written; the core's part is in 179, 190, 192. | window (+ counts) | P5 | 11, 179, 192 | 172 |
| 210 | Google's reader and the neutral names | 163 as written. | core | P6 | 176, 186 | 163 |
| 211 | A Google Account: browser login, Available Calendars, removal | 164 as written. | core + shell + window | P6 | none | 164 |
| 212 | Add a Google Account's Calendars and pull them, read-only | 25, reworded to the marks (it still names a pointer and a Join). | core | P6 | 194, 208, 210, 211 | 25 |
| 213 | Write to Google Calendars | 26's boxes on writing: conditional writes from the Event Versions, Google's answer as the server's form, Undo, Conflicts, Mass-Change Guard and offline against the fake Google server, an Exception without its Recurring Event. | core | P6 | 193, 201, 212 | 26 (part) |
| 214 | Google login expiry; exports keep Google's fields | 26's other boxes: an expired login is a Sync Error asking to log in again; `google:` entries exported as `X-` properties and read back on import. | core + window | P6 | 09, 213 | 26 (part) |
| 217 | An Exception created on both sides is compared with its Occurrence | From 177's build: two new Exceptions for one Occurrence are compared with the Occurrence the base's Recurring Event gives, by Field Group, instead of always a Conflict. | core | P4 | 198 | none |

Implementation order is the numbering, except 216, which runs after 179 and before 180. 211 has no blocker in the redesign and can go anywhere.

### Old tickets in scope

| Old | Goes to |
|---|---|
| 133, 134, 135, 136, 137 | 190, 191, 199, 204, 205 |
| 138 | dropped: wontfix already; the User's Pointer is gone (ADR 0018) |
| 142, 143, 144, 145 | 174, 175, 176, 178 |
| 146 | 179 |
| 147 | 180, 181, 182 |
| 148, 149, 150, 151, 152, 153, 154, 155, 156, 157 | 185, 186, 187, 188, 189, 192, 193, 194, 208, 196 |
| 158, 159, 160, 161, 162 | 183, 207, 184, 202, 206 |
| 163, 164 | 210, 211 |
| 165 | dropped: wontfix already; Held during Undo is gone (ADR 0020) |
| 166 | 200, 203 |
| 167 | 177 |
| 168 | 198, 206 |
| 169, 171, 172 | 197, 195, 209 (core part in 179) |
| 139, 140, 141, 170 | stay as they are |
| 15 | stays; only its last box is open, which 190 ticks; references fixed |
| 18 | stays; references fixed (Blocked by 190, 192, 201) |
| 20 | stays wontfix |
| 21 | dropped, because it is split into 177, 190, 191 and 199 |
| 22 | 201, 203 |
| 23 | stays; references fixed (Blocked by 16, 190) and "a Conflict stops from syncing" reworded to "the server's version replaces changes" |
| 25 | 212 |
| 26 | 213, 214 (it was L) |
| 09, 11, 12, 24, 68, 71, 73, 74, 75, 126 | stay; references fixed (143 → 175, 144 → 176, 147 → 180 to 182, 148 → 185, 149 → 186, 153 → 192, 161 → 202, 163 → 210, …); 71 cites ADR 0018's former section "The pointers" |

### Switched-off tests

Spans count tickets from off to on; all are within 3. Tests not listed are either untouched, rewritten in the ticket that changes their feature, or deleted with it (see "Test sorting").

| Test | Off in | On in | Why |
|---|---|---|---|
| `core/tests/caldav_sync.rs`, the undo of a sync (`restores_*`, `of_a_journal_transaction_*`, `of_a_delete_*`, `brings_back_*`, `of_one_event_of_a_sync_*`, `undoing_the_sync_that_brought_*`) | 180 | 181 | undo becomes a Snapshot; 181 rewrites them with the Undo that asks first |
| `core/tests/caldav_send.rs`, `undoing_*` and `an_undone_sync_*` | 180 | 181 | same |
| `core/tests/caldav_first_sync.rs`, the two `comes_back_with_the_undo…` tests | 180 | 181 | same |
| `core/tests/guests.rs`, `undoing_a_sync_skips_an_event_with_guests` | 180 | 181 | same |
| `src/lib/UndoDialog.test.ts`, the non-dangerous tests | 182 | 183 | the interface's undo types change; 183 rewrites the toasts and dialog |
| `src/lib/windowDialogs.test.ts`, the Undo flow outside the describe "the Undo History" (the Change Journal's dialog) | 182 | 183 | same |
| `src/App.events.test.ts`, the undo toast and Ctrl+Z tests | 182 | 183 | same |
| `src/App.import.test.ts`, "Undo undoes its Journal Transaction", "offers no Undo" | 182 | 183 | same |
| the core's tests of `event_history` and `App.events.test.ts`'s "has its History" | 182 | 184 | An Event's History is read from the Calendar History |
| `core/tests/caldav_send.rs`, the ETag and Conflict group (`an_edit_is_not_written_over_a_newer_*`, `an_update…`/`a_delete_without_a_stored_etag_waits`, `a_write_answered_without_an_etag_*`) | 189 | 192 (the last one 193) | the old Conflict detection goes; 192 sends from the marks, 193 reads back |

Deleted, not switched off, though their feature comes back: the Change Journal's listing (`core/tests/change_journal_list.rs`, `src/App.journal.test.ts`, `windowDialogs.test.ts` "the Undo History") goes in 182, and the Calendar History window (207) is 25 tickets later; 207 writes its tests anew from US-83 and US-155, using those files' scenarios (git keeps them).

### Test sorting

From an Explore subagent's sort by names and keywords, not test bodies; B2 confirms where it reads the code.

**Survive** (at most the rename of `EventId` to the key in 174, and helpers updated): `core/tests/` `caldav_accounts`, `caldav_discovery`, `caldav_calendars` (check `removing_an_account_removes_…_their_history` in 179), `clock`, `settings`, `sidebar_state`, `view_state`, `local_account`, `local_store`, `local_calendars`, `event_times`, `import`, `events`, `sync_timing`, most of `sync_status`, the Ignored Resource and pairing group of `caldav_sync` (harness changes where they assert the Mirror, in 187), the scheduling and UID hand-on group of `caldav_send`; unit tests in `sync/caldav/mod.rs`, `caldav.rs`, `import/`, `structured_text.rs`, `guests.rs`, `event/when.rs`, `event/recurrence_id.rs`, `sync_scheduler.rs`, `mass_change_guard.rs` (kept for 201); every test in `src-tauri/`; frontend `App.test`, `App.navigation`, `App.settings`, `App.sidebar`, `fakeCore.test`, `tauriCore.test` and `i18n` (keys updated), `Toast`, `shortcuts`, `skippedEvents`, `Sidebar.accounts`, `syncDisplay` (its Conflict state in 190), and the `src/lib` tests of `browserContextMenu`, `colourScheme`, `EventEditor`, `gridDrag`, `palette`, `repeat`, `ScopeDialog`, `Sidebar`, `week`, `when`, `windowModel`; the Scope Prompt, one-dialog, writes and import groups of `windowDialogs.test.ts`. Helpers survive: `core/tests/support/` (`yoga()` returns a key from 174), `src/test/*` (`answers.ts` and `ids.ts` follow the interface).

**Rewrite**, in the ticket named:
- the undo tests of `recurrence_scopes.rs`, `recurring_events.rs`, `import_run.rs` (the "no Journal Transaction" ones become "no Snapshot") and the reusable scenarios of `undo_per_event.rs`: 180;
- the server undo tests listed above: 181; `UndoDialog`, the undo parts of `windowDialogs`, `App.events`, `App.import`: 183; `event_history`: 184;
- `caldav_send.rs` "send results" (the patching writer's assertions, unknown fields kept): 187;
- `caldav_sync.rs` "pull" group (`a_sync_pulls_*`, `a_sync_deletes_*`, `a_sync_replaces_*`, `a_sync_token_*`, floating zones): 189;
- `offline_editing.rs`, `sync/caldav/sender.rs` tests on resource naming and refusals: 192;
- `caldav_first_sync.rs`, `sync_status.rs`'s First Sync tests: 194;
- `read_only.rs` (without the Guards' exceptions): 196;
- `caldav_sync.rs` read-only and permission group, `caldav_send.rs` `a_write_the_server_refuses_is_a_sync_error_of_the_calendar`, `caldav.rs` `a_write_refused_for_its_content_is_no_refusal_of_writes`: 200 (the privilege read once at adding survives);
- `CalendarGrid.test.ts` "an Occurrence with Unsent Changes", `unsent.test.ts`, the unsent count in `App.sync.test.ts`: 202; `App.quit.test.ts` survives as long as the interface reads as before (192's box), else 202.

**Delete**, in the ticket that removes the feature:
- 182: `core/tests/change_journal.rs`, `change_journal_list.rs`, the Dangerous Undo and per-Event mechanics of `undo_per_event.rs`, `journal/mod.rs` tests, the Dangerous Undo tests in `caldav_sync.rs`, `App.journal.test.ts`, `UndoDialog`'s and `windowDialogs`'s dangerous parts;
- 187: `sync/caldav/resource/tests.rs` (ideas carried to 176), `store/schema.rs` `an_href_is_never_both_mirrored_and_ignored` (or rewritten for the Ignored Resources);
- 192: the Unsent Changes and Sent Event Rows group of `caldav_send.rs`, the stored send-failure tests of `sender.rs`;
- 194: `caldav_sync.rs` `a_calendar_whose_first_sync_didnt_finish_*`, `sync_status.rs` `a_calendar_quit_during_its_first_sync_*`, the three "First Sync is unfinished" tests of `App.sync.test.ts`, `unsent.test.ts` "an unfinished First Sync holds the changes back";
- 196: `store/mod.rs` `only_its_guard_makes_a_write_permission`.

### Answers from B2

- **Unsent Changes between 182 and 192**: little code, so the order stays. The write is one call in the Change Journal's recording step (about 12 lines), computed from the previous and new versions, not from journal rows; 182 moves it into the Snapshot Writer, and the Sender's three hooks (about 50 lines) move to the Sender, comparing row IDs in place of the hash. 192 deletes both with the table. Details: `redesign-layout.md`, P2, 182.
- **180 and 182's size**: 180 split (see "B2's changes"). 182 is mostly deletions once the server undo tests move to 181.
- **177 and the Event Version's type**: 177 compares a value type of its own, the Event Version Content, which 178's row stores; no order change.
- **Old Conflict tests**: none exist (no Conflict code yet); the ETag group of `caldav_send.rs` in the table is all that 189 switches off.

### Consistency with `GLOSSARY.md` (checked 2026-11-28)

The plan's decisions agree with the glossary's entries; the plan uses no term that is neither there nor in ADRs 0017 to 0020 ("switched-off test" is this plan's process word, not a domain term). Two points to know:

- **The First Sync between 179 and 194.** The glossary (and ADR 0018) has a First Sync take in everything as one Incoming Sync Snapshot. From 179 until 194 replaces the old First Sync, each page it writes is an Incoming Sync Snapshot of its own, as every write makes a Snapshot. That is the old First Sync recorded honestly, not glue; 179's text says so, and no test asserts one Snapshot per First Sync before 194.
- **"Undo History"** names a dialog in today's code (the Change Journal's) but, in the glossary, what Ctrl+Z steps through. 182 removes the dialog; 183's toasts and 207's window must not take the name back for a dialog.

Entries marked "until the code follows ADR …" are revised by the ticket that makes the change; each such ticket gets a box "`GLOSSARY.md` follows" for these:

| Ticket | Entries |
|---|---|
| 178 | Event Row (`event` is a view), Event Version (no longer the Change Journal's), Server Object and Corresponding Server Object (Event Versions), Recurrence ID as obtained |
| 182 | Change Journal, Journal Transaction, Journal Event Modification, Record a Journal Event Modification, Transaction Origin (a Snapshot's origin), Dangerous Undo go; Database Transaction and Incoming Sync lose the Journal Transaction; First Sync's "the Change Journal doesn't record it" |
| 186 | Event Version (Unimplemented Fields stored) |
| 187 | Mirror goes; CalDAV Server Object (no text), Known Server Resources, Ignored Resource ("kept in the CalDAV table"), Database Transaction (the Mirror's writes), Sync Engine ("the only code that writes the Mirror") |
| 189 | Propagate a Server Object goes |
| 192 | Unsent Change (derived, no list), Sent Event Rows goes, Sender, Outgoing Sync ("changes the Mirror"), Combined Event Key ("what an Unsent Change is about") |
| 196 | Write Permission, Guard, Unguarded Write go |
| 212 | Google Server Object (no JSON) (moved from 210 by B6: 212 creates the table) |
