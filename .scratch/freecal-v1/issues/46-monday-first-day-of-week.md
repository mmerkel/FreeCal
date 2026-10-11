# 46: Weeks start on Monday by default

**What to build:** Weeks start on Monday by default, whatever the system locale. Everything that lays out weeks (the grid's month and week views, the date field's popover) reads one shared first-day-of-week value, so that ticket 06's Settings option only has to change that value. The value only affects display, never repeat rules. Spec: user story 55, "Time zones and locale" under Implementation Decisions.

**Blocked by:** none

**Status:** done

- [x] One frontend module holds the first day of week (`monday`, `saturday` or `sunday`, default `monday`) with a `setFirstDayOfWeek` switch, following the pattern of `src/lib/clock.svelte.ts`; `CalendarGrid` sets FullCalendar's `firstDay` from it
- [x] The month view starts its rows on Monday on an `en-US` system (component test with an `en-US` locale)
- [x] Setting the value to Sunday or Saturday in a test moves the grid's first column at once, so ticket 06 has a tested switch to wire to Settings
- [x] Date formats still follow the system locale

Out of scope: the Settings option itself (ticket 06), the date field's popover (ticket 43, which reads this value), the week view (ticket 04), week numbers.

## Comments

- 2026-10-11: Done. The value lives in `src/lib/week.svelte.ts` (`setFirstDayOfWeek` is the switch for ticket 06; `firstWeekdayNumber` gives FullCalendar's `firstDay`). The grid reads it when it is created, so the first range it reports already starts on Monday, and follows changes through `setOption`.
