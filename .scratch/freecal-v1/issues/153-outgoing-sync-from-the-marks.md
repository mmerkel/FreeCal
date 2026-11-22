# 153: An Outgoing Sync sends what differs between the Current-Version Marks; `unsent_change` goes

**What to build:** What to send is derived, not recorded (ADR 0019): the Events whose current Event Version differs from the one with the server's, or exists on one side only. An Outgoing Sync records its outcome in one closing Database Transaction. The list of Unsent Changes and the Sent Event Rows come out, with the glue of tickets 147, 151 and 152. This is step 3 of the order in which the old model goes.

- **What to send:** read at the start of the Outgoing Sync, with the Event Versions that were current then, which are "what was sent". Versions on both sides are an update, on the user's side only a create, on the server's side only a delete. On CalDAV the Sender sends per Calendar and UID, one resource for all that differs in it.
- **May it send:** under the rules the Sync Engine's entry has today for a Server read-only Calendar (`sync::sync_calendar`, `core/src/sync/mod.rs`).
- **The requests** run outside any Database Transaction. Every update and delete is conditional on the stored ETag.
- **The closing Database Transaction,** one per Outgoing Sync:
  - stores the new ETags and moves the server's mark of each Event sent to the version that was sent;
  - leaves what wasn't sent as it is: it still differs and is sent next time. No Snapshot is made.
- **A change the user makes while an Outgoing Sync runs** ends the sent version and is simply still different afterwards. No Sent Event Rows are needed.
- **FreeCal stopped in the middle** of an Outgoing Sync: nothing was recorded, the next Incoming Sync finds changed ETags and fetches, and until then the Events show as not sent.
- **An unsent change, as the user sees it,** is such a difference. Everything that reads the list today reads the marks instead: the Unsent Mark's data on an Event and an Occurrence, the warning when quitting, the Sync Engine's check for a Server read-only Calendar with changes to send. "Since when" is the time of the Snapshot with which the user's Event Version began.
- **Why a send failed.** The reason the Sender's last try for a UID didn't succeed (ticket 124, shown in the Event's details) is kept today on the Unsent Change. It moves to a table of its own, by Calendar and UID, named after its existing type, written in the closing Database Transaction and removed when the UID no longer differs. Chosen here, not in an ADR: the reason is the one thing about an unsent change that can't be derived.
- **What comes out:** `unsent_change` and its indexes; the recording of Unsent Changes at each change (ticket 147's glue), per sent UID (ticket 151's) and after the comparison (ticket 152's); the Sent Event Rows.
- **The Mass-Change Guard** is ticket 22: it will check this difference before sending.
- **Docs:** `GLOSSARY.md` revises Unsent Change, Outgoing Sync, Sender, Combined Event Key, Read-only Calendar and User's Version where they speak of a kept list, and loses Sent Event Rows.

Spec: US-62, US-63, US-72, US-131; "Sync engine"; "Local Store". ADR 0019 ("What to send", "An Outgoing Sync").

**Blocked by:** 152

**Status:** ready-for-agent

**Effort:** M

- [ ] The Sender reads what to send from the two marks; `unsent_change` and the Sent Event Rows are gone from the code and the schema
- [ ] One closing Database Transaction per Outgoing Sync records the ETags and the server's marks of what was sent
- [ ] After a full send the server's marks are on the versions that were sent, an Undo's too, and nothing differs
- [ ] After a partial send (one UID refused) the server's marks moved for what was sent only, and the refused UID still differs
- [ ] A change made while the requests run is sent by the next Outgoing Sync (test with a held request)
- [ ] An edit and its undo before any sync send nothing
- [ ] An edit and an edit back by hand to the server's content send nothing and show no Unsent Mark
- [ ] A stop between the requests and the closing Database Transaction is repaired by the next Incoming Sync
- [ ] The Unsent Mark's data, the quit warning and the reason a send failed read as before through the core interface
- [ ] The Store Check passes after every scenario; `GLOSSARY.md` follows

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Third of four parts (see ticket 151). Against the split as first listed, putting the server's form of a write in place is in ticket 154, with the read-back that first makes it reachable on CalDAV.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). The Server's Pointer no longer moves backwards.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): no pointers or Reference Snapshot; a partial Outgoing Sync moves the marks of what was sent and makes no Snapshot. Nothing holds an Outgoing Sync back for a Conflict.
