# 103: One CalDAV Server Object per address

**What to build:** Agreed on 2026-11-06. A CalDAV Server Object stands for one resource, and no path writes a second row at one address: the Incoming Sync writes over the row at the address when there is one, and a moved Server Object only lands on an address with no row. The Local Store enforces it, and the code for "the first of several" goes.

- `caldav_server_object_calendar_href` becomes a unique index on `(calendar_id, href)`. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- `objects_at` becomes `object_at`, returning at most one Server Object. `server_objects`'s "the first at each, should it have several", the Incoming Sync's `.first()` and the plural in marking a resource unreadable (`mark_unreadable`) go.

**Blocked by:** 102

**Status:** done

**Effort:** S

- [x] Writing a second CalDAV Server Object at one address in a Calendar fails in the Local Store
- [x] `object_at` replaces `objects_at`; no code handles several Server Objects at one address
- [x] A Server Object moved to another address in one Incoming Sync keeps its Event Rows, as before

## Comments

Done on 2026-11-06. `caldav_server_object_calendar_href` is a unique index in the baseline schema; `a_second_server_object_at_one_address_fails` (`core/src/journal/mod.rs`) checks that the Local Store refuses a second Server Object at one href. `object_at` replaces `objects_at`. The Incoming Sync only tries a move when the href has no row, so no order within one sync hits the index. `an_event_whose_object_moved_to_another_address_stays_the_same_event` (`core/tests/caldav_sync.rs`) still passes. The test `of_two_server_objects_holding_a_uid_the_one_holding_the_recurring_event_corresponds` had put two Server Objects at one href and now writes the copy at its own.

No new terms.

The review raised two points that are older than this ticket and left open: hrefs are compared as the server spells them, so one resource under two spellings would get two rows; and a `sync-collection` reply listing one href as both gone and changed loses its Server Object at the end of the Incoming Sync.
