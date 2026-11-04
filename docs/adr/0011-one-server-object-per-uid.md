# One Server Object per UID, and the Change Journal doesn't know Server Objects

Two concerns are kept apart. A Recurring Event and its Exceptions belong together because they share a Calendar and a UID, for every Provider; the `event` table and the Change Journal hold that, and nothing about servers. How a server lays those Event Rows out in resources depends on the Provider, and only the Mirror and the Provider code hold it.

A Calendar's Event Rows with one UID belong to their Corresponding Server Object, the one Server Object that holds that UID: on CalDAV the Server Object holding the UID, as RFC 4791 requires all components with one UID to share a resource; on Google the Server Object holding that UID and recurrence ID, since each Exception is a resource of its own. The link is not stored on the Event Row. A sync fills `combined_event_key` from each Server Object's data, one Combined Event Key per component, and Event Rows find their Server Object through it. A Combined Event Key holds the recurrence ID normalised exactly as the Event Rows hold it, for every Provider, and the Store renormalises a UID's Combined Event Keys in the same step as its Event Rows. When two Server Objects in a Calendar hold one UID, the one holding the Recurring Event is the Corresponding Server Object, otherwise the first one mirrored. The other is mirrored without Event Rows, to be reported as a problem of its Calendar, and becomes the Corresponding Server Object, from its mirrored data, when the first one goes.

What FreeCal has to send is a list of Combined Event Keys in `unsent_change` (Calendar, UID, recurrence ID, since when), which every change but a sync adds in a writable server Calendar, for the Event's previous Combined Event Key and its new one. The Sender compares each Combined Event Key's Event Rows with its Server Object: Event Rows and a Server Object is an update, Event Rows without a Server Object a create, a Server Object without Event Rows a delete. Only a sync and a successful send change the Mirror's server data.

Event Versions and their hashes hold FreeCal's fields only. An Event Row getting a Server Object, or moving to another, doesn't change the Event, so it is neither recorded nor in the way of an undo. Journaling is an invariant of the Local Store: the module that writes the Change Journal is the only code that writes Event Rows, Server Objects, Combined Event Keys and Unsent Changes, and the module structure enforces that.

## Considered Options

- **The Server Object's ID on each Event Row, in the journaled Event Version** (ticket 86): the Event Row had to get its Server Object somewhere, so the journal module decided placement for new Event Rows and undos; a create reaching the server would either have been a Modification that changes nothing visible, or would have broken the hash; and a server rearranging its resources recorded changes the user can't see.
- **The Server Object's ID on each Event Row, outside the journal, plus a key table for Event Rows without one**: exact for Server Objects that break RFC 4791, but it keeps a stored link, a key table and placement rules side by side for two edge cases.
- **A `uid` column on `server_object`**: Server Objects can hold several UIDs, and Google needs the recurrence ID.
- **Recurrence IDs in the Combined Event Keys only for Google**: CalDAV wouldn't need to renormalise them, but one column would mean different things per Provider. One rule for every Provider was preferred to the work it saves, since long-term consistency counts for more than short-term ease in code agents write.
- **A pending mark on the Combined Event Keys or the Server Objects**: a new Event has no Server Object or Combined Event Key yet, a sync rewrites a Server Object's Combined Event Keys and would take the mark with it, and FreeCal's own changes would write the Mirror.

## Consequences

- A Server Object that holds a UID whose Corresponding Server Object is another one isn't shown, though other clients may show it. That reverses ticket 88's decision to keep both copies of an Exception in different Server Objects.
- Undoing a delete or a move not sent yet puts the Event Row back where its Server Object still is, so nothing is sent for it.
- A Conflict is a server change to a Server Object while one of its Combined Event Keys is in `unsent_change`.
- The hash of an Event Version changes format; no Local Store needs rehashing.
