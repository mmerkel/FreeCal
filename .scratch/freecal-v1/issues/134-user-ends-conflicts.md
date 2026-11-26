# 134: The user resolves a Conflict

**What to build:** Through the core interface, the user resolves an open Conflict (ticket 133) in one of three ways (ADR 0019, "Conflicts end at once"): keeping the server's version, restoring their own, or changing the Event again. Each sets the Conflict's resolved time, once, in the Database Transaction that ends it. How it ended isn't stored; the Calendar History shows it.

- **Keep the server's version:** a call naming the Conflict sets its resolved time with no Snapshot. Nothing else changes and nothing is sent. "Keep all" for a Calendar is one call resolving each of its open Conflicts, in one Database Transaction.
- **Restore my version:** an Undo of the Conflict's Incoming Sync Snapshot limited to its UID (ticket 147), from the Conflict, from the toast of a refused Ctrl+Z (ticket 158) or by "Undo this Event" in the Calendar History (ticket 159); or a whole Undo of that Incoming Sync Snapshot, which restores and resolves all its Conflicts. It goes through the one place that makes a change's Snapshot, so it is sent like any change and counted by the Mass-Change Guard (ticket 22). Refused like any Undo: if the Event changed again since and the two can't be combined (ticket 147), the refusal names the later change.
- **One rule for restoring and changing again:** any Snapshot of the user's own or Undo that changes an Event with an open Conflict (an edit, a move, a delete, an import or Backup restore over it, an Undo reaching it, the restore above) resolves it, in the same Database Transaction. The place that makes a change's Snapshot does it, so no caller can leave it out.
- **Redo** of a restore puts the server's version back and the Conflict stays resolved: the resolved time is set once, and nothing reopens a Conflict. After a restore, changing one's mind is redo, or an Undo from the Calendar History.
- **A stale call:** "keep" on a Conflict that is already resolved fails without changing anything, with an error of its own; the window reads the list again.
- **Through ticket 131's module:** keeping is allowed in a Read-only Calendar, as it changes no Event. A restore is an Undo, and so refused in a Calendar marked read-only (ADR 0020). While the server refuses writes, a restore is made and waits like any change (ticket 166).
- **The Store Check** gains: no open Conflict's Event was changed by the user or an Undo since its Incoming Sync Snapshot.
- **Tests,** through the core interface against Radicale: each of the three kinds kept and restored, with the server's state after the next sync; "Keep all"; an edit, a delete and an import over an Event in Conflict each resolve it; a change to another Event doesn't; a restore from the Calendar History resolves it; a whole Undo of the Incoming Sync Snapshot restores and resolves all its Conflicts, and takes back the Events it merged or followed; an Undo limited to another Event of that Incoming Sync Snapshot leaves the Conflict open; a redo of the restore leaves it resolved; a refused restore; keeping twice; the Read-only rules; the Store Check after each.

Spec: US-73 to US-75, US-77, US-153; "Core application interface"; "Sync engine"; "Calendar History"; "Writability"; "Untrusted content". ADRs 0019 ("Conflicts end at once", "The Conflict tables"), 0020 ("Limited to one Event").

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** M

- [ ] "Keep" for one Conflict and "Keep all" for a Calendar set the resolved time without a Snapshot
- [ ] An Undo of the Incoming Sync Snapshot, limited to the Event or whole, restores the User's Version, from wherever it was asked for, and the restore reaches the server at the next sync
- [ ] Every Snapshot of the user's own or Undo that changes an Event with an open Conflict resolves it in its Database Transaction
- [ ] A redo of a restore leaves the Conflict resolved
- [ ] A "keep" on a resolved Conflict fails without changing anything
- [ ] Keeping works in a Read-only Calendar; a restore is refused there and waits while writes are refused
- [ ] The Store Check's new query, with a test that breaks it by hand
- [ ] The tests listed above, written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137. Ticket 20's "keeps the losing Event Version, so a wrong choice can be undone" now holds only for "Server version": the Change Journal records changes to Event Rows, and nothing else. A wrong "Your version", once sent, can only be put right by hand. Ticket 19 needs no box for it, as undoing "Server version" is an ordinary undo.
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above describe the model of ADRs 0009 and 0014 and are history. The list moved to ticket 133. Both choices are now the same kind of change, a merge Snapshot, and both can be undone; the Journal Event Modification for "Server version" and the "Your version can't be undone" rule are gone.
- 2026-11-16: **Open, hence `needs-info`:** the call ends all of a Calendar's Conflicts at once, where the spec's core interface says "resolve one". Reason: with no table for Conflicts, "Your version" for one Conflict can't be kept while another in the Calendar is open (the Reference Snapshot can't move for one Event), and ADR 0019 has "the same second step with the user's choices" and "picks one side whole"; the prototype ends them all at once too. Strongest con: nothing is applied until every Conflict of the Calendar has a choice, and choices not yet applied live in the window only (ticket 137). The alternative is a stored choice per Event, against ADR 0019's "no table and no mark on an Event holds a Conflict". If agreed: `ready-for-agent`, and the spec's line becomes "end a Calendar's Conflicts by keeping, for each, the User's Version or the Server Version".
- 2026-11-16: Agreed by the user: a Calendar's Conflicts are ended together, with a choice for each. `ready-for-agent`; the spec's core interface line says so now.
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). Undo at a merge moved here from ticket 151; it no longer asks which side.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADR 0019 rewritten): Conflicts end at once with the server's version, so the user no longer ends a Calendar's Conflicts together by a merge but resolves each: keep, restore (an Undo of the Server Event Replacement), or change again, each writing a row of `resolution`. The merge, its undo question, the stale-choice value and "no Your version while Server read-only" are gone. Ticket 138's rule (a change of the user's ends a Conflict) is folded in as "changed again".
- 2026-11-22: Reworded after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a restore whose Event changed since is combined where the managed fields changed on one side only, as any Undo (ticket 147). An Automatic Resolution is ticket 168's.
- 2026-11-26: Rewritten from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): resolving sets the Conflict's resolved time instead of writing a row of `resolution`; restoring is an Undo of the Incoming Sync Snapshot limited to the Event, or a whole one; restoring and changing again are one rule, so the Store Check's query for each way a Conflict ended goes.
