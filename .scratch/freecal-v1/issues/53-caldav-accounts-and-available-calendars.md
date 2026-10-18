# 53: CalDAV Accounts and Available Calendars in the core

**What to build:** The core can add a CalDAV Account from a server URL, username and password, discover its Calendars as Available Calendars, add or remove Calendars of it, and remove the whole Account. Credentials live in the system keyring. This ticket is core only and tested against a real Radicale server; pulling Events is ticket 54 and the UI is ticket 55. Spec: user story 15. Split from ticket 14.

**Blocked by:** 02, 03

**Status:** wontfix

**Split into:** 76, 77, 78, 79

- [ ] Tests run against a real Radicale server started per test run, with a recording fake keyring
- [ ] Credentials are stored in the system keyring (Secret Service) behind a fakeable interface; without a working keyring, adding a server Account fails with a clear error
- [ ] Adding an Account checks the credentials and discovers the server's Calendars
- [ ] Discovery lists all Calendars of the Account as Available Calendars; FreeCal stores nothing for Available Calendars and doesn't sync them
- [ ] Calendars that appear on the server later stay Available Calendars until the user adds them; listing the Account's Calendars fetches the server's current list
- [ ] Removing a Calendar deletes only its local copy, never touches the server, and makes it an Available Calendar again; `delete_calendar` gets a Provider check for server Calendars like the one in `create_calendar` (ticket 02's note)
- [ ] Removing the Account removes its Calendars and deletes its credentials from the keyring
- [ ] Several CalDAV Accounts can be connected side by side
- [ ] CalDAV XML requests are built with an XML writer, never from strings; Calendar names and server error text are treated as untrusted
- [ ] Core errors cross to the frontend as codes, not English strings

Out of scope: pulling Events (ticket 54); the dialogs and sidebar (ticket 55); read-only (ticket 56).

## Comments

- 2026-10-13: Split from ticket 14.
- 2026-10-18: Split into 76 (add a CalDAV Account), 77 (discover Available Calendars), 78 (add and remove Calendars) and 79 (remove an Account, several side by side), so each part fits one session. Work from those; this file stays for its history.
