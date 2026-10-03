# 16: Sync timing and triggers

**What to build:** Changes made elsewhere appear within about two minutes while the window is open, and within 15 minutes in the background. FreeCal also syncs on window focus, on wake from sleep, right after local changes, and when the network comes back. The user can sync now from a button, a keyboard shortcut or the tray menu.

**Blocked by:** 13, 15

**Status:** ready-for-agent

- [ ] Polls every 2 minutes with the window open and every 15 minutes in the background (controllable clock)
- [ ] Syncs on window focus, on wake from sleep and after local edits
- [ ] Sync now: per Account or for all; a window control, a Google-style shortcut and a tray menu item
- [ ] The NetworkMonitor portal is watched: when the network returns, a sync runs right away; with no network at all, polls are skipped and every server Account is marked Offline without trying (tested with a controllable fake network status)
