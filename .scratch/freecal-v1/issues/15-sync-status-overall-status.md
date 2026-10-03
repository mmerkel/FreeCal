# 15: Sync Status and the overall status

**What to build:** The window shows whether FreeCal is syncing and when the last sync finished, through a status indicator showing the overall status. The sidebar shows each server Account's and Calendar's Sync Status (Syncing, Offline, Sync Error) when it isn't Synced, and the number of unsent changes when it isn't zero. Being offline is not an error.

**Blocked by:** 14

**Status:** ready-for-agent

- [ ] The core sends "Sync status changed" separately for Accounts and Calendars, carrying the new state, the time and outcome of the last finished sync, the number of unsent local changes, and a first-sync marker; the Local Account never sends it
- [ ] A network-level failure (DNS, connection refused, timeout) makes that Account Offline; an error response from the server is a Sync Error with a clear message, on the Account or Calendar where it happened
- [ ] Account-level states appear once on the Account row, not repeated on every Calendar
- [ ] Overall status: Conflict, else Sync Error, else Offline (at least one server Account and every server Account Offline), else Syncing, else Online and synced; correct with no server Accounts
- [ ] "Syncing" is displayed only after a sync has run for about 1 second, or immediately if the user started it; the Signals are still sent straight away
- [ ] Component tests check that the indicator and sidebar markers follow "Sync status changed"
