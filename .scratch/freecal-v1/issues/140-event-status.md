# 140: The status of an Event (`STATUS`: tentative, confirmed, cancelled)

**What to build:** FreeCal reads an Event's `STATUS` (RFC 5545 §3.8.1.11: `TENTATIVE`, `CONFIRMED`, `CANCELLED`) and shows it, so that a meeting its organiser cancelled doesn't look like a normal Event. Today FreeCal doesn't show `STATUS`; from ticket 149 on it keeps it among the Event's Unimplemented Fields and writes it back unchanged (ADR 0017). Outlook and Google often leave a cancelled meeting in the attendee's calendar with `STATUS:CANCELLED` instead of deleting it. The first step is a grilling session with the user (/grilling); the decisions go into this ticket, into the spec and into ADRs where they are lasting, and the ticket is then split into implementation tickets.

Questions for the grilling, among others:

- Which statuses FreeCal shows, and how: a cancelled Event struck through, faded or hidden; a tentative one marked or not (the spec's "faded look already means past or tentative", "Event chips stay sparse").
- Whether the user can set the status in the Event editor, or it is read-only and only kept.
- A cancelled Exception of a Recurring Event (`RECURRENCE-ID` with `STATUS:CANCELLED`) against a cancelled Occurrence (`EXDATE`): whether both show the same way, and what deleting an Occurrence writes (US-44).
- Google Accounts: Google's `status` field (`confirmed`, `tentative`, `cancelled`), where `cancelled` also marks deleted Events and Exceptions in incremental sync.
- Whether a cancelled Event counts in Conflict badges, reminders (ticket 12) and the busy time of `TRANSP`.
- Where the status lives: it is a candidate neutral name (ADR 0017; ticket 163 assigns them), and showing it or letting the user set it makes it a managed field with a column and a migration.
- How a status change shows in the Calendar History: a server's change to it is a Snapshot today, listed as "changed in fields FreeCal doesn't show". Whether a change of status alone can be a Conflict once it is managed.
- Untrusted content: an unknown or malformed `STATUS` value.

Spec: US-44; "Untrusted content"; "Visual style"; "Event chips stay sparse".

**Blocked by:** none

**Status:** ready-for-human

**Effort:** M

- [ ] Grilling session with the user; decisions recorded here, in the spec and in ADRs
- [ ] Split into implementation tickets with Effort and Blocked by lines

## Comments

- 2026-11-15: From the overview of iCalendar fields in `ical.md`, where `STATUS` was the one important field that shows Events wrongly and had no ticket.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: the questions name the Calendar History and the Unimplemented Fields, and the status is a candidate neutral name.
