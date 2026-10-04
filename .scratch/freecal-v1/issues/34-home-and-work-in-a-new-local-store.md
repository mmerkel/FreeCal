# 34: Home and Work in a new Local Store, an empty hint and row tooltips

**What to build:** A new Local Store starts with two Local Calendars, "Home" (shown) and "Work" (hidden), so a new user sees a filled dot and a ring side by side and has a Calendar to put Events in from the first launch. When the Local Account has no Calendars, its section shows a hint pointing to "+". Each Calendar row's tooltip says whether it is shown and what a click does. The Local Store allows only one Local Account. Spec: user stories 1–3 and 132, "Local Store" under Implementation Decisions and "Sidebar" under Frontend.

**Blocked by:** 02, 32

**Status:** ready-for-agent

- [ ] A new Local Store is created with the Local Account and two Calendars in it: "Home" (`#039be5`, shown) and "Work" (`#d50000`, hidden), the first two colours of `NEW_CALENDAR_ORDER` in `src/lib/palette.ts`. They are created in the same step as the Local Account, so no Signal is sent
- [ ] Only a new Local Store gets them: opening an existing Local Store never creates Calendars, so deleted ones never come back (core test: delete both, reopen, still none)
- [ ] `a_new_local_store_has_no_calendars` becomes a test that a new Local Store has Home (shown) and Work (hidden)
- [ ] A new migration adds a partial unique index so that the Local Store can't hold a second Local Account (core test: inserting one is refused)
- [ ] When the Local Account has no Calendars, its section shows "No calendars yet. Use + to create one."
- [ ] Each Calendar row's tooltip is "<name>: shown, click to hide" or "<name>: hidden, click to show". It replaces the name-only tooltip on the name, so long names can still be read in full
- [ ] Every string goes through `t()`
- [ ] Component tests against the fake core: the hint appears with no Calendars and goes when one is created; the tooltip follows the shown state
- [ ] Checked by hand with `run-freecal start --fresh`: Home and Work appear, and the driver's notes in `.claude/skills/run-freecal/SKILL.md` say that a fresh Local Store now has them

The names are stored in English, because v1 ships English only and the core creates them before the frontend runs. When FreeCal is translated, the core should pick the two names from the system locale when it creates the Local Store; they stay as they are afterwards, like any name the user typed.

Out of scope: what creating an Event does when there is no writable Calendar (ticket 03).
