# 25: Add a Google Account, choose its Calendars, pull read-only

**What to build:** The user adds a Google Account by logging in through the browser with FreeCal's shared (unverified) client, chooses which of its Available Calendars to add, and sees them in FreeCal. "Manage Calendars…" on the Account line works as for CalDAV. Several Google Accounts can be connected.

**Blocked by:** 14, 15

**Status:** ready-for-agent

- [ ] Browser login with a loopback redirect and PKCE; tokens stored in the keyring
- [ ] Discovery lists the Account's Calendars as Available Calendars; the user chooses which to add; later-appearing Calendars stay Available Calendars until added through "Manage Calendars…"
- [ ] Incremental pulls through the Google Calendar REST API (ADR 0002) using sync tokens; a 410 "token expired" triggers a full resync
- [ ] Server permissions decide read-only; before 1.0 newly added Google Calendars default to read-only
- [ ] Removing the Account removes its Calendars and its tokens from the keyring
- [ ] Tested against a fake Google HTTP server that mimics the REST API (sync tokens, ETags, 410, permissions), exercising the real Google Provider code
