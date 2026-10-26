Human adds items here. Remove when a ticket was created or otherwise the work is finished.

# Global
- [ ] implement 91
- [ ] simplify from work by tickets 90 on
- [ ] database:
  - [ ] document all invariants, if possible and reasonable directly in sql schema, otherwise as comments
  - [ ] all invariants within and across tables are secured by changes through single transactions
  - [ ] also secure, e.g. edit: don't allow a write from another place between read and write - otherwise risk of journal inconsistency/loss
- [ ] architecture work
    - [ ] wouldn't things be easier if we force things into a simpler structure in the db?  e.g. each exception should correspond to an occurrence. if not, then ask the user what to do with it (ask once, it's also a setting), but we won't do two exceptions for an occurrence - this only adds mess to the code, while it's almost never needed.
    - [ ] also, later, with Google recurring events, the convention is anyways different, so I guess we need to figure something out, here
    - [ ] two exceptions for one occurrence: why not show them both?
  - [ ] event::Loaded vs Event - pattern probably repeats, where what is sent to the frontend is a projection of a backend type - correct? If so, shall we develop a consistent naming scheme; e.g. backend: Event, interface to frontend: EventView; any better suggestion that suffix View?
    - [ ] naming with respect to events and recurrence - Loaded, Opened, Edited, Occurrence
  - [ ] all passes through tauri layer as single list of funciton calls, is there a more structured way? would that be better, or is it better the way it is?
- [ ] have a file, e.g. in docs/, where all decisions made are listed, e.g. a doc that can later be converted to a detailed reference for a user, or for developers; e.g. what to do when DTSTART does not fulfill the RRULE 
- [ ] caldav sync: 
  - [ ] I would like to understand the different authentication methods, and what was built in and why
- [ ] implement 15, 16
- [ ] journal::recompute_instants is weird - check again once the time zone stuff has been implemented
- [ ] test add caldav account & calendar by hand
  - [ ] rename accounts (keep account type in parens)
- [ ] ical: include STATUS field?
- [ ] repeat rules edit in the frontend (edit event): partially hide and only show all of it when the user actually wants to change it (somewhat like Google calendar)
  - [ ] frontend recurrence dates, also 2nd Wed of a month or so
- [ ] font - which one is currently used; what are the rights; what about constant-width, so that times align more often?
- [ ] paler tone for past events
- [ ] test keyboard navigation - for human
- [ ] test aria part - for human - how do I do this?
- [ ] change in today can change the view of the calendar grid, which interferes with any ongoing drag-and-drop operation; what was the solution we had for that?  Should there not be a more general solution, related to any calendar change?  Or not, it could be interesting to drag an event to the next month, by pressing n during a drag-and-drop operation.  This is possible already, actually.  But we should abort any drag-and-drop on any other change of the calendar grid!
- [ ] frontend bug: there is a weird vertical line appearing on the "week" button when changing from the month to the week view using "w" on the keyboard.
- [ ] restore window size (and status - minimized/maximized etc) on app restart - what is the best solution?  doing it from the app side or let the window manager do it?

# Ticket leftovers

## Simplify tickets 85-89

My stuff:

- [ ] greying out in the scope dialog does only affect the checkbox, but not the text next to it - but it should.  Same for the radio buttons.

Possible bug, for `/code-review`:

- [ ] code that loops over all of `Loaded.exceptions` doesn't check "Event of its own" (`Loaded::of_its_own`): `delete_all` and `carry_exceptions` may delete Exceptions without their Recurring Event that `take` keeps

Skipped:

- [ ] split `Loaded.exceptions` in `Loaded::sorted` into those paired with an Occurrence and Events of their own, so callers can't forget the check (ties into the possible bug; changes behaviour)
- [ ] do `Loaded::join` inside the writers that create or replace a Recurring Event (`Loaded::take`, `take_object`) instead of in import and sync
- [ ] dedupe all Exceptions by `SEQUENCE` in `assemble`, joined ones too, and drop `latest` from `Loaded::take`
- [ ] keep the "unreadable objects are fetched again" rule out of the CalDAV client (`Held.unreadable` in `caldav.rs`): sync passes the hrefs to fetch, so a future Remote (Google) needn't repeat it
- [ ] unique index on `server_object (calendar_id, href)`, `objects_at` returning one object; unsure whether duplicates can arise
- [ ] fewer `Loaded::of_uid` loads in import (`run`, up to three per Event) and sync (`join` loads even when a Recurring Event is stored; `moved` re-queries removed objects for each new one)
- [ ] drop `journal::hash_as` and call `hash_with(…, None)` directly
- [ ] one helper for writing a `StoredEvent` with a given recurrence ID (`take_exception`, `take_object`, `Loaded::join`)
- [ ] `ReadFile` in import: one list of Events, split by `original` in `assemble`
- [ ] old-row SQL test helper for the two migration tests in `journal.rs`
- [ ] `reschedule` in `App.svelte`: avoid the `choice!` assertions

## Ticket 08: recurrence scopes

Choices to review:

- [ ] an Exception shows its Recurring Event's repeat rule, so the whole Recurring Event can be changed from an Exception
- [ ] editing a whole Recurring Event takes its Exceptions along to another Calendar; removing its repeat deletes its Exceptions

Known limits:

- [ ] hour and minute parts of a repeat rule (`BYHOUR`, `BYMINUTE`) don't move with a time change
- [ ] a "this and following" split whose first part has only cancelled Occurrences left keeps that empty first part

## Ticket 89: Exceptions in the scope dialog (from the recurrence lessons)

- [ ] a new repeat rule for "this and following" or "all events" deletes the Exceptions whose Occurrence the new rule doesn't have (`carry_exceptions`), but the dialog doesn't say so; undo is the only way back. Suggestion: a line such as "1 event changed on its own will be removed", with the count from `Core::exception_counts`

## Tickets 07 and 57: time zones in imported Recurring Events (from the recurrence lessons)

Possible bugs, found by reading the code, not run:

- [ ] an `EXDATE` or `RECURRENCE-ID` in UTC for an Occurrence in a DST gap doesn't match it: for a rule at 02:30 `Europe/Berlin`, `EXDATE:20260329T013000Z` becomes wall time 03:30 (`recurrence_id_of` in `core/src/import/mod.rs`), but the expansion gives 02:30 (wall times dressed as UTC, `core/src/event/rule.rs`), so the Occurrence stays and an Exception doesn't attach. Start with a failing test
- [ ] a Recurring Event whose `DTSTART` is in UTC, in a file with `X-WR-TIMEZONE`, gets that zone (`utc` in `core/src/import/mod.rs`) and its rule then runs in that zone's wall time: weekly 07:00Z from October shows at 09:00 Berlin in winter, where RFC 5545 says 08:00 (the rule runs in UTC). The spec chose `X-WR-TIMEZONE` for Google's exports; decide whether it should also apply to Recurring Events

## Tickets 25 and 26: Google Recurring Events (from the recurrence lessons)

- [ ] decide the sync mode: `singleEvents=false` (masters and Exceptions; FreeCal expands rules itself, as for CalDAV, works offline for any range, can write the rule back) or `singleEvents=true` (Google expands; no rules). Strongest con of `false`: where FreeCal's expansion differs from Google's (a `DTSTART` off its rule, ticket 73; `RSCALE`), FreeCal shows other Occurrences than Google, and an Exception's `originalStartTime` can match no Occurrence
- [ ] cancelled Exceptions are stubs (only `id`, `recurringEventId`, `originalStartTime` guaranteed; other fields may be empty): store them as cancelled Exceptions "for the lifetime of the parent recurring event", never show or drop them; with `singleEvents=false` they come even without `showDeleted`
- [ ] a cancelled Occurrence can also be an `EXDATE` line in the master's `recurrence`: handle both forms
- [ ] pair an Exception with its Occurrence by `recurringEventId` + `originalStartTime`, never by the shape of `id` (documented as opaque); `recurringEventId` is the master's Google `id`, not its `iCalUID`
- [ ] `recurrence` may hold `EXRULE`, which RFC 5545 deprecated: decide what FreeCal does with it (keep it for writing back at least)
- [ ] `start.timeZone` is required on Recurring Events and is the zone the rule is expanded in (the Event Time Zone)
- [ ] an Exception may arrive on an earlier page than its master: check that the pairing of tickets 87/88 covers it
- [ ] `syncToken` can't be combined with `timeMin`/`timeMax`, `orderBy`, `q`, `iCalUID`, `updatedMin`
- [ ] unknown, test against a real account: what Google's own app writes when cancelling one Occurrence (`EXDATE` or stub) and for "this and following"; whether the fake Google server (ticket 25) should mimic it

## Ticket 17: writing to CalDAV (from the recurrence lessons)

- [ ] after a successful write, what does the mirror hold: the object FreeCal sent, or the one the server now holds? Servers may store something else than was sent: Radicale 3.8.3 adds a missing `VTIMEZONE` and reorders properties, and still returns a strong `ETag` for the `PUT`, though RFC 4791 §5.3.4 says it "MUST NOT" when the stored object isn't byte-equal. Keeping the sent object with that ETag leaves the mirror different from the server, and no sync shows it, since the ETag says nothing changed. Option: `GET` the object after each `PUT` (one more request per write). Reproduce with `~/Learning/calendar-recurrence/assets/caldav-lab.sh` and `lessons/0007/yoga.ics`, then `curl -i` on the `PUT`

## Ticket 30: opening invitations from email (from the recurrence lessons)

- [ ] an iTIP cancellation (`METHOD:CANCEL`, RFC 5546 §3.2.5) may leave out `DTSTART`, and the RFC's own examples do; import then fails the whole file as "not a calendar" (`event.get_start().ok_or_else(not_calendar)?` in `read_events`, `core/src/import/mod.rs`). Found by reading the code, not run; try `~/Learning/calendar-recurrence/lessons/0009/cancel.ics`. Import reads neither `METHOD` nor `STATUS`, so a cancellation that has a `DTSTART` comes in as an ordinary Event (see "ical: include STATUS field?"). Decide: a clear message that FreeCal doesn't handle invitation replies and cancellations, or apply a `CANCEL` to the Event the Calendar already has

## Import: versions of one Exception (from the recurrence lessons)

- [ ] `latest` (`core/src/event/mod.rs`) keeps the highest `SEQUENCE` and, on a tie, the version that comes first; `DTSTAMP` isn't read anywhere in `core/src`. RFC 5546 §2.1.5 breaks ties by the latest `DTSTAMP`. Found by reading the code, not run. Ties into "dedupe all Exceptions by `SEQUENCE` in `assemble`" above

## Ticket 80: first sync of a CalDAV Calendar

Review risks left as they are:

- [ ] a Calendar removed during its first sync stops it only once a later page writes an Event; a page of only unreadable objects keeps fetching
