# 32: Visual style: shared base and restyled sidebar

**What to build:** FreeCal gets one neutral, flat look (in the style of Notion Calendar and shadcn/ui) that follows the system's light or dark style, built as a shared base the whole window uses. The sidebar is restyled with it: colour dots instead of checkboxes, a ⋮ menu per Calendar row with a new "Show only this", and a modal dialog to confirm deleting a Calendar. Spec: user stories 14, 123, 124 and "Visual style" under Frontend.

**Blocked by:** 02

**Status:** ready-for-agent

The look was chosen from a throwaway prototype: option C on branch `prototype/sidebar-styles` (`src/lib/prototype-sidebar/VariantC.svelte`, commit `2b46cc9`). Build it exactly as prototyped, but never merge that branch. To see it: switch to the branch, `.claude/skills/run-freecal/freecal.sh build`, run `./target/debug/freecal` and press → until C shows.

- [ ] A shared base in `src/app.css`: colour, spacing, radius and font-size tokens as CSS custom properties, plus outline, solid, danger and ghost buttons, text inputs, a menu and a focus ring; components use the tokens, not their own colour values
- [ ] Every token has a light and a dark value; the window follows `prefers-color-scheme` through one attribute on the root element (`data-color-scheme`), so that ticket 33 can later set it from a setting without touching components
- [ ] The grid uses FullCalendar's `breezy` theme with the `indigo` palette, and switches to its dark variant together with the window
- [ ] A shared `Dialog` component built on `<dialog>` and `showModal()`: title, body, a row of buttons on the right, Escape and Cancel close it, focus returns to where it was; the import preview (10), the recurrence scope prompt (08) and "Working…" reuse it
- [ ] The system UI font (`system-ui`, no bundled font). Sizes in `rem`; no fixed heights around text (padding and `min-height` instead); long Account and Calendar names are cut off with "…" and shown in full in a tooltip
- [ ] Icons from `@lucide/svelte` (ISC), only the icons used ending up in the bundle; no text symbols such as ✎ ✕ remain
- [ ] Sidebar: light grey background, 1px right border, 14rem wide; each Account label in small uppercase grey text with a ghost "+" button to add a Calendar
- [ ] Calendar rows are compact, with a colour dot instead of a checkbox: filled when shown, a ring when hidden, and a hidden Calendar's name greyed. Clicking the row shows or hides the Calendar; the row is a `role="switch"` with `aria-checked`
- [ ] A ghost ⋮ button per row, visible on row hover or keyboard focus, opens the row menu: a "Colour" label with a row of 8 swatches (ring on the current colour), Rename, Show only this, a divider, then Delete in red. The menu opens inside the sidebar so it isn't clipped
- [ ] Right-click on a row, the context-menu key and Shift+F10 on a focused row open the same menu
- [ ] "Show only this" shows that Calendar and hides every other Calendar in all Accounts, through one new core call that sends one "Calendars changed" Signal (core test); there is no special undo
- [ ] Delete asks in the shared `Dialog`: title `Delete "<name>"?`, grey body text, "Cancel" (outline, initial focus) and "Delete" (solid red); the inline confirmation in the sidebar is gone
- [ ] New and renamed Calendars still use the inline name input (small, bordered; Enter saves, Escape cancels), restyled
- [ ] Every new string goes through `t()`; Calendar names stay plain text (the hostile test Event's Calendar name goes through the restyled rows, the menu and the delete dialog)
- [ ] Component tests against the fake core: toggling by click and keyboard, the menu by ⋮, right-click and Shift+F10, "Show only this", recolour from the swatches, and delete confirmed and cancelled
- [ ] Checked by hand in the real app with `run-freecal`, in light and dark, and once with DejaVu Sans as the system font (the widest common one) to catch overflow

Known trade-off, accepted: the colour dot makes hiding a Calendar less discoverable than a checkbox. Keyboard and screen readers get a switch.

Out of scope: the setting that overrides light or dark (ticket 33); restyling the toolbar (ticket 04 uses the shared base when it lands); the markers later tickets add to rows (lock 14, sync status 15, unsent count 18), which should use the same grey, near-colourless style.
