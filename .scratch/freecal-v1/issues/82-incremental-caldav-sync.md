# 82: Incremental sync of a CalDAV Account

**What to build:** A core operation syncs a CalDAV Account on demand, fetching only what changed since the last sync of each Calendar, and records each Calendar's sync as one Change Journal unit. Timing and triggers are ticket 16. Split from ticket 54.

**Blocked by:** 81

**Status:** done

**Effort:** M

- [x] A core operation syncs an Account on demand: every Calendar of it, through the Provider interface from ticket 80
- [x] Incremental fetching via `sync-collection` / ETags; the sync state is kept per Calendar. Tested against Radicale: create, change and delete on the server, then sync
- [x] Each incremental sync of a Calendar is one Change Journal unit of a new origin sync, its entries holding each Event's previous version, in Read-only Calendars too (the module lets origin sync write there); a sync that changes nothing records no unit
- [x] One "Occurrences changed" per Calendar, when its changes are committed
- [x] A new version whose start and end both still float keeps the zone pinned before (ADR 0007 rule 2, `StoredWhen::in_zones_of`); one floating end gets the other's zone (rule 3)
- [x] Undoing a sync unit restores the previous versions locally (writing an undo to the server is ticket 17)

Out of scope: timing and triggers (ticket 16); Sync Status (ticket 15); read-only rules (ticket 56); Conflicts (ticket 20).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session.
- 2026-10-18 (from 80): a first sync that fails leaves its Calendar with the Events of the pages committed so far, and the Calendars after it with none; `add_calendars` returns the error but keeps them. Nothing stores that a first sync didn't finish (`Calendar.first_sync` is in memory only), so the sync state kept per Calendar must tell "never fully synced" apart, and such a Calendar needs a full fetch, not an incremental one. Pages pair Exceptions with their Recurring Event within one page only; RFC 4791 keeps one UID in one resource. The Provider interface is `Remote` in `core/src/remote.rs`.
- 2026-10-18 (from 81): a sync writes `OriginalServerData` (href, ETag, object) on each Recurring Event's or single Event's row through `with_server_data` in `journal.rs`, which needs the sync origin added there like `FirstSync` (as given). Note that undoing an older unit puts back that version's ETag and object too, which may be older than what the server now holds.
- 2026-10-18: Done. `Core::sync_account` syncs each Calendar of a server Account through `Remote::changes` and returns each Calendar with its unit; should one fail, the others still sync and the first error is returned. The sync state is `calendar.synced` (its first sync finished) and `calendar.sync_token`, as `SyncPoint`. CalDAV asks `sync-collection` (RFC 6578) with the token; without one, or when the server refuses it, it lists the whole Calendar with ETags (`PROPFIND`), and an object FreeCal holds that isn't listed counts as removed. Only objects whose ETag differs are fetched (`calendar-multiget`). `sync::apply` writes the changes as one unit of `Origin::Sync`, matching Events by UID, so an object moved to another address stays the same Event. A Calendar whose first sync didn't finish gets the rest of it as `Origin::FirstSync`, unrecorded, with `Calendar.first_sync` set while it runs. An object FreeCal can no longer read leaves its Event as it was. Radicale drops the part of an unescaped `SUMMARY` after a comma and refuses a PUT that changes an object's UID (409), so the tests avoid both.
- 2026-10-18: Left to other tickets: undoing a sync unit in a Read-only Calendar is still refused, as for every undo there (ticket 56: undo skips their Events, ADR 0009). A whole listing that misses objects would delete their Events locally, which the Mass-Change Guard should stop (ticket 22). A truncated `sync-collection` answer (507) is not followed up in the same sync; the next sync gets the rest. A server that gives no sync token is only covered through the refused-token path.
