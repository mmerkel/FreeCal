# 97: Database Transactions and Write Permissions

**What to build:** Three separate things, which today are all the Journal Transaction, as agreed on 2026-11-06 (ADR 0011, `GLOSSARY.md`: Database Transaction, Write Permission). Behaviour stays the same.

1. **A Database Transaction value.** `DatabaseTransaction` replaces the closure-only `Store::transaction` (`core/src/store/mod.rs`). Every write in `core/src/store/sql/` takes it, so a write outside a Database Transaction doesn't compile. Reads may stay on `Store`.
2. **The Journal Transaction is built on it** and covers Event Rows, the Change Journal and Unsent Changes, as now. The Mirror's writes move off `JournalTransaction`: `mirror`, `rewrite_combined_event_keys`, `unreadable` and `remove_object` go, together with their `debug_assert!(self.origin.is_sync())` checks. An Incoming Sync runs its Mirror writes on the Journal Transaction's Database Transaction, so they still commit together with its Event Rows.
3. **One Write Permission per guard**, replacing `Journaled`. Each has a private constructor, so only its module can make it.
   - `JournalWritePermission`, made in `core/src/journal/`, for the writes in `sql/journal.rs`: Event Rows, Journal Transactions and Journal Event Modifications, Unsent Changes, deleting Calendars and accounts, and renormalising Combined Event Keys (`rejoin_combined_event_keys`, unchanged by this ticket).
   - `SyncEngineWritePermission`, made in `core/src/sync/`, for the writes in `sql/mirror.rs`: writing a Server Object's data and its Combined Event Keys, rewriting a Server Object's Combined Event Keys of one UID, recording one as unreadable and removing one.
   - `sql/unguarded.rs` writes take only the Database Transaction.

**Blocked by:** 96

**Status:** ready-for-agent

**Effort:** M

- [ ] `DatabaseTransaction` exists, and every write takes it; `Store::transaction` is gone
- [ ] `JournalTransaction` holds a `DatabaseTransaction` and has no Mirror writes; `is_sync` is gone if nothing else needs it
- [ ] Every write in `sql/journal.rs` takes `&JournalWritePermission`, and every write in `sql/mirror.rs` takes `&SyncEngineWritePermission`; `Journaled` is gone
- [ ] Neither permission can be made outside its module; a compile-fail doctest (or an equivalent check) shows it for each
- [ ] The Incoming Sync writes the Mirror with `SyncEngineWritePermission`, in the same Database Transaction as its Journal Transaction
- [ ] `CODING_STANDARDS.md` "Local Store": each guarded write takes its guard's Write Permission and a Database Transaction (ADR 0011)
- [ ] Every existing test passes unchanged apart from names and signatures
