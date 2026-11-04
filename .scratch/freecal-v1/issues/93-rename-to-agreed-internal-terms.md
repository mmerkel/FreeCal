# 93: Name the code after the agreed internal terms

**What to build:** The code uses the terms of the "Internals" section of `GLOSSARY.md`, agreed on 2026-11-04, so that the code, the ADRs and the conversation with the user say the same thing. A rename: behaviour stays the same. The glossary, ADRs 0004 and 0009–0011, the spec and the open tickets already use the new terms.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

| Old term | Agreed term |
|---|---|
| unit (`journal::Unit`, `UnitId`, `journal_unit`) | Journal Transaction |
| origin (`Origin`) | Transaction Origin |
| entry (`Entry`, `journal_entry`) | Modification |
| version (of an Event, in the journal) | Event Version |
| row (of the `event` table) | Event Row |
| key (`server_object_key`, `rejoin_keys`, ...) | Combined Event Key |
| the object that owns a UID (`owned_uids`) | Corresponding Server Object |
| baseline (a Calendar's first sync) | First Sync |

One naming rule: types, functions, modules and tables carry the full term (`JournalTransaction`, `JournalTransactionId`, `TransactionOrigin`, `Modification`, `journal_transaction`, `modification`, `combined_event_key`), because they appear unqualified where they are used. Fields and columns may drop a qualifier their enclosing name already gives (`origin` in `journal_transaction`, `journal_transaction_id` in `modification`).

- [ ] Rust core: types, functions, modules, local names and doc comments, including `Unit::entry`, `rejoin_keys`, `keys_with_uid`, `insert_keys`, `write_uid_keys`, `owned_uids`, and the "baseline" in the doc comment of `Origin::FirstSync`. "Server object" in comments becomes "Server Object"
- [ ] Schema, in the baseline migration (nothing is released, ADR 0008; no Local Store needs converting): `journal_unit`, `journal_entry`, `server_object_key` and their columns and indexes
- [ ] Core interface: `UnitId`, the `unit` field of `Changed` and the other results that carry it, and the error codes `unitNotFound` and `changedSinceUnit`; then `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`
- [ ] Frontend: every use of the renamed interface types, fields and error codes, and the messages keyed by those error codes
- [ ] Test names and test helpers
- [ ] No old term is left for these concepts: a search for `unit`, `entry`, `entries`, `owned`, `server_object_key` and `baseline` in `core/src` and `src` finds only unrelated uses (`rusqlite::Row`, keyring keys, "of its own", the schema baseline)
- [ ] Every existing test passes unchanged apart from names

Out of scope: closed tickets keep their wording as a record.

## Comments

- 2026-11-04: Best done before tickets 17, 19 and 72, which touch the journal and the Server Objects and would otherwise add more code under the old names.
