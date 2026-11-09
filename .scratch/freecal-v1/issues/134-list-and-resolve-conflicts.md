# 134: The core lists and resolves Conflicts

**What to build:** The second part of ticket 20 (read it for the history). Through the core interface (spec: "Conflicts: list them, and resolve one…"), the user lists every open Conflict that ticket 133 holds and resolves one by choosing a side (ADR 0014's amendment of 2026-11-09).

- **List:** each open Conflict with its Calendar, the Event, when it was raised, its kind (derived, ticket 133), the User's Version from the Event Rows and the Server Version from `conflict_event`, as Event fields. A deleted side is absent. Ticket 135 adds what each side changed. The list also says whether "Your version" can be chosen (below).
- **One choice for every kind:** "Your version" (the User's Version) or "Server version" (the Server Version). The frontend words each choice per kind (ticket 137).
  - **Server version:** the copy is written to the Event Rows, and the Event's Unsent Changes and the Conflict go. This is an ordinary change to the Event Rows, a Journal Event Modification in a Journal Transaction with a Transaction Origin for resolving, and ticket 19's undo undoes it like any other change, bringing the User's Version back as an Unsent Change.
  - **Your version:** the Conflict goes, and the Unsent Changes stay for the Sender. After a remote delete, the Server Object is gone from the Mirror, so the Sender creates it again. Otherwise it updates or deletes with `If-Match` on the Server Version's ETag. No Event Rows change, so nothing is recorded in the Change Journal, and this choice can't be undone.
- **Through ticket 131's module:** resolving asks `ReadOnly::may_edit` (`core/src/read_only.rs`) like any edit. Taking the Server Version is allowed in a Read-only Calendar for the module's existing reason: it loads what the server has, as an Incoming Sync does. Extend that reason to the new Transaction Origin, without an exception.
- **No "Your version" while the server doesn't allow changes:** while the Calendar is Server read-only, choosing the User's Version fails with an error, and the Conflict stays open until the user takes the Server Version or the permission returns. A Calendar only the user marked read-only still allows it, as Unsent Changes made before the mark are still sent.
- **A stale choice:** the call names the state of both sides the user saw. If a sync replaced the Server Version or a local change (ticket 138) changed the User's Version meanwhile, resolving fails without changing anything. A Conflict that ended meanwhile fails too, with its own error.
- **After resolving:** "Conflicts changed", and "Occurrences changed" for the Calendar, sent through the Journal Transaction as ticket 133 wires it. Resolving is a local change, so it asks for a sync of the Calendar as an edit does (`Core::after_local_change`, ticket 124).

Spec: US-77; "Core application interface"; "Signals"; "Change Journal"; "Writability"; "Untrusted content". ADRs 0009, 0014.

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** M

- [ ] The core lists the open Conflicts with both versions and whether "Your version" can be chosen
- [ ] Each of the three kinds resolves both ways, and the choice reaches the server at the next sync
- [ ] "Server version" is a Journal Event Modification; "Your version" records nothing in the Change Journal
- [ ] "Server version" resolves in a Read-only Calendar through ticket 131's module; "Your version" fails while the Calendar is Server read-only
- [ ] Resolving against a changed side or an ended Conflict fails without changing anything
- [ ] Resolving sends "Conflicts changed" and "Occurrences changed", and asks for a sync of the Calendar
- [ ] Written test-first against Radicale

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): rewritten after the grilling of tickets 133 to 137. Ticket 20's "keeps the losing Event Version, so a wrong choice can be undone" now holds only for "Server version": the Change Journal records changes to Event Rows, and nothing else. A wrong "Your version", once sent, can only be put right by hand. Ticket 19 needs no box for it, as undoing "Server version" is an ordinary undo.
