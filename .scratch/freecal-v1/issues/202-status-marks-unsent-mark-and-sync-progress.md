# 202: Status Marks in the sidebar, the Unsent Mark, and the progress of a long sync

**What to build:** The sidebar and the Event chips show a Calendar's state with marks, not text (US-69, US-72), and a sync that takes long shows how far it has come. Old ticket 161 (`git show design/snapshot-line:.scratch/freecal-v1/issues/161-status-marks-unsent-mark-and-sync-progress.md`) as written, on the Sync Status of 190 and 194.

- **The Signal "Sync status changed"** carries whether a Calendar has unsent changes instead of how many (read from the Current-Version Marks, 192), the number of open Conflicts (190), and how far a long sync has come. The Account's report follows. The unsent count leaves the core interface.
- **Progress:** the Sync Engine reports it through the Running Sync, counted per Provider (CalDAV: resources fetched of those to fetch, UIDs sent of those to send), at most a few times a second, and only once the sync has run longer than the overall status's delay.
- **Status Marks on a line:** an Account's or a Calendar's line carries the Status Mark of every state that applies, side by side, in the order Conflict, Sync Error, Offline, Syncing, and none when there is nothing to report: the marks of the status indicator (`src-tauri/icons/src/mark-<status>.svg`) at about the size of the sidebar's text. Offline and an Account's own Sync Errors on the Account, a Conflict and a Calendar's own Sync Errors on the Calendar, never repeated.
- **A Calendar not loaded yet** (194: no finished sync): the Syncing mark's tooltip says "Loading for the first time" with the progress; a Sync Error's or Offline mark's tooltip adds "Not loaded yet".
- **Clicks and tooltips:** a click on a Sync Error mark shows its message and its action (log in again, retry; "Send" and "Stop waiting" are 203's). Offline and Syncing have a tooltip only; the Syncing mark follows the overall status's delay rule, and its tooltip and the status indicator's show the progress. The Conflict mark is drawn from the count; its click does nothing until 205 opens the Conflict list.
- **The Unsent Mark on a Calendar's line:** an hourglass after the Status Marks while any of its Events is unsent, with the reason as tooltip (Offline, writes refused, paused, or waiting to be sent), no count. A click does nothing. The lock of a Read-only Calendar stays.
- **The Unsent Mark on a chip:** at the chip's right edge, taking room from the title and never dropped, on small chips too; once the change has waited more than a few seconds, or at once when Offline or writes are refused. A chip carries at most one mark: the Conflict badge (204) comes first. **Event details** show "Not yet synced: <reason>", as now. **The status indicator's text** stays, apart from the progress.
- **Accessibility:** each mark has an accessible name through `t`; the line's description names its states.

Spec: US-68, US-69, US-72; "Signals"; the status indicator, "Event chips stay sparse" and "Event details" under the frontend. ADR 0019 ("What to send"). Layout: "By phase", "P5 and P6".

**Blocked by:** 192, 194

**Status:** ready-for-agent

**Effort:** M

**Rewritten:** `src/lib/CalendarGrid.test.ts`'s "an Occurrence with Unsent Changes"; `src/lib/unsent.test.ts`; the unsent count in `src/App.sync.test.ts`; `src/App.quit.test.ts` only if the quit warning's reading of the interface changes.

- [ ] "Sync status changed" carries has-unsent-changes, the Conflict count and the progress; the unsent count is gone from the core interface; the TypeScript types are regenerated
- [ ] A long Incoming Sync and a long Outgoing Sync report progress (test with held requests); a short one reports none
- [ ] A line shows every Status Mark that applies, side by side, and none when Synced; Account-level states are on the Account's line only
- [ ] A Calendar with no finished sync yet shows "Loading for the first time" while syncing and "Not loaded yet" after a failure
- [ ] A click on a Sync Error mark shows its message and action; the Syncing mark's tooltip shows the progress
- [ ] A Calendar with unsent Events shows the hourglass with its reason; no number anywhere
- [ ] A chip shows the Unsent Mark after a few seconds, or at once when Offline; small chips keep it
- [ ] Each mark has an accessible name
- [ ] Component tests on the fake core; the tests above rewritten; one look in the real window in light and dark (`run-freecal`)
