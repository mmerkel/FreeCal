# 95: Types for Combined Event Keys

**What to build:** A Combined Event Key travels as a type rather than as a tuple, agreed on 2026-11-04 (ticket 93 left it for later). Two types:

- `EventKeyInCalendar { uid, recurrence_id }`: an **Event Key in Calendar**, where a Server Object gives the Calendar, as when a sync hands an object's Events to the Mirror.
- `CombinedEventKey { calendar_id, event_key }`: a Calendar and an Event Key in Calendar, where nothing else gives the Calendar, as for an Unsent Change.

Writes to the Mirror can't then be given a key of another Calendar: the keys there have no Calendar of their own. A refactoring: behaviour stays the same. The table `combined_event_key` keeps its name: each row with its Server Object's Calendar is a full Combined Event Key (ADR 0011).

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `GLOSSARY.md`: an Internals entry for Event Key in Calendar; Combined Event Key says it is a Calendar and an Event Key in Calendar
- [x] The Mirror's writes (`JournalTransaction::mirror`, `rewrite_combined_event_keys`, `Store::write_object` and the writes under it) and the sync code that calls them take `EventKeyInCalendar`; no `(&str, Option<&str>)` tuple is left for a Combined Event Key
- [x] Adding an Unsent Change takes a `CombinedEventKey`; the tests' record of an Unsent Change is a `CombinedEventKey` and its `since`
- [x] Every existing test passes unchanged apart from names and types

## Comments

- 2026-11-04 (implemented): both types live in `core/src/store/mod.rs`, owned (`String`), so that one type serves both what a sync hands over and what the Local Store reads back. Parameters holding Event Keys in Calendar are `event_keys`; the sync's helper that makes them from a Server Object's Events is `event_keys_in_calendar` (`core/src/sync.rs`, was `combined_event_keys`). `Store::combined_event_keys`, a test reader, gives `EventKeyInCalendar`s. The tests' constant `DENTIST_COMBINED_EVENT_KEYS` is the function `dentist_event_keys()`. Behaviour unchanged; every test passes with only names and types changed. No new terms beyond Event Key in Calendar, agreed before the work.
