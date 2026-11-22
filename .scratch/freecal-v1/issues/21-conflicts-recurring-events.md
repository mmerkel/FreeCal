# 21: Conflicts on Recurring Events

**What to build:** The comparison of the two sides (ticket 152) decides per Occurrence for Recurring Events too. Changes the two sides made to different Occurrences of one Recurring Event are both kept, without asking. A change to the whole Recurring Event that meets a change to one of its Occurrences on the other side is one Conflict on the whole Recurring Event, which the user ends as one decision (US-76). Ticket 152 compares at the grain Propagate had; read what it left before starting.

- **Inside ticket 152's function,** for every Provider. No second comparison, and nothing per Provider: on Google each Exception is a Server Object of its own, and the comparison still reads Event Versions.
- **What counts as a change to which Occurrence,** on either side, against the versions with the server's mark:
  - a change to the Recurring Event itself (any managed field, its recurrence rule and excluded dates included), its creation or its deletion counts as a change to every Occurrence;
  - an Exception changed, created or removed counts as a change to that Occurrence alone.
- **The outcome per Occurrence** is ticket 152's: only the server changed it, it follows the server; only the user, it stays to be sent; both to the same result, they agree; both differently, a Conflict.
- **Without a Conflict,** the result holds each Exception from the side that changed it and the Recurring Event from the side that changed it, in the Incoming Sync Snapshot as for any Event. The Outgoing Sync then sends the merged Recurring Event (on CalDAV one resource for the UID).
- **With an Occurrence in Conflict,** the whole UID is in Conflict: one listed Conflict (ticket 133), with the Recurring Event and its Exceptions in each of the three versions. Its Server Event Replacement puts the server's version of the whole UID in place, the server's changes to other Occurrences included, and nothing of the user's changes to the UID is kept beside it; restoring (ticket 134) brings the user's whole UID back, and the server's changes to it are then overwritten. Trade-off, named: one decision as US-76 asks, at the price of a change to an Occurrence that wasn't itself in Conflict.
- **A Server Event Merge** (ticket 168) combines within one Event Version only: an Exception's title changed on one side against an alarm added to the same Exception on the other is combined, but a change to the Recurring Event against any change to one Occurrence stays a Conflict on the whole Recurring Event (ADR 0019, decided 2026-11-22).
- **Where the two sides differ** (ticket 135) for a Recurring Event: the fields of the Recurring Event as for any Event, and each Occurrence whose Exception differs, or exists on one side only, by its date.
- **The badge** (ticket 136) is on every chip of the Recurring Event; the window (ticket 137) needs no change beyond showing the Occurrences ticket 135 names.
- **Tests,** as unit tests of the comparison and through the core interface against Radicale: the user's edit to one Exception and the server's to another are both kept, and the server holds both after the Outgoing Sync; a change to the Recurring Event on one side alone is followed or sent; a change to the Recurring Event against a change to one Occurrence is one Conflict, ended both ways; both sides changing the same Exception differently; an Exception removed on one side and edited on the other; an excluded date on one side against an Exception on the other; the Recurring Event renamed against an alarm added on the server to one Exception, a Conflict; an Exception's title changed against an alarm added on the server to the same Exception, a Server Event Merge.

Spec: US-76; "Sync engine". ADR 0019 ("Whether the server changed", "Conflicts").

**Blocked by:** 152, 134, 135, 168

**Status:** ready-for-agent

**Effort:** M

- [ ] Changes to different Occurrences of one Recurring Event are both kept without a Conflict, and the Outgoing Sync sends the merged Recurring Event
- [ ] A change to the Recurring Event itself, on either side, counts as a change to every Occurrence
- [ ] A change to the whole Recurring Event against a change to one Occurrence on the other side is one Conflict on the whole Recurring Event, listed with all three versions
- [ ] Its Server Event Replacement takes the server's version of the whole UID; restoring brings back the user's whole UID
- [ ] What each side changed names the Occurrences that side changed
- [ ] The tests listed above, written test-first

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-25 (grilling, ADR 0011): blocked by 91, as ticket 20 is. A Recurring Event's Event Rows are one Server Object on CalDAV, but several on Google; a Conflict there is on the Server Objects whose Combined Event Keys are unsent.
- 2026-11-02: Ticket 91 was split; `unsent_change` is now ticket 92, so this ticket is blocked by both.
- 2026-11-06 (grilling, ADR 0014): detection per Occurrence, with changes to different Occurrences merged without asking (in line with the spec: both can be kept). This replaces the comment of 2026-10-25 on Google Server Objects whose Combined Event Keys are unsent: Conflicts are found in the Provider-neutral core for every Provider. Effort re-sized from M to L for the comparison of Server Versions and the merge.
- 2026-11-09: Ticket 20 was split into 133 to 137. This ticket now waits on the core parts: 133 (finding and holding), 134 (listing and resolving) and 135 (what each side changed). It extends each of them to Recurring Events. The window parts (136, 137) show a Conflict per Event, so they need no change for Recurring Events.
- 2026-11-09 (grilling of 133 to 137, ADR 0014's amendment of 2026-11-09): the Last Agreed Version is kept per UID in `conflict_event`, the Recurring Event with its Exceptions, beside the Server Version; extend 133's rule to compare per Occurrence against it. The rule also runs on local changes (138), so this ticket waits on 138 too. Ticket 20's "common version from the Change Journal" here means the Last Agreed Version.
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above describe the model of ADRs 0009, 0014 and 0015 and are history. The comparison is against the Reference Snapshot, inside ticket 152's function; the previous Server Version from the Mirror, the Unsent Changes and `conflict_event` are gone. Re-sized from L to M, as the comparison, the holding and the merge Snapshot exist by then. Chosen here, for review: an excluded date is a change to the Recurring Event (so to every Occurrence), not to one Occurrence; and the choice takes the chosen side's whole UID.
- 2026-11-21: A Join, with one parent, takes the server's changes in where a merge did; ending a Conflict is still a merge (grilling, ADR 0019).
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): a Conflict on a Recurring Event ends at once with the server's version of the whole UID; restoring brings the user's back. Merges and Joins are gone.
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a Conflict on a Recurring Event can end with an Automatic Resolution (ticket 168), so this ticket waits for it.
- 2026-11-22: Reworded from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADR 0019): changes to different field groups are combined in a Server Event Merge, but never across a Recurring Event and its Exceptions: a change to the Recurring Event against a change to one Occurrence is a Conflict, the Recurring Event renamed against an alarm on one Exception too.
- 2026-11-22: "What each side changed" became "where the two sides differ" (point 4 of `.scratch/freecal-v1/critiques-2.md`, ADR 0019): no Last Agreed Version is kept after the Incoming Sync.
