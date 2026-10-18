# 77: Discover a CalDAV Account's Available Calendars

**What to build:** Adding a CalDAV Account discovers the server's Calendars, and listing the Account's Calendars fetches the server's current list each time. Calendars the user hasn't added are Available Calendars: FreeCal stores nothing for them. Spec: user story 15. Split from ticket 53.

**Blocked by:** 76

**Status:** ready-for-agent

**Effort:** M

- [ ] Adding an Account discovers the server's Calendars
- [ ] Discovery lists all Calendars of the Account as Available Calendars; FreeCal stores nothing for Available Calendars and doesn't sync them
- [ ] Calendars that appear on the server later stay Available Calendars until the user adds them; listing the Account's Calendars fetches the server's current list
- [ ] Discovery requests are built with the XML writer from ticket 76; Calendar names from the server are treated as untrusted
- [ ] New errors cross to the frontend as codes, not English strings

Out of scope: adding or removing Calendars (ticket 78).

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 76): `core/src/caldav.rs` has the client, `propfind_body` (XML writer) and a namespace-aware `parse` into `Element`s with `Element::property`; discovery can add Depth 1 and the CalDAV namespace. Reading the password back goes through `Keyring::secret` (it has no caller yet). Redirects aren't followed (the password must stay with the server the user named); `/.well-known/caldav` discovery should follow its redirect on the same host by hand, and a redirect should get a clearer error than "refused (status 301)".
