# 80: First sync of a CalDAV Calendar

**What to build:** Adding a CalDAV Calendar pulls its Events into the Local Store, through a Provider interface that later Providers (Google, Microsoft Graph, ICS feeds) plug into without changing the sync engine. This is the first sync only: the baseline, not recorded in the Change Journal. Incremental syncs are ticket 82; storing the original server data is ticket 81. Split from ticket 54.

**Blocked by:** 78, 69

**Status:** done

**Effort:** M

- [x] A Provider interface that can take Microsoft Graph and ICS feeds later without changing the sync engine; CalDAV is its first implementation. It is internal, not a test seam: tests run against Radicale
- [x] Adding a Calendar (`Core::add_calendars`) pulls its Events
- [x] The first sync commits page by page with one "Occurrences changed" per page, and the core reports that a first sync is running (ticket 55 shows it)
- [x] The first sync goes through the Change Journal module (the only writer of Events, ADR 0009) as a new origin that records nothing (ADR 0009: the first sync is the baseline)
- [x] Events are read with ticket 57's iCalendar reading, time zone mapping included; floating times are pinned by ADR 0007 to the Calendar's `calendar-timezone` (RFC 4791) if it has one, otherwise to the Display Time Zone, and store the pinned mark (`event.zone_pinned`, shown since ticket 58)
- [x] Event fields from the server are treated as untrusted; Radicale serving the hostile test Event (`fixtures/hostile-event.json`): it is pulled and stored without running anything

Out of scope: the original server data (ticket 81); syncing an Account on demand, incremental fetching and recording syncs (ticket 82); showing the first-sync state (ticket 55); read-only rules (ticket 56); Sync Status (ticket 15).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session. From 54's comments: the CalDAV client opens a new connection per request (`max_idle_connections(0)` in `core/src/caldav.rs`), because ureq reused connections that Radicale, an HTTP/1.0 server, had closed; worth revisiting if pulling many Calendars is slow. A server Calendar's address is `calendar.url` (`Store::calendar_urls`); only URLs the server offered at the time of adding get there.
- 2026-10-18: Done. The Provider interface is `Remote` in `core/src/remote.rs` (`discover`, `all_events` paged by `FIRST_SYNC_PAGE` = 50, `connect` per Provider); CalDAV lists a Calendar's members with `PROPFIND` and fetches them with `calendar-multiget`. The first sync writes through `journal::record` with `Origin::FirstSync`, which records nothing and may write into Read-only Calendars. "First sync running" is `Calendar.first_sync` (in memory, not stored), with "Calendars changed" as it starts and ends; `add_calendars` blocks until every first sync is done. CalDAV's XML turns a CR LF inside a field into LF (XML 1.0, 2.11), so the hostile title's `\r\n` arrives as `\n`.
