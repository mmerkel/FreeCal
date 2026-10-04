# 06: Time zones and locale

**What to build:** Events are shown in the Display Time Zone (the system time zone), which is shown in the UI. New Events get it by default, and the editor lets the user pick a different time zone for an Event. First day of week, 12/24-hour clock and date format follow the system locale. The Date & time section of the Settings dialog (ticket 33) lets the user override the Display Time Zone, the format region, first day of week and the clock. Spec: user stories 50–55, 127 and 128.

**Blocked by:** 03, 33

**Status:** ready-for-agent

- [ ] The core returns Occurrences already converted to the Display Time Zone
- [ ] The Display Time Zone is shown in the UI
- [ ] New Events default to the Display Time Zone; the editor lets the user choose another one per Event
- [ ] Events created in other time zones appear at the correct local time
- [ ] When the system time zone changes, the core sends "Display Time Zone changed"; the frontend redraws the range and label; an open editor keeps its draft, including the draft's own time zone (tested with a controllable fake system time zone)
- [ ] First day of week, 12/24-hour clock and date format come from the system locale
- [ ] Date & time in Settings: a searchable list of IANA time zones (`Intl.supportedValuesOf('timeZone')`) with "System (<zone>)" at the top; while an override is set, changes to the system time zone are ignored; changing the override sends "Display Time Zone changed" like a system change (core test)
- [ ] A "Formats" region picker: "System" plus a curated list of about 30 regions (major English, European, Latin American and Asian formats), names shown through `Intl.DisplayNames`; each entry shows 31 December of the current year in short and long form (e.g. "31.12.2026 · 31. Dezember 2026"). Dates are formatted with `Intl.DateTimeFormat` for the chosen region
- [ ] First day of week (System, or a day) and clock (System, 12-hour, 24-hour) overrides, each defaulting to System, applied to the grid and the editor at once
- [ ] The overrides are settings in the Local Store, written as partial updates through ticket 33's operations
- [ ] Component tests against the fake core for the pickers and that the grid follows each override
