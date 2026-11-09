# 137: List and resolve Conflicts in the window

**What to build:** The fifth part of ticket 20 (read it for the history). In the window, the user lists all open Conflicts, sees both versions with what each side changed, and chooses. This builds on the core's list and resolve (tickets 134, 135).

- **The Conflicts list:** every open Conflict, with its Event's title, Calendar and kind. It opens from the toolbar's status indicator while the overall status is Conflict, and is read again on "Problems changed".
- **The Conflict dialog:** opens from the list, and from the Event details of an Event in Conflict. It shows the two sides side by side, labelled "Your version" (the User's Version) and "Server version" (the Server Version), each with what it changed ("you changed the time, they changed the title", ticket 135). A deleted side says so. Without a common version, it shows the versions only.
- **The choices, per kind:**
  - edit/edit: keep "Your version" or the "Server version";
  - local edit vs remote delete: "restore with my changes" or "accept the delete";
  - local delete vs remote edit: "delete anyway" or "keep the server version".
  After a choice, the toast offers undo, as other changes do.
- **A stale choice:** if resolving fails because a sync replaced the Server Version (ticket 134), the dialog shows the new version and says that it changed.
- The dialogs go through Window Dialogs (ticket 118). The tests run on the recording fake core (ticket 116). All text in `src/i18n/en.ts`. Event fields are untrusted text.

Spec: US-33, US-71, US-77; "Problems"; "Untrusted content".

**Blocked by:** 134, 135, 136

**Status:** ready-for-agent

**Effort:** M

- [ ] A list of all open Conflicts, opened from the status indicator and kept current by "Problems changed"
- [ ] The Conflict dialog shows "Your version" and "Server version" with what each side changed
- [ ] Edit/edit: both versions shown, the user chooses one
- [ ] Local edit vs remote delete: "restore with my changes" or "accept the delete"
- [ ] Local delete vs remote edit: "delete anyway" or "keep the server version"
- [ ] Resolving applies the choice and can be undone from the toast
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-09: Split from ticket 20.
