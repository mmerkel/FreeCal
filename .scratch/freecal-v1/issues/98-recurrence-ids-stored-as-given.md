# 98: Recurrence IDs stored as given, compared by Occurrence

**What to build:** Agreed on 2026-11-06 (ADR 0012). Every Event Row, Unsent Change and Server Object keeps its recurrence ID as it was given, and whether two recurrence IDs mean the same Occurrence is worked out when comparing, against the Recurring Event the Event Rows hold. Nothing renormalises recurrence IDs any more. What the user sees stays the same.

- A `RecurrenceId` type holds a recurrence ID as given. Its only comparison takes the Recurring Event (or its absence), so that no code compares recurrence IDs as text; it has no `PartialEq`.
- A Loaded Event computes each Exception's normal form once, against its Recurring Event (`Exception.rid`, `Loaded::sorted`, `core/src/event/mod.rs`): the Occurrence's wall time in the Recurring Event's Event Time Zone, or its date when the Recurring Event is all-day. Without a Recurring Event, the recurrence ID counts as given (ticket 88).
- Every place that pairs an Exception with an Occurrence or another Exception compares through it: expanding Occurrences (`occurrences_in`), `Loaded::exception_at` and `Loaded::cancel`, Propagate (`take_object`) pairing incoming Exceptions with stored ones, and imports.
- An Occurrence's ID in the core interface is the computed normal form; the edit and delete commands (`core/src/lib.rs`) find the stored Exception by comparing by Occurrence.
- Renormalising goes: `Loaded::join` no longer rewrites Event Rows, and `JournalTransaction::rejoin_combined_event_keys`, `rejoined_id` (`core/src/import/mod.rs`) and the writes that set a stored recurrence ID in `combined_event_key` and `unsent_change` (`core/src/store/sql/journal.rs`) are removed. The Combined Event Keys the Sync Engine writes from a Server Object's data hold its recurrence IDs as given.
- FreeCal writes the recurrence IDs of Exceptions it makes in the normal form, and moving a Recurring Event's time still rewrites its Exceptions' recurrence IDs (`core/src/event/scope.rs`).

**Blocked by:** none

**Status:** done

**Effort:** L

- [x] `RecurrenceId` exists without `PartialEq`; recurrence IDs are compared only through its comparison against the Recurring Event
- [x] An Exception that arrived without its Recurring Event, its recurrence ID given in another zone, changes its Occurrence once the Recurring Event arrives, in the same or a later Incoming Sync, and in another Server Object; its stored recurrence ID stays as given
- [x] Undoing the Incoming Sync that brought the Recurring Event leaves the Exception an Event of its own again, with nothing in `unsent_change` in any other form
- [x] Importing a Recurring Event whose Exceptions came alone from the server leaves no Unsent Change in a form the Event Rows don't hold
- [x] A floating recurrence ID, a date for a timed Recurring Event and a time for an all-day Recurring Event each pair as ticket 88 decided
- [x] `rejoin_combined_event_keys`, `rejoined_id` and the stored recurrence ID updates are gone
- [x] Editing and deleting an Occurrence by the ID the interface gives finds the Exception stored in another form

## Comments

- 2026-11-06: Done. `RecurrenceId` (`core/src/event/recurrence_id.rs`) holds a recurrence ID as given, has no `PartialEq`, and is compared through `same_occurrence` / `normal_form`, both against the Recurring Event. Decisions made along the way:
  - Imports store every Exception's recurrence ID as given too, also when the file holds its Recurring Event (`2026-10-12T09:00:00[Europe/Berlin]`). One rule for every source.
  - Replacing a stored Exception for the same Occurrence: an import keeps the stored recurrence ID, so it leaves no Unsent Change in a form the Server Object doesn't hold. Propagate (`event::take_object`) gives the Event Row the Server Object's recurrence ID as given, as its Combined Event Keys hold it. Otherwise, after `settle`, an Exception alone could hold a form its Server Object doesn't.
  - Propagate pairs against the Recurring Event the incoming Server Object holds, falling back to the one the Event Rows hold. The ticket's "the Recurring Event the Event Rows hold" alone would miss a Recurring Event arriving in the same Server Object as its Exceptions.
  - The interface's `Event.recurrence_id` is the normal form too, like the Occurrence's ID (as stored when it isn't a recurrence ID).
  - Changing a Recurring Event with Scope "all" or "this and following" keeps an Exception's stored recurrence ID while it names the same Occurrence. It rewrites the ID only when the Occurrence moves.
  - New terms, for approval: **normal form** (new term), an Occurrence's wall time in the Recurring Event's Event Time Zone, or its date when that is all-day, computed in memory only (from ADR 0012). **Recurrence ID as given** (new term), a recurrence ID in the text it arrived in (`RecurrenceId`). **Original start** (new term), an Occurrence's start as `RECURRENCE-ID` gives it (`RecurrenceId::of_original`). **Occurrence key** (new term), which Occurrence an incoming Event stands for when Propagate pairs it with Event Rows (`occurrence_key`).
- 2026-11-06 (review): "normal form" approved as **Normal Form**, "recurrence ID as given" renamed **Recurrence ID as obtained**; both are in GLOSSARY's Internals, and the code says `RecurrenceId::obtained` / `as_obtained`. A `NormalForm` type now holds the Normal Form, so normal forms no longer travel as plain `String`s. Fixed from the review: an edit with Scope "all" that didn't move the time rewrote Exceptions' recurrence IDs into the Normal Form; and `take_object` swallowed a Local Store error.
