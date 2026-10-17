# 04: Views, navigation and tracking today

**What to build:** The user can switch between day, week and month views, move to the previous or next period and back to today, using buttons or Google-style shortcuts. FreeCal opens in month view on first launch, then in the last-used view, always on today. The Today button is a toggle that is pressed while the view tracks today.

**Blocked by:** 01

**Status:** done

**Effort:** M

- [x] Day, week and month views; prev / next / today
- [x] Google-style shortcuts for the views and for prev / next / today, handled by FreeCal and not FullCalendar
- [x] Month view on first launch; afterwards the last-used view is persisted as app state and restored, always opening on today
- [x] Tracking today is turned on when FreeCal opens and when Today is pressed (which also jumps to today)
- [x] It is turned off by un-pressing Today (the view stays) or by changing the date (prev/next, clicking a day number), even if the new period contains today
- [ ] … and by picking a date in the toolbar's date picker (moved to ticket 43)
- [x] It stays on across view switches, scrolling, selecting, creating or editing Events and opening dialogs
- [x] The core sends "Date changed" at midnight and on waking up on a new day (tested with the controllable clock)
- [x] On "Date changed" the today highlight moves; the view jumps to the new today only while tracking
- [x] The `classic` theme's day and week views (ticket 40) checked by hand with `run-freecal` in light and dark mode; anything that clashes with the visual style is fixed or noted

## Comments

- 2026-10-11: The week view starts on ticket 46's first-day-of-week value (Monday by default).
- 2026-10-11: Done. The `t` shortcut always goes to today and turns tracking on; only the Today button toggles. The shell asks the core every 10 seconds whether the date changed (`Core::check_date`), which also notices waking up on a new day. FullCalendar reads `now` only when it is created, so the grid is rebuilt when today changes.
- 2026-10-11: Checked by hand with `run-freecal` in light and dark mode: the day and week views fit the visual style. Two things are noted but not changed: in day view, today's tint fills the whole column, and in dark mode the pressed Today button differs only slightly from the unpressed one.
- 2026-10-11: The toolbar has no date picker yet. It should reuse ticket 43's month popover rather than build a second one, so that part of the criterion moved there.
- 2026-10-11: Added after the review: in day and week view, the month names in the title are buttons that open their month in month view, as a way back after clicking a day number. A week across two months has two, each opening its own month. Opening the shown date's month is a view switch and keeps tracking today; another month changes the date and ends it.
