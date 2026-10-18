# 71: A Takeout `.zip` into several Calendars

**What to build:** Importing a `.zip` gives each `.ics` in it its own target Calendar, which can be a new Local Calendar, so a Google Takeout export migrates several Calendars at once. Spec: US-95; "Import/export" under Implementation Decisions.

**Blocked by:** 69

**Status:** ready-for-agent

**Effort:** M

- [ ] The preview reads each file's Calendar name: `X-WR-CALNAME`, otherwise the file name without `.ics` (untrusted text)
- [ ] The preview lists each `.ics` with its counts and a Calendar select: by default the writable Calendar with the same name, otherwise "New Local Calendar “<name>”"; "Don't import" leaves the file out
- [ ] A new Local Calendar gets the next palette colour; its name can be changed in the preview
- [ ] A single `.ics` offers "New Local Calendar" in the same select
- [ ] The update / skip / copy choice stays one per import
- [ ] The whole `.zip` is one import and one Change Journal unit; undoing it removes the Calendars it created if they are empty by then, otherwise keeps them and says so
- [ ] Core tests and component tests against the fake core; checked by hand with `run-freecal` on a Takeout-like `.zip` of two Calendars

## Comments

- 2026-10-18: From grilling ticket 59's decisions (TODO.md): a `.zip` went into one Calendar, which left story 95 unmet. `X-APPLE-CALENDAR-COLOR` is ignored for now.
