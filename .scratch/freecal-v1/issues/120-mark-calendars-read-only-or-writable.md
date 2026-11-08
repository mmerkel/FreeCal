# 120: The user marks Calendars read-only or writable

**What to build:** The first part of ticket 17 (read it for the history). The user can mark any Calendar read-only in FreeCal and switch a server Calendar from read-only to writable; a Calendar the server reports read-only stays read-only whatever the user's mark (spec: "Writability", US-17, US-19, US-30). Before 1.0, newly added server Calendars still start marked read-only (US-18).

- A core interface call sets the user's mark (`calendar.read_only`); `server_read_only` is the server's and the user can't change it. Each change sends "Calendars changed".
- The core reads the server's privileges only when a Calendar is added. Now that the user can unmark Calendars, re-read `current-user-privilege-set` on each sync and send "Calendars changed" when `server_read_only` changes (spec: "Calendars changed").
- The frontend offers the mark where Calendars are managed in the sidebar, and doesn't offer "writable" for a Calendar the server reports read-only, saying why.
- Tests unmark Calendars with `make_writable` in `core/tests/support/mod.rs`; replace it with this call.

Spec: US-17, US-18, US-19, US-30; "Writability"; "Calendars changed".

**Blocked by:** 56

**Status:** ready-for-agent

**Effort:** M

- [ ] The user can set any Calendar read-only or writable in FreeCal; server-reported read-only still wins
- [ ] Each sync re-reads the server's privileges and sends "Calendars changed" when they change
- [ ] `make_writable` is gone; tests use the interface call
- [ ] Core tests, a component test for the sidebar's mark, and a sync test against Radicale for a privilege the server changes

## Comments
