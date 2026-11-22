# 154: One sync activity per Calendar; a write answered without an ETag is read back, and the server's form takes the sent one's place

**What to build:** The last part of the sync on two pointers (ADR 0019): the rule that a Calendar has one sync activity at a time, and what FreeCal does when a server stored another form of an Event than FreeCal sent.

- **One sync activity per Calendar:** an Incoming Sync waits for a running Outgoing Sync of the same Calendar, and the other way round. It is a property of the Sync Engine's entry (`sync::sync_calendar`, `core/src/sync/mod.rs`), held whoever starts a sync: the Sync Scheduler, "sync now", or the sync that follows adding a Calendar. The closing Database Transaction of ticket 153 relies on it: no Incoming Sync moves the server's marks between an Outgoing Sync's requests and its record.
- **A CalDAV answer with an ETag** means the text was stored unchanged (RFC 4791 §5.3.4): nothing more is done.
- **An answer without an ETag:** FreeCal reads the resource back before the closing Database Transaction and stores the ETag it then gets.
  - If what it reads equals what it sent, as Event Versions, nothing else happens.
  - If the Events it sent come back in another form, that form is the server's form of FreeCal's own write, and no change by anyone.
  - Events in that fetch that FreeCal didn't send are someone else's change and make a sync Snapshot, through the Incoming Sync's two steps.
- **The server's form in place:** it is written as a new Event Version with the sent one's `begin` and `end`, which takes the server's mark, and the sent one is deleted, in the closing Database Transaction, by one function of the Calendar History's module. It is the one case in which an Event Version is replaced (ADR 0018); no Snapshot is made for it. Google's answer to a write will use the same function (ticket 26).
- **Consequences to test, from ADR 0019:** an undo after such a write goes back to the Snapshot before it and doesn't loop (the reason a Snapshot of its own was rejected); a Snapshot older than the Outgoing Sync shows the server's form; a change the user made to that Event while the Outgoing Sync ran was made on the sent form and is sent as the user made it.
- **Tests** use the proxy that rewrites the server's answers (`RewritingProxy`, `core/tests/caldav_sync.rs`): it strips the ETag from the answer to a write, and in a second scenario also changes the stored resource.

Spec: "Sync engine". ADRs 0018 ("Who may write"), 0019 ("An Outgoing Sync").

**Blocked by:** 153

**Status:** ready-for-agent

**Effort:** M

- [ ] An Incoming Sync and an Outgoing Sync of one Calendar never overlap, whoever starts them (test with a held request)
- [ ] A write answered without an ETag is read back; the stored ETag is the read-back's
- [ ] A server's form that differs takes the sent Event Version's place in every Snapshot; the sent one is gone; no Snapshot is added
- [ ] After it, the two sides agree and the next Outgoing Sync sends nothing
- [ ] Undo after it goes back one step of the user's and doesn't come back to the same state
- [ ] Someone else's change found in the read-back makes a sync Snapshot
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Fourth of four parts (see ticket 151). With it, steps 0 to 3 of the order in which the old model goes are done.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): the server's form replaces the sent row with the same `begin` and `end`, as there are no links.
