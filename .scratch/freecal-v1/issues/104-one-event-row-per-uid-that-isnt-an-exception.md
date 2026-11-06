# 104: One Event Row per UID that isn't an Exception, and one way to pair Exceptions

**What to build:** Agreed on 2026-11-06. A Calendar has at most one Event Row per UID that isn't an Exception, and the Local Store enforces it. That lets the three ways of pairing Exceptions by Occurrence become one.

- A unique index on `event (calendar_id, uid) WHERE recurrence_id IS NULL`. Change the baseline schema (ADR 0008).
- The one path that can write a second: undoing a delete puts the Event Version back under its old ID (`put_back`, `core/src/journal/mod.rs`) even when an Incoming Sync has since added an Event Row with the UID. Undo refuses that, as it refuses an Event changed since.
- The Loaded Event's "the first by ID, should the Calendar have several with the UID" goes.
- One pairing method on `Loaded` (`core/src/event/mod.rs`): given incoming Events and the Recurring Event to pair against, it returns each incoming Event with the stored Event Row for the same Occurrence, if any, and the stored Exceptions left over, from Normal Forms computed once. The import of a whole Event (`Loaded::take`), the import of a lone Exception (`Loaded::take_exception`) and Propagate (`propagate_to_event_rows`) use it and keep only what differs: which recurrence ID is stored, and what happens to the leftovers. Propagate works on a `Loaded` instead of the raw Event Rows.

**Blocked by:** 105

**Status:** ready-for-agent

**Effort:** M

- [ ] Writing a second Event Row with one UID that isn't an Exception in a Calendar fails in the Local Store
- [ ] Undoing a delete after an Incoming Sync added an Event Row with the UID is refused, as an undo of an Event changed since is
- [ ] Import, lone-Exception import and Propagate pair Exceptions through one method on `Loaded`
- [ ] Each stored Exception's Normal Form is computed once per pairing
