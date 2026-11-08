# 114: The frontend tests run on the recording fake core, and the simulating one goes

**What to build:** The fake core of the frontend tests (`FakeCore`, `src/test/fakeCore.ts`, about 940 lines) has grown into a partial second core: about 420 lines reimplement the Change Journal and its undo, the Occurrences of a Recurring Event, edits in a recurrence scope (without "this and following"), deleting by UID, import update / skip / copy, and Server Accounts. It can drift from the Rust core with no test noticing. The spec's Seam 2 asks only that user actions produce the right core calls and that the window follows the Signals. Move every frontend test to the recording fake core of ticket 116 and delete `FakeCore`. From the architecture review of 2026-11-07 (candidate 2).

- **One test file at a time**, each move a commit with every test passing: `App.navigation`, `App.settings`, `App.sidebar`, `App.sync`, `App.import`, `App.events`, `Sidebar`, `Sidebar.accounts`.
- **What stays at the App level:** each user flow keeps one test there, checking that the user's action makes the right core call, that a Signal changes what is shown, and that one dialog opens over another. Their variations live only in the Window Model's and Window Dialogs' tests (tickets 117 to 119): which read wins, the Date changed hold, the startup order, the scope prompt's choices and Cancel, the undo's retry.
- **Tests emit the Signals.** A test that checks the window after a change scripts the new answer and emits the Signal the core would send, such as "Occurrences changed".
- **What leaves the frontend tests:** an outcome that only the fake simulated, such as "a Recurring Event is on every date", "Undo undoes the import in one step" or "hiding a Calendar hides its Events". The window's part of each stays: the call made and the re-read on the Signal. Each outcome is dropped only once a test at the core interface (Seam 1) is found that covers it; one without is added there first, in Rust.
- **The special lists and knobs go:** `scopedChanges`, `sidebarSaves`, `settingsUpdates`, `openedLinks`, `fileDialogs`, `holdFirstSyncs`, `importFile`, `caldavServer`, `startupError` and the like become the recorded calls and scripted answers of `RecordingCore`.
- **The hostile test Event** (spec "Testing Decisions") still goes through the grid, its details, the editor, the delete question and the refused drag, as a scripted answer.
- **At the end** `FakeCore` is deleted and `RecordingCore` takes over its name and file (`FakeCore`, `src/test/fakeCore.ts`), the spec's "fake core interface".

Spec: "Testing Decisions" (Seam 2, the hostile test Event).

**Blocked by:** 119

**Status:** done

**Effort:** M

- [x] No frontend test uses the old `FakeCore`; it is deleted, and the recording fake is `FakeCore` in `src/test/fakeCore.ts`
- [x] Nothing in the fake decides what the core would do: no Change Journal, no Occurrences of a Recurring Event, no scope edits, no import rules
- [x] Every outcome dropped from the frontend tests is covered by a named Seam 1 test, listed in this ticket's comments
- [x] The hostile test Event still runs through every path it ran through before
- [x] `npm test` and the pre-commit checks pass

## Comments

- 2026-11-08: Ticket 113 was split into 116 to 119; this ticket now waits on 119, the last of them.
- 2026-11-08: Done. Every frontend test runs on `FakeCore` (`src/test/fakeCore.ts`), the recording fake of ticket 116 under its final name; the simulating fake is deleted. `src/test/answers.ts` holds what the App-level tests share: `firstLaunch` (a FakeCore scripted as a first launch on 2026-10-03, optionally with Accounts, Calendars and Events), `scriptWindow`, `calendarsChanged` and `occurrencesChanged` (script the next reads, then emit the Signal as the core would), and builders for Calendars, Events, Occurrences and CalDAV Accounts.
  - **Outcomes dropped from the frontend tests, with the Seam 1 tests that cover them:**
    - A Recurring Event on every date; a new repeating Event having Occurrences: `recurring_events::a_recurring_event_has_an_occurrence_on_every_date_it_falls_on`, `a_recurring_event_without_an_end_still_has_occurrences_years_later`.
    - "This event" making an Exception, "All events" changing the whole, "This and following": `recurrence_scopes::moving_this_event_changes_only_that_occurrence`, `editing_this_event_again_changes_its_exception_and_keeps_the_rest`, `editing_all_events_from_an_exception_changes_only_what_was_changed`, `editing_this_and_following_events_changes_only_those`.
    - Deleting one Occurrence, or the whole Recurring Event: `recurring_events::deleting_one_occurrence_cancels_it_and_leaves_the_rest_and_can_be_undone`, `deleting_a_recurring_event_deletes_all_its_occurrences`.
    - Hiding a Calendar hides its Events: `events::hidden_calendars_have_no_occurrences`.
    - Created, edited, rescheduled and deleted Events: `events::a_created_event_can_be_read_back_and_is_signalled`, `editing_changes_every_field_and_survives_a_relaunch`, `rescheduling_changes_only_the_time_and_is_signalled`, `deleting_an_event_removes_it_and_is_signalled`.
    - An untouched description kept as it came: `events::an_untouched_description_keeps_its_original_bytes`.
    - Calendars created, renamed, recoloured, deleted, hidden, "Show only this": `local_calendars::a_created_calendar_is_listed_under_its_account_and_shown`, `renaming_a_calendar_changes_only_its_name`, `recolouring_a_calendar_changes_only_its_colour`, `deleting_a_calendar_removes_it`, `hiding_and_showing_a_calendar_is_recorded_and_signalled`, `showing_only_one_calendar_hides_all_others_with_one_signal`.
    - The preview's count of Events already in the Calendar: `import::events_whose_uid_the_calendar_has_count_as_existing`.
    - Import skip and update: `import_run::skip_leaves_events_the_calendar_has_as_they_are`, `update_changes_events_the_calendar_has_and_adds_the_others`; a failed import into a read-only Calendar: `an_import_into_a_read_only_calendar_is_refused_and_stores_nothing`.
    - Undo undoing the import in one step: `change_journal::every_change_is_a_unit_that_undo_reverses_exactly::import_that_adds_and_updates`; Undo refused in a Calendar now read-only: `undo_is_refused_in_a_calendar_now_read_only`.
    - An import that changes nothing offers no Undo: `import_run::an_import_that_changes_nothing_is_no_journal_transaction_and_sends_no_signal`, and for an update that leaves every Event as it was, `an_update_that_leaves_every_event_as_it_was_is_no_journal_transaction_and_sends_no_signal`, **added by this ticket** (it passed at once).
    - The password in the keyring, and deleted with the Account: `caldav_accounts::adding_an_account_checks_its_credentials_and_keeps_its_password_in_the_keyring`, `removing_an_account_deletes_its_password_from_the_keyring`.
    - A newly added server Calendar read-only; the Available Calendars leaving out added ones; a removed one available again; an Account removed with its Calendars: `caldav_calendars::a_newly_added_calendar_refuses_changes_until_1_0`, `adding_an_available_calendar_makes_it_a_calendar_of_the_account`, `removing_a_calendar_deletes_only_its_local_copy_and_makes_it_available_again`, `removing_an_account_removes_its_calendars_their_events_and_their_history`.
    - The First Sync reported while it runs: `caldav_first_sync::the_first_sync_commits_page_by_page_and_reports_that_it_runs`.
  - **Variations left only in the Window Model's, Window Dialogs' and ScopeDialog's tests:** the "Date changed" held by a drop's Scope Prompt and a press released without one; subscribing before the first read and the startup order; the Scope Prompt's choices, its Cancel, a changed repeat, the Exception counts and the drop's Cancel; a failed move's retry; a refused Undo's retry; a file picked while another dialog opened. A failed settings write is one App test now, with an error code as the core gives.
  - **The hostile test Event** still goes through the grid, its details, the editor, the delete question, the refused drag, the import dialog, the sidebar's Calendar and Account names and the server's refusal, from scripted answers.
  - **New terms**, for approval: **FakeCore** (the spec's "fake core interface", taking over from RecordingCore), **First Launch** (`firstLaunch`, a FakeCore scripted as a new Local Store), **Scripted Window** (`ScriptedWindow`/`scriptWindow`, the Accounts, Calendars and Occurrences scripted for the window's next reads), **Builders** (the helpers in `answers.ts` that fill in what a test leaves out), **copyAsSent** (FakeCore's deep copy of what crosses the interface), **Outcome** (how FakeCore answers a call: value, default or failure). Ticket 116's Default Answer, Scripted Answer, Pending Answer, Failure and Call are still unapproved.
  - **Left as judgement calls from `/code-review`:** each test file keeps its own `showApp`, since they wait for different things and return different helpers; `plainText` in `answers.ts` builds plain structured text for tests that give no links, and a test that needs the core's structuring gives it.
