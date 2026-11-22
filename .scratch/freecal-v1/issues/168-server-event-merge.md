# 168: The Server Event Merge combines changes to different field groups

**What to build:** Since ticket 152 every Event changed differently on both sides gets a Server Event Replacement, even where ticket 167's comparison could combine it. Now an Incoming Sync combines those in a **Server Event Merge** (ADR 0019, "Whether the server changed", "An Incoming Sync", "A Server Event Merge"; ADR 0018, "Zero-length Event Versions"; `GLOSSARY.md`).

- **When:** ticket 167's comparison gives a combination: each field group changed on at most one side, the times only against the Unimplemented Fields, within one Event Version. Not where the base is untrusted (ticket 171).
- **What is written,** in the Incoming Sync's one Database Transaction: the server's version ends with the Incoming Sync Snapshot at once, a zero-length Event Version with the server's mark, as ticket 152 does for a Conflict; then a Snapshot of its own with the origin Server Event Merge, referring to its Incoming Sync Snapshot (ticket 152's column and CHECK), in which the Event's user side takes the combination. No Conflict and no row of `resolution`. The step lives in the one place that writes an Incoming Sync.
- **Sent like any change:** the combination differs from the version with the server's mark, so the next Outgoing Sync sends it, through the Mass-Change Guard like any change (ticket 22).
- **Undo:** an Undo of a Server Event Merge brings back the user's version without the server's change and is sent (ticket 147's Undo, nothing of its own). Ctrl+Z passes over it (ticket 147).
- **The toast,** once per Incoming Sync with Server Event Merges, also where the server changed only Unimplemented Fields: for one, `Standup was combined with a change on the server` with "Undo" (the Undo above) and "Open"; for several, `3 Events were combined with changes on the server` with "Show", which opens the Calendar History at that Incoming Sync (ticket 159). The core reports the Server Event Merges of each Incoming Sync with its Signal. No desktop notification.
- **Shown elsewhere:** the Calendar History names the origin "combined with a change on the server" inside the Incoming Sync's entry (ticket 159); an Event's History shows the zero-length Event Version before a Server Event Merge as "on the server, before combining", and leaves it out before a Server Event Replacement (ticket 160). Whichever of 159, 160 and this ticket comes last adds it.
- **The Store Check** gains: every Server Event Merge's Event has a zero-length Event Version at the Incoming Sync Snapshot it refers to (shared with ticket 152's query for the Server Event Replacement).
- **Tests,** through the core interface against Radicale: the user changes the title and the server adds an alarm (an `ical:` entry): the Event shows the user's title with the server's alarm, no Conflict is listed, the toast is reported, and after the Outgoing Sync the server holds both; the title against the Location, combined; the times against the title, a Conflict; the times against an alarm, combined; the Unimplemented Fields on both sides, a Conflict; an Incoming Sync whose only change is merged still makes its Incoming Sync Snapshot; an Undo of the Server Event Merge restores the user's version and sends it; an Undo of the Incoming Sync Snapshot leaves the merged Event alone; the Store Check after each.

Spec: US-73; "Sync engine"; "Calendar History"; "Untrusted content". ADRs 0017 ("Part of the version"), 0018 ("Zero-length Event Versions", "Snapshots"), 0019 ("Whether the server changed", "An Incoming Sync", "A Server Event Merge").

**Blocked by:** 152, 167

**Status:** ready-for-agent

**Effort:** M

- [ ] An Incoming Sync makes a Server Event Merge for each Event the comparison combines, after the zero-length Event Version of its Incoming Sync Snapshot, in one Database Transaction, with no Conflict and no row of `resolution`
- [ ] The combination is sent at the next Outgoing Sync; an Undo of the Server Event Merge restores the user's version
- [ ] The toast after an Incoming Sync with Server Event Merges, with "Undo" for one and "Show" for several
- [ ] The Store Check's query covers Server Event Merges, with a test that breaks it by hand
- [ ] The tests listed above, written test-first against Radicale

## Comments

- 2026-11-22: Written after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a version combined inside the Incoming Sync Snapshot would be one the server doesn't hold, so the server's mark would have no row. The Server Event Replacement first, then the Automatic Resolution, keeps the mark on the server's version.
- 2026-11-22: Rewritten from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0017 to 0020): the Automatic Resolution, a Server Event Replacement followed by a second Snapshot and a row of `resolution`, is replaced by the Server Event Merge, a Snapshot after the Incoming Sync Snapshot whose zero-length Event Version carries the server's mark. Combining goes by field groups. Renamed from `168-automatic-resolution.md`; Blocked by 133 became 152 and 167, Effort S became M for the toast.
