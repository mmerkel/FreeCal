# Triage of the open tickets against ADRs 0017 to 0020 (working file, delete when Step 2 is done)

Branch `explore/unknown-fields-and-change-journal`. Design: `docs/adr/0017` to `0020`, `GLOSSARY.md`, `spec.md` (commit e9b8cc8). `TODO.md` stays out of commits. Commit only when the user says so. Stay below 130k tokens: grep first, short ranges.

State on 2026-11-16: Step 1 (triage) presented. **Agreed:** 19 is closed as `wontfix` with a comment naming its successors; ticket 71's created Calendars take the import as their first Snapshot (see "Open"). **Also agreed (2026-11-16, later):** the whole table; no ticket above M, so the L and XL tickets are split as listed below, each part a ticket of its own; the cap goes into `docs/agents/issue-tracker.md`, and `.claude/skills/implement/SKILL.md` gets three lines (the Explore map and the commit before the first layer go into a ticket comment; past about 100k of context, commit at the next green point, write what is left into the ticket comment and stop; `/code-review` runs in a fresh session from the recorded commit); 138 stays a ticket of its own, rewritten as S (not `wontfix`, not folded into 134); for ticket 71, Ctrl+Z on the import takes it back in the Calendars that existed and leaves the created ones, and FreeCal says so, with a line each in ADRs 0018 and 0020, the spec's Calendar History and `GLOSSARY.md`. Everything under "Open" and every "proposed" below is therefore decided.

**Batch 1 done (2026-11-16, committed in bb1e088):** tickets 142 to 154 written in `issues/`. **All numbers in this file are the new ones** (see "Numbering").

**Batch 2 done (2026-11-16, committed after bb1e088):** tickets 155 to 162 written; the M cap is in `docs/agents/issue-tracker.md`; `.claude/skills/implement/SKILL.md` has its three lines; ticket 71's rule is in ADRs 0018 and 0020, the spec's Calendar History and `GLOSSARY.md` (Snapshot).

**Batch 3 done (2026-11-16, committed in 78ca9c2):** tickets 133 to 138, 21 and 22 rewritten (133, 134 and 138 under new file names); 19 and 129 closed as `wontfix` with their successors. Its two open points are decided (see "What batch 3 chose"). 

**Batch 4 done (2026-11-16, not committed yet):** the rewords of 09, 11, 12, 15, 18, 23, 24, 25, 26, 68, 71, 73, 74, 75, 126, 139, 140 and 141; 25 split into 163, 164 and 25 (file renamed); `tickets-pdf` rebuilt. Step 2 is done once the user has gone through "What batch 4 chose"; then this file is deleted.

## Order in which the old model comes out

Every step leaves the tests green. No migration code: no real Local Store exists.

0. 142, 143, 144: nothing comes out. Row IDs leave the core interface and the window; the Unimplemented Fields tree, CalDAV reader and writer are pure code.
1. 145 to 148 (the journal goes in 147): the `event` table (becomes a view), `journal_transaction`, `journal_event_modification`, the hash, undo per Event (`core/src/journal/undo.rs`, `list.rs`), Dangerous Undo, the core's Undo History. Glue until step 3: `unsent_change` is written on every move of the User's Pointer; an Incoming Sync moves the User's Pointer directly.
2. 149, 150: the Mirror's text, the patching writer and the Sender's comparison of texts (150).
3. 151 to 154: Propagate (152), `unsent_change` and Sent Event Rows (153). The glue goes in 153. A Conflict is held from 152 on (Server's Pointer off the Reference Snapshot), so every Conflict ticket waits on 152.
4. 155, 156: the stored unfinished First Sync (tickets 80, 111), `SyncStatus::Syncing { first_sync }`.
5. 157: Write Permissions, Guards, Unguarded Writes (code, `CODING_STANDARDS.md`, `GLOSSARY.md`).

## Numbering

Written (batch 1), in build order:

| # | Title | Effort | Blocked by | Status |
|---|---|---|---|---|
| 142 | An Event is identified by its Combined Event Key in the core interface and the window | M | none | ready-for-agent |
| 143 | Unimplemented Fields: the tree, its canonical form and the CalDAV reader | M | none | ready-for-agent |
| 144 | The CalDAV writer builds a resource from managed fields and Unimplemented Fields | M | 143 | ready-for-agent |
| 145 | Event Versions are rows that never change; `event` becomes a view | M | 142 | ready-for-agent |
| 146 | Snapshots, the User's Pointer and the Store Check | M | 145 | ready-for-agent |
| 147 | Undo and redo move the User's Pointer; the Change Journal goes | M | 146 | ready-for-agent |
| 148 | Undo for one Calendar, and Snapshots dropped after 90 days | M | 147 | ready-for-agent |
| 149 | Event Versions carry Unimplemented Fields | M | 143, 147 | ready-for-agent |
| 150 | The Sender builds every resource from Event Versions; the Mirror's text goes | M | 144, 149 | ready-for-agent |
| 151 | The Server's Pointer: an Incoming Sync records the server's state | M | 148, 150 | ready-for-agent |
| 152 | The Reference Snapshot and the comparison of the two sides; Propagate goes | M | 151 | ready-for-agent |
| 153 | An Outgoing Sync sends what differs between the marks; `unsent_change` goes | M | 152 | ready-for-agent |
| 154 | One sync activity per Calendar; read-back; the server's form | M | 153 | ready-for-agent |

Written (batch 2), all `ready-for-agent`. "Was" is the number in the triage as first presented:

| # | Was | Title | Effort | Blocked by |
|---|---|---|---|---|
| 155 | 147 | A Calendar enters the Local Store with its first Snapshot: First Sync in memory, "First Sync progress" | M | 154 |
| 156 | 148 | The dialog that adds server Calendars and shows their First Sync (modal) | S | 155 |
| 157 | 149 | Write Permissions and Guards go; module visibility replaces them | S | 154, 155 |
| 158 | 150 | Ctrl+Z and redo in the window: question before a sync, at a merge, at a redo fork; toasts' wording | M | 151 (checked: 152 to 154 don't change an undo's choices) |
| 159 | 151 | A Calendar's History in the sidebar menu, with "Go back to before this" | M | 148 |
| 160 | 152 | An Event's History in its details | S | 149 |
| 161 | 153 | Status Marks in the sidebar, the Unsent Mark, progress of a long sync | M | 153 |
| 162 | 154 | The Conflict bar, and the dialog when a Calendar stops Tracking | M | 133, 137 |

Written (batch 4), both `ready-for-agent`:

| # | Title | Effort | Blocked by |
|---|---|---|---|
| 163 | Google's reader: an event resource becomes managed fields and Unimplemented Fields; neutral names are assigned | M | 144, 149 |
| 164 | A Google Account: browser login, its Available Calendars, removal | M | none |

25 keeps its number for the third part (the Calendars and their sync, M, blocked by 154, 155, 156, 163, 164). 134 stays M with 138 a ticket of its own (S).

### What batch 1 changed against the split as listed, and what later batches must pick up

- The end of the redo path is in 147, not 148 (a change after an undo would otherwise fork the user's side).
- The Sender's comparison of texts goes in 150 with the text, not in 153.
- The server's form of a write is put in place in 154 (with the read-back), not 153.
- Undo and redo at a merge are in the core from 151; the window's questions are 158. Until 158 the window goes back through the user's side.
- 147 already removes: the Undo button on toasts, the Change Journal dialog and menu entry, the History button in an Event's details. 158, 159 and 160 bring back what the spec asks for.
- 150 removes the showing of the server's original text (ticket 81's).
- 152 holds Conflicts but shows nothing until 133. Running the comparison after each move of the User's Pointer is left to 138.
- 153 keeps the reason a send failed in a table of its own (not in an ADR; listed in the ticket as chosen there).
- 148: Ctrl+Z on a Snapshot spanning Calendars that one Calendar has left, or whose first Snapshot it is, undoes it in the others (covers ticket 71's agreed rule).
- 143: "the first Exception" for calendar-level entries is the one with the earliest Occurrence.

### Decided after batch 1 (2026-11-16), in ADR 0017 and the tickets

- `SEQUENCE` is an entry, written back unchanged until ticket 126 (comment added there).
- A new Exception takes the Recurring Event's tree.
- Prefixes name the format: everything from iCalendar text is `ical:`; `caldav:` is reserved, assigned to nothing. Each Provider names the prefixes it holds (Local and ICS feed `ical:`; CalDAV `ical:`, `caldav:`; Google `google:`); a move or an import keeps the neutral entries and those the target holds. Tickets 24, 25, 26, 09 and 71 take this wording in their rewords.
- 144 and 149 are `ready-for-agent`.

### What batch 2 chose, and what later batches must pick up

- 155: until 156 the window waits for the whole add as today. Ticket 111's rules for Calendars a sync didn't reach have no First Sync case left. Reword of 15: the first-sync marker is gone with 155. Reword of 25: Google's First Sync uses 155's operation and Signal.
- 158: the core reports the number of Events per step and per way (new there). Read-only Calendars are left out of Ctrl+Z in 158 if 147 didn't. Wording of the merge question and "Nothing to undo" chosen in the ticket, for review.
- 159: at a merge the list shows both sides' Snapshots by time; "before" a server's-side Snapshot is its parent on that side (chosen in the ticket, for review). Decided (2026-11-16): a "Go back" is taken back both by Ctrl+Shift+Z (noted in 158) and by "Go forward to this" on the Snapshots beyond the User's Pointer, listed in the dialog; the spec's Calendar History has the sentence.
- Redo across Calendars (decided 2026-11-16, ADR 0020 "Which Calendar", spec, tickets 147, 158, 159): redo runs when exactly one Snapshot is next beyond all User's Pointers, otherwise asks; no order from times, IDs or undo numbers. Numbering the undos is in ADR 0020's "Left open".
- 160: the history follows the user's side; a "deleted" line (chosen in the ticket, for review).
- 161: the Conflict count in "Sync status changed" is added there and is 0 until 133 fills it; the Conflict mark's click waits for 137; "Send" on a paused Calendar's Sync Error mark waits for 22. So the rewrites of 133, 137 and 22 name these.
- 162: blocked by 133 and 137 as batch 3 rewrites them. "Stopped Tracking" is seen by the window (count from 0 to more), nothing stored.
- 156: decided (2026-11-16): the dialog is modal and locks the window (ADR 0018 and the spec changed); 156 is S; no read of the running First Syncs is needed.

### What batch 3 chose, and what later batches must pick up

- **Decided (2026-11-16, agreed):** 134's call ends all of a Calendar's Conflicts at once, with a choice for each, as one merge Snapshot; 137 keeps the choices in the window until every Conflict of the Calendar has one, then "Apply". Reason: no table holds a Conflict and the Reference Snapshot is per Calendar, so "Your version" for one Conflict can't be kept while another is open; ADR 0019 ("the user's choices", "picks one side whole") and the prototype read the same way. Both are `ready-for-agent`, and the spec's core interface line says "end a Calendar's Conflicts together by keeping, for each, …".
- The spec's Mass-Change Guard says "more than 10 Events or more than 5%"; the code (`core/src/mass_change_guard.rs`, ticket 72) has "and". Decided (2026-11-16): "and", so that small Calendars aren't bothered; the spec (US-85, Sync engine) changed, 22 keeps the code's rule.
- 133 has the list (moved from 134), S, blocked by 152. A listed Conflict is one Calendar and UID; no "when raised". "Conflicts changed" is sent after every Database Transaction that moves a pointer or the Reference Snapshot of a Calendar that isn't Tracking before or after. 133 ticks ticket 15's overall-status box, so the reword of 15 finds it ticked or names 133. 133 adds 161's Conflict count field itself if 161 isn't built.
- 135 and 136 are blocked by 133 only. 135: an empty Last Agreed Version (both sides created the UID) replaces "no Last Agreed Version".
- 137 isn't blocked by 138 and doesn't open from the toolbar's status indicator. 138 drops the "Conflict resolved" toast and the result that fed it.
- 21: M. An excluded date is a change to the Recurring Event; the choice takes the chosen side's whole UID.
- 22: creations aren't counted; the share is of the server's side; "Send" is a core call, refused when not paused; the restore's origin is read once ticket 11 records it (so the reword of 11 names the origin). 18's reword: a paused Calendar is one of the quit flow's reasons.
- 19 and 129 are `wontfix`. 129's Effort line is gone (split).

### What batch 4 chose, for the user to go through

- **25 in three parts, not two:** 163 the reader and the neutral names, 164 the login and the Available Calendars, 25 the Calendars and their sync. The login and the sync together were above M. 25 keeps its number, so 26 and 27 stay blocked by it; its blocker 15 is dropped.
- **Still above M, written before the cap and not on the list of splits:** 12 (L), 26 (L), 75 (L). Each has a proposed split in its comment of 2026-11-16. 126 (XL) and 141 (L) are grillings that end in a split.
- **Blockers beyond the table:** 11 also by 144 and 149 (a Backup must hold the Unimplemented Fields, or its restore strips them from the server); 12 also by 144; 15 also by 133 (133 ticks its only open box; nothing else is left in 15); 18 also by 133 (the Conflict reason); 25 also by 156, 163, 164; 71 by 148 (was 69, done).
- **11, chosen for review:** "changed since the last Backup" is the User's Pointer off the Snapshot it was on at that Backup; no restore into a Read-only Calendar; the restore's confirmation names the Events added, changed and removed. Whichever of 11 and 22 comes second adds the restore's origin to the guard.
- **12:** reminders become a column; the user can't change them in v1, so the comparison of the two sides takes the server's and no Conflict arises from them (141 decides for editing). Snoozes and dismissals are outside the Event Version.
- **18:** the quit dialog's reasons in order: Account Offline or Sync Error, paused, Conflict, the Calendar's other Sync Error, not sent yet. The tray Quit box follows the spec unless 139's review decides otherwise.
- **23:** tray lines and notifications are per Calendar with Conflicts, as the spec has them; the comment on resources with a UID another holds is outdated (Ignored Resources go to the Log).
- **163, chosen for review:** a cancelled Google Exception is a cancelled Occurrence, any other cancelled resource a delete; the list of neutral names is decided there and added to ADR 0017, with the CalDAV reader and writer and a migration following.
- **25, chosen for review:** a Google resource whose UID and Occurrence a kept one holds is ignored and logged, as CalDAV's Ambiguous UID; no Google Calendar can be made writable until 26; the First Sync's progress counts Events loaded.
- **26:** the Q39 question (an Exception without its Recurring Event) is in the body as "to decide here".
- **Spec:** "Import/export" said that undoing a `.zip` import removes the Calendars it created if empty; it now has ticket 71's agreed rule.

## Open tickets

| # | Verdict | What changes |
|---|---|---|
| 09 | reword | Export writes Unimplemented Fields through 144's writer. Add blockers 144, 149. |
| 11 | reword | A restore is one Snapshot; a Calendar with a Conflict can be restored and is compared again. Add blocker 147. |
| 12 | reword | Alarms arrive as Unimplemented Fields; managing them moves them to a column (ADR 0017). Add blocker 149. |
| 15 | reword | Open box (overall status Conflict) reads Tracking. Signal: has unsent changes, Conflict count, progress; first-sync marker goes with 155. Blocked by 153. |
| 18 | reword | Only the two quit-flow boxes remain; reasons from derived state. Blockers 153, 22. |
| 19 | drop (`wontfix`, agreed) | Superseded by ADR 0020; successors 147, 148, 158, 159, 160. |
| 21 | rewrite | Comparison per Occurrence for Recurring Events and the merge of changes to different Occurrences, inside 152's comparison. Blocked by 152, 134, 135. |
| 22 | rewrite | Checks every Outgoing Sync on the difference between the marks; imports and restores the server hasn't reached aren't counted; "Send"; pause ends under the thresholds. Blocked by 153. Check `core/src/mass_change_guard.rs` first. |
| 23 | reword | "Problems changed" → "Conflicts changed"; comment of 2026-10-25 outdated. Blocked by 16, 133. |
| 24 | reword | One Snapshot spanning both Calendars; neutral entries and the target Provider's carried. Blocked by 149, 153. |
| 25 | reword | First Sync through 155, 156; an Incoming Sync is a Snapshot; Google's reader fills Unimplemented Fields and assigns neutral names. Add blockers 154, 155. |
| 26 | reword | Google's Server Object table holds where and ETag only; a write's answer is the server's form; Mirror comments outdated; uses 154's function for the server's form. Blocked by 25, 154, 09. |
| 61 | keep | needs-triage; its "Mirror" is FullCalendar's. |
| 68 | reword | "dashed outline" → Unsent Mark. |
| 71 | reword | One Snapshot spanning its Calendars; a Calendar the import creates takes it as its first Snapshot (agreed, see "Open"). |
| 73, 74, 75 | reword | Hash clause goes; until built the fields sit in the Unimplemented Fields; the ticket moves them to columns. 75: comment of 2026-11-15 becomes the body. Add blocker 149. |
| 126, 140, 141 | reword | Questions name the Calendar History and Unimplemented Fields. 140: status is a candidate neutral name. |
| 129 | split | ADR box done (0017 to 0020); the three cases are in 149 and 150 (done in batch 1; 129 itself still to close). |
| 133 | rewrite | Finding and holding is 152. Left: the core reports Conflicts (which Events, three versions read from the pointers and the Reference Snapshot, count in the Sync Status, "Conflicts changed", overall status). No `conflict` tables. S. Blocked by 152. |
| 134 | rewrite | Ending a Conflict by the user's choices: one merge Snapshot through the comparison's second step, undoable. Stale-choice and Server read-only rules stay. Blocked by 133. |
| 135 | reword | Compared against the Reference Snapshot; the "no Last Agreed Version" case goes. |
| 136 | reword | The badge is the chip's only mark. Blocked by 133. |
| 137 | rewrite | Conflict list for all Calendars or one, opened from the bar (162) and the Status Mark; no Undo toast, no "can't be undone" note. Blocked by 134, 135, 136. |
| 138 | rewrite as S (agreed) | The comparison runs after every move of the User's Pointer while the Server's Pointer is off the Reference Snapshot. |
| 139 | reword | One term: unsent changes, as the user sees them. |
| 27, 28, 30, 38, 43, 44, 45, 49, 50, 51, 52, 63, 64, 67, 89 | keep | 63 runs after 157. |

## Open

- (Done in batch 2.) Ticket 71, agreed in principle: a Calendar an import creates enters the Local Store with the import's Snapshot as its first, as a First Sync's does. ADRs 0018 ("an empty one for a Local Calendar") and 0020 ("Undo can't go before a Calendar's first Snapshot"), the spec's Calendar History and `GLOSSARY.md` need a line for it. To confirm: Ctrl+Z on that import takes it back in the Calendars that existed and leaves the created ones as they are, and FreeCal says so.
- (Done in batch 2.) Whether tickets may be above M at all (see the proposed split), and the matching lines in `docs/agents/issue-tracker.md` and `.claude/skills/implement/SKILL.md`.

## Step 2 batches

1. Done: 142 to 154. 2. Done: 155 to 162; also the M cap in `docs/agents/issue-tracker.md`, the three lines in `.claude/skills/implement/SKILL.md`, and ticket 71's line in ADRs 0018 and 0020, the spec and `GLOSSARY.md`. 3. Done: Conflict tickets 133 to 138, 21, 22, 19, 129. 4. Done: the rewords, 25's split, `tickets-pdf`.
