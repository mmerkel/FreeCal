# 94: Rename Modification to Journal Event Modification

**What to build:** "Modification" becomes **Journal Event Modification**, agreed on 2026-11-04, so that the term says what it is part of: one Journal Transaction holds several Journal Event Modifications. A rename: behaviour stays the same.

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `GLOSSARY.md`: the Safety entry, its uses in other entries, and the Internals entry "Record a Modification", which becomes "Record a Journal Event Modification"
- [x] ADRs 0004, 0009, 0010 and 0011, the spec and open tickets 19 and 72
- [x] Rust core: the type `JournalEventModification`, `record_journal_event_modification`, `insert_journal_event_modification`, `Store::journal_event_modifications`, doc comments and test names
- [x] Schema, in the baseline migration (ADR 0008): the table `journal_event_modification` and its indexes
- [x] Frontend test fake (`src/test/fakeCore.ts`): its Journal Event Modifications
- [x] A search for `Modification` and `modification` in `core`, `src`, the glossary, the ADRs, the spec and the open tickets finds only the new term, and locals holding one Journal Event Modification
- [x] Every existing test passes unchanged apart from names

Out of scope: closed tickets keep their wording as a record.

## Comments

- 2026-11-04 (implemented): renamed throughout; behaviour unchanged, every test passes with only names changed. Locals holding one or more Journal Event Modifications stay `modification`/`modifications`; the indexes are `journal_event_modification_journal_transaction` and `journal_event_modification_event`; the field of the frontend fake's Journal Transaction is `journalEventModifications`. The test `deleting_a_calendar_removes_the_modifications_that_would_put_an_event_back_into_it` is `deleting_a_calendar_removes_the_journal_event_modifications_putting_events_back`, to fit the line. "Modification (on its own)" joins the glossary's _Avoid_ list. No new terms.
