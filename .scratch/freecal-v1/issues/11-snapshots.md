# 11: Snapshots

**What to build:** FreeCal takes a compressed Snapshot of each changed Calendar once a day, keeps the last 14, skips Calendars unchanged since their last Snapshot, and lets the user restore a Calendar from a Snapshot.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] A daily compressed `.ics` Snapshot per changed Calendar in the data directory, including Local Calendars (tested with the controllable clock)
- [ ] Unchanged Calendars are skipped
- [ ] Only the last 14 Snapshots per Calendar are kept
- [ ] The user can list Snapshots and restore a Calendar from one, under the modal "Working…" dialog; restore sends "Occurrences changed"
