Human adds items here. Remove when a ticket was created or otherwise the work is finished.

- [ ] discuss remainder of tickets below

## Ticket 08: recurrence scopes

Choices to review:

- [ ] an Exception shows its Recurring Event's repeat rule, so the whole Recurring Event can be changed from an Exception
- [ ] editing a whole Recurring Event takes its Exceptions along to another Calendar; removing its repeat deletes its Exceptions

Known limits:

- [ ] hour and minute parts of a repeat rule (`BYHOUR`, `BYMINUTE`) don't move with a time change
- [ ] a "this and following" split whose first part has only cancelled Occurrences left keeps that empty first part

## Ticket 80: first sync of a CalDAV Calendar

Review risks left as they are:

- [ ] a Calendar removed during its first sync stops it only once a later page writes an Event; a page of only unreadable objects keeps fetching

## Simplify tickets 85-89

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

# Global

- [ ] squash db migrations
- [ ] architecture work
  - [ ] table with events, table with server objects; task of the later is related to sync, but (after tickets 86-89), it will also be used to associate events that belong to the same recurring event.  This does not feel right.
    - [ ] also, later, with Google recurring events, the convention is anyways different, so I guess we need to figure something out, here
    - [ ] I would like to better understand how occurrences and exceptions work in ical and caldav
      - [ ] how are they linked in Google calendar?
      - [ ] Why is it needed that exceptions stay associated to their recurring event?
    - [ ] two exceptions for one occurrence: why not show them both?
  - [ ] event::Loaded vs Event - pattern probably repeats, where what is sent to the frontend is a projection of a backend type - correct? If so, shall we develop a consistent naming scheme; e.g. backend: Event, interface to frontend: EventView; any better suggestion that suffix View?
    - [ ] naming wtih respect to events and recurrence - Loaded, Opened, Edited, Occurrence
  - [ ] also no single owner of events table (journal, store)
  - [ ] all passes through tauri layer as single list of funciton calls, is there a more structured way? would that be better, or is it better the way it is?
- [ ] caldav sync: 
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

