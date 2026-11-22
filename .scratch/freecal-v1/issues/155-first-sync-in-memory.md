# 155: A Calendar enters the Local Store with its first Snapshot: the First Sync runs in memory and reports "First Sync progress"

**What to build:** The First Sync as ADR 0018 decides it, in the core: nothing of a server Calendar is stored until its First Sync has finished, and then the Calendar, its Events and its first Snapshot are written in one Database Transaction. The stored state "First Sync unfinished" (tickets 80 and 111) comes out. This is step 4 of the order in which the old model goes. The dialog that shows the progress is ticket 156.

- **Adding Available Calendars** (`Core::add_calendars`, `core/src/lib.rs`) starts one First Sync per chosen Calendar and stores nothing at the start. Each Calendar is added when its own First Sync finishes, independently of the others.
- **The First Sync** shares the fetching with an Incoming Sync (CalDAV: the member listing and the multiget in pages, `core/src/caldav.rs`), but holds what it fetched in memory. Its write is its own (today `first_sync`, `core/src/sync/caldav/incoming.rs`): one Database Transaction creates the Calendar row, the Event Versions, the CalDAV table's rows with their ETags, the Ignored Resources and one Server Intake, with the server's mark on every Event Version. It compares nothing.
- **"First Sync progress"** (status Signal, new): for each Calendar being added, how far its First Sync has come (what is counted depends on the Provider; CalDAV counts resources fetched of resources listed), its Sync Error if it has one, and that it finished or was cancelled. It names the Calendar by what the Available Calendar is known by, as no Calendar ID exists yet. "Calendars changed" is sent when a Calendar is added, with "Sync status changed" for its first Sync Status, Synced.
- **Cancel:** the operation can be cancelled as a whole. First Syncs still running stop and leave nothing; finished ones stay.
- **A Sync Error** ends that Calendar's First Sync and leaves nothing of it. The others run on. A retry is adding the Calendar again, from the beginning. A failure about the whole Account (offline, login, keyring) ends all that are still running, and is reported for each.
- **Interrupted** by the network or by FreeCal closing, a First Sync leaves nothing behind and isn't retried by itself (ADR 0018, "Consequences"). The Calendar is an Available Calendar again.
- **Other syncs:** a Calendar that isn't stored can't be reached by the Sync Scheduler or "sync now", so ticket 111's rules for Calendars a sync didn't reach have no case left for a First Sync. Adding a Calendar that is already being added is refused.
- **What comes out:** the Calendar row's First Sync flag and `first_sync_unfinished` in the Sync Status report (`core/src/sync_status.rs`, `core/src/store/sql/mod.rs`); the Incoming Sync's branch that runs a First Sync for a stored Calendar holding nothing; in the window, the sidebar's texts "Loading for the first time…", "Not loaded yet" and "· not fully loaded" (`src/lib/Sidebar.svelte`) and the Unsent Mark's reason for an unfinished First Sync (`src/lib/unsent.ts`).
- **The window until ticket 156:** "Manage Calendars…" waits for the whole operation as it does today and shows the Calendars when they are added. No progress and no cancel yet.
- **The Store Check** gains nothing: it already asks that every Calendar has its pointers set. The schema no longer needs to allow a server Calendar without them, if it still did.
- **Docs:** `GLOSSARY.md` revises First Sync (if ticket 147 left anything of the old meaning), Available Calendar and Sync Status where they speak of an unfinished First Sync.

Spec: US-5, US-152; "Signals"; "Sync engine"; "Choosing server Calendars". ADR 0018 ("The pointers", "Consequences").

**Blocked by:** 154

**Status:** ready-for-agent

**Effort:** M

- [ ] While a First Sync runs, the Local Store holds nothing of the Calendar (test with a held request, and by reading the store)
- [ ] When it finishes, the Calendar, its Events and its first Snapshot appear in one Database Transaction as one Server Intake, every version with the server's mark; no Undo takes it back
- [ ] "First Sync progress" reports each Calendar's progress, its Sync Error, and that it finished or was cancelled
- [ ] Cancel stops the First Syncs still running and keeps the finished Calendars
- [ ] A First Sync that fails, or FreeCal closing during one, leaves nothing; the Calendar is still an Available Calendar and can be added again
- [ ] One Calendar's Sync Error doesn't stop the others; an Account-level failure ends all still running
- [ ] The First Sync flag and `first_sync_unfinished` are gone from the Local Store, the core interface and the window
- [ ] The tests of `core/tests/caldav_first_sync.rs` are rewritten to this; the Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of two parts: 155 the core, 156 the dialog. Google's First Sync uses the same operation and Signal (ticket 25).
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the First Sync is a Server Intake from nothing; no pointers or Reference Snapshot.
