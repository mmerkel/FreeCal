# 192: An Outgoing Sync sends what differs between the Current-Version Marks; `unsent_change` goes

**What to build:** What to send is derived, not recorded (ADR 0019): the Events whose current Event Version differs from the one with the server's mark, or exists on one side only. An Outgoing Sync records its outcome in one closing Database Transaction. The list of Unsent Changes and the Sent Event Rows come out, with the glue of 182, 188 and 189. Old ticket 153 (`git show design/snapshot-line:.scratch/freecal-v1/issues/153-outgoing-sync-from-the-marks.md`) as written.

- **What to send:** read at the start of the Outgoing Sync by a reader in `history/`, with the Event Versions current then, which are "what was sent". Versions on both sides are an update, on the user's side only a create, on the server's side only a delete. On CalDAV the Sender sends per Calendar and UID, one resource for all that differs in it. Whether it may send: the rules the Sync Engine has today for a Server read-only Calendar (`sync::sync_calendar`).
- **The requests** run outside any Database Transaction. Every update and delete is conditional on the stored ETag.
- **The closing Database Transaction,** one per Outgoing Sync: stores the new ETags and moves the server's mark of each Event sent to the version that was sent; leaves what wasn't sent as it is, still different and sent next time. No Snapshot is made.
- **A change the user makes while an Outgoing Sync runs** ends the sent version and is simply still different afterwards; no Sent Event Rows are needed.
- **FreeCal stopped in the middle:** nothing was recorded; the next Incoming Sync finds changed ETags and fetches, and until then the Events show as not sent.
- **An Unsent Change, as the user sees it,** is such a difference. Everything that reads the list today reads the marks instead (`store/sql/mod.rs`'s readers rewritten): the Unsent Mark's data on an Event and an Occurrence, the warning when quitting, the Sync Engine's check for a Server read-only Calendar with changes to send. "Since when" is the time of the Snapshot with which the user's Event Version began. The core interface reads as before.
- **Why a send failed** (ticket 124, shown in the Event's details), kept today on the Unsent Change, moves to a table of its own by Calendar and UID, named after its existing type, written in the closing Database Transaction and removed when the UID no longer differs. Chosen here, not in an ADR: the reason is the one thing about an unsent change that can't be derived.
- **What comes out:** `unsent_change` and its indexes; the `SnapshotWriter`'s recording of Unsent Changes and the Sender's hooks (182), the move of the marks per sent UID (188), the setting after an Incoming Sync (189); the Sent Event Rows.

Spec: US-62, US-63, US-72, US-131; "Sync engine"; "Local Store". ADR 0019 ("What to send", "An Outgoing Sync"). Layout: "Tables (end state)" (the Current-Version Marks); "By phase", P3, 192.

**Blocked by:** 189

**Status:** ready-for-agent

**Effort:** M

**Switched-off tests:** turns on and rewrites the tests of `core/tests/caldav_send.rs` that 189 switched off with "until ticket 192" (`an_edit_is_not_written_over_a_newer_*`, `an_update…`, `a_delete_without_a_stored_etag_waits`); `a_write_answered_without_an_etag_*` stays off until 193. **Rewritten:** `core/tests/offline_editing.rs`; the tests in `sync/caldav/sender.rs` on resource naming and refusals. **Deleted tests:** the Unsent Changes and Sent Event Rows group of `caldav_send.rs`; the stored send-failure tests of `sender.rs`.

- [ ] The Sender reads what to send from the two marks; `unsent_change` and the Sent Event Rows are gone from the code and the schema
- [ ] One closing Database Transaction per Outgoing Sync records the ETags and the server's marks of what was sent
- [ ] After a full send the server's marks are on the versions that were sent, an Undo's too, and nothing differs; after a partial send (one UID refused) they moved for what was sent only
- [ ] A change made while the requests run is sent by the next Outgoing Sync (test with a held request)
- [ ] An edit and its Undo before any sync send nothing; an edit and an edit back by hand to the server's content send nothing and show no Unsent Mark
- [ ] A stop between the requests and the closing Database Transaction is repaired by the next Incoming Sync
- [ ] The Unsent Mark's data, the quit warning and the reason a send failed read as before through the core interface; `src/App.quit.test.ts` passes unchanged
- [ ] The tests above on again, rewritten or deleted; the Store Check passes after every scenario
- [ ] `GLOSSARY.md` follows: Unsent Change, Outgoing Sync, Sender, Combined Event Key, Read-only Calendar and User's Version where they speak of a kept list; Sent Event Rows goes
