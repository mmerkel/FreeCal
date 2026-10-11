# 35: Store Event times as wall time and Event Time Zone

**What to build:** The Local Store keeps a timed Event's start and end as wall-clock times, each with its own Event Time Zone, as decided in ADR 0007. UTC instants are derived from them for range queries and ordering, and recomputed when FreeCal ships new time zone rules. All-day Events stay dates. This replaces ticket 03's storage, where the UTC instant was the truth. Nothing changes in the UI. Spec: user stories 52–54, "Time zones and locale" under Implementation Decisions.

**Blocked by:** 03

**Status:** done

- [x] A new migration stores, for each Event: start and end as wall time (`YYYY-MM-DDTHH:MM:SS`) or date, a start zone and an end zone (IANA names), and derived `start_utc` / `end_utc` as INTEGER Unix seconds. A `CHECK` makes timed Events have both zones and all-day Events none. The index for range queries uses the derived columns
- [x] The migration converts existing rows: each UTC instant becomes the wall time in the row's `time_zone`, used for both start and end (core test: a Local Store with ticket 03's schema and rows opens with the same Occurrences)
- [x] The Local Store records which time zone rules the UTC columns were computed with: the tzdata version `chrono-tz` was built with, or the `chrono-tz` version if it doesn't expose one. On opening, a different version recomputes every derived column in one transaction (core test: set an old version and wrong UTC values, reopen, values recomputed)
- [x] A wall time the clocks skip is stored as entered; its UTC instant is the time shifted by the gap, as RFC 5545 says. An ambiguous one maps to the first instant (core tests for both, round-tripping the wall time unchanged)
- [x] New Events get the Display Time Zone for start and end
- [x] Editing keeps the Event Time Zone: the draft's times, given in the Display Time Zone, are converted to wall times in the Event's own start and end zones. An edit that leaves the time as shown keeps the stored wall times exactly (core test: an Event made in America/New_York, edited from Europe/Berlin, keeps America/New_York)
- [x] Start and end in different zones are stored, queried and shown correctly (core test arranges such a row directly, since nothing creates one before tickets 06 and 10)
- [x] Range queries and ordering read only the derived columns; tests across a DST change still pass
- [x] `StoredWhen`'s doc comment and ticket 03's notes no longer describe UTC as the stored truth

The core interface doesn't change here: Occurrences and Events still cross as wall times in the Display Time Zone. Ticket 06 adds the Event Time Zone to `When` and the editor's zone choice. Floating times and the pinning rule (ADR 0007) come with the first ticket that brings them in, ticket 10 or 14, together with a mark that an Event's zone was pinned.

Out of scope: time zone names that aren't IANA names (tickets 10 and 14).

## Comments

- 2026-10-11: Done. Notes for later tickets:
  - **Schema** (migration 6, a Rust migration: `Migration::Code` in `core/src/store.rs`): `event.start_at` / `end_at` hold wall times or dates, `start_zone` / `end_zone` the IANA names, `start_utc` / `end_utc` the derived Unix seconds, indexed as `(calendar_id, start_utc)`. All-day Events' derived columns are their dates as midnight UTC, so the range query compares them with the range's dates. `app_state.time_zone_rules` holds `chrono_tz::IANA_TZDB_VERSION`; `Store::open` recomputes every derived column inside the migration transaction when it differs. A row it can't read keeps its old values.
  - **Conversions** live in `StoredWhen` (`core/src/event.rs`): `new` (Display Time Zone, wall times as entered), `edit` (keeps the Event Time Zones; a start or end left as shown keeps its stored wall time), `to_when` (wall time → instant → Display Time Zone) and `instants`. A gap time's instant uses the offset from before the gap, so it is shifted by the gap's length, not a fixed hour; it is still shown shifted, as under ticket 03.
  - **Tests**: `core/tests/event_times.rs` for what the interface shows. What it can't show yet (the stored wall text of a skipped time, that an edit from Berlin keeps America/New_York, that new Events get the Display Time Zone) is covered by unit tests in `core/src/event.rs`; they can move to the interface once ticket 06 adds the zone to `When`.
  - **Ordering**: Occurrences sort by their start date in the Display Time Zone, all-day first, then by `start_utc`, so the repeated hour sorts in the order things happen.
  - **Migrating hostile rows**: a ticket 03 row with an unknown or missing zone becomes `UTC` with the same instant; a row whose values can't be read keeps its text and gets derived columns of 0, so it shows in no range and reads as unreadable. Ticket 10/14's pinned-zone mark would be the place to record such a replacement if it matters.
  - Test helpers `date`, `time`, `timed`, `all_day` and `draft` moved into `core/tests/support/mod.rs`.
