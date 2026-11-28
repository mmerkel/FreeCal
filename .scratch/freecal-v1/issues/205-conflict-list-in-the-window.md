# 205: The Conflict list in the window

**What to build:** The user sees the open Conflicts of all Calendars or of one, sees both versions of each with what each side changed, and resolves them by keeping the server's version or restoring their own. On the core's list (190), what each side changed (199) and the calls that resolve (191). Old ticket 137 (`git show design/snapshot-line:.scratch/freecal-v1/issues/137-conflicts-in-the-window.md`) as written.

- **The Conflict list** is a dialog through the Window Dialogs, for all Calendars or one, grouped by Calendar, each Conflict with its Event's title, its kind and when the Incoming Sync was. It is read again on "Conflicts changed"; a Conflict resolved elsewhere leaves it, and it closes when none is left.
- **Where it opens:** the click on a Calendar's Conflict Status Mark (202 leaves that click doing nothing; this ticket wires it), with that Calendar's Conflicts; an Event's details when the Event is in Conflict (204's flag), with its Calendar's Conflicts and that one open; and "Review" on the Conflict bar (206).
- **One Conflict** shows the two sides side by side, "Your version" (the User's Version) and "Server version" (what FreeCal shows), with what each side changed marked in words from 199's values ("changed details FreeCal doesn't show" for the Unimplemented Fields alone; the Occurrences of a Recurring Event by date; "differs" without a Last Agreed Version). A side where the Event was deleted says so.
- **The choices,** applied at once: "Keep the server's version" and "Restore my version", worded per kind ("Restore my changes" after the server's delete, "Delete it again" after the user's); "Keep all" per Calendar. A restore is an Undo: 183's toast with "Redo", and 183's dialog where the core asks first. A "keep" refused as already resolved reads the list again.
- **A Calendar marked read-only:** "Restore my version" is disabled with the read-only note (191).
- All text in `src/i18n/en.ts`; Event fields and Calendar names are untrusted text.

Spec: US-71, US-73 to US-77, US-153; "Core application interface"; the status indicator and the Conflict bar under the frontend; "Untrusted content". ADR 0019 ("Conflicts end at once"). Layout: "By phase", "P5 and P6".

**Blocked by:** 191, 199, 202, 204

**Status:** ready-for-agent

**Effort:** M

- [ ] The list for all Calendars or one, grouped by Calendar, opened from the Conflict Status Mark, an Event's details and the bar, kept current by "Conflicts changed"
- [ ] A Conflict shows "Your version" and "Server version" with what each side changed, a Recurring Event's Occurrences by date, and a deleted side
- [ ] "Keep the server's version", "Restore my version" (worded per kind) and "Keep all" each resolve at once
- [ ] A restore shows 183's toast with "Redo", or its dialog where the core asks first; a stale "keep" reads the list again
- [ ] "Restore my version" is disabled in a Calendar marked read-only
- [ ] Component tests on the fake core; one run in the real window against Radicale (`run-freecal`)
