# 147: Undo and redo are Snapshots; the Change Journal goes

**What to build:** The third part of the Calendar History: Ctrl+Z makes an Undo, a Snapshot that takes back one earlier Snapshot (ADR 0020), and the Change Journal, the hash of an Event Version and undo per Event come out. This is step 1 of the order in which the old model goes.

- **An Undo** is a Snapshot whose origin names the Snapshot it takes back, written once. Its content is ticket 167's comparison per Occurrence, with what the taken-back Snapshot left as what both are compared with, the current versions as one side and the versions it ended as the other; where nothing has changed since, the Event goes back to the version before, as a copy (ADR 0018). It is made in one Database Transaction for all the Calendars, through the one place that makes a change's Snapshot, so it is sent like any change.
- **Refused whole,** writing nothing, when the comparison finds a Conflict in any Calendar, when it would change nothing, when the Snapshot is a First Sync's, and when it spans a Calendar marked read-only. The refusal says which: for a Conflict, the Occurrences and the Snapshot that changed each later (its origin and, for the user's own, its description), so that the window can name it. With "both" asked for, the taken-back Snapshot's parent wins in every Occurrence changed on both sides ("Undo both changes").
- **Ctrl+Z's target:** the core says which Snapshot the next Ctrl+Z takes back: the newest of the user's own Snapshots in effect, across all Calendars, by when each was made. The user's own: an action of the user, an import, a Backup restore and "Stop waiting" (from ticket 166). Passed over: Server Intakes, the First Sync's among them, Server Event Replacements (from ticket 152), and Undos. In effect: no Undo in effect takes it back. Calendars marked read-only are left out.
- **Redo** takes back the newest Undo in effect, by the same function, however that Undo was made. The core says which one; there is none once the user has made a change of their own after it. Syncs don't end it. Nothing is dropped when it ends.
- **The Calendar History is only added to.** Nothing is written beside the Undo's Snapshot; the redo path, its end and the Snapshots dropped with it don't exist.
- **The Sent Event Rows** name each Event Version by its row ID instead of its hash, as a row never changes.
- **What comes out of the core:** `journal_transaction`, `journal_event_modification` and their SQL (`core/src/store/sql/journal.rs`); the hash and its format; undo per Event and its search for changes in between (`core/src/journal/undo.rs`); the Change Journal's listings (`core/src/journal/list.rs`); the Undo History read from the Change Journal; the Dangerous Undo; the Transaction Origin, which the Snapshot's origin replaces; the 90-day removal of Journal Transactions (dropping Snapshots is ticket 148); ticket 145's carried ID column. What is left of `core/src/journal/mod.rs`, the running of a set of writes as one step, belongs to the Calendar History's module, and one such step makes one Snapshot.
- **What comes out of the core interface and the window:** `undo` with its targets, `change_journal`, `journal_transaction_events`, `event_history`, `undo_history`, and the Journal Transaction in the result of every change (`Changed<T>`); the Change Journal dialog (`src/lib/ChangeJournalDialog.svelte`) and its menu entry; the History button in an Event's details; the Undo button on toasts, whose target was a Journal Transaction. Ctrl+Z and Ctrl+Shift+Z call the new undo and redo, and a refusal shows as a toast with the core's reason in plain words; ticket 158 words the toasts and adds their buttons. The Calendar History window and the Event's History return with their own tickets.
- **Write Permissions** stay for now, under the Calendar History's module in place of the Change Journal.
- **The Store Check** gains: an Undo takes back an older Snapshot.
- **Tests:** `core/tests/change_journal.rs`, `change_journal_list.rs` and `undo_per_event.rs` are replaced by tests of the Calendar History through the core interface: undo and redo of each kind of change, of a move and of an import into several Calendars, across a restart; an undo refused by a later change of the user, with its name, and "both"; redo after a change of the user offered no more; a First Sync's Snapshot never taken back. The tests of undo reaching the server in `core/tests/caldav_send.rs` are kept and rewritten to Ctrl+Z. The prototype's walkthroughs 1, 2, 3, 6 and 9 (`core/prototypes/calendar-history.PROTOTYPE.html`) are cases to cover.
- **Docs:** `GLOSSARY.md` loses or revises the entries Change Journal, Journal Transaction, Journal Event Modification, Record a Journal Event Modification, Transaction Origin, Dangerous Undo, Event Row, First Sync and Local Store, and the lines in other entries that name them. `CODING_STANDARDS.md` follows where it names the Change Journal.

Spec: US-81, US-146, US-150; "Calendar History". ADRs 0018, 0020.

**Blocked by:** 146, 167

**Status:** ready-for-agent

**Effort:** M

- [ ] An Undo is a Snapshot naming the one it takes back, made by ticket 167's comparison; a Snapshot spanning Calendars is taken back whole, in one Database Transaction
- [ ] An Undo is refused whole, writing nothing, in each case listed, and says why; "both" undoes over the later change
- [ ] The core names the next Ctrl+Z's Snapshot and the next redo's Undo by the rules above
- [ ] Redo takes back the newest Undo in effect until the user's next change; the Store Check passes after each step
- [ ] An undo and a redo in a server Calendar reach the server (tests against Radicale)
- [ ] The Change Journal's tables, the hash, undo per Event, the listings, the Dangerous Undo and the carried ID column are gone from the code and the schema
- [ ] The core interface and the window no longer name a Journal Transaction; the generated types are rewritten
- [ ] `GLOSSARY.md` and `CODING_STANDARDS.md` follow

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Third of four parts (see ticket 145). Against the split as first listed, the end of the redo path is here and not in ticket 148: without it a change after an undo would fork the user's side.
- 2026-11-16: The window's questions at a merge and at a redo fork, and the toasts' final wording, are the ticket for Ctrl+Z in the window (to be written; first listed as 150).
- 2026-11-21: An Incoming Sync no longer ends the redo path; the server's changes wait instead (Held during Undo, tickets 152 and 165; grilling, ADR 0019 and 0020).
- 2026-11-21: Rewritten from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten): undo no longer moves the User's Pointer back but makes an Undo, so the redo path, Held during Undo and asking first all go. The comments above are history. Blocked by the new ticket 167 for the comparison.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): one line of Snapshots without pointers, Undos write copies, merges and Joins are gone; Ctrl+Z passes over Server Intakes and Server Event Replacements.
