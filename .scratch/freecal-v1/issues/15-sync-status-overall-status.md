# 15: Sync Status and the overall status

**What to build:** The window shows whether FreeCal is syncing and when the last sync finished, through a status indicator showing the overall status. The sidebar shows each server Account's and Calendar's Sync Status (Syncing, Offline, Sync Error) when it isn't Synced, and the number of unsent changes when it isn't zero. Being offline is not an error.

**Blocked by:** 82, 55

**Status:** ready-for-agent

**Effort:** M

- [x] The core sends "Sync status changed" separately for Accounts and Calendars, carrying the new state, the time and outcome of the last finished sync, the number of unsent local changes, and a first-sync marker; the Local Account never sends it
- [x] A network-level failure (DNS, connection refused, timeout) makes that Account Offline; an error response from the server is a Sync Error with a clear message, on the Account or Calendar where it happened
- [x] Account-level states appear once on the Account row, not repeated on every Calendar
- [ ] Overall status: Conflict, else Sync Error, else Offline (at least one server Account and every server Account Offline), else Syncing, else Online and synced; correct with no server Accounts
- [x] "Syncing" is displayed only after a sync has run for about 1 second, or immediately if the user started it; the Signals are still sent straight away
- [x] Component tests check that the indicator and sidebar markers follow "Sync status changed"

## Comments

- 2026-10-18 (from 80): ticket 80 reports a running first sync as `Calendar.first_sync` with "Calendars changed"; decide whether "Sync status changed" takes it over or it stays alongside.
- 2026-11-07 (implementation): every box is done but the overall status's Conflict: the priority order is in place (`OverallState::Conflict` in `core/src/sync_status.rs`), but nothing raises a Conflict until ticket 20, which then has to feed it into the overall status (`SyncStatusTracker::overall`).
- 2026-11-07 (implementation): decisions.
  - "Sync status changed" takes over `Calendar.first_sync`: the field is gone, and a First Sync is `SyncStatus::Syncing { first_sync: true }`. Adding Calendars sends "Calendars changed" once, no longer at the end of the First Sync.
  - Sync Status lives in memory for the running FreeCal, so after a launch everything reads Synced with no last sync until the first sync (ticket 16 syncs at launch). Storing the last sync in the Local Store is a later choice. (Superseded below: the Local Store keeps it.)
  - A failure about the whole Account (offline, TLS, wrong or missing login, keyring) stops that Account's sync instead of waiting on each Calendar; its Calendars keep their Sync Status. Any other failure is the Calendar's Sync Error, and the other Calendars still sync.
  - A Calendar that is syncing is left out of another sync, not only one on its First Sync.
  - `Core::sync_account` takes who started it (`StartedBy::User` or `StartedBy::FreeCal`) for the 1-second rule; ticket 16's sync now passes `User`. Adding Calendars counts as the user's.
  - While FreeCal retries an Offline Account by itself, the overall status stays Offline instead of flickering to Syncing.
  - The unsent count shows on Calendar rows only; the Account's report carries the total.
  - The status indicator sits in the toolbar before ⋮. It isn't a live region, so screen readers don't announce every routine sync.
- 2026-11-07 (implementation): new terms, for /code-review's approval.
  - Sync Status Report (new term): a Sync Status with the last finished sync and the number of Unsent Changes, as "Sync status changed" carries it (`SyncStatusReport`).
  - Sync Outcome (new term): how a finished sync ended: Synced, Offline or Sync Error (`SyncOutcome`), with Finished Sync, its time and outcome (`FinishedSync`).
  - Started By (new term): whether the user or FreeCal started a sync (`StartedBy`).
  - Overall State (new term): the spec's overall status as a value (`OverallState`, `OverallStatus` with the last sync time).
  - Sync Status Tracker (new term): the in-memory Sync Status of every server Account and Calendar (`SyncStatusTracker`).
  - Syncing Delay (new term): the window holding back a routine sync's "Syncing" for about 1 second (`SyncingDelay`, `src/lib/syncDisplay.ts`).
- 2026-11-07 (review with the user): the status indicator's time is the oldest of the server Accounts' and Calendars' last successful syncs, so "Synced 09:00" means everything is synced at least up to 09:00, and a failed attempt doesn't count. It had been the latest finished sync of any Account, whatever its outcome. Trade-off: one Account that hasn't synced for a while holds the time back for all; the sidebar rows keep their own times. No time shows while any of them hasn't synced since FreeCal started (`SyncStatusTracker::overall`).
- 2026-11-07 (review with the user): the Local Store keeps each server Account's and Calendar's Sync Record (new term): how its last sync ended and when, and when its last successful sync finished (`SyncRecord`, `core/src/sync_status.rs`; migration 2 in `core/src/store/schema.rs`). A relaunch shows them until the next sync, such as "Offline · last sync yesterday 18:05", instead of Synced with no time. Read back untrusted: a part that can't be read is left out, and a stored Sync Error's server text is made plain again (`ErrorCode::checked`). The outcome is stored as the JSON the interface sends, so renaming a Sync Outcome or error code later drops older stored outcomes, which are then read as unreadable. Failing to keep a Sync Record makes the sync return that error. `serde_json` moves from the core's test dependencies to its dependencies.
