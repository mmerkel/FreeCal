# 32: Visual style: shared base and restyled sidebar

**What to build:** FreeCal gets one neutral, flat look (in the style of Notion Calendar and shadcn/ui) that follows the system's light or dark style, built as a shared base the whole window uses. The sidebar is restyled with it: colour dots instead of checkboxes, a ⋮ menu per Calendar row with a new "Show only this", and a modal dialog to confirm deleting a Calendar. Spec: user stories 14, 123, 124 and "Visual style" under Frontend.

**Blocked by:** 02

**Status:** done

The look was chosen from a throwaway prototype: option C on branch `prototype/sidebar-styles` (`src/lib/prototype-sidebar/VariantC.svelte`, commit `2b46cc9`). Build it exactly as prototyped, but never merge that branch. To see it: switch to the branch, `.claude/skills/run-freecal/freecal.sh build`, run `./target/debug/freecal` and press → until C shows.

- [x] A shared base in `src/app.css`: colour, spacing, radius and font-size tokens as CSS custom properties, plus outline, solid, danger and ghost buttons, text inputs, a menu and a focus ring; components use the tokens, not their own colour values
- [x] Every token has a light and a dark value; the window follows `prefers-color-scheme` through one attribute on the root element (`data-color-scheme`), so that ticket 33 can later set it from a setting without touching components
- [x] The grid uses FullCalendar's `breezy` theme with the `indigo` palette, and switches to its dark variant together with the window
- [x] A shared `Dialog` component built on `<dialog>` and `showModal()`: title, body, a row of buttons on the right, Escape and Cancel close it, focus returns to where it was; the import preview (10), the recurrence scope prompt (08) and "Working…" reuse it
- [x] The system UI font (`system-ui`, no bundled font). Sizes in `rem`; no fixed heights around text (padding and `min-height` instead); long Account and Calendar names are cut off with "…" and shown in full in a tooltip
- [x] Icons from `@lucide/svelte` (ISC), only the icons used ending up in the bundle; no text symbols such as ✎ ✕ remain
- [x] Sidebar: light grey background, 1px right border, 14rem wide; each Account label in small uppercase grey text with a ghost "+" button to add a Calendar
- [x] Calendar rows are compact, with a colour dot instead of a checkbox: filled when shown, a ring when hidden, and a hidden Calendar's name greyed. Clicking the row shows or hides the Calendar; the row is a `role="switch"` with `aria-checked`
- [x] A ghost ⋮ button per row, visible on row hover or keyboard focus, opens the row menu: a "Colour" label with a row of 8 swatches (ring on the current colour), Rename, Show only this, a divider, then Delete in red. The menu opens inside the sidebar so it isn't clipped
- [x] Right-click on a row, the context-menu key and Shift+F10 on a focused row open the same menu
- [x] "Show only this" shows that Calendar and hides every other Calendar in all Accounts, through one new core call that sends one "Calendars changed" Signal (core test); there is no special undo
- [x] Delete asks in the shared `Dialog`: title `Delete "<name>"?`, grey body text, "Cancel" (outline, initial focus) and "Delete" (solid red); the inline confirmation in the sidebar is gone
- [x] New and renamed Calendars still use the inline name input (small, bordered; Enter saves, Escape cancels), restyled
- [x] Every new string goes through `t()`; Calendar names stay plain text (the hostile test Event's Calendar name goes through the restyled rows, the menu and the delete dialog)
- [x] Component tests against the fake core: toggling by click and keyboard, the menu by ⋮, right-click and Shift+F10, "Show only this", recolour from the swatches, and delete confirmed and cancelled
- [x] Checked by hand in the real app with `run-freecal`, in light and dark, and once with DejaVu Sans as the system font (the widest common one) to catch overflow

Known trade-off, accepted: the colour dot makes hiding a Calendar less discoverable than a checkbox. Keyboard and screen readers get a switch.

Out of scope: the setting that overrides light or dark (ticket 33); restyling the toolbar (ticket 04 uses the shared base when it lands); the markers later tickets add to rows (lock 14, sync status 15, unsent count 18), which should use the same grey, near-colourless style.

## Comments

2026-10-04: Done.
- The shared base is `src/app.css` (tokens, `.button` with `outline`/`solid`/`danger`, `.icon-button`, `.input`, `.menu-*`). `src/lib/Dialog.svelte` and `src/lib/Menu.svelte` are the shared modal dialog and menu; ticket 33's main menu should reuse `Menu`. `src/lib/colourScheme.ts` sets `data-color-scheme` from the system; ticket 33 replaces that with the setting.
- The core gained `show_only_calendar` (one UPDATE, one "Calendars changed").
- Swatches replace the browser colour picker (24 since the follow-ups below). Colours outside them (none yet, but server Calendars later) simply show no ring.
- A menu must not close on the click that opened it: in the real webview, that click is still bubbling when the menu appears. jsdom dispatches synchronously, so component tests can't show this; it was caught in the real window. How `Menu` closes now is in the follow-ups below.
- jsdom has no `showModal()`; `src/test/setup.ts` adds a minimal stand-in.
- Checked by hand with `run-freecal`: light, dark (`GTK_THEME=Adwaita:dark`) and DejaVu Sans. Breezy's dark grid background is slightly blue-tinted (Tailwind grey), while the chrome is neutral grey. Checked by hand and kept on purpose: it looks right as it is.

2026-10-04: Follow-ups from testing by hand.
- A click or right-click outside an open menu now only closes it. `Menu` stops every mouse press, release and click outside itself on `window` in the capture phase while open, and closes on the `click` or `contextmenu`. This replaces the `pointerdown` listener and its opener exception; ⋮ toggles because its click is stopped before reaching it. A transparent layer was the first plan, but jsdom doesn't hit-test, so the component tests couldn't have shown it working.
- The webview's own context menu is off everywhere, text fields included, for a consistent look (`src/lib/browserContextMenu.ts`).
- 24 swatches (Google Calendar's colours, which include the 8 before), in three rows by hue. The palette lives in `src/lib/palette.ts`: a new Calendar takes the first colour in `NEW_CALENDAR_ORDER` that the fewest Calendars have, a farthest-point order in OKLab with browns and greys last.
- `run-freecal` gained `rightclick`, and its menu coordinates follow the taller menu. All checked in the real window.

2026-10-04: Fixes from `/code-review` against `b9a2b26`.
- Spacing tokens `--space-1` to `--space-12` (N × 2px) in `src/app.css`; every padding, margin and gap uses them, with values unchanged (screenshots match pixel for pixel).
- Account headings show their full name in a tooltip, like Calendar names.
- ⋮ shows while its row has keyboard focus (`li:has(:focus-visible)`), but not after a mouse click.
- The browser context menu is turned off in the capture phase, so a handler that stops a right-click from spreading can't bring it back.
- The palette moved to `src/lib/palette.ts`, with each colour written once.
- The Calendar dots and rings are radial gradients with a 1px soft edge, chosen by hand over a rounded box, a border, SVG and a 12px size: at 10px they look rounder.
