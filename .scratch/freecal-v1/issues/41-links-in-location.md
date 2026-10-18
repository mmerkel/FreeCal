# 41: Clickable links in an Event's location

**What to build:** Links in an Event's location become clickable in the details, exactly like those in the description: the core turns the location into text and link pieces, and the details draw them with the description's link display. Spec: US-140, "Links" under "Untrusted content".

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] The core sends the location as structured text (text and link pieces) next to its plain form, which the editor keeps editing; the same code turns description and location into pieces
- [x] Only `http(s)://` and `mailto:` become links (core tests, including a `javascript:` URL staying text)
- [x] The details draw the location's pieces with the description's link display; a click opens the system browser through the shell
- [x] The TypeScript types and samples are regenerated (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`)
- [x] Component test: a location with a Zoom URL shows a link that goes through `onopenlink`

Out of scope: a map link for addresses.
