# 100: CalDAV Mirror tables

**What to build:** Agreed on 2026-11-06 (ADR 0013). The Mirror's CalDAV Server Objects get a table of their own, and the generic `server_object` and `combined_event_key` go.

- `caldav_server_object` replaces `server_object` for CalDAV: the Calendar, href, ETag, the iCalendar text as received, the unreadable mark, the UID, and whether it holds the Event that isn't an Exception (the single Event or the Recurring Event). There is no `format` column and no key table. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- A CalDAV Server Object holds exactly one UID (RFC 4791 §4.1). A resource holding several is mirrored with its first UID and without Event Rows, as a second Server Object holding a UID already is (ADR 0011).
- The Corresponding Server Object of a UID in a Calendar is found in `caldav_server_object`: of those holding the UID, the one holding the Event that isn't an Exception, otherwise the first one mirrored (`corresponding_server_object`). Corresponding UIDs and held UIDs collapse to the one UID a CalDAV Server Object holds.
- Combined Event Keys are no longer stored in the Mirror. `EventKeyInCalendar` folds into `CombinedEventKey`: a Calendar, a UID and a recurrence ID as the Event Row holds it (`GLOSSARY.md`). Rewriting a Server Object's Combined Event Keys (`rewrite_combined_event_keys`) goes with the table.
- The Sync Engine's Write Permission covers `caldav_server_object`, in `core/src/store/sql/mirror/caldav.rs`.
- `CODING_STANDARDS.md` "Local Store" lists the new guarded table instead of `server_object` and `combined_event_key`.

**Blocked by:** 98, 99

**Status:** ready-for-agent

**Effort:** M

- [ ] `server_object` and `combined_event_key` are gone; `caldav_server_object` holds the UID and whether it holds the Event that isn't an Exception
- [ ] Two resources with one UID, the Recurring Event in one and an invitation to one Occurrence in the other: the Recurring Event's is the Corresponding Server Object, whichever was mirrored first
- [ ] When the Corresponding Server Object goes, the other becomes it and its Events take the Event Rows' place, as before
- [ ] A resource holding two UIDs is mirrored without Event Rows and shows no Events
- [ ] A Server Object moved to another href in one Incoming Sync keeps its Event Rows, as before
- [ ] `EventKeyInCalendar` and `rewrite_combined_event_keys` are gone
- [ ] `CODING_STANDARDS.md` "Local Store" lists `caldav_server_object` as guarded by the Sync Engine
