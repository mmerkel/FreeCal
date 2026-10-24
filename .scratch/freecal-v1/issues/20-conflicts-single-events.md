# 20: Conflicts on single Events

**What to build:** When a local change and a newer server change to the same Event can't both be kept, FreeCal holds a Conflict instead of guessing. The Event carries a Conflict badge, the user can list all open Conflicts, see both versions and choose.

**Blocked by:** 17, 87

**Status:** ready-for-agent

**Effort:** L

- [ ] Edit/edit: both versions shown, the user chooses one
- [ ] Local edit vs remote delete: "restore with my changes" or "accept the delete"
- [ ] Local delete vs remote edit: "delete anyway" or "keep the server version"
- [ ] The Conflict badge is the only badge on an Event chip
- [ ] A list of all open Conflicts; resolving one applies the choice and records it in the Change Journal when it writes to the server
- [ ] The core sends "Problems changed" when a Conflict is raised or resolved
- [ ] The Conflict shows what each side changed against the common version from the Change Journal ("you changed the time, they changed the title")
- [ ] Resolving is a Change Journal unit that keeps the losing version, so a wrong choice can be undone
- [ ] Written test-first against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-24: Per ADR 0010, a server change to an object that is pending (changed in FreeCal, not sent yet) is a Conflict, never a silent replacement; today sync takes it. This includes an undone pulled change waiting to be sent.
