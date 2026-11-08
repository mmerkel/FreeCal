# 127: The CalDAV Sender sends a Calendar's Unsent Changes

**What to build:** The first part of ticket 123, itself the fourth part of ticket 17 (read both for the history). An Outgoing Sync: the CalDAV Sender (`core/src/sync/caldav/sender.rs`), reached through the Sync Engine's entry in `core/src/sync/mod.rs` as the Incoming Sync is, sends a writable CalDAV Calendar's Unsent Changes. This ticket runs it as part of every sync of the Account; ticket 124 also runs it right after each change. What follows a DELETE of a UID that an Ignored Resource also holds is ticket 128.

- The Remote gets the writes: PUT and DELETE of a resource, conditional, returning the new ETag if the server gives one.
- Unsent Changes keep their recurrence IDs as obtained and are merged per UID, since CalDAV sends one resource per UID. The Sender finds the CalDAV Server Object by Calendar and UID in `caldav_server_object`: Event Rows with a Server Object is a PUT of ticket 122's text with `If-Match` on the Mirror's ETag; Event Rows without one a create with `If-None-Match: *`; a Server Object without Event Rows a DELETE with `If-Match`.
- An Unsent Change can match neither Event Rows nor a Server Object, for example an Event's previous Combined Event Key after its recurrence ID changed, or after an import joined an Exception (ticket 92's comment). The Sender drops it.
- An update or a delete is sent only with `If-Match` on the Mirror's ETag. If the Server Object has no stored ETag, the Unsent Change waits until the server's version reads, and the Event stays not yet synced (ADR 0015's amendment, ticket 109).
- A success updates the Mirror and removes the UID's Unsent Changes. A response without an ETag (RFC 4791 allows it when the server changed the text) leaves the Mirror holding the text sent without an ETag, and the next Incoming Sync fetches what the server stored.
- A server change met while sending (a failed precondition) leaves the Unsent Change in place; finding and holding it is tickets 20 and 21 (ADR 0014), not the Sender's.
- Decide whether a sync's Outgoing Sync runs before or after its Incoming Sync, name the trade-off and record it in a comment. `Core::sync_calendar` runs only an Incoming Sync today; rename it to say which direction it runs, or keep the name if it runs both (ticket 96's comment).
- Every change is already in the Change Journal before it is sent (ticket 69); check it with a test, as the Sender adds no Journal Transaction.

Spec: "Untrusted content". ADRs 0011, 0014, 0015.

**Blocked by:** 69, 91, 92, 97, 100, 122

**Status:** ready-for-agent

**Effort:** M

- [ ] Creates, edits, moves and deletes of single Events in a writable CalDAV Calendar reach the server with conditional requests (ETags), and a success updates the Mirror and clears the Unsent Changes
- [ ] An update or delete without a stored ETag waits; a response without an ETag leaves the next Incoming Sync to fetch the server's text
- [ ] Before each change is sent, it is in the Change Journal as ticket 69 records it (previous version, hash of the new one)
- [ ] Tested against Radicale

## Comments
