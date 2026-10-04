# 13: Background running, tray basics and autostart

**What to build:** Where a system tray exists, closing the window keeps FreeCal running in the background (on by default); without a tray, closing quits. A tray enabled while FreeCal runs is noticed at the next close. The user can turn background running off and turn start-at-login on. The tray menu offers Open FreeCal and Quit. Both options live in the Background section of the Settings dialog (ticket 33). Spec: user stories 103–108 and 129.

**Blocked by:** 01, 33

**Status:** ready-for-agent

- [ ] At every window close, a D-Bus check for a StatusNotifierWatcher with a registered host decides whether to keep running or quit
- [ ] Background running is on by default and can be turned off, so closing always quits
- [ ] Start at login is off by default and uses the Background portal where available
- [ ] A StatusNotifierItem tray icon (works natively on KDE Plasma) with a menu: Open FreeCal, Quit
- [ ] When the window is re-created, the frontend subscribes to Signals first and then reads current state
- [ ] Tray detection, tray and autostart are behind fakeable desktop interfaces with tests
- [ ] The Background section of Settings has "Keep running in the background" and "Start at login" switches. The background switch is always enabled, with the hint "Needs a system tray; without one, closing quits", because a tray is only checked for at window close
- [ ] Turning on start at login goes through the Background portal at the moment the switch is flipped (the portal may ask the user); if it is refused, the switch goes back off with a translated message
