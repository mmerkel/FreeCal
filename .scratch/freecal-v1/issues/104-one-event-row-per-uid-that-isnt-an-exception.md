# 104: One Event Row per UID that isn't an Exception, and one way to pair Exceptions

**What to build:** Agreed on 2026-11-06. A Calendar has at most one Event Row per UID that isn't an Exception, and the Local Store enforces it. That lets the three ways of pairing Exceptions by Occurrence become one.

- A unique index on `event (calendar_id, uid) WHERE recurrence_id IS NULL`. Change the baseline schema (ADR 0008).
- The one path that can write a second: undoing a delete puts the Event Version back under its old ID (`put_back`, `core/src/journal/mod.rs`) even when an Incoming Sync has since added an Event Row with the UID. Undo refuses that, as it refuses an Event changed since.
- The Loaded Event's "the first by ID, should the Calendar have several with the UID" goes.
- One pairing method on `Loaded` (`core/src/event/mod.rs`): given incoming Events and the Recurring Event to pair against, it returns each incoming Event with the stored Event Row for the same Occurrence, if any, and the stored Exceptions left over, from Normal Forms computed once. The import of a whole Event (`Loaded::take`), the import of a lone Exception (`Loaded::take_exception`) and Propagate (`propagate_to_event_rows`) use it and keep only what differs: which recurrence ID is stored, and what happens to the leftovers. Propagate works on a `Loaded` instead of the raw Event Rows.

**Blocked by:** 105

**Status:** done

**Effort:** M

- [x] Writing a second Event Row with one UID that isn't an Exception in a Calendar fails in the Local Store
- [x] Undoing a delete after an Incoming Sync added an Event Row with the UID is refused, as an undo of an Event changed since is
- [x] Import, lone-Exception import and Propagate pair Exceptions through one method on `Loaded`
- [x] Each stored Exception's Normal Form is computed once per pairing

## Comments

2026-11-06, implemented:

- The baseline schema has the unique index `event_calendar_uid_not_exception` on `event (calendar_id, uid) WHERE recurrence_id IS NULL` (`core/src/store/schema.rs`).
- Undo refuses to put back an Event Version that isn't an Exception while its Calendar has another Event Row with the UID that isn't one, with `ChangedSinceJournalTransaction` (`JournalTransaction::put_back`, `core/src/journal/mod.rs`). This covers undoing a delete, and also undoing a move out of a Calendar that has since gained the UID. Undo runs latest first, so a Journal Transaction that deleted one Event Row and created another with the UID still undoes.
- `Loaded::pair` (`core/src/event/mod.rs`) pairs incoming Events with the stored ones by Occurrence, against a given Recurring Event. It keeps the latest of several incoming Events for one Occurrence, and computes each Normal Form once per pairing. It returns `Pairing`: each incoming Event with its stored Event, if any, and the stored Event and Exceptions left over. `Loaded::write_paired` writes the pairs, and `Kept` says which recurrence ID is kept. `take` and `take_exception` keep the stored one; Propagate takes the incoming one and works on a `Loaded`. Each caller does its own thing with the leftovers.
- `Loaded` no longer picks "the first by ID" of several Events that aren't Exceptions; a `debug_assert!` relies on the index.
- Left open, not in this ticket:
  - `occurrence_key` still returns `Option<Option<NormalForm>>`. A small enum would name its three cases.

New terms: **Pairing** (new term), incoming Events paired by Occurrence with the stored Events with their UID (`Pairing`, `Paired`, `Loaded::pair`, `Loaded::write_paired`). **Kept** (new term), which recurrence ID an Event Row keeps when an incoming Event is written over it.

2026-11-07, follow-up agreed with the user: an Event that arrives in a Calendar already holding its UID gets a new UID. A move does this through the "all" and "alone" scopes (`core/src/event/scope.rs`); import's "copy" and "this and following" already gave a new UID. `Loaded::write_over` writes a moved Event over its stored version. If the target Calendar has any Event Row with the UID, an Exception without its Recurring Event too, it deletes the Event Row and creates it there under a new UID, and the Exceptions it carries follow. A moved lone Exception so stays an Event of its own instead of joining the target's Recurring Event. Undoing the move puts it back under its old ID and UID. Undo's refusal (`put_back`) stays, as an undo puts back an Event Version rather than bringing an Event in.

2026-11-07: the user approved the new terms **Pairing** and **Kept**, and confirmed that undo keeps refusing to put back an Event Version beside another Event with its UID.
