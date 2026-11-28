# 211: A Google Account: browser login, its Available Calendars, removal

**What to build:** The user adds a Google Account by logging in through the browser with FreeCal's shared (unverified) client and sees its Calendars listed as Available Calendars. Several Google Accounts can be connected, and an Account can be removed. Adding the Calendars and syncing them is 212. Old ticket 164 (`git show design/snapshot-line:.scratch/freecal-v1/issues/164-google-account-login-and-available-calendars.md`) as written. Independent of the rest of the redesign: it can go anywhere in the order.

- **Login:** the system browser, a loopback redirect and PKCE. The tokens go to the keyring (`core/src/keyring.rs`) and nowhere else; the access token is renewed with the refresh token when it runs out. A login that can't be renewed is a Sync Error on the Account; asking to log in again is 214's.
- **Discovery:** the Account's calendar list becomes Available Calendars, each with its name and colour (untrusted text) and whether the server lets the user change it. Calendars that appear on the server later stay Available Calendars.
- **Where:** the Account and its HTTP client in the core's `google/` (210 starts it; this ticket adds its part to `redesign-layout.md`'s module tree); the loopback listener and opening the browser in the shell (`src-tauri/`) if the core can't do them.
- **In the window:** the dialog that adds an Account offers Google beside CalDAV; "Manage Calendars…" on the Account's line lists the Available Calendars as for CalDAV, which until 212 can't be added.
- **Removing the Account** removes its tokens from the keyring, and its Calendars once 212 has added any.
- **The fake Google server:** a test HTTP server mimicking Google's token endpoint and calendar list, exercising the real Google code; 212 extends it with events.
- **Offline and errors:** a network failure makes the Account Offline, an error answer is a Sync Error with a clear message, as for CalDAV.

Spec: US-6, US-9, US-12; "Choosing server Calendars"; "Keyring" under Desktop interfaces; "Untrusted content". ADR 0002. Layout: "By phase", "P5 and P6".

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] Browser login with a loopback redirect and PKCE; tokens stored in the keyring only, and renewed
- [ ] Discovery lists the Account's Calendars as Available Calendars, with whether the server lets the user change each
- [ ] The window offers Google when adding an Account; several Google Accounts can be connected
- [ ] Removing the Account removes its tokens from the keyring
- [ ] Tested against the fake Google server: login, renewal, a refused renewal, the calendar list, a hostile Calendar name, offline
