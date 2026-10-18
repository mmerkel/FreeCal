# 49: Display Time Zone shown and followed

**What to build:** The window shows the Display Time Zone (the system time zone), and FreeCal follows it when the system time zone changes: the core sends "Display Time Zone changed" and the frontend redraws the shown range and its label. FullCalendar uses the Display Time Zone rather than the browser's local zone. Spec: US-50 and US-51. Split from ticket 06.

**Blocked by:** 03, 35

**Status:** ready-for-agent

**Effort:** M

- [ ] The core returns Occurrences already converted to the Display Time Zone (already so since ticket 03; keep it covered by a core test)
- [ ] The Display Time Zone is shown in the UI
- [ ] When the system time zone changes, the core sends "Display Time Zone changed"; the frontend redraws the range and the label (core test with `TestClock::set_time_zone`; component test against the fake core)
- [ ] FullCalendar is given the Display Time Zone, so the grid stays right once ticket 51 lets it differ from the browser's zone
- [ ] Every string goes through `t()`

Out of scope: the Event Time Zone in the editor (ticket 50); the override in Settings (ticket 51); formats, first day of week and clock options (ticket 52).

## Comments

- 2026-10-13: Split from ticket 06. Ticket 03's note: the `Clock` gained `time_zone()` (the system zone via `iana-time-zone` and `chrono-tz`), and `TestClock::set_time_zone` controls it.
