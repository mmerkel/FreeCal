# 13: Background running, tray basics and autostart

**What to build:** Where a system tray exists, closing the window keeps FreeCal running in the background (on by default); without a tray, closing quits. A tray enabled while FreeCal runs is noticed at the next close. The user can turn background running off and turn start-at-login on. The tray menu offers Open FreeCal and Quit. Both options live in the Background section of the Settings dialog (ticket 33). Spec: US-103–US-108 and US-129.

**Blocked by:** 01, 33

**Status:** ready-for-agent

**Effort:** M

- [ ] At every window close, a D-Bus check for a StatusNotifierWatcher with a registered host decides whether to keep running or quit
- [ ] Background running is on by default and can be turned off, so closing always quits
- [ ] Start at login is off by default and uses the Background portal where available
- [ ] A StatusNotifierItem tray icon (works natively on KDE Plasma) with a menu: Open FreeCal, Quit
- [ ] When the window is re-created, the frontend subscribes to Signals first and then reads current state
- [ ] Tray detection, tray and autostart are behind fakeable desktop interfaces with tests
- [ ] The Background section of Settings has "Keep running in the background" and "Start at login" switches. The background switch is always enabled, with the hint "Needs a system tray; without one, closing quits", because a tray is only checked for at window close
- [ ] Turning on start at login goes through the Background portal at the moment the switch is flipped (the portal may ask the user); if it is refused, the switch goes back off with a translated message

## Comments

- 2026-10-17 (from ticket 29): closing the window on a startup error screen must still quit FreeCal. `on_window_event` in `src-tauri/src/main.rs` quits when `StartupFailure` is managed; a start that fails after the core opened shows the same screen without that state, so the frontend or shell needs to cover it too.
- 2026-11-07 (from 16): hiding the window for background running must call `Core::set_window_open(false)`, and showing it again `true`, so that the Sync Scheduler polls every 15 minutes in the background. The tray menu's Sync now (ticket 16) calls `Core::sync_now(None)`.
- 2026-11-09 (from 18): the tray menu's Quit must quit as Quit in the main menu does, trying to send the Unsent Changes first: emit `freecal://quit-asked` to the window (as closing it does in `on_window_event`, `src-tauri/src/main.rs`), and quit at once only when no window is there to ask. Hiding the window for background running must not go through the close that asks to quit.
- 2026-11-10 (implementation, layer 1 of 3: core, commit 7cc729b; the review later covers everything since d009307). Decisions, for review:
  - **decision, for review:** whether FreeCal starts at login is a setting the Local Store keeps (`Settings::start_at_login`), because the Background portal can turn autostart on or off but can't say whether it is on. Strongest con: if the user deletes the autostart entry outside FreeCal, the switch still shows on until it is flipped.
  - **decision, for review:** a stored on/off setting that isn't 0 or 1 reads as its default (`stored_switch`, `core/src/store/sql/mod.rs`). Strongest con: the sidebar's stored `shown` flag uses a different rule (anything but 0 is on), so two rules exist until that one is aligned.
  - Settings now carry `runInBackground` (default on) and `startAtLogin` (default off); `SettingsUpdate` can give either.
  - Next: layer 2, the shell. Plan: (a) tray detection over D-Bus (a StatusNotifierWatcher with `IsStatusNotifierHostRegistered`), (b) the tray icon with Open FreeCal and Quit, (c) start at login through the Background portal, with an XDG autostart file as the fallback. The shell's `update_settings` turns start at login on or off on the desktop before storing it, and a refusal becomes a new `ErrorCode`. (d) `on_window_event`: a close with background running on and a tray present hides the window and calls `Core::set_window_open(false)`; Open FreeCal shows it again and calls `true`. Tray Quit emits `freecal://quit-asked`. The startup error screen still quits. (a) and (c) are tested against fakes on a private bus, as `desktop_watch.rs` does. Layer 3 is the frontend: the Background section, the refusal message, and subscribing before reading when the window comes back.
