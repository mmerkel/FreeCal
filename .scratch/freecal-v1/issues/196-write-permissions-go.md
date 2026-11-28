# 196: Write Permissions and Guards go; module visibility replaces them

**What to build:** The value every write to a guarded table takes is removed, and Rust's visibility says who may write (ADR 0018, "Who may write"). No behaviour changes. Old ticket 157 (`git show design/snapshot-line:.scratch/freecal-v1/issues/157-write-permissions-go.md`) as written.

- **What comes out:** `JournalWritePermission`'s remains, `SyncEngineWritePermission` (`sync/mod.rs`) and the parameters carrying them, in the store (`store/mod.rs`, `store/sql/`) and at their callers (`sync/`, `lib.rs`, `read_only.rs`, `sync_status.rs`, `history/`).
- **What replaces them:** the writes of Event Versions, Snapshots, `end`, the server's marks and Conflicts are `pub(super)` in `history/`; those of the CalDAV table and the Ignored Resources `pub(in crate::sync)` in `store/sql/caldav.rs`. Every write still runs in a Database Transaction, by its type. `unguarded.rs` becomes `store/sql/settings.rs`.
- **`history/` stays small:** the functions that reach a low-level write (setting a mark, setting `end`) are few enough to read in one sitting, and each low-level write has a comment naming the one function that may call it, the price ADR 0018 names for dropping the value.
- **Checked by shape:** no operation moves the server's marks alone (ADR 0019, "The invariant"); every change of the user's goes through the `SnapshotWriter`, which ends a Conflict it touches (191).
- **Docs:** `CODING_STANDARDS.md` replaces its rules on guards and Write Permissions with the visibility rule and the Store Check's rule (a change to the schema touches the schema, the Store Check and an ADR). Ticket 63 (squash the migrations) runs after this one.

Spec: none. ADR 0018 ("Who may write", "Where the invariants are recorded", "Considered Options"). Layout: "The target module tree"; "By phase", P3, 196.

**Blocked by:** 193, 194

**Status:** ready-for-agent

**Effort:** S

**Rewritten:** `core/tests/read_only.rs`, without the Guards' exceptions. **Deleted tests:** `store/mod.rs`'s `only_its_guard_makes_a_write_permission`.

- [ ] No Write Permission type, parameter or Guard is left in `core/`
- [ ] The Calendar History's writes can't be called from outside `history/`, nor the CalDAV table's and the Ignored Resources' from outside `sync/` (a compile-fail doc test for each)
- [ ] Each low-level write names its one caller in a comment
- [ ] `CODING_STANDARDS.md` follows
- [ ] Every other test passes unchanged
- [ ] `GLOSSARY.md` follows: Write Permission, Guard and Unguarded Write go, with the lines in other entries that name them
