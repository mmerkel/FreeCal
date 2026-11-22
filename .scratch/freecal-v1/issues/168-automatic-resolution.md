# 168: The Automatic Resolution ends a Conflict at once where it can

**What to build:** Since ticket 152 every Event changed differently on both sides is a Conflict, whatever the fields, and gets a Server Event Replacement. Where the managed fields changed on at most one side, an Incoming Sync now ends that Conflict at once with an **Automatic Resolution** (ADR 0019, "Whether the server changed", "An Incoming Sync"; `GLOSSARY.md`).

- **When:** ticket 167's comparison says the Conflict can be combined: the managed fields changed on at most one side. The combination takes the managed fields from the side that changed them, and the Unimplemented Fields from the side that changed them, the server's if both did (ADR 0017, "Part of the version").
- **What is written,** in the Incoming Sync's one Database Transaction, after the Event's Server Event Replacement: a Snapshot of its own with the origin Automatic Resolution, in which the Event takes the combination, and its row of `resolution` (ticket 133), "Automatic Resolution", naming that Snapshot. Then the server's marks move to the fetched versions, as before. The step lives in the one place that writes an Incoming Sync; no other code makes one.
- **Sent like any change:** the combination differs from the version with the server's mark, so the next Outgoing Sync sends it, through the Mass-Change Guard like any change (ticket 22).
- **What the user sees:** nothing beyond the Calendar History (ticket 159 names the origin): the Conflict is resolved, so it isn't listed and has no badge, bar, toast or notification. Ctrl+Z passes over it (ticket 147); an Undo of it from the Calendar History puts the server's version back, and the Conflict stays resolved.
- **The Store Check** gains: an Automatic Resolution's Snapshot comes right after its Server Event Replacement and changes only that Event.
- **Tests,** through the core interface against Radicale: the user changes the title and the server adds an alarm (an `ical:` entry): the Event shows the user's title with the server's alarm, the Conflict isn't listed, and after the Outgoing Sync the server holds both; the server changes the time and the user only the Unimplemented Fields; both change only the Unimplemented Fields, the server's winning; both change the managed fields, which stays an open Conflict; an Undo of the Automatic Resolution; the Store Check after each.

Spec: US-73; "Sync engine"; "Calendar History"; "Untrusted content". ADRs 0017 ("Part of the version"), 0018 ("Snapshots"), 0019 ("Whether the server changed", "An Incoming Sync", "Conflicts end at once").

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] An Incoming Sync ends each Conflict that can be combined with an Automatic Resolution, a Snapshot after its Server Event Replacement, with its row of `resolution`, in one Database Transaction
- [ ] The combination is sent at the next Outgoing Sync
- [ ] A Conflict ended this way is not listed and shows nothing in the window beyond the Calendar History
- [ ] The Store Check's new query, with a test that breaks it by hand
- [ ] The tests listed above, written test-first against Radicale

## Comments

- 2026-11-22: Written after the prototype on the line (commit `2a4bbe6`; ADRs 0017 to 0020, `.scratch/freecal-v1/critiques.md`, point 2): a version combined inside the Server Intake would be one the server doesn't hold, so the server's mark would have no row. The Server Event Replacement first, then the Automatic Resolution, keeps the mark on the server's version.
