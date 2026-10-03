# FreeCal v1

Status: ready-for-agent

## Problem Statement

I keep my calendars in several places: Google, one or more CalDAV servers (Nextcloud, Fastmail, iCloud…), and things that live only on my machine. On Linux there's no desktop calendar app that brings them together with the ergonomics of Google Calendar, works properly offline, and that I can trust not to quietly damage my calendars when sync goes wrong. Today I use a browser tab for Google, other tools for the rest, and I have no safety net if a sync tool deletes or mangles Events on a server.

## Solution

FreeCal is a desktop calendar app for one local user, mainly for Linux. It shows Calendars from many Accounts (Local, CalDAV and Google) in one Google-Calendar-like view with day, week and month views, drag-to-create, drag-to-move/resize and keyboard shortcuts. It is offline-first: every synced Calendar is held locally, can be viewed and edited without a network, and syncs two ways when a connection is available. When a local and a remote change can't both be kept, FreeCal asks instead of guessing. Every change it sends to a server is recorded in a Change Journal and can be reversed exactly. A Mass-Change Guard stops runaway syncs before they reach the server. Daily Snapshots provide a last-resort backup. FreeCal runs in the background where a system tray exists, shows reminder notifications, and reports Conflicts and Sync Errors through a tray icon.

The domain vocabulary is defined in the glossary; architecture decisions are in ADRs 0001–0004 (Tauri + Svelte 5 + FullCalendar behind `CalendarGrid`, Google via REST, GPLv3, SQLite).

## User Stories

### Accounts and Calendars

1. As a user, I want a Local Account to exist from the first launch, so that I can start using FreeCal without connecting anything.
2. As a user, I want the Local Account to be impossible to remove, so that my local-only Calendars always have a home.
3. As a user, I want to create, rename, recolour and delete Calendars in the Local Account, so that I can organise Events that live only on this computer.
4. As a user, I want to add a CalDAV Account by entering a server URL, username and password, so that I can use my Nextcloud, Fastmail, iCloud or other CalDAV calendars.
5. As a user, I want FreeCal to discover all Calendars of a CalDAV Account automatically and let me choose which of them to add, so that I don't have to enter each Calendar's URL and only see the ones I want.
6. As a user, I want to add a Google Account by logging in through my browser and choose which of its Calendars to add, so that I never type my Google password into FreeCal and only see the Calendars I want.
7. As a user of the public 1.0, I want a guided "bring your own client" setup for Google, so that I can connect Google even though FreeCal's shared client isn't verified yet.
8. As a developer or tester, I want to use FreeCal's shared (unverified) Google client, so that I can connect Google without creating my own client.
9. As a user, I want to connect several Accounts of each Provider, so that work and personal calendars appear side by side.
10. As a user, I want my passwords and tokens kept in the system keyring, so that they are protected by my desktop login.
11. As a user without a working system keyring, I want a clear error explaining why I can't add a server Account, so that I know what to fix.
12. As a user, I want to remove a Google or CalDAV Account, so that its Calendars disappear from FreeCal and its credentials are deleted from the keyring.
13. As a user, I want each Calendar to have a colour, so that I can tell Events apart at a glance.
14. As a user, I want to show and hide individual Calendars from a sidebar, so that I can focus on what matters right now.
15. As a user, I want Calendars grouped by Account in the sidebar, with each server Account's line letting me fetch the server's current list of Calendars and add or remove them, so that I can see where each Calendar comes from and pick up Calendars created on the server later.
16. As a user, I want Calendars the server doesn't let me change to be Read-only Calendars automatically, so that FreeCal never tries edits that will fail.
17. As a user, I want to mark any Calendar as read-only in FreeCal, so that I can protect a Calendar from accidental edits.
18. As a user before FreeCal 1.0, I want newly connected Google and CalDAV Calendars to start as read-only, so that I can watch sync work before trusting FreeCal to write.
19. As a user, I want to switch a Calendar from read-only to writable myself, so that I decide when FreeCal may change it.

### Viewing

20. As a new user, I want FreeCal to open in month view on first launch, so that I immediately get an overview.
21. As a returning user, I want FreeCal to reopen in the view I last used, so that it matches my habit.
22. As a returning user, I want FreeCal to always open on today, so that I'm not shown some date from weeks ago.
23. As a user, I want day, week and month views, so that I can see my time at different scales.
24. As a user, I want to move to the previous or next period and jump back to today, so that I can browse my calendar.
25. As a user, I want the view to follow today, while the Today button is pressed (from when FreeCal opens or I press Today, until I un-press it or move to another date), so that the view moves to the new day after midnight or sleep, but never pulls me away from a date I chose.
26. As a user, I want overlapping Events laid out side by side in day and week views, so that none of them is hidden.
27. As a user, I want all-day and multi-day Events shown in an all-day area, so that they don't clutter the time grid.
28. As a user, I want to open an Event to see its details, so that I can read its title, time, place, description and Calendar.
29. As a user, I want Occurrences of Recurring Events to appear on every date they fall on, so that my calendar is complete.
30. As a user, I want Read-only Calendars marked with a lock in the sidebar, and an Event's details to say when it is read-only, so that I can find out without the grid getting cluttered.
31. As a user, I want an Event's details to say when it has other attendees ("has guests") and that I should edit it in Google or on my server for now, so that I know why it's read-only and where to go.
32. As a user, when I try to drag or resize a read-only Event, I want a short message explaining why it can't change, so that a refused drag doesn't look like a bug.
33. As a user, I want Events with a Conflict to carry a visible badge, so that I notice they need my decision.

### Creating and editing Events

34. As a user, I want to click an empty slot to create an Event there, so that adding an Event takes one gesture.
35. As a user, I want to drag across a time range to create an Event covering it, so that I can set the duration directly.
36. As a user, I want to choose which writable Calendar a new Event goes into, so that it lands in the right place.
37. As a user, I want to edit an Event's title, time, all-day flag, place, description and Calendar, so that I can keep it accurate.
38. As a user, I want to drag an Event to another time or day, so that rescheduling is quick.
39. As a user, I want to drag an Event's edge to change its duration, so that I don't have to open the editor.
40. As a user, I want to delete an Event, so that I can remove things that won't happen.
41. As a user, I want to set an Event's repeat rule (daily, weekly, monthly, yearly, with an interval and an end), so that I can create Recurring Events.
42. As a user, when I change an Occurrence of a Recurring Event, I want to choose "this event", "this and following events" or "all events", so that I change exactly what I mean.
43. As a user, when I drag an Occurrence, I want the same three-way choice, and I want the drag to be undone if I cancel, so that a stray drag never changes my series.
44. As a user, I want to delete a single Occurrence, so that a cancelled meeting disappears without ending the series.
45. As a user, I want my Exceptions kept when I change unrelated parts of a Recurring Event where possible, so that individual changes aren't lost.
46. As a user, I want edits to Events in Read-only Calendars or with other attendees to be impossible, so that I can't cause damage by accident.
47. As a user, I want FreeCal to preserve every Event detail it doesn't show (attendees, conference links, custom properties) whenever it changes an Event, so that I never lose data elsewhere.
48. As a user, I want to move an Event to a Calendar in another Account, so that I can turn a local draft into a real meeting.
49. As a user, I want to be warned before such a move only if the Event has details that would be lost, so that I'm not nagged needlessly.

### Time zones and locale

50. As a user, I want Events shown in my system time zone by default, so that times match my clock.
51. As a user, I want the Display Time Zone shown in the UI, so that I always know which time zone I'm looking at.
52. As a user, I want new Events to get the Display Time Zone by default, so that I rarely need to think about it.
53. As a user, I want to choose a different time zone for an Event in the editor, so that I can enter "9:00 New York" correctly.
54. As a user, I want Events created in other time zones to appear at the correct local time, so that I never miss a meeting when travelling.
55. As a user, I want first day of the week, 12/24-hour clock and date format to follow my system locale, so that FreeCal looks right without configuration.
56. As a user, I want all UI text to come from a translation layer, so that FreeCal can be translated later (v1 ships English only).

### Keyboard

57. As a keyboard user, I want Google-style shortcuts to switch views (day, week, month), so that I don't need the mouse.
58. As a keyboard user, I want shortcuts for previous, next and today, so that I can browse quickly.
59. As a keyboard user, I want a shortcut to create an Event, so that adding one is instant.
60. As a keyboard user, I want a shortcut to sync now, so that I can force a refresh.

### Sync

61. As a user, I want every synced Calendar stored locally, so that FreeCal opens instantly and works offline.
62. As a user, I want to create, edit, move and delete Events while offline, so that I'm never blocked by the network.
63. As a user, I want my offline changes sent automatically when the connection returns, so that I don't have to remember them.
64. As a user, I want changes made elsewhere (phone, web) to appear in FreeCal within about two minutes while the window is open, so that I see up-to-date data.
65. As a user, I want FreeCal to sync every 15 minutes while running in the background, so that reminders and the tray stay accurate without wasting resources.
66. As a user, I want FreeCal to sync when I focus its window, when my computer wakes from sleep and right after I change something, so that it's fresh when I need it.
67. As a user, I want a manual "sync now", so that I can refresh on demand.
68. As a user, I want the window to show when FreeCal is syncing and when the last sync finished, so that I know whether what I see is current.
69. As a user, I want each Account and Calendar in the sidebar to show whether it is syncing, offline or has a Sync Error, so that I can see which one is affected.
70. As a user, I want changes pulled from a server to appear in the open view without a manual refresh, so that the window is never out of date.
71. As a user, I want new Conflicts and Sync Errors to appear in the window as soon as they happen, so that I don't have to reopen anything to see them.
72. As a user, I want to see which of my changes haven't reached the server yet (a count per Account or Calendar, a subtle mark on the Event and the reason in its details), so that I know whether my edits made offline have gone out.
73. As a user, when an Event was changed both in FreeCal and on the server, I want to see both versions and choose one, so that no change is lost silently.
74. As a user, when I edited an Event that was deleted on the server, I want to choose "restore with my changes" or "accept the delete", so that I decide.
75. As a user, when I deleted an Event that was edited on the server, I want to choose "delete anyway" or "keep the server version", so that I decide.
76. As a user, when a change to a whole Recurring Event collides with a remote change to one Occurrence, I want a Conflict on the whole Recurring Event showing both versions, so that I can resolve it as one decision.
77. As a user, I want a list of all open Conflicts, so that I can work through them.
78. As a user, I want Sync Errors (expired login, server refusing a request, a paused Calendar) reported per Account or Calendar with a clear message, so that I know what's wrong and where.
79. As a user, I want being offline not to count as an error, so that I'm not alarmed by normal situations.
80. As a user with an expired Google login, I want to be asked to log in again, so that sync resumes.

### Safety

81. As a user, I want every change FreeCal sends to a server recorded with the previous server version, so that any change FreeCal made can be reversed exactly.
82. As a user, I want to undo a change FreeCal made to a server Event, so that a mistake or bug can be repaired with the original ID and all details intact.
83. As a user, I want to undo all changes FreeCal made to a Calendar since a given time, so that I can recover from a bad sync session.
84. As a user, I want Change Journal entries kept for 90 days, so that I have time to notice problems.
85. As a user, if one sync would delete or change more than 10 Events or more than 5% of a Calendar, I want that Calendar paused and myself asked first, so that a bug can't wipe a Calendar.
86. As a user, I want a paused Calendar to show as a Sync Error with an explanation ("Paused: 120 Events would be deleted. Review?"), so that I understand what happened.
87. As a user, I want to approve or reject the paused changes, so that sync can continue the way I choose.
88. As a user, I want a compressed Snapshot of each changed Calendar taken daily, with the last 14 kept, so that I have a last-resort backup.
89. As a user, I want Snapshots to skip Calendars that haven't changed, so that disk usage stays small.
90. As a user, I want to restore a Calendar from a Snapshot, so that I can recover even if the Change Journal can't help.

### Import and export

91. As a user, I want to export any Calendar to an `.ics` file, so that I have a portable copy.
92. As a user, I want to export everything to a `.zip` with one `.ics` per Calendar organised by Account, so that I can back up or migrate in one step.
93. As a user, I want exports to keep Google-only details, so that a Google Event can be restored completely.
94. As a user, I want to import an `.ics` file into any writable Calendar, so that I can bring Events in from elsewhere.
95. As a user, I want to import a `.zip` of `.ics` files (e.g. a Google Takeout export), so that I can migrate several Calendars at once.
96. As a user, I want an import preview showing how many Events are new and how many already exist, so that I know what will happen.
97. As a user, when some imported Events already exist (same UID), I want to choose once per import whether to update them, skip them or import them as copies, so that restores don't create duplicates.
98. As a user, I want the import preview's confirmation to count as my approval, so that the Mass-Change Guard doesn't stop an import I just confirmed.
99. As a user, I want to undo an entire import in one step, so that a wrong import is easy to reverse.
100. As a user, I want import to be unavailable for Read-only Calendars, so that I can't write where FreeCal mustn't.

### Desktop integration

101. As a user, I want desktop notifications for the reminders stored on my Events, so that I don't miss meetings.
102. As a user, I want to snooze or dismiss a reminder notification, so that I can handle it on my terms.
103. As a user with a system tray, I want FreeCal to keep running in the background when I close the window (on by default), so that sync and reminders continue.
104. As a user without a system tray, I want closing the window to quit FreeCal, so that it never runs invisibly with no way back.
105. As a user who enables a tray while FreeCal runs (e.g. the GNOME AppIndicator extension), I want FreeCal to notice the next time I close the window, so that I don't need to restart it.
106. As a user, I want to turn background running off, so that closing always quits.
107. As a user, I want an option to start FreeCal at login (off by default), so that reminders work from the moment I log in.
108. As a KDE Plasma user, I want the tray icon to work natively, so that FreeCal fits my desktop.
109. As a user, I want one tray icon showing the most important state (Conflict, then Sync Error, then Offline, then Syncing, then Online and synced), so that I can see FreeCal's health at a glance.
110. As a user, I want the tray menu to offer Open FreeCal, Sync now and Quit, so that I can control FreeCal without its window.
111. As a user, I want the tray menu to list current Conflicts and Sync Errors, so that I can go straight to them.
112. As a user with FreeCal in the background, I want one desktop notification per new Conflict or Sync Error, so that problems don't go unnoticed for days.

### Distribution

113. As a Debian or Ubuntu user, I want a `.deb` package, so that FreeCal installs natively.
114. As a Fedora or openSUSE user, I want an `.rpm` package, so that FreeCal installs natively.
115. As a user of any distribution, I want an AppImage, so that I can run FreeCal without installing it.
116. As a future Flathub user, I want FreeCal built to use desktop portals for autostart, keyring and file dialogs, so that a sandboxed version works without loss of function.

## Implementation Decisions

- **Architecture** (ADR 0001): a Rust core library that holds all domain logic, a Tauri shell exposing the core to the frontend as commands, and a Svelte 5 frontend (runes only, plain Vite, no SvelteKit). Vite is used only for development and the build; the shipped binary embeds the compiled frontend.
- **Core application interface**: the core exposes one application-level interface, which is also the main test seam. Its operations cover:
  - Accounts: add, remove, re-authenticate
  - Calendars: list, create, rename, recolour, delete, set read-only; for a server Account, list the server's current Calendars, add Available Calendars and remove Calendars from FreeCal
  - Occurrences: get those in a date range, already converted to the Display Time Zone, each with whether it has unsent local changes and since when
  - Events: create, edit, move or delete, including the recurrence scope (this / this and following / all) and moves across Accounts
  - Sync: sync now (per Account, or for all); get the current sync status of every Account and Calendar
  - Problems: list and resolve Conflicts, list Sync Errors, approve or reject a paused Calendar
  - Change Journal: list entries, undo one entry, undo everything since a given time, undo an import
  - Import/export: preview an import, run it, export
  - Snapshots: take, list, restore
  - App state and settings: last-used view, background mode, autostart
  The frontend talks only to this interface.
- **Signals (core → frontend)**: besides answering requests, the core pushes Signals: messages saying that something changed. The Tauri shell forwards them to the frontend as Tauri events. In code and docs they are always called Signals, never "events" (that word means calendar Events) or "notifications" (those are desktop notifications). There are two kinds:
  - A **status Signal** carries the new state itself, because it is small and the state is the whole message.
  - A **change Signal** only names what changed. The frontend re-reads through the core interface, so the Local Store stays the single source of truth.

  The v1 Signals are:
  - **Sync status changed** (status): the new Sync Status of an Account or Calendar, with the time and outcome of the last finished sync and the number of local changes not yet sent.
  - **Occurrences changed** (change): the Events of one or more Calendars changed, for any reason (remote pull, local edit, import, undo, Snapshot restore, Conflict resolution). It carries the affected Calendar IDs.
  - **Calendars changed** (change): Accounts or Calendars were added, removed, renamed or recoloured, or their writability changed (for example the user adding a server Calendar, or a server permission change).
  - **Problems changed** (change): a Conflict or Sync Error was raised or resolved. The tray and the notifications query the core for details; they run in the shell next to the core.
  - **Display Time Zone changed** (status): the system time zone changed, for example after travelling. It carries the new zone. The frontend redraws the visible range and the time zone label. An open editor keeps its draft, including the draft's own time zone.
  - **Date changed** (status): the local date changed, at midnight or on waking up on a new day. It carries the new date. The frontend moves the today highlight, and jumps to the new today if it is tracking today (see View state). The core sends it because it already detects waking from sleep and owns the clock that tests control.

  There are no Signals for reminders, which are desktop notifications only, or for the progress of long operations (import, export, Snapshot restore). While such a command runs, the window shows a modal "Working…" dialog that blocks it until the command finishes, so edits can't interleave with an import or restore. A non-blocking version can come later if large imports turn out to be slow.

  Account-level states (Offline, an expired login, a locked keyring) and Calendar-level states (paused by the Mass-Change Guard, writes refused) each live where they happen. "Sync status changed" is sent separately for Accounts and for Calendars, and the sidebar shows an Account-level state once on the Account row, not repeated on every Calendar. The Local Account and its Calendars have no Sync Status and never send "Sync status changed".

  Signals are batched: an incremental sync sends at most one "Occurrences changed" per Calendar, when that Calendar's changes are committed. A Calendar's first sync is different: it commits page by page and sends one "Occurrences changed" per committed page, so Events appear as they load. Its "Sync status changed" marks it as a first sync. Signals are fire-and-forget: with no window open, they are dropped. When the frontend starts, or the window is re-created after running in the background, it subscribes first and then reads the current state, so nothing falls in the gap. The tray and the background desktop notifications use the same Signals inside the shell, so the window and the tray never disagree.
- **Provider interface** (internal, not a test seam): one implementation per Provider: Local, CalDAV and Google (REST, ADR 0002). It covers discovering Calendars, incremental fetching (CalDAV `sync-collection`/ETags, Google sync tokens with 410 handling), and conditional create, update and delete. The interface must allow Microsoft Graph and ICS feeds to be added later without changing the sync engine.
- **Local Store** (ADR 0004): one SQLite database in the XDG data directory. For each Event it holds the original server data exactly as received, next to the fields FreeCal uses, so that unknown fields are preserved byte-for-byte on writes. It also holds the pending-change queue, the Change Journal, Conflicts, Sync Errors, settings and app state. Snapshots are compressed `.ics` files in the same directory.
- **Sync engine**:
  - It applies local changes to the Local Store immediately and queues them.
  - It pushes queued changes with conditional requests and pulls remote changes incrementally.
  - It detects Conflicts (edit/edit, edit/remote delete, delete/remote edit, and a series edit vs a remote Occurrence edit, which becomes a Conflict on the whole Recurring Event) and holds them for the user to resolve.
  - Before a sync is applied in either direction, it checks the Mass-Change Guard: more than 10 Events or more than 5% of a Calendar pauses that Calendar as a Sync Error. Imports skip the guard because the import preview is the approval.
  - Being offline is a Sync Status, not a Sync Error. It is decided per Account: a network-level failure (DNS, connection refused, timeout) makes that Account Offline, while an error response from the server is a Sync Error.
- **Overall status** (shown by the tray icon and the window's status indicator): Conflict if any Event is in Conflict; otherwise Sync Error if any Account or Calendar has one; otherwise Offline if there is at least one server Account and **every server Account** is Offline; otherwise Syncing if any is syncing; otherwise Online and synced. If only some server Accounts are Offline, only the sidebar marks them. "Syncing" is shown only when a sync has run for more than about 1 second, or when the user started it (sync now, the shortcut, the tray menu), so routine polls don't make the display flicker. The Signals are still sent straight away; only the display waits.
- **Sync timing**: every 2 minutes while the window is open, every 15 minutes in the background, plus syncs on window focus, on wake from sleep and after local edits. Hard-coded for v1 and may become settings later. FreeCal also listens to the system's network status through the NetworkMonitor portal. When the network comes back, it syncs right away, which sends queued offline changes. While there is no network at all, it skips polls and marks every server Account Offline without trying. Otherwise, whether an Account is Offline is still decided by its actual requests.
- **Change Journal**: before any change is sent to a server, the Event's previous server version is stored together with the new one. Undo writes the previous version back under the same identity. Imports are grouped as one Journal unit, including imports into Local Calendars, so that any import can be undone in one step. Entries are kept for 90 days.
- **Snapshots**: daily, compressed, skipping Calendars unchanged since the last Snapshot, 14 kept. They include the Local Account's Calendars.
- **Recurrence**: the core expands Recurring Events into Occurrences for the requested range. The frontend never interprets repeat rules. Editing supports this / this and following / all, and Exceptions are preserved where possible.
- **Attendees**: no attendee features. Any Event with attendees other than the user is read-only, and its details say "has guests" (edit it in Google or on the server for now). Attendee data is never shown and always preserved.
- **Choosing server Calendars**: when a CalDAV or Google Account is added, FreeCal lists the Calendars it discovered as Available Calendars and the user chooses which to add. Calendars that appear on the server later stay Available Calendars until the user adds them. The Account's line in the sidebar offers "Manage Calendars…", which fetches the server's current list and lets the user add Available Calendars or remove Calendars. Removing a server Calendar from FreeCal deletes only its local copy, never touches the server, and makes it an Available Calendar again.
- **Writability**: a Calendar is read-only if the server says so or if the user marks it read-only in FreeCal. Before 1.0, newly added Google and CalDAV Calendars default to read-only.
- **Moves across Accounts**: implemented as create in the target, then delete in the source, both recorded in the Change Journal. The warning is shown only when the Event has Provider-specific details that the target can't hold.
- **Import/export**:
  - Import accepts `.ics` and `.zip` of `.ics` files; CSV is not supported.
  - Export produces a single `.ics`, or for everything a `.zip` with one `.ics` per Calendar organised by Account.
  - Google-only fields are written as `X-` properties and read back on import.
  - Existing UIDs trigger one update / skip / copy choice per import.
  - Importing into Read-only Calendars is not offered.
- **Time zones and locale**:
  - The Display Time Zone is the system time zone and is shown in the UI. New Events default to it, and each Event can have its own time zone.
  - Locale defaults (first day of week, 12/24-hour clock, date format) are read from the system.
  - All UI strings go through an i18n layer; v1 ships English only.
  - The settings page that adds overrides is a later milestone.
- **Frontend**:
  - FullCalendar v7 (MIT standard edition) is wrapped in a `CalendarGrid` component. It only displays the Occurrences it receives and reports user actions (range selected, Event clicked, moved, resized). FreeCal owns all state, including which Event is selected.
  - When an Occurrence is dropped, the frontend asks for the recurrence scope and either commits the move or reverts it.
  - Keyboard shortcuts are handled by FreeCal, not FullCalendar.
  - A status indicator in the window shows the overall status (the same as the tray icon), plus the time of the last finished sync. The sidebar shows each Account's and Calendar's Sync Status when it isn't Synced. A Calendar on its first sync shows "Loading for the first time…" instead of looking empty. The sidebar also shows the number of unsent changes when it isn't zero, and a lock on Read-only Calendars.
  - **Event chips stay sparse**: the Conflict badge is the only badge on a chip. An Occurrence with unsent changes gets a dashed outline, not a paler colour, because a faded look already means past or tentative in Google-like calendars and a colour change is lost on light Calendar colours. The dashed outline appears once the change has waited more than a few seconds, or straight away when its Account is Offline, so that it doesn't flash on every edit. Read-only and "has guests" are not shown on chips. The grid simply offers no move or resize for those Events.
  - **Event details** show "Not yet synced: <reason>" for unsent changes, and say when the Event is read-only, giving the reason: a Read-only Calendar, or "has guests" with the hint to edit it in Google or on the server.
  - Trying to drag or resize a read-only Event shows a short message saying why it can't change.
  - On an "Occurrences changed" Signal for a shown Calendar, the frontend fetches the visible range again and keeps the current selection. An open Event editor is never overwritten: the user's draft stays, and saving it goes through normal Conflict detection.
- **View state**: month view on first launch, then the last-used view, always opening on today. There is no setting for the first-launch view.
- **Tracking today**: a mode, like "follow my position" in a map app. The Today button is a toggle, and its pressed state *is* the mode.
  - It is turned on when FreeCal opens and when the user presses Today, which also jumps to today.
  - It is turned off when the user un-presses Today (the view stays where it is) or changes the date: prev/next, the date picker, or clicking a day number. This holds even if the new period also contains today.
  - It stays on when the user switches between day, week and month, scrolls the time grid, selects, creates or edits Events, or opens dialogs.
  - When a "Date changed" Signal arrives, the view jumps to the period containing the new today only while tracking. Otherwise only the today highlight moves.
- **Desktop interfaces** (fakeable in tests):
  - **Keyring**: Secret Service via libsecret; report an error if it's unavailable.
  - **Notifications**: reminders with snooze and dismiss; one notification per new Conflict or Sync Error while the window is closed.
  - **Tray**: StatusNotifierItem through Tauri. The menu is the only interaction (Linux gives no click events). One icon showing the overall status (see Sync engine). The menu offers Open FreeCal, Sync now, problem lines and Quit.
  - **Tray detection**: a D-Bus check for a StatusNotifierWatcher with a registered host, made at every window close.
  - **Autostart**: through the Background portal where available; off by default.
  - **Network status**: the NetworkMonitor portal (it also works outside Flatpak).
- **Google OAuth**: browser-based login with a loopback redirect and PKCE.
  - The shared FreeCal client is set to "In production", unverified, for the developer and testers. Unverified clients have a lifetime cap of 100 users.
  - The public 1.0 offers "bring your own client" with a guided setup.
  - Verification comes after 1.0.
- **Packaging**: before 1.0, `.deb`, `.rpm` and AppImage built by Tauri. All OS integration goes through portal-friendly APIs so that the Flathub release at 1.0 needs no code changes.
- **License**: GPLv3 (ADR 0003). All dependencies must be GPLv3-compatible.

## Testing Decisions

- **What makes a good test**: it checks behaviour visible through a seam (what the user or an external server would see), never internal structure. Tests must keep passing through refactors that don't change behaviour.
- **Seam 1: the core application interface** (the main seam). Almost all behaviour is tested here:
  - CalDAV sync against a real **Radicale** server started per test run.
  - Google sync against a **fake Google HTTP server** that mimics the Calendar REST API (sync tokens, ETags, 410 "token expired", permissions). The real Google Provider code is exercised.
  - A real SQLite database in a temporary directory, never mocked.
  - A controllable clock for polling, reminders, Journal retention and Snapshot timing.
  - Recording fakes for the keyring, notifications and tray, a controllable fake network status and system time zone, and a recording Signal subscriber.
  - Covered here: offline queueing and replay, every Conflict scenario, the Mass-Change Guard thresholds and approval, Change Journal undo (single entry, since a given time, whole import), Snapshot creation, skipping and restore, read-only enforcement (server-reported, user-set, the pre-1.0 default, "has guests"), byte-preservation of unknown fields across edits, recurrence scopes and Exceptions, moves across Accounts and their warning condition, import update / skip / copy, export round-trips including Google `X-` properties, Display Time Zone conversion, tray state priority, the Signals each operation and sync sends (one "Occurrences changed" per Calendar per incremental sync, one per page on a first sync, "Date changed" at midnight and on waking up on a new day, "Display Time Zone changed"), the overall status including the case with no server Accounts, and the sync triggered and the polls skipped on network changes.
  - Conflict and Mass-Change Guard scenarios are written test-first.
- **Seam 2: the frontend against a fake core interface**. Thin Svelte component tests (Vitest) for `CalendarGrid` and the Event editor. They check that user actions (click and drag to create, drag to move or resize, the recurrence scope prompt including cancel-reverts, keyboard shortcuts, disabled editing for read-only and "has guests" Events, including the message on a refused drag) produce the right core calls. The fake core can also emit Signals, so tests check that the view re-fetches on "Occurrences changed", that the status indicator and sidebar markers follow "Sync status changed", that an open editor's draft survives a re-fetch, that the dashed outline for unsent changes follows its delay rule, and that tracking today starts, ends and reacts to "Date changed" as specified.
- **The Provider interface is not a test seam.** Tests never replace a Provider.
- **No end-to-end tests of the real Tauri window in v1.** A manual release checklist is run against a real Google test account before each release; Nextcloud tests come later.
- **Prior art**: none. This is a new codebase, so these tests set the pattern.

## Out of Scope

- Attendee features: showing attendees, RSVP, inviting, and editing Events that have other attendees.
- ICS feed subscriptions, Microsoft Graph / Outlook, Exchange EWS, GNOME Online Accounts, vdir folders.
- The settings page (overrides for locale, time zone, sync timings, retention values, default view) and the secondary time-zone column.
- Field-level automatic merge of Conflicts (planned later as an opt-in setting).
- Moving Events with the keyboard, natural-language quick-add, search, editing reminders, upcoming Events in the tray menu.
- Translations other than English.
- Tasks (VTODO / Google Tasks) and contacts (CardDAV / Google People).
- Google OAuth verification of the shared client (after 1.0).
- The Flathub release (at 1.0), Snap, AUR packaging by the project, and macOS and Windows as targets.
- FullCalendar Premium features (timeline, print).
- Automatic backups beyond Snapshots; system backups are expected to cover the data directory.

## Further Notes

- Build order: the **Local** vertical slice first (grid, editor, Local Store, Change Journal, Snapshots, import/export), then **CalDAV** (developed and tested against Radicale), then **Google**.
- Expected disk usage is about 30 MB for a typical user (about 3,000 Events) and about 150 MB for a heavy user (about 15,000 Events), dominated by Snapshots.
- The Flathub manifest will need permission to talk to `org.kde.StatusNotifierWatcher` over D-Bus.
- Once frontend code exists, add a rule to `CLAUDE.md` that Svelte code uses Svelte 5 runes only, because AI-generated code often slips into Svelte 4 syntax.
- The pre-1.0 read-only default and the shared unverified Google client are temporary measures with explicit end points (1.0 and verification).
