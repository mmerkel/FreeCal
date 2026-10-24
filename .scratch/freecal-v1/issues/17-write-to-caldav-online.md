# 17: Write to CalDAV Calendars while online

**What to build:** The user can switch a CalDAV Calendar from read-only to writable, and mark any Calendar read-only. In a writable CalDAV Calendar, creating, editing, moving and deleting Events (including recurrence scopes) is sent to the server. Every change sent is recorded in the Change Journal. Details FreeCal doesn't show are never lost, and Events with other attendees can't be edited.

**Blocked by:** 08, 15, 56, 69, 87

**Status:** ready-for-agent

**Effort:** L

- [ ] The user can set any Calendar read-only or writable in FreeCal (server-reported read-only still wins)
- [ ] Changes are applied to the Local Store immediately, queued, and pushed with conditional requests (ETags)
- [ ] Recurrence scope edits (this / this and following / all) and Exceptions reach the server correctly
- [ ] Unknown fields (attendees, conference links, custom properties) are preserved byte-for-byte across edits
- [ ] Any Event with attendees other than the user is read-only; its details say "has guests" with the hint to edit it on the server; the grid offers no move or resize, and a refused drag explains why
- [ ] Before each change is sent, it is in the Change Journal as ticket 69 records it (previous version, hash of the new one)
- [ ] An undo or redo of a change in a server Calendar is sent to the server like any other change, under the original identity
- [ ] A server refusing writes to a Calendar becomes a Sync Error on that Calendar
- [ ] Tested against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-18 (from 81): each server Event's original object is `EventFields.server_data` (href, ETag, iCalendar text as received), kept on the Recurring Event's row only. User edits keep it; a move to another Calendar drops it. An undo restores an older version's ETag and object, so a write after an undo may need the server's current version for If-Match. A server object holding VEVENTs with different UIDs (invalid in CalDAV) gives each Event a copy of the same object; writing them back must not duplicate it.
- 2026-10-18 (from 56): the core reads the server's privileges only when a Calendar is added (`calendar.server_read_only`). A permission the server revokes or grants later isn't noticed; once the user can unmark Calendars, re-read `current-user-privilege-set` on each sync and send "Calendars changed" when it changes (spec: "Calendars changed"). Tests unmark Calendars with `make_writable` in `core/tests/support/mod.rs`; replace it with this ticket's interface call.
- 2026-10-18 (from 82): undoing a sync unit restores the previous version locally but leaves `calendar.sync_token` alone, so the server's newer version isn't fetched again until it changes there. The undo has to reach the server through this ticket, or the next whole listing (without a token) brings the server's version back.
- 2026-10-24: Blocked by 87: writes build on server objects that mirror the server (ADR 0010). A write sends a pending object with `If-Match` on the mirror's ETag, and a success updates the mirror.
