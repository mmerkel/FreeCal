# 100: CalDAV Mirror tables

**What to build:** Agreed on 2026-11-06 (ADR 0013). The Mirror's CalDAV Server Objects get a table of their own, and the generic `server_object` and `combined_event_key` go.

- `caldav_server_object` replaces `server_object` for CalDAV: the Calendar, href, ETag, the iCalendar text as received, the unreadable mark, the UID, and whether it holds the Event that isn't an Exception (the single Event or the Recurring Event). There is no `format` column and no key table. Change the baseline schema (ADR 0008); there are no real Local Stores yet.
- A CalDAV Server Object holds exactly one UID (RFC 4791 §4.1). A resource holding several is mirrored with its first UID and without Event Rows, as a second Server Object holding a UID already is (ADR 0011).
- The Corresponding Server Object of a UID in a Calendar is found in `caldav_server_object`: of those holding the UID, the one holding the Event that isn't an Exception, otherwise the first one mirrored (`corresponding_server_object`). Corresponding UIDs and held UIDs collapse to the one UID a CalDAV Server Object holds.
- Combined Event Keys are no longer stored in the Mirror. `EventKeyInCalendar` folds into `CombinedEventKey`: a Calendar, a UID and a recurrence ID as the Event Row holds it (`GLOSSARY.md`). Rewriting a Server Object's Combined Event Keys (`rewrite_combined_event_keys`) goes with the table.
- The Sync Engine's Write Permission covers `caldav_server_object`, in `core/src/store/sql/mirror/caldav.rs`.
- `CODING_STANDARDS.md` "Local Store" lists the new guarded table instead of `server_object` and `combined_event_key`.

**Blocked by:** 98, 99

**Status:** done

**Effort:** M

- [x] `server_object` and `combined_event_key` are gone; `caldav_server_object` holds the UID and whether it holds the Event that isn't an Exception
- [x] Two resources with one UID, the Recurring Event in one and an invitation to one Occurrence in the other: the Recurring Event's is the Corresponding Server Object, whichever was mirrored first
- [x] When the Corresponding Server Object goes, the other becomes it and its Events take the Event Rows' place, as before
- [x] A resource holding two UIDs is mirrored without Event Rows and shows no Events
- [x] A Server Object moved to another href in one Incoming Sync keeps its Event Rows, as before
- [x] `EventKeyInCalendar` and `rewrite_combined_event_keys` are gone
- [x] `CODING_STANDARDS.md` "Local Store" lists `caldav_server_object` as guarded by the Sync Engine

## Comments

2026-11-06, `/implement`:

- `caldav_server_object` holds the Calendar, href, ETag, data, `unreadable`, `uid` and `holds_non_exception` (`core/src/store/schema.rs`). The Corresponding Server Object is the UID's Server Object holding the Event that isn't an Exception, otherwise the first mirrored (`Store::corresponding_server_object`, `core/src/store/sql/mod.rs`).
- Propagate now takes one UID and its Events (`event::propagate_to_event_rows`). Given no Events, it deletes the UID's Event Rows if the Server Object corresponds.
- A resource holding several UIDs is mirrored with its first UID, in the order the reader lists Events (Events that aren't Exceptions first). It counts as holding no Events of that UID (`of_its_uid`, `core/src/sync/caldav/mod.rs`), so `holds_non_exception` is false. If it is the UID's Corresponding Server Object, the UID has no Event Rows. The same applies when it becomes that UID's Corresponding Server Object later (`settle`).
- **Open question for the user.** Such a resource can still be the Corresponding Server Object when it was mirrored first. In that case, a later valid resource with only an Exception of that UID shows nothing, and a known resource that comes to hold two UIDs takes its UID's Event Rows away. The ticket's "as a second Server Object holding a UID already is" could instead mean that it never corresponds. That would need a stored mark, which is a column beyond the ones the ticket lists. I kept to the ticket's columns.
- The two-UID case is tested by the unit test `a_server_object_holding_two_uids_is_mirrored_without_event_rows` (`core/src/sync/caldav/incoming.rs`), not by an integration test: Radicale answers 404 for such a file, even when it is written into its storage. The other boxes are covered by the existing integration tests in `core/tests/caldav_sync.rs`: `the_object_holding_the_recurring_event_corresponds_whichever_came_first`, `the_next_object_corresponds_when_the_corresponding_one_goes` and `an_event_whose_object_moved_to_another_address_stays_the_same_event`.
- New terms:
  - **holds_non_exception** (new term): whether a CalDAV Server Object holds the Event of its UID that isn't an Exception. It is a column, a parameter, and a test reader.
  - **corresponding UID** (new term): the UID a Server Object holds, if it is that UID's Corresponding Server Object (`corresponding_uid`).
  - **held UID** (new term): the one UID a CalDAV Server Object holds, whether it corresponds or not (`held_uid`).
  - **of its UID** (new term): a CalDAV Server Object's UID with its Events of it, which are none if it holds several UIDs (`of_its_uid`).
