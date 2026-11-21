# 147: Undo and redo move the User's Pointer; the Change Journal goes

**What to build:** The third part of the Calendar History: Ctrl+Z becomes a move of the User's Pointer (ADR 0020), and the Change Journal, the hash of an Event Version and undo per Event come out. This is step 1 of the order in which the old model goes.

- **Undo** takes the newest Snapshot that a User's Pointer is on, in any Calendar, and moves that Calendar's User's Pointer to its parent. A Snapshot spanning several Calendars is undone whole, in one Database Transaction. Nothing else is written, and the undo is not itself recorded.
- **Redo** moves the pointer forward again along the line it came back on. Which Calendar: the core reports the Snapshots that are next beyond a User's Pointer, one per Calendar with a redo path, a Snapshot spanning several counted once. With exactly one, redo takes it; with several, the caller names one (ADR 0020, "Which Calendar"). No order among them is derived from times or row IDs. Until ticket 158 asks the user, the window redoes nothing when there are several, and a toast says so.
- **The redo path ends** when a User's Pointer leaves its line: by a change the user makes. An Incoming Sync doesn't end it: from ticket 152 on, the server's changes wait while it exists (Held during Undo, ADR 0019); "Continue" ends it, ticket 165. The Snapshots that are then neither an ancestor of a pointer nor beyond the User's Pointer are dropped in the same Database Transaction, with the Event Versions only they referred to. It is the Store Check's query of ticket 146, now with something to find. (Forks at a merge come with ticket 151; undo for one Calendar with ticket 148.)
- **The first Snapshot:** undo can't go before a Calendar's first Snapshot.
- **Asking first:** the core says what the next undo and the next redo would take back or bring back: the Snapshot's origin, its time and its Calendars. The window asks before undoing a Snapshot whose origin is an Incoming Sync, with the confirmation it has today for an undo that reverts a change from a server, and not otherwise. The Dangerous Undo's other two reasons (undoing later changes, the Mass-Change Guard's thresholds) go; the Mass-Change Guard checks the Outgoing Sync that follows, which is ticket 22.
- **Sending an undo.** Glue until ticket 153, named: the function that moves the User's Pointer records an Unsent Change for every Combined Event Key whose user's Event Version changes with the move, in writable server Calendars, so an undo and a redo are sent like any change. An Incoming Sync's own move of the pointer records none. Ticket 153 removes this with `unsent_change`.
- **The Sent Event Rows** name each Event Version by its row ID instead of its hash, as a row never changes.
- **What comes out of the core:** `journal_transaction`, `journal_event_modification` and their SQL (`core/src/store/sql/journal.rs`); the hash and its format; undo per Event and its search for changes in between (`core/src/journal/undo.rs`); the Change Journal's listings (`core/src/journal/list.rs`); the Undo History read from the Change Journal; the Dangerous Undo; the Transaction Origin, which the Snapshot's origin replaces; the 90-day removal of Journal Transactions (dropping Snapshots is ticket 148); ticket 145's carried ID column. What is left of `core/src/journal/mod.rs`, the running of a set of writes as one step, belongs to the Calendar History's module, and one such step makes one Snapshot.
- **What comes out of the core interface and the window:** `undo` with its targets, `change_journal`, `journal_transaction_events`, `event_history`, `undo_history`, and the Journal Transaction in the result of every change (`Changed<T>`); the Change Journal dialog (`src/lib/ChangeJournalDialog.svelte`) and its menu entry; the History button in an Event's details; the Undo button on toasts, whose target was a Journal Transaction. The Calendar's History and the Event's History return with their own tickets.
- **Write Permissions** stay for now, under the Calendar History's module in place of the Change Journal.
- **Tests:** `core/tests/change_journal.rs`, `change_journal_list.rs` and `undo_per_event.rs` are replaced by tests of the Calendar History through the core interface: undo and redo of each kind of change, of a move and of an import into several Calendars, across a restart, down to the first Snapshot, and the end of the redo path. The tests of undo reaching the server in `core/tests/caldav_send.rs` are kept and rewritten to Ctrl+Z.
- **Docs:** `GLOSSARY.md` loses or revises the entries Change Journal, Journal Transaction, Journal Event Modification, Record a Journal Event Modification, Transaction Origin, Undo History, Dangerous Undo, Event Row, First Sync and Local Store, and the lines in other entries that name them. `CODING_STANDARDS.md` follows where it names the Change Journal.

Spec: US-81, US-146, US-150; "Calendar History". ADRs 0018, 0020.

**Blocked by:** 146

**Status:** ready-for-agent

**Effort:** M

- [ ] Undo and redo are moves of the User's Pointer; an undo writes no Event Version and no Snapshot
- [ ] A Snapshot spanning Calendars is undone and redone whole
- [ ] A change by the user ends the redo path and drops its Snapshots; the Store Check passes after each
- [ ] Undo stops at a Calendar's first Snapshot
- [ ] The core reports the origin of the next undo and redo; the window asks before undoing an Incoming Sync and not otherwise
- [ ] An undo and a redo in a server Calendar reach the server (tests against Radicale)
- [ ] The Change Journal's tables, the hash, undo per Event, the listings, the Dangerous Undo and the carried ID column are gone from the code and the schema
- [ ] The core interface and the window no longer name a Journal Transaction; the generated types are rewritten
- [ ] `GLOSSARY.md` and `CODING_STANDARDS.md` follow

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Third of four parts (see ticket 145). Against the split as first listed, the end of the redo path is here and not in ticket 148: without it a change after an undo would fork the user's side.
- The window's questions at a merge and at a redo fork, and the toasts' final wording, are the ticket for Ctrl+Z in the window (to be written; first listed as 150).
- 2026-11-21: An Incoming Sync no longer ends the redo path; the server's changes wait instead (Held during Undo, tickets 152 and 165; grilling, ADR 0019 and 0020).
