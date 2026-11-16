# 161: Status Marks in the sidebar, the Unsent Mark, and the progress of a long sync

**What to build:** The sidebar and the Event chips show a Calendar's state with marks, not text (US-69, US-72), and a sync that takes long shows how far it has come. Today the sidebar writes a Sync Status as text and shows a count of unsent changes (`src/lib/Sidebar.svelte`), and chips mark an unsent Occurrence otherwise (`src/lib/CalendarGrid.svelte`, `src/lib/unsent.ts`).

- **"Sync status changed"** for a Calendar carries whether it has unsent changes instead of how many (read from the Current-Version Marks, ticket 153), how many Conflicts it has, and how far a sync that takes long has come. The count of Conflicts is 0 until ticket 133 fills it. The Account's report follows.
- **Progress of an Incoming or Outgoing Sync:** the Sync Engine reports it through the Running Sync; what is counted depends on the Provider (CalDAV: resources fetched of those to fetch, UIDs sent of those to send). It is sent at most a few times a second, and only once the sync has run longer than the overall status's delay.
- **Status Marks on a line:** an Account's or a Calendar's line carries the Status Mark of every state that applies to it, side by side, in the order Conflict, Sync Error, Offline, Syncing, and none when there is nothing to report. They are the marks of the status indicator (`src-tauri/icons/src/mark-<status>.svg`) at about the size of the sidebar's text. Each state is on the line it belongs to and not repeated: Offline and an Account's own Sync Errors on the Account, a Conflict and a Calendar's own Sync Errors on the Calendar.
- **Clicks and tooltips:** a click on a Sync Error mark shows its message and its action (log in again, retry; "Send" for a paused Calendar comes with ticket 22). The Offline and Syncing marks have a tooltip only. The Syncing mark follows the overall status's delay rule, and its tooltip and the status indicator's show the progress. The Conflict mark is drawn from the count; its click opens the Conflict list with that Calendar's Conflicts once ticket 137 has built it, and does nothing before.
- **The Unsent Mark on a Calendar's line:** an hourglass after the Status Marks while any of its Events is unsent, with the reason as its tooltip (Offline, a Conflict in the Calendar, or waiting to be sent) and no count. A click does nothing. The lock of a Read-only Calendar stays.
- **The Unsent Mark on a chip:** at the chip's right edge, taking room from the title and never dropped, on small chips too. It appears once the change has waited more than a few seconds, or at once when the Account is Offline or the Calendar has a Conflict. A chip carries at most one mark: the Conflict badge (ticket 136) comes before it.
- **Event details** show the Unsent Mark with "Not yet synced: <reason>", as now.
- **The status indicator's text** at the bottom of the sidebar and its tooltip stay as they are, apart from the progress.
- **Accessibility:** each mark has an accessible name through `t`; the line's description names its states, as the count's does today.

Spec: US-68, US-69, US-72; "Signals"; the status indicator, "Event chips stay sparse" and "Event details" under the frontend. ADR 0019 ("What to send").

**Blocked by:** 153

**Status:** ready-for-agent

**Effort:** M

- [ ] "Sync status changed" carries has-unsent-changes, the Conflict count and the progress; the unsent count is gone from the core interface
- [ ] A long Incoming Sync and a long Outgoing Sync report progress (test with held requests); a short one reports none
- [ ] A line shows every Status Mark that applies, side by side, and none when Synced; Account-level states are on the Account's line only
- [ ] A click on a Sync Error mark shows its message and action; the Syncing mark's tooltip shows the progress
- [ ] A Calendar with unsent Events shows the hourglass with its reason; no number anywhere
- [ ] A chip shows the Unsent Mark after a few seconds, or at once when Offline; small chips keep it
- [ ] Each mark has an accessible name
- [ ] Component tests on the fake core; one look in the real window in light and dark (`run-freecal`)

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Ticket 15 keeps the overall status; its reword (the Signal's fields, the first-sync marker gone with ticket 155) follows in the triage's last batch. Ticket 68's "dashed outline" becomes the Unsent Mark there.
