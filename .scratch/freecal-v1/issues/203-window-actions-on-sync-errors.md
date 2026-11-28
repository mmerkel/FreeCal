# 203: The window's actions on Sync Errors: "Send" on a paused Calendar, the "writes refused" bar

**What to build:** The words and actions for the two Sync Error kinds that 200 and 201 added to the core, which the window shows until now as a plain fallback text. The window's parts of old tickets 22 (`git show design/snapshot-line:.scratch/freecal-v1/issues/22-mass-change-guard.md`) and 166 (`git show design/snapshot-line:.scratch/freecal-v1/issues/166-writes-refused-waits-like-offline.md`).

- **Paused (201):** the Sync Error mark's click (202) shows "Paused: 120 Events would be deleted on the server" with the numbers to change and delete, and "Send", which calls 201's call for that Calendar. An Event's details and the Unsent Mark's tooltip give the reason "paused" in plain words. A refused "Send" (the Calendar is no longer paused) reads the Sync Status again and shows nothing more.
- **Writes refused (200):** a bar across the top of the window while any Calendar has the Sync Error "writes refused", naming the Calendar (or how many), with "Stop waiting and mark read-only", or "Stop waiting" on a Calendar already marked. It takes its height from the content below and covers nothing; it can't be dismissed and goes with the Sync Error. The Sync Error mark's click shows the same message and action. 206's Conflict bar reuses this bar's component.
- **The confirmation** counts with 200's number of waiting Events: “Discard 3 unsent changes in “Work” and mark it read-only? You can remove the mark once the server accepts changes again.” (spec: "Writability"); "Stop waiting" alone drops the last clause. On confirm, 200's call, with or without the mark.
- **Removing the read-only mark** where the server can't be reached or says no shows 200's error: “Can't reach the server; try again when “Work” is online”, or that the server still refuses changes.
- **Through the Window Dialogs** (`src/lib/windowDialogs.svelte.ts`); all text in `src/i18n/en.ts`; Calendar names are untrusted text, shown as text only. The fallback texts of 200 and 201 go.

Spec: US-16, US-19, US-78, US-86, US-87, US-154; "Writability"; the status indicator under the frontend; "Untrusted content". ADR 0019 ("When the server refuses changes", "Consequences"). Layout: "By phase", "P5 and P6".

**Blocked by:** 200, 201, 202

**Status:** ready-for-agent

**Effort:** S

- [ ] A paused Calendar's Sync Error shows the numbers and "Send", which calls the core; an unsent Event's reason says paused
- [ ] The bar shows while any Calendar's writes are refused, names the Calendar or the count, and goes with the Sync Error
- [ ] "Stop waiting and mark read-only" and "Stop waiting" ask first with the count of waiting changes, then call the core
- [ ] Removing the mark shows the core's error when the server can't be reached or says no
- [ ] The fallback texts of 200 and 201 are gone
- [ ] Component tests on the fake core for each rule; one look in the real window (`run-freecal`)
