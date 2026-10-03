# 02: Local Calendars

**What to build:** The user can create, rename, recolour and delete Calendars in the Local Account, and show or hide each Calendar from a sidebar where Calendars are grouped by Account.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] Create, rename, recolour and delete a Local Calendar through the core interface and the sidebar
- [ ] Each Calendar has a colour
- [ ] Calendars are grouped by Account in the sidebar and can be shown or hidden individually
- [ ] The core sends "Calendars changed" on every add, remove, rename or recolour, and the sidebar refreshes on it
- [ ] The frontend subscribes to Signals before reading the initial state
