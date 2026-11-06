# 103: One CalDAV Server Object per address

**What to build:** Agreed on 2026-11-06. A CalDAV Server Object stands for one resource, and no path writes a second row at one address: the Incoming Sync writes over the row at the address when there is one, and a moved Server Object only lands on an address with no row. The Local Store enforces it, and the code for "the first of several" goes.

- `caldav_server_object_calendar_href` becomes a unique index on `(calendar_id, href)`. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- `objects_at` becomes `object_at`, returning at most one Server Object. `server_objects`'s "the first at each, should it have several", the Incoming Sync's `.first()` and the plural in marking a resource unreadable (`mark_unreadable`) go.

**Blocked by:** 102

**Status:** ready-for-agent

**Effort:** S

- [ ] Writing a second CalDAV Server Object at one address in a Calendar fails in the Local Store
- [ ] `object_at` replaces `objects_at`; no code handles several Server Objects at one address
- [ ] A Server Object moved to another address in one Incoming Sync keeps its Event Rows, as before
