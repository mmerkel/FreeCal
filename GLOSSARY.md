# FreeCal

A desktop calendar app for one local user that brings together calendars from several Google, CalDAV and ICS sources in one ergonomic view.

## Language

**Provider**:
A kind of calendar service FreeCal can talk to, such as Local, Google, CalDAV or ICS feed. Local means the Calendars live only on this computer, with no server.
_Avoid_: Backend, service, source type

**Account**:
One configured source of Calendars, belonging to exactly one Provider: a Google login, a CalDAV server with credentials, a single ICS feed URL, or the Local Account.
_Avoid_: Source, subscription, connection, profile

**Local Account**:
The single built-in Account of the Local Provider, present from the start and not removable; it holds all Calendars that live only on this computer.
_Avoid_: Offline account, default account

**Calendar**:
A named collection of Events inside an Account. An ICS-feed Account has exactly one Calendar.
_Avoid_: Feed, list, collection

**Available Calendar**:
A Calendar that a server Account offers but that the user hasn't added to FreeCal. FreeCal stores nothing for it and doesn't sync it. It appears only in "Manage Calendars…". Adding it makes it a Calendar in FreeCal; removing a Calendar from FreeCal makes it available again.
_Avoid_: Subscribe/unsubscribe (for adding or removing), unsubscribed calendar, hidden calendar (a hidden Calendar is added and syncs, just not shown)

**Read-only Calendar**:
A Calendar whose Events FreeCal never changes, either because the server doesn't allow it or because the user marked it read-only in FreeCal. Every other Calendar is writable.
_Avoid_: Locked, view-only, subscribed calendar

**Event**:
A single entry in a Calendar.
_Avoid_: Appointment, entry, item

**Recurring Event**:
An Event with a repeat rule, taken as a whole.
_Avoid_: Series, repeating event, recurrence

**Occurrence**:
One dated appearance of a Recurring Event.
_Avoid_: Instance, repetition

**Exception**:
An Occurrence that was changed or cancelled on its own, differently from the rest of its Recurring Event.
_Avoid_: Override, modified instance, detached occurrence

**Display Time Zone**:
The time zone in which FreeCal currently shows Events; the system time zone unless the user overrides it. New Events get it by default.
_Avoid_: Local time, current time zone, home time zone

## Storage

**Local Store**:
The one place on this computer where FreeCal keeps everything it knows: the local copy of every Calendar, the changes not yet sent, the Change Journal and the settings.
_Avoid_: Database, cache, local copy

## Sync

**Conflict**:
The state of an Event when a change made in FreeCal and a newer change on the server can't both be kept, so the user has to choose.
_Avoid_: Collision, sync error

**Sync Error**:
A problem that stops an Account or Calendar from syncing and isn't about a specific Event's content, such as an expired login, a server refusing a request, or a Calendar paused by the Mass-Change Guard. Being offline is not a Sync Error.
_Avoid_: Sync failure, error state

**Sync Status**:
The current sync state of one server Account or one of its Calendars (the Local Account and its Calendars have none): Syncing, Synced, Offline (its server can't be reached at the network level) or Sync Error. A Calendar paused by the Mass-Change Guard has a Sync Error.
_Avoid_: Connection state, health

## Safety

**Change Journal**:
The local record of every change FreeCal has sent to a server, and of every import, each entry holding the Event's previous version so that the change can be reversed exactly.
_Avoid_: History, log, audit trail, undo stack

**Mass-Change Guard**:
The safeguard that pauses a Calendar's sync and asks the user before one sync deletes or changes an unusually large number of its Events.
_Avoid_: Circuit breaker, sync limit

**Snapshot**:
A periodic full `.ics` copy of a Calendar, kept locally as a last-resort backup.
_Avoid_: Backup, dump, export (an export is user-initiated)
