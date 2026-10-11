# 36: 24-hour clock by default

**What to build:** Times are shown on a 24-hour clock by default, whatever the system locale. All time formatting (the grid's slot labels and Event times, the details, the editor) reads one shared clock value, so that ticket 06's 12-hour option only has to change that value. Spec: user story 55, "Time zones and locale" under Implementation Decisions.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] One frontend module holds the clock (`24` or `12`, default `24`) and the formatting helpers that use it; `when.ts` and `CalendarGrid` format times only through it
- [ ] The grid's slot labels and Event times and the details' time line show 24-hour times on an `en-US` system (component tests with an `en-US` locale); the editor follows with ticket 38
- [ ] Setting the clock to 12-hour in a test switches every place to AM/PM, so ticket 06 has a tested switch to wire to Settings
- [ ] Date formats still follow the system locale

Out of scope: the Settings option itself (ticket 06).

## Comments

- 2026-10-11: WebKitGTK's native time input shows AM/PM whatever the locale (seen on a French system), so the editor's times only follow the clock once ticket 38 replaces that input.
