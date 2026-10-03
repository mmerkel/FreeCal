# 20: Conflicts on single Events

**What to build:** When a local change and a newer server change to the same Event can't both be kept, FreeCal holds a Conflict instead of guessing. The Event carries a Conflict badge, the user can list all open Conflicts, see both versions and choose.

**Blocked by:** 17

**Status:** ready-for-agent

- [ ] Edit/edit: both versions shown, the user chooses one
- [ ] Local edit vs remote delete: "restore with my changes" or "accept the delete"
- [ ] Local delete vs remote edit: "delete anyway" or "keep the server version"
- [ ] The Conflict badge is the only badge on an Event chip
- [ ] A list of all open Conflicts; resolving one applies the choice and records it in the Change Journal when it writes to the server
- [ ] The core sends "Problems changed" when a Conflict is raised or resolved
- [ ] Written test-first against Radicale
