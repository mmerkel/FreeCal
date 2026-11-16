# 134: The user ends a Calendar's Conflicts by choosing a version for each

**What to build:** Through the core interface, the user ends the Conflicts of one Calendar by naming, for each, the User's Version or the Server Version. It is the comparison's second step run with the user's choices (ADR 0019): one merge Snapshot with both sides as parents, after which the Calendar is Tracking again and what differs is sent. The list of Conflicts is ticket 133's.

- **One call for one Calendar,** with a choice for every Conflict the Calendar has, each with the value ticket 133's list gave for it. Nothing can hold a choice for one Conflict while another stays open: no table holds a Conflict, and the Reference Snapshot is one Snapshot for the whole Calendar. So the choices are made in the window (ticket 137) and handed over together.
- **What the call does,** in one Database Transaction: it runs the comparison of the two sides (ticket 152's function) with the choices. An Occurrence in Conflict takes the chosen side's version; every other Occurrence is decided as in any comparison. The result is one merge Snapshot at the User's Pointer, with the user's and the server's Snapshot as parents and an origin of its own (Conflicts ended by the user). It is made also when every choice names the same side and the result equals that parent's content, so that undo can ask which side to go back to (ADR 0020). The Reference Snapshot then moves to the Server's Pointer.
- **The Unimplemented Fields** follow the comparison's rule whatever the choice: one side changed them, that side's are taken; both did, the server's.
- **Afterwards** the Calendar is Tracking. What the user's side has beyond the server's is sent by the next Outgoing Sync, which the call asks for as an edit does (`Core::after_local_change`): "Your version" after the server's delete creates the Event again, "Your version" after the user's delete deletes it on the server. The Mass-Change Guard checks that Outgoing Sync as any other (ticket 22). The Signals are ticket 133's: "Conflicts changed", "Occurrences changed", and "Sync status changed" with no Conflicts.
- **Undo:** the merge Snapshot is undone and redone as any merge (tickets 151 and 158: Ctrl+Z asks which side to go back through). Nothing here is special to Conflicts, and no choice is one that can't be undone.
- **A stale choice:** the call fails without changing anything unless the Conflicts the comparison finds now are exactly those the choices name, with the versions the values stand for. A Calendar that has no Conflict any more fails with an error of its own.
- **Through ticket 131's module:** the call asks `ReadOnly::may_edit` (`core/src/read_only.rs`) as any edit does. Choices that all take the Server Version are allowed in a Read-only Calendar for the module's existing reason: they load what the server has, as an Incoming Sync does.
- **No "Your version" while the server doesn't allow changes:** while the Calendar is Server read-only, a call with any choice for the User's Version fails with an error, and the Conflicts stay until the user takes the Server Version for all of them or the permission returns. The list (ticket 133) says per Calendar whether "Your version" can be chosen. A Calendar only the user marked read-only still allows it, as changes made before the mark are still sent.
- **Tests,** through the core interface against Radicale: each of the three kinds ended both ways, with the server's state after the next sync; two Conflicts in one Calendar ended with different choices, and an Event only one side changed merged in the same Snapshot; every choice for one side still makes a merge Snapshot; undo of the merge through either side; each stale case; the Read-only rules; the Store Check after each.

Spec: US-73 to US-75, US-77; "Core application interface"; "Sync engine"; "Calendar History"; "Writability"; "Untrusted content". ADRs 0019 ("An Incoming Sync", "Conflicts"), 0020.

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** M

- [ ] One call ends all Conflicts of a Calendar with a choice for each, as one merge Snapshot with both sides as parents, in one Database Transaction
- [ ] Each of the three kinds ends both ways, and the result reaches the server at the next sync
- [ ] The merge Snapshot is undone and redone as any merge
- [ ] A call on changed versions, on other Conflicts than the Calendar has now, or on a Calendar without a Conflict fails without changing anything
- [ ] Choices for the Server Version alone work in a Read-only Calendar; any choice for the User's Version fails while the Calendar is Server read-only
- [ ] The Signals are sent and a sync of the Calendar is asked for
- [ ] The tests listed above, written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137. Ticket 20's "keeps the losing Event Version, so a wrong choice can be undone" now holds only for "Server version": the Change Journal records changes to Event Rows, and nothing else. A wrong "Your version", once sent, can only be put right by hand. Ticket 19 needs no box for it, as undoing "Server version" is an ordinary undo.
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above describe the model of ADRs 0009 and 0014 and are history. The list moved to ticket 133. Both choices are now the same kind of change, a merge Snapshot, and both can be undone; the Journal Event Modification for "Server version" and the "Your version can't be undone" rule are gone.
- 2026-11-16: **Open, hence `needs-info`:** the call ends all of a Calendar's Conflicts at once, where the spec's core interface says "resolve one". Reason: with no table for Conflicts, "Your version" for one Conflict can't be kept while another in the Calendar is open (the Reference Snapshot can't move for one Event), and ADR 0019 has "the same second step with the user's choices" and "picks one side whole"; the prototype ends them all at once too. Strongest con: nothing is applied until every Conflict of the Calendar has a choice, and choices not yet applied live in the window only (ticket 137). The alternative is a stored choice per Event, against ADR 0019's "no table and no mark on an Event holds a Conflict". If agreed: `ready-for-agent`, and the spec's line becomes "end a Calendar's Conflicts by keeping, for each, the User's Version or the Server Version".
- 2026-11-16: Agreed by the user: a Calendar's Conflicts are ended together, with a choice for each. `ready-for-agent`; the spec's core interface line says so now.
