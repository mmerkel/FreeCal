# 21: Conflicts on Recurring Events

**What to build:** When a local change to a whole Recurring Event collides with a remote change to one of its Occurrences, FreeCal raises one Conflict on the whole Recurring Event, showing both versions, so the user resolves it as one decision.

**Blocked by:** 20, 91, 92

**Status:** ready-for-agent

**Effort:** L

- [ ] A series edit vs a remote Occurrence edit becomes one Conflict on the whole Recurring Event
- [ ] Both versions are shown, with what each side changed against the common version from the Change Journal; resolving applies the chosen one to the whole Recurring Event and can be undone
- [ ] Detection is per Occurrence (ADR 0014): with no Unsent Change an Occurrence is propagated; with an Unsent Change that the server didn't change it keeps the User's Version; changed on both sides it is a Conflict, which then covers the whole Recurring Event
- [ ] A change to the Recurring Event itself, on either side, counts as a change to every Occurrence
- [ ] What the server changed comes from comparing the Server Version with the previous one, read from the Mirror's data before it is overwritten; without a readable previous one, every Occurrence counts as changed by the server
- [ ] An unsent edit to one Exception and a server change to another Exception of the same Recurring Event are both kept without a Conflict, and the Outgoing Sync then sends the merged Recurring Event
- [ ] Written test-first against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-25 (grilling, ADR 0011): blocked by 91, as ticket 20 is. A Recurring Event's Event Rows are one Server Object on CalDAV, but several on Google; a Conflict there is on the Server Objects whose Combined Event Keys are unsent.
- 2026-11-02: Ticket 91 was split; `unsent_change` is now ticket 92, so this ticket is blocked by both.
- 2026-11-06 (grilling, ADR 0014): detection per Occurrence, with changes to different Occurrences merged without asking (in line with the spec: both can be kept). This replaces the comment of 2026-10-25 on Google Server Objects whose Combined Event Keys are unsent: Conflicts are found in the Provider-neutral core for every Provider. Effort re-sized from M to L for the comparison of Server Versions and the merge.
