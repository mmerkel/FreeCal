# 18: Offline editing and replay

**What to build:** The user can create, edit, move and delete Events in server Calendars while offline. Queued changes are sent automatically when the connection returns. The user can see which changes haven't reached the server yet, and is warned when quitting with changes not yet sent. Spec: US-131 and "Quitting" under Frontend.

**Blocked by:** 13, 16, 33, 91, 92, 124, 127, 128

**Status:** ready-for-agent

**Effort:** M

- [x] Edits while the Account is Offline succeed locally and are queued
- [x] Queued changes are replayed when the network returns, in order, through the same conditional requests and Change Journal recording
- [x] Occurrences returned by the core say whether they have unsent changes and since when
- [x] The sidebar shows the unsent count per Account and Calendar when not zero
- [x] An Occurrence with unsent changes gets a dashed outline once the change has waited more than a few seconds, or immediately when its Account is Offline (component test)
- [x] Event details show "Not yet synced: <reason>"
- [ ] Quitting with unsent changes (Quit in the main menu, Quit in the tray menu, or closing the window when that quits FreeCal) first tries to send them for a few seconds under the "Working…" dialog ("Sending 2 changes…"), then quits if that worked
- [ ] If changes are still unsent, a modal dialog says how many, lists the Calendars with the reason for each (Offline, Conflict, waiting for approval, error) and says they'll be sent at the next start, or that they need attention for a Conflict or a paused Calendar. "Cancel" has initial focus; "Quit anyway" quits
- [x] Logout and system shutdown never wait for this and never show the dialog; the queue stays in the Local Store
- [x] Core tests for the send-before-quit attempt with a controllable clock; component tests against the fake core for the dialog, Cancel and Quit anyway

## Comments
- 2026-10-24: Start with a grilling on whether ADR 0010's structure is right for recording changes not yet sent: a pending mark on objects, rows without an object (not created yet) and objects without rows (deletes not sent), instead of the "queue" this ticket's wording assumes. Also: order of replay, and what the user sees as "not yet sent".
- 2026-10-25 (grilling, ADR 0011): blocked by 91. The structure asked about above is decided: Unsent Changes are Combined Event Keys in `unsent_change` (Calendar, UID, recurrence ID, since), not a pending mark on Server Objects; the three cases are read off Event Rows and Server Objects per Combined Event Key. Order of replay and what the user sees remain for this ticket's grilling.
- 2026-10-25: Re-sized from XL to L. Ticket 91 brings the record of unsent changes with their `since`, and ticket 17 the sender that replay reuses; what's left is the replay trigger and its order, the counts and outlines, the reason shown, and the quit flow.
- 2026-11-02: Ticket 91 was split; `unsent_change` is now ticket 92, so this ticket is blocked by both.
- 2026-11-07: Re-sized from L to M. Unsent Changes persist in the Local Store (ticket 92), so replay is ticket 17's Sender run when the network returns (ticket 16 watches it), and ticket 15 brings the unsent counts in "Sync status changed". What's left: the per-Occurrence unsent mark and outline, the reason shown, the quit flow, and the grilling on order and what the user sees.
- 2026-11-08 (from 124): a change is sent at once only while its Account isn't Offline; otherwise it waits for a Poll or the network's return. The reasons for "Not yet synced: <reason>" are decided in ticket 124's comment: an Unsent Change's stored Send Failure (`unsent_change.send_failure`, `SendFailure` in `core/src/sync/mod.rs`), shown after the Account's or Calendar's own reason, which takes precedence. This ticket adds the interface call that reads it.
- 2026-11-09 (/implement): all but two boxes done; the grilling this ticket's comments asked for wasn't held, so the choices below were made in its place, for review.
  - **Still open.** Box 7: Quit in the tray menu comes with ticket 13's tray, which should call the window's quit flow (`WindowDialogs::quit`, `src/lib/windowDialogs.svelte.ts`) as closing the window does, through the "quit asked" event. Box 8: the dialog lists Offline and Sync Errors, but "needs attention for a Conflict or a paused Calendar" waits on tickets 20/21 (Conflicts) and 22 (paused Calendars); it always says they'll be sent at the next start. It also doesn't show a Calendar's stored Send Failures: a Calendar with no Account or Calendar reason shows "not sent yet", or "still sending" while it syncs.
  - **Box 4** was done by ticket 15, which decided the count shows on Calendar rows only; the Account's row shows none.
  - **Replay order (decision).** Replay is the Sender's run at the network's return or the next Poll, in the order the UIDs' first Unsent Changes were made (`Store::unsent_uids`). Nothing new was needed; tested in `core/tests/offline_editing.rs`.
  - **Which Occurrences show an Unsent Change (decision).** An Event Row's own Unsent Change, by its Combined Event Key compared by Occurrence: an Occurrence an Exception changed shows the Exception's, every other Occurrence of a Recurring Event the Recurring Event's. So changing one Occurrence outlines it alone, while deleting one (an EXDATE on the Recurring Event) outlines the whole series. A stored `since` that can't be read counts as the Unix epoch, so the change still shows. `Occurrence.unsent_since`, `Event.unsent_change` (`UnsentChange` with `since` and the `send_failure`), `core/src/event/mod.rs`.
  - **"Not yet synced: <reason>" (as ticket 124 decided).** The Account's reason first (Offline, Sync Error), then the Calendar's (Sync Error, First Sync unfinished), then the Event's own stored Send Failure; with none, just "Not yet synced". The user's Read-only mark is no reason, as it holds nothing back (spec: "Writability"). `notYetSyncedText`, `src/lib/unsent.ts`.
  - **The outline (decision).** It appears 5 seconds after the change, or at once while its Account is Offline (`OUTLINE_DELAY_MS`). A sync of a Calendar that had Unsent Changes sends "Occurrences changed" for it, as a send changes no Event Row but changes what the window shows (`Core::sync_calendar`).
  - **Sending before quit (decision).** `Core::send_before_quit` asks for a sync of each Calendar with Unsent Changes, as a change does, and waits until those ran or 5 seconds passed on the core's clock; with no network it returns at once. It returns the Sync Statuses, from which the window lists what still waits. To wait, the Sync Scheduler counts the batches of syncs it hands out and those that finished (`next_batch`, `batch_run`, `finish_batch`). A sync still running at 5 seconds goes on until FreeCal exits; what it doesn't finish is sent at the next start.
  - **Closing the window (decision).** With Unsent Changes waiting, the shell keeps the window open and asks the window to quit as Quit does (`freecal://quit-asked`, `CoreApi.onQuitAsked`). With none, closing quits at once, so that a window whose page is broken can still close.
  - **Logout and shutdown (decision).** The shell watches the Inhibit portal's session monitor (logout) and logind's `PrepareForShutdown` (shutdown); while the session ends, closing quits at once, and a cancelled end clears that (`watch_session_end`, `src-tauri/src/desktop_watch.rs`). Tested against fake services on a private bus; not tried on a real logout. KDE's portal may not offer the session monitor, in which case a logout's close would show the dialog.
  - **Untrusted content:** `fixtures/hostile-event.json` gains `notYetSynced` (a server message in a Send Failure) and `quitCalendar` (a Calendar name in the quit dialog).
  - **New terms**, for approval:
    - **Send Failure** (new term, ticket 124's): now part of the interface (`SendFailure`), shown in Event details.
    - **Batch** (new term): one taking of the syncs due by the Sync Scheduler, which `send_before_quit` waits to finish.
    - **Session End** (new term): a logout or shutdown, during which closing quits without trying to send (`SessionEnding`, `watch_session_end`).
    - **Quit Asked** (new term): the shell asking the window to quit as Quit does, when closing it with Unsent Changes waiting (`QUIT_ASKED_EVENT`, `onQuitAsked`).
    - Code names: `KeptUnsentChange` (an Unsent Change as read from the Local Store, untrusted), `UnsentChangeRow` (renamed test-only row), `UnsentAtQuit`, `QuitDialog`, `heldBackBy`, `notYetSyncedText`, `outlineDue`, `offlineAccounts`, `unsent_since`/`unsentSince`.
- 2026-11-09 (review with the user): Batch, Session End, Quit Asked and Send Failure approved as code names, not added to `GLOSSARY.md`.
