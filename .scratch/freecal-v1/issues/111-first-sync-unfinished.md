# 111: A Calendar's unfinished First Sync shows, and a failed First Sync leaves the rest as they were

**What to build:** Adding Calendars runs their First Syncs in a loop of its own (`Core::add_calendars`, `core/src/lib.rs`), separate from the one that syncs an Account (`Core::sync_account`), and its copy gets two rules wrong:

- When one First Sync fails, the Calendars added with it whose First Sync hasn't run are ended as if they had synced. They get a Synced Sync Record with last successful sync = now, which also moves the overall last sync.
- A failure about one Calendar becomes a Sync Error of the Account. The spec keeps a Calendar-level state on the Calendar, and `sync_account` already filters with `sync_status::about_account`.

Also, a Calendar whose First Sync is unfinished can look empty without any hint. That happens after the failure above, after quitting during a First Sync (on relaunch it has no Sync Record, so it shows Synced), and when a First Sync fails partway (it shows only its Sync Error).

- **One loop:** adding Calendars and syncing an Account share one loop, "sync these Calendars of an Account, started by X". It ends each Calendar's sync and the Account's by the same rules. Adding Calendars still marks every added Calendar as syncing its First Sync from when it is added, through an input that only it uses.
- **Calendars a sync didn't reach** keep their Sync Status as it was before the sync, whoever started it. For one that has never synced, that is Synced with no last sync. The next sync runs the rest of their First Syncs.
- **First Sync unfinished:** each Calendar's Sync Status report carries whether its First Sync is unfinished. The core reads it from the Local Store (the Calendar row's First Sync flag) and never keeps it in memory. It replaces the `first_sync` marker on Syncing (`SyncStatus::Syncing`, `core/src/sync_status.rs`), which goes, so the core no longer tracks First Syncs while they run. Accounts have no such flag.
- **Sidebar:** a Calendar with its First Sync unfinished shows "Loading for the first time…" while it syncs, "Not loaded yet" while it waits for the next sync (also when its Account is Offline), and its Sync Error's usual text followed by " · not fully loaded" after one. A Calendar with nothing loaded and one with some pages loaded look the same.
- **Overall status and tray:** they ignore the flag. A Calendar that never synced successfully already makes the overall last sync unknown, and that stays.
- Regenerate the TypeScript types after the interface change (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`).

Spec: US-68; US-69; "Signals" ("Sync status changed" and the First Sync paragraph); the UI section's sidebar line (updated 2026-11-07).

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] When one of several Calendars being added fails its First Sync, the Calendars after it keep their Sync Status as before (Synced, no last sync) with their First Sync unfinished, and the next sync of the Account finishes them
- [ ] A Calendar-level failure while adding Calendars is that Calendar's Sync Error, not the Account's; a failure about the Account is the Account's, as in an Account sync
- [ ] Adding Calendars and syncing an Account go through one loop; each added Calendar shows as syncing its First Sync from when it is added
- [ ] Each Calendar's "Sync status changed" and `sync_statuses()` say whether its First Sync is unfinished: while it syncs, after a failure, and after a relaunch during a First Sync
- [ ] `SyncStatus::Syncing` has no First Sync marker any more
- [ ] The sidebar shows "Loading for the first time…", "Not loaded yet" and "<Sync Error> · not fully loaded" as above (Vitest with the fake core)
- [ ] The overall status and the tray don't change for a Calendar with its First Sync unfinished
