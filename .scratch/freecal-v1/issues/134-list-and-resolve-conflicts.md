# 134: The core lists and resolves Conflicts

**What to build:** The second part of ticket 20 (read it for the history). Through the core interface (spec: "Problems: list and resolve Conflicts"), the user can list every open Conflict that ticket 133 holds and resolve one by choosing a side. A wrong choice can be undone.

- **List:** each open Conflict with its Calendar, the Event, its kind (edit/edit, local edit vs remote delete, local delete vs remote edit), the User's Version and the Server Version as Event fields. A deleted side is absent. Ticket 135 adds what each side changed.
- **One choice for every kind:** keep the User's Version or keep the Server Version. The frontend gives each choice its words per kind (ticket 137): "restore with my changes" and "delete anyway" keep the User's Version; "accept the delete" and "keep the server version" keep the Server Version.
  - **Keep the User's Version:** the Conflict goes, and the Unsent Changes stay for the Sender. After a remote delete, the Server Object is gone from the Mirror, so the Sender creates it again. After a remote edit, it deletes or updates with `If-Match` on the Server Version's ETag.
  - **Keep the Server Version:** the copy is written to the Event Rows, and the Event's Unsent Changes and the Conflict go.
- **Resolving is a Journal Transaction** (ADR 0009) that keeps the losing Event Version, so that undoing it puts the losing side back. Keeping the Server Version changes the Event Rows, so a normal undo restores the User's Version as an Unsent Change. Keeping the User's Version leaves the Event Rows as they are, so the Journal Transaction must still record the Server Version as the one that lost, for the undo to write it. Decide how and record it; amend ADR 0009 if it changes what a Journal Event Modification holds. Undoing a resolution doesn't reopen the Conflict.
- **A stale choice:** the user chooses against the copy they saw. If a later Incoming Sync replaced it meanwhile, resolving fails with an error, and the frontend reads the Conflict again.
- **Signals:** "Problems changed" on resolving, and "Occurrences changed" for the Calendar when the Event Rows change.

Spec: US-77; "Core application interface"; "Signals"; "Change Journal"; "Untrusted content". ADRs 0009, 0014.

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists the open Conflicts with both versions
- [ ] Each of the three kinds resolves both ways, and the choice reaches the server at the next sync
- [ ] Resolving is a Journal Transaction keeping the losing Event Version; undoing it puts the losing side back
- [ ] Resolving against a replaced copy fails without changing anything
- [ ] The core sends "Problems changed" on resolving, and "Occurrences changed" when Event Rows change
- [ ] Written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20. It restates 20's comments that still hold: per ADR 0009, a Conflict resolution is a change recorded in the Change Journal.
