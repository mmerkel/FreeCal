# 83: An Event module in the core: loading a Recurring Event and its operations

**What to build:** A deep `core/src/event/` module that loads any Event as one value, a Recurring Event with its rule and Exceptions included, and owns the operations that today rebuild that value at each call site in `lib.rs`, `sync.rs` and `import.rs`. A refactor: behaviour stays the same except for the recurrence ID fix below. Decided in an architecture review and its grilling on 2026-10-24; ticket 84 finishes the module. Spec: "Untrusted content"; ADR 0007, ADR 0009.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

`event.rs` becomes the directory `event/`:

- `event/when.rs`: today's `event.rs` (`StoredWhen`), and `EventFields` moved from `store.rs`
- `event/rule.rs`: today's `recurrence.rs`, a leaf that depends only on `event/when.rs`. Outside the module it offers only the span of an Event's Occurrences (what `EventFields::instants` gives today, for `journal.rs` and `store.rs`) and reading an imported rule with its `EXDATE`s, both normalised; everything else in it is private to the module. `Repeat`, `Frequency` and `RepeatEnd` stay re-exported from `lib.rs` under the same names
- `event/mod.rs`: the loaded value (a name such as `event::Loaded`; not `Event`, the window's type), loading and the operations

Steps, each with the suite green:

- [ ] Move `EventFields` and `recurrence.rs` as above. `journal.rs` and `store.rs` call the rule leaf for the span, so no module depends back on its caller
- [x] One Store query, every Event of a Calendar with a UID, replaces `find_event_by_uid`, `exceptions` and `event_uid` and their `Unit` pass-throughs (`event_at` stays). The module sorts the rows into the Recurring Event, its Exceptions and orphaned Exceptions (each an Event of its own, as GLOSSARY says), keeping "the first by ID" when a Calendar has several Recurring Events with one UID
- [x] Loading from an Event (what the window gives: the Recurring Event or one of its Exceptions, plus a recurrence ID) and from a Calendar and UID (what sync and import give). Loading from &Store serves reads; changes write through a `journal::Unit`, never the Store, so ADR 0009's recording and the read-only check stay in `journal.rs`
- [x] Move into the module: the Occurrences in a range (`Core::occurrences`), deleting a whole Recurring Event (`Core::delete_event`, `sync::delete_with_exceptions`), cancelling one Occurrence (`Core::delete_occurrence`), and the window's `Event` as a projection of the loaded value in the Display Time Zone (`Core::to_event`, with an Exception's repeat from its Recurring Event)
- [x] Move `ImportedEvent::create_in` / `update_in` into the module as taking an incoming version, an `ImportedEvent` with its Exceptions, used by import and sync
- [x] The module is the only place that compares recurrence IDs, always normalised. Today `update_in` (`import.rs:204`) compares the raw text while `delete_occurrence` and `scope::edit` normalise: an incoming Exception whose `RECURRENCE-ID` is written differently (with seconds, in another form of the same time) is added next to the stored one, which is deleted, instead of replacing it. A Core test for import and one for sync, failing first
- [x] Tests stay at the `Core` interface: the Recurring Event tests (`recurring_events.rs`, `recurrence_scopes.rs`) pass unchanged; the recurrence ID tests above are the only new ones. In-module tests are allowed only for `event/rule.rs`, where a table of rule cases is clearly easier without a Store
- [x] `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` gives no diff

Out of scope: `scope.rs` and the single-Event create, edit and reschedule paths (ticket 84).

## Comments

**2026-10-24, implementation.** All but the first box are done; `cargo test`, clippy and the generated types are clean, and the two recurrence ID tests (`recurring_events.rs`, `caldav_sync.rs`) failed first.

- Box 1 is partly done: `EventFields` (with `OriginalServerData`) is in `event/when.rs`, `recurrence.rs` is `event/rule.rs`, and `journal.rs` and `store.rs` take the span from `rule::span`. What is left: the rule functions `scope.rs` and the single-Event paths in `lib.rs` use (`rule_text`, `repeat_of`, `occurrence_at`, `recurrence_id`, `with_exdate` and the scope helpers) are still open to the crate, as the header of `rule.rs` says. Ticket 84 finishes this once it moves those callers into the module.
- Until then `scope.rs` reaches the loaded value through `Loaded::into_parts` and `Opened::into_loaded`, and still pairs Exceptions by recurrence ID itself (`is_rid`). Ticket 84 removes all three.
- Behaviour changes beyond the recurrence ID fix, both on inconsistent data, both following GLOSSARY ("An Exception whose Recurring Event is gone is an Event of its own"):
  - Deleting an Exception whose Recurring Event is gone (the UID's first Event that isn't an Exception has no rule) deletes only it, not also that Event and every Exception.
  - Cancelling an Occurrence without a Recurring Event deletes only the Event given, not also Exceptions of its UID with that recurrence ID.

