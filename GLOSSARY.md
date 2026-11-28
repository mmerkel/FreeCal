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
A Calendar marked read-only in FreeCal, by the user or, when it is added, by FreeCal because its server doesn't allow changes. The user can't change its Events in FreeCal. The mark only stops new changes: Unsent Changes made before it are still sent. A Calendar whose server refuses changes but that isn't marked is writable, and its changes wait under a Sync Error. Every other Calendar is writable.
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
An Event changed differently in FreeCal and on the server in the same part (its times, title, Location, description, or the fields FreeCal doesn't show), or in its times on one side and another part it shows on the other, or as a whole Recurring Event on one side and in one Occurrence on the other. FreeCal takes the server's version and keeps the user's; the Conflict stays open until the user resolves it. Changes to different parts are combined instead, in a Server Event Merge, and are no Conflict.
_Avoid_: Collision, sync error

**Server Version**:
The Events a server holds for one Calendar and UID, the Recurring Event with its Exceptions, as FreeCal reads them; empty when the server deleted them. In a Conflict it stands against the User's Version, shown as "Server version".
_Avoid_: Remote version, their version, server copy

**User's Version**:
The Events FreeCal holds for one Calendar and UID, with the changes not sent yet, whether from an edit, an import or an undo. In a Conflict it stands against the Server Version, shown as "Your version".
_Avoid_: Local version (Local is for Local Accounts and Calendars), mine, client version

**Last Agreed Version**:
The last Server Version that FreeCal and the server both held for one Calendar and UID, before either side changed it: the version with the server's mark, against which an Incoming Sync tells what each side changed. A Conflict keeps it, to show what each side changed.
_Avoid_: Common version, common reference version, base version, ancestor, original

**Resolution**:
How a Conflict ends: by the user keeping the server's version, restoring their own, or changing the Event again. FreeCal keeps the resolved Conflict for as long as its Incoming Sync Snapshot.
_Avoid_: Merge (a Server Event Merge ends no Conflict), Automatic Resolution, conflict decision

**Unsent Change**:
A change to an Event in a writable server Calendar that FreeCal has made in the Local Store but not yet sent to the server: the user's current version of the Event differs from the one the server holds. Each counts once per Event, however often it changed meanwhile. Until the code follows ADR 0019, they are stored as a list until a send succeeds.
_Avoid_: Pending change, queued change, outbox entry

**Sync Error**:
A problem that stops an Account or Calendar from syncing and isn't about a specific Event's content, such as an expired login, a server refusing a request or refusing all changes to a Calendar (writes refused), or a Calendar paused by the Mass-Change Guard. Being offline is not a Sync Error.
_Avoid_: Sync failure, error state

**Sync Status**:
The current sync state of one server Account or one of its Calendars (the Local Account and its Calendars have none): Syncing, Synced, Offline (its server can't be reached at the network level) or Sync Error. A Calendar paused by the Mass-Change Guard has a Sync Error.
_Avoid_: Connection state, health

## Safety

**Change Journal**:
The local record of every change to an Event, whether made in FreeCal or pulled from a server. Each change is a Journal Event Modification holding the Event's previous Event Version, so that the change can be reversed exactly. Changes made together, such as an import or one sync of a Calendar, form one Journal Transaction, which records its Transaction Origin. Goes when the code follows ADR 0018.
_Avoid_: History, audit trail, undo stack

**Journal Transaction**:
Changes recorded together in the Change Journal, such as one user action, an import or one sync of a Calendar, with their Transaction Origin. It runs as one database transaction, and all writes to Event Rows go through one. Goes when the code follows ADR 0018.
_Avoid_: Unit, transaction (on its own: a plain database transaction), batch, change set

**Journal Event Modification**:
One change to one Event inside a Journal Transaction, holding the Event's previous Event Version and the hash of its new one. Creating and deleting an Event are Journal Event Modifications too. Goes when the code follows ADR 0018.
_Avoid_: Modification (on its own), entry, journal entry, change (on its own: Unsent Change and Change Journal use it)

**Event Version**:
One Event at one moment: the fields FreeCal manages and its Unimplemented Fields, for a single Event, a Recurring Event or one Exception. Once stored it never changes; a change to the Event makes a new one. Until the code follows ADR 0018, the Change Journal's stored Event Versions hold the managed fields only.
_Avoid_: Version (on its own), revision, snapshot (a Snapshot is a whole Calendar's state)

**Undo**:
A Snapshot that takes back one earlier Snapshot: what that Snapshot changed goes back to how it was before, and everything else stays. It takes back every Event it can; where it would leave some out, because something it would take back has changed since or their Calendar is read-only or removed, it asks first. A redo is an Undo of an Undo.
_Avoid_: Reverting Undo, revert, rollback

**Undo History**:
What Ctrl+Z and Ctrl+Shift+Z step through, read from the Calendar History: for Ctrl+Z the user's own changes, imports and restores still in effect, newest first across all Calendars, passing over what the server brought in; for Ctrl+Shift+Z the Undos still in effect, newest first, until the user's next change. A restart doesn't reset it.
_Avoid_: Session Undo, undo stack, History (on its own: an Event's History)

**Mass-Change Guard**:
The safeguard that pauses a Calendar and asks the user before one Outgoing Sync deletes or changes an unusually large number of its Events on the server.
_Avoid_: Circuit breaker, sync limit

**Backup**:
A periodic full `.ics` copy of a Calendar, kept locally as a last resort.
_Avoid_: Snapshot (a Snapshot is a state in the Calendar History), dump, export (an export is user-initiated)

**Target Calendar**:
The existing Calendar that one source of an import or a restore goes into: an `.ics` file, or one Calendar's Backup. A source without one is left out.
_Avoid_: Association, mapping, destination

**Snapshot**:
The state of every Calendar's Events at one moment, as the Calendar History keeps it. Each change makes a new one from the one before: an action of the user, an import, a restore, an Undo, what an Incoming Sync brings in (an Incoming Sync Snapshot, with a Server Event Merge or a Server Event Replacement for each Event both sides changed), or "Stop waiting and mark read-only". A step that changes several Calendars together, such as a move, is one Snapshot.
_Avoid_: Commit, version (on its own), backup (a Backup is a file)

**Calendar History**:
The line of Snapshots of all Calendars together, each arisen from the one before.
_Avoid_: History (on its own: an Event's History), undo history (the Undo History is what Ctrl+Z steps through), log

## Internals

Terms for the code and the ADRs, never shown to the user. Each needs the user's agreement before it is used (see `CLAUDE.md`).

**Status Mark**:
The coloured badge showing a status (`src-tauri/icons/src/mark-<status>.svg`): the overall one in the window's status indicator and on the tray icon's calendar, and an Account's or a Calendar's own on its line in the sidebar.
_Avoid_: Badge, overlay, status icon

**Unsent Mark**:
The hourglass on an Event, and on its Calendar's line in the sidebar, while the user's version of the Event differs from the server's.
_Avoid_: Pending icon, dashed outline, unsent count

**Event Row**:
One stored Event in the `event` table: a single Event, a Recurring Event or one Exception. A Recurring Event with Exceptions is several Event Rows. Until the code follows ADR 0018; then `event` is a view of the user's current Event Versions.
_Avoid_: Row (on its own), event table row, record

**Loaded Event**:
An Event read from the Local Store as one value, a Recurring Event together with its rule and Exceptions (`event::Loaded`).
_Avoid_: Opened event, full event

**Unimplemented Fields**:
What a server holds about one Event beyond the fields FreeCal manages, such as alarms, attachments, custom properties and unknown parameters. FreeCal keeps them without showing them.
_Avoid_: Unknown fields, unmanaged fields, not-yet-implemented fields, server data (on its own)

**Incoming Sync Snapshot**:
The Snapshot in which an Incoming Sync takes in what it fetched from one Calendar. The server's versions of Events only the server changed, or in Conflict (a Server Event Replacement), become current; those of Events merged are zero-length Event Versions beside the combination (a Server Event Merge). Each Event's part can be undone alone. A First Sync's takes in everything. Ctrl+Z passes over it.
_Avoid_: Server Intake, Join, pull, fetch (both name the Incoming Sync)

**Full Fetch**:
An Incoming Sync that fetches every resource of a Calendar and compares it with the versions carrying the server's mark, because no sync token or ETags say what changed: a First Sync, one after the server reset its sync token, and every Incoming Sync of a server without ETags; also a Mark Reconciliation.
_Avoid_: Initial load, resync, full sync

**Mark Reconciliation**:
A Full Fetch run because a Calendar's last Full Fetch is more than a week old, to check the server's Current-Version Marks against what the server holds.
_Avoid_: Resync, consistency check, audit

**Server Event Replacement**:
What an Incoming Sync Snapshot does to one Event in Conflict: the Server Version takes the place of the User's Version, and the Conflict is recorded. Undoing the Incoming Sync Snapshot for that Event restores the User's Version.
_Avoid_: Override (an Exception's name elsewhere), merge, auto-merge, resolution (the Resolution is the user's)

**Conflict Event**:
One single Event of a Conflict, the Recurring Event or one Exception, found on either side or in the Last Agreed Version, with that Last Agreed Version, if there is one: a row of `conflict_event`. A Conflict on a Recurring Event has one per Event of its UID.
_Avoid_: Conflict row, conflict entry

**Server Event Merge**:
What an Incoming Sync Snapshot does to one Event that both sides changed in different field groups: it takes their combination, each group from the side that changed it. No Conflict: it is sent like any change. Undoing the Incoming Sync Snapshot for that Event restores the User's Version without the server's change. Short form: Merge.
_Avoid_: Automatic Resolution, auto-merge, merge (lowercase: a Snapshot with two parents, `design/snapshot-tree`)

**Field Group**:
One of the parts an Event is compared by where both sides changed it: the times (with the time zone and the repeat rule), the title, the Location, the description, or the Unimplemented Fields. Changes to different Field Groups are combined; changes to the same one, or to the times against another group of the fields FreeCal shows, are a Conflict.
_Avoid_: Part (on its own), field set, section

**Zero-length Event Version**:
An Event Version that begins and ends with the same Incoming Sync Snapshot: the server's version of an Event merged, in no Snapshot's state and never shown in the grid, kept to carry the server's Current-Version Mark.
_Avoid_: Server-only row, ghost version

**Current-Version Mark**:
One of two marks on an Event Version, saying that it is the current version of its Event on that side. The user's is that the Event Version hasn't ended in the Calendar History; the server's is stored, and is FreeCal's record of what the server holds. An Event Version that both sides agree on has both.
_Avoid_: Mark (on its own: a Status Mark is a badge, an Unsent Mark an hourglass), current flag, head

**Store Check**:
The function that asks a Local Store whether its stored invariants hold, each as a named query, and reports every violation. The tests run it, and debug builds after every write to the Calendar History; it is where those invariants are recorded.
_Avoid_: Integrity check (SQLite's own, of the file), validation, fsck

**Server Object**:
What a server stores as one piece and FreeCal writes back: one CalDAV resource at its href, or one Google event resource. It can hold several Event Rows. Until the code follows ADR 0018 it holds Event Rows; then Event Versions.
_Avoid_: Resource (on its own), remote event, original server data

**CalDAV Server Object**:
A Server Object of a CalDAV Calendar: one resource, holding the components of exactly one UID, kept as the iCalendar text received. It is the Server Object of its Calendar and UID, whichever href holds it, so a move to another href doesn't make it another one. Until the code follows ADR 0017; then it is kept as the Event Versions read from it, with no text.
_Avoid_: Calendar object resource, CalDAV event

**Google Server Object**:
A Server Object of a Google Calendar: one Google event resource, a single Event, a Recurring Event or one Exception, kept as the JSON received. Until the code follows ADR 0017; then it is kept as the Event Versions read from it, with no JSON.
_Avoid_: Google event (on its own)

**Mirror**:
FreeCal's copy of every Server Object as the server last had it, in tables of its Provider. Only the Sync Engine writes it, and it goes with its Calendar. Until the code follows ADRs 0017 and 0019; then the server's content is held as the Event Versions with the server's Current-Version Mark, and the CalDAV table keeps only each resource's href and ETag.
_Avoid_: Cache, server copy

**Ignored Resource**:
A CalDAV resource the Sync Engine fetched but doesn't mirror, such as one holding several UIDs or one it couldn't read. It is remembered with its ETag, so that it isn't fetched again until it changes; one it couldn't read is remembered without, so that it is fetched on every Incoming Sync until it reads. It is not part of the Mirror and has no Event Rows. Until the code follows ADR 0019; then "mirror" reads "keep in the CalDAV table", and it has no Event Versions.
_Avoid_: Skipped object, rejected resource

**Forget an Ignored Resource**:
Remove an Ignored Resource from the Sync Engine's memory, as an Incoming Sync finds it gone from the server, mirrored now, or holding nothing FreeCal keeps.
_Avoid_: Unignore, release

**Ambiguous UID**:
A UID that more than one CalDAV resource of a Calendar holds, which RFC 4791 §4.1 forbids. One of them keeps the UID: it is mirrored, as the UID's Corresponding Server Object, and the others are Ignored Resources. The first by a fixed order keeps it among new ones; once mirrored, it keeps it for as long as it holds it, and when it stops, an Ignored Resource holding the UID takes over (ADR 0015). A resource holding several UIDs makes none of them ambiguous, as it is ignored for that.
_Avoid_: Duplicate UID, shared UID

**Known Server Resources**:
The Server Objects FreeCal holds and the Ignored Resources it remembers in a Calendar, by address, with their versions: what an Incoming Sync compares the server's listing with, so that it fetches only what changed and finds what went. Until the code follows ADR 0019; then the resources the CalDAV table keeps and the Ignored Resources.
_Avoid_: Known set, known objects

**Combined Event Key**:
A Calendar, a UID and a recurrence ID as the Event Row holds it, compared by Occurrence: what an Unsent Change is about. The Sender merges the Combined Event Keys of a Calendar and UID where a Server Object holds a whole UID, as on CalDAV.
_Avoid_: Key (on its own), Event Key (on its own), Event Key in Calendar, Event ID (that is the Event Row's own ID)

**Recurrence ID as obtained**:
An Exception's recurrence ID in the text it arrived in, from a file, a server or FreeCal: a date, a wall time with no zone, or a wall time with the zone it came in. Event Rows, Unsent Changes and Server Objects store it so, and code compares it only by Occurrence, through its Normal Form (ADR 0012).
_Avoid_: Recurrence ID as given, raw recurrence ID, normalised recurrence ID

**Normal Form**:
The form of a recurrence ID that names its Occurrence against the Recurring Event: the Occurrence's wall time in the Recurring Event's Event Time Zone, or its date when the Recurring Event is all-day; without a Recurring Event, the recurrence ID as obtained. It exists only in memory, and the window gets an Occurrence's as its ID.
_Avoid_: Normalised recurrence ID, canonical form

**Corresponding Server Object**:
The one Server Object of a Calendar that a UID's Event Rows belong to. On CalDAV, the one CalDAV Server Object holding that UID, as any other resource holding it is an Ignored Resource; on Google, the Google Server Object holding that UID and Occurrence. Until the code follows ADR 0018 it is of Event Rows; then of Event Versions.
_Avoid_: Owner, primary object, home object

**Propagate a Server Object**:
Bring what a Server Object holds into the Event Rows of its UID, as part of its Server Version: each Occurrence is replaced, added or deleted to match, unless an Unsent Change about it keeps the User's Version or raises a Conflict. Always "to its Event Rows", from the server inwards. Goes when the code follows ADR 0019.
_Avoid_: Take, apply, reconcile, rewrite Combined Event Keys

**Database Transaction**:
One SQLite transaction on the Local Store, committed all or nothing. Every write takes one; a Journal Transaction runs inside one, and so do the Mirror's writes of an Incoming Sync.
_Avoid_: Transaction (on its own), db transaction

**Write Permission**:
A value that only one module can make and that every write to the tables it guards takes, so the compiler keeps other code from writing them. The Change Journal's covers Event Rows, the Change Journal, Unsent Changes, Conflicts and deleting Calendars and accounts; the Sync Engine's covers the Mirror and the Ignored Resources. Goes when the code follows ADR 0018.
_Avoid_: Token, Journaled, owner

**Guard**:
The module that alone writes a set of tables, by holding their Write Permission: the Change Journal or the Sync Engine. Not the Mass-Change Guard, which is user-facing. Goes when the code follows ADR 0018.
_Avoid_: Owner, gatekeeper

**Unguarded Write**:
A write to the Local Store that no Guard's Write Permission covers, such as to accounts, Calendars, settings, the sidebar, the view or the time zone rules. It still runs in a Database Transaction. Goes when the code follows ADR 0018.
_Avoid_: Free write, open write

**Record a Journal Event Modification**:
Store one Journal Event Modification in the Change Journal, starting the Journal Transaction with its first one. Goes when the code follows ADR 0018.
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
The code that sends the Unsent Changes, comparing the Event Rows they are about with their Corresponding Server Object to decide between create, update and delete. On CalDAV it sends per Calendar and UID, one resource for all its Unsent Changes, whatever their recurrence IDs. Until the code follows ADR 0019; then it sends the Events whose user's current version differs from the one with the server's mark, built from the user's Event Versions alone, and reads no Unsent Changes.
_Avoid_: Uploader, pusher, outbox

**Sent Event Rows**:
The Event Rows of one Calendar and UID as the Sender read them to send them, each Event Version by its hash. A send that succeeds removes the UID's Unsent Changes only while its Event Rows still are the Sent Event Rows, so that a change made while it ran is sent next time. Goes when the code follows ADR 0019.
_Avoid_: Snapshot (a Snapshot is a state in the Calendar History), sent version

**Sync Scheduler**:
The code that decides when FreeCal syncs: Polls, syncs on window focus, on waking from sleep and after local edits, sync now, and the system's network status. It only decides; the syncs run one Account after another on one thread.
_Avoid_: Sync timer, sync loop, syncer

**Poll**:
The regular sync of every server Account, every 5 minutes while the window is open and every 15 minutes in the background. Any sync of every Account restarts its period.
_Avoid_: Refresh, periodic sync, tick

**Running Sync**:
One sync of a server Account or Calendar while it runs, from its start, which shows Syncing, to its end, which gives its Sync Status how it ended. Ending it is the only way a sync changes a Sync Status; one dropped without an end leaves the Sync Status as before the sync.
_Avoid_: Sync job, sync handle, active sync

**Transaction Origin**:
Where a Journal Transaction's changes came from: a user action, an import, an undo, a First Sync or an Incoming Sync. Goes when the code follows ADR 0018.
_Avoid_: Origin (on its own), source, cause

**First Sync**:
A Calendar's first Incoming Sync, a Full Fetch with no server's marks yet, after the Calendar was stored empty when the user added it. Until the Calendar History is built, the Change Journal doesn't record it.
_Avoid_: Baseline (that word is the squashed schema migration, ADR 0008), initial import

**Dangerous Undo**:
An undo that also undoes later changes, reverts a change from a server, or exceeds the Mass-Change Guard's thresholds, and so asks first. Every other undo is benign and runs at once. Goes when the code follows ADR 0020.
_Avoid_: Risky undo, unsafe undo

**Pinned Zone**:
The Event Time Zone FreeCal gave an Event that arrived with floating times (ADR 0007). It stays until the user changes the Event's time.
_Avoid_: Assumed zone, default zone

**Window Model**:
The frontend module that holds what the window shows (the Accounts and Calendars, the Sync Statuses, the Occurrences in view, the Event whose details are open, today and the view) and keeps it current by re-reading on Signals. It only reads; changes go through the core and come back as Signals, except the window's own view, sidebar and settings.
_Avoid_: Store (on its own: the Local Store), app state, view model

**Window Dialogs**:
The frontend module that holds what is open over the calendar grid, one at a time, and the steps between them: an Event's details and editor, the recurrence scope question, import, notices, settings and About, with the message at the bottom of the window.
_Avoid_: Modal manager, dialog stack, overlays

**Scope Prompt**:
The open question which Occurrences of a Recurring Event a change applies to (this one, this and following, or all of them), with the counts of the Exceptions it would change once read. Window Dialogs holds it; the Scope Dialog shows it.
_Avoid_: Scope question, recurrence dialog

**Window Shown**:
Whether the window has been shown since FreeCal started. Once it has, the shell no longer shows the window by itself when the frontend is slow to start, since the user may have closed it to the tray.
_Avoid_: Shown once, first show
