# Server Objects mirror the server

A Server Object is what a server stores as one piece and FreeCal writes back: one CalDAV resource at its href, or one Google event resource. Its row mirrors the server's last known state of it (address, version tag, data as received), and only a sync or a successful write to the server changes it. Event Rows refer to the Server Object they came from or will be written to. A read-only feed has no Server Objects, because FreeCal never writes to it.

We store Server Objects in a table of their own instead of on the Recurring Event's Event Row, because the Server Object is not always one Recurring Event: a CalDAV Server Object may hold only Exceptions (an invitation to one Occurrence), and Google keeps each Exception as a resource of its own. Sync matches incoming Server Objects by href, and each Server Object updates only its own Event Rows; the UID still groups a Calendar's Event Rows into a Recurring Event and its Exceptions, for every Provider. A Google Server Object's data is the JSON as received, since converting it to iCalendar would lose the fields it is kept for.

The Change Journal records Event Rows only, with the Server Object each refers to, never the Server Objects. What FreeCal has to send is where Event Rows and Server Object differ: a Server Object marked pending by a user action, an undo or an import; Event Rows without a Server Object (not created on the server yet); a Server Object without Event Rows (a delete not sent yet). A move to another Calendar is a delete in the old one and a new Server Object in the new one. Undo therefore writes back FreeCal's fields and leaves the server's version, ETag included, to the Mirror, so that the next write is conditional on what the server has now.

## Considered Options

- **Server data on the Recurring Event's Event Row** (ticket 81): Server Objects holding only Exceptions had nowhere to live and were dropped, Google's per-Exception resources didn't fit, and two Server Objects with one UID took each other's Event Row on every sync without a sync token.
- **Server Objects journaled as part of each Event Version**: undo restored an ETag the server no longer had, so the write after an undo of a pulled change failed as a false Conflict; and Server Objects shared by several Event Rows needed Journal Event Modifications, hashes and an undo order of their own.
- **Mirror, plus the Server Object data of each Event Version in the journal**: exact undo of fields FreeCal doesn't understand, at the cost of two copies of each Server Object and a merge of "my unknown fields" with the server's on every write.
- **Grouping by an internal group ID instead of the UID**: the UID lookups move to sync and import rather than going away, Google's paging and late-arriving Recurring Events need group merges, and a group ID that disagrees with the UID is wrong in a way neither shows. It only helps where RFC 4791 forbids the data.
- **Finding pending changes by comparing Event Rows with the Server Object**: FreeCal changes what it stores (Pinned Zones, normalised recurrence IDs, rules it can't expand), so unchanged Events would look changed.

## Consequences

- Undo restores the fields FreeCal manages exactly; fields it doesn't understand keep the server's current values (amends ADR 0004).
- A Server Object FreeCal can no longer read keeps its last readable version in the Mirror, so Event Rows and Mirror agree; it is fetched again on each sync until it reads, and a write meanwhile meets a Conflict.
- A server change arriving while a Server Object is pending is a Conflict, never a silent replacement.
- Undoing a delete already sent creates the Event again under a new href.
- Server Object IDs, like Event IDs, are never reused.

## Amendments

- 2026-10-25, ADR 0011: a Calendar's Event Rows with one UID belong to their Corresponding Server Object, found through the Server Objects' Combined Event Keys instead of a Server Object ID on each Event Row, so Server Objects that break RFC 4791 are no longer kept with Event Rows of their own. The Change Journal no longer records any Server Object, and what FreeCal has to send is a list of Unsent Changes instead of a pending mark on Server Objects.
