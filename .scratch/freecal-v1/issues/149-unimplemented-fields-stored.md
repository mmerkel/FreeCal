# 149: Event Versions carry Unimplemented Fields: an Incoming Sync, an import and every edit keep them

**What to build:** The tree of ticket 143 becomes part of every Event Version (ADR 0017): stored, compared, undone and moved with the Event. The Sender still patches the Mirror's text; it switches in ticket 150, so nothing changes on a server yet.

- **The column:** `event_version` holds the tree's canonical form in one column. It never crosses the core interface.
- **Part of the version:** two Event Versions are equal when their managed fields and their trees are. A change to the tree alone is a change: it makes a new Event Version and, from a server, a Snapshot.
- **A CalDAV Incoming Sync** reads each resource through ticket 143's reader and gives every Event Version it writes its tree. Propagate treats the tree as part of the Event it replaces. A First Sync does the same.
- **An import** of an `.ics` file or a `.zip` reads through the same reader, so imported Events keep their alarms and other fields, under `ical:` names, in a Local Calendar too.
- **An edit in FreeCal** carries the tree from the old Event Version to the new one. For each managed field the edit changes, the entry holding that property's unknown parameters is dropped, as they described the old value (a title's `LANGUAGE`).
- **A new Exception,** made by changing one Occurrence, takes the Recurring Event's tree, without the calendar-level entries, so the changed Occurrence keeps its alarm.
- **"This and following events":** the new Recurring Event takes the original's tree, without the calendar-level entries. This is the third case of ticket 129, on the Local Store's side.
- **A move to another Calendar** carries the neutral entries and those with a prefix the target's Provider holds, and drops the rest: Local holds `ical:`, CalDAV `ical:` and `caldav:`. So no move drops anything yet, and the rule is still written as one function that asks the target's Provider for its prefixes; a unit test gives it a `google:` entry and a Local target.
- **Undo** brings a tree back with its Event Version, with no code of its own.
- The Mirror's text stays stored and the patching writer stays in use until ticket 150.

Spec: "Local Store"; "Untrusted content"; "Moves across Accounts"; "Import/export". ADR 0017.

**Blocked by:** 143, 147

**Status:** ready-for-agent

**Effort:** M

- [ ] The tree's column on `event_version`; equality of Event Versions includes it
- [ ] An Incoming Sync and a First Sync fill it; a change on the server to an alarm alone makes a Snapshot, and Ctrl+Z after asking takes it back
- [ ] An import fills it
- [ ] An edit carries the tree and drops the unknown parameters of the fields it changed, and only those
- [ ] A new Exception and the new Recurring Event of "this and following events" have the original's tree, without the calendar-level entries
- [ ] A move between CalDAV Calendars, into a Local Calendar and out of one carries the tree
- [ ] The tree isn't in any type of the core interface
- [ ] The Store Check passes after every scenario

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of two parts of what was proposed as one ticket (L); the Sender's switch is ticket 150. Blocked by 147 and not only 146, so that the hash of an Event Version is gone before the tree becomes part of what makes two versions equal.
- 2026-11-16: decided by the user, with lines in ADR 0017: a new Exception takes the Recurring Event's tree; the prefix names the format (`ical:`), each Provider names the prefixes it holds, and a Local Calendar holds `ical:` (this replaces "a Local Calendar keeps every entry", agreed earlier the same day).
