# 52: Date format from the locale, with formats, first day and clock in Settings

**What to build:** The date format follows the system locale, and FullCalendar's own labels use it. The Date & time section of the Settings dialog (ticket 33) lets the user override the format region, start weeks on Saturday or Sunday, and switch to a 12-hour clock. Spec: user stories 55 and 128. Split from ticket 06.

**Blocked by:** 33, 36, 46

**Status:** ready-for-agent

**Effort:** M

- [ ] The date format comes from the system locale; FullCalendar's locale is wired to it (ticket 01's note: its month title and weekday names come from its default locale today)
- [ ] A "Formats" region picker: "System" plus a curated list of about 30 regions (major English, European, Latin American and Asian formats), names shown through `Intl.DisplayNames`; each entry shows 31 December of the current year in short and long form (e.g. "31.12.2026 · 31. Dezember 2026"). Dates are formatted with `Intl.DateTimeFormat` for the chosen region
- [ ] A first day of week option (Monday, Saturday, Sunday), defaulting to Monday and setting ticket 46's shared value (`setFirstDayOfWeek` in `src/lib/week.svelte.ts`)
- [ ] A clock option (24-hour, 12-hour), defaulting to 24-hour and setting ticket 36's shared value (`setClockHours` in `src/lib/clock.svelte.ts`), applied to the grid, the details and the editor at once
- [ ] Neither the first day nor the clock has a "System" choice or a `system` value in the Local Store
- [ ] The overrides are settings in the Local Store, written as partial updates through ticket 33's operations
- [ ] Component tests against the fake core for the pickers and that the grid follows each override
- [ ] Every string goes through `t()`

## Comments

- 2026-10-13: Split from ticket 06. Ticket 51 also adds to the Date & time section; whichever comes first makes the section appear (ticket 33 hides a section with no settings).
