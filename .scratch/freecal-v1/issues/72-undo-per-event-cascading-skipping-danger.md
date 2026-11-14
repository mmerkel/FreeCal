# 72: Undo per Event: cascading, skipping and danger

**What to build:** Undo goes per Event as ADR 0009 describes, on top of ticket 69's Change Journal module: an Event changed since a Journal Event Modification has its later Journal Event Modifications undone first, a Journal Transaction can be undone one Event at a time, Events that can't be undone are skipped with a reason, and the core says whether an undo is benign or a Dangerous Undo. Spec: US-81, US-82 and US-149; "Change Journal" under Implementation Decisions.

**Blocked by:** 69, 83, 85

**Status:** done

**Effort:** M

- [x] An Event that no longer matches the Journal Event Modification's hash has its later Journal Event Modifications undone first, newest first, replacing ticket 69's `ChangedSinceUnit` refusal; the core reports them before running, so the frontend can ask (ticket 70)
- [x] A Journal Transaction of the user's own action is undone only whole (ADR 0009): its Journal Event Modifications depend on each other, as when "this and following" splits a Recurring Event in two, and undoing one half would make Occurrences vanish or appear twice. An import's or sync's Journal Transaction can be undone whole or one Event at a time, a Recurring Event with its Exceptions together (ticket 83's loading by Calendar and UID says which they are); undoing the rest of a partly undone Journal Transaction works
- [x] Events in Read-only Calendars are skipped and their Calendars named; the rest is undone (replaces ticket 58's refusal of the whole undo)
- [x] An Event whose current version matches no Journal Event Modification is skipped with "Can't undo “<title>”: its current version isn't in the Change Journal of the Local Store." (title as text in bidi isolates, through `t()`)
- [x] The core classifies an undo as benign or a Dangerous Undo (also undoes later changes, reverts a change pulled from a server, or exceeds the Mass-Change Guard's thresholds of 10 Events or 5% of a Calendar) and says why
- [x] Journal Event Modifications older than 90 days are removed (controllable clock); moved here from ticket 69
- [x] Core tests for every point above, including a split by "this and following" undone from either half, and ticket 58's cases: an imported Event moved since, an updated one edited or deleted since

Out of scope: Ctrl+Z, toasts and confirmations (ticket 70); the list and History (ticket 19); server writes of an undo (ticket 17).

## Comments

- 2026-10-18: Split from ticket 69 after an architecture review and its grilling: 69 builds the Change Journal module with whole-unit undo, this ticket adds the per-Event rules.
- 2026-10-18: Dropped the skip for an Event whose previous version's Calendar was deleted: ticket 69 now removes those entries when the Calendar is deleted, so no undo can meet one.
- 2026-10-24: Blocked by 83 after an architecture review: a user action's unit is undone only whole (ADR 0009, amended), and an import's or sync's per Recurring Event, which ticket 83's module says.
- 2026-10-24: Blocked by 85: an Event's later entries are found by its ID, which must not be shared with an earlier Exception of the same Recurring Event.
- 2026-11-14: Implemented. Decisions, for review, each with its strongest con:
  - **Decision, for review:** following an Event's later changes into a user action's or an undo's Journal Transaction undoes that one whole too, as ADR 0009 undoes it only whole; into an import's or a sync's, only the Events of that UID. Con: an undo can change Events the user didn't pick; the Dangerous Undo names them under the later Journal Transactions.
  - **Decision, for review:** the Mass-Change Guard's thresholds are both needed, more than 10 Events *and* more than 5% of the Calendar, per Calendar (`core/src/mass_change_guard.rs`). Con: a Calendar of 10,000 Events loses up to 500 to one undo without asking; with "or", almost every undo in a Calendar of under 20 Events would ask.
  - **Decision, for review:** Events are counted as Event Rows, so a Recurring Event with Exceptions counts once per Exception. Con: it reaches the thresholds sooner than the user counts.
  - **Decision, for review:** one Event at a time only for an import's or a sync's Journal Transaction, not an undo's; a user action's Event refuses with the new error `UndoneOnlyWhole`. Con: an import undone whole and then redone can't be redone one Event at a time.
  - **Decision, for review:** an Event target brings along every Event of its UID in that Journal Transaction, a Recurring Event with its Exceptions. Con: an Exception imported without its Recurring Event goes with any other of that UID.
  - **Decision, for review:** an Event with guests is skipped too (reason `hasGuests`), like one in a Read-only Calendar, instead of failing the whole undo. Con: one more reason the frontend must word.
  - **Decision, for review:** a Dangerous Undo writes nothing; it runs when called again with the danger it has then, compared as a whole. Con: anything that changes in between (a sync) asks again.
  - **Decision, for review:** removing Journal Transactions older than 90 days runs with each new Journal Transaction and when the Local Store opens, with no index on `made_at`. Con: a scan of the `journal_transaction` table on every change; cheap at its size.
  - **Decision, for review:** until ticket 70 asks before a Dangerous Undo, the import toast's Undo runs one that is dangerous only for its number of Events (the user pressed Undo on that import), and refuses one that would undo later or server changes with "events it changed were changed since". Con: two rules for one button, until ticket 70 replaces them.
  - **Decision, for review:** the refusal of ticket 104 stays: an Event Version that would join another Event of its UID in its Calendar, other than as an Exception, still refuses with `ChangedSinceJournalTransaction`.
  - **Decision, for review:** an Event skipped skips the Events changed together with it: every Event of a user action's or an undo's Journal Transaction, and those of its UID in an import's or a sync's, each with the new reason "it was changed together with “<title>”, which can't be undone". So a "this and following" split with one half in a Read-only Calendar is undone neither half. Con: one read-only Event can leave a large user action wholly undone.
  - Following an Event back through its later changes tries every Journal Event Modification that made its current version, as an undo and the change it undid's predecessor can both have made it (from the review). Con: it searches, which is exponential only in an Event's history of identical versions.
  - New terms: **Undo Target** (new term): what an undo undoes, a whole Journal Transaction or one Event of it. **Skipped Event** (new term): an Event an undo left as it is, with why. **Tracked Event** (new term): an Event as the Change Journal tracks it, by its ID and UID (`TrackedEvent`, in place of "Event Key", which the glossary avoids). The interface types `DangerousUndo`, `LaterJournalTransaction`, `SkipReason`, `UndoOutcome` are named after existing terms.
  - The core interface `undo_journal_transaction` became `undo(target, approved)`; the shell command and the import toast's Undo follow. Tests: `core/tests/undo_per_event.rs`, the CalDAV ones in `core/tests/caldav_sync.rs` (`mod undo`) and `core/tests/guests.rs`; the hostile test Event's `undoSkippedReadOnly`.
