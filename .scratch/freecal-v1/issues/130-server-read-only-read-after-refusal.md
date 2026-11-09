# 130: The server's privileges are read again only after a refused write, and only they hold back Unsent Changes

**What to build:** Ticket 120 made each sync of a server Calendar start by reading whether the server lets the user change it: a Depth-0 `PROPFIND` of its `current-user-privilege-set` (`Remote::server_read_only`, called from `Core::sync_calendar`, `core/src/lib.rs`). That doubles a Poll's requests for an unchanged Calendar on a server with `sync-collection`, one more round trip per Calendar on every sync. Read Server read-only again only when FreeCal has a reason to think it changed:

- **Revoked:** a write the server refuses for lack of privileges (`CoreError::WritesRefused`, from `write_refusal`, `core/src/caldav.rs`) re-reads the Calendar's privileges, sets Server read-only, and sends "Calendars changed" when it changed. The refused write stays the Calendar's Sync Error, as now (ticket 124).
- **Granted:** a Server read-only Calendar sends no writes, so no refusal tells FreeCal that the server granted the permission again. A sync the user starts (`StartedBy::User`, "Sync now") and the syncs at launch re-read the privileges of each Server read-only Calendar, and only of those. The trade-off: Polls alone never notice a grant, so the Calendar stays read-only until the user syncs by hand or FreeCal starts again; in return a Poll never pays for it, which matters for subscribed and shared Calendars that are read-only for good.
- Adding a Calendar still reads its privileges, as now (`discover`).
- Unsent Changes of a Server read-only Calendar still fail with `WritesRefused` without a request, as built.
- **The user's Read-only mark only stops new changes.** It decides, with Server read-only, whether the user may change a Calendar's Events now (`JournalTransaction::writable`, `core/src/journal/mod.rs`), and nothing else. Unsent Changes already made are sent whatever the mark: only Server read-only holds them back. Today the CalDAV Sender also holds them back, without a Sync Error, while the user's mark is set (`send`, `core/src/sync/caldav/sender.rs`). That makes odd cases: a change made, then the Calendar marked, waits with the Calendar showing "Synced", can't be undone (undo is refused in a Read-only Calendar), and keeps the User's Version of its Events against every server change until the mark is removed; and when the server also revokes the permission, removing the mark changes nothing visible, so the outcome depends on the order of the edit, the mark and the server's change. The trade-off: the mark no longer pauses sending, so a change the user regrets must be undone before marking. One rule for the mark (it stops changes, never sends) beats one with two jobs.
- Update the docs to match: the Sender's doc comment; the spec's "Writability" (the user's mark stops new changes; Unsent Changes made before it are still sent); and the `GLOSSARY.md` definition of Read-only Calendar, whose "FreeCal never changes its Events" no longer holds for changes made before the user marked it.

Spec: US-17, US-19, US-30; "Writability"; "Calendars changed".

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

- [ ] A Poll of a server Calendar sends no privilege request
- [ ] A write refused for lack of privileges sets Server read-only and sends "Calendars changed"
- [ ] A permission the server grants again is noticed by a sync the user starts and at launch, for Server read-only Calendars only
- [ ] Unsent Changes made before the user marked a Calendar read-only are sent at its next sync; the Sender looks only at Server read-only (replacing `unsent_changes_of_a_read_only_calendar_wait` and `unsent_changes_of_a_calendar_the_user_made_read_only_are_no_sync_error`, `core/tests/caldav_send.rs`)
- [ ] Sync tests against Radicale through `RewritingProxy` for a revocation met by a write and a grant met by a sync the user starts (replacing `each_sync_reads_whether_the_server_lets_the_user_change_the_calendar`, `core/tests/caldav_sync.rs`), and a test that a Poll sends no privilege request

## Comments
- 2026-11-08 (/simplify since ticket 120): found by the efficiency review; the user chose to re-read only after a refused write.
- 2026-11-08: the user chose the recommendation for a granted permission: re-read on a sync the user starts and at launch, for Server read-only Calendars only.
- 2026-11-09: the user chose to fold in the rule that only Server read-only holds back Unsent Changes; the user's Read-only mark only stops new changes (from `TODO.md`).
