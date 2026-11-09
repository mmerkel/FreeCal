# 137: List and resolve Conflicts in the window

**What to build:** The fifth part of ticket 20 (read it for the history). In the window, the user lists all open Conflicts, sees both versions with what each side changed, and chooses. This builds on the core's list and resolve (tickets 134, 135) and on local changes ending a Conflict (138).

- **The Conflicts list:** every open Conflict, with its Event's title, Calendar and kind. It opens from the toolbar's status indicator while the overall status is Conflict, and is read again on "Conflicts changed".
- **The Conflict dialog:** opens from the list, and from the Event details of an Event in Conflict. It shows the two sides side by side, labelled "Your version" (the User's Version) and "Server version" (the Server Version), each with what it changed against the Last Agreed Version ("you changed the time, they changed the title", ticket 135). A deleted side says so. Without a Last Agreed Version, it shows the versions only.
- **The choices, per kind:**
  - edit/edit: keep "Your version" or the "Server version";
  - local edit vs remote delete: "restore with my changes" or "accept the delete";
  - local delete vs remote edit: "delete anyway" or "keep the server version".
- **While the Calendar is Server read-only,** the choice keeping the User's Version is disabled, with a note that the server doesn't allow changes to this Calendar (ticket 134).
- **Undo:** after choosing the Server Version, the toast offers undo, as other changes do. Choosing the User's Version can't be undone, so the dialog says so beside that choice before the user takes it.
- **Changes from elsewhere:**
  - If resolving fails because a side changed (ticket 134), the dialog shows the new versions and says what changed.
  - If the Conflict ends while its dialog is open (a sync, or a local change, ticket 138), the dialog closes with "This Conflict was resolved".
  - An undo toast for a change that ended a Conflict says "Conflict resolved".
- **Editing stays open:** the editor saves an Event in Conflict normally (spec: an open editor saves as an ordinary edit).
- The dialogs go through Window Dialogs (ticket 118). The tests run on the recording fake core (ticket 116). All text in `src/i18n/en.ts`. Event fields are untrusted text.

Spec: US-33, US-71, US-77; "Core application interface"; "Untrusted content".

**Blocked by:** 134, 135, 136, 138

**Status:** ready-for-agent

**Effort:** M

- [ ] A list of all open Conflicts, opened from the status indicator and kept current by "Conflicts changed"
- [ ] The Conflict dialog shows "Your version" and "Server version" with what each side changed
- [ ] Edit/edit: both versions shown, the user chooses one
- [ ] Local edit vs remote delete: "restore with my changes" or "accept the delete"
- [ ] Local delete vs remote edit: "delete anyway" or "keep the server version"
- [ ] Keeping the User's Version is disabled while the Calendar is Server read-only, and its dialog says it can't be undone
- [ ] Choosing the Server Version can be undone from the toast; a Conflict that ends elsewhere closes its dialog
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137: only "Server version" can be undone, "Your version" is disabled while the Calendar is Server read-only, a Conflict can end while its dialog is open, and the Signal is "Conflicts changed".
