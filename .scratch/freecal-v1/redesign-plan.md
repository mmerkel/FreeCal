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

## Batches

- [x] **B1, the ticket plan.** Read `GLOSSARY.md`, ADRs 0017 to 0020, the spec's sections they touch, `CODING_STANDARDS.md`, and the old tickets 133 and later (skim; their "What to build" and boxes, not their comments). Find older tickets that name any of them in `Blocked by` or text (`grep -l`). Sort the existing tests (Rust and frontend) into survive / rewrite / delete-with-their-feature, at the level of files or test modules, via an Explore subagent asked for a list, not contents. Write into this file, under "Ticket plan": every new ticket (number from 173, title, a few sentences of scope, layer, phase, Blocked by, the old tickets it replaces), the table of switched-off tests (test, off in ticket, on in ticket; check the 3 to 4 ticket limit), and the test sorting. Each old ticket in scope maps to new tickets or to "dropped, because …". Don't write ticket files yet.
- [ ] **B2, the layout plan and the skill.** Map the backend through an Explore subagent (seams, modules, tables, `file:line` pointers, no contents). Write `.scratch/freecal-v1/redesign-layout.md` as in decision 8, ordered by phase, naming which new ticket creates or changes each module. Propose new terms to the user. Adjust the ticket plan where the layout shows a ticket too large or in the wrong order. Write `.claude/skills/implement-redesign/SKILL.md` per decision 5 (use `/writing-for-agents`).
- [ ] **B3 and later, the ticket files.** Write about 8 to 10 tickets per batch from the ticket plan and the layout plan, in the format of `docs/agents/issue-tracker.md` and decision 3, each citing the sections of `redesign-layout.md` it builds. Note here which numbers each batch wrote.
- [ ] **Last batch, the switch.** Delete the old ticket files 133 and later (except 139, 140, 141, 170) and the old tickets that new ones replace. Fix references to old ticket numbers in the remaining tickets, the spec, the ADRs, `GLOSSARY.md` and `TODO.md` (`grep -rn -E 'ticket[s]? (1[3-7][0-9])'` and the `Blocked by` lines). Run `tickets-pdf`. Commit.

## Ticket plan

Written by B1 (2026-11-28). B2 may adjust sizes and order where the layout shows a need; record each change here.

### Changes to the draft phases

- **Conflicts move into P3.** The comparison ticket (188) ends the old Conflict detection, so the Conflict tests switched off there have to come back within 3 to 4 tickets: recording and resolving Conflicts (189, 190) follow it directly. P4 keeps the Server Event Merge, what each side changed, writes refused and the Mass-Change Guard.
- **An Event's History moves into P2** (183), right after the Change Journal goes, for the same reason. The Calendar History window stays in P5: the Change Journal dialog's tests are deleted with the dialog (181), and 206 writes new ones (see the table).
- **Old ticket 21 is split**, not rewritten: its rule per Occurrence for Recurring Events belongs in the comparison (177), its whole-UID Conflict in 189 and 190, and "what each side changed" per Occurrence in 198.
- **Old 147 is split** into the core's Undo (180), the removal of the Change Journal (181) and, as before, the window (182).
- **Layers.** Where a ticket changes the core interface's types, the window has to follow in the same ticket, because the generated types bind them (174, 181, 186, 189, 193). These tickets keep the window's part to what the type change forces.

### New tickets

| # | Title | Scope | Layer | Phase | Blocked by | Replaces |
|---|---|---|---|---|---|---|
| 173 | Switched-off tests name an open ticket | A pre-commit check: every `#[ignore = "until ticket NN: …"]` and every `it.skip` carries such a note, and ticket NN exists and isn't done. Bare `#[ignore]` and `it.skip` are refused. | tooling | P0 | none | — (decision 1) |
| 174 | An Event is identified by its Combined Event Key | 142 as written: the core interface and the window name Events by key; `EventId` leaves the generated types; the row ID stays inside the core. | core + window | P1 | none | 142 |
| 175 | Unimplemented Fields: the tree, its canonical form and the CalDAV reader | 143 as written. Pure code. | core | P1 | none | 143 |
| 176 | The CalDAV writer builds a resource from managed fields and the tree | 144 as written. Pure code. Takes over the ideas of the patching writer's tests worth keeping (hostile Event Rows, folding and CRLF, VTIMEZONE), as `resource/tests.rs` is deleted in 186. | core | P1 | 175 | 144 |
| 177 | The comparison of the two sides, as one function | 167, plus old 21's rule per Occurrence: a change to a Recurring Event counts for every Occurrence, an Exception's for its own; the Field Groups from one exhaustive function; a winning side for "… all over the later changes". Pure, unit-tested. | core | P1 | 175 | 167, part of 21 |
| 178 | Event Versions are rows that never change; `event` becomes a view | 145 as written. The Change Journal keeps recording. | core | P2 | 174 | 145 |
| 179 | Snapshots on one line, `begin` and `end`, and the Store Check | 146: every write makes one Snapshot with its origin (an action of the user records which), each First Sync page an Incoming Sync Snapshot as any write is (no special case). Adds the core's part of removing a Calendar (ADR 0018, "Removing a Calendar"): its Event Versions go, Snapshots left empty go. | core | P2 | 178 | 146, core part of 172 |
| 180 | Undo and redo are Snapshots in the core | First half of 147: the Undo through 177's comparison, limited to one UID, Events left out with their reason (Conflict, read-only, removed Calendar), moves with one side unwritable, the next Ctrl+Z's Snapshot and the next redo's Undo, "Undo the others" and "… all over the later changes". The core interface reports what was left out; the window only shows it in plain words. Rewrites the undo tests of `recurrence_scopes.rs`, `recurring_events.rs`, `import_run.rs` and the reusable parts of `undo_per_event.rs`. | core | P2 | 177, 179 | 147 (part) |
| 181 | The Change Journal goes | Second half of 147: its tables, the hash and its migrations, undo per Event, the listings, the Dangerous Undo and the carried ID column, from the core, the interface and the window (the Change Journal's dialog, which today's code calls "Undo History", though the glossary's Undo History is now what Ctrl+Z steps through; and the History button). Keeps `mass_change_guard::exceeded` for 200. Rewrites the server undo tests switched off in 180. `GLOSSARY.md` and `CODING_STANDARDS.md` follow. | core + window | P2 | 180 | 147 (part) |
| 182 | Ctrl+Z and redo in the window: the toasts and the dialog asking first | 158 as written, on 180's interface. | window | P2 | 181 | 158 |
| 183 | An Event's History in its details | 160 without its two boxes that need later tickets: "changed in fields FreeCal doesn't show" moves to 185, "since before <date>" to 184. | core + window | P2 | 181 | 160 |
| 184 | Snapshots dropped after 90 days | 148, with the roots and edges known so far (90 days, Undo → taken back); later tickets add their root or edge (187, 189). An Event's History reads "since before <date>". | core | P2 | 181 | 148 |
| 185 | Event Versions carry Unimplemented Fields | 149 as written; an Event's History reads "changed in fields FreeCal doesn't show". | core | P2 | 175, 183 | 149 |
| 186 | The Sender builds every resource from Event Versions; the Mirror's text goes | 150 as written; the window loses the view of the server's original text. | core + window | P2 | 176, 185 | 150 |
| 187 | The server's Current-Version Mark | 151 as written; adds the drop's root for versions with the server's mark and the trigger refusing their delete. | core | P3 | 184, 186 | 151 |
| 188 | The comparison against the server's marks; Incoming Sync Snapshots and Server Event Replacements; Propagate goes | 152 as written, through 177. Every Event changed differently on both sides is replaced for now (the Server Event Merge is 197). Ends the old Conflict detection. | core | P3 | 177, 187 | 152 |
| 189 | The core records and reports Conflicts | 133 as written, with old 21's whole-UID Conflict (a Conflict Event per single Event of the UID) and old 15's last box (overall status Conflict); the window's sync display follows the new Sync Status. | core + window | P3 | 188 | 133, last box of 15, part of 21 |
| 190 | The user resolves a Conflict | 134 as written, the restore of a whole Recurring Event's UID included. | core | P3 | 189 | 134, part of 21 |
| 191 | An Outgoing Sync sends what differs between the Current-Version Marks; `unsent_change` goes | 153 as written. | core | P3 | 188 | 153 |
| 192 | One sync activity per Calendar; read-back; the server's form takes the sent one's place | 154 as written. | core | P3 | 191 | 154 |
| 193 | A server Calendar is stored empty when added; its First Sync is a Full Fetch | 155 as written; the window drops the First Sync flag. | core + window | P3 | 192 | 155 |
| 194 | Mark Reconciliation and the untrusted base | 171 as written. | core | P3 | 189, 193 | 171 |
| 195 | Write Permissions and Guards go | 157 as written. | core | P3 | 192, 193 | 157 |
| 196 | The round-trip check at fetch | 169 as written. | core (+ a note in the details) | P3 | 185, 186 | 169 |
| 197 | The Server Event Merge | 168's core part; its toast moves to 205. | core | P4 | 189 | 168 (core) |
| 198 | A Conflict shows what each side changed | 135, with old 21's Occurrences named per side for a Recurring Event. | core | P4 | 189 | 135, part of 21 |
| 199 | Writes refused: changes wait as when offline | 166's core part: the Sync Error "writes refused", privileges read on every Poll, the mark only by the user or when added, "Stop waiting" as a Snapshot of its own origin, removing the mark asks the server. | core | P4 | 180, 191 | 166 (core) |
| 200 | The Mass-Change Guard checks every Outgoing Sync | 22's core part, including the call "Send". | core | P4 | 191 | 22 (core) |
| 201 | Status Marks, the Unsent Mark and the progress of a long sync | 161 as written. | window (+ the Signal) | P5 | 191, 193 | 161 |
| 202 | The window's actions on Sync Errors | "Send" on a paused Calendar (22), the bar for "writes refused" with "Stop waiting and mark read-only" and removing the read-only mark (166). | window | P5 | 199, 200, 201 | 22 (window), 166 (window) |
| 203 | The Conflict badge on Event chips | 136 as written. | window | P5 | 189 | 136 |
| 204 | The Conflict list in the window | 137 as written. | window | P5 | 190, 198, 203 | 137 |
| 205 | The Conflict bar and the toasts | 162, plus 168's toast after a Server Event Merge. | window | P5 | 197, 204 | 162, 168 (toast) |
| 206 | The Calendar History window | 159 as written, with new tests in place of the deleted Change Journal dialog's. | core + window | P5 | 182, 184 | 159 |
| 207 | The dialog that chooses server Calendars to add | 156 as written. | window | P5 | 193 | 156 |
| 208 | Removing a Calendar says it can't be undone | 172 as written; the core's part is in 179, 189, 191. | window (+ counts) | P5 | 11, 179, 191 | 172 |
| 209 | Google's reader and the neutral names | 163 as written. | core | P6 | 176, 185 | 163 |
| 210 | A Google Account: browser login, Available Calendars, removal | 164 as written. | core + shell + window | P6 | none | 164 |
| 211 | Add a Google Account's Calendars and pull them, read-only | 25, reworded to the marks (it still names a pointer and a Join). | core | P6 | 193, 207, 209, 210 | 25 |
| 212 | Write to Google Calendars | 26's boxes on writing: conditional writes from the Event Versions, Google's answer as the server's form, Undo, Conflicts, Mass-Change Guard and offline against the fake Google server, an Exception without its Recurring Event. | core | P6 | 192, 200, 211 | 26 (part) |
| 213 | Google login expiry; exports keep Google's fields | 26's other boxes: an expired login is a Sync Error asking to log in again; `google:` entries exported as `X-` properties and read back on import. | core + window | P6 | 09, 212 | 26 (part) |

Implementation order is the numbering. 210 has no blocker in the redesign and can go anywhere.

### Old tickets in scope

| Old | Goes to |
|---|---|
| 133, 134, 135, 136, 137 | 189, 190, 198, 203, 204 |
| 138 | dropped: wontfix already; the User's Pointer is gone (ADR 0018) |
| 142, 143, 144, 145 | 174, 175, 176, 178 |
| 146 | 179 |
| 147 | 180, 181 |
| 148, 149, 150, 151, 152, 153, 154, 155, 156, 157 | 184, 185, 186, 187, 188, 191, 192, 193, 207, 195 |
| 158, 159, 160, 161, 162 | 182, 206, 183, 201, 205 |
| 163, 164 | 209, 210 |
| 165 | dropped: wontfix already; Held during Undo is gone (ADR 0020) |
| 166 | 199, 202 |
| 167 | 177 |
| 168 | 197, 205 |
| 169, 171, 172 | 196, 194, 208 (core part in 179) |
| 139, 140, 141, 170 | stay as they are |
| 15 | stays; only its last box is open, which 189 ticks; references fixed |
| 18 | stays; references fixed (Blocked by 189, 191, 200) |
| 20 | stays wontfix |
| 21 | dropped, because it is split into 177, 189, 190 and 198 |
| 22 | 200, 202 |
| 23 | stays; references fixed (Blocked by 16, 189) and "a Conflict stops from syncing" reworded to "the server's version replaces changes" |
| 25 | 211 |
| 26 | 212, 213 (it was L) |
| 09, 11, 12, 24, 68, 71, 73, 74, 75, 126 | stay; references fixed (143 → 175, 144 → 176, 147 → 180, 148 → 184, 149 → 185, 153 → 191, 161 → 201, 163 → 209, …); 71 cites ADR 0018's former section "The pointers" |

### Switched-off tests

Spans count tickets from off to on; all are within 3. Tests not listed are either untouched, rewritten in the ticket that changes their feature, or deleted with it (see "Test sorting").

| Test | Off in | On in | Why |
|---|---|---|---|
| `core/tests/caldav_sync.rs`, the undo of a sync (`restores_*`, `of_a_journal_transaction_*`, `of_a_delete_*`, `brings_back_*`, `of_one_event_of_a_sync_*`, `undoing_the_sync_that_brought_*`) | 180 | 181 | undo becomes a Snapshot; 181 rewrites them while the journal goes |
| `core/tests/caldav_send.rs`, `undoing_*` and `an_undone_sync_*` | 180 | 181 | same |
| `core/tests/caldav_first_sync.rs`, the two `comes_back_with_the_undo…` tests | 180 | 181 | same |
| `core/tests/guests.rs`, `undoing_a_sync_skips_an_event_with_guests` | 180 | 181 | same |
| `src/lib/UndoDialog.test.ts`, the non-dangerous tests | 181 | 182 | the interface's undo types change; 182 rewrites the toasts and dialog |
| `src/lib/windowDialogs.test.ts`, the Undo flow outside the describe "the Undo History" (the Change Journal's dialog) | 181 | 182 | same |
| `src/App.events.test.ts`, the undo toast and Ctrl+Z tests | 181 | 182 | same |
| `src/App.import.test.ts`, "Undo undoes its Journal Transaction", "offers no Undo" | 181 | 182 | same |
| the core's tests of `event_history` and `App.events.test.ts`'s "has its History" | 181 | 183 | An Event's History is read from the Calendar History |
| `core/tests/caldav_send.rs`, the ETag and Conflict group (`an_edit_is_not_written_over_a_newer_*`, `an_update…`/`a_delete_without_a_stored_etag_waits`, `a_write_answered_without_an_etag_*`) | 188 | 191 (the last one 192) | the old Conflict detection goes; 191 sends from the marks, 192 reads back |
| the core's old Conflict tests, if any survive the grep in B2 | 188 | 189 or 190 | Conflicts are recorded again in 189 |

Deleted, not switched off, though their feature comes back: the Change Journal's listing (`core/tests/change_journal_list.rs`, `src/App.journal.test.ts`, `windowDialogs.test.ts` "the Undo History") goes in 181, and the Calendar History window (206) is 25 tickets later; 206 writes its tests anew from US-83 and US-155, using those files' scenarios (git keeps them).

### Test sorting

From an Explore subagent's sort by names and keywords, not test bodies; B2 confirms where it reads the code.

**Survive** (at most the rename of `EventId` to the key in 174, and helpers updated): `core/tests/` `caldav_accounts`, `caldav_discovery`, `caldav_calendars` (check `removing_an_account_removes_…_their_history` in 179), `clock`, `settings`, `sidebar_state`, `view_state`, `local_account`, `local_store`, `local_calendars`, `event_times`, `import`, `events`, `sync_timing`, most of `sync_status`, the Ignored Resource and pairing group of `caldav_sync` (harness changes where they assert the Mirror, in 186), the scheduling and UID hand-on group of `caldav_send`; unit tests in `sync/caldav/mod.rs`, `caldav.rs`, `import/`, `structured_text.rs`, `guests.rs`, `event/when.rs`, `event/recurrence_id.rs`, `sync_scheduler.rs`, `mass_change_guard.rs` (kept for 200); every test in `src-tauri/`; frontend `App.test`, `App.navigation`, `App.settings`, `App.sidebar`, `fakeCore.test`, `tauriCore.test` and `i18n` (keys updated), `Toast`, `shortcuts`, `skippedEvents`, `Sidebar.accounts`, `syncDisplay` (its Conflict state in 189), and the `src/lib` tests of `browserContextMenu`, `colourScheme`, `EventEditor`, `gridDrag`, `palette`, `repeat`, `ScopeDialog`, `Sidebar`, `week`, `when`, `windowModel`; the Scope Prompt, one-dialog, writes and import groups of `windowDialogs.test.ts`. Helpers survive: `core/tests/support/` (`yoga()` returns a key from 174), `src/test/*` (`answers.ts` and `ids.ts` follow the interface).

**Rewrite**, in the ticket named:
- the undo tests of `recurrence_scopes.rs`, `recurring_events.rs`, `import_run.rs` (the "no Journal Transaction" ones become "no Snapshot") and the reusable scenarios of `undo_per_event.rs`: 180;
- the server undo tests listed above: 181; `UndoDialog`, the undo parts of `windowDialogs`, `App.events`, `App.import`: 182; `event_history`: 183;
- `caldav_send.rs` "send results" (the patching writer's assertions, unknown fields kept): 186;
- `caldav_sync.rs` "pull" group (`a_sync_pulls_*`, `a_sync_deletes_*`, `a_sync_replaces_*`, `a_sync_token_*`, floating zones): 188;
- `offline_editing.rs`, `sync/caldav/sender.rs` tests on resource naming and refusals: 191;
- `caldav_first_sync.rs`, `sync_status.rs`'s First Sync tests: 193;
- `read_only.rs` (without the Guards' exceptions): 195;
- `caldav_sync.rs` read-only and permission group, `caldav_send.rs` `a_write_the_server_refuses_is_a_sync_error_of_the_calendar`, `caldav.rs` `a_write_refused_for_its_content_is_no_refusal_of_writes`: 199 (the privilege read once at adding survives);
- `CalendarGrid.test.ts` "an Occurrence with Unsent Changes", `unsent.test.ts`, the unsent count in `App.sync.test.ts`: 201; `App.quit.test.ts` survives as long as the interface reads as before (191's box), else 201.

**Delete**, in the ticket that removes the feature:
- 181: `core/tests/change_journal.rs`, `change_journal_list.rs`, the Dangerous Undo and per-Event mechanics of `undo_per_event.rs`, `journal/mod.rs` tests, the Dangerous Undo tests in `caldav_sync.rs`, `App.journal.test.ts`, `UndoDialog`'s and `windowDialogs`'s dangerous parts;
- 186: `sync/caldav/resource/tests.rs` (ideas carried to 176), `store/schema.rs` `an_href_is_never_both_mirrored_and_ignored` (or rewritten for the Ignored Resources);
- 191: the Unsent Changes and Sent Event Rows group of `caldav_send.rs`, the stored send-failure tests of `sender.rs`;
- 193: `caldav_sync.rs` `a_calendar_whose_first_sync_didnt_finish_*`, `sync_status.rs` `a_calendar_quit_during_its_first_sync_*`, the three "First Sync is unfinished" tests of `App.sync.test.ts`, `unsent.test.ts` "an unfinished First Sync holds the changes back";
- 195: `store/mod.rs` `only_its_guard_makes_a_write_permission`.

### Open questions for B2

- **Unsent Changes between 181 and 191** (decided by the user, 2026-11-28). If the Change Journal's write path records the Unsent Changes, the Calendar History's write path takes over recording `unsent_change` in 181, and 191 removes it with the table: an accepted exception to decision 1, *if it is little code*. B2 checks the size in the layout; if it is more than a small call from the Snapshot write, B2 moves 187, 188 and 191 before 181 instead and tells the user.
- Whether 180 and 181 fit one context window each, given the undo tests they rewrite; split 180 further (the Undo; then asking first and moves) if not.
- Whether 177 needs the Event Version's type (178) or can compare a value type of its own.

### Consistency with `GLOSSARY.md` (checked 2026-11-28)

The plan's decisions agree with the glossary's entries; the plan uses no term that is neither there nor in ADRs 0017 to 0020 ("switched-off test" is this plan's process word, not a domain term). Two points to know:

- **The First Sync between 179 and 193.** The glossary (and ADR 0018) has a First Sync take in everything as one Incoming Sync Snapshot. From 179 until 193 replaces the old First Sync, each page it writes is an Incoming Sync Snapshot of its own, as every write makes a Snapshot. That is the old First Sync recorded honestly, not glue; 179's text says so, and no test asserts one Snapshot per First Sync before 193.
- **"Undo History"** names a dialog in today's code (the Change Journal's) but, in the glossary, what Ctrl+Z steps through. 181 removes the dialog; 182's toasts and 206's window must not take the name back for a dialog.

Entries marked "until the code follows ADR …" are revised by the ticket that makes the change; each such ticket gets a box "`GLOSSARY.md` follows" for these:

| Ticket | Entries |
|---|---|
| 178 | Event Row (`event` is a view), Event Version (no longer the Change Journal's), Server Object and Corresponding Server Object (Event Versions), Recurrence ID as obtained |
| 181 | Change Journal, Journal Transaction, Journal Event Modification, Record a Journal Event Modification, Transaction Origin (a Snapshot's origin), Dangerous Undo go; Database Transaction and Incoming Sync lose the Journal Transaction; First Sync's "the Change Journal doesn't record it" |
| 185 | Event Version (Unimplemented Fields stored) |
| 186 | Mirror goes; CalDAV Server Object (no text), Known Server Resources, Ignored Resource ("kept in the CalDAV table"), Database Transaction (the Mirror's writes), Sync Engine ("the only code that writes the Mirror") |
| 188 | Propagate a Server Object goes |
| 191 | Unsent Change (derived, no list), Sent Event Rows goes, Sender, Outgoing Sync ("changes the Mirror"), Combined Event Key ("what an Unsent Change is about") |
| 195 | Write Permission, Guard, Unguarded Write go |
| 209 | Google Server Object (no JSON) |
