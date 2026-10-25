# 85: Event IDs are never reused

**What to build:** An Event's ID names one Event for good: SQLite gives a new Event the ID of a deleted one when that one had the highest ID, because `event.id` is an `INTEGER PRIMARY KEY` without `AUTOINCREMENT`. The Change Journal tells Events apart by ID and UID (`Entry.uid` in `core/src/journal.rs`), which fails for Exceptions: they share their Recurring Event's UID. Create an Exception, delete it, change another Occurrence, and the new Exception gets the deleted one's ID and the same UID. The Journal then takes both for one Event:

- Today: undoing the delete is refused with `ChangedSinceUnit`, since the row at that ID and UID is the other Exception. With a fresh ID it would be put back.
- Ticket 72: undoing an entry first undoes "the later entries for the same Events", which would undo the change to the other Occurrence too, and ask the user about it as a later change to the same Event.

Spec: "Change Journal" under Implementation Decisions; ADR 0009.

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `event.id` is `INTEGER PRIMARY KEY AUTOINCREMENT`, in a migration that rebuilds the `event` table with its rows, IDs and indexes, and starts the sequence above the highest ID the Change Journal names, so that no ID an entry holds is given out again
- [x] Undo still puts an Event back under its own ID (`Unit::put_back`); with no ID taken by another Event, its branch for a taken ID goes, and `Entry.uid`'s comment no longer says the UID tells Events apart
- [x] Core test: an Exception created, then deleted, then another Occurrence changed; the new Exception has another ID, and undoing the delete puts the first one back
- [x] Core test: after the migration, a new Event never gets an ID that a Change Journal entry holds, also when the Event with the highest ID was deleted before migrating

Out of scope: per-Event undo and cascading (ticket 72); squashing migrations (ticket 63, which takes this schema into its baseline).

## Comments

- 2026-10-24: Found while going through the IDs FreeCal uses: `(event_id, uid)` is unique for Events of different UIDs but not for Exceptions of one Recurring Event. Keying on the recurrence ID instead was rejected: "this and following" changes an Exception's recurrence ID, and a split makes new rows.
