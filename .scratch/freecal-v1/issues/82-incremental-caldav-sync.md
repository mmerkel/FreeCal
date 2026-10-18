# 82: Incremental sync of a CalDAV Account

**What to build:** A core operation syncs a CalDAV Account on demand, fetching only what changed since the last sync of each Calendar, and records each Calendar's sync as one Change Journal unit. Timing and triggers are ticket 16. Split from ticket 54.

**Blocked by:** 81

**Status:** ready-for-agent

**Effort:** M

- [ ] A core operation syncs an Account on demand: every Calendar of it, through the Provider interface from ticket 80
- [ ] Incremental fetching via `sync-collection` / ETags; the sync state is kept per Calendar. Tested against Radicale: create, change and delete on the server, then sync
- [ ] Each incremental sync of a Calendar is one Change Journal unit of a new origin sync, its entries holding each Event's previous version, in Read-only Calendars too (the module lets origin sync write there); a sync that changes nothing records no unit
- [ ] One "Occurrences changed" per Calendar, when its changes are committed
- [ ] A new version whose start and end both still float keeps the zone pinned before (ADR 0007 rule 2, `StoredWhen::in_zones_of`); one floating end gets the other's zone (rule 3)
- [ ] Undoing a sync unit restores the previous versions locally (writing an undo to the server is ticket 17)

Out of scope: timing and triggers (ticket 16); Sync Status (ticket 15); read-only rules (ticket 56); Conflicts (ticket 20).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session.
- 2026-10-18 (from 80): a first sync that fails leaves its Calendar with the Events of the pages committed so far, and the Calendars after it with none; `add_calendars` returns the error but keeps them. Nothing stores that a first sync didn't finish (`Calendar.first_sync` is in memory only), so the sync state kept per Calendar must tell "never fully synced" apart, and such a Calendar needs a full fetch, not an incremental one. Pages pair Exceptions with their Recurring Event within one page only; RFC 4791 keeps one UID in one resource. The Provider interface is `Remote` in `core/src/remote.rs`.
- 2026-10-18 (from 81): a sync writes `OriginalServerData` (href, ETag, object) on each Recurring Event's or single Event's row through `with_server_data` in `journal.rs`, which needs the sync origin added there like `FirstSync` (as given). Note that undoing an older unit puts back that version's ETag and object too, which may be older than what the server now holds.
