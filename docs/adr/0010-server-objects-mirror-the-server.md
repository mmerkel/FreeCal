# Server objects mirror the server

A server object is the unit a server stores and FreeCal writes back: one CalDAV resource at its href, or one Google event resource. Its row mirrors the server's last known state of it (address, version tag, data as received), and only a sync or a successful write to the server changes it. Event rows refer to the object they came from or will be written to. A read-only feed has no server objects, because FreeCal never writes to it.

We store server objects in a table of their own instead of on the Recurring Event's row, because the object is not always one Recurring Event: a CalDAV object may hold only Exceptions (an invitation to one Occurrence), and Google keeps each Exception as a resource of its own. Sync matches incoming objects by href, and each object updates only its own rows; the UID still groups a Calendar's rows into a Recurring Event and its Exceptions, for every Provider. A Google object's data is the JSON as received, since converting it to iCalendar would lose the fields it is kept for.

The Change Journal records Event rows only, with the object each refers to, never the objects. What FreeCal has to send is where rows and object differ: an object marked pending by a user action, an undo or an import; rows without an object (not created on the server yet); an object without rows (a delete not sent yet). A move to another Calendar is a delete in the old one and a new object in the new one. Undo therefore writes back FreeCal's fields and leaves the server's version, ETag included, to the mirror, so that the next write is conditional on what the server has now.

## Considered Options

- **Server data on the Recurring Event's row** (ticket 81): objects holding only Exceptions had nowhere to live and were dropped, Google's per-Exception resources didn't fit, and two objects with one UID took each other's row on every sync without a sync token.
- **Objects journaled as part of each Event's version**: undo restored an ETag the server no longer had, so the write after an undo of a pulled change failed as a false Conflict; and objects shared by several rows needed entries, hashes and an undo order of their own.
- **Mirror, plus the object data of each version in the journal**: exact undo of fields FreeCal doesn't understand, at the cost of two copies of each object and a merge of "my unknown fields" with the server's on every write.
- **Grouping by an internal group ID instead of the UID**: the UID lookups move to sync and import rather than going away, Google's paging and late-arriving Recurring Events need group merges, and a group ID that disagrees with the UID is wrong in a way neither shows. It only helps where RFC 4791 forbids the data.
- **Finding pending changes by comparing rows with the object**: FreeCal changes what it stores (pinned zones, normalised recurrence IDs, rules it can't expand), so unchanged Events would look changed.

## Consequences

- Undo restores the fields FreeCal manages exactly; fields it doesn't understand keep the server's current values (amends ADR 0004).
- An object FreeCal can no longer read keeps its last readable version in the mirror, so rows and mirror agree; it is fetched again on each sync until it reads, and a write meanwhile meets a Conflict.
- A server change arriving while an object is pending is a Conflict, never a silent replacement.
- Undoing a delete already sent creates the Event again under a new href.
- Object IDs, like Event IDs, are never reused.

## Amendments

- 2026-10-25, ADR 0011: a Calendar's rows with one UID belong to the one object holding it, found through the objects' keys instead of an object ID on each row, so objects that break RFC 4791 are no longer kept with rows of their own. The Change Journal no longer records any object, and what FreeCal has to send is a list of unsent changes instead of a pending mark on objects.
