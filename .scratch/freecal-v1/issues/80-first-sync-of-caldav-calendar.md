# 80: First sync of a CalDAV Calendar

**What to build:** Adding a CalDAV Calendar pulls its Events into the Local Store, through a Provider interface that later Providers (Google, Microsoft Graph, ICS feeds) plug into without changing the sync engine. This is the first sync only: the baseline, not recorded in the Change Journal. Incremental syncs are ticket 82; storing the original server data is ticket 81. Split from ticket 54.

**Blocked by:** 78, 69

**Status:** ready-for-agent

**Effort:** M

- [ ] A Provider interface that can take Microsoft Graph and ICS feeds later without changing the sync engine; CalDAV is its first implementation. It is internal, not a test seam: tests run against Radicale
- [ ] Adding a Calendar (`Core::add_calendars`) pulls its Events
- [ ] The first sync commits page by page with one "Occurrences changed" per page, and the core reports that a first sync is running (ticket 55 shows it)
- [ ] The first sync goes through the Change Journal module (the only writer of Events, ADR 0009) as a new origin that records nothing (ADR 0009: the first sync is the baseline)
- [ ] Events are read with ticket 57's iCalendar reading, time zone mapping included; floating times are pinned by ADR 0007 to the Calendar's `calendar-timezone` (RFC 4791) if it has one, otherwise to the Display Time Zone, and store the pinned mark (`event.zone_pinned`, shown since ticket 58)
- [ ] Event fields from the server are treated as untrusted; Radicale serving the hostile test Event (`fixtures/hostile-event.json`): it is pulled and stored without running anything

Out of scope: the original server data (ticket 81); syncing an Account on demand, incremental fetching and recording syncs (ticket 82); showing the first-sync state (ticket 55); read-only rules (ticket 56); Sync Status (ticket 15).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session. From 54's comments: the CalDAV client opens a new connection per request (`max_idle_connections(0)` in `core/src/caldav.rs`), because ureq reused connections that Radicale, an HTTP/1.0 server, had closed; worth revisiting if pulling many Calendars is slow. A server Calendar's address is `calendar.url` (`Store::calendar_urls`); only URLs the server offered at the time of adding get there.
