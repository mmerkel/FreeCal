# 96: Incoming Sync in the code, and the code laid out by guard

**What to build:** Two mechanical changes agreed on 2026-11-06. Behaviour stays the same.

1. **"sync" splits into Incoming Sync and Outgoing Sync** (`GLOSSARY.md`). The code says Incoming Sync wherever it means bringing in what the server changed: the Transaction Origin `TransactionOrigin::Sync` becomes `IncomingSync`, and its stored value `"sync"` becomes `"incoming_sync"` (`core/src/store/journal.rs`, `recorded_as`). There are no real Local Stores yet, so there's no migration. "Sync" on its own stays only where both directions are meant.
2. **The code is laid out by guard.** Each guard is the module that alone writes a set of tables: the Change Journal or the Sync Engine.
   - The Change Journal moves from `core/src/store/journal.rs` to `core/src/journal/`.
   - The Sync Engine moves from `core/src/sync.rs` to `core/src/sync/`: `mod.rs` holds what both directions share (`settle` and the helpers), and `incoming.rs` holds the Incoming Sync (`apply`, `take`, `remove`). `sender.rs` comes with ticket 17, so don't create it empty.
   - All SQL stays under `core/src/store/`. `sql.rs` becomes `sql/mod.rs` (the shared readers), `sql/journal.rs` (writes the Change Journal guards), `sql/mirror.rs` (writes the Sync Engine guards) and `sql/unguarded.rs` (accounts, Calendars, settings, sidebar, view, time zone rules).
   - Renormalising a UID's Combined Event Keys when an Exception joins its Recurring Event stays as it is now: called from the Change Journal (`JournalTransaction::rejoin_combined_event_keys`), with its SQL in `sql/journal.rs`.
   - Deleting a Calendar or an account stays in the Change Journal, with its SQL in `sql/journal.rs`.

Until ticket 97, `Journaled` still guards every write. The new files only show the split that ticket 97 enforces.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `TransactionOrigin::IncomingSync`, stored as `"incoming_sync"`; identifiers, comments and test names that mean the incoming direction say Incoming Sync (or `incoming`)
- [x] `core/src/journal/` holds the Change Journal and `core/src/sync/` holds the Sync Engine, with `mod.rs` and `incoming.rs`
- [x] `core/src/store/sql/` has `mod.rs`, `journal.rs`, `mirror.rs` and `unguarded.rs`; each write sits in the file of the guard that will hold its Write Permission in ticket 97; there's no `sql.rs` left
- [x] Moves keep git history (`git mv` before editing)
- [x] `CODING_STANDARDS.md` "Local Store" names the new SQL files and what each holds, and drops the old `modification` table name
- [x] Every existing test passes unchanged apart from paths, names and the stored Transaction Origin value

## Comments

- 2026-11-06 (implemented): The Transaction Origin `TransactionOrigin::IncomingSync` is stored as `"incoming_sync"`. `is_sync` is now `is_incoming_sync`, which also covers the First Sync, since a First Sync is an Incoming Sync; ticket 97 removes it. Comments in `journal/`, `sync/`, `remote.rs`, `event/`, `schema.rs` and `lib.rs` now say Incoming Sync where they mean the incoming direction, and First Sync is capitalised. The one generated type that changed is a doc comment on `Calendar::first_sync`. These unit tests were renamed `an_incoming_sync_…`: `an_incoming_sync_mirrors_…`, `an_incoming_sync_leaves_nothing_unsent` and `an_incoming_sync_that_moves_…`.
  - Layout: the Change Journal is in `core/src/journal/mod.rs`. Callers now import it from `crate::journal`, and `store` no longer re-exports it. `sync.rs` moved to `sync/incoming.rs` (`apply`, `take`, `remove`, `moved`, tests). `settle`, `recurring_of`, `object_events` and `event_keys_in_calendar` went to `sync/mod.rs`. `sql/mod.rs` holds every reader, including the ones only the Change Journal uses. `sql/journal.rs`, `sql/mirror.rs` and `sql/unguarded.rs` hold the writes, split as ticket 97 assigns the Write Permissions. `set_synced` (the Calendar's sync token) is in `unguarded.rs`. Visibility changes the moves needed: `Journaled` and the Store methods the Change Journal calls are `pub(crate)`, as are the test-only `Store::connection` and `store::UnsentChange`. All four moves show as renames.
  - Left as is for the user to decide:
    - The integration tests in `core/tests/caldav_sync.rs` (`a_sync_…`) drive `Core::sync_account`, the user's Sync, so they keep "sync".
    - `Core::sync_calendar` runs only an Incoming Sync today and could become `incoming_sync_calendar`. It keeps its name in case ticket 17 has it run both directions.
    - The Calendar's `synced` and `sync_token` columns and `JournalTransaction::synced` keep their names.
  - Terms from this ticket that are in neither section of `GLOSSARY.md`:
    - **Guard**: the module that alone writes a set of tables, the Change Journal or the Sync Engine. Approved and added to `GLOSSARY.md` Internals on 2026-11-06.
    - **Unguarded Write**: a write that no Guard's Write Permission covers (`sql/unguarded.rs`). Approved and added to `GLOSSARY.md` Internals on 2026-11-06.
    - (new term) **Reader**: a Local Store query, in `sql/mod.rs`.
