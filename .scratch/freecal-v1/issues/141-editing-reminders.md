# 141: Editing reminders (`VALARM`)

**What to build:** The user can add, change and remove an Event's reminders in the Event editor, and the changes sync as `VALARM`s (CalDAV) and Google's `reminders`. Ticket 12 shows the reminders stored on Events but keeps them read-only, and the spec lists "editing reminders" as out of scope for v1. The first step is a grilling session with the user (/grilling); the decisions go into this ticket, into the spec and into ADRs where they are lasting, and the ticket is then split into implementation tickets.

Questions for the grilling, among others:

- Whether editing reminders comes before v1, which moves it out of the spec's "Out of Scope".
- Which reminders the editor offers: only relative to the start ("15 minutes before"), or also to the end and at an absolute time (`TRIGGER;RELATED=END`, `TRIGGER;VALUE=DATE-TIME`); presets or free entry; a limit on how many.
- Which `ACTION`s FreeCal writes (`DISPLAY` only?) and what it does with the `EMAIL` and `AUDIO` alarms it can't show: kept untouched, shown read-only, or editable.
- Default reminders for new Events: per Calendar, per Account or none; Google's default reminders (`reminders.useDefault`) and CalDAV's `default-alarm-vevent-datetime` (RFC 7986 / Apple).
- How an edited `VALARM` is written: its `UID` (RFC 9074), the properties FreeCal doesn't know inside it, and Thunderbird's `X-MOZ-*` and Apple's `ACKNOWLEDGED` state. Whether snooze and dismiss (ticket 12) are written to the server or kept local.
- Recurring Events: whether a reminder change goes through the recurrence scope prompt like any other field.
- Whether reminder changes are recorded in the Change Journal, count for the Mass-Change Guard, and can cause Conflicts.
- Untrusted content: the reminders read from servers and imports, and alarm descriptions.

Spec: US-101, US-102; "Out of Scope"; "Untrusted content".

**Blocked by:** 12

**Status:** ready-for-human

**Effort:** L

- [ ] Grilling session with the user; decisions recorded here, in the spec and in ADRs
- [ ] Split into implementation tickets with Effort and Blocked by lines

## Comments

- 2026-11-15: From the overview of iCalendar fields in `ical.md`.
