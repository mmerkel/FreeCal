# 12: Reminder notifications

**What to build:** FreeCal shows desktop notifications for the reminders stored on Events, and the user can snooze or dismiss them. Reminders are not editable in v1; they come from servers and imports.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] A notification is shown when a reminder stored on an Event (or Occurrence) becomes due (tested with the controllable clock and a recording fake notifier)
- [ ] Snooze re-shows the notification later; dismiss stops it
- [ ] No Signals are used for reminders
