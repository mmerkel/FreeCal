# 84: The Event module takes over recurrence scopes and single Events

**What to build:** Ticket 83's `event/` module becomes the only writer of Event changes the user makes: edits in a recurrence scope move into it from `scope.rs`, and creating, editing and rescheduling a single Event move into it from `lib.rs`, so `Core` no longer branches on whether an Event recurs. A refactor: behaviour stays the same. Decided in an architecture review and its grilling on 2026-10-24.

**Blocked by:** 83

**Status:** ready-for-agent

**Effort:** M

- [ ] `scope.rs` becomes `event/scope.rs`. Its private `Recurring` and `Opened` are replaced by ticket 83's loaded value; `scope::cancel` and cancelling an Occurrence become one, and so do `carry_exceptions` and taking an incoming version where both pair Exceptions by recurrence ID. `Scope` stays re-exported from `lib.rs`
- [ ] A single Event is the one-Occurrence case: `Core::edit_event` and `Core::reschedule_event` lose their `recurring()` branch, and a single Event's edit is `alone` without a Recurring Event. One rule for keeping an unchanged description as stored (today `description_of` in `lib.rs` and `description_text` in `scope.rs`) and for dropping a pinned zone when the time changes
- [ ] `Core::create_event` goes through the module, so turning the editor's `Repeat` into a rule (`rule_text`) and refusing one that gives no Occurrence (`InvalidRepeat`) are private to it
- [ ] Outside the module, `event/rule.rs` offers only the span and the imported rule (ticket 83's first box): the functions `scope.rs` and `lib.rs` use become private to the module, and `Loaded::into_parts` and `Opened::into_loaded` go
- [ ] `Core` keeps the lock, the `journal::Unit`, Signals and the conversion for the window
- [ ] The Core tests pass unchanged; `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` gives no diff

Out of scope: new recurrence features (tickets 73 to 75), which build on this.
