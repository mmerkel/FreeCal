# 09: Export to .ics and .zip

**What to build:** The user can export any Calendar to an `.ics` file, or export everything to a `.zip` with one `.ics` per Calendar organised by Account. While the export runs, a modal "Working…" dialog blocks the window.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] Export one Calendar to a single `.ics`
- [ ] Export everything to a `.zip`, one `.ics` per Calendar, organised by Account
- [ ] The file dialog goes through a portal-friendly API
- [ ] Long-running commands show a modal "Working…" dialog that blocks the window until they finish
- [ ] Exported data round-trips: re-importing yields the same Events (verified once 10 exists, or by parsing in the test)
- [ ] The `.ics` is written through a library writer, never built from strings; the hostile test Event round-trips without injecting properties (`\r\nATTENDEE:…` stays inside its field)
- [ ] `.zip` entry names made from Account and Calendar names are sanitized: a Calendar named `../../.bashrc` stays in its folder
