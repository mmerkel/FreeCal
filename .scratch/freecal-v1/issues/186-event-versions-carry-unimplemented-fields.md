# 186: Event Versions carry Unimplemented Fields: an Incoming Sync, an import and every edit keep them

**What to build:** 175's tree becomes part of every Event Version (ADR 0017): stored, compared, undone and moved with the Event. Old ticket 149 (`git show design/snapshot-line:.scratch/freecal-v1/issues/149-unimplemented-fields-stored.md`) as written. The Sender still patches the Mirror's text until 187, so nothing changes on a server yet.

- **The column:** `event_version.unimplemented` holds the tree's canonical form. It never crosses the core interface. The `SnapshotWriter` stores the whole Event Version Content, so equality of Event Versions (177) now includes the tree in the Local Store too: a change to the tree alone makes a new Event Version and, from a server, an Incoming Sync Snapshot.
- **A CalDAV Incoming Sync** reads each resource through 175's reader and gives every Event Version it writes its tree. Propagate, until 189, treats the tree as part of the Event it replaces; the First Sync, until 194, does the same.
- **An import** of an `.ics` file or a `.zip` reads through the same reader, so imported Events keep their alarms and other fields under `ical:` names, in a Local Calendar too.
- **An edit in FreeCal** carries the tree from the old Event Version to the new one. For each managed field the edit changes, the entries under that property's name are dropped, as they described the old value: its unknown parameters (a title's `LANGUAGE`) and what the reader couldn't represent exactly (a second `DESCRIPTION`).
- **A new Exception,** made by changing one Occurrence, takes the Recurring Event's tree without the calendar-level entries, so the changed Occurrence keeps its alarm. **"This and following events":** the new Recurring Event takes the original's tree, without the calendar-level entries.
- **A move to another Calendar** carries the neutral entries and those with a prefix the target's Provider holds, and drops the rest: Local holds `ical:`, CalDAV `ical:` and `caldav:`. No move drops anything yet; the rule is still one function that asks the target's Provider for its prefixes, and a unit test gives it a `google:` entry and a Local target.
- **Undo** brings a tree back with its Event Version, with no code of its own.
- **An Event's History** (184): a version that differs from the one before in the Unimplemented Fields alone reads "changed in fields FreeCal doesn't show"; their content isn't shown.
- **Untrusted content:** the tree comes from files and servers; it is stored and compared, never interpreted beyond 175's reader.

Spec: US-148; "Local Store"; "Untrusted content"; "Moves across Accounts"; "Import/export". ADR 0017 ("What is in the tree", "Part of the version"). Layout: "Tables (end state)" (`event_version.unimplemented`); "By phase", P2, 186.

**Blocked by:** 175, 184, 216

**Status:** ready-for-agent

**Effort:** M

- [ ] The tree's column on `event_version`; equality of Event Versions includes it
- [ ] An Incoming Sync and a First Sync fill it; a change on the server to an alarm alone makes an Incoming Sync Snapshot, and an Undo of that Snapshot takes it back
- [ ] An import fills it
- [ ] An edit carries the tree and drops the entries under the names of the fields it changed (unknown parameters, what the reader couldn't represent exactly), and only those
- [ ] A new Exception and the new Recurring Event of "this and following events" have the original's tree, without the calendar-level entries
- [ ] A move between CalDAV Calendars, into a Local Calendar and out of one carries the tree
- [ ] An Event's History reads "changed in fields FreeCal doesn't show" for a change to the tree alone
- [ ] The tree isn't in any type of the core interface; the Store Check passes after every scenario
- [ ] 216's Store Check also reads each Event Version's tree: it deserialises and is already in canonical form; a test breaks each by hand and sees it reported
- [ ] `GLOSSARY.md` follows (Event Version: Unimplemented Fields stored)
