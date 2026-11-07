# 12: Reminder notifications

**What to build:** FreeCal shows desktop notifications for the reminders stored on Events, and the user can snooze or dismiss them. Reminders are not editable in v1; they come from servers and imports.

**Blocked by:** 03

**Status:** ready-for-agent

**Effort:** L

- [ ] A notification is shown when a reminder stored on an Event (or Occurrence) becomes due (tested with the controllable clock and a recording fake notifier)
- [ ] Snooze re-shows the notification later; dismiss stops it
- [ ] No Signals are used for reminders
- [ ] Notification bodies are escaped: the hostile test Event shows its title and location literally, with no markup interpreted (checked with the recording fake notifier)

## Comments
- 2026-11-07: Re-sized from M to L. Nothing reads reminders yet: neither import nor CalDAV keeps `VALARM`, so this ticket also reads and stores them (from files and Server Objects, untrusted), works out when each is due across a Recurring Event's Occurrences, and keeps snoozes and dismissals.
