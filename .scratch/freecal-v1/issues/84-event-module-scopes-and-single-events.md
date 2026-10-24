# 84: The Event module takes over recurrence scopes and single Events

**What to build:** Ticket 83's `event/` module becomes the only writer of Event changes the user makes: edits in a recurrence scope move into it from `scope.rs`, and creating, editing and rescheduling a single Event move into it from `lib.rs`, so `Core` no longer branches on whether an Event recurs. A refactor: behaviour stays the same. Decided in an architecture review and its grilling on 2026-10-24.

**Blocked by:** 83

**Status:** done

**Effort:** M

- [x] `scope.rs` becomes `event/scope.rs`. Its private `Recurring` and `Opened` are replaced by ticket 83's loaded value; `scope::cancel` and cancelling an Occurrence become one, and so do `carry_exceptions` and taking an incoming version where both pair Exceptions by recurrence ID. `Scope` stays re-exported from `lib.rs`
- [x] A single Event is the one-Occurrence case: `Core::edit_event` and `Core::reschedule_event` lose their `recurring()` branch, and a single Event's edit is `alone` without a Recurring Event. One rule for keeping an unchanged description as stored (today `description_of` in `lib.rs` and `description_text` in `scope.rs`) and for dropping a pinned zone when the time changes
- [x] `Core::create_event` goes through the module, so turning the editor's `Repeat` into a rule (`rule_text`) and refusing one that gives no Occurrence (`InvalidRepeat`) are private to it
- [x] Outside the module, `event/rule.rs` offers only the span and the imported rule (ticket 83's first box): the functions `scope.rs` and `lib.rs` use become private to the module, and `Loaded::into_parts` and `Opened::into_loaded` go
- [x] `Core` keeps the lock, the `journal::Unit`, Signals and the conversion for the window
- [x] The Core tests pass unchanged; `FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` gives no diff

Out of scope: new recurrence features (tickets 73 to 75), which build on this.

## Comments

**2026-10-24, implementation.** Done; `cargo test`, clippy and the generated types are clean, and no test changed.

- `scope.rs` is `event/scope.rs` and works on `Loaded`: its `Recurring` is gone, and the Event a change makes (the second half of a split, or the whole after "all") is a `Loaded` with no Exceptions yet. Its `Opened` became `Changing`, what is being changed (a stored Event or an Occurrence the rule gives); that isn't a loaded value, so it stays.
- `carry_exceptions` and `Loaded::take` share `Loaded::write_exception` (replace, or delete and create under another UID). They still find their pairs differently: `take` pairs incoming Exceptions with stored ones by recurrence ID, `carry_exceptions` moves each stored Exception to its moved recurrence ID. `Loaded::exception_at` and `same_recurrence_id` are the only comparisons.
- `carry_exceptions` uses the Exceptions loaded up front instead of loading them again; the writes before it only touch the Recurring Event.
- Behaviour change on inconsistent data only: moving one Occurrence to another Calendar now cancels it through `Loaded::cancel`, as deleting an Occurrence does, which deletes every Exception for that Occurrence, not only the one edited, should a Calendar have two.
