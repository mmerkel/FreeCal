# 17: Write to CalDAV Calendars while online

**What to build:** The user can switch a CalDAV Calendar from read-only to writable, and mark any Calendar read-only. In a writable CalDAV Calendar, creating, editing, moving and deleting Events (including recurrence scopes) is sent to the server. Every change sent is recorded in the Change Journal. Details FreeCal doesn't show are never lost, and Events with other attendees can't be edited.

**Blocked by:** 08, 14, 15

**Status:** ready-for-agent

- [ ] The user can set any Calendar read-only or writable in FreeCal (server-reported read-only still wins)
- [ ] Changes are applied to the Local Store immediately, queued, and pushed with conditional requests (ETags)
- [ ] Recurrence scope edits (this / this and following / all) and Exceptions reach the server correctly
- [ ] Unknown fields (attendees, conference links, custom properties) are preserved byte-for-byte across edits
- [ ] Any Event with attendees other than the user is read-only; its details say "has guests" with the hint to edit it on the server; the grid offers no move or resize, and a refused drag explains why
- [ ] Before each change is sent, the Event's previous server version is stored in the Change Journal together with the new one
- [ ] A server refusing writes to a Calendar becomes a Sync Error on that Calendar
- [ ] Tested against Radicale
