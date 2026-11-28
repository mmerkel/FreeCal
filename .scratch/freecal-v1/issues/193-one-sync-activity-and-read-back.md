# 193: One sync activity per Calendar; a write answered without an ETag is read back; the server's form takes the sent one's place

**What to build:** The last part of the sync from the marks (ADR 0019, "An Outgoing Sync"): a Calendar has one sync activity at a time, and when a server stores another form of an Event than FreeCal sent, that form takes the sent Event Version's place. Old ticket 154 (`git show design/snapshot-line:.scratch/freecal-v1/issues/154-one-sync-activity-and-read-back.md`) as written.

- **One sync activity per Calendar:** an Incoming Sync waits for a running Outgoing Sync of the same Calendar, and the other way round, whoever starts it (the Sync Scheduler, "sync now", the sync after adding a Calendar). It is a property of the Sync Engine's entry (`sync_both_ways`, `sync/mod.rs`). 192's closing Database Transaction relies on it: no Incoming Sync moves the server's marks between an Outgoing Sync's requests and its record.
- **An answer with an ETag** means the text was stored unchanged (RFC 4791 §5.3.4): nothing more is done.
- **An answer without an ETag:** the Sender reads the resource back before the closing Database Transaction and stores the ETag it then gets. If it equals what was sent, as Event Versions, nothing else happens. If the Events sent come back in another form, that is the server's form of FreeCal's own write, no change by anyone. Events in that read-back FreeCal didn't send are someone else's change and go through 189's plan and writer as an Incoming Sync Snapshot.
- **The server's form in place:** written in the closing Database Transaction by one function of `history/` as a new Event Version with the sent one's `begin` and `end`, carrying the server's mark; the sent one is deleted. The one case in which an Event Version is replaced (ADR 0018, "Who may write"); no Snapshot is made. A comment at the function names that rule; 179's and 188's triggers allow it only there. Google's answer to a write uses the same function (213).
- **Consequences to test (ADR 0019):** an Undo after such a write goes back to the Snapshot before it and doesn't loop; a Snapshot older than the Outgoing Sync shows the server's form; a change the user made while the Outgoing Sync ran was made on the sent form and is sent as the user made it.
- **Tests** use `RewritingProxy` (`core/tests/support/radicale.rs`): it strips the ETag from the answer to a write, and in a second scenario also changes the stored resource.

Spec: "Sync engine". ADRs 0018 ("Who may write"), 0019 ("An Outgoing Sync"). Layout: "Seams for `/tdd`", 3; "By phase", P3, 193.

**Blocked by:** 192

**Status:** ready-for-agent

**Effort:** M

**Switched-off tests:** turns on and rewrites `core/tests/caldav_send.rs`'s `a_write_answered_without_an_etag_*`, switched off by 189 with "until ticket 193".

- [ ] An Incoming Sync and an Outgoing Sync of one Calendar never overlap, whoever starts them (test with a held request)
- [ ] A write answered without an ETag is read back; the stored ETag is the read-back's
- [ ] A server's form that differs takes the sent Event Version's place in every Snapshot; the sent one is gone; no Snapshot is added
- [ ] After it, the two sides agree and the next Outgoing Sync sends nothing
- [ ] An Undo after it goes back one step of the user's and doesn't come back to the same state
- [ ] Someone else's change found in the read-back makes an Incoming Sync Snapshot
- [ ] The test above on again; the Store Check passes after every scenario
