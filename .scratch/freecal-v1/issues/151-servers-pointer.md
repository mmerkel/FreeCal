# 151: The server's Current-Version Mark: an Incoming Sync records what the server holds

**What to build:** The server's side of the Calendar History (ADRs 0018 and 0019, rewritten 2026-11-22): the server's Current-Version Mark, so that the Local Store holds what each Calendar's server last had as Event Versions. Propagate still decides what the user's side takes; the comparison against the server's marks replaces it in ticket 152, and the Sender still reads the Unsent Changes until ticket 153.

- **The server's Current-Version Mark** is a column of `event_version`, with its own unique index per Calendar, UID and recurrence ID. The trigger allows it to be set and cleared, beside ticket 146's two late changes. An Event Version both sides agree on is one row: the server's mark moves to the user's row (ADR 0018, "Event Versions").
- **An Incoming Sync**, in one Database Transaction: what Propagate takes in is one Server Intake, a Snapshot with the origin Incoming Sync, and the server's marks move to the fetched versions, with their ETags. A Server Intake that would change nothing isn't made. The server's marks move only in that Database Transaction (ADR 0019, "The invariant").
- **A First Sync** is one Server Intake whose versions all carry the server's mark (still built as in ticket 146 until ticket 155 makes it a Full Fetch).
- **An Outgoing Sync.** Glue until ticket 153, named: each send of a UID that succeeds moves, in the Database Transaction that stores its new ETag, the server's marks of that UID to the user's versions that were sent. No Snapshot is made for it.
- **The stored ETag** of a kept resource is written in the same Database Transaction as the Event Versions with the server's mark it belongs to. A Server Object FreeCal can no longer read keeps its Event Versions and loses its ETag, as now.
- **The Store Check** gains: every stored ETag belongs to a UID with Event Versions carrying the server's mark, and every Event Version with a server's mark has a stored ETag where its Provider gives one.
- **Dropping** (ticket 148) keeps an Event Version with the server's mark, and with it the Snapshot it ended with; its test comes here.
- No change to what the user sees.

Spec: US-81; "Calendar History"; "Sync engine". ADRs 0018 ("Event Versions", "Keeping and dropping"), 0019 ("The invariant", "Server Objects").

**Blocked by:** 148, 150

**Status:** ready-for-agent

**Effort:** S

- [ ] The server's Current-Version Mark with its unique index; the trigger allows it
- [ ] After any Incoming Sync, the Event Versions with the server's mark read as the server's resources do (test against Radicale, also with Unsent Changes in the Calendar)
- [ ] An Incoming Sync makes one Server Intake and moves the server's marks in the same Database Transaction; one that changes nothing makes no Snapshot
- [ ] After a send that succeeds, the server's marks of its UID are on the versions sent
- [ ] Ctrl+Z passes over a Server Intake; an Undo of one from the core takes back the server's changes
- [ ] An Event Version with the server's mark older than 90 days is kept, with the Snapshot it ended with
- [ ] The Store Check's new queries, each with a test that breaks it by hand
- [ ] The existing sync, send and undo tests pass unchanged

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of four parts of what was proposed as one ticket (XL): 151 the Server's Pointer, 152 the Reference Snapshot and the comparison, 153 the Outgoing Sync from the marks, 154 one sync activity and the read-back.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: A Join with one parent replaces the merge of an Incoming Sync; merges with two parents come only from ending a Conflict (grilling, ADR 0019).
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo and redo at a merge moved to ticket 134, as merges only end Conflicts; there are no forks to redo through.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): one line of Snapshots, so no Server's Pointer, no server Snapshots and no Join; the server's mark is the record of what the server holds. Re-sized from M to S. The comments above describe the tree and are history; the file name keeps the old title.
