# 07: Recurring Events: create, expand, delete one Occurrence

**What to build:** The user can give an Event a repeat rule (daily, weekly, monthly, yearly, with an interval and an end). Its Occurrences appear on every date they fall on, and the user can delete a single Occurrence without ending the Recurring Event.

**Blocked by:** 03, 35

**Status:** ready-for-agent

**Effort:** L

- [ ] The editor sets a repeat rule: daily, weekly, monthly or yearly, with an interval and an end
- [ ] The core expands Recurring Events into Occurrences for the requested range; the frontend never interprets repeat rules
- [ ] Deleting a single Occurrence creates a cancelled Exception and leaves the rest of the Recurring Event in place
- [ ] Import carries `RRULE`, `EXDATE` and `RECURRENCE-ID` overrides (ticket 57 skips overrides; until now an imported Recurring Event is stored as its first Occurrence)

## Comments

- 2026-10-11: Expand repeat rules in wall time in the Event Time Zone (ticket 35, ADR 0007), so a weekly 9:00 meeting stays at 9:00 across DST changes. The range query needs derived UTC bounds for the whole Recurring Event (first start, and the end of the last Occurrence or open-ended).
- 2026-10-11: The first-day-of-week setting (ticket 46) only affects display and never goes into a repeat rule. Rules keep their own `WKST`; new rules get the RFC 5545 default (`MO`), so "every 2 weeks on Sunday and Tuesday" never moves when the setting changes.
- 2026-10-17 (from ticket 59): The import preview doesn't warn that a Recurring Event is imported as its first Occurrence only. Once import carries `RRULE`, nothing needs adding; if this ticket waits long, consider a line in the preview.
