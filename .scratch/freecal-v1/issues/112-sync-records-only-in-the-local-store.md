# 112: Sync Records live only in the Local Store

**What to build:** The core keeps each server Account's and Calendar's Sync Record twice: in memory (`SyncStatusTracker`, `core/src/sync_status.rs`, loaded on launch) and in the Local Store (the `account` and `calendar` rows), and writes both on every change. So every path that removes an Account or Calendar must also tell the in-memory copy to forget it (`forget_account`, `forget_calendar`). Removing an Account first lists its Calendars so that each can be forgotten, and ending a sync has special cases for one removed meanwhile.

- **One copy:** the Local Store is the only copy of the Sync Records. Memory keeps only what exists while a sync runs: how many syncs of each Account and Calendar run, and whether the user started one.
- **Reading the Sync Status** reads the Sync Records in the same Local Store read as the Unsent Change counts and ticket 111's First Sync flag. The stored Sync Outcome is checked on every read, as now (`SyncRecord::from_stored`; spec "Untrusted content").
- **Removing** an Account or Calendar removes its Sync Record with its row. The forget calls, the listing of an Account's Calendars before removing it, and the "removed meanwhile" special cases go. Writing the Sync Record of a row removed meanwhile changes nothing.
- No change to what the user sees or to the core interface.

Spec: "Sync engine"; "Overall status"; "Untrusted content".

**Blocked by:** 111

**Status:** ready-for-agent

**Effort:** S

- [ ] The in-memory sync state holds only the running syncs; the Sync Records are read from the Local Store
- [ ] Removing an Account or Calendar needs no step for its Sync Status; a sync ending after its Calendar or Account was removed writes nothing and sends no "Sync status changed" for it
- [ ] A Sync Record that can't be read is left out, as now (existing tests keep passing)
- [ ] The existing Sync Status tests pass unchanged
