# 51: Override the Display Time Zone in Settings

**What to build:** The Date & time section of the Settings dialog (ticket 33) lets the user view their calendar in a time zone other than the system's. Spec: US-127. Split from ticket 06.

**Blocked by:** 49, 50

**Status:** ready-for-agent

**Effort:** S

- [ ] Date & time in Settings: a searchable list of IANA time zones (ticket 50's list) with "System (<zone>)" at the top
- [ ] While an override is set, changes to the system time zone are ignored; changing the override sends "Display Time Zone changed" like a system change (core test)
- [ ] The override is a setting in the Local Store, written as a partial update through ticket 33's operations
- [ ] Component tests against the fake core for the picker and that the grid follows the override
- [ ] Every string goes through `t()`

## Comments

- 2026-10-13: Split from ticket 06. Ticket 52 also adds to the Date & time section; whichever comes first makes the section appear (ticket 33 hides a section with no settings).
