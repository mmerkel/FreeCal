# 164: A Google Account: browser login, its Available Calendars, removal

**What to build:** The user adds a Google Account by logging in through the browser with FreeCal's shared (unverified) client, and sees the Account's Calendars listed as Available Calendars. Several Google Accounts can be connected, and an Account can be removed. Adding the Calendars and syncing them is ticket 25. Second of three parts of ticket 25.

- **Login:** the system browser, a loopback redirect and PKCE. The tokens go to the keyring (`core/src/keyring.rs`) and nowhere else; the access token is renewed with the refresh token when it runs out. A login that can't be renewed is a Sync Error on the Account; asking the user to log in again from it is ticket 26.
- **Discovery:** the Account's calendar list becomes Available Calendars, each with its name and colour (untrusted text) and whether the server lets the user change it. Calendars that appear on the server later stay Available Calendars.
- **In the window:** the dialog that adds an Account offers Google beside CalDAV, and "Manage Calendars…" on the Account's line lists the Available Calendars as for CalDAV (ticket 55's dialogs). Until ticket 25 they are listed and can't be added.
- **Removing the Account** removes its tokens from the keyring, and its Calendars once ticket 25 has added any.
- **The fake Google server:** a test HTTP server that mimics Google's token endpoint and calendar list, exercising the real Google Provider code. Ticket 25 extends it with events.
- **Offline and errors:** a network failure makes the Account Offline and an error answer is a Sync Error with a clear message, as for CalDAV (ticket 15).

Spec: US-6, US-9, US-12; "Choosing server Calendars"; "Keyring" under Desktop interfaces; "Untrusted content". ADR 0002.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] Browser login with a loopback redirect and PKCE; tokens stored in the keyring only, and renewed
- [ ] Discovery lists the Account's Calendars as Available Calendars, with whether the server lets the user change each
- [ ] The window offers Google when adding an Account; several Google Accounts can be connected
- [ ] Removing the Account removes its tokens from the keyring
- [ ] Tested against the fake Google server (login, renewal, a refused renewal, the calendar list, a hostile Calendar name)

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020, as the second of three parts of ticket 25 (see ticket 163). Its former blockers 55, 56, 108 and 115 are done, so it is blocked by nothing and can be built beside the Calendar History.
