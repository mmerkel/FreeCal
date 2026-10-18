# 56: Read-only CalDAV Calendars

**What to build:** Calendars the server doesn't allow changing are Read-only Calendars, and before 1.0 every newly added CalDAV Calendar starts read-only. A lock shows in the sidebar, the Event details say the Event is read-only, and trying to drag or resize it explains why. Spec: US-16, US-18, US-30 and US-32. Split from ticket 14.

**Blocked by:** 54, 55

**Status:** ready-for-agent

**Effort:** M

- [ ] The core reads the server's privileges and makes Calendars the server doesn't allow changing Read-only Calendars (tested against Radicale)
- [ ] Before 1.0, all newly added CalDAV Calendars default to read-only
- [ ] A lock shows on the Calendar's row in the sidebar, in ticket 32's grey marker style; the Event details say the Event is read-only and offer only ✕ (ticket 37)
- [ ] Trying to drag or resize a read-only Event shows a short message explaining why (see comment)
- [ ] Read-only Calendars aren't offered when choosing an Event's Calendar
- [ ] Every string goes through `t()`
- [ ] Component tests against the fake core

Out of scope: switching a Calendar between read-only and writable (ticket 17).

## Comments

- 2026-10-13: Split from ticket 14. Dragging and resizing come with ticket 05; if 05 isn't done yet, the refused-drag message moves to 05.
