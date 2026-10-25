# 23: Problems in the tray and notifications

**What to build:** The tray icon shows the overall status (Conflict, then Sync Error, then Offline, then Syncing, then Online and synced). The tray menu lists current Conflicts and Sync Errors so the user can go straight to them. While FreeCal runs in the background, each new Conflict or Sync Error gets one desktop notification. New problems also appear in an open window immediately.

**Blocked by:** 16, 20

**Status:** ready-for-agent

**Effort:** M

- [ ] The tray icon follows the same overall status as the window indicator, driven by the same Signals inside the shell
- [ ] The tray menu: Open FreeCal, Sync now, a line per current Conflict and Sync Error, Quit; choosing a problem line opens the window at it
- [ ] With the window closed, one notification per new Conflict or Sync Error
- [ ] An open window shows new Conflicts and Sync Errors as soon as "Problems changed" arrives
- [ ] Tested with the recording fake tray and notifier
- [ ] Tray labels escape `_` and notification bodies are escaped: problem lines naming the hostile test Event or a hostile Calendar name show literally (checked with the recording fakes)

## Comments
- 2026-10-25 (grilling, ADR 0011): an object that holds a UID another object of its Calendar owns is mirrored without rows and isn't shown. List it here as a problem of its Calendar, through ticket 91's ownership function.
