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

**Log**:
A text file in which FreeCal notes, for troubleshooting, what it skipped or couldn't do, such as a server resource it ignored. Nothing in FreeCal reads it back, and it is not the Change Journal.
_Avoid_: Diagnostic log, debug log

## Sync

**Conflict**:
The state of an Event when a change made in FreeCal and a newer change on the server can't both be kept, so the user has to choose.
_Avoid_: Collision, sync error

**Server Version**:
The Events a server holds for one Calendar and UID, the Recurring Event with its Exceptions, as FreeCal reads them; empty when the server deleted them. In a Conflict it stands against the User's Version, shown as "Server version".
_Avoid_: Remote version, their version, server copy

**User's Version**:
The Events FreeCal holds for one Calendar and UID, with the changes not sent yet, whether from an edit, an import or an undo. In a Conflict it stands against the Server Version, shown as "Your version".
_Avoid_: Local version (Local is for Local Accounts and Calendars), mine, client version

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
_Avoid_: History, audit trail, undo stack

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

**CalDAV Server Object**:
A Server Object of a CalDAV Calendar: one resource, holding the components of exactly one UID, kept as the iCalendar text received. It is the Server Object of its Calendar and UID, whichever href holds it, so a move to another href doesn't make it another one.
_Avoid_: Calendar object resource, CalDAV event

**Google Server Object**:
A Server Object of a Google Calendar: one Google event resource, a single Event, a Recurring Event or one Exception, kept as the JSON received.
_Avoid_: Google event (on its own)

**Mirror**:
FreeCal's copy of every Server Object as the server last had it, in tables of its Provider. Only the Sync Engine writes it, and it goes with its Calendar.
_Avoid_: Cache, server copy

**Ignored Resource**:
A CalDAV resource the Sync Engine fetched but doesn't mirror, such as one holding several UIDs or one it couldn't read. It is remembered with its ETag, so that it isn't fetched again until it changes; one it couldn't read is remembered without, so that it is fetched on every Incoming Sync until it reads. It is not part of the Mirror and has no Event Rows.
_Avoid_: Skipped object, rejected resource

**Forget an Ignored Resource**:
Remove an Ignored Resource from the Sync Engine's memory, as an Incoming Sync finds it gone from the server, mirrored now, or holding nothing FreeCal keeps.
_Avoid_: Unignore, release

**Ambiguous UID**:
A UID that more than one CalDAV resource of a Calendar holds, which RFC 4791 §4.1 forbids. One of them keeps the UID: it is mirrored, as the UID's Corresponding Server Object, and the others are Ignored Resources. The first by a fixed order keeps it among new ones; once mirrored, it keeps it for as long as it holds it, and when it stops, an Ignored Resource holding the UID takes over (ADR 0015). A resource holding several UIDs makes none of them ambiguous, as it is ignored for that.
_Avoid_: Duplicate UID, shared UID

**Known Server Resources**:
The Server Objects FreeCal holds and the Ignored Resources it remembers in a Calendar, by address, with their versions: what an Incoming Sync compares the server's listing with, so that it fetches only what changed and finds what went.
_Avoid_: Known set, known objects

**Combined Event Key**:
A Calendar, a UID and a recurrence ID as the Event Row holds it, compared by Occurrence: what an Unsent Change is about, and what the Sender finds a Server Object by.
_Avoid_: Key (on its own), Event Key (on its own), Event Key in Calendar, Event ID (that is the Event Row's own ID)

**Recurrence ID as obtained**:
An Exception's recurrence ID in the text it arrived in, from a file, a server or FreeCal: a date, a wall time with no zone, or a wall time with the zone it came in. Event Rows, Unsent Changes and Server Objects store it so, and code compares it only by Occurrence, through its Normal Form (ADR 0012).
_Avoid_: Recurrence ID as given, raw recurrence ID, normalised recurrence ID

**Normal Form**:
The form of a recurrence ID that names its Occurrence against the Recurring Event: the Occurrence's wall time in the Recurring Event's Event Time Zone, or its date when the Recurring Event is all-day; without a Recurring Event, the recurrence ID as obtained. It exists only in memory, and the window gets an Occurrence's as its ID.
_Avoid_: Normalised recurrence ID, canonical form

**Corresponding Server Object**:
The one Server Object of a Calendar that a UID's Event Rows belong to. On CalDAV, the one CalDAV Server Object holding that UID, as any other resource holding it is an Ignored Resource; on Google, the Google Server Object holding that UID and Occurrence.
_Avoid_: Owner, primary object, home object

**Propagate a Server Object**:
Bring what a Server Object holds into the Event Rows of its UID, as part of its Server Version: each Occurrence is replaced, added or deleted to match, unless an Unsent Change about it keeps the User's Version or raises a Conflict. Always "to its Event Rows", from the server inwards.
_Avoid_: Take, apply, reconcile, rewrite Combined Event Keys

**Database Transaction**:
One SQLite transaction on the Local Store, committed all or nothing. Every write takes one; a Journal Transaction runs inside one, and so do the Mirror's writes of an Incoming Sync.
_Avoid_: Transaction (on its own), db transaction

**Write Permission**:
A value that only one module can make and that every write to the tables it guards takes, so the compiler keeps other code from writing them. The Change Journal's covers Event Rows, the Change Journal, Unsent Changes, Conflicts and deleting Calendars and accounts; the Sync Engine's covers the Mirror and the Ignored Resources.
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
The code that exchanges a Calendar's changes with its server, by Incoming Syncs and Outgoing Syncs. It is the only code that writes the Mirror.
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
