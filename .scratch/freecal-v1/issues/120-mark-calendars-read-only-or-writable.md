# 120: The user marks Calendars read-only or writable

**What to build:** The first part of ticket 17 (read it for the history). The user can mark any Calendar read-only in FreeCal and switch a server Calendar from read-only to writable; a Calendar the server reports read-only stays read-only whatever the user's mark (spec: "Writability", US-17, US-19, US-30). Before 1.0, newly added server Calendars still start marked read-only (US-18).

- A core interface call sets the user's mark (`calendar.read_only`); `server_read_only` is the server's and the user can't change it. Each change sends "Calendars changed".
- The core reads the server's privileges only when a Calendar is added. Now that the user can unmark Calendars, re-read `current-user-privilege-set` on each sync and send "Calendars changed" when `server_read_only` changes (spec: "Calendars changed").
- The frontend offers the mark where Calendars are managed in the sidebar, and doesn't offer "writable" for a Calendar the server reports read-only, saying why.
- Tests unmark Calendars with `make_writable` in `core/tests/support/mod.rs`; replace it with this call.

Spec: US-17, US-18, US-19, US-30; "Writability"; "Calendars changed".

**Blocked by:** 56

**Status:** done

**Effort:** M

- [x] The user can set any Calendar read-only or writable in FreeCal; server-reported read-only still wins
- [x] Each sync re-reads the server's privileges and sends "Calendars changed" when they change
- [x] `make_writable` is gone; tests use the interface call
- [x] Core tests, a component test for the sidebar's mark, and a sync test against Radicale for a privilege the server changes

## Comments
- 2026-11-08 (implemented): `Core::set_calendar_read_only` sets the user's mark; the sidebar offers it as a "Read-only" checkbox item in each Calendar's ⋮ menu, checked while the Calendar is read-only. For a Calendar the server reports read-only the item is checked, disabled and described by "The server doesn't allow changes to this calendar." Each Calendar's sync starts with a Depth-0 `PROPFIND` of its `current-user-privilege-set` (`Remote::server_read_only`), and "Calendars changed" follows when it changed. A failed read is that Calendar's Sync Error, like any failed request of its sync. Radicale loads its rights file only at startup, so the sync test runs Radicale's real answer through `RewritingProxy`, which turns `<read />` into `<all />` while rewriting.
- Open for the user (from /code-review):
  - While the server reports a Calendar read-only, the user can't change their own mark, and the menu shows the Calendar as read-only, not the mark. If the user had left it writable before the server revoked access, it becomes writable again when the server grants access. Showing and setting the mark separately would need the mark in the interface type `Calendar`.
  - The privilege read sits in `Core::sync_calendar`, not in the Sync Engine, since it writes the `calendar` table (an Unguarded Write) and Core sends the Signals.
- New terms (unapproved): **Read-only mark** (new term), the user's own read-only flag on a Calendar (`calendar.read_only`), as opposed to the server's; **Server read-only** (new term), the server not letting the user change a Calendar's Events (`server_read_only`).
