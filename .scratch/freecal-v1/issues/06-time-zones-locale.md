# 06: Time zones and locale

**What to build:** Events are shown in the Display Time Zone (the system time zone), which is shown in the UI. New Events get it by default, and the editor lets the user pick a different time zone for an Event. The date format follows the system locale; weeks start on Monday (ticket 46) and the clock is 24-hour (ticket 36) by default. The Date & time section of the Settings dialog (ticket 33) lets the user override the Display Time Zone, the format region, first day of week and the clock. Spec: user stories 50–55, 127 and 128.

**Blocked by:** 03, 33, 35, 36, 46

**Status:** wontfix

**Split into:** 49, 50, 51, 52

- [ ] The core returns Occurrences already converted to the Display Time Zone
- [ ] The Display Time Zone is shown in the UI
- [ ] New Events default to the Display Time Zone; the editor lets the user choose another one per Event
- [ ] Events created in other time zones appear at the correct local time
- [ ] When the system time zone changes, the core sends "Display Time Zone changed"; the frontend redraws the range and label; an open editor keeps its draft, including the draft's own time zone (tested with a controllable fake system time zone)
- [ ] The date format comes from the system locale
- [ ] Date & time in Settings: a searchable list of IANA time zones (`Intl.supportedValuesOf('timeZone')`) with "System (<zone>)" at the top; while an override is set, changes to the system time zone are ignored; changing the override sends "Display Time Zone changed" like a system change (core test)
- [ ] A "Formats" region picker: "System" plus a curated list of about 30 regions (major English, European, Latin American and Asian formats), names shown through `Intl.DisplayNames`; each entry shows 31 December of the current year in short and long form (e.g. "31.12.2026 · 31. Dezember 2026"). Dates are formatted with `Intl.DateTimeFormat` for the chosen region
- [ ] A first day of week option (Monday, Saturday, Sunday), defaulting to Monday and setting ticket 46's shared first-day value, and a clock option (24-hour, 12-hour), defaulting to 24-hour and setting ticket 36's shared clock value, applied to the grid, the details and the editor at once
- [ ] The overrides are settings in the Local Store, written as partial updates through ticket 33's operations
- [ ] Component tests against the fake core for the pickers and that the grid follows each override

## Comments

- 2026-10-11: Ticket 35 stores wall time plus a start and an end Event Time Zone (ADR 0007). This ticket adds the Event Time Zone to `When` so the editor can show and change it. If ticket 10 or 14 is done first and has added the mark for pinned zones, the details and editor show "The original has no time zone. FreeCal shows it in <zone>." for such Events (ADR 0007, rule 5); otherwise whichever of them comes first adds the note.
- 2026-10-11: The clock is now 24-hour by default, whatever the locale (story 55 changed). Ticket 36 builds the shared clock value; this ticket only adds the 24-hour / 12-hour option, with no "System" choice.
- 2026-10-11: Weeks now start on Monday by default, whatever the locale (story 55 changed). Ticket 46 builds the shared first-day value; this ticket only adds the Monday / Saturday / Sunday option, with no "System" choice. Neither it nor the clock has a `system` value in the Local Store.
- 2026-10-13: Split into 49 (Display Time Zone shown and followed), 50 (Event Time Zone in the editor), 51 (override the Display Time Zone in Settings) and 52 (date format from the locale, with formats, first day and clock in Settings). Work from those; this file stays for its history.
