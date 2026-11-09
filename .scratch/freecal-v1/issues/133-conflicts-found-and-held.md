# 133: The core finds and holds Conflicts on single Events

**What to build:** The first part of ticket 20 (read it for the history). When an Incoming Sync brings a server change to a single Event that has an Unsent Change, FreeCal holds a Conflict instead of taking the server's version (ADR 0014 with its amendment of 2026-11-09). This part finds, stores and reports the Conflict in the core when a sync runs. The same rule on local writes is ticket 138. Listing and resolving is 134, what each side changed is 135, and the window is 136 and 137. Recurring Events are ticket 21's.

- **Where:** in the Provider-neutral core. The Sync Engine gives the Server Version of each Calendar and UID the server changed, together with the previous Server Version: the Mirror's data before the sync overwrites it (ADR 0015's amendment of 2026-11-07). A neutral step next to the Change Journal propagates the Server Version to the Event Rows (`event::propagate_to_event_rows`, `core/src/event/mod.rs`), or applies the rule below when an Unsent Change is about the Event. For a single Event, "per Occurrence" means the Event itself.
- **The rule** (one function, which 138 also calls), with the User's Version (the Event Rows), the Server Version and the Last Agreed Version:
  - User's Version equal to the Server Version (as `as_propagated` compares them): no Conflict, and the Unsent Changes are removed;
  - Server Version equal to the Last Agreed Version (the server reverted its change): no Conflict, and the User's Version stays, to be sent;
  - User's Version equal to the Last Agreed Version: no Conflict, the Server Version is propagated, and the Unsent Changes are removed;
  - otherwise, a Conflict.
  Without a readable previous Server Version, there is no Last Agreed Version, and only the first and last cases apply.
- **Holding:** the User's Version stays in the Event Rows with its Unsent Changes. A Conflict is one row of the new table `conflict` (Calendar, UID, when it was raised), with copies of the Server Version and the Last Agreed Version, as Event fields, in the new table `conflict_event`, told apart by a side column. A Rust enum for the side maps to and from the column, in one place. Either copy may hold no Event: a Server Version is empty after a remote delete. Both tables are written with the Change Journal's Write Permission. A later Incoming Sync that changes the same UID applies the rule again and replaces the Server Version's copy. The Last Agreed Version stays. Deleting a Calendar or Account removes its Conflicts.
- **The kind of Conflict is derived, never stored:** no Event Rows means a local delete vs a remote edit, an empty Server Version means a local edit vs a remote delete, and both present means edit/edit. One function decides it, for 134 to return.
- **Nothing in the Change Journal:** raising, replacing or dissolving a Conflict changes no Event Rows, so it adds no Journal Event Modification. Dissolving one that propagates the Server Version is an ordinary change to the Event Rows.
- **The Sender holds back:** the Incoming Sync stores the Server Version's ETag in the Mirror, so a send would overwrite the Server Version without asking. The CalDAV Sender's choice of what to send (`unsent_uids`) leaves out every UID with an open Conflict, as a condition of that query. The reason for "Not yet synced" (ticket 124's comment) is read off the Conflict, not stored.
- **Overall status:** feed the open Conflicts into `SyncStatusTracker::overall` (`core/src/sync_status.rs`), so `OverallState::Conflict` is raised, and tick ticket 15's overall-status box.
- **Signal "Conflicts changed"** (new, spec: "Signals"): a Conflict was raised, replaced or resolved. The Journal Transaction records the Calendars whose Conflicts changed, as it records those whose Events changed, and `Core::signal` (`core/src/lib.rs`) sends both Signals from it. A Calendar whose Conflict changed is also among those sent "Occurrences changed", for the badge (ticket 136).

Spec: US-33, US-71; "Sync engine"; "Signals"; "Untrusted content". ADRs 0009, 0014, 0015.

**Blocked by:** 91, 92, 100, 127, 128

**Status:** ready-for-agent

**Effort:** M

- [ ] Edit/edit, local edit vs remote delete and local delete vs remote edit each hold a Conflict that keeps the User's Version in the Event Rows, and the Server Version and Last Agreed Version in `conflict` and `conflict_event`
- [ ] The rule's other cases raise no Conflict: equal versions, a server back at the Last Agreed Version, a User's Version back at it
- [ ] A later Incoming Sync changing the same UID applies the rule again: it replaces the Server Version's copy or dissolves the Conflict
- [ ] The Sender sends nothing for a UID in Conflict
- [ ] The overall status is Conflict while one is open (ticket 15's box ticked)
- [ ] "Conflicts changed" and "Occurrences changed" are sent from the Journal Transaction when a sync raises, replaces or dissolves a Conflict
- [ ] Written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20. It restates 20's comments that still hold: per ADR 0010, a server change to an Event with an Unsent Change is never a silent replacement, including an undone pulled change waiting to be sent; and ticket 15's note on the overall status. It also closes the gap ticket 127 left open: until now, Propagate overwrites unsent edits of single Events.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137 (ADR 0014's amendment of 2026-11-09). The User's Version stays in the Event Rows. Moving it into the Conflict was considered and rejected, as the user's own edit would vanish from the grid. The Last Agreed Version comes from the Mirror, not the Change Journal. One table keeps both copies. "Conflicts changed" replaces the spec's "Problems changed"; Sync Errors stay on "Sync status changed".
