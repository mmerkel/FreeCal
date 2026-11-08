# 124: Changes are sent as they're made, and a refused write is a Sync Error

**What to build:** The fifth part of ticket 17 (read it for the history). A change in a writable server Calendar is applied to the Local Store at once, as now, and then sent straight away by an Outgoing Sync of its Calendar (ticket 127's Sender), not only at the next sync. Sends that fail are kept apart by why.

- After a change, the core asks the Sync Scheduler for an Outgoing Sync of the changed Calendars while the Account is online; offline replay is ticket 18.
- A server refusing writes to a Calendar (for example 403, or privileges lost) becomes a Sync Error on that Calendar (spec: "Sync Error"); its Unsent Changes stay.
- Decide here what an Event shows when its send fails, "Not yet synced: <reason>": what reasons there are and where the reason is kept (`unsent_change` has no reason column yet). Ticket 18 shows it.

Spec: "Sync Error"; "Event details".

**Blocked by:** 15, 127, 128

**Status:** done

**Effort:** M

- [x] A change in a writable server Calendar is sent right after it is made while the Account is online
- [x] A server refusing writes to a Calendar becomes a Sync Error on that Calendar
- [x] The reason a send failed is kept for "Not yet synced: <reason>" (shown by ticket 18)
- [x] Core tests with a controllable clock, and tested against Radicale

## Comments
- 2026-11-08 (/implement 128): a sent DELETE of a UID that Ignored Resources also hold hands the UID on only at the next Incoming Sync (ADR 0015's amendment for ticket 128). An Outgoing Sync run right after a change, without an Incoming Sync, leaves the Event gone until the next sync. Decide whether such a send runs the Calendar's Incoming Sync too.
- 2026-11-08 (/implement): done. Tests: `core/tests/caldav_send.rs` against Radicale (modules `sent_as_made` and `writes_refused`), unit tests in `core/src/sync/caldav/sender.rs` (Send Failures, with a fake server) and `core/src/caldav.rs` (`write_refusal`).
  - **A change syncs its Calendars alone, both ways (decision).** After a change, the core asks the Sync Scheduler for a sync of the changed Calendars only, not their whole Account (`SyncScheduler::ask_calendar`, `Core::after_local_change`). Like every sync, it runs the Outgoing Sync and then the Incoming Sync. This answers the comment from 128: a handed-on UID shows right after its DELETE is sent, and a write answered without an ETag is fetched at once. One kind of sync stays the only kind. The cost: each change also costs a privileges `PROPFIND` and a `REPORT`, and an Incoming Sync may meet the Mass-Change Guard (ticket 22).
  - **Offline:** while the Account is Offline, a change asks for no sync. The next Poll, or the network's return, sends it (ticket 18). The Offline test moves the controllable clock to the next Poll.
  - **Account's Sync Record:** a sync of some Calendars still starts and ends the Account's Running Sync, so the Account's last sync time moves. The overall status's time is the oldest of the Accounts' and Calendars' last syncs, so an unsynced Calendar still holds it back.
  - **Refused writes (decision).** A write answered 403 is the new error `WritesRefused` ("the server doesn't allow changes to this calendar"), the Calendar's Sync Error. It stops the Calendar's Outgoing Sync, the Unsent Changes stay, and the Incoming Sync still runs.
    - **Not about the Calendar:** a 403 whose body is a `DAV:error` naming a CalDAV precondition, such as `valid-calendar-data`, is about that one resource. It stays `ServerRefused` (`write_refusal`, `core/src/caldav.rs`).
    - **Lost privileges:** a Calendar its server reports read-only while Unsent Changes of it wait is `WritesRefused` too, with no request sent. The user's own Read-only mark alone is no Sync Error, and the changes wait.
    - **Open for the user:** a Calendar both marked read-only by the user and reported read-only by the server still shows the Sync Error. The Local Store keeps both flags, but the Calendar the Sender reads (`Store::find_calendar`) merges them into `read_only`, so the Sender checks the server's flag first.
  - **Not yet synced: <reason> (decision, for ticket 18).** A Send Failure is kept per UID in `unsent_change.send_failure` (migration 4), as JSON of `SendFailure` (`core/src/sync/mod.rs`). The Sender overwrites it on each try, and it goes with the Unsent Changes once they are sent.
    - **Stored reasons, about the UID itself:**
      - the server changed it (an update or delete failed its precondition; a Conflict once tickets 20 and 21 land)
      - its address is taken (a create failed its precondition; this covers 127's "blocked name")
      - waiting for the server's version (no ETag)
      - its Events can't be built
      - the request failed with an error code
    - **Reasons read off the Sync Status, not stored:** Offline, the Calendar's Sync Error (writes refused, paused), the Read-only mark, First Sync unfinished.
    - **Which reason to show:** ticket 18 should show the Account's or Calendar's reason first, as a stored one may be older. An Unsent Change never tried has no reason ("Not yet synced"). A change made after a failed try adds an Unsent Change without one, beside the UID's older ones.
    - **Not exposed yet:** no interface call returns the reason. Ticket 18 adds the reader; the tests' `Store::send_failures` and `SendFailure::from_stored` are test-only until then.
  - **New terms:**
    - **Send Failure** (new term, `SendFailure`, `unsent_change.send_failure`): why the Sender's last try to send a UID's Unsent Changes didn't send them, kept with them until a send succeeds.
    - **Writes Refused** (new term, `ErrorCode::WritesRefused`): the server doesn't let the user change a Calendar's Events while Unsent Changes of it wait.
    - Code names:
      - `SendFailure` variants: `ServerChanged`, `AddressTaken`, `WaitingForServerVersion`, `Unbuildable`, `Failed`
      - `journal::not_sent`: records a Send Failure
      - `DatabaseTransaction::set_send_failure`
      - `write_refusal`: classifies a 403 answer to a write
      - `beyond_uid`: a failure about the Account or Calendar, not the UID
      - `SyncScheduler::ask_calendar`, `SyncRequests::calendars_of`, `Core::sync_account_calendars`: a sync of some Calendars of an Account
- 2026-11-08: the user approved syncing both ways after every change.
- 2026-11-08: the user decided the open point on read-only Calendars: keep it as built. A Calendar the server reports read-only shows the Sync Error while Unsent Changes wait, even if the user marked it read-only too.
