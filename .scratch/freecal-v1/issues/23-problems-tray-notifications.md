# 23: Problems in the tray and notifications

**What to build:** The tray icon shows the overall status (Conflict, then Sync Error, then Offline, then Syncing, then Online and synced). The tray menu lists the Calendars with Conflicts and the current Sync Errors, so the user can go straight to them. While the window is closed, FreeCal sends one desktop notification per new Sync Error and one per Calendar that a Conflict stops from syncing. New problems also appear in an open window at once.

- **Two Signals** feed the tray and the notifications inside the shell: "Conflicts changed" for Conflicts (ticket 133) and "Sync status changed" for Sync Errors. The spec's former "Problems changed" doesn't exist.
- **Problem lines in the tray menu:** one per Calendar with Conflicts, with their number ("Work: 3 Conflicts"), and one per Sync Error, Conflicts first. At most 8 problem lines are listed; past that, a last line "n more…" opens the window. Choosing a Calendar's line opens the window at the Conflict list for that Calendar (ticket 137; until it is built, it opens the window). Choosing a Sync Error's line opens the window.
- **Notifications for Conflicts** are per Calendar, not per Conflict: one when the server's version replaces changes of the user's ("“Work”: 3 of your changes were replaced by the server's"), none for further Conflicts until its open ones are resolved, and one again after that. The shell sees that as the Calendar's number of open Conflicts going from 0 to more; nothing is stored (as ticket 162 does in the window).
- **An open window** follows the same two Signals: the Conflict bar and the Status Marks are tickets 162 and 161. This ticket checks that they show without a reopen.

Spec: US-71, US-109, US-110, US-111; "Tray" and "Notifications" under Desktop interfaces; "Signals"; "Untrusted content".

**Blocked by:** 16, 133

**Status:** ready-for-agent

**Effort:** M

- [ ] The tray icon follows the same overall status as the window indicator, driven by the same Signals inside the shell
- [ ] The tray menu: Open FreeCal, Sync now, a line per Calendar with Conflicts (with their number) and per Sync Error, at most 8 and then "n more…", Quit; choosing a problem line opens the window at it
- [ ] With the window closed, one notification per new Sync Error and one per Calendar whose open Conflicts go from none to some, none for its further Conflicts
- [ ] An open window shows new Conflicts and Sync Errors as soon as "Conflicts changed" or "Sync status changed" arrives
- [ ] Tested with the recording fake tray and notifier
- [ ] Tray labels escape `_` and notification bodies are escaped: problem lines and notifications naming a hostile Calendar name show it literally (checked with the recording fakes)

## Comments
- 2026-10-25 (grilling, ADR 0011): an object that holds a UID another object of its Calendar owns is mirrored without rows and isn't shown. List it here as a problem of its Calendar, through ticket 91's ownership function.
- 2026-11-09: Ticket 20 was split into 133 to 137. This ticket now waits on 134, where the core lists open Conflicts. "Problems changed" comes with 133, which 134 waits on. Ticket 133 sends it only for Conflicts; whether Sync Errors send it too is decided here.
- 2026-11-09 (grilling of 133 to 137): the spec's "Problems changed" is now "Conflicts changed" (ticket 133), for Conflicts only; Sync Errors come with "Sync status changed". The tray and the notifications follow both, so the question in the comment above is settled.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. "Problems changed" is "Conflicts changed"; blocked by 16 and 133 (133 now has the list, not 134). The tray's lines and the notifications are per Calendar with Conflicts, as the spec's "Tray" and "Notifications" now read. The comment of 2026-10-25 is outdated: a resource holding a UID that a kept resource already holds is an Ignored Resource, written to the Log and not listed as a problem (ADR 0019).
- 2026-11-21 (review with the user): the tray lists at most 8 problem lines, Conflicts first, then "n more…" opening the window, so that many Calendars with problems don't make the menu run off the screen. The number 8 is a proposal, open to change. Con: a problem past the eighth line is reached only through the window.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): Calendars no longer stop syncing; the notification says the server's version replaced changes, as in the spec.
