# 43: Date field with typed entry and a month popover

**What to build:** The editor's start and end dates use a field of FreeCal's own instead of the native `type="date"` input, whose GTK popover draws a grey band over the field and which shows US order whatever the locale. The user can type a date in their date format or pick it from a small month calendar that shows the Event's range. Spec: user story 142, "Date field" under Frontend.

**Blocked by:** 38, 46

**Status:** ready-for-agent

- [ ] A pure parser (unit tests) follows the order of the date format, accepts `.`, `/` and `-` as separators and ISO `2026-10-04`; `4` means the 4th of the shown month, `4/10` that day in the shown year; two-digit years mean 20xx; no words
- [ ] The field shows the date in the date format and normalises typed text on Enter, Tab or blur; invalid text stays, marked red with `aria-invalid`, and blocks Save
- [ ] A popover (ARIA date-picker dialog) shows a month grid with ‹ › for months, today ringed and the selected day in the accent colour
- [ ] Weeks start on ticket 46's first-day-of-week value (Monday by default); no week numbers
- [ ] Keyboard: ↓ opens it; arrows move by day, PgUp/PgDn by month, Enter picks, Escape closes only the popover, not the dialog
- [ ] End-date popover: the start date is marked, the days between start and end are highlighted, days before the start are disabled
- [ ] Start-date popover: the end date and the range are shown dimmed; every day is selectable, since the end follows the start (ticket 38)
- [ ] Shares the popover and list code with ticket 38's time field where they overlap
- [ ] The toolbar gets a date picker built on the same popover (moved from ticket 04): picking a date shows its period in the current view and ends tracking today
- [ ] Every string goes through `t`
- [ ] Component tests for typing, the popover's keys and range marks; checked by hand in the real app with `run-freecal`, including that no grey band appears

Out of scope: words such as "tomorrow".
