# 12: Reminder notifications

**What to build:** FreeCal shows desktop notifications for the reminders stored on Events, and the user can snooze or dismiss them. Reminders aren't editable here; they come from servers and imports, and ticket 141 makes them editable.

- **Where reminders are today:** since ticket 186 an Event's alarms (`VALARM`) are kept among its Unimplemented Fields, under `ical:` names, from servers and from imports. Nothing reads them.
- **Reminders become a managed field** (ADR 0017, "When a field changes home"): a column of the Event Version. The reader (ticket 175) fills it and the writer (ticket 176) writes it. A migration, in Rust, moves the alarms of every Event Version from the tree to the column; an alarm FreeCal can't read stays an entry under its `ical:` name and is written back untouched. Properties FreeCal doesn't know inside an alarm are kept with it. Every stored reminder is untrusted.
- **Still part of the Event Version:** a server's change to a reminder makes a Snapshot and is undone like any other change. Until ticket 141 the user can't change reminders, so the user's side never differs in them by itself and no Conflict arises from them alone; the comparison of the two sides takes the server's, as it did for the Unimplemented Fields. Ticket 141, which makes them editable, decides what holds then.
- **When a reminder is due** is worked out across a Recurring Event's Occurrences and its Exceptions, on the controllable clock.
- **Snoozes and dismissals** are kept in the Local Store beside the Calendar History, not in the Event Version: they make no Snapshot, aren't sent and aren't undone. Ticket 141 decides whether they are ever written to a server.

Spec: US-101, US-102; "Notifications" under Desktop interfaces; "Untrusted content". ADR 0017.

**Blocked by:** 03, 176, 186

**Status:** ready-for-agent

**Effort:** L

- [ ] Reminders are a column of the Event Version, read by the reader, written by the writer; the migration moves them out of the Unimplemented Fields
- [ ] A notification is shown when a reminder stored on an Event (or Occurrence) becomes due (tested with the controllable clock and a recording fake notifier)
- [ ] Snooze re-shows the notification later; dismiss stops it; neither makes a Snapshot
- [ ] No Signals are used for reminders
- [ ] Notification bodies are escaped: the hostile test Event shows its title and location literally, with no markup interpreted (checked with the recording fake notifier)

## Comments
- 2026-11-07: Re-sized from M to L. Nothing reads reminders yet: neither import nor CalDAV keeps `VALARM`, so this ticket also reads and stores them (from files and Server Objects, untrusted), works out when each is due across a Recurring Event's Occurrences, and keeps snoozes and dismissals.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The comment of 2026-11-07 is outdated in one point: alarms are now kept, as Unimplemented Fields (ticket 149), so this ticket no longer reads them from nothing; it moves them to a column instead. Blocked by 149 as agreed, and by 144 for the writer. Effort stays L: it was sized before the M cap (2026-11-16) and wasn't on the triage's list of splits. A split into "reminders become a managed field" and "notifications, snooze, dismiss" would bring both parts to M; for the user to decide.
- 2026-11-21 (review with the user): reminders as a column of the Event Version agreed; ticket 141 will make them editable.
- 2026-11-28: References in the body renumbered for the redesign (new tickets 173 to 214). Comments above this one cite the old numbers; `redesign-plan.md`, "Old tickets in scope", maps them.
