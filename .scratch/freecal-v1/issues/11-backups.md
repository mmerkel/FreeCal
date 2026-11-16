# 11: Backups

**What to build:** FreeCal takes a compressed Backup of each changed Calendar once a day, keeps the last 14, skips Calendars unchanged since their last Backup, and lets the user restore a Calendar from a Backup.

**Blocked by:** 03

**Status:** ready-for-agent

**Effort:** M

- [ ] A daily compressed `.ics` Backup per changed Calendar in the data directory, including Local Calendars (tested with the controllable clock)
- [ ] Unchanged Calendars are skipped
- [ ] Only the last 14 Backups per Calendar are kept
- [ ] The user can list Backups and restore a Calendar from one, under the modal "Working…" dialog; restore sends "Occurrences changed"
