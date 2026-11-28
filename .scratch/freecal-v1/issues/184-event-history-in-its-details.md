# 184: An Event's History in its details

**What to build:** The view of one Event's versions (US-148), read from the Calendar History, without an undo of its own (ADR 0018, "An Event's History"). It brings back the History button that 182 took out of the Event's details. Old ticket 160 (`git show design/snapshot-line:.scratch/freecal-v1/issues/160-event-history-in-its-details.md`) without three lines that need later tickets: "changed in fields FreeCal doesn't show" is 186's, "since before <date>" 185's, "replaced by the server's version" 190's.

- **The core** (`history/event_history.rs`; `Core::event_history` takes the Combined Event Key) reads the Event Versions of the key's UID in the order of `begin`, newest first, each with the time and origin of the Snapshot it began with. A Snapshot in which the Event didn't exist (deleted, then brought back by an Undo or a sync) shows as a line "deleted".
- **For an Occurrence** of a Recurring Event the history is the Recurring Event's; for an Exception its own, followed by the Recurring Event's from before the Exception existed.
- **Each line** says when, what made it (the Snapshot's origin in words; the Calendar History window, 207, reuses them) and which managed fields differ from the version before, with the old and the new value. An edit back by hand shows a line, and so does an Undo, whose copy is a new version (ADR 0018). A version an Incoming Sync Snapshot began shows "changes from the server".
- **The window:** the Event's details (`src/lib/EventDetails.svelte`) get the History button back, for read-only Events too. No line has an Undo or a "restore" (ADR 0020, "Consequences").
- **Untrusted content:** old titles, descriptions and Locations are stored fields, shown as text only.

Spec: US-148; "Calendar History"; "Untrusted content". ADRs 0018 ("An Event's History"), 0020 ("Consequences"). Layout: "By phase", P2, 184.

**Blocked by:** 182

**Status:** ready-for-agent

**Effort:** S

**Switched-off tests:** turns on and rewrites the tests 182 switched off with "until ticket 184": the core's tests of `event_history` and `src/App.events.test.ts`'s "has its History".

- [ ] The core returns an Event's versions with time and origin, for a single Event, a Recurring Event and an Exception
- [ ] A delete followed by an Undo shows as "deleted" and then the version again; an edit back by hand and an Undo each show a line
- [ ] A version from an Incoming Sync reads "changes from the server"
- [ ] The details show the History for writable and read-only Events, with no way to undo from it
- [ ] Core tests through the core interface; a component test on the fake core; the tests above on again
