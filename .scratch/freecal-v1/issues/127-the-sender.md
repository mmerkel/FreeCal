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

**Status:** done

**Effort:** M

- [x] Creates, edits, moves and deletes of single Events in a writable CalDAV Calendar reach the server with conditional requests (ETags), and a success updates the Mirror and clears the Unsent Changes
- [x] An update or delete without a stored ETag waits; a response without an ETag leaves the next Incoming Sync to fetch the server's text
- [x] Before each change is sent, it is in the Change Journal as ticket 69 records it (previous version, hash of the new one)
- [x] Tested against Radicale

## Comments
- 2026-11-08 (/implement): done. The CalDAV Sender is `core/src/sync/caldav/sender.rs`, reached through `sync::outgoing` (`core/src/sync/mod.rs`). Tests: `core/tests/caldav_send.rs` against Radicale, plus unit tests in `sender.rs`.
  - **Outgoing Sync first (decision).** `Core::sync_calendar` runs the Outgoing Sync, then the Incoming Sync, so the name stays: it runs both. Sending first gets the user's changes to the server soonest. The Incoming Sync right after it also fetches, in the same sync, what the server stored for a write answered without an ETag, and it reads the server change behind a failed precondition. The cost: a server change since the last sync makes a write fail its precondition, a request in vain. Running the Incoming Sync first would have read the new version beforehand.
  - **Failures.** A failure about the whole Account (offline, login) stops the sync. Any other failure of one UID leaves its Unsent Changes in place. The Sender still sends the other UIDs, and the Incoming Sync still runs. The failure is returned afterwards, so the Calendar shows a Sync Error, and that sync's Incoming Journal Transaction isn't returned. A 403 or other refusal therefore already becomes the Calendar's Sync Error, unwritten and untested as such: ticket 124 decides it.
  - **A failed precondition and a 404 are the same thing:** the server changed. The Unsent Change stays, with no Sync Error.
  - **Rules beyond the boxes:**
    - **Nothing sent before the First Sync finishes:** a Calendar whose First Sync is unfinished sends nothing, as the Mirror doesn't yet hold every Server Object.
    - **No text, no request:** a UID whose Event Rows build the Server Object's text unchanged is dropped without a request. An undo of a change not yet sent is one such case.
    - **Unbuildable resource:** a UID whose resource can't be built (ticket 122's `UnreadableEventTime`) writes a Log line and waits. That is a problem of its Events, not a Sync Error.
    - **Change during the send:** a change made while its UID is being sent stays unsent. The Change Journal removes a UID's Unsent Changes only while its Event Rows still hash as they did when read (`journal::sending` / `journal::sent`).
    - **Weak ETags:** a weak ETag in a `PUT` answer counts as none.
    - **New resource names:** a new resource is named `<UID>.ics` when the UID is at most 100 characters of `[A-Za-z0-9._-]`. Otherwise it is named after the UID's SHA-256 in hex. One UID always gets the same name, so a create sent again after a crash fails its precondition instead of making a second resource.
  - **Open, for 124:**
    - **Blocked name:** if another resource holding another UID already sits at that name, the create fails its precondition on every sync, silently. "Not yet synced: <reason>" should cover this.
    - **Created href:** the Mirror's href for a create is the path FreeCal `PUT` to, not one the server gave. This holds on Radicale and on servers that store where they are `PUT`.
  - **Not done here:**
    - **Propagate overwrites unsent edits:** until tickets 20 and 21, Propagate still overwrites Event Rows with Unsent Changes. So after a failed precondition, or an update waiting for an ETag, the Incoming Sync of the same sync replaces the user's edit with the server's version. The tests check only that the server isn't overwritten.
    - **GLOSSARY and ADR 0015 wording:** both say the Sender compares "each Combined Event Key's Event Rows with its Server Object". It does so per UID, as this ticket says.
  - **New terms:**
    - **Sent Event Rows** (new term, `journal::SentEventRows`): one Calendar and UID's Event Rows as the Sender read them to send them, each Event Version by its hash.
    - Code names: `Precondition` (what a write of a Server Object is conditional on: nothing there, or the ETag the Mirror holds), `WriteOutcome` with `ServerChanged` (how a conditional write went: done, or its precondition failed or the object is gone), `Put` (the href and strong ETag of what a server took), `ToSend` (one Calendar and UID whose Unsent Changes are sent), `Mirrored` (what the Mirror takes after a send: nothing sent, a Server Object written, or deleted), `unsent_uids`, `new_resource_name`, `strong_etag`, `refusal`.
- 2026-11-08: New terms approved: Sent Event Rows, and the code names `Precondition`, `WriteOutcome` (`ServerChanged`), `Put`, `ToSend` and `Mirrored`.
