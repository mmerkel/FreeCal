# 128: A sent DELETE hands its UID on to an Ignored Resource

**What to build:** The second part of ticket 123, itself the fourth part of ticket 17 (read both for the history). When the CalDAV Sender's DELETE (ticket 127) removes a UID's Corresponding Server Object and an Ignored Resource of the Calendar holds that UID (an Ambiguous UID), that resource takes over as after an Incoming Sync (ADR 0015): the Ignored Resources holding the UID are fetched, the first by the fixed order is mirrored as the Corresponding Server Object, and its Events take the Event Rows' place. The Event comes back, as it does in other clients. If none can be read, the UID stays without Event Rows.

- The takeover goes through the Incoming Sync's code for it (`keep` and its callers, `core/src/sync/caldav/incoming.rs`), not a second copy, so that one rule decides which resource keeps a UID.
- The Sender doesn't delete the Ignored Resources.
- The Events that come back are written by the Sync Engine, as an Incoming Sync writes them, so they are no Unsent Changes.

Spec: "Untrusted content". ADRs 0011, 0015.

**Blocked by:** 127

**Status:** done

**Effort:** S

- [x] A DELETE hands its UID on to the first Ignored Resource holding it by the fixed order, whose Events then show
- [x] With no readable Ignored Resource holding the UID, the DELETE leaves no Event Rows for it
- [x] Tested against Radicale

## Comments
- 2026-11-08 (/implement): done. Tests: `core/tests/caldav_send.rs` against Radicale (`a_delete_hands_its_uid_on_to_the_first_ignored_resource_holding_it`, `a_delete_with_no_readable_ignored_resource_holding_its_uid_leaves_no_event_rows`).
  - **How (decision, recorded in ADR 0015's amendment).** The Sender's DELETE sends no further request. In the Database Transaction that removes the Server Object from the Mirror (`deleted_by_sender`, `core/src/sync/caldav/mod.rs`), it forgets the ETags of the Ignored Resources holding the UID. The Incoming Sync fetches every Known Server Resource without an ETag, so the next one fetches them, and its `keep` hands the UID to the first by the fixed order. A sync of the Account runs that Incoming Sync right after the Outgoing Sync. The trade-off: the takeover is kept in the Local Store, so it still happens if FreeCal stops or the Incoming Sync fails first, and only the Incoming Sync's rule decides which resource keeps a UID. The cost: until that Incoming Sync, the Event is gone (see the comment on ticket 124). An ETag-less Known Server Resource now also means "fetch it for a takeover", not only "unreadable".
  - **Side effects:**
    - The Ignored Resources that don't take over write their "Ignored" Log line again, naming the resource keeping the UID now.
    - An Ignored Resource that turned unreadable on the server after the last Incoming Sync, and whose UID is then handed on, writes no "Couldn't read" Log line. Its ETag is already gone, which is how FreeCal tells a resource that already was unreadable. It is still fetched on every sync until it reads.
  - **Not tested:** a takeover after the Incoming Sync of the same sync failed. The Local Store keeps the forgotten ETags, so the next sync fetches them.
  - **New terms:**
    - **Hand on a UID** (new term): after the Sender deletes the Server Object keeping a UID, let an Ignored Resource holding it take it over at the next Incoming Sync.
    - Code names: `deleted_by_sender` (removes a Server Object the Sender deleted from the Mirror and hands its UID on), `DatabaseTransaction::forget_etags_holding` (forgets the ETags of the Ignored Resources holding a UID, `core/src/store/sql/mirror/caldav.rs`).
