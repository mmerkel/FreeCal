# 90: Journaling inside the Local Store

**What to build:** The Change Journal becomes part of the Local Store's module, and the compiler makes sure that only it writes Event rows, server objects and the journal's tables (ADR 0011). Today `journal.rs` runs its own SQL through `Store::connection()`, so "every change to an Event is recorded" (ADR 0009) holds only because nothing else happens to write those tables. A restructuring: no behaviour changes. Decided in a grilling on 2026-10-25.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] `store` becomes a directory: `store/mod.rs` keeps opening, the schema, reads, and the writes nobody journals (Accounts, Calendars, settings, sync state); `store/journal.rs` holds `Unit`, `record`, `undo` and the hash; the SQL that writes `event`, `server_object`, `journal_unit` and `journal_entry` lives in a submodule of `store`, visible only inside it (`pub(super)` or private)
- [ ] `Unit` and the journal's public functions are re-exported from `store`, so the only way to change an Event from outside the module is a `Unit`; the callers in `event`, `import`, `sync` and `lib.rs` change only their imports
- [ ] `Store::connection()` is no longer `pub(crate)`; tests that set rows up directly use a `#[cfg(test)]` helper of `Store`
- [ ] The split inside `store` stays navigable: each file's doc comment says what it owns (reads and unjournaled writes, the journal, the row writes)
- [ ] Every existing test passes unchanged apart from its imports

## Comments

- 2026-10-25: Rust's `pub(in path)` only names ancestor modules, so "only the journal may call this" needs the journal inside `store`; a token type or a test that greps for SQL were weighed and rejected as weaker or pointing the dependency the wrong way.
