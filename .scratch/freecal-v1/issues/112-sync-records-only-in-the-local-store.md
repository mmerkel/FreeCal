# 112: Sync Records live only in the Local Store

**What to build:** The core keeps each server Account's and Calendar's Sync Record twice: in memory (`SyncStatusTracker`, `core/src/sync_status.rs`, loaded on launch) and in the Local Store (the `account` and `calendar` rows), and writes both on every change. So every path that removes an Account or Calendar must also tell the in-memory copy to forget it (`forget_account`, `forget_calendar`). Removing an Account first lists its Calendars so that each can be forgotten, and ending a sync has special cases for one removed meanwhile.

- **One copy:** the Local Store is the only copy of the Sync Records. Memory keeps only what exists while a sync runs: how many syncs of each Account and Calendar run, and whether the user started one.
- **Reading the Sync Status** reads the Sync Records in the same Local Store read as the Unsent Change counts and ticket 111's First Sync flag. The stored Sync Outcome is checked on every read, as now (`SyncRecord::from_stored`; spec "Untrusted content").
- **Removing** an Account or Calendar removes its Sync Record with its row. The forget calls, the listing of an Account's Calendars before removing it, and the "removed meanwhile" special cases go. Writing the Sync Record of a row removed meanwhile changes nothing.
- **One Sync Status module** (amended 2026-11-07 by the architecture review, candidate 3): today each start and end of a sync updates the tracker, writes the Sync Record and sends "Sync status changed" by hand, in about fifteen private methods of the core interface (`start_calendar_sync`, `end_calendar_sync`, `finish_calendar_sync`, `end_account_sync`, `sync_status_signals`, `send_sync_statuses`, `mark_offline_without_trying`, `core/src/lib.rs`), with the lock order resting on a dummy `&Store` parameter (`Core::tracker`). Leave one module in `core/src/sync_status.rs` that alone touches the running syncs, the Sync Records and "Sync status changed":
  - Starting the sync of an Account or Calendar gives a **Running Sync** (new term), none while one of that Calendar runs already. The sync ends through `end(result)`, which takes the sync's raw result and applies the rules now in `end_calendar_sync`: a Calendar removed meanwhile is no failure, and a failure about the whole Account (`sync_status::about_account`) leaves the Calendar's Sync Status as before and goes to the Account's. A Running Sync dropped without `end` ends as before the sync, as for a Calendar a sync didn't reach, and only logs a failed write. The `unreached` loop of `Core::sync_calendars` goes.
  - Reading the Sync Statuses (`Core::sync_statuses`), marking the server Accounts Offline when the network goes, and the "Sync status changed" after a local change (`Core::after_local_change`) go through the module too.
  - The module is made on demand from the core, with the Store, the clock and the Signal sender, and takes the Store lock and its own in one order; the dummy `&Store` parameter goes.
- No change to what the user sees or to the core interface.

Spec: "Sync engine"; "Overall status"; "Untrusted content".

**Blocked by:** 111

**Status:** ready-for-agent

**Effort:** M

- [ ] The in-memory sync state holds only the running syncs; the Sync Records are read from the Local Store
- [ ] Removing an Account or Calendar needs no step for its Sync Status; a sync ending after its Calendar or Account was removed writes nothing and sends no "Sync status changed" for it
- [ ] A Sync Record that can't be read is left out, as now (existing tests keep passing)
- [ ] The existing Sync Status tests pass unchanged
- [ ] Nothing outside `core/src/sync_status.rs` touches the running syncs, writes a Sync Record or sends "Sync status changed"; every sync start gives a Running Sync, and `Core::tracker` with its dummy `&Store` is gone

## Comments
- 2026-11-07: Amended by the architecture review (candidate 3): the Sync Status module and the Running Sync; effort S → M.
