# 194: A server Calendar is stored empty when added; its First Sync is a Full Fetch

**What to build:** The First Sync as ADR 0018 decides it: adding an Available Calendar stores the Calendar at once, with no Event Versions, sync token or ETags, and its First Sync is its first Incoming Sync, a Full Fetch (ADR 0019) that takes in everything as one Incoming Sync Snapshot in one Database Transaction. The First Sync's own write, its stored state "First Sync unfinished" and the paged rest of an unfinished one come out. Old ticket 155 (`git show design/snapshot-line:.scratch/freecal-v1/issues/155-first-sync-is-a-full-fetch.md`) as written. The dialog choosing the Calendars is 208.

- **Adding Available Calendars** (`Core::add_calendars`) stores each chosen Calendar, sends "Calendars changed" once and asks the Sync Scheduler for their Incoming Syncs, without waiting for them. Until 208, "Manage Calendars…" returns once the Calendars are stored, and they fill in as their First Syncs finish.
- **The Full Fetch** is the Incoming Sync's path when a Calendar has no sync token, its server gives no ETags, or the server resets the sync token: every resource fetched into memory (CalDAV: the member listing and the multiget in pages, `caldav.rs`), then 189's plan and writer, with the CalDAV table's rows, their ETags, the Ignored Resources and the sync token in the same Database Transaction. With no marks, all of it is "only the server changed". Nothing is written page by page; this ends 179's one Snapshot per page and 189's separate First Sync path.
- **Interrupted** by the network or by FreeCal closing, a Full Fetch leaves nothing behind; the next Incoming Sync starts it again.
- **"Not loaded yet" is read, not stored:** the Sync Status reports whether a Calendar has finished a sync; none means its First Sync hasn't. The window's texts for it are 202's.
- **Editing a Calendar still loading** is allowed; the Full Fetch finds such an Event on the user's side only, and the next Outgoing Sync sends it. **Cancelling** is removing the Calendar (ticket 112's rule for a Calendar removed meanwhile). **An Undo of the First Sync's Snapshot** is no special case.
- **What comes out:** `first_sync` (`sync/caldav/incoming.rs`) and the paged rest; the Calendar's First Sync flag columns, `unguarded.rs`'s `finish_*_sync`, and `first_sync_unfinished` in the Sync Status (`sync_status.rs`, `store/sql/mod.rs`); in the window, the sidebar's "Loading for the first time…", "Not loaded yet" and "· not fully loaded" as made today (`src/lib/Sidebar.svelte`) and the Unsent Mark's reason for an unfinished First Sync (`src/lib/unsent.ts`). The window's part stays what the type change forces; 202 brings the texts back from the Sync Status.

Spec: US-5, US-152; "Signals"; "Sync engine"; "Choosing server Calendars". ADRs 0018 ("Calendars", "Consequences"), 0019 ("Whether the server changed"). Layout: "Tables (end state)" (`calendar`); "By phase", P3, 194.

**Blocked by:** 193

**Status:** ready-for-agent

**Effort:** M

**Rewritten:** `core/tests/caldav_first_sync.rs`; `sync_status.rs`'s First Sync tests. **Deleted tests:** `caldav_sync.rs`'s `a_calendar_whose_first_sync_didnt_finish_*`; `sync_status.rs`'s `a_calendar_quit_during_its_first_sync_*`; the three "First Sync is unfinished" tests of `src/App.sync.test.ts`; `src/lib/unsent.test.ts`'s "an unfinished First Sync holds the changes back".

- [ ] Adding a Calendar stores it at once, empty, and returns without waiting for its First Sync
- [ ] Its First Sync is one Incoming Sync Snapshot in one Database Transaction, every version with the server's mark; the Local Store holds no Event Version of it before (test with a held request)
- [ ] A sync-token reset and a server without ETags go through the same Full Fetch
- [ ] An interrupted First Sync leaves nothing and runs again from the beginning at the next Incoming Sync
- [ ] An Event created in a Calendar still loading is sent after its First Sync
- [ ] The core reports whether a Calendar has finished a sync; the TypeScript types are regenerated
- [ ] The First Sync flag, `first_sync_unfinished` and the First Sync's own write are gone from the Local Store, the core and the window
- [ ] The tests above rewritten or deleted; the Store Check passes after every scenario
- [ ] `GLOSSARY.md`'s Sync Status and Available Calendar checked for an unfinished First Sync
