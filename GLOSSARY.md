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
An Event with a repeat rule, taken as a whole: its rule, its Occurrences and its Exceptions. An Exception without its Recurring Event, gone or not yet arrived, or that changes none of its Occurrences, is an Event of its own.
_Avoid_: Series, repeating event, recurrence

**Location**:
Where an Event happens, as free text: an address, a room or a video-call link.
_Avoid_: Place, venue

**Occurrence**:
One dated appearance of a Recurring Event.
_Avoid_: Instance, repetition

**Exception**:
An Occurrence that was changed or cancelled on its own, differently from the rest of its Recurring Event.
_Avoid_: Override, modified instance, detached occurrence

**Display Time Zone**:
The time zone in which FreeCal currently shows Events; the system time zone unless the user overrides it. New Events get it by default.
_Avoid_: Local time, current time zone, home time zone

**Event Time Zone**:
The time zone an Event's times are given in, such as 9:00 in Europe/Berlin. Every Event with a time of day has one; all-day Events have none.
_Avoid_: Original time zone, floating time (FreeCal has none)

## Storage

**Local Store**:
The one place on this computer where FreeCal keeps everything it knows: the local copy of every Calendar, the changes not yet sent, the Change Journal and the settings.
_Avoid_: Database, cache, local copy

## Sync

**Conflict**:
The state of an Event when a change made in FreeCal and a newer change on the server can't both be kept, so the user has to choose.
_Avoid_: Collision, sync error

**Unsent Change**:
A change to an Event in a writable server Calendar that FreeCal has made in the Local Store but not yet sent to the server, kept until a send succeeds. Each counts once per Event, however often it changed meanwhile.
_Avoid_: Pending change, queued change, outbox entry

**Sync Error**:
A problem that stops an Account or Calendar from syncing and isn't about a specific Event's content, such as an expired login, a server refusing a request, or a Calendar paused by the Mass-Change Guard. Being offline is not a Sync Error.
_Avoid_: Sync failure, error state

**Sync Status**:
The current sync state of one server Account or one of its Calendars (the Local Account and its Calendars have none): Syncing, Synced, Offline (its server can't be reached at the network level) or Sync Error. A Calendar paused by the Mass-Change Guard has a Sync Error.
_Avoid_: Connection state, health

## Safety

**Change Journal**:
The local record of every change to an Event, whether made in FreeCal or pulled from a server. Each change is a Journal Event Modification holding the Event's previous Event Version, so that the change can be reversed exactly. Changes made together, such as an import or one sync of a Calendar, form one Journal Transaction, which records its Transaction Origin.
_Avoid_: History, log, audit trail, undo stack

**Journal Transaction**:
Changes recorded together in the Change Journal, such as one user action, an import or one sync of a Calendar, with their Transaction Origin. It runs as one database transaction, and all writes to Event Rows go through one.
_Avoid_: Unit, transaction (on its own: a plain database transaction), batch, change set

**Journal Event Modification**:
One change to one Event inside a Journal Transaction, holding the Event's previous Event Version and the hash of its new one. Creating and deleting an Event are Journal Event Modifications too.
_Avoid_: Modification (on its own), entry, journal entry, change (on its own: Unsent Change and Change Journal use it)

**Event Version**:
The fields FreeCal manages of one Event at one moment, as a Journal Event Modification stores it. Server data is not part of it.
_Avoid_: Version (on its own), revision, snapshot (a Snapshot is a backup file)

**Mass-Change Guard**:
The safeguard that pauses a Calendar's sync and asks the user before one sync deletes or changes an unusually large number of its Events.
_Avoid_: Circuit breaker, sync limit

**Snapshot**:
A periodic full `.ics` copy of a Calendar, kept locally as a last-resort backup.
_Avoid_: Backup, dump, export (an export is user-initiated)

## Internals

Terms for the code and the ADRs, never shown to the user. Each needs the user's agreement before it is used (see `CLAUDE.md`).

**Event Row**:
One stored Event in the `event` table: a single Event, a Recurring Event or one Exception. A Recurring Event with Exceptions is several Event Rows.
_Avoid_: Row (on its own), event table row, record

**Loaded Event**:
An Event read from the Local Store as one value, a Recurring Event together with its rule and Exceptions (`event::Loaded`).
_Avoid_: Opened event, full event

**Server Object**:
What a server stores as one piece and FreeCal writes back: one CalDAV resource at its href, or one Google event resource. It can hold several Event Rows.
_Avoid_: Resource (on its own), remote event, original server data

**Mirror**:
FreeCal's copy of every Server Object as the server last had it. Only the Sync Engine changes its server data; its Combined Event Keys are renormalised together with the Event Rows, and it goes with its Calendar.
_Avoid_: Cache, server copy

**Combined Event Key**:
A Calendar and an Event Key in Calendar: a Calendar, a UID and a recurrence ID, with the recurrence ID normalised as the Event Rows hold it. Event Rows find their Server Object through its Combined Event Keys (`combined_event_key`); an Unsent Change is a Combined Event Key.
_Avoid_: Key (on its own), Event Key (on its own), Event ID (that is the Event Row's own ID)

**Event Key in Calendar**:
A UID and a recurrence ID, normalised as the Event Rows hold it: a Combined Event Key without its Calendar, for where something else gives the Calendar, as a Server Object gives it its Combined Event Keys. It is unique only within one Calendar.
_Avoid_: Event Key (on its own), UID key

**Corresponding Server Object**:
The one Server Object of a Calendar that a UID's Event Rows belong to: of the Server Objects holding that UID, the one holding the Recurring Event, otherwise the first one mirrored. Any other Server Object holding the UID has no Event Rows.
_Avoid_: Owner, primary object, home object

**Corresponding UIDs**:
The UIDs a Server Object is the Corresponding Server Object of; it corresponds to each of them.
_Avoid_: Owned UIDs

**Rewrite Combined Event Keys**:
Replace a Server Object's Combined Event Keys of one UID, its data unchanged, for when its Event Rows come to hold their recurrence IDs otherwise.
_Avoid_: Rekey

**Database Transaction**:
One SQLite transaction on the Local Store, committed all or nothing. Every write takes one; a Journal Transaction runs inside one, and so do the Mirror's writes of an Incoming Sync.
_Avoid_: Transaction (on its own), db transaction

**Write Permission**:
A value that only one module can make and that every write to the tables it guards takes, so the compiler keeps other code from writing them. The Change Journal's covers Event Rows, the Change Journal, Unsent Changes, deleting Calendars and accounts, and renormalising Combined Event Keys with their Event Rows; the Sync Engine's covers the Mirror's server data.
_Avoid_: Token, Journaled, owner

**Guard**:
The module that alone writes a set of tables, by holding their Write Permission: the Change Journal or the Sync Engine. Not the Mass-Change Guard, which is user-facing.
_Avoid_: Owner, gatekeeper

**Unguarded Write**:
A write to the Local Store that no Guard's Write Permission covers, such as to accounts, Calendars, settings, the sidebar, the view or the time zone rules. It still runs in a Database Transaction.
_Avoid_: Free write, open write

**Record a Journal Event Modification**:
Store one Journal Event Modification in the Change Journal, starting the Journal Transaction with its first one.
_Avoid_: Entry, add entry

**Sync Engine**:
The code that exchanges a Calendar's changes with its server, by Incoming Syncs and Outgoing Syncs. It is the only code that changes the Mirror's server data.
_Avoid_: Syncer, sync module, server code

**Incoming Sync**:
The Sync Engine bringing in what the server changed in one Calendar, as one Journal Transaction. A First Sync is a Calendar's first Incoming Sync. "Sync" on its own means both directions, as the user sees it.
_Avoid_: Sync (on its own, for this direction), pull, fetch

**Outgoing Sync**:
The Sync Engine sending a Calendar's Unsent Changes through its Sender; each one that succeeds changes the Mirror.
_Avoid_: Send (on its own), push, upload

**Sender**:
The code that sends the Unsent Changes, comparing each Combined Event Key's Event Rows with its Server Object to decide between create, update and delete.
_Avoid_: Uploader, pusher, outbox

**Transaction Origin**:
Where a Journal Transaction's changes came from: a user action, an import, an undo, a First Sync or an Incoming Sync.
_Avoid_: Origin (on its own), source, cause

**First Sync**:
A Calendar's first Incoming Sync, which the Change Journal doesn't record because undoing it would delete the whole Calendar.
_Avoid_: Baseline (that word is the squashed schema migration, ADR 0008), initial import

**Dangerous Undo**:
An undo that also undoes later changes, reverts a change from a server, or exceeds the Mass-Change Guard's thresholds, and so asks first. Every other undo is benign and runs at once.
_Avoid_: Risky undo, unsafe undo

**Pinned Zone**:
The Event Time Zone FreeCal gave an Event that arrived with floating times (ADR 0007). It stays until the user changes the Event's time.
_Avoid_: Assumed zone, default zone
