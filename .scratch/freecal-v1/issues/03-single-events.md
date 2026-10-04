# 03: Create, view, edit and delete single Events

**What to build:** In a Local Calendar, the user can click an empty slot or drag across a time range to create an Event, choose which writable Calendar it goes into, open it to see its details, edit its title, time, all-day flag, place, description and Calendar, and delete it. A keyboard shortcut creates an Event.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] The core returns the Occurrences in a date range
- [ ] Clicking an empty slot or dragging across a range opens the editor prefilled with that time
- [ ] The editor offers only writable Calendars
- [ ] Event details show title, time, place, description and Calendar
- [ ] Edit and delete work and are persisted in the Local Store
- [ ] Every change sends "Occurrences changed" with the affected Calendar IDs; the frontend re-fetches the visible range and keeps the current selection
- [ ] An open editor's draft survives a re-fetch
- [ ] A Google-style shortcut creates an Event
- [ ] Hidden Calendars' Events are not shown
- [ ] Descriptions cross to the frontend as structured content (paragraphs of text and link pieces), built in the core; HTML is converted as in "Untrusted content" in the spec, and the editor edits the plain-text form. An untouched description keeps its original bytes
- [ ] Only `http(s)://` and `mailto:` become links; clicking one opens the system browser through the shell, which checks the scheme again in Rust; a link whose text differs from its URL asks "Open <full URL> in your browser?" first
- [ ] The hostile test Event is created at both seams and runs through the core, the grid, Event details and the editor: nothing in it runs or is rendered as HTML, and disallowed links are plain text
