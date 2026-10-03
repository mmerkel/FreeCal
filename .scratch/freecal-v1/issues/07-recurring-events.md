# 07: Recurring Events: create, expand, delete one Occurrence

**What to build:** The user can give an Event a repeat rule (daily, weekly, monthly, yearly, with an interval and an end). Its Occurrences appear on every date they fall on, and the user can delete a single Occurrence without ending the Recurring Event.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] The editor sets a repeat rule: daily, weekly, monthly or yearly, with an interval and an end
- [ ] The core expands Recurring Events into Occurrences for the requested range; the frontend never interprets repeat rules
- [ ] Deleting a single Occurrence creates a cancelled Exception and leaves the rest of the Recurring Event in place
