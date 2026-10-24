Human adds items here. Remove when a ticket was created or otherwise the work is finished.

- [ ] architecture - recurring events
  - [ ] afterwards check:
    - [ ] import.rs calling into new module
    - [ ] wiring between store and new module - it seems new module neds store through journal, but it is also a dependency for store 
- [ ] compare context on fresh session
- [ ] explicitly ask on how to reduce token usage with my current setup
- [ ] architecture work
- [ ] event::Loaded vs Event - pattern probably repeats, where what is sent to the frontend is a projection of a backend type - correct? If so, shall we develop a consistent naming scheme; e.g. backend: Event, interface to frontend: EventView; any better suggestion that suffix View?
  - [ ] also no single owner of events table
  - [ ] all basses throgh tauri layer as single list of funciton calls, is there a more structured way? would that be better, or is it better the way it is?
- [ ] caldav sync: 
  - [ ] I would like to understand all ids and tokens involved (event ID, UID, etag, sync token, etc), and how the code deals with it
  - [ ] I would like to understand the different authentication methods, and what was built in and why
- [ ] implement 15, 16
- [ ] test add caldav account & calendar by hand
  - [ ] rename accounts (keep account type in parens)
- [ ] repeat rules edit in the frontend (edit event): partially hide and only show all of it when the user actually wants to change it (somewhat like Google calendar)
- [ ] font - which one is currently used; what are the rights; what about constant-width, so that times align more often?
- [ ] paler tone for past events
- [ ] test keyboard navigation - for human
- [ ] test aria part - for human - how do I do this?
- [ ] change in today can change the view of the calendar grid, which interferes with any ongoing drag-and-drop operation; what was the solution we had for that?  Should there not be a more general solution, related to any calendar change?  Or not, it could be interesting to drag an event to the next month, by pressing n during a drag-and-drop operation.  This is possible already, actually.  But we should abort any drag-and-drop on any other change of the calendar grid!
- [ ] frontend bug: there is a weird vertical line appearing on the "week" button when changing from the month to the week view using "w" on the keyboard.
- [ ] restore window size (and status - minimized/maximized etc) on app restart - what is the best solution?  doing it from the app side or let the window manager do it?
- [ ] dialog choosing this and following etc: different button arrangement (all in onw column, maybe) or radio buttons like Google calendar

## Ticket 08: recurrence scopes

Choices to review:

- [ ] the recurrence scope is asked after Save (as in Google Calendar), not before editing; "This event" isn't offered when the repeat rule was changed
- [ ] the editor for an Occurrence starts from that Occurrence's date, not the Recurring Event's first date
- [ ] an Exception shows its Recurring Event's repeat rule, so the whole Recurring Event can be changed from an Exception
- [ ] "This event" moved to another Calendar becomes an Event of its own, no longer part of the Recurring Event
- [ ] editing a whole Recurring Event takes its Exceptions along to another Calendar; removing its repeat deletes its Exceptions

Known limits:

- [ ] hour and minute parts of a repeat rule (`BYHOUR`, `BYMINUTE`) don't move with a time change
- [ ] a "this and following" split whose first part has only cancelled Occurrences left keeps that empty first part
- [ ] "All events" drops Exceptions beyond the 50,000th Occurrence
- [ ] no warning yet for moving an Occurrence across Accounts (comes with server Accounts)

## Ticket 80: first sync of a CalDAV Calendar

Review risks left as they are:

- [ ] a Calendar removed during its first sync stops it only once a later page writes an Event; a page of only unreadable objects keeps fetching
- [ ] server objects pass from the CalDAV client to the iCalendar reading as plain `(name, text)` pairs (`import::read_server_events`), not a named type
