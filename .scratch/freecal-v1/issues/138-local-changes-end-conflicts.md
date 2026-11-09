# 138: A local change can end a Conflict

**What to build:** The sixth part of ticket 20 (read it for the history). The user may edit, move, delete and undo an Event in Conflict, and import over it: such a change changes the User's Version, and the Conflict stays open. Ticket 133's rule decides whether a Conflict exists. It runs on every change to either side (ADR 0014's amendment of 2026-11-09), so it also runs in the Change Journal's write path for each UID in Conflict that a Journal Transaction changes.

- **Back at the Last Agreed Version** (for example, after undoing the user's own edit): no Conflict, the Server Version is propagated to the Event Rows in the same Journal Transaction, and the Unsent Changes are removed.
- **Equal to the Server Version** (the user typed the server's values): no Conflict, and the Unsent Changes are removed.
- **Otherwise** the Conflict stays, and its kind follows (a delete makes it local delete vs remote edit).
- No refusals: an Event in Conflict is no Read-only Event, and neither ticket 131's module nor the window's copy of it (`whyReadOnly`, `src/lib/windowModel.svelte.ts`) learns about Conflicts.
- A Journal Transaction that ends a Conflict says so in its result, so that the undo toast can say "Conflict resolved" (ticket 137). It sends "Conflicts changed" as 133 wires it.

Spec: "Change Journal"; "Signals"; the open editor (View state). ADRs 0009, 0014.

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] An edit, move, delete, undo or import of an Event in Conflict changes the User's Version and keeps the Conflict open
- [ ] A change bringing the User's Version back to the Last Agreed Version ends the Conflict and propagates the Server Version
- [ ] A change making the User's Version equal to the Server Version ends the Conflict and removes the Unsent Changes
- [ ] The Journal Transaction's result says that it ended a Conflict, and "Conflicts changed" is sent
- [ ] Written test-first

## Comments
- 2026-11-09 (grilling): split from 133 when the grilling of tickets 133 to 137 decided that an Event in Conflict stays editable and that the rule runs on every change to either side.
