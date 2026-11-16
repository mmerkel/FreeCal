# 71: A Takeout `.zip` into several Calendars

**What to build:** Importing a `.zip` gives each `.ics` in it its own target Calendar, which can be a new Local Calendar, so a Google Takeout export migrates several Calendars at once. Spec: US-95; "Import/export" under Implementation Decisions; "Calendar History". ADRs 0018 ("The pointers") and 0020.

- **One Snapshot spanning its Calendars** records the whole import (ADR 0018; ticket 146).
- **A Local Calendar the import creates** enters the Local Store together with that Snapshot as its first, in the same Database Transaction, as a First Sync's Calendar does with its own. It has no empty Snapshot before it.
- **Ctrl+Z on the import** takes it back in the Calendars that existed and leaves the created ones as they are, since undo can't go before a Calendar's first Snapshot (ADR 0020). FreeCal says so, naming the Calendars left (ticket 148's answer carries them; the toast's wording is ticket 158's kind). To be rid of a created Calendar, the user deletes it.
- **Unimplemented Fields:** each file is read through the import's reader (ticket 149), and each target keeps the neutral entries and those with a prefix its Provider holds (ADR 0017).

**Blocked by:** 148

**Status:** ready-for-agent

**Effort:** M

- [ ] The preview reads each file's Calendar name: `X-WR-CALNAME`, otherwise the file name without `.ics` (untrusted text)
- [ ] The preview lists each `.ics` with its counts and a Calendar select: by default the writable Calendar with the same name, otherwise "New Local Calendar “<name>”"; "Don't import" leaves the file out
- [ ] A new Local Calendar gets the next palette colour; its name can be changed in the preview
- [ ] A single `.ics` offers "New Local Calendar" in the same select
- [ ] The update / skip / copy choice stays one per import
- [ ] The whole `.zip` is one import and one Snapshot spanning its Calendars; a Calendar it creates takes that Snapshot as its first
- [ ] Ctrl+Z takes the import back in the Calendars that existed, leaves the created ones, and FreeCal names them
- [ ] Core tests and component tests against the fake core; checked by hand with `run-freecal` on a Takeout-like `.zip` of two Calendars

## Comments
- 2026-10-18: From grilling ticket 59's decisions (TODO.md): a `.zip` went into one Calendar, which left story 95 unmet. `X-APPLE-CALENDAR-COLOR` is ignored for now.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020, with the rule agreed there: the import is one Snapshot spanning its Calendars, a created Calendar takes it as its first Snapshot, and Ctrl+Z leaves the created Calendars and says so. "Undoing it removes the Calendars it created if they are empty by then" is gone. Blocked by 148, which has the undo of a Snapshot that is a Calendar's first; its former blocker 69 is done.
