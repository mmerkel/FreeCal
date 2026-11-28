# Redesign layout: the backend for ADRs 0017 to 0020

The target layout of `core/` (and what `src-tauri/` and the window follow) for tickets 173 to 214 of `redesign-plan.md`: modules and files, main types and tables, the seams where `/tdd` tests sit, and the tests through the core interface that survive the whole redesign. Written by B2 (2026-11-28) from a map of the code at commit `9090586`. A target that tickets may amend, not a contract: a ticket that departs from it edits the section here and adds `(changed in ticket NN: <why>)` (see `.claude/skills/implement-redesign/SKILL.md`).

Pointers are `file:line` at `9090586`, relative to `core/src/` unless they say otherwise. They locate. Read only the lines you change. The names under "New terms" are approved; a ticket that needs another proposes it as CLAUDE.md says.

## New terms

Approved by the user on 2026-11-28, with their renames: Event Version Content, Event Comparison, `Rescheduled` and `MovedBetweenCalendars`. Not yet in `GLOSSARY.md`.

- **Event Version Content**: what one Event Version holds that the comparison reads: the managed fields and the Unimplemented Fields, without its row ID, Calendar or Snapshots. Two Event Versions are equal when their Event Version Contents' canonical forms are (ADR 0018, "Event Versions"). Type `EventVersionContent`.
- **Event Comparison**: the comparison per Occurrence, three ways and by Field Group (ADR 0019, "Whether the server changed"; ADR 0020, "Where both changed"), run over every single Event of one UID at once, with the outcome for that UID. Incoming Sync and Undo both call it. Module `comparison`, function `compare_event`.
- **Snapshot Writer**: the handle through which one Database Transaction makes one Snapshot: Event Versions begin and end through it, and the Snapshot row is written with its first change, never for a step that changes nothing. It replaces the Journal Transaction as the only writer of the user's side. Type `SnapshotWriter`.
- **Incoming Sync Plan**: what one Incoming Sync will write, decided from the fetched Server Objects, the versions with the server's mark and the current ones, before anything is written: per UID, follow the server, keep the user's, agree, Server Event Merge or Server Event Replacement, and the marks and ETags to move. Type `IncomingSyncPlan`.
- **Sides** of the comparison, as type names: `Base` (what both last agreed on: the versions with the server's mark, or what a taken-back Snapshot left), `Ours` (the current versions on the user's side) and `Theirs` (the fetched versions, or the versions a taken-back Snapshot ended). Proposed as plain names inside `comparison`, not glossary entries.
- **`UserAction` values**: `Moved` (a drag or resize in the grid, `journal/mod.rs:119`) is renamed `Rescheduled`, as `Core::reschedule_event` already says, and `MovedBetweenCalendars` is added for the move to another Calendar; both in 180, which first needs the move (ADR 0020, "A move with one side that can't be written").

## Today's code, in brief

| Area | Where | Goes or changes in |
|---|---|---|
| Core interface `Core` | `lib.rs:90`, `impl` :112; events :805–973, undo and journal :1013–1057, `record` :1058, `after_local_change` :1099 | throughout |
| Interface types for TypeScript | each module's `#[cfg_attr(test, derive(ts_rs::TS))]`; collected in `typescript.rs:33` | 174, 180–182, 187, 190, 194 |
| `EventId` | `event/when.rs:18`; 21 files (13 in `core/src`, 7 in `core/tests`, `src-tauri/src/main.rs`) | 174 (interface), 178 (inside) |
| Managed fields | `EventFields` `event/when.rs:249` (no `PartialEq`); `StoredEvent` `store/mod.rs:32`; `RecurrenceId` `event/recurrence_id.rs:22`, compared through `NormalForm` :92 | 177 (`EventVersionContent`) |
| Change Journal | `journal/mod.rs` (1337): `record` :202, `JournalTransaction` :338 (`create` :369, `replace` :384, `delete` :403, `put_back` :418, `writable` :458), `record_journal_event_modification` :486, `hash` :579; `TransactionOrigin` :98, `UserAction` :119 | 178–182 |
| Undo per Event, Dangerous Undo | `journal/undo.rs` (664): `undo` :145, `Plan` :214, `danger` :281, `Planner` :505 | 180–182 |
| Listings, Event History | `journal/list.rs` (311): `event_history` :256 | 182, 184 |
| Unsent Changes | written in `record_journal_event_modification` (`journal/mod.rs:497–509`) via `add_unsent_change` (`store/sql/journal.rs:77`); Sender hooks `sending` :284, `sent` :309, `not_sent` :321; reads `store/sql/mod.rs:295`, :560–645 | 182 (moved), 192 (gone) |
| Event Row writes | `store/sql/journal.rs` (322): `write_event` :19, delete :69, journal inserts :137/:157 | 178, 182 |
| Readers | `store/sql/mod.rs` (889): 9 queries read `event`; journal readers :664–756 | 178 (view), 182 |
| Schema | `store/schema.rs`: `MIGRATIONS` :10, baseline :63–195 (`event` :129, journal :156/:161, `unsent_change` :120, `caldav_server_object` :87, `caldav_ignored_resource` :97 with triggers :105–115), `migrate` :278 | most tickets |
| Write Permissions | `JournalWritePermission` `journal/mod.rs:46`, `SyncEngineWritePermission` `sync/mod.rs:262`, test `store/mod.rs:189`; Unguarded Writes `store/sql/unguarded.rs` | 196 |
| Event writes by scope | `event/scope.rs` (848): `edit_event` :218, `all` :503, `following` :550, `alone` :774; `Loaded::write_paired` `event/mod.rs:410`, `write_exception` :445 | 178 (through the Snapshot Writer) |
| Propagate | `event/mod.rs`: `pair_with_server` :539, `ServerPairing` :595, `propagate_to_event_rows` :904, `fields_over` :995 | 189 |
| Sync Engine entry | `sync/mod.rs`: `sync_calendar` :94, `sync_both_ways` :122, `Recorder` :31, `SendFailure` :221 | 189, 193, 200 |
| Incoming Sync, First Sync | `sync/caldav/incoming.rs` (326): `sync` :32, `first_sync` :60, `incoming_sync` :103, `mirror_and_propagate` :150, `keep` :236 | 189, 194 |
| Mirror, Ignored Resources | `sync/caldav/mod.rs` (515): `read_caldav_resource` :125, `mirror` :223, `ignore` :304; SQL `store/sql/mirror/caldav.rs` (221) | 187 (text goes), 189 |
| Sender | `sync/caldav/sender.rs` (669): `send` :40, `try_send` :132, `sent` :205 | 187, 192, 193 |
| Patching writer | `sync/caldav/resource/` (`mod.rs` 430, `write.rs` 524, `tests.rs` 928) | 176 (new writer beside it), 187 (patcher goes) |
| iCalendar reader | `import/mod.rs`: `read_events` :402, `read_vcalendars` :475; `ImportedEvent` :129 | 175 |
| Mass-Change Guard | `mass_change_guard.rs:16`, used only by Dangerous Undo (`journal/undo.rs:325`) | 201 |
| Sync Status | `sync_status.rs`: `OverallState::Conflict` :188 (placeholder), `SyncStatusKeeper` :350 | 190, 194, 202 |
| Shell | `src-tauri/src/main.rs`: 49 commands :105–599 wrapping `Core` one to one; undo and journal :518–553 | follows the interface |

No Conflict code exists today: the Sender's `SendFailure::ServerChanged` (`sender.rs:193`) is the nearest. So no Conflict test is switched off in 189 beyond the ETag group of `caldav_send.rs` already in the plan's table.

## The target module tree

```
core/src/
  lib.rs                    Core: the interface; Events by Combined Event Key (174)
  unimplemented.rs          the tree of Unimplemented Fields, canonical form, prefixes per Provider (175)
  comparison.rs             EventVersionContent, Field Groups, compare_event (177)
  history/                  the Calendar History: the only writer of the user's side (178–185)
    mod.rs                  SnapshotWriter, Snapshot origins, reading a Snapshot's state
    versions.rs             SQL of event_version and snapshot (moved from store/sql/journal.rs)
    undo.rs                 Undo, redo, Ctrl+Z's choice, asking first (180, 181)
    drop.rs                 dropping Snapshots no root reaches (185)
    event_history.rs        An Event's History (184)
    incoming.rs             IncomingSyncWriter: the one write of an Incoming Sync Snapshot (189)
    conflict.rs             recording, resolving and reading Conflicts (190, 191, 199)
  store/
    mod.rs, schema.rs       the Local Store and migrations
    check.rs                the Store Check (179)
    sql/mod.rs              readers, through the `event` view
    sql/settings.rs         today's unguarded.rs, renamed when Guards go (196)
    sql/caldav.rs           the CalDAV table and Ignored Resources, visible to the Sync Engine only (187, 192)
  event/                    as today; writes go through SnapshotWriter (178); propagate goes (189)
  ical/                     the CalDAV reader and writer (175, 176): read.rs, write.rs, roundtrip.rs (197)
  sync/
    mod.rs                  sync_calendar: one sync activity per Calendar (193)
    plan.rs                 IncomingSyncPlan from compare_event (189)
    caldav/incoming.rs      fetch; Full Fetch; Mark Reconciliation (189, 194, 195)
    caldav/sender.rs        Outgoing Sync from the Current-Version Marks (192, 193)
    caldav/mod.rs           Known Server Resources, Ignored Resources (stays)
  import/                   reads files through ical/ (175)
  mass_change_guard.rs      counts every Outgoing Sync (201)
```

`journal/` goes whole in 182. `sync/caldav/resource/` goes in 187.

## Interfaces designed twice

Decision 8 of the plan asks for "design it twice" on three interfaces. Each names the choice and its strongest con.

### The store of Event Versions: `SnapshotWriter`

- **A, a handle like today's Journal Transaction.** `history::write(core, origin, |w| …)` opens the Database Transaction and gives a `SnapshotWriter` with `put(key, EventVersionContent)`, `remove(key)`, `current(key)` and the reads the edit needs; the Snapshot row is inserted with the first change; nothing changed means no Snapshot. `event/scope.rs` and `import/` keep their shape, calling `w` where they call the `JournalTransaction` today.
- **B, a list of changes.** Callers compute every change up front and call `history::apply(origin, Vec<(CombinedEventKey, Option<EventVersionContent>)>)`.
- **Chosen: A.** B is purer, but the scope edits read and write in turns (`Loaded::write_paired`), and rewriting them all is scope no ticket carries. A keeps one writer for every origin, the way the Journal Transaction is today. Con: the order of `put`s inside one Snapshot is free, so the Store Check (not the type) catches a key written twice in one Snapshot.
- Visibility: `SnapshotWriter`'s SQL lives in `history/versions.rs`, `pub(super)`; nothing outside `history` writes `event_version`, `snapshot` or `conflict` (ADR 0018, "Who may write"). Incoming Sync gets its own handle, `IncomingSyncWriter` (189), because only it moves the server's marks and writes Conflicts: it has no operation that moves marks alone.

### The comparison: `compare_event`

- **A, per single Event.** `compare(base, ours, theirs) -> Outcome` per Occurrence, with the caller handling a Recurring Event against its Exceptions.
- **B, per UID.** `compare_event(base: &UidVersions, ours: &UidVersions, theirs: &UidVersions, on_conflict: OnConflict) -> UidOutcome`, where `UidVersions` maps a recurrence ID (`NormalForm`) to a `EventVersionContent`, and `OnConflict` is `Report` or `Win(Side)` ("… all over the later changes", ADR 0020). The rule that a change to the Recurring Event counts for every Occurrence, and that a Conflict covers the whole UID, live inside.
- **Chosen: B.** Both callers (Incoming Sync and Undo) need the UID-level rules (ADR 0019: "a Conflict on the whole Recurring Event, never combined"), and with A each would rewrite them. Con: the simplest test still builds three maps; 177 adds a helper for one single Event.
- `UidOutcome` (177 settles the variants): per recurrence ID, the version to make current (`Theirs`, `Ours`, a combination) or none, or `Conflict` for the UID. One exhaustive function `field_group(field) -> FieldGroup` over every managed field, without a catch-all (ADR 0019).
- 177 needs neither 178 nor the store: `EventVersionContent` is a value type (managed fields + 175's tree), and 178's row stores it. `EventFields` has no `PartialEq` (`RecurrenceId` hasn't one), so `EventVersionContent` compares through `NormalForm` and the tree's canonical form, never by `derive`.

### The Incoming Sync: `IncomingSyncPlan`

- **A, plan then write.** Inside the Database Transaction, read the server's marks and the current versions of the fetched UIDs, compute a pure `IncomingSyncPlan` with `compare_event`, then `IncomingSyncWriter` writes it.
- **B, write per resource.** The writer compares and writes as each fetched resource comes.
- **Chosen: A.** The plan is a seam the tests reach without a server: each case of ADR 0019's list (server alone, merge, replacement, agree, untrusted base) becomes a unit test of `sync/plan.rs`, and the Radicale tests check the wiring only. Con: two shapes for one step; the plan's types are internal and never reach the interface.
- Both are in one Database Transaction with the fetch's outcome (ADR 0019, "The invariant"); the fetch itself runs before it, as today (`incoming.rs:84`).

## Tables (end state)

All in `store/schema.rs`; each ticket appends a migration to `MIGRATIONS`. No migration converts old rows (no real Local Store exists): a table that changes shape is dropped and created. Each constraint carries a comment naming its rule; the Store Check holds the rest (ADR 0018, "Where the invariants are recorded").

| Table | Columns, constraints | Ticket |
|---|---|---|
| `snapshot` | `id` (the line's order), `made_at`, `origin` (user action with its `UserAction`, import, restore, undo, incoming sync, stop waiting), `undoes` → `snapshot` `DEFERRABLE INITIALLY DEFERRED`, `undo_uid` with `CHECK (undo_uid IS NULL OR origin = 'undo')` | 178 (`id`, `made_at`), 179 (`origin`, `undoes`), 181 (`undo_uid`) |
| `event_version` | `id`, `calendar`, `uid`, `recurrence_id` as obtained, the managed fields' columns (today's `event` :129), `unimplemented` (175's canonical form, 186), `begin` → `snapshot` `ON DELETE SET NULL`, `end` → `snapshot` `ON DELETE CASCADE`, `server_mark` (1 or NULL); partial unique index on (calendar, uid, recurrence ID) where `end IS NULL`, another where `server_mark`; index on (calendar, start) where `end IS NULL`. Triggers: no update but `end` from NULL, `begin` to NULL, `server_mark`; `end = begin` only with an Incoming Sync Snapshot; no delete of a row with `server_mark` and none of an open Conflict's Snapshot, except in a Calendar's removal | 178 (rows, `begin`/`end`), 179 (triggers), 186, 188 (`server_mark` and its trigger) |
| `event` | a view: `event_version` where `end IS NULL`; `store/sql/mod.rs` reads it unchanged | 178 |
| `conflict` | `id`, `snapshot` → `snapshot` `ON DELETE CASCADE`, `calendar`, `uid`, `resolved_at` (set once, by trigger); insert trigger: Incoming Sync Snapshots only | 190 |
| `conflict_event` | `conflict` → `conflict` `ON DELETE CASCADE`, `recurrence_id`, `last_agreed` → `event_version` `DEFERRABLE INITIALLY DEFERRED` (the drop's edge) | 190 |
| `caldav_server_object` | `calendar`, `uid`, `href`, `etag`, the round-trip mark (197); no text (187); unique (calendar, href), (calendar, uid) | 187, 197 |
| `caldav_ignored_resource` | as today, with its triggers reworded from "mirrored" to "kept" | 187 |
| `calendar` | the sync token with `last_full_fetch` beside it (195); the First Sync flag columns go (194); the "writes refused" state (200) | 194, 195, 200 |
| goes | `journal_transaction`, `journal_event_modification` (182), `unsent_change` (192), the `event` table (178, now a view) | |

The Current-Version Marks: the user's is `end IS NULL`, the server's is `server_mark`; the ETag sits in `caldav_server_object`, keyed by Calendar and UID, written in the same Database Transaction as the mark (ADR 0018, "Who may write").

## Seams for `/tdd`

1. **Pure, unit tests in the module**: `unimplemented.rs` and `ical/` (175, 176, 197, 210); `comparison.rs` (177); `sync/plan.rs` (189, 195, 198); `history/drop.rs`'s choice of Snapshots (185), given a small Local Store.
2. **The core interface** (`Core`, through `core/tests/support/Harness`): every behaviour the user sees. From 179 the Harness runs the Store Check after each test (on drop of the `Harness`, failing the test on a violation), so every scenario checks the invariants.
3. **The core interface against a server**: `core/tests/support/radicale.rs` (`Radicale`, `RewritingProxy`), for sync wiring, ETags and refusals (189–201).
4. **The window**: `src/test/fakeCore` and the App tests, unchanged in approach.

## Tests through the core interface that survive the whole redesign

These files stay green from 173 to 214 with at most mechanical changes (174's key in place of `EventId`, helpers in `core/tests/support/` following): `caldav_accounts`, `caldav_discovery`, `caldav_calendars`, `clock`, `settings`, `sidebar_state`, `view_state`, `local_account`, `local_store`, `local_calendars`, `event_times`, `import`, `events`, `sync_timing`. They are the regression net while other tests are switched off: a ticket that has to change one beyond that has departed from the plan and says why. The plan's "Test sorting" lists the rest.

## By phase

### P0

**173.** A script beside the pre-commit hook (`.husky/pre-commit`, which lists the checks) that greps `#[ignore` and `it.skip` / `describe.skip` / `test.skip` over `core/`, `src-tauri/` and `src/`, and refuses a bare one, a note not of the form `until ticket NN: <why>`, or an NN whose file in `.scratch/freecal-v1/issues/` is missing or `**Status:** done` or `wontfix` (wontfix added in ticket 173). No backend module. Built as `.husky/switched_off_tests.py`, tested by `.husky/test_switched_off_tests.py`; the frontend note is a comment `// until ticket NN: <why>` on the line right before the skip. The keyring test of `src-tauri/src/keyring.rs`, which needs a desktop session, names ticket 28 (packaging). The check runs on every commit, not only those staging code (changed in ticket 173: marking a ticket done can break it).

### P1

**174.** `Core`'s event functions (`lib.rs:805–973`) take and return `CombinedEventKey` (glossary; today in the store, `store/mod.rs`) in place of `EventId`; `Event` (`event/mod.rs:58`) carries its key. `EventId` stays as `pub(crate)` inside `event/` and `store/` until 178 makes it the Event Version's row ID. `src-tauri/src/main.rs` and `src/core/*` follow; `src/test/ids.ts` makes keys. `original_server_data` (`lib.rs:819`) takes the key too, until 187 removes it.

**175.** New `unimplemented.rs`: `UnimplementedFields` (a tree of named entries, serde, canonical form with sorted entries), prefixes per Provider. New `ical/read.rs`: the CalDAV reader, built from `import/mod.rs:402` (`read_events`), returning `ImportedEvent`s with their `UnimplementedFields`; the import reads through it too, so `.ics` files keep `ical:` entries. Its precise rules and what they drop on purpose are listed beside them (ADR 0017). Pure; tests in `ical/read.rs`.

**176.** New `ical/write.rs`: one resource from a UID's single Events as managed fields plus tree, VTIMEZONE (from `resource/write.rs:388`), folding and CRLF, the writer's fresh `DTSTAMP`/`LAST-MODIFIED`; `SEQUENCE` from the tree. Takes over the cases worth keeping from `resource/tests.rs` (hostile Event Rows, folding, VTIMEZONE). Not wired to the Sender yet (187).

**177.** New `comparison.rs` as designed above. Pure.

### P2

**178.** New `history/` with `SnapshotWriter`: `event_version` the table, `event` a view, and a bare `snapshot` table (`id`, `made_at`) so that `begin` and `end` refer to a Snapshot from the first row on. Every write through the `SnapshotWriter` makes one Snapshot and ends the rows it replaces; a placeholder for `end` would be glue that 179 deletes. The `JournalTransaction` (`journal/mod.rs:338`) calls the `SnapshotWriter` for its row writes, and the Change Journal keeps recording beside it. `EventId` is the Event Version's row ID and changes with every write; callers in `event/` reload by key. (B2 moved `begin`/`end` here from 179; see the plan's "Changes to the draft phases".)

**179.** The Snapshot's origin and the line's rules: the insert and update triggers on `event_version`; `TransactionOrigin` (`journal/mod.rs:98`) becomes the Snapshot's origin type in `history/mod.rs`, First Sync pages write Incoming Sync Snapshots. `store/check.rs`, the Store Check, and the Harness running it. The core's part of removing a Calendar (`Core::remove_calendar`, `lib.rs` :693–804; `delete_local_copy` :753) deletes its Event Versions and empty Snapshots.

**180.** `history/undo.rs`: `Core::undo_snapshot`, `Core::ctrl_z`, `Core::redo` (names for 180 to settle), using `compare_event` with `Base` = what the taken-back Snapshot left, `Ours` = current, `Theirs` = what it ended. New interface types, added to `typescript.rs:33` beside the old ones; the window doesn't call them until 183. Today's `Core::undo` (`lib.rs:1013`) and `journal/undo.rs` stay until 182.

**181.** `history/undo.rs` grows the left-out Events (`LeftOut { key, reason }`), limits to one UID (`snapshot.undo_uid`), moves with one side unwritable (the Snapshot's `UserAction::MovedBetweenCalendars`), and `OnConflict::Win`.

**182.** Delete `journal/` (`mod.rs`, `undo.rs`, `list.rs`), `store/sql/journal.rs`'s journal parts, the journal tables, `Core::undo`/`change_journal`/`journal_transaction_events`/`undo_history` (`lib.rs:1013–1057`) and their commands (`src-tauri/src/main.rs:518–553`), the window's Change Journal dialog and History button. **Unsent Changes until 192** (the plan's open question): the write is today one call in `record_journal_event_modification` (`journal/mod.rs:497–509`, about 12 lines), computed from the previous and new versions, not from journal rows. 182 moves it into the `SnapshotWriter`: each Event Version begun or ended by a Snapshot whose origin isn't an Incoming Sync adds its Combined Event Key to `unsent_change`. `sending`/`sent`/`not_sent` (`journal/mod.rs:284–335`, about 50 lines) move to `sync/caldav/sender.rs` and compare the Event Versions' row IDs in place of the hash, since a row never changes. Little code, moved rather than written: the exception to decision 1 holds, and 192 deletes it with the table.

**183.** Window only, on 180's and 181's types.

**184.** `history/event_history.rs`: the versions of one key's UID in the order of `begin`; `Core::event_history` takes the key.

**185.** `history/drop.rs`: roots (90 days; later 188 the server's mark, 190 open Conflicts) and edges (Undo → taken back; later 190 Conflict → Last Agreed Version), run where `journal::record` prunes today (`journal/mod.rs:202`, `delete_journal_before`), i.e. at the start of a write, or once a day; 185 decides and says which.

**186.** `event_version.unimplemented`; the reader's tree reaches the row through the Incoming Sync and the import; the scope edits carry it (ADR 0017, "What is in the tree"); An Event's History says "changed in fields FreeCal doesn't show".

**187.** The Sender (`sender.rs:132`) builds every resource with `ical/write.rs` from the Event Versions of the UID; `sync/caldav/resource/` and `caldav_server_object`'s text go; `Core::original_server_data` and the window's view of it go. Propagate (`event/mod.rs:904`) reads the fetched `ImportedEvent`s, not stored text, so it stays until 189.

### P3

**188.** `event_version.server_mark`, its unique index, the delete trigger; Incoming Sync sets it on the rows it writes, the Sender moves it on success. The drop's root (185).

**189.** `sync/plan.rs` (`IncomingSyncPlan`) and `history/incoming.rs` (`IncomingSyncWriter`): `incoming.rs:103` and `mirror_and_propagate` :150 become fetch → plan → write in one Database Transaction. Propagate (`event/mod.rs:539–1012`'s pairing and `propagate_to_event_rows`) goes. Every Event changed differently on both sides is a Server Event Replacement for now; the Conflict row comes in 190.

**190.** `conflict`, `conflict_event`, their triggers; the plan's Conflict branch writes them; `sync_status.rs:188` `OverallState::Conflict` becomes real; the window's sync display follows. Drop's root and edge (185).

**191.** `history/conflict.rs`: resolving (keep the server's version; restore mine = 181's Undo limited to the UID; any later change sets `resolved_at`).

**192.** `sender.rs`: what to send is the difference between the two marks (a reader in `history/`), not `unsent_change`; the table, 182's write and the Sender's hooks go; the Unsent Changes the window shows are derived (`store/sql/mod.rs:560–645` rewritten).

**193.** `sync/mod.rs:122` (`sync_both_ways`): one activity per Calendar; read-back without ETag; the server's form in the sent one's place (`history/` writes the new row with the sent one's `begin`/`end`).

**194.** `incoming.rs:60` (`first_sync`) goes: no sync token means a Full Fetch through 189's path; a Calendar is added empty; the First Sync flag columns and `unguarded.rs`'s `finish_*_sync` (:106–152) go; the window drops the flag.

**195.** `last_full_fetch` beside the sync token; Mark Reconciliation as the next Incoming Sync after 7 days; the untrusted base as a case of `sync/plan.rs`.

**196.** Write Permissions go: `JournalWritePermission`'s remains, `SyncEngineWritePermission` (`sync/mod.rs:262`), the test `store/mod.rs:189`; Rust visibility (`pub(super)` in `history/`, `pub(in crate::sync)` for `store/sql/caldav.rs`) takes their place; `unguarded.rs` is renamed.

**197.** `ical/roundtrip.rs`: write back in memory and compare; the mark in `caldav_server_object`; a note in the details.

### P4

**198.** `sync/plan.rs`: the merge branch, with `compare_event`'s combination; zero-length Event Versions through `IncomingSyncWriter`.

**199.** `history/conflict.rs`: what each side changed, per Field Group, from `last_agreed`.

**200.** Sync Error "writes refused" in `sync_status.rs`; privileges read on Poll (`remote.rs:97`, `server_read_only`); the "Stop waiting" Snapshot origin.

**201.** `mass_change_guard.rs` counts every Outgoing Sync in `sender.rs`, reading Snapshot origins from `history/`.

### P5 and P6

The window tickets (202 to 209) change `src/` and add only the core readers they need (counts for 209, the Calendar History's listing for 207, in `history/`). Google (210 to 214): `google/` beside `ical/`, with `google_server_object` keyed per Event, as ticket 26 had it; the layout of `google/` is left to 210 and 211, which add it here.
