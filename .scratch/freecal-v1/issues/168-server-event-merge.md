# 168: The Server Event Merge combines changes to different field groups

**What to build:** Since ticket 152 every Event changed differently on both sides is replaced by the server's version, even where ticket 167's comparison could combine it. Now an Incoming Sync combines those in a **Server Event Merge**, within its Incoming Sync Snapshot (ADR 0019, "Whether the server changed", "An Incoming Sync", "A Server Event Merge"; ADR 0018, "Zero-length Event Versions"; `GLOSSARY.md`).

- **When:** ticket 167's comparison gives a combination: each field group changed on at most one side, the times only against the Unimplemented Fields, within one Event Version. Not where the base is untrusted (ticket 171).
- **What is written,** in the Incoming Sync Snapshot, in its one Database Transaction: the server's version begins and ends with it, a **zero-length Event Version** carrying only the server's mark, as the combination is no version the server holds; the combination begins with it on the user's side. No Conflict and no row of `conflict`, and no record of its own: a Server Event Merge is recognised by its zero-length Event Versions. The step lives in the one place that writes an Incoming Sync.
- **Zero-length Event Versions in the schema:** an insert trigger allows `end` equal to `begin` only where that Snapshot is an Incoming Sync Snapshot, `end` after `begin` everywhere else, with a comment naming ADR 0018. Every query of a Snapshot's state already reads `begin` at or before it and `end` after it or empty, so a zero-length Event Version is in no Snapshot's state.
- **Sent like any change:** the combination differs from the version with the server's mark, so the next Outgoing Sync sends it, through the Mass-Change Guard like any change (ticket 22).
- **Undo:** an Undo of the Incoming Sync Snapshot limited to the Event (ticket 147) brings back the user's version without the server's change and is sent; a whole Undo of the Incoming Sync Snapshot takes back its Server Event Merges with the rest. Nothing of its own. Ctrl+Z passes over Incoming Sync Snapshots (ticket 147).
- **The toast,** once per Incoming Sync with Server Event Merges, also where the server changed only Unimplemented Fields: for one, `Standup was combined with a change on the server` with "Undo" (the limited Undo above) and "Open"; for several, `3 Events were combined with changes on the server` with "Show", which opens the Calendar History at that Incoming Sync (ticket 159). The core reports the Server Event Merges of each Incoming Sync with its Signal. No desktop notification.
- **Shown elsewhere:** the Calendar History names the Events "combined with a change on the server" inside the Incoming Sync's entry (ticket 159); an Event's History shows the zero-length Event Version as "on the server, before combining", before the combination (ticket 160). Whichever of 159, 160 and this ticket comes last adds it.
- **The Store Check** gains: a zero-length Event Version only beside a version of the same Event beginning with the same Snapshot.
- **Tests,** through the core interface against Radicale: the user changes the title and the server adds an alarm (an `ical:` entry): the Event shows the user's title with the server's alarm, no Conflict is listed, the toast is reported, and after the Outgoing Sync the server holds both; the title against the Location, combined; the times against the title, a Conflict; the times against an alarm, combined; the Unimplemented Fields on both sides, a Conflict; an Incoming Sync whose only change is merged still makes its Incoming Sync Snapshot; an Undo of the Incoming Sync Snapshot limited to the Event restores the user's version and sends it; a whole Undo of it takes back the merge with the rest; the trigger refuses a zero-length Event Version at any other Snapshot; the Store Check after each.

Spec: US-73; "Sync engine"; "Calendar History"; "Untrusted content". ADRs 0017 ("Part of the version"), 0018 ("Zero-length Event Versions", "Snapshots"), 0019 ("Whether the server changed", "An Incoming Sync", "A Server Event Merge"), 0020 ("Limited to one Event").

**Blocked by:** 152, 167

**Status:** ready-for-agent

**Effort:** M

- [ ] An Incoming Sync combines each Event the comparison combines in its Incoming Sync Snapshot, with the server's version as a zero-length Event Version, with no Conflict and no row of `conflict`
- [ ] The insert trigger allows zero-length Event Versions only at an Incoming Sync Snapshot
- [ ] The combination is sent at the next Outgoing Sync; an Undo of the Incoming Sync Snapshot limited to the Event restores the user's version
- [ ] The toast after an Incoming Sync with Server Event Merges, with "Undo" for one and "Show" for several
- [ ] The Store Check's query for zero-length Event Versions, with a test that breaks it by hand
- [ ] The tests listed above, written test-first against Radicale

## Comments

- 2026-11-22: Written after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a version combined inside the Incoming Sync Snapshot would be one the server doesn't hold, so the server's mark would have no row. The Server Event Replacement first, then the Automatic Resolution, keeps the mark on the server's version.
- 2026-11-22: Rewritten from the grilling on point 2 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0017 to 0020): the Automatic Resolution, a Server Event Replacement followed by a second Snapshot and a row of `resolution`, is replaced by the Server Event Merge, a Snapshot after the Incoming Sync Snapshot whose zero-length Event Version carries the server's mark. Combining goes by field groups. Renamed from `168-automatic-resolution.md`; Blocked by 133 became 152 and 167, Effort S became M for the toast.
- 2026-11-26: Rewritten from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): the Server Event Merge is no longer a Snapshot of its own but what the Incoming Sync Snapshot does to an Event combined; its Undo is an Undo of the Incoming Sync Snapshot limited to the Event. Zero-length Event Versions and their trigger moved here from ticket 152, as only a Merge needs them.
