# 79: Remove a CalDAV Account, and several side by side

**What to build:** The core can remove a whole CalDAV Account, with its Calendars and its credentials, and several CalDAV Accounts can be connected side by side. Spec: US-15. Split from ticket 53.

**Blocked by:** 78

**Status:** done

**Effort:** S

- [x] Removing the Account removes its Calendars and deletes its credentials from the keyring
- [x] Several CalDAV Accounts can be connected side by side, each with its own credentials and Calendars; removing one leaves the others untouched

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 76): `Core::remove_account` already removes a CalDAV Account and deletes its secret from the keyring (tested in `core/tests/caldav_accounts.rs`). Still to do here: its Calendars. `Store::delete_account` relies on `ON DELETE CASCADE`, which would leave their Change Journal entries behind, so delete them like `journal::delete_calendar` does first.
- 2026-10-18: Done. `Core::remove_account` now removes the Account's Calendars as `remove_calendar` removes each, through the new `journal::delete_account`: every Calendar's Events and Change Journal entries, then the Account, in one transaction. `Store::delete_account` is gone, so nothing can delete an Account through `ON DELETE CASCADE` and leave journal entries behind. The keyring is still emptied first (ticket 76's order). Side by side, each Account keeps its own password and Calendars; this already worked and is now tested. Tests: `core/tests/caldav_calendars.rs`, including a Local Calendar's Events and history kept, and Bob's Account still logging in after Alice's is removed. The removal story is US-12 rather than US-15.
