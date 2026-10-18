# 79: Remove a CalDAV Account, and several side by side

**What to build:** The core can remove a whole CalDAV Account, with its Calendars and its credentials, and several CalDAV Accounts can be connected side by side. Spec: user story 15. Split from ticket 53.

**Blocked by:** 78

**Status:** ready-for-agent

**Effort:** S

- [ ] Removing the Account removes its Calendars and deletes its credentials from the keyring
- [ ] Several CalDAV Accounts can be connected side by side, each with its own credentials and Calendars; removing one leaves the others untouched

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 76): `Core::remove_account` already removes a CalDAV Account and deletes its secret from the keyring (tested in `core/tests/caldav_accounts.rs`). Still to do here: its Calendars. `Store::delete_account` relies on `ON DELETE CASCADE`, which would leave their Change Journal entries behind, so delete them like `journal::delete_calendar` does first.
