# 71: A Takeout `.zip` into several Calendars

**What to build:** Importing a `.zip` gives each `.ics` in it its own Target Calendar among the existing Calendars, so a Google Takeout export migrates several Calendars at once. Spec: US-95; "Import/export" under Implementation Decisions; "Calendar History". ADRs 0018 ("The pointers") and 0020.

- **One Snapshot spanning its Calendars** records the whole import (ADR 0018; ticket 179).
- **An import never creates a Calendar** (ADR 0018, decided 2026-11-21). A file without a Target Calendar is left out; to bring it in, the user cancels, creates the Calendar and imports again.
- **Two files can share a Target Calendar:** their Events are added together, and the one update / skip / copy choice settles a UID they both hold.
- **Ctrl+Z on the import** takes it back whole (ADR 0020).
- **The list of sources with a Target Calendar select** is built so that the restore of Backups (ticket 11) uses it too.
- **Unimplemented Fields:** each file is read through the import's reader (ticket 186), and each target keeps the neutral entries and those with a prefix its Provider holds (ADR 0017).

**Blocked by:** 181

**Status:** ready-for-agent

**Effort:** M

- [ ] The preview reads each file's Calendar name: `X-WR-CALNAME`, otherwise the file name without `.ics` (untrusted text)
- [ ] The preview lists each `.ics` with its counts and a Target Calendar select over the writable Calendars: by default the one with the same name, otherwise "Don't import", which leaves the file out; the counts follow the select
- [ ] Two files can share a Target Calendar
- [ ] No import creates a Calendar
- [ ] The update / skip / copy choice stays one per import
- [ ] The whole `.zip` is one import and one Snapshot spanning its Target Calendars; Ctrl+Z takes it back whole
- [ ] Core tests and component tests against the fake core; checked by hand with `run-freecal` on a Takeout-like `.zip` of two Calendars

## Comments
- 2026-10-18: From grilling ticket 59's decisions (TODO.md): a `.zip` went into one Calendar, which left story 95 unmet. `X-APPLE-CALENDAR-COLOR` is ignored for now.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020, with the rule agreed there: the import is one Snapshot spanning its Calendars, a created Calendar takes it as its first Snapshot, and Ctrl+Z leaves the created Calendars and says so. "Undoing it removes the Calendars it created if they are empty by then" is gone. Blocked by 148, which has the undo of a Snapshot that is a Calendar's first; its former blocker 69 is done.
- 2026-11-21: Decided in a grilling: imports never create Calendars. Each file gets a Target Calendar among the existing writable ones or none; two files can share one. The created Calendars, their first Snapshot and the half-undo go. The list with its selects is shared with ticket 11's restore.
- 2026-11-28: References in the body renumbered for the redesign (new tickets 173 to 214). Comments above this one cite the old numbers; `redesign-plan.md`, "Old tickets in scope", maps them.
