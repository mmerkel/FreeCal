# 21: Conflicts on Recurring Events

**What to build:** The comparison of the two sides (ticket 152) decides per Occurrence for Recurring Events too. Changes the two sides made to different Occurrences of one Recurring Event are both kept, without asking. A change to the whole Recurring Event that meets a change to one of its Occurrences on the other side is one Conflict on the whole Recurring Event, which the user ends as one decision (US-76). Ticket 152 compares at the grain Propagate had; read what it left before starting.

- **Inside ticket 152's function,** for every Provider. No second comparison, and nothing per Provider: on Google each Exception is a Server Object of its own, and the comparison still reads Event Versions.
- **What counts as a change to which Occurrence,** on either side, against the Reference Snapshot:
  - a change to the Recurring Event itself (any managed field, its recurrence rule and excluded dates included), its creation or its deletion counts as a change to every Occurrence;
  - an Exception changed, created or removed counts as a change to that Occurrence alone.
- **The outcome per Occurrence** is ticket 152's: only the server changed it, it follows the server; only the user, it stays to be sent; both to the same result, they agree; both differently, a Conflict.
- **Without a Conflict,** the result holds each Exception from the side that changed it and the Recurring Event from the side that changed it, in the follow or the merge Snapshot as for any Event. The Outgoing Sync then sends the merged Recurring Event (on CalDAV one resource for the UID).
- **With an Occurrence in Conflict,** the whole UID is in Conflict: one listed Conflict (ticket 133), with the Recurring Event and its Exceptions in each of the three versions; nothing of the UID is merged meanwhile. The choice (ticket 134) takes the chosen side's version of the whole UID, the changes it made to other Occurrences included, and the other side's changes to the UID are dropped. Trade-off, named: one decision as US-76 asks, at the price of a change to an Occurrence that wasn't itself in Conflict.
- **What each side changed** (ticket 135) for a Recurring Event: the fields of the Recurring Event as for any Event, and each Occurrence whose Exception that side changed, created or removed, by its date.
- **The badge** (ticket 136) is on every chip of the Recurring Event; the window (ticket 137) needs no change beyond showing the Occurrences ticket 135 names.
- **Tests,** as unit tests of the comparison and through the core interface against Radicale: the user's edit to one Exception and the server's to another are both kept, and the server holds both after the Outgoing Sync; a change to the Recurring Event on one side alone is followed or sent; a change to the Recurring Event against a change to one Occurrence is one Conflict, ended both ways; both sides changing the same Exception differently; an Exception removed on one side and edited on the other; an excluded date on one side against an Exception on the other.

Spec: US-76; "Sync engine". ADR 0019 ("Whether the server changed", "Conflicts").

**Blocked by:** 152, 134, 135

**Status:** ready-for-agent

**Effort:** M

- [ ] Changes to different Occurrences of one Recurring Event are both kept without a Conflict, and the Outgoing Sync sends the merged Recurring Event
- [ ] A change to the Recurring Event itself, on either side, counts as a change to every Occurrence
- [ ] A change to the whole Recurring Event against a change to one Occurrence on the other side is one Conflict on the whole Recurring Event, listed with all three versions
- [ ] Ending it takes the chosen side's version of the whole UID, and can be undone as any merge
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
