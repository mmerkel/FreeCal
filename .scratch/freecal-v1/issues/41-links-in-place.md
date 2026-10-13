# 41: Clickable links in an Event's place

**What to build:** Links in an Event's place become clickable in the details, exactly like those in the description: the core turns the place into text and link pieces, and the details draw them with the description's link display. Spec: user story 140, "Links" under "Untrusted content".

**Blocked by:** none

**Status:** done

- [x] The core sends the place as structured text (text and link pieces) next to its plain form, which the editor keeps editing; the same code turns description and place into pieces
- [x] Only `http(s)://` and `mailto:` become links (core tests, including a `javascript:` URL staying text)
- [x] The details draw the place's pieces with the description's link display; a click opens the system browser through the shell
- [x] The TypeScript types and samples are regenerated (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`)
- [x] Component test: a place with a Zoom URL shows a link that goes through `onopenlink`

Out of scope: a map link for addresses.
