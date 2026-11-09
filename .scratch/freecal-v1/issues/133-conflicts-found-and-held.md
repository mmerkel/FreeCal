# 133: The core finds and holds Conflicts on single Events

**What to build:** The first part of ticket 20 (read it for the history). When an Incoming Sync brings a server change to a single Event that has an Unsent Change, FreeCal holds a Conflict instead of taking the server's version. This part finds, stores and reports the Conflict in the core. Listing and resolving Conflicts is ticket 134, what each side changed is 135, and the window is 136 and 137. Recurring Events are ticket 21's.

- **Where:** Conflicts are found in the Provider-neutral core (ADR 0014). The Sync Engine gives the Server Version of each Calendar and UID the server changed. A neutral step next to the Change Journal propagates it to the Event Rows (`event::propagate_to_event_rows`, `core/src/event/mod.rs`), or raises a Conflict when an Unsent Change is about the Event. For a single Event, "per Occurrence" means the Event itself.
- **The three kinds**, each with the Event's Unsent Change:
  - edit/edit: the user edited the Event and the server changed it;
  - local edit vs remote delete: the Server Version is empty;
  - local delete vs remote edit: the User's Version is empty (no Event Rows).
- **Holding:** a Conflict keeps the User's Version in the Event Rows and its Unsent Changes, and keeps a copy of the Server Version, as Event fields, in the new tables `conflict` and `conflict_server_event`. They are written with the Change Journal's Write Permission. A later Incoming Sync that changes the same UID replaces the copy. Deleting a Calendar or Account removes its Conflicts. A local edit to an Event in Conflict changes the User's Version, and the Conflict stays.
- **Nothing to choose:** if the Server Version equals the User's Version (as `as_propagated` compares them), no Conflict is raised, and the Unsent Changes are removed.
- **Recurring Events:** a UID whose Events have a recurrence rule or Exceptions, on either side, propagates as today until ticket 21.
- **The Sender holds back:** the Incoming Sync stores the Server Version's ETag in the Mirror. Without a rule, the next Outgoing Sync would then `PUT` the User's Version with `If-Match` on that ETag and overwrite the server version without asking. So the CalDAV Sender sends nothing for a UID with an open Conflict. Its reason for "Not yet synced" (ticket 124's comment) is read off the Conflict, not stored.
- **Overall status:** feed the open Conflicts into `SyncStatusTracker::overall` (`core/src/sync_status.rs`), so `OverallState::Conflict` is raised, and tick ticket 15's overall-status box.
- **Signal:** add "Problems changed" (spec: "Signals") and send it when a sync raises a Conflict or replaces one's copy. Ticket 134 sends it on resolving. Whether Sync Errors send it too is ticket 23's.

Spec: US-33, US-71; "Sync engine"; "Signals"; "Untrusted content". ADRs 0009, 0014, 0015.

**Blocked by:** 91, 92, 100, 127, 128

**Status:** ready-for-agent

**Effort:** M

- [ ] Edit/edit, local edit vs remote delete and local delete vs remote edit each hold a Conflict that keeps the User's Version and a copy of the Server Version in `conflict` and `conflict_server_event`
- [ ] A later Incoming Sync changing the same UID replaces the copy; equal versions raise no Conflict
- [ ] The Sender sends nothing for a UID in Conflict
- [ ] The overall status is Conflict while one is open (ticket 15's box ticked)
- [ ] The core sends "Problems changed" when a sync raises a Conflict or replaces its copy
- [ ] Written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20. It restates 20's comments that still hold: per ADR 0010, a server change to an Event with an Unsent Change is a Conflict, never a silent replacement, including an undone pulled change waiting to be sent; and ticket 15's note on the overall status. It also closes the gap ticket 127 left open: until now, Propagate overwrites unsent edits of single Events.
