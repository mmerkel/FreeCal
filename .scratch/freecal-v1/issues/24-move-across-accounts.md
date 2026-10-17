# 24: Move Events across Accounts

**What to build:** The user can move an Event to a Calendar in another Account, for example turning a local draft into a real server Event. FreeCal warns first only when the Event has Provider-specific details the target can't hold.

**Blocked by:** 17

**Status:** ready-for-agent

**Effort:** M

- [ ] Changing an Event's Calendar to one in another Account creates it in the target and then deletes it from the source
- [ ] Both steps are recorded in the Change Journal where they touch a server
- [ ] The warning appears only when details would be lost, and the move is skipped if the user cancels

## Comments

- 2026-10-11: An Event whose zone was pinned (ADR 0007) is written to the target with the pinned zone made explicit, since the target Calendar may read floating times differently.
