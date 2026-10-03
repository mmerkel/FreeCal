# 04: Views, navigation and tracking today

**What to build:** The user can switch between day, week and month views, move to the previous or next period and back to today, using buttons or Google-style shortcuts. FreeCal opens in month view on first launch, then in the last-used view, always on today. The Today button is a toggle that is pressed while the view tracks today.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] Day, week and month views; prev / next / today
- [ ] Google-style shortcuts for the views and for prev / next / today, handled by FreeCal and not FullCalendar
- [ ] Month view on first launch; afterwards the last-used view is persisted as app state and restored, always opening on today
- [ ] Tracking today is turned on when FreeCal opens and when Today is pressed (which also jumps to today)
- [ ] It is turned off by un-pressing Today (the view stays) or by changing the date (prev/next, date picker, clicking a day number), even if the new period contains today
- [ ] It stays on across view switches, scrolling, selecting, creating or editing Events and opening dialogs
- [ ] The core sends "Date changed" at midnight and on waking up on a new day (tested with the controllable clock)
- [ ] On "Date changed" the today highlight moves; the view jumps to the new today only while tracking
