# 18: Offline editing and replay

**What to build:** The user can create, edit, move and delete Events in server Calendars while offline. Queued changes are sent automatically when the connection returns. The user can see which changes haven't reached the server yet.

**Blocked by:** 16, 17

**Status:** ready-for-agent

- [ ] Edits while the Account is Offline succeed locally and are queued
- [ ] Queued changes are replayed when the network returns, in order, through the same conditional requests and Change Journal recording
- [ ] Occurrences returned by the core say whether they have unsent changes and since when
- [ ] The sidebar shows the unsent count per Account and Calendar when not zero
- [ ] An Occurrence with unsent changes gets a dashed outline once the change has waited more than a few seconds, or immediately when its Account is Offline (component test)
- [ ] Event details show "Not yet synced: <reason>"
