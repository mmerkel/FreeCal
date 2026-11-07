# 109: A resource FreeCal can't read is fetched again until it reads, with sync tokens too

**What to build:** ADR 0015 says a Server Object FreeCal can't read "is fetched again until it reads". On a server with sync tokens (Radicale), this holds only for a Server Object already mirrored. A resource that is listed once and isn't read is lost until it changes on the server, because the next sync token doesn't list it again:

- **A new resource FreeCal can't read:** nothing is remembered. `mark_unreadable` (`core/src/store/sql/mirror/caldav.rs`) updates a `caldav_server_object` row that doesn't exist.
- **An Ignored Resource FreeCal can't read:** it keeps its old ETag, but `known_server_resources` (`core/src/store/sql/mod.rs`) reports every Ignored Resource as readable. So the loop in `Client::changes` (`core/src/caldav.rs`) that adds unreadable resources the sync token didn't list skips it.

One rule replaces the `unreadable` mark: **a Known Server Resource with no stored ETag is fetched on every Incoming Sync**, listed or not, and with or without a sync token.

- **A mirrored Server Object FreeCal can't read:** its ETag is forgotten. It keeps its last readable text and its UID, and its Event Rows stay as they are.
- **A new resource FreeCal can't read:** it becomes an Ignored Resource with no ETag and no UID.
- **An Ignored Resource FreeCal can't read:** its ETag is forgotten, and it keeps its UID.
- **A resource that reads again:** it is handled as any fetched resource. It is mirrored, ignored with its ETag, or Forgotten.
- **A server that gives no ETags (RFC 4791 §5.3.4 forbids this):** its resources are fetched on every Incoming Sync too.
- **The Log:** one line when a resource becomes unreadable, that is, when it is new or its stored ETag goes from known to missing. There is no line for each fetch while it stays unreadable. Like every Log line, this one escapes the address.
- **Schema:** the `unreadable` column of `caldav_server_object` goes, with `Held.unreadable` (`core/src/remote.rs`), `mark_unreadable` and the separate loop in `Client::changes`. Change the baseline migration; there are no Local Stores to migrate yet. The schema comment says what a missing ETag means. It also says that a missing UID on an Ignored Resource means "holds several UIDs" or "not read".

ADR 0015's amendment of 2026-11-07 records this.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] With sync tokens, a new resource FreeCal can't read is fetched on each Incoming Sync until it reads, and its Event then shows (seam test, Radicale)
- [ ] With sync tokens, an Ignored Resource that becomes unreadable is fetched on each Incoming Sync until it reads (seam test)
- [ ] A mirrored Server Object that becomes unreadable keeps its text, UID and Event Rows, and is fetched on each Incoming Sync until it reads. The existing test arranges this with `etag = NULL` instead of `unreadable = 1`
- [ ] The Log gets one line when a resource becomes unreadable, and none on the fetches after while it stays so
- [ ] No `unreadable` column, field or function remains. A Known Server Resource crossing the Provider interface is its address and optional ETag, plus what ticket 108 removes
- [ ] The glossary's Ignored Resource entry and the schema comment agree with ADR 0015's amendment
