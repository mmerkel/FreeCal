# 54: Pull CalDAV Calendars into the Local Store

**What to build:** Calendars added from a CalDAV Account are pulled into the Local Store and kept up to date incrementally, through a Provider interface that later Providers (Google, Microsoft Graph, ICS feeds) plug into without changing the sync engine. Split from ticket 14.

**Blocked by:** 78, 69

**Status:** ready-for-agent

**Effort:** L

- [ ] A Provider interface that can take Microsoft Graph and ICS feeds later without changing the sync engine; CalDAV is its first implementation
- [ ] Adding a Calendar pulls it; a core operation syncs an Account on demand (timing and triggers come in ticket 16)
- [ ] Incremental fetching via `sync-collection` / ETags (tested against Radicale: create, change and delete on the server, then sync)
- [ ] The original server data of each Event is stored as received next to the fields FreeCal uses
- [ ] Each incremental sync of a Calendar is one Change Journal unit, its entries holding each Event's previous version, in Read-only Calendars too; the first sync is not recorded
- [ ] A Calendar's first sync commits page by page with one "Occurrences changed" per page, and the core reports that a first sync is running (ticket 55 shows it)
- [ ] Floating times are pinned by ADR 0007 (see comment) and the Event stores a mark that its zone was pinned
- [ ] Event fields from the server are treated as untrusted; Radicale serving the hostile test Event: it is pulled and stored without running anything

Out of scope: showing the first-sync state (ticket 55); read-only rules (ticket 56); Sync Status (ticket 15).

## Comments

- 2026-10-13: Split from ticket 14, which noted: floating times are pinned to the Calendar's `calendar-timezone` (RFC 4791) if it has one, otherwise to the Display Time Zone when the Event arrives, and the pinned zone is kept on later syncs. Ticket 50 shows a note for pinned zones; add the note here if 50 isn't done yet. Map time zone names that aren't IANA names as decided in ticket 57, or decide here if 54 comes first.
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-18 (from 77): the CalDAV client opens a new connection per request (`max_idle_connections(0)` in `core/src/caldav.rs`), because ureq reused connections that Radicale, an HTTP/1.0 server, had closed. Worth revisiting if pulling many Calendars is slow.
