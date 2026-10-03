# 26: Write to Google Calendars

**What to build:** Writable Google Calendars get the same editing, offline replay, Change Journal, Conflict and Mass-Change Guard behaviour as CalDAV. When the Google login expires, the user is asked to log in again and sync resumes. Exports keep Google-only details, so a Google Event can be restored completely.

**Blocked by:** 09, 18, 25

**Status:** ready-for-agent

- [ ] Conditional create, update and delete with ETags; unknown fields preserved byte-for-byte
- [ ] Change Journal, Conflicts, the Mass-Change Guard and offline replay work for Google Calendars (tested against the fake Google server)
- [ ] An expired login is a Sync Error on the Account that asks the user to log in again; after re-authentication sync resumes
- [ ] Google-only fields are exported as `X-` properties and read back on import, round-tripping exactly
