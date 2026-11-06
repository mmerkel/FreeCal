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

**Status:** ready-for-agent

**Effort:** M

- [ ] `TransactionOrigin::IncomingSync`, stored as `"incoming_sync"`; identifiers, comments and test names that mean the incoming direction say Incoming Sync (or `incoming`)
- [ ] `core/src/journal/` holds the Change Journal and `core/src/sync/` holds the Sync Engine, with `mod.rs` and `incoming.rs`
- [ ] `core/src/store/sql/` has `mod.rs`, `journal.rs`, `mirror.rs` and `unguarded.rs`; each write sits in the file of the guard that will hold its Write Permission in ticket 97; there's no `sql.rs` left
- [ ] Moves keep git history (`git mv` before editing)
- [ ] `CODING_STANDARDS.md` "Local Store" names the new SQL files and what each holds, and drops the old `modification` table name
- [ ] Every existing test passes unchanged apart from paths, names and the stored Transaction Origin value
