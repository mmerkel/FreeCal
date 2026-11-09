# 132: The Sync Engine syncs a Calendar both ways

**What to build:** The core decides how a server Calendar syncs (`Core::sync_calendar`, `core/src/lib.rs`):
- the Outgoing Sync comes first, then the Incoming Sync, which runs even after a failed send;
- which error wins: a failure about the whole Account first, then the send's error once the Incoming Sync is done;
- when the server's privileges are read again: before sending, when looking for grants on a Server read-only Calendar, and after a refused write (ticket 130).

The core also passes the Sender a `server_read_only` flag, and the flags `look_for_grants` and `server_read_only` travel through `sync_account_calendars → sync_calendars → sync_calendar → sync::outgoing → caldav::sender::send`. Meanwhile the Sync Engine's own entry points `sync::incoming` and `sync::outgoing` (`core/src/sync/mod.rs`) only `match` on the Provider. Give the Sync Engine one entry that syncs a Calendar both ways and owns these rules. From the architecture review of 2026-11-09 (candidate 2).

- **The entry:** one Provider-neutral function in `core/src/sync/mod.rs`, replacing `sync::incoming` and `sync::outgoing`. It owns the order, which error wins, the check whether the Outgoing Sync may send, and the re-read of privileges. Each Provider's Incoming Sync and Sender stay behind an exhaustive `match` on the Provider.
- **May the Outgoing Sync send?** Decided in the Provider-neutral entry, before the Provider's Sender: while the stored Calendar is Server read-only and has Unsent Changes, it sends nothing, and the Calendar's Sync Error is `WritesRefused`, as now. The user's mark plays no part (ticket 130). The CalDAV Sender loses its `server_read_only` parameter.
- **Re-reading the server's privileges** moves into the entry, Provider-neutral. The core passes the facts: who started the sync, and whether it is the launch sync. The entry applies both triggers and saves Server read-only (`set_server_read_only`):
  - **look for grants:** before sending, for a Server read-only Calendar, on a sync the user starts or the launch sync;
  - **after a refused write:** to notice a revoked permission.
  A failed re-read after a refusal leaves `WritesRefused` the Sync Error unless it is about the whole Account, as now. Trade-off, named: the re-read policy exists because reading a CalDAV Calendar's privileges costs one request per Calendar, so this puts a CalDAV cost rule in Provider-neutral code. With one Provider, a per-Provider policy would be a hypothetical seam. Ticket 26 may move it into the Provider if Google's Calendar list already carries the access level.
- **Signals stay with the core.** The entry's result carries the Calendar's Journal Transaction, its outcome (success or error), and whether Server read-only changed. Changed and error can both hold, as with a refused write that sets Server read-only. The core sends "Calendars changed" from that, and the other Signals as now.
- **The core keeps** the Running Syncs, the loop over an Account's Calendars and its stop on a failure about the whole Account, the Sync Status and the Signals.
- **Out of scope:** the check whether a failure is about the whole Account (`sync_status::about_account`) stays one function, asked by both the core and the Sync Engine.
- No change to what the user sees or to the core interface. The ticket-130 tests through `RewritingProxy` (`core/tests/caldav_sync.rs`) and the send tests pass unchanged.

Spec: US-17, US-30; "Sync engine"; "Writability"; "Calendars changed".

**Blocked by:** 131

**Status:** ready-for-agent

**Effort:** M

- [ ] One entry in `core/src/sync/mod.rs` syncs a Calendar both ways; `sync::incoming` and `sync::outgoing` are gone; `Core::sync_calendar` only starts it and sends Signals from its result
- [ ] The entry decides whether the Outgoing Sync may send; the CalDAV Sender takes no `server_read_only`
- [ ] The re-read of privileges (looking for grants, and after a refused write) is the entry's; `look_for_grants` and `server_read_only` no longer travel through the core's sync functions
- [ ] "Calendars changed" is sent from the entry's result, also when the sync fails with `WritesRefused`
- [ ] The existing CalDAV, send and Sync Status tests pass unchanged

## Comments
- 2026-11-09 (/improve-codebase-architecture): the user chose: one Sync Engine entry owns order, error precedence, the "may it send?" check and the privilege re-read; the "may it send?" check and the re-read are Provider-neutral for now; the core sends "Calendars changed" from the entry's result; `about_account` is out of scope. Blocked by 131 because both change how Server read-only is read.
- For ticket 26 (Google): check whether Google needs its own privilege policy; if so, move the re-read into each Provider's Sync Engine.
