# 14: Add a CalDAV Account, choose its Calendars, pull read-only

**What to build:** The user adds a CalDAV Account with a server URL, username and password. FreeCal discovers its Calendars as Available Calendars and lets the user choose which to add. Chosen Calendars are pulled into the Local Store and shown. The Account's line in the sidebar offers "Manage Calendars…", which fetches the server's current list so the user can add Available Calendars or remove Calendars later. The user can remove the whole Account.

**Blocked by:** 02, 03

**Status:** ready-for-agent

- [ ] Credentials are stored in the system keyring (Secret Service); without a working keyring, adding a server Account fails with a clear error
- [ ] Discovery lists all Calendars of the Account as Available Calendars; the user chooses which to add
- [ ] FreeCal stores nothing for Available Calendars and doesn't sync them
- [ ] Calendars that appear on the server later stay Available Calendars until the user adds them
- [ ] "Manage Calendars…" on the Account line fetches the server's current list; adding an Available Calendar pulls it; removing a Calendar deletes only its local copy, never touches the server, and makes it an Available Calendar again
- [ ] Incremental fetching via `sync-collection` / ETags, through a Provider interface that can take Microsoft Graph and ICS feeds later without changing the sync engine
- [ ] The original server data of each Event is stored as received next to the fields FreeCal uses
- [ ] A Calendar's first sync commits page by page with one "Occurrences changed" per page, and the sidebar shows "Loading for the first time…"
- [ ] Calendars the server doesn't allow changing are Read-only Calendars; before 1.0 all newly added CalDAV Calendars default to read-only; a lock shows in the sidebar and the Event details say it is read-only
- [ ] Trying to drag or resize a read-only Event shows a short message explaining why
- [ ] Removing the Account removes its Calendars and deletes its credentials from the keyring
- [ ] Several CalDAV Accounts can be connected side by side
- [ ] Tests run against a real Radicale server started per test run, with a recording fake keyring
- [ ] CalDAV XML requests are built with an XML writer, never from strings; Calendar names, Event fields and server error text from the server are treated as untrusted and shown only as text
- [ ] Radicale serving the hostile test Event: it is pulled, stored and shown without running or rendering anything
