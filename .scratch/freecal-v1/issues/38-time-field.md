# 38: Time field with typed entry and a time list

**What to build:** The editor's start and end times use a combobox of FreeCal's own instead of the native `type="time"` input: the user can type a time freely or pick it from a list in 15-minute steps, `a` and `p` set AM or PM, and changing the start moves the end with it. Spec: US-135–US-137, "Time field" and "Event editor" under Frontend.

**Blocked by:** 36

**Status:** ready-for-agent

**Effort:** M

- [ ] A pure parser (unit tests) accepts `14`, `1430`, `14:30`, `14.30`, `14h30`, `2:30p`, `2.30 pm`, `2p`, `2 pm` and any minute; rejects hours above 23 and minutes above 59. On a 12-hour clock an hour from 13 to 23 means PM, `12a` is midnight and `12p` noon
- [ ] The field shows the time in the shared clock format (ticket 36) and normalises typed text on Enter, Tab or blur
- [ ] Invalid text stays in the field, marked red with `aria-invalid`, and blocks Save with the "incomplete" message
- [ ] `a` and `p` pressed anywhere in the field set AM or PM of the current time in both clock modes and are not inserted (24-hour: `p` on 2:00 gives 14:00, `a` on 14:00 gives 2:00, `p` on 14:00 stays 14:00)
- [ ] The list has 15-minute steps over the whole day and scrolls to the current value when it opens
- [ ] When start and end are on the same day, the end list starts at the start time and shows durations ("15:30 (1 h 30 min)"); otherwise it lists the whole day without durations
- [ ] Keyboard: ↓ opens the list; while open, ↑/↓ move the active option, Enter picks it and Escape closes only the list, not the dialog. ↑/↓ never step the time while the list is closed
- [ ] ARIA combobox pattern: `role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant`, options with `role="option"`
- [ ] Changing the start date or time moves the end by the same amount; changing the end changes only the end
- [ ] Dates keep the native date input
- [ ] Every string goes through `t`
- [ ] Component tests for typing, `a`/`p`, the list and its keys, invalid input, and the end following the start; checked by hand in the real app with `run-freecal`

Out of scope: the Event Time Zone choice in the editor (ticket 50).

## Comments

- 2026-10-16: Ticket 48 moves `Menu` into the top layer as a `popover="manual"` placed with CSS anchor positioning (`position-area: bottom`, `position-try-fallbacks: flip-block`), which the real webview supports, including shifting to stay inside the window. The time list can use the same approach, which also keeps it from being clipped by the editor dialog.
