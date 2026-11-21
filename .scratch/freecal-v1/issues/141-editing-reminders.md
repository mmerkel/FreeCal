# 141: Editing reminders (`VALARM`)

**What to build:** The user can add, change and remove an Event's reminders in the Event editor, and the changes sync as `VALARM`s (CalDAV) and Google's `reminders`. Ticket 12 shows the reminders stored on Events but keeps them read-only, and the spec lists "editing reminders" as out of scope for v1. The first step is a grilling session with the user (/grilling); the decisions go into this ticket, into the spec and into ADRs where they are lasting, and the ticket is then split into implementation tickets.

Questions for the grilling, among others:

- Whether editing reminders comes before v1, which moves it out of the spec's "Out of Scope".
- Which reminders the editor offers: only relative to the start ("15 minutes before"), or also to the end and at an absolute time (`TRIGGER;RELATED=END`, `TRIGGER;VALUE=DATE-TIME`); presets or free entry; a limit on how many.
- Which `ACTION`s FreeCal writes (`DISPLAY` only?) and what it does with the `EMAIL` and `AUDIO` alarms it can't show: kept untouched, shown read-only, or editable.
- Default reminders for new Events: per Calendar, per Account or none; Google's default reminders (`reminders.useDefault`) and CalDAV's `default-alarm-vevent-datetime` (RFC 7986 / Apple).
- How an edited `VALARM` is written: its `UID` (RFC 9074), the properties FreeCal doesn't know inside it (kept with the reminder since ticket 12 made reminders a managed field), and Thunderbird's `X-MOZ-*` and Apple's `ACKNOWLEDGED` state. Whether snooze and dismiss (ticket 12) are written to the server or kept local.
- Recurring Events: whether a reminder change goes through the recurrence scope prompt like any other field.
- Reminders are part of the Event Version, so a change to them is a Snapshot in the Calendar History and is undone like any other (ADR 0017). To decide: whether such a change counts for the Mass-Change Guard, and what the comparison of the two sides does when both changed an Event's reminders: a Conflict, as for the other managed fields, or the server's win, as ticket 12 leaves it.
- Untrusted content: the reminders read from servers and imports, and alarm descriptions.

Spec: US-101, US-102; "Out of Scope"; "Untrusted content".

**Blocked by:** 12

**Status:** ready-for-human

**Effort:** L

- [ ] Grilling session with the user; decisions recorded here, in the spec and in ADRs
- [ ] Split into implementation tickets with Effort and Blocked by lines

## Comments

- 2026-11-15: From the overview of iCalendar fields in `ical.md`.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: the questions name the Calendar History and the Unimplemented Fields. Effort L is for the work after the grilling, which the second box splits under the M cap.
- 2026-11-21 (review with the user): this ticket will make reminders editable. Ticket 12 makes them a column of the Event Version (agreed), so the editor works on that column. When editing comes (before v1 or after) is still the grilling's first question.
