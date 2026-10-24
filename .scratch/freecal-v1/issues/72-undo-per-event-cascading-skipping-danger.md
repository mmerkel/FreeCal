# 72: Undo per Event: cascading, skipping and danger

**What to build:** Undo goes per Event as ADR 0009 describes, on top of ticket 69's Change Journal module: an Event changed since an entry has its later entries undone first, a unit can be undone one Event at a time, Events that can't be undone are skipped with a reason, and the core says whether an undo is benign or dangerous. Spec: US-81, US-82 and US-149; "Change Journal" under Implementation Decisions.

**Blocked by:** 69, 83

**Status:** ready-for-agent

**Effort:** M

- [ ] An Event that no longer matches the entry's hash has its later entries undone first, newest first, replacing ticket 69's `ChangedSinceUnit` refusal; the core reports them before running, so the frontend can ask (ticket 70)
- [ ] A unit of the user's own action is undone only whole (ADR 0009): its entries depend on each other, as when "this and following" splits a Recurring Event in two, and undoing one half would make Occurrences vanish or appear twice. An import's or sync's unit can be undone whole or one Event at a time, a Recurring Event with its Exceptions together (ticket 83's loading by Calendar and UID says which they are); undoing the rest of a partly undone unit works
- [ ] Events in Read-only Calendars are skipped and their Calendars named; the rest is undone (replaces ticket 58's refusal of the whole undo)
- [ ] An Event whose current version matches no entry is skipped with "Can't undo “<title>”: its current version isn't in the Change Journal of the Local Store." (title as text in bidi isolates, through `t()`)
- [ ] The core classifies an undo as benign or dangerous (also undoes later changes, reverts a change pulled from a server, or exceeds the Mass-Change Guard's thresholds of 10 Events or 5% of a Calendar) and says why
- [ ] Entries older than 90 days are removed (controllable clock); moved here from ticket 69
- [ ] Core tests for every point above, including a split by "this and following" undone from either half, and ticket 58's cases: an imported Event moved since, an updated one edited or deleted since

Out of scope: Ctrl+Z, toasts and confirmations (ticket 70); the list and History (ticket 19); server writes of an undo (ticket 17).

## Comments

- 2026-10-18: Split from ticket 69 after an architecture review and its grilling: 69 builds the Change Journal module with whole-unit undo, this ticket adds the per-Event rules.
- 2026-10-18: Dropped the skip for an Event whose previous version's Calendar was deleted: ticket 69 now removes those entries when the Calendar is deleted, so no undo can meet one.
- 2026-10-24: Blocked by 83 after an architecture review: a user action's unit is undone only whole (ADR 0009, amended), and an import's or sync's per Recurring Event, which ticket 83's module says.
