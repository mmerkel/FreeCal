# 138: The two sides are compared after every move of the User's Pointer

**What to build:** The user may edit, move, delete, undo and redo in a Calendar with a Conflict, import into it and go back in its Calendar History: each moves the User's Pointer, and nothing is sent. Since ticket 152 such a Calendar is compared again only by its next Incoming Sync. Now the comparison's second step also runs after every move of the User's Pointer while the Server's Pointer is off the Reference Snapshot (ADR 0019), so a Conflict the user's own change, undo or redo did away with ends at once.

- **Where:** in the one place that moves a Calendar's User's Pointer (ticket 147), in the same Database Transaction as the move, so that no caller can leave it out. It runs ticket 152's function; no second comparison is written.
- **A Conflict is left:** nothing more happens. The change changed the User's Version, and the kind of the Conflict follows (a delete makes it the user's delete against the server's edit).
- **No Conflict is left:** as in an Incoming Sync's second step. A Join takes the server's changes in, whether the user undid their part, typed the server's values, or deleted what the server deleted. Then the Reference Snapshot moves to the Server's Pointer, the Calendar is Tracking, and a sync is asked for.
- **No refusals:** an Event in Conflict is no Read-only Event, and neither ticket 131's module nor the window's copy of it (`whyReadOnly`, `src/lib/windowModel.svelte.ts`) learns about Conflicts.
- **Signals** are ticket 133's rule: "Conflicts changed", "Occurrences changed", and "Sync status changed" when the number differs.
- **The Store Check's** query from ticket 152 (the Reference Snapshot is another Snapshot than the Server's Pointer's exactly while the comparison finds a Conflict) now holds after every Database Transaction; take out whatever let a Calendar wait for its next Incoming Sync.
- **Tests,** through the core interface against Radicale: an edit, a delete, an import, an undo and a redo of an Event in Conflict each keep the Conflict and change its User's Version; an Undo of the user's part ends it with a Join, and a redo of that Undo brings the user's change back over the server's only with "Redo over the server's change", as a Conflict no longer stands; typing the server's values ends it with a Join; a change to another Event of the Calendar keeps it; one of two Conflicts ended this way leaves the other.

Spec: "Sync engine" (Tracking); "Calendar History" (undo while a Calendar has a Conflict); "Signals"; the open editor (View state). ADRs 0019 ("An Incoming Sync", "Conflicts"), 0020.

**Blocked by:** 133

**Status:** wontfix

**Effort:** S

- [ ] The comparison's second step runs in the Database Transaction of every move of the User's Pointer while the Server's Pointer is off the Reference Snapshot
- [ ] A change, undo or redo that leaves a Conflict changes its User's Version and nothing else
- [ ] One that leaves no Conflict makes the Calendar Tracking again: a Join is made
- [ ] The Signals are sent
- [ ] The tests listed above, written test-first; the Store Check passes after each

## Comments
- 2026-11-09 (grilling): split from 133 when the grilling of tickets 133 to 137 decided that an Event in Conflict stays editable and that the rule runs on every change to either side.
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; kept as a ticket of its own, S. Ticket 133's rule and the Change Journal's write path are gone: this is ticket 152's comparison run at one more point. The result that told the window "this change ended a Conflict", and its toast "Conflict resolved", are dropped: the bar goes and the list says so (tickets 162, 137).
- 2026-11-21: An undo that ends a Conflict leaves the Calendar Held during Undo until the redo path ends; typing the server's values ends it with a Join (grilling, ADR 0019).
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). An Undo that ends a Conflict ends it with a Join, like any change.
- 2026-11-22: `wontfix` after the grilling on critique 2 (ADR 0019 rewritten): Conflicts end at once with a Server Event Replacement, so no Calendar waits for a comparison, and the User's Pointer, the Reference Snapshot and Joins are gone. That a change of the user's to an Event in Conflict resolves it is now ticket 134's "changed again".
