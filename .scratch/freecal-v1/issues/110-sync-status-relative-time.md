# 110: The sync status says how long ago the last sync was, at the bottom of the sidebar

**What to build:** The status indicator says how long ago the last finished sync was, not its clock time, and its text moves from the toolbar to the bottom of the sidebar. Today it says "Synced 14:32" or "Offline · last sync 14:32", with the date when the sync wasn't today (`lastSyncText`, `src/lib/syncDisplay.ts`; `Toolbar.svelte`).

- **Wording:** "Synced 3 minutes ago"; for any other overall state, "Offline · last sync 3 minutes ago" (likewise Syncing…, Sync error, Conflict). Without a last sync, the state alone, as now. Singular and plural through `Intl.RelativeTimeFormat` with the app's locale, so that translations get it right.
- **Units and rounding:** the largest whole unit of seconds, minutes, hours and days, rounded half up: 89 s is "1 minute", 90 s "2 minutes", 1 h 25 min "1 hour", 1 h 31 min "2 hours". A value that rounds up to the next unit shows in that unit: 59 min 40 s is "1 hour", not "60 minutes". Under a minute it counts up every second; the text updates as time passes, not only on Signals.
- **Time base:** the time ago is counted between two instants, so that a DST change doesn't add or lose an hour. The overall status's `lastSync` is wall time in the Display Time Zone today; if that can't give the instant reliably, the interface carries the instant instead.
- **Where:** the sync icon stays in the toolbar where it is, and its tooltip always shows the full text. The text sits on one line at the bottom of the sidebar, outside its scrolling area, its left edge lined up with the Calendar names at every sidebar width. While the sidebar is hidden, only the tooltip shows it.
- **Accessibility:** the status group keeps its accessible name ("Sync status") and stays not a live region, as now.
- `.claude/skills/run-freecal/SKILL.md` gets the new coordinates.

Spec: US-68; "Overall status"; the UI section's status indicator line (updated 2026-11-07).

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

- [ ] The text says how long ago the last sync was, in the largest whole unit, rounded half up, counting up every second under a minute (Vitest with fake timers, including 89 s, 90 s, 1 h 25 min, 1 h 31 min and 59 min 40 s)
- [ ] Other overall states say "<state> · last sync <time ago>"; with no last sync, the state alone
- [ ] The time ago doesn't jump across a DST change
- [ ] The text sits at the bottom of the sidebar, left-aligned with the Calendar names at every sidebar width, and doesn't scroll with the Calendars
- [ ] The toolbar keeps the icon, whose tooltip always shows the full text, also while the sidebar is hidden
