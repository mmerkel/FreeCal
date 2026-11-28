# 201: The Mass-Change Guard checks every Outgoing Sync

**What to build:** Before an Outgoing Sync sends, the core counts what it would change or delete on the server; past the thresholds it sends nothing and pauses the Calendar as a Sync Error naming the numbers, which the user ends with "Send" or by undoing (ADR 0019, spec "Sync engine"). As what to send is derived since 192, this is what stands between a fault in deriving it and the server. Incoming Syncs aren't checked. The core part of old ticket 22 (`git show design/snapshot-line:.scratch/freecal-v1/issues/22-mass-change-guard.md`); the window's "Send" is 203's.

- **Where:** in the Sender (`sync/caldav/sender.rs`), after it has read what differs between the Current-Version Marks and before the first request.
- **What counts:** the Events it would change or delete on the server: those with a version carrying the server's mark whose current user's version differs or is missing. Creations don't count. The share is of the Events on the server's side. The thresholds are `mass_change_guard::exceeded` as it is (more than 10 Events **and** more than 5%).
- **Not counted:** an Event whose current version began with a Snapshot of origin import or Backup restore (read from `history/`) and doesn't carry the server's mark: the import preview or the restore's confirmation was the approval. The one place where sync reads origins. The restore's case is tested once ticket 11 makes restores.
- **Everything else counts:** an Undo or redo (of an import too), an Undo of an Incoming Sync Snapshot whole or limited to one Event (191's restore, 198's merge taken back), a Server Event Merge's combination, many small edits made offline or while writes were refused. A Server Event Replacement leaves nothing that differs, so it never counts.
- **Paused:** past the thresholds nothing is sent, and the Calendar has a Sync Error of its own kind with the numbers to change and delete as values; the Sync Status and the overall status follow as for any Sync Error. A paused Calendar takes in Incoming Syncs and stays editable; every later Outgoing Sync is checked afresh: once under the thresholds it sends and the Sync Error goes, while past them the numbers are brought up to date. An Event's reason for being unsent says its Calendar is paused.
- **"Send":** a call on the core interface for one Calendar that runs one Outgoing Sync without the check, sending what differs at that moment. Nothing stores the approval; the next Outgoing Sync is checked again. Refused when the Calendar isn't paused.
- **Types:** the Sync Error kind, the unsent reason and the call are interface types; regenerate the TypeScript types. The window's part stays what the type change forces; 203 adds "Send" and the words.

Spec: US-98; the Mass-Change Guard under "Sync engine"; "Signals"; "Core application interface". ADRs 0018 (a Snapshot's origin), 0019 ("An Outgoing Sync", "Consequences"). Layout: "By phase", P4, 201.

**Blocked by:** 192

**Status:** ready-for-agent

**Effort:** M

- [ ] Every Outgoing Sync is checked before its first request; Incoming Syncs aren't
- [ ] Changes and deletes on the server count; creations don't; an Event last changed by an import the server's side hasn't reached doesn't
- [ ] Past the thresholds nothing is sent and the Calendar is paused as a Sync Error with the numbers; an unsent Event's reason says paused
- [ ] A paused Calendar takes in Incoming Syncs and is sent again by itself once under the thresholds
- [ ] "Send" runs one Outgoing Sync unchecked, stores nothing, and is refused on a Calendar that isn't paused
- [ ] Tests through the core interface against Radicale, test-first: the exact boundaries of both thresholds; changes and deletes counted, creations not; an import past the thresholds sent without a pause; its Undo paused; a redo of that Undo paused; an import the server's side has reached, then edited, counts; "Send" sends and the next Outgoing Sync is checked again; a pause ended because the user undid enough; an Incoming Sync taken in while paused; a large Incoming Sync not checked; a Server Event Replacement isn't counted (old 152's test, moved here)
