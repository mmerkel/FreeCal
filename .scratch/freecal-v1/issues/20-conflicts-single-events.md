# 20: Conflicts on single Events

**What to build:** When a local change and a newer server change to the same Event can't both be kept, FreeCal holds a Conflict instead of guessing. The Event carries a Conflict badge, the user can list all open Conflicts, see both versions and choose.

**Blocked by:** 91, 92, 100, 127, 128

**Status:** wontfix

**Split into:** 133, 134, 135, 136, 137, 138

- [ ] Edit/edit: both versions shown, the user chooses one
- [ ] Local edit vs remote delete: "restore with my changes" or "accept the delete"
- [ ] Local delete vs remote edit: "delete anyway" or "keep the server version"
- [ ] The Conflict badge is the only badge on an Event chip
- [ ] A list of all open Conflicts; resolving one applies the choice and records it in the Change Journal when it writes to the server
- [ ] The core sends "Problems changed" when a Conflict is raised or resolved
- [ ] The Conflict shows what each side changed against the common version from the Change Journal ("you changed the time, they changed the title")
- [ ] Resolving is a Journal Transaction that keeps the losing Event Version, so a wrong choice can be undone
- [ ] Conflicts are found in the Provider-neutral core (ADR 0014): the Sync Engine gives the Server Version of each Calendar and UID the server changed, and a neutral step propagates it to the Event Rows or raises a Conflict when an Unsent Change is about the Event
- [ ] A Conflict keeps a copy of the Server Version in `conflict` and `conflict_server_event`, written with the Change Journal's Write Permission; a later Incoming Sync changing the same UID replaces the copy, and "keep the server version" writes it to the Event Rows
- [ ] The two sides are labelled "Your version" (the User's Version) and "Server version" (the Server Version)
- [ ] Written test-first against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-24: Per ADR 0010, a server change to an object that is pending (changed in FreeCal, not sent yet) is a Conflict, never a silent replacement; today sync takes it. This includes an undone pulled change waiting to be sent.
- 2026-10-25 (grilling, ADR 0011): blocked by 91. "Pending" now means a Server Object with one of its Combined Event Keys in `unsent_change`.
- 2026-11-02: Ticket 91 was split; `unsent_change` is now ticket 92, so this ticket is blocked by both.
- 2026-11-06 (grilling, ADR 0014): blocked by 100. A Conflict is no longer "a server change to a Server Object while one of its Combined Event Keys is in `unsent_change`" (comment of 2026-10-25); it is found per Occurrence in the Provider-neutral core from the Server Version, as ADR 0014 says. For a single Event, "per Occurrence" is the Event itself; the rules for Recurring Events are ticket 21's. New terms in `GLOSSARY.md`: Server Version, User's Version.
- 2026-11-07 (from 15): the overall status already ranks Conflict first (`OverallState::Conflict`), but nothing raises it: feed the Conflicts into `SyncStatusTracker::overall` (`core/src/sync_status.rs`) and tick ticket 15's overall-status box.
- 2026-11-09: Split into 133 (the core finds and holds Conflicts on single Events, with the overall status and "Problems changed"), 134 (the core lists and resolves Conflicts, undoably), 135 (what each side changed against the common version), 136 (the Conflict badge on Event chips) and 137 (list and resolve Conflicts in the window), so that each part's `/implement` fits one context window. The parts restate the comments above that still hold, and add rules this ticket left open: the Sender holds back a UID in Conflict, equal versions raise no Conflict, one choice (User's Version or Server Version) serves all three kinds, and resolving against a replaced copy fails. Work from the parts; this file stays for its history.
- 2026-11-09 (grilling of 133 to 137, ADR 0014's amendment of 2026-11-09): the parts were rewritten, and 138 (a local change can end a Conflict) split from 133. Changed against this ticket: the User's Version stays in the Event Rows and the Sender holds back a UID in Conflict; the common version is the Last Agreed Version (new in `GLOSSARY.md`), kept from the Mirror rather than read from the Change Journal; an Event in Conflict stays editable; only "Server version" can be undone, as "Your version" changes no Event Rows; "Your version" can't be chosen while the Calendar is Server read-only; the Signal is "Conflicts changed" (spec amended), not "Problems changed".
