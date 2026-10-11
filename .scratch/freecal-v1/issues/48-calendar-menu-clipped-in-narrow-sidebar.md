# 48: Calendar menu clipped in a narrow sidebar

**What to build:** A Calendar's ⋮ menu shows in full whatever the sidebar's width. Today it opens below its row, anchored to the row's right end and inside the sidebar, which clips it. The menu is about 178px wide, so below about 11.5rem its left part (the first column of swatches, the start of Rename, Show only this and Delete) runs past the window's left edge. That was already so at ticket 31's 10rem minimum; ticket 47's 8rem minimum makes it worse.

**Blocked by:** none

**Status:** needs-triage

- [ ] At 8rem, 12rem and 20rem the whole menu shows, and stays inside the window
- [ ] It still opens, closes and returns focus as now (ticket 32's menu)

## Comments

- 2026-10-11: Found while checking ticket 47 by hand with `run-freecal`: `update app_state set sidebar_width = 8`, restart, then the Home row's ⋮ at about 106,56. Possible ways: open the menu in the top layer (a popover) placed against the ⋮ button, or anchor it to the row's left end and let it overlap the grid, which the sidebar's `overflow` currently prevents.
