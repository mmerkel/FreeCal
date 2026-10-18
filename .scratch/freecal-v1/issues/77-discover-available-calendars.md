# 77: Discover a CalDAV Account's Available Calendars

**What to build:** Adding a CalDAV Account discovers the server's Calendars, and listing the Account's Calendars fetches the server's current list each time. Calendars the user hasn't added are Available Calendars: FreeCal stores nothing for them. Spec: US-15. Split from ticket 53.

**Blocked by:** 76

**Status:** done

**Effort:** M

- [x] Adding an Account discovers the server's Calendars
- [x] Discovery lists all Calendars of the Account as Available Calendars; FreeCal stores nothing for Available Calendars and doesn't sync them
- [x] Calendars that appear on the server later stay Available Calendars until the user adds them; listing the Account's Calendars fetches the server's current list
- [x] Discovery requests are built with the XML writer from ticket 76; Calendar names from the server are treated as untrusted
- [x] New errors cross to the frontend as codes, not English strings

Out of scope: adding or removing Calendars (ticket 78).

## Comments

- 2026-10-18: Split from ticket 53 so each part fits one session.
- 2026-10-18 (from 76): `core/src/caldav.rs` has the client, `propfind_body` (XML writer) and a namespace-aware `parse` into `Element`s with `Element::property`; discovery can add Depth 1 and the CalDAV namespace. Reading the password back goes through `Keyring::secret` (it has no caller yet). Redirects aren't followed (the password must stay with the server the user named); `/.well-known/caldav` discovery should follow its redirect on the same host by hand, and a redirect should get a clearer error than "refused (status 301)".
- 2026-10-18: Done. `add_caldav_account` now returns an `AddedAccount` (the Account and its `available` Calendars), and `Core::available_calendars(account_id)` asks the server each time; neither stores anything for them. Discovery: `current-user-principal`, then `calendar-home-set` (Depth 0), then each home with Depth 1 for `resourcetype`, `displayname`, Apple's `calendar-color` and `supported-calendar-component-set`. Choices:
  - When the given URL leads to no principal (a web page, 404/405/501, a redirect elsewhere), discovery asks `/.well-known/caldav` on the same server. Redirects are followed by hand on every `PROPFIND`, at most 5, only to the same host and port or from `http` to `https`; anything else is the new code `redirectedElsewhere { url }`. A 3xx without a `Location`, or too many, still shows as `serverRefused`.
  - Hrefs in answers (principal, home sets, Calendars) may name another host, because iCloud's calendar homes do; they can't move from `https` to `http`, carry a login, or use another scheme. Settled with the user: the first request already sends the password to the named host, so a hostile server gains nothing from either. The rule guards an honest server with a hole: an open redirect is a common bug, so redirects stay on the server; hrefs come from the server's own XML and are hard for a third party to inject.
  - Calendars that take no Events (task lists) aren't offered. A Calendar without a name is named after the last part of its address. Colours come as `#rrggbb` or `#rrggbbaa`, otherwise none.
  - A server whose principal has no calendar home now fails adding with `notCalDavServer`.
  - New codes: `passwordNotInKeyring`, `redirectedElsewhere`. The hostile fixture gained `calendarName` and `redirectUrl`; `calendarName` gets its `expected` entry when the window shows Available Calendars (ticket 55).
  - ureq kept connections an HTTP/1.0 server (Radicale on its own) had closed, so the client now opens one per request (`max_idle_connections(0)`); ticket 54 may want pooling back for servers that keep connections alive.
  - `available_calendars` lists every server Calendar for now; ticket 78 leaves out the added ones.
