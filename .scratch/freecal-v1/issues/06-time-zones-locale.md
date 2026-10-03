# 06: Time zones and locale

**What to build:** Events are shown in the Display Time Zone (the system time zone), which is shown in the UI. New Events get it by default, and the editor lets the user pick a different time zone for an Event. First day of week, 12/24-hour clock and date format follow the system locale.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] The core returns Occurrences already converted to the Display Time Zone
- [ ] The Display Time Zone is shown in the UI
- [ ] New Events default to the Display Time Zone; the editor lets the user choose another one per Event
- [ ] Events created in other time zones appear at the correct local time
- [ ] When the system time zone changes, the core sends "Display Time Zone changed"; the frontend redraws the range and label; an open editor keeps its draft, including the draft's own time zone (tested with a controllable fake system time zone)
- [ ] First day of week, 12/24-hour clock and date format come from the system locale
