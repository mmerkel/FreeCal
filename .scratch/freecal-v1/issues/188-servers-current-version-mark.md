# 188: The server's Current-Version Mark: an Incoming Sync records what the server holds

**What to build:** The server's side of the Calendar History (ADRs 0018, 0019): the server's Current-Version Mark, so that the Local Store holds what each Calendar's server last had as Event Versions. Old ticket 151 (`git show design/snapshot-line:.scratch/freecal-v1/issues/151-servers-current-version-mark.md`) as written. Propagate still decides what the user's side takes until 189, and the Sender still reads the Unsent Changes until 192.

- **The mark:** `event_version.server_mark` (1 or empty), with its own partial unique index per Calendar, UID and recurrence ID. 179's update trigger allows setting and clearing it. An Event Version both sides agree on is one row: the server's mark moves to the user's row (ADR 0018, "Event Versions"). A trigger refuses to delete an Event Version carrying it, except in the removal of its Calendar.
- **An Incoming Sync**, in its Database Transaction: what Propagate takes in is its Incoming Sync Snapshot (179), and the server's marks move to the fetched versions, with their ETags. The marks move only in that Database Transaction (ADR 0019, "The invariant"). The First Sync's pages, until 194, do the same: their versions carry the server's mark.
- **An Outgoing Sync. Glue until 192, named:** each send of a UID that succeeds moves, in the Database Transaction that stores its new ETag, the server's marks of that UID to the user's versions that were sent. No Snapshot is made for it.
- **The stored ETag** is written in the same Database Transaction as the Event Versions with the server's mark it belongs to. A Server Object FreeCal can no longer read keeps its Event Versions and loses its ETag, as now.
- **Dropping** (185): the Snapshot with which a version carrying the server's mark ended is a root.
- **The Store Check** gains: every stored ETag belongs to a UID with Event Versions carrying the server's mark, and every Event Version with the server's mark has a stored ETag where its Provider gives one.
- No change to what the user sees.

Spec: US-81; "Calendar History"; "Sync engine". ADRs 0018 ("Event Versions", "Keeping and dropping", "Removing a Calendar"), 0019 ("The invariant", "Server Objects"). Layout: "Tables (end state)" (`event_version.server_mark`, the Current-Version Marks); "By phase", P3, 188.

**Blocked by:** 185, 187

**Status:** ready-for-agent

**Effort:** S

- [ ] `server_mark` with its unique index; the trigger allows it; deleting a version carrying it is refused outside a Calendar's removal
- [ ] After any Incoming Sync, the Event Versions with the server's mark read as the server's resources do (test against Radicale, also with Unsent Changes in the Calendar)
- [ ] The marks and ETags move in the Incoming Sync's Database Transaction only; one that changes nothing makes no Snapshot
- [ ] After a send that succeeds, the server's marks of its UID are on the versions sent
- [ ] An Event Version with the server's mark older than 90 days is kept, with the Snapshot it ended with
- [ ] The Store Check's new queries, each with a test that breaks it by hand
- [ ] The existing sync, send and undo tests pass unchanged
