# 198: The Server Event Merge combines changes to different Field Groups

**What to build:** Since 189 every Event changed differently on both sides is replaced by the server's version, even where 177's `compare_event` could combine it. Now an Incoming Sync combines those in a **Server Event Merge**, within its Incoming Sync Snapshot (ADR 0019, "A Server Event Merge"; ADR 0018, "Zero-length Event Versions"). The core part of old ticket 168 (`git show design/snapshot-line:.scratch/freecal-v1/issues/168-server-event-merge.md`); its toast is 206's.

- **When:** `compare_event` gives a combination: each Field Group changed on at most one side, the times only against the Unimplemented Fields, within one Event Version (177's rule; a change to a Recurring Event against one to an Occurrence stays a Conflict). Never where 195's plan flags the base as untrusted.
- **What is written,** by the `IncomingSyncWriter` in the Incoming Sync Snapshot's Database Transaction (`sync/plan.rs`'s merge branch): the server's version begins and ends with the Snapshot, a zero-length Event Version carrying only the server's mark, as the combination is no version the server holds; the combination begins with it on the user's side. No Conflict, no row of `conflict`, no record of its own: a Server Event Merge is recognised by its zero-length Event Versions. 179's trigger already allows `end = begin` only at an Incoming Sync Snapshot.
- **Sent like any change:** the combination differs from the version with the server's mark, so the next Outgoing Sync sends it (and 201's Mass-Change Guard counts it).
- **Undo:** nothing of its own. An Undo of the Incoming Sync Snapshot limited to the Event (181) brings back the user's version without the server's change and is sent; a whole Undo takes back its Server Event Merges with the rest.
- **Reported:** the Signal of each Incoming Sync carries its Server Event Merges (the Combined Event Keys), for 206's toast; regenerate the TypeScript types and give the fake core its answer. No desktop notification.
- **An Event's History** (184) shows the zero-length Event Version as "on the server, before combining", before the combination. The Calendar History's line for it is 207's.
- **The Store Check** gains: a zero-length Event Version only beside a version of the same Event beginning with the same Snapshot.

Spec: US-73; "Sync engine"; "Calendar History"; "Untrusted content". ADRs 0017 ("Part of the version"), 0018 ("Zero-length Event Versions", "Snapshots"), 0019 ("Whether the server changed", "An Incoming Sync", "A Server Event Merge"), 0020 ("Limited to one Event"). Layout: "Interfaces designed twice", "The comparison", "The Incoming Sync"; "Seams for `/tdd`", 1 and 3; "By phase", P4, 198.

**Blocked by:** 190

**Status:** ready-for-agent

**Effort:** M

- [ ] An Incoming Sync combines each Event `compare_event` combines in its Incoming Sync Snapshot, with the server's version as a zero-length Event Version, no Conflict and no row of `conflict`
- [ ] Where the base is untrusted (195), no Server Event Merge is made, even when only different Field Groups differ
- [ ] The combination is sent at the next Outgoing Sync; an Undo of the Incoming Sync Snapshot limited to the Event restores the user's version and sends it; a whole Undo takes the merge back with the rest
- [ ] The Signal reports each Incoming Sync's Server Event Merges
- [ ] An Event's History shows "on the server, before combining"
- [ ] The Store Check's query for zero-length Event Versions, with a test that breaks it by hand
- [ ] The merge branch unit-tested in `sync/plan.rs`; through the core interface against Radicale, test-first: the user changes the title and the server adds an alarm (an `ical:` entry): the user's title with the server's alarm, no Conflict, the merge reported, and after the Outgoing Sync the server holds both; the title against the Location, combined; the times against the title, a Conflict; the times against an alarm, combined; the Unimplemented Fields on both sides, a Conflict; an Incoming Sync whose only change is merged still makes its Snapshot
