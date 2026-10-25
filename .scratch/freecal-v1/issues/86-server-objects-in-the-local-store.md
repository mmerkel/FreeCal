# 86: Server objects in the Local Store

**What to build:** Original server data moves from the Recurring Event's row into server objects that mirror the server (ADR 0010), and the Change Journal stops recording it. Sync still matches by UID here; ticket 87 makes it match by href. Decided in a grilling on 2026-10-24.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] A `server_object` table (`id INTEGER PRIMARY KEY AUTOINCREMENT`, the Calendar, href, ETag, data as received and its format, a pending mark) and an optional `object_id` on each Event row; a migration moves each row's `server_href`, `server_etag` and `server_data` into an object and drops those columns from `event` and `journal_entry`
- [x] The type's doc comment defines a server object with ADR 0010's first paragraph
- [x] The Change Journal records rows only, with their `object_id`; a new hash format covers `object_id` instead of the server data, with `journal::rehash`. A sync that changes only an object's ETag or data changes no row, so it doesn't block undoing earlier units
- [x] `Unit::with_server_data` goes. A user action, an undo or an import that changes an object's rows marks the object pending; a sync never does. A move to another Calendar leaves the old object without the moved rows and the moved rows without an object
- [x] Undo restores a row's `object_id` while the object exists, and none otherwise, so the row counts as not yet on the server
- [x] Sync writes the objects it fetches and points the rows it takes at them; `Core::original_server_data` reads from the object
- [x] Core tests for each point, including an undo of a pulled change that leaves the mirror's ETag as the server gave it, and the hostile test Event's data stored unchanged in its object

Out of scope: matching by href, unreadable objects (ticket 87); sending pending objects (tickets 17, 18); Google objects (ticket 25/26 store their JSON as received, per ADR 0010).

## Comments

- 2026-10-24: Split with ticket 87 from one "server objects" ticket: once rows no longer hold the server data, the journal and hash change with them, so this ticket is schema, journal and `Unit`, and 87 is sync behaviour.
- 2026-10-25: Done. Objects are `ServerObject` (href, ETag, data) in `server_object`, and `EventFields.object_id` replaces `server_data`. `Unit::placed` decides a row's object, replacing `with_server_data`. Beyond the ticket, a new row from a user action or an import joins the object of its Calendar's rows with its UID, as a CalDAV object holds a Recurring Event with its Exceptions. Google, whose Exceptions are objects of their own, will need another rule. `Unit::entry` marks objects pending. Sync writes objects with `Unit::mirror` (upsert by Calendar and href) and removes them with `Unit::remove_objects`. Format-3 hashes covered columns the migration drops, so the migration re-hashes them itself (`journal::rehash_server_data`), and `journal::rehash` now recognises formats 1 and 2. The migration points Exceptions at their Recurring Event's object. Their journal entries, like those of rows without an object at their href, get none. Tests of the pending marks are unit tests in `journal.rs`.
