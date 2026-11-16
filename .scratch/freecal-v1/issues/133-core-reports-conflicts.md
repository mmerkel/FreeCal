# 133: The core reports Conflicts

**What to build:** Since ticket 152 a Calendar holds Conflicts: its Server's Pointer stays off its Reference Snapshot, and nothing of it is sent or merged. Nothing shows that yet. This ticket makes the core report Conflicts: a list read from the two pointers and the Reference Snapshot, their number in the Sync Status, the Signal "Conflicts changed" and the overall status. No table and no mark on an Event holds a Conflict (ADR 0019). Ending Conflicts by the user's choice is ticket 134, what each side changed is 135, the badge is 136, and the window is 137 and 162.

- **The list,** through the core interface, for all Calendars or for one (spec: "Conflicts: list them…"). For each Calendar whose Server's Pointer is off its Reference Snapshot, the comparison of the two sides (ticket 152's function, run without writing) says which Occurrences are in Conflict. A Calendar that is Tracking isn't compared and lists nothing. One listed Conflict is one Calendar and UID with at least one Occurrence in Conflict, and has:
  - the Calendar and the UID;
  - the User's Version (the UID's Event Versions with the user's Current-Version Mark), the Server Version (those with the server's) and the Last Agreed Version (those in the Reference Snapshot), each as the managed fields of its Events, an Event named by its Combined Event Key (ticket 142). Any of the three may be empty: the Event was deleted on that side, or wasn't there when both sides created the UID;
  - its kind, derived by one function and never stored: an edit on both sides; the user's edit against the server's delete (the Server Version is empty); the user's delete against the server's edit (the User's Version is empty);
  - a value that stands for the two versions as listed, built from their Event Versions and opaque to the window. Ticket 134's call hands it back, so that a choice made on versions that have changed since is found.
- **The number of Conflicts** of a Calendar is the number of listed Conflicts. It is the field "Sync status changed" has for it (ticket 161 adds the field with 0 in it; if 161 isn't built yet, this ticket adds it as 161 describes it). The Account's report follows.
- **The overall status** is Conflict while any Calendar has one: feed the numbers into `SyncStatusTracker::overall` (`core/src/sync_status.rs`), and tick ticket 15's overall-status box.
- **Why an Event is unsent:** while a Calendar has a Conflict, the core's reason for each of its unsent changes ("Not yet synced: …" in the Event's details, the Unsent Mark's tooltip) is the Conflict, before any other reason.
- **"Conflicts changed"** (spec: "Signals") is sent for a Calendar after every Database Transaction that moved one of its pointers or its Reference Snapshot while its Server's Pointer was off the Reference Snapshot before it or after it. One rule covers a Conflict that arises, one whose User's Version or Server Version changes, and one that ends. It can be sent when the list reads the same as before; the window reads again. Trade-off, named: no comparison of the list before and after, at the price of a Signal too many now and then. Such a Calendar is also among those sent "Occurrences changed", for the badge (ticket 136), and "Sync status changed" is sent when its number differs.
- **Types:** the list's types are interface types; regenerate the TypeScript types (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`) and give the fake core its answer. Every field of a version is a stored field and untrusted.
- **Tests,** through the core interface against Radicale: each of the three kinds is listed with its three versions; a UID both sides created differently; the list for one Calendar and for all; a second change on the server to an Event in Conflict changes the listed Server Version; a Conflict that ends because the server came to agree leaves the list; the number, the overall status and the Signals for each of these.

Spec: US-33, US-71, US-77; "Core application interface"; "Signals"; the overall status under "Sync engine"; "Untrusted content". ADR 0019 ("Conflicts", "Tracking and the Reference Snapshot").

**Blocked by:** 152

**Status:** ready-for-agent

**Effort:** S

- [ ] The core lists the Conflicts of all Calendars or of one, each with its User's Version, Server Version, Last Agreed Version and kind, read from the pointers and the Reference Snapshot; no table holds them
- [ ] "Sync status changed" carries a Calendar's number of Conflicts, and the overall status is Conflict while any Calendar has one (ticket 15's box ticked)
- [ ] An unsent change in a Calendar with a Conflict gives the Conflict as its reason
- [ ] "Conflicts changed" and "Occurrences changed" are sent when a Conflict arises, changes on either side, or ends
- [ ] The tests listed above, written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20. It restates 20's comments that still hold: per ADR 0010, a server change to an Event with an Unsent Change is never a silent replacement, including an undone pulled change waiting to be sent; and ticket 15's note on the overall status. It also closes the gap ticket 127 left open: until now, Propagate overwrites unsent edits of single Events.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137 (ADR 0014's amendment of 2026-11-09). The User's Version stays in the Event Rows. Moving it into the Conflict was considered and rejected, as the user's own edit would vanish from the grid. The Last Agreed Version comes from the Mirror, not the Change Journal. One table keeps both copies. "Conflicts changed" replaces the spec's "Problems changed"; Sync Errors stay on "Sync status changed".
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above describe the model of ADRs 0014 and 0015 and are history. Finding and holding a Conflict is ticket 152's. The list moved here from ticket 134, which keeps the ending. The `conflict` and `conflict_event` tables, the rule's four cases, the copy of the Last Agreed Version from the Mirror and the Sender's condition are gone. Re-sized from M to S. Chosen here, for review: a listed Conflict no longer says when it was raised, as nothing stores it and it can't be read from the Snapshots (a Conflict can arise from the user's change while another is open); and "Conflicts changed" is sent by the one rule above, without comparing the list before and after.
