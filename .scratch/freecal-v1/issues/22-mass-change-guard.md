# 22: The Mass-Change Guard checks every Outgoing Sync

**What to build:** Before an Outgoing Sync sends, FreeCal counts what it would change or delete on the server. Past the thresholds it sends nothing and pauses the Calendar as a Sync Error that names the numbers and offers "Send" (ADR 0019, spec: "Sync engine"). Intent is derived since ticket 153, so this is what stands between a fault in deriving what to send and the server. Incoming Syncs aren't checked.

- **Where:** in the Outgoing Sync, after it has read what differs between the Current-Version Marks (ticket 153) and before the first request. A Calendar with a Conflict sends nothing and isn't checked.
- **What is counted:** the Events the Outgoing Sync would change or delete on the server: those with an Event Version on the server's side whose version on the user's side differs or is missing. Events it would create aren't counted. The share is taken of the Events the server's side holds.
- **Not counted:** an Event whose current version began with a Snapshot whose recorded origin (ADR 0018) is an import or the restore of a Backup, and doesn't have the server's mark. The import preview or the restore's confirmation was the approval. Nothing stores the approval; this is the one place where sync reads origins. The restore's origin comes with ticket 11; until then only the import's is read.
- **Everything else counts:** an undo or a redo, of an import too, and of a Server Event Replacement (a restore); many small edits made offline or while writes were refused.
- **The thresholds** are the one rule in `core/src/mass_change_guard.rs` (`exceeded`), which a Dangerous Undo used until ticket 147. Use it as it is; if it went with the Dangerous Undo, bring it back from the history with its test.
- **Paused:** past the thresholds nothing is sent, and the Calendar has a Sync Error of a kind of its own, with the numbers that would be changed and deleted as values; the words are the frontend's ("Paused: 120 Events would be deleted"). The Sync Status and the overall status follow as for any Sync Error of a Calendar.
- **A paused Calendar** keeps taking in Incoming Syncs, and the user keeps editing it. Every later Outgoing Sync is checked afresh: once what differs is under the thresholds it is sent and the Sync Error goes; while it is past them, the numbers are brought up to date. To drop the changes, the user undoes them.
- **"Send":** a call on the core interface for one Calendar that runs one Outgoing Sync without the check, sending what differs at that moment. Nothing stores the approval: a later Outgoing Sync is checked again. The call is refused when the Calendar isn't paused.
- **In the window:** the Sync Error's action is "Send", where the sidebar offers a Sync Error's actions: the click on the Sync Error mark once ticket 161 is built. The reason an Event is unsent says that its Calendar is paused.
- **The old approval goes:** an undo is checked like any other change; the comment of 2026-11-14 below is history.
- **Tests,** through the core interface against Radicale, written test-first: the exact boundaries of both thresholds; changes and deletes counted, creations not; an import past the thresholds is sent without a pause; its undo is paused; a redo of that undo is paused; an import the server's side has reached, then edited, counts; "Send" sends and the next Outgoing Sync is checked again; a pause that ends because the user undid enough; an Incoming Sync taken in while paused; a large Incoming Sync that isn't checked.

Spec: US-98; the Mass-Change Guard under "Sync engine"; "Signals"; "Core application interface". ADRs 0018 (a Snapshot's origin), 0019 ("An Outgoing Sync", "Consequences").

**Blocked by:** 153

**Status:** ready-for-agent

**Effort:** M

- [ ] Every Outgoing Sync is checked before its first request; Incoming Syncs aren't
- [ ] Changes and deletes on the server count; an Event last changed by an import the server's side hasn't reached doesn't
- [ ] Past the thresholds nothing is sent and the Calendar is paused as a Sync Error with the numbers
- [ ] A paused Calendar takes in Incoming Syncs and is sent again by itself once under the thresholds
- [ ] "Send" runs one Outgoing Sync unchecked and stores nothing
- [ ] The window offers "Send" on the paused Calendar's Sync Error
- [ ] The tests listed above, with the exact boundaries

## Comments
- 2026-10-18 (from 82): a sync's changes for one Calendar are `remote::Changes` (changed Events, removed object addresses), applied in `sync::apply` as one Journal Transaction; the guard can count them there before it is recorded. A whole listing (no or refused sync token) counts every held object it doesn't list as removed.
- 2026-11-14 (from 19): an undo's or redo's Unsent Changes skip the guard: a Dangerous Undo's confirmation (`Core::undo` with `approved`) is the guard's approval, so the next sync must not pause the Calendar for them (ADR 0009, spec "Change Journal").
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above are history. The guard checks Outgoing Syncs only, on the difference between the Current-Version Marks. Approve and reject became "Send" and the user's own undo. An undo is no longer let through. Chosen here, for review: Events an Outgoing Sync would create aren't counted (the spec says "delete or change"); the share is of the Events on the server's side; "Send" is refused on a Calendar that isn't paused.
- 2026-11-16: **To settle:** the spec says "more than 10 Events **or** more than 5% of the Calendar"; the code's rule (`exceeded`, from ticket 72) is "more than 10 **and** more than 5%", so that a small Calendar isn't paused at every change. This ticket keeps the code's rule; the spec's sentence should follow, or the other way round.
- 2026-11-16: Settled by the user: "and", so that small Calendars aren't bothered. The spec (US-85 and the Sync engine) now says more than 10 Events and more than 5%, as the code does.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): what isn't counted is read from the current version's `begin` and the server's mark, not ancestry; merges are gone, and a restore is counted as an Undo.
