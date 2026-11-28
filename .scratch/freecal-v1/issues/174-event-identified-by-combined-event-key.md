# 174: An Event is identified by its Combined Event Key

**What to build:** Old ticket 142 as written (`git show design/snapshot-line:.scratch/freecal-v1/issues/142-event-identified-by-combined-event-key.md`): the core interface and the window name an Event by its Combined Event Key, never by the Event Row's ID (ADR 0018, "Event Versions": a row ID changes with every edit). Nothing in the Local Store changes.

- Every operation of `Core` that names an Event or an Occurrence (reading, editing, rescheduling and deleting them, exception counts, the original server data, an Event's History, the Change Journal's listings and undo per Event) takes the key; `Event` and `Occurrence` carry it. One rule for every operation, the ones 182 deletes included, so that `EventId` leaves the generated types in one step.
- The key is branded in TypeScript and opaque to the window: no part of it is compared, parsed or shown there. One function derives FullCalendar's string ID from the key and the Occurrence's Normal Form. `src/test/ids.ts` makes keys.
- Inside the core the row ID stays (`pub(crate)`) until 178 makes it the Event Version's row ID.
- An Event that is gone answers as an unknown ID does today. A key whose Event was deleted and created again names the new Event.
- No change to what the user sees.

Spec: "Core application interface"; "Untrusted content". ADRs 0012, 0018. Layout: "Today's code, in brief" (`EventId`, Core interface); "By phase", P1, 174.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] No type of the core interface holds an Event Row's ID; `EventId` is gone from `src/core/types.generated.ts`
- [x] Every operation that names an Event or an Occurrence takes the key
- [x] The window holds keys only; one function derives FullCalendar's string ID
- [x] `core/tests/support/` helpers (`yoga()` among them) return keys; the tests through the core interface and the window's tests pass with keys, changed only mechanically
- [x] A test edits an Event twice through the key it got at creation

## Comments

- 2026-11-28: Started at commit `219b694` (opens P1).
- 2026-11-28: Done. The core interface, the shell and the window name Events by their Combined Event Key; the Event Row's ID is `pub(crate)` in `core/src/event/when.rs`. New tests: `an_event_is_edited_twice_through_the_key_it_got_at_creation` and `a_key_whose_event_was_deleted_and_made_again_names_the_new_event` (`core/tests/events.rs`), `an_exception_with_an_unreadable_recurrence_id_is_found_by_its_key` (`core/tests/recurrence_scopes.rs`), the key's string form with a hostile UID (`store/mod.rs` unit test), FullCalendar's ID telling Occurrences apart (`src/lib/gridDrag.test.ts`), and the details following the key an edit answers (`src/lib/windowDialogs.test.ts`).
  - **Decision, for review: a key's Calendar and recurrence ID are part of its identity, so some changes give an Event a new key.** Moving an Event to another Calendar answers a new key; the window now reopens the details with the key the edit answers (before, it reopened with the ID it had). An Exception whose recurrence ID the core rewrites (a sync joins it to its Recurring Event in another written form, or a change to the Recurring Event moves it with its Occurrence) gets a new key too; the old one still finds it where the Occurrence is the same by Normal Form, else answers "no such Event". Strongest con: a window holding an old key (an open details pane during a sync) loses its Event, where the row ID would also have changed (ADR 0018), but the user may see the details close.
  - **Decision, for review: the key is one opaque string on the wire**, the JSON array `[calendar, uid, recurrence ID as obtained]`, and the window compares whole keys with `===` only (marking the open Event, finding an Occurrence in view). Strongest con: the core parses a string the window hands back; it reads it as untrusted and refuses a malformed one.
  - **Decision, for review: a key whose recurrence ID can't be read finds the Exception stored with the same text**, so such an Exception (shown as an Event of its own) still opens. Strongest con: one narrow place compares recurrence IDs by text rather than Normal Form (`RecurrenceId::same_unreadable`).
  - **Decision, for review: undo per Event in the Change Journal's listings carries a key without a recurrence ID**, since that undo takes every row of the UID. Strongest con: the key there may not name the exact Exception that was changed; only its UID counts.
  - **Decision, for review: `ListedJournalEventModification` drops its Event ID instead of carrying a key**; the window never used it. `CoreError::EventNotFound` and `OccurrenceNotFound` drop their IDs. Strongest con: a later History feature that opens the Event from a listing has to add the key back.
  - Tests not changed only mechanically: four tests that edit again after moving an Event to another Calendar take the key the move answered (`change_journal.rs`, `recurrence_scopes.rs`); two `caldav_sync.rs` tests and one `recurring_events.rs` test asserted that an Exception keeps its row ID when its recurrence ID is rewritten, and now assert the old key still finds it, or drop that assertion where the Occurrence moved (see the first decision); two `events.rs` tests make their deleted Event before counting signals. `Unbranded` (`src/core/types.ts`) now unbrands branded strings for the samples.
  - Departures recorded in the layout plan, P1, 174. No switched-off tests. No new terms (`support::event_row` is a test helper).
