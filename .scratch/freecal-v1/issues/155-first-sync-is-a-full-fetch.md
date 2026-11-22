# 155: A server Calendar is stored empty when added, and its First Sync is a Full Fetch like any Incoming Sync

**What to build:** The First Sync as ADR 0018 decides it since 2026-11-22, in the core: adding an Available Calendar stores the Calendar at once, with no Event Versions, sync token or ETags, and its First Sync is its first Incoming Sync, a Full Fetch (ADR 0019) that takes in everything as one Incoming Sync Snapshot in one Database Transaction. The First Sync's own write, its stored state "First Sync unfinished" (tickets 80 and 111) and the page-by-page rest of an unfinished one come out. This is step 4 of the order in which the old model goes. The dialog that chooses the Calendars is ticket 156.

- **Adding Available Calendars** (`Core::add_calendars`, `core/src/lib.rs`) stores each chosen Calendar, sends "Calendars changed" once, and asks the Sync Scheduler for their Incoming Syncs. It doesn't wait for them.
- **The Full Fetch** is the Incoming Sync's path when a Calendar has no sync token, or its server gives no ETags (CalDAV: the member listing and the multiget in pages, `core/src/caldav.rs`): every resource is fetched into memory and compared with the versions carrying the server's mark, through the same comparison as any Incoming Sync (ticket 167). With no marks, all of it is "only the server changed". The CalDAV table's rows, their ETags, the Ignored Resources and the sync token are written in the same Database Transaction. A sync-token reset by the server takes the same path. Nothing is written page by page.
- **Interrupted** by the network or by FreeCal closing, a Full Fetch leaves nothing behind, and the next Incoming Sync starts it again from the beginning, as any Incoming Sync that failed.
- **"Not loaded yet"** is read, not stored: the Sync Status (ticket 15) reports a Calendar's last finished sync, and none means its First Sync hasn't finished. The core interface exposes that; the window's texts are ticket 161's.
- **Editing a Calendar still loading** is allowed like any edit. The Full Fetch's comparison finds such an Event on the user's side only, and the next Outgoing Sync sends it.
- **Cancelling** is removing the Calendar from FreeCal; the running sync ends as ticket 112's rule for a Calendar removed meanwhile.
- **An Undo of the first Incoming Sync Snapshot** is no special case: the Mass-Change Guard counts it like every Undo (ADR 0020).
- **What comes out:** the Calendar row's First Sync flag and `first_sync_unfinished` in the Sync Status report (`core/src/sync_status.rs`, `core/src/store/sql/mod.rs`); the First Sync's own write (`first_sync`, `core/src/sync/caldav/incoming.rs`) and the paged rest of an unfinished one; in the window, the sidebar's texts "Loading for the first time…", "Not loaded yet" and "· not fully loaded" as they are made today (`src/lib/Sidebar.svelte`; ticket 161 brings them back read from the Sync Status) and the Unsent Mark's reason for an unfinished First Sync (`src/lib/unsent.ts`).
- **The window until ticket 156:** "Manage Calendars…" returns once the Calendars are stored, and they fill in as their First Syncs finish.
- **The Store Check** gains nothing.
- **Docs:** `GLOSSARY.md`'s First Sync and Full Fetch say it already (2026-11-22); Sync Status and Available Calendar are checked for an unfinished First Sync.

Spec: US-5, US-152; "Signals"; "Sync engine"; "Choosing server Calendars". ADR 0018 ("Calendars", "Consequences"), ADR 0019 ("Whether the server changed").

**Blocked by:** 154

**Status:** ready-for-agent

**Effort:** S

- [ ] Adding a Calendar stores it at once, empty, and returns without waiting for its First Sync
- [ ] Its First Sync is one Incoming Sync Snapshot in one Database Transaction, every version with the server's mark; the Local Store holds no Event Version of it before (test with a held request)
- [ ] A sync-token reset and a server without ETags go through the same Full Fetch
- [ ] An interrupted First Sync leaves nothing and runs again from the beginning at the next Incoming Sync
- [ ] An Event created in a Calendar still loading is sent after its First Sync
- [ ] The core reports whether a Calendar has finished a sync
- [ ] The First Sync flag, `first_sync_unfinished` and the First Sync's own write are gone from the Local Store, the core and the window
- [ ] The tests of `core/tests/caldav_first_sync.rs` are rewritten to this; the Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of two parts: 155 the core, 156 the dialog. Google's First Sync uses the same operation and Signal (ticket 25).
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the First Sync is an Incoming Sync Snapshot from nothing; no pointers or Reference Snapshot.
- 2026-11-22: Rewritten and renamed (was `155-first-sync-in-memory`) from the grilling on point 4 of `.scratch/freecal-v1/critiques.md`: no First Sync in memory behind a modal dialog, no "First Sync progress" Signal and no Cancel of its own; the Calendar is stored empty and its First Sync is a Full Fetch. Re-sized from M to S, as it mostly removes code. Google's First Sync is its Full Fetch too (ticket 25).
