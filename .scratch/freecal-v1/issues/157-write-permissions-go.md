# 157: Write Permissions and Guards go; module visibility replaces them

**What to build:** The last step of the order in which the old model goes (ADR 0018, "Who may write"). The value that every write to a guarded table takes is removed, and Rust's visibility says who may write instead. No behaviour changes.

- **What comes out:** the Write Permission types and the parameters that carry them, in the store (`core/src/store/mod.rs`, `core/src/store/sql/mod.rs`, `core/src/store/sql/mirror/caldav.rs`, `core/src/store/sql/unguarded.rs`) and at their callers (`core/src/sync/mod.rs`, `core/src/lib.rs`, `core/src/read_only.rs`, `core/src/sync_status.rs`, and what tickets 147 to 155 left of them under the Calendar History's module).
- **What replaces them:**
  - the functions that write Event Versions, Snapshots, `end`, the server's Current-Version Marks and Resolutions are visible only to the Calendar History's module;
  - those of the CalDAV table and the Ignored Resources only to the Sync Engine's;
  - every write still runs in a Database Transaction, by its type, as before.
- **The Calendar History's module stays small:** the functions that can reach a low-level write (setting a mark, setting `end`) are few enough to read in one sitting, and each low-level write has a comment naming the one function that may call it. This is the price ADR 0018 names for dropping the value.
- **The rules of the interface** that the Store Check can't ask stay checked by shape: no operation moves the server's marks alone (ADR 0019, "The invariant"), and every change of the user's goes through the one place that makes its Snapshot and writes the Resolution of a Conflict it ends (ticket 134).
- **Docs:** `CODING_STANDARDS.md` replaces its rules on guards and Write Permissions (the paragraphs on where SQL lives, which file a write sits in, guarded tables and the private field) with the visibility rule and the Store Check's rule (a change to the schema touches the schema, the Store Check and an ADR). `GLOSSARY.md` loses Write Permission, Guard and Unguarded Write, and the lines in other entries that name them.
- Ticket 63 (squash the schema migrations) runs after this one.

Spec: none. ADR 0018 ("Who may write", "Where the invariants are recorded", "Considered Options": keeping a Write Permission).

**Blocked by:** 154, 155

**Status:** ready-for-agent

**Effort:** S

- [ ] No Write Permission type, parameter or Guard is left in `core/`
- [ ] The Calendar History's writes can't be called from outside its module, nor the CalDAV table's and the Ignored Resources' from outside the Sync Engine's (a compile-fail test or a doc test for each)
- [ ] Each low-level write names its one caller in a comment
- [ ] `CODING_STANDARDS.md` and `GLOSSARY.md` no longer name Write Permissions, Guards or Unguarded Writes
- [ ] Every existing test passes unchanged

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): no links or pointers; the rules of the interface name the server's marks and the Resolution.
