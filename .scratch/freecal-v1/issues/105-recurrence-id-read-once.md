# 105: A recurrence ID read once

**What to build:** Agreed on 2026-11-06 (ADR 0012 unchanged: recurrence IDs are stored as text, as obtained). A recurrence ID is parsed once, when FreeCal obtains it, instead of at every comparison; an incoming Exception's no longer goes from its start to text and back.

- `RecurrenceId` (`core/src/event/recurrence_id.rs`) holds `obtained: String`, the text the Local Store writes, and `parsed: Option<Time>`, the original start of the Occurrence it names, none when it can't be read (it pairs with nothing).
- Made from a file or a server, both halves are filled from what the reader already has; made from a stored field or the window, the text is parsed once. The Normal Form works from `parsed` only.
- `ImportedEvent`'s `original: Option<Time>` becomes `recurrence_id: Option<RecurrenceId>`, filled by the reader; `ImportedEvent::recurrence_id()` goes. Code that asks whether an Event is an Exception tests `recurrence_id.is_some()`; code that needs the start uses `recurrence_id.parsed`.
- The text columns stay (`event`, `unsent_change`, `journal_event_modification`).

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `RecurrenceId` holds `obtained` and `parsed`; nothing parses a recurrence ID's text more than once
- [x] `ImportedEvent` holds `recurrence_id: Option<RecurrenceId>`; `original` and `ImportedEvent::recurrence_id()` are gone
- [x] A stored recurrence ID that can't be read still pairs with nothing
- [x] The Local Store's columns are unchanged

## Comments

2026-11-06, implemented:

- `RecurrenceId` (`core/src/event/recurrence_id.rs`) holds `obtained` and `parsed`. `RecurrenceId::obtained` reads a stored field's or the window's text once. `RecurrenceId::of_original` takes the start the file or server reader already has and writes its text (`recurrence_id_text`). `normal_form` works from `parsed` only.
- The fields stay private, with no `parsed()` accessor. Nothing outside the module needs the start, and private fields keep "compare only by the Normal Form". This departs from the ticket's wording "code that needs the start uses `recurrence_id.parsed`".
- `ImportedEvent`, `ImportPreview` and `Holds` lost `PartialEq`/`Eq`, because `RecurrenceId` deliberately has none. The `Holds` tests use `matches!`.
- Left as is:
  - An Exception FreeCal makes (`NormalForm::to_recurrence_id`) parses the Normal Form text it was just given, once. `NormalForm` holds only text.
  - The repeat rule's functions (`start_of_id` callers in `core/src/event/rule.rs`) still parse Normal Form and EXDATE text. Those aren't recurrence IDs as obtained.

New terms: none. `parsed` and `obtained` are the ticket's field names.
