# 76: Add a CalDAV Account in the core

**What to build:** The core can add a CalDAV Account from a server URL, username and password: it checks the credentials against the server and stores them in the system keyring. This ticket lays the groundwork the other CalDAV tickets build on: the Radicale test harness, the keyring interface with its recording fake, the CalDAV client with its XML writer, and the error codes. Discovering Calendars is ticket 77. Spec: user story 15. Split from ticket 53.

**Blocked by:** 02, 03

**Status:** done

**Effort:** M

- [x] Tests run against a real Radicale server started per test run, with a recording fake keyring
- [x] Credentials are stored in the system keyring (Secret Service) behind a fakeable interface; without a working keyring, adding a server Account fails with a clear error
- [x] Adding an Account checks the credentials against the server; wrong credentials or an unreachable server fail without storing anything
- [x] CalDAV XML requests are built with an XML writer, never from strings; server error text is treated as untrusted
- [x] Core errors cross to the frontend as codes, not English strings

Out of scope: discovering Calendars (ticket 77); adding or removing Calendars (ticket 78); removing the Account (ticket 79).

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18: Done. `Core::add_caldav_account` asks the server for `current-user-principal` (PROPFIND, Depth 0) to check the login, then stores the password in the keyring under a random key and the Account (URL, username, key) in the Local Store. Choices:
  - Keyring: the `Keyring` trait lives in the core; the shell's real one (`src-tauri/src/keyring.rs`) uses `oo7` rather than libsecret. It speaks Secret Service over D-Bus and, inside a Flatpak, the Secret portal, with no C library. An ignored test exercises it against the session's keyring (`cargo test -p freecal -- --ignored`).
  - HTTP: `ureq` with native-tls (OpenSSL, the system's trusted certificates). Redirects aren't followed, so the password goes only to the URL the user typed; a redirect currently shows as "refused (status 30x)" (see ticket 77).
  - New codes: `invalidServerUrl`, `wrongCredentials` (also a username with `:`), `serverUnreachable`, `secureConnectionFailed`, `notCalDavServer`, `serverRefused { status, message }`, `keyringUnavailable`, and `notLocalAccount` for `create_calendar` on a server Account. `message` is the server's text only when it sent `text/plain`, with control and bidi characters turned into spaces, on one line, at most 200 characters. The hostile fixture gained `serverMessage` and `username`.
  - Radicale runs once per test binary, and each test uses a user of its own. It needs `pip install --user radicale` (README).
  - Not wired as a Tauri command yet: that comes with the dialog in ticket 55.
  - Beyond the ticket: `remove_account` already removes a CalDAV Account and deletes its secret (ticket 79 finishes it with its Calendars).
