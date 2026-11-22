# 133: The core reports open Conflicts

**What to build:** Since ticket 152 an Incoming Sync ends each Conflict at once with a Server Event Replacement. Nothing shows that yet. This ticket adds the `resolution` table, which tickets 134 and 168 write, and makes the core report the open Conflicts: a list, their number in the Sync Status, the Signal "Conflicts changed" and the overall status (ADR 0019, "Conflicts end at once"). Resolving is ticket 134, what each side changed is 135, the badge is 136, and the window is 137 and 162.

- **The `resolution` table**, written once (a trigger refuses updates): the Server Event Replacement it resolves (unique, `ON DELETE CASCADE`), how it ended (kept the server's version, restored the user's, changed again, Automatic Resolution), the Snapshot that ended it and the time. A CHECK makes the Snapshot empty exactly when the server's version was kept (ADR 0019). An open Conflict is a Server Event Replacement without a row. No other table or mark holds a Conflict.
- **The list,** through the core interface, for all Calendars or for one (spec: "Conflicts: list the open ones…"), newest first. One listed Conflict is one Server Event Replacement without a Resolution, and has:
  - the Server Event Replacement's ID, which ticket 134's calls take;
  - the Calendar, the UID and when it was replaced;
  - the User's Version (the Event Versions the replacement ended), the Server Version (those it began) and the Last Agreed Version (named by its origin), each as the managed fields of its Events, an Event named by its Combined Event Key (ticket 142). Any of the three may be empty: the Event was deleted on that side, or wasn't there when both sides created the UID;
  - its kind, derived by one function and never stored: an edit on both sides; the user's edit against the server's delete (the Server Version is empty); the user's delete against the server's edit (the User's Version is empty).
- **The number of open Conflicts** of a Calendar is the field "Sync status changed" has for it (ticket 161 adds the field with 0 in it; if 161 isn't built yet, this ticket adds it as 161 describes it). The Account's report follows.
- **The overall status** is Conflict while any Calendar has an open one: feed the numbers into `SyncStatusTracker::overall` (`core/src/sync_status.rs`), and tick ticket 15's overall-status box. A Conflict stops nothing from syncing; the status says the user has something to look at.
- **"Conflicts changed"** (spec: "Signals") is sent for a Calendar after every Database Transaction that writes or drops one of its Server Event Replacements or Resolutions. Such a Calendar is also among those sent "Occurrences changed", for the badge (ticket 136), and "Sync status changed" is sent when its number differs.
- **Types:** the list's types are interface types; regenerate the TypeScript types (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`) and give the fake core its answer. Every field of a version is a stored field and untrusted.
- **Tests,** through the core interface against Radicale: each of the three kinds is listed with its three versions; a UID both sides created differently; the list for one Calendar and for all; a Resolution written by hand takes it off the list; a second change on the server to the Event afterwards leaves the listed Server Version as it was replaced; the number, the overall status and the Signals for each of these.

Spec: US-33, US-71, US-77; "Core application interface"; "Signals"; the overall status under "Sync engine"; "Untrusted content". ADR 0019 ("Conflicts end at once").

**Blocked by:** 152

**Status:** ready-for-agent

**Effort:** S

- [ ] The `resolution` table, written once, cascading with its Server Event Replacement, its CHECK refusing a "kept" with a Snapshot and any other without one
- [ ] The core lists the open Conflicts of all Calendars or of one, each with its User's Version, Server Version, Last Agreed Version and kind, read from the Server Event Replacements without a Resolution
- [ ] "Sync status changed" carries a Calendar's number of open Conflicts, and the overall status is Conflict while any Calendar has one (ticket 15's box ticked)
- [ ] "Conflicts changed" and "Occurrences changed" are sent when a Conflict arises, is resolved or is dropped
- [ ] The tests listed above, written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20. It restates 20's comments that still hold: per ADR 0010, a server change to an Event with an Unsent Change is never a silent replacement, including an undone pulled change waiting to be sent; and ticket 15's note on the overall status. It also closes the gap ticket 127 left open: until now, Propagate overwrites unsent edits of single Events.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137 (ADR 0014's amendment of 2026-11-09). The User's Version stays in the Event Rows. Moving it into the Conflict was considered and rejected, as the user's own edit would vanish from the grid. The Last Agreed Version comes from the Mirror, not the Change Journal. One table keeps both copies. "Conflicts changed" replaces the spec's "Problems changed"; Sync Errors stay on "Sync status changed".
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above describe the model of ADRs 0014 and 0015 and are history. Finding and holding a Conflict is ticket 152's. The list moved here from ticket 134, which keeps the ending. The `conflict` and `conflict_event` tables, the rule's four cases, the copy of the Last Agreed Version from the Mirror and the Sender's condition are gone. Re-sized from M to S. Chosen here, for review: a listed Conflict no longer says when it was raised, as nothing stores it and it can't be read from the Snapshots (a Conflict can arise from the user's change while another is open); and "Conflicts changed" is sent by the one rule above, without comparing the list before and after.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADR 0019 rewritten): a Conflict is a Server Event Replacement without a Resolution, so the list is read from those, says when it was replaced, and needs no opaque value against stale choices; a Conflict no longer makes changes unsent, so the "reason: Conflict" goes (the 2026-11-09 comment on ADR 0010 is history too: the server's version now replaces the user's, which is kept and shown). The `resolution` table is added here, as the list needs it.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): the `resolution` table gains the Automatic Resolution as a way a Conflict ends (written by ticket 168) and a CHECK: no Snapshot exactly for "kept".
