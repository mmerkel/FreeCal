# One server object per UID, and the Change Journal doesn't know server objects

Two concerns are kept apart. A Recurring Event and its Exceptions belong together because they share a Calendar and a UID, for every Provider; the `event` table and the Change Journal hold that, and nothing about servers. How a server lays those rows out in resources depends on the Provider, and only the mirror and the Provider code hold it.

A Calendar's rows with one UID belong to the one server object that holds that UID: on CalDAV the object holding the UID, as RFC 4791 requires all components with one UID to share a resource; on Google the object holding that UID and recurrence ID, since each Exception is a resource of its own. The link is not stored on the row. A sync fills `server_object_key` from each object's data, one key (UID, recurrence ID) per component, and rows find their object through it. A key holds the recurrence ID normalised exactly as the rows hold it, for every Provider, and the Store renormalises a UID's keys in the same step as its rows. When two objects in a Calendar hold one UID, the one holding the Recurring Event owns it, otherwise the first one mirrored. The other is mirrored without rows, to be reported as a problem of its Calendar, and takes over from its mirrored data when the owner goes.

What FreeCal has to send is a list of keys in `unsent_change` (Calendar, UID, recurrence ID, since when), which every change but a sync adds in a writable server Calendar, for the Event's previous key and its new one. The sender compares each key's rows with its object: rows and an object is an update, rows without an object a create, an object without rows a delete. Only a sync and a successful send change the mirror's server data.

Journal versions and their hashes hold FreeCal's fields only. A row getting an object, or moving to another, doesn't change the Event, so it is neither recorded nor in the way of an undo. Journaling is an invariant of the Local Store: the module that writes the Change Journal is the only code that writes Event rows, objects, keys and unsent changes, and the module structure enforces that.

## Considered Options

- **The object's ID on each row, in the Event's journaled version** (ticket 86): the row had to get its object somewhere, so the journal module decided placement for new rows and undos; a create reaching the server would either have been a journal entry that changes nothing visible, or would have broken the hash; and a server rearranging its resources recorded changes the user can't see.
- **The object's ID on each row, outside the journal, plus a key table for rows without one**: exact for objects that break RFC 4791, but it keeps a stored link, a key table and placement rules side by side for two edge cases.
- **A `uid` column on `server_object`**: objects can hold several UIDs, and Google needs the recurrence ID.
- **Recurrence IDs in the keys only for Google**: CalDAV wouldn't need to renormalise them, but one column would mean different things per Provider. One rule for every Provider was preferred to the work it saves, since long-term consistency counts for more than short-term ease in code agents write.
- **A pending mark on the keys or the objects**: a new Event has no object or key yet, a sync rewrites an object's keys and would take the mark with it, and FreeCal's own changes would write the mirror.

## Consequences

- An object that holds a UID another object owns isn't shown, though other clients may show it. That reverses ticket 88's decision to keep both copies of an Exception in different objects.
- Undoing a delete or a move not sent yet puts the row back where its object still is, so nothing is sent for it.
- A Conflict is a server change to an object while one of its keys is in `unsent_change`.
- The hash of a version changes format; no Local Store needs rehashing.
