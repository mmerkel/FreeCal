# 137: The Conflict list in the window

**What to build:** In the window, the user sees the Conflicts of all Calendars or of one, sees both versions of each with what each side changed, chooses, and ends a Calendar's Conflicts. This builds on the core's list (ticket 133), on what each side changed (135) and on the call that ends a Calendar's Conflicts (134).

- **The Conflict list** is a dialog, for all Calendars or for one. It groups the Conflicts by Calendar and shows each with its Event's title, its kind and the choice made so far. It is read again on "Conflicts changed".
- **Where it opens from:** the Conflict Status Mark on a Calendar's line (ticket 161), with that Calendar's Conflicts, and the details of an Event in Conflict, with its Calendar's Conflicts and that one open. The Conflict bar and the dialog when a Calendar stops Tracking open it too; they are ticket 162.
- **One Conflict** opens from the list and shows the two sides side by side, labelled "Your version" (the User's Version) and "Server version" (the Server Version), each with what it changed against the Last Agreed Version ("you changed the time, they changed the title", ticket 135). A deleted side says so; a side that created the Event says so.
- **The choices, per kind:**
  - an edit on both sides: keep "Your version" or the "Server version";
  - the user's edit against the server's delete: "restore with my changes" or "accept the delete";
  - the user's delete against the server's edit: "delete anyway" or "keep the server version".
- **Choices are applied per Calendar, together** (ticket 134). A choice is kept in the window until every Conflict of its Calendar has one; the Calendar's group says how many are still open and has "Apply", enabled once none is. Two buttons per Calendar set every choice at once, "Your version for all" and "Server version for all"; the user still applies. Choices not applied are kept while FreeCal runs, also when the list is closed, and are stored nowhere.
- **After "Apply"** the Calendar's group goes, and the list closes when no Calendar is left. No toast offers undo and no choice carries a note that it can't be undone: Ctrl+Z undoes the merge as any other (ticket 158).
- **While the Calendar is Server read-only,** each choice keeping the User's Version is disabled, with a note that the server doesn't allow changes to this Calendar (ticket 134).
- **Changes from elsewhere:**
  - On "Conflicts changed" the list is read again. A choice is kept if its Conflict is still listed with the same versions (ticket 133's value) and dropped otherwise; a Conflict whose choice was dropped says that it changed.
  - If "Apply" fails because a choice was stale (ticket 134), the list is read again in the same way and says so.
  - If a Calendar's Conflicts end while the list is open (a sync, or the user's change, ticket 138), its group goes with "“Work” is syncing again".
- **Editing stays open:** the editor saves an Event in Conflict normally (spec: an open editor saves as an ordinary edit).
- The dialogs go through Window Dialogs (ticket 118). The tests run on the recording fake core (ticket 116). All text in `src/i18n/en.ts`. Event fields and Calendar names are untrusted text.

Spec: US-71, US-73 to US-77; "Core application interface"; "A Calendar with a Conflict isn't syncing" under the frontend; "Untrusted content". ADR 0019 ("Conflicts").

**Blocked by:** 134, 135, 136

**Status:** ready-for-agent

**Effort:** M

- [ ] The Conflict list for all Calendars or for one, grouped by Calendar, opened from the Conflict Status Mark and from an Event's details, kept current by "Conflicts changed"
- [ ] A Conflict shows "Your version" and "Server version" with what each side changed
- [ ] The three kinds each offer their two choices, in the spec's words
- [ ] "Apply" is enabled once every Conflict of the Calendar has a choice, and ends them with one call; "for all" sets every choice
- [ ] Keeping the User's Version is disabled while the Calendar is Server read-only
- [ ] A choice whose Conflict changed is dropped and says so; a Calendar whose Conflicts ended elsewhere leaves the list
- [ ] No undo toast and no "can't be undone" note
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137: only "Server version" can be undone, "Your version" is disabled while the Calendar is Server read-only, a Conflict can end while its dialog is open, and the Signal is "Conflicts changed".
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above are history. The list is for all Calendars or for one and opens from the Conflict Status Mark (161), the Conflict bar and the stopped-Tracking dialog (162) and an Event's details, no longer from the toolbar's status indicator. The undo toast after "Server version", the "can't be undone" note on "Your version" and the toast "Conflict resolved" are gone. No longer blocked by 138: a Conflict that ends elsewhere is seen through "Conflicts changed".
- 2026-11-16: **Open, hence `needs-info`:** choices are applied per Calendar, together, as ticket 134's call takes them (see its comment for the reason and the con). Chosen here, for review, if that is agreed: the "for all" buttons, choices kept in the window while FreeCal runs, and the text "“Work” is syncing again".
- 2026-11-16: Agreed by the user: choices are applied per Calendar, together (see ticket 134). `ready-for-agent`. The "for all" buttons, the keeping of choices and the text stay for review.
