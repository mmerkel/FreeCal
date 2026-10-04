# 18: Offline editing and replay

**What to build:** The user can create, edit, move and delete Events in server Calendars while offline. Queued changes are sent automatically when the connection returns. The user can see which changes haven't reached the server yet, and is warned when quitting with changes not yet sent. Spec: user story 131 and "Quitting" under Frontend.

**Blocked by:** 13, 16, 17, 33

**Status:** ready-for-agent

- [ ] Edits while the Account is Offline succeed locally and are queued
- [ ] Queued changes are replayed when the network returns, in order, through the same conditional requests and Change Journal recording
- [ ] Occurrences returned by the core say whether they have unsent changes and since when
- [ ] The sidebar shows the unsent count per Account and Calendar when not zero
- [ ] An Occurrence with unsent changes gets a dashed outline once the change has waited more than a few seconds, or immediately when its Account is Offline (component test)
- [ ] Event details show "Not yet synced: <reason>"
- [ ] Quitting with unsent changes (Quit in the main menu, Quit in the tray menu, or closing the window when that quits FreeCal) first tries to send them for a few seconds under the "Working…" dialog ("Sending 2 changes…"), then quits if that worked
- [ ] If changes are still unsent, a modal dialog says how many, lists the Calendars with the reason for each (Offline, Conflict, waiting for approval, error) and says they'll be sent at the next start, or that they need attention for a Conflict or a paused Calendar. "Cancel" has initial focus; "Quit anyway" quits
- [ ] Logout and system shutdown never wait for this and never show the dialog; the queue stays in the Local Store
- [ ] Core tests for the send-before-quit attempt with a controllable clock; component tests against the fake core for the dialog, Cancel and Quit anyway
