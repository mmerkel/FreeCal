# 98: Recurrence IDs stored as given, compared by Occurrence

**What to build:** Agreed on 2026-11-06 (ADR 0012). Every Event Row, Unsent Change and Server Object keeps its recurrence ID as it was given, and whether two recurrence IDs mean the same Occurrence is worked out when comparing, against the Recurring Event the Event Rows hold. Nothing renormalises recurrence IDs any more. What the user sees stays the same.

- A `RecurrenceId` type holds a recurrence ID as given. Its only comparison takes the Recurring Event (or its absence), so that no code compares recurrence IDs as text; it has no `PartialEq`.
- A Loaded Event computes each Exception's normal form once, against its Recurring Event (`Exception.rid`, `Loaded::sorted`, `core/src/event/mod.rs`): the Occurrence's wall time in the Recurring Event's Event Time Zone, or its date when the Recurring Event is all-day. Without a Recurring Event, the recurrence ID counts as given (ticket 88).
- Every place that pairs an Exception with an Occurrence or another Exception compares through it: expanding Occurrences (`occurrences_in`), `Loaded::exception_at` and `Loaded::cancel`, Propagate (`take_object`) pairing incoming Exceptions with stored ones, and imports.
- An Occurrence's ID in the core interface is the computed normal form; the edit and delete commands (`core/src/lib.rs`) find the stored Exception by comparing by Occurrence.
- Renormalising goes: `Loaded::join` no longer rewrites Event Rows, and `JournalTransaction::rejoin_combined_event_keys`, `rejoined_id` (`core/src/import/mod.rs`) and the writes that set a stored recurrence ID in `combined_event_key` and `unsent_change` (`core/src/store/sql/journal.rs`) are removed. The Combined Event Keys the Sync Engine writes from a Server Object's data hold its recurrence IDs as given.
- FreeCal writes the recurrence IDs of Exceptions it makes in the normal form, and moving a Recurring Event's time still rewrites its Exceptions' recurrence IDs (`core/src/event/scope.rs`).

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** L

- [ ] `RecurrenceId` exists without `PartialEq`; recurrence IDs are compared only through its comparison against the Recurring Event
- [ ] An Exception that arrived without its Recurring Event, its recurrence ID given in another zone, changes its Occurrence once the Recurring Event arrives, in the same or a later Incoming Sync, and in another Server Object; its stored recurrence ID stays as given
- [ ] Undoing the Incoming Sync that brought the Recurring Event leaves the Exception an Event of its own again, with nothing in `unsent_change` in any other form
- [ ] Importing a Recurring Event whose Exceptions came alone from the server leaves no Unsent Change in a form the Event Rows don't hold
- [ ] A floating recurrence ID, a date for a timed Recurring Event and a time for an all-day Recurring Event each pair as ticket 88 decided
- [ ] `rejoin_combined_event_keys`, `rejoined_id` and the stored recurrence ID updates are gone
- [ ] Editing and deleting an Occurrence by the ID the interface gives finds the Exception stored in another form
