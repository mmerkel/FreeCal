# 50: Event Time Zone in the editor

**What to build:** New Events get the Display Time Zone, and the editor lets the user choose a different time zone for an Event, so "9:00 New York" can be entered as such. Events created in other time zones appear at the correct local time. An open editor keeps its draft, including the draft's own time zone, when the Display Time Zone changes. Spec: US-52–US-54. Split from ticket 06.

**Blocked by:** 49

**Status:** ready-for-agent

**Effort:** M

- [ ] The core interface's `When` carries the Event Time Zone (start and end zone, ADR 0007); regenerate the TypeScript types
- [ ] New Events default to the Display Time Zone
- [ ] The editor shows the Event's time zone and lets the user choose another one from a searchable list of IANA time zones (`Intl.supportedValuesOf('timeZone')`); build the list as a component ticket 51 can reuse
- [ ] Events created in other time zones appear at the correct local time (core test)
- [ ] An open editor keeps its draft, including the draft's own time zone, when "Display Time Zone changed" arrives (component test against the fake core)
- [ ] The unit tests in `core/src/event.rs` that ticket 35 left for this (an edit from Berlin keeps America/New_York, new Events get the Display Time Zone) move to the interface tests
- [ ] Every string goes through `t()`

## Comments

- 2026-10-13: Split from ticket 06, which noted: if ticket 57 or 54 is done first and has added the mark for pinned zones, the details and editor show "The original has no time zone. FreeCal shows it in <zone>." for such Events (ADR 0007, rule 5); otherwise whichever of them comes first adds the note.
- 2026-10-25 (from 89): Add ticket 89's last test here: changing a Recurring Event's Event Time Zone with "all events" moves its Exceptions' recurrence IDs and its cancelled Occurrences with it, so none becomes an Event of its own (`core/tests/recurrence_scopes.rs`). Then tick 89's box and close it.
