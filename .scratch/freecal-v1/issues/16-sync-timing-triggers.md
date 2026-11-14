# 16: Sync timing and triggers

**What to build:** Changes made elsewhere appear within about five minutes while the window is open, and within 15 minutes in the background. FreeCal also syncs on window focus, on wake from sleep, right after local changes, and when the network comes back. The user can sync now from a button, a keyboard shortcut or the tray menu.

**Blocked by:** 13, 15

**Status:** done

**Effort:** M

- [x] Polls every 5 minutes with the window open and every 15 minutes in the background (controllable clock)
- [x] Syncs on window focus, on wake from sleep and after local edits
- [x] Sync now: per Account or for all; a window control, a Google-style shortcut and a tray menu item
- [x] The NetworkMonitor portal is watched: when the network returns, a sync runs right away; with no network at all, polls are skipped and every server Account is marked Offline without trying (tested with a controllable fake network status)

## Comments
- 2026-10-18 (from 82): the on-demand sync is `Core::sync_account(AccountId)`, which blocks until each Calendar is synced; it leaves out a Calendar on its first sync.
- 2026-11-07 (from 15): `Core::sync_account` now takes `StartedBy`: sync now passes `StartedBy::User` so that Syncing shows at once, polls and triggers `StartedBy::FreeCal`. A Calendar that is syncing is left out of another sync.
- 2026-11-07 (implementation): what's left. Both open boxes wait on ticket 13. Box 1: the core polls every 5 minutes (2 when this was written) with the window open and every 15 in the background, tested with the controllable clock (`core/tests/sync_timing.rs`), but nothing tells it yet that the window is hidden; ticket 13 calls `Core::set_window_open(false)` when it hides the window and `true` when it shows it again. Box 3: sync now is in the window (main menu "Sync now", the `R` shortcut, which is Google Calendar's "Refresh", and "Sync now" in each server Account's menu); the tray item, `Core::sync_now(None)`, comes with ticket 13's tray menu.
- 2026-11-07 (implementation): decisions.
  - The Sync Scheduler (`SyncScheduler`, `core/src/sync_scheduler.rs`) only decides when to sync. One shell thread runs every scheduled sync, one Account after another (`Core::run_scheduled_syncs`), and waits for the next request or 10 seconds (`Core::wait_for_sync_request`). Syncs asked for meanwhile merge into one, and the user wins over FreeCal for the Syncing Delay. Trade-off: a sync now asked for during a running poll waits for the poll to end, so its Syncing shows only once it starts.
  - Sync now resolves at once; the Sync Status tells how it goes.
  - The first poll runs at launch. Any sync of every Account (a poll, focus, wake, the network's return, sync now for all) restarts the poll period, counted from when it started.
  - A window focus within a minute of the last sync of every Account syncs nothing, so the focus right after launch and switching between windows don't sync again and again. Not in the spec; trade-off: a change made elsewhere in that minute shows only at the next poll.
  - A local edit in a server Account's Calendar syncs that Account only. Until tickets 17 and 18 send Unsent Changes, that sync only brings in changes.
  - With no network: when it goes away, every server Account not syncing now is Offline at once, with a Sync Record at that time. Polls, triggers and sync now are then dropped, as the network's return syncs every Account anyway. A batch of syncs running when it goes away stops before the next Account. Until the portal first answers, the network counts as available, so a launch with no network finds out from the first poll's requests.
  - Waking from sleep comes from logind's `PrepareForSleep`, the network status from the NetworkMonitor portal's `GetAvailable`; a local-only network counts as a network (`src-tauri/src/desktop_watch.rs`). Without either, nothing is reported.
- 2026-11-07 (implementation): new terms, for approval.
  - Sync Scheduler (new term): decides when FreeCal syncs: polls, triggers, sync now and the network status (`SyncScheduler`).
  - Poll (new term): the regular sync of every server Account, every 2 or 15 minutes.
  - Sync Trigger (new term): something the desktop notices that syncs every server Account, a window focus or waking from sleep (`SyncTrigger`).
  - Sync Requests (new term): the syncs asked for and not started yet, each with who asked (`SyncRequests`).
  - Desktop Watch (new term): the shell's threads that watch the desktop for the network status and waking from sleep (`desktop_watch`).
- 2026-11-07 (review with the user): Sync Scheduler, Poll, Sync Trigger, Sync Requests and Desktop Watch approved as code names; Sync Scheduler and Poll also added to the Internals section of `GLOSSARY.md`.
- 2026-11-07 (user): the poll with the window open is every 5 minutes, not the spec's 2 (`POLL_WITH_WINDOW`, `core/src/sync_scheduler.rs`). The background poll stays at 15 minutes. Settings for these times are out of scope for v1 (spec, "Out of Scope").
- 2026-11-14 (implementation, finishing after ticket 13).
  - Box 1: ticket 13 now tells the core when the window hides and shows (`Core::set_window_open`, called by `close_main_window` and `bring_to_front`, `src-tauri/src/main.rs`), so the background poll of 15 minutes applies while the window is hidden. The timing itself was already tested with the controllable clock (`core/tests/sync_timing.rs`).
  - Box 3: the tray menu now reads Open FreeCal, Sync now, Quit (US-110), and Sync now there is the window's Sync now for all, `Core::sync_now(None)` (`TrayChoice::SyncNow`, `src-tauri/src/tray.rs`; `on_tray_choice`, `src-tauri/src/main.rs`). After a failed start, with no Core, it does nothing.
  - Like the rest of the tray menu, it has no fake or test (ticket 13's comment of 2026-11-14). Checked in the debug build on Xvfb, on the user's bus: the menu's layout over D-Bus lists the three items in that order, and a click on Sync now synced a CalDAV Account at that second (its `last_sync_at`, after the launch poll's).
