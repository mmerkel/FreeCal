# 35: Store Event times as wall time and Event Time Zone

**What to build:** The Local Store keeps a timed Event's start and end as wall-clock times, each with its own Event Time Zone, as decided in ADR 0007. UTC instants are derived from them for range queries and ordering, and recomputed when FreeCal ships new time zone rules. All-day Events stay dates. This replaces ticket 03's storage, where the UTC instant was the truth. Nothing changes in the UI. Spec: user stories 52–54, "Time zones and locale" under Implementation Decisions.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] A new migration stores, for each Event: start and end as wall time (`YYYY-MM-DDTHH:MM:SS`) or date, a start zone and an end zone (IANA names), and derived `start_utc` / `end_utc` as INTEGER Unix seconds. A `CHECK` makes timed Events have both zones and all-day Events none. The index for range queries uses the derived columns
- [ ] The migration converts existing rows: each UTC instant becomes the wall time in the row's `time_zone`, used for both start and end (core test: a Local Store with ticket 03's schema and rows opens with the same Occurrences)
- [ ] The Local Store records which time zone rules the UTC columns were computed with: the tzdata version `chrono-tz` was built with, or the `chrono-tz` version if it doesn't expose one. On opening, a different version recomputes every derived column in one transaction (core test: set an old version and wrong UTC values, reopen, values recomputed)
- [ ] A wall time the clocks skip is stored as entered; its UTC instant is the time shifted by the gap, as RFC 5545 says. An ambiguous one maps to the first instant (core tests for both, round-tripping the wall time unchanged)
- [ ] New Events get the Display Time Zone for start and end
- [ ] Editing keeps the Event Time Zone: the draft's times, given in the Display Time Zone, are converted to wall times in the Event's own start and end zones. An edit that leaves the time as shown keeps the stored wall times exactly (core test: an Event made in America/New_York, edited from Europe/Berlin, keeps America/New_York)
- [ ] Start and end in different zones are stored, queried and shown correctly (core test arranges such a row directly, since nothing creates one before tickets 06 and 10)
- [ ] Range queries and ordering read only the derived columns; tests across a DST change still pass
- [ ] `StoredWhen`'s doc comment and ticket 03's notes no longer describe UTC as the stored truth

The core interface doesn't change here: Occurrences and Events still cross as wall times in the Display Time Zone. Ticket 06 adds the Event Time Zone to `When` and the editor's zone choice. Floating times and the pinning rule (ADR 0007) come with the first ticket that brings them in, ticket 10 or 14, together with a mark that an Event's zone was pinned.

Out of scope: time zone names that aren't IANA names (tickets 10 and 14).
