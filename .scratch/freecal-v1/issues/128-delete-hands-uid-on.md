# 128: A sent DELETE hands its UID on to an Ignored Resource

**What to build:** The second part of ticket 123, itself the fourth part of ticket 17 (read both for the history). When the CalDAV Sender's DELETE (ticket 127) removes a UID's Corresponding Server Object and an Ignored Resource of the Calendar holds that UID (an Ambiguous UID), that resource takes over as after an Incoming Sync (ADR 0015): the Ignored Resources holding the UID are fetched, the first by the fixed order is mirrored as the Corresponding Server Object, and its Events take the Event Rows' place. The Event comes back, as it does in other clients. If none can be read, the UID stays without Event Rows.

- The takeover goes through the Incoming Sync's code for it (`keep` and its callers, `core/src/sync/caldav/incoming.rs`), not a second copy, so that one rule decides which resource keeps a UID.
- The Sender doesn't delete the Ignored Resources.
- The Events that come back are written by the Sync Engine, as an Incoming Sync writes them, so they are no Unsent Changes.

Spec: "Untrusted content". ADRs 0011, 0015.

**Blocked by:** 127

**Status:** ready-for-agent

**Effort:** S

- [ ] A DELETE hands its UID on to the first Ignored Resource holding it by the fixed order, whose Events then show
- [ ] With no readable Ignored Resource holding the UID, the DELETE leaves no Event Rows for it
- [ ] Tested against Radicale

## Comments
