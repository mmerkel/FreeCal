# 182: The Change Journal goes

**What to build:** The last part of old ticket 147 (`git show design/snapshot-line:.scratch/freecal-v1/issues/147-undo-and-redo-are-snapshots.md`): with Undo a Snapshot (180, 181), the Change Journal and everything built on it come out of the core, the core interface and the window. Mostly deletions. Between this ticket and 183 the window has no undo, and between this ticket and 184 no Event's History (decision 1 of the redesign plan).

- **The core:** `journal/` whole (`mod.rs`, `undo.rs`, `list.rs`); `journal_transaction`, `journal_event_modification` and their SQL in `store/sql/journal.rs`; the hash of an Event Version; undo per Event; the listings and `event_history`; the Dangerous Undo; the 90-day removal of Journal Transactions (dropping Snapshots is 185); 178's carried ID column. The running of a set of writes as one step belongs to `history/`: one such step is one Snapshot through the `SnapshotWriter`. The Snapshot's origin type replaces `TransactionOrigin`. `mass_change_guard::exceeded` stays, unused, for 201.
- **Unsent Changes until 192:** the `SnapshotWriter` adds the Combined Event Key of each Event Version begun or ended by a Snapshot whose origin isn't an Incoming Sync to `unsent_change`, as `record_journal_event_modification` did. The Sent Event Rows' hooks (`sending`, `sent`, `not_sent`) move to `sync/caldav/sender.rs` and compare the Event Versions' row IDs in place of the hash, since a row never changes. Moved, not rewritten; 192 deletes both with the table.
- **The core interface:** `undo` with its targets, `change_journal`, `journal_transaction_events`, `event_history`, `undo_history`, and the Journal Transaction in the result of every change (`Changed<T>`); their commands in `src-tauri/src/main.rs`. The generated types are rewritten.
- **The window:** the Change Journal's dialog (today's code calls it "Undo History"; the name now belongs to what Ctrl+Z steps through, so nothing takes it back for a dialog) and its menu entry; the History button in an Event's details; the Undo button on toasts. Ctrl+Z and redo do nothing until 183. Only what the type change forces.
- **Docs:** `GLOSSARY.md` loses Change Journal, Journal Transaction, Journal Event Modification, Record a Journal Event Modification, Transaction Origin (a Snapshot's origin) and Dangerous Undo; Database Transaction and Incoming Sync lose the Journal Transaction; First Sync loses "the Change Journal doesn't record it". `CODING_STANDARDS.md` follows where it names the Change Journal.

Spec: US-81; "Calendar History". ADRs 0018, 0020. Layout: "Today's code, in brief" (Change Journal, Undo per Event, Listings, Unsent Changes); "By phase", P2, 182; the plan's "Answers from B2".

**Blocked by:** 181

**Status:** ready-for-agent

**Effort:** M

**Deleted tests:** `core/tests/change_journal.rs`, `change_journal_list.rs`, the Dangerous Undo and per-Event tests of `undo_per_event.rs`, the tests in `journal/mod.rs`, the Dangerous Undo tests in `caldav_sync.rs`; `src/App.journal.test.ts`, the dangerous parts of `src/lib/UndoDialog.test.ts` and `windowDialogs.test.ts`, and `windowDialogs.test.ts`'s describe "the Undo History" (the Calendar History window, 207, writes its tests anew).

**Switched-off tests:**
- until ticket 183 (the interface's undo types change): `UndoDialog.test.ts`'s other tests; `windowDialogs.test.ts`'s Undo flow outside "the Undo History"; `App.events.test.ts`'s undo toast and Ctrl+Z tests; `App.import.test.ts`'s "Undo undoes its Journal Transaction" and "offers no Undo";
- until ticket 184 (An Event's History is read from the Calendar History): the core's tests of `event_history` and `App.events.test.ts`'s "has its History".

- [ ] `journal/`, its tables, the hash, undo per Event, the listings, the Dangerous Undo and the carried ID column are gone from code and schema
- [ ] The core interface and the window name no Journal Transaction; the generated types are rewritten; the dialog, its menu entry, the History button and the toasts' Undo button are gone
- [ ] Unsent Changes are written by the `SnapshotWriter` and the Sent Event Rows compare row IDs; the send and offline tests pass unchanged
- [ ] The tests above deleted or switched off with their notes; every other test passes
- [ ] `GLOSSARY.md` and `CODING_STANDARDS.md` follow
