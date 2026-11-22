# 137: The Conflict list in the window

**What to build:** In the window, the user sees the open Conflicts of all Calendars or of one, sees both versions of each with the fields where they differ, and resolves them by keeping the server's version or restoring their own. This builds on the core's list (ticket 133), on what each side changed (135) and on the Resolution calls (134).

- **The Conflict list** is a dialog, for all Calendars or for one. It groups the open Conflicts by Calendar and shows each with its Event's title, its kind and when it was replaced. It is read again on "Conflicts changed".
- **Where it opens from:** the Conflict Status Mark on a Calendar's line (ticket 161), with that Calendar's Conflicts; the details of an Event in Conflict, with its Calendar's Conflicts and that one open; and "Review" on the Conflict bar (ticket 162).
- **One Conflict** opens from the list and shows the two sides side by side, labelled "Your version" (the User's Version, which was replaced) and "Server version" (the Server Version, which FreeCal shows), with the fields where they differ marked (ticket 135). A side where the Event was deleted says so.
- **The choices,** each applied at once, per Conflict: "Keep the server's version" and "Restore my version", with the restore worded per kind: "Restore my changes" after the server's delete, "Delete it again" after the user's delete. Each Calendar's group has "Keep all". A restore is an Undo: its toast is the Undo's, with "Redo" (ticket 158), and a refused restore shows the refusal's toast.
- **After a choice** the Conflict leaves the list, and the list closes when none is left. Editing the Event from its details resolves it too (ticket 134), and the list follows through "Conflicts changed".
- **In a Calendar marked read-only** "Restore my version" is disabled with the read-only note, as an Undo is refused there (ticket 134).
- **Changes from elsewhere:** on "Conflicts changed" the list is read again; a Conflict resolved elsewhere leaves it. If "Keep" fails because the Conflict was already resolved (ticket 134), the list is read again.
- The dialogs go through Window Dialogs (ticket 118). The tests run on the recording fake core (ticket 116). All text in `src/i18n/en.ts`. Event fields and Calendar names are untrusted text.

Spec: US-71, US-73 to US-77, US-153; "Core application interface"; the Conflict bar under the frontend; "Untrusted content". ADR 0019 ("Conflicts end at once").

**Blocked by:** 134, 135, 136

**Status:** ready-for-agent

**Effort:** M

- [ ] The Conflict list for all Calendars or for one, grouped by Calendar, opened from the Conflict Status Mark, an Event's details and the bar, kept current by "Conflicts changed"
- [ ] A Conflict shows "Your version" and "Server version" with what each side changed
- [ ] "Keep the server's version", "Restore my version" (worded per kind) and "Keep all" each resolve at once
- [ ] A restore shows the Undo's toast with "Redo"; a refused one shows the refusal
- [ ] "Restore my version" is disabled in a Calendar marked read-only
- [ ] A Conflict resolved elsewhere leaves the list
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-16: Rewritten from the triage against ADRs 0017 to 0020; the comments above are history. The list is for all Calendars or for one and opens from the Conflict Status Mark (161), the Conflict bar and the stopped-Tracking dialog (162) and an Event's details, no longer from the toolbar's status indicator. The undo toast after "Server version", the "can't be undone" note on "Your version" and the toast "Conflict resolved" are gone. No longer blocked by 138: a Conflict that ends elsewhere is seen through "Conflicts changed".
- 2026-11-16: **Open, hence `needs-info`:** choices are applied per Calendar, together, as ticket 134's call takes them (see its comment for the reason and the con). Chosen here, for review, if that is agreed: the "for all" buttons, choices kept in the window while FreeCal runs, and the text "“Work” is syncing again".
- 2026-11-16: Agreed by the user: choices are applied per Calendar, together (see ticket 134). `ready-for-agent`. The "for all" buttons, the keeping of choices and the text stay for review.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADR 0019 rewritten): Conflicts end at once with the server's version, so each is resolved on its own and at once (keep or restore); "Apply", choices held in the window, "Your version for all" and "is syncing again" are gone. The restore's wording per kind ("Restore my changes", "Delete it again").
- 2026-11-22: Agreed by the user: the restore's wording per kind ("Restore my changes", "Delete it again").
- 2026-11-22: The two sides are shown with the fields where they differ, not what each changed against the Last Agreed Version, which is no longer kept (point 4 of `.scratch/freecal-v1/critiques-2.md`, ADR 0019).
