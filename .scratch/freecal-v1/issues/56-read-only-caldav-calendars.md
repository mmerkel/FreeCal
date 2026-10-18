# 56: Read-only CalDAV Calendars

**What to build:** Calendars the server doesn't allow changing are Read-only Calendars, and before 1.0 every newly added CalDAV Calendar starts read-only. A lock shows in the sidebar, the Event details say the Event is read-only, and trying to drag or resize it explains why. Spec: US-16, US-18, US-30 and US-32. Split from ticket 14.

**Blocked by:** 82, 55

**Status:** ready-for-agent

**Effort:** M

- [x] The core reads the server's privileges and makes Calendars the server doesn't allow changing Read-only Calendars (tested against Radicale)
- [x] Before 1.0, all newly added CalDAV Calendars default to read-only
- [ ] A lock shows on the Calendar's row in the sidebar, in ticket 32's grey marker style; the Event details say the Event is read-only and offer only ✕ (ticket 37)
- [x] Trying to drag or resize a read-only Event shows a short message explaining why (see comment)
- [x] Read-only Calendars aren't offered when choosing an Event's Calendar
- [x] Every string goes through `t()`
- [x] Component tests against the fake core

Out of scope: switching a Calendar between read-only and writable (ticket 17).

## Comments

- 2026-10-13: Split from ticket 14. Dragging and resizing come with ticket 05; if 05 isn't done yet, the refused-drag message moves to 05.
- 2026-10-18: Done except the ✕: the details of a read-only Event say "Can't be changed: its calendar is read-only." and offer only Close, which becomes ✕ with ticket 37 (the box stays open for that). The core reads `DAV:current-user-privilege-set` when it discovers Calendars: `all`, `write`, or `bind` + `unbind` + `write-content` mean writable, and a server that doesn't say counts as writable. `Calendar.server_read_only` keeps the server's say apart from the user's mark (`calendar.read_only` column; `Calendar.read_only` is either), for ticket 17. Privileges are read only when a Calendar is added, not on each sync (see ticket 17). A refused drag is reported for mouse drags only; touch drags (long press in FullCalendar) show nothing. Radicale tests use a rights file where an `admin` user creates `read-only-*` Calendars that their owner may only read.
