# 121: Events with guests are read-only

**What to build:** The second part of ticket 17 (read it for the history). Any Event with attendees other than the user is read-only, as an Event of a Read-only Calendar is. Attendee data is never shown and always preserved (spec: "Attendees").

- The core reads whether an Event has guests from the ATTENDEE properties of its Corresponding Server Object, and the core interface says so on the Event. The core refuses edits, moves and deletes of such an Event, as for a Read-only Calendar.
- Decide how the core knows the user's own address, so that an Event whose only attendee is the user stays writable (for CalDAV, for example, the principal's `calendar-user-address-set`), and record it in a comment. Google (ticket 26) will need the same answer.
- The Event's details say "has guests" with the hint to edit it on the server; the grid offers no move or resize for it, and a refused drag explains why (US-31, US-32).

Spec: US-31, US-32, US-46; "Attendees"; "Event details".

**Blocked by:** 56

**Status:** done

**Effort:** M

- [x] Any Event with attendees other than the user is read-only; the core refuses changes to it
- [x] Its details say "has guests" with the hint to edit it on the server; the grid offers no move or resize, and a refused drag explains why
- [x] Core tests for who counts as a guest; component tests for the details, the grid and the refused drag

## Comments
- 2026-11-08 (implemented): who counts as a Guest lives in `core/src/guests.rs`, whose top comment records how the core knows the user. An Account's Own Addresses are read once, when the Account is added: for CalDAV, the principal's `calendar-user-address-set`, read in the same request as `calendar-home-set`, plus the username when it is an email address (Radicale gives only the principal's path); for Google (ticket 26), the Google account's address. They are kept in `account.own_addresses`. Each CalDAV Server Object stores its `ATTENDEE`s (alarm attendees excluded) in `caldav_server_object.attendees`, worked out from its text whenever the Mirror writes it. Addresses are compared without `mailto:` and in lower case. An Event has guests when its Corresponding Server Object names an attendee who isn't one of the Own Addresses; this makes the whole Recurring Event read-only, since an Exception with a guest does. Stored values FreeCal can't read keep the Event read-only. The Change Journal refuses every write to such an Event except an Incoming Sync's (`CoreError::EventHasGuests`); this covers user actions, imports and undos. `Event.has_guests` and `Occurrence.has_guests` are in the interface. The details offer only ✕ and say "Has guests, so it can't be changed in FreeCal. Edit it in Google or on its server for now." The grid offers no move or resize, and a refused drag shows a toast that names the Read-only Calendar first, otherwise the guests.
- Open for the user (from /code-review):
  - With "update", an import that meets an existing Event with guests fails as a whole, as an import into a Read-only Calendar does. Counting it as skipped instead would change the import's rules.
  - Events without a Server Object never have guests: those of Local Calendars, and server Events not yet sent. Attendees of imported files aren't kept at all yet.
  - Own Addresses aren't read again later, so an address the server adds afterwards leaves its Events read-only.
  - The edited Occurrence that `Core::edit_occurrence` returns sets `has_guests: false` without reading it, as the Change Journal refused any change to an Event with guests.
- New terms (approved by the user, 2026-11-08): **Guest**, an attendee of an Event who isn't the user (user-facing as "has guests"); **Own Addresses**, the calendar user addresses that name the user on an Account's server (`account.own_addresses`). In the code only: `caldav::Discovered`, what CalDAV discovery finds (the Calendars and the user's addresses as the server gave them); and the "form addresses compare in" (`guests::compared`).
