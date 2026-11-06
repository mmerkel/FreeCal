# 20: Conflicts on single Events

**What to build:** When a local change and a newer server change to the same Event can't both be kept, FreeCal holds a Conflict instead of guessing. The Event carries a Conflict badge, the user can list all open Conflicts, see both versions and choose.

**Blocked by:** 17, 91, 92, 100

**Status:** ready-for-agent

**Effort:** L

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
