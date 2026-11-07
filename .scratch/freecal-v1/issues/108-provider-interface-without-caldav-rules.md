# 108: The Provider interface without CalDAV's rules

**What to build:** The Provider interface (spec: "Provider interface"; `Remote`, `core/src/remote.rs`) must fit Google, Microsoft Graph and ICS feeds without changing the Sync Engine. Since tickets 102–107 it carries CalDAV's rules for Ambiguous UIDs and Ignored Resources (ADR 0015). Move them into the CalDAV part of the Sync Engine (`core/src/sync/caldav/`), so that the Provider interface only fetches and reports what changed.

What leaked, and where it goes:

- **Known Server Resources** (`Held`) carry an `ignored` flag and the UID held, both used only for the takeover. Across the Provider interface they carry only what the server side needs: the address and the ETag, none when FreeCal doesn't know it (ticket 109). To the server side, an Ignored Resource is just a known address and version that isn't fetched again until it changes. The CalDAV Sync Engine reads the UIDs and the ignored flag from its own tables.
- **Fetched Server Objects** come with `Holds`, CalDAV's reading of a resource (one UID per resource, RFC 4791 §4.1), which `Client::multiget` does (`core/src/caldav.rs`). The Provider interface gives the Server Objects as received, and the CalDAV Sync Engine reads them (`read_caldav_resource`, `core/src/import/mod.rs`), pinning floating times to the zone the Provider interface reports, as now.
- **The takeover** (`takeover`, `core/src/caldav.rs`, called from `Client::changes`) predicts which Ignored Resources to fetch again when the Server Object keeping an Ambiguous UID stops keeping it (`Holds::keeps`), and fetches them in a second request. The Incoming Sync then decides the same thing again inside the Journal Transaction (`mirror_and_propagate`, `core/src/sync/caldav/incoming.rs`), and the two have to agree. The prediction goes (grilling of 2026-11-07). Before anything is fetched, the CalDAV Sync Engine adds to the addresses to fetch every Ignored Resource whose UID a Server Object listed as changed or removed keeps, whether or not that Server Object still keeps it. They go in the same request as the rest, so there is no second request. The decision is made once, inside the Journal Transaction, on the Mirror as it is. Fetching too much in this rare case is what lets the two places stop having to agree. Both requests stay before the Journal Transaction: there is still no network request inside a Database Transaction (ticket 107's constraint).
- **The Log for a fetched Ignored Resource:** one fetched with an unchanged ETag, as the step above can fetch, writes no Log line. It is written to the Log only when it changes, as ticket 106 says.
- **`Remote::changes`'s contract** says nothing about UIDs, Ignored Resources or a takeover any more.

**A CalDAV Server Object is its Calendar and UID** (grilling of 2026-11-07, ADR 0015's amendment). The unique index already allows one Server Object per UID, and nothing stored refers to a Server Object's ID.

- A move to another href and a takeover are the same thing: the UID's Server Object now lives at another href, with new text. A moved resource competes with the Ignored Resources holding its UID by the fixed order, like any takeover. `moved`, `moved_from`, the `Keeper` case for a moved object and the rule "a moved one beats Ignored Resources" go.
- What the Mirror held for the UID before the Incoming Sync is its previous Server Version, after a move or a takeover too (ADR 0014).
- `ObjectId` goes. The CalDAV Mirror's readers and writers take a Calendar and UID, or a Calendar and href where the address is the question. This covers `object_at`, `held_uid`, `server_object`, `object_of_event`, and `write_object`'s `over`. `Core::original_server_data` finds the Server Object by the Event's Calendar and UID. `caldav_server_object` keeps a plain primary key without `AUTOINCREMENT`. Change the baseline migration; there are no Local Stores to migrate yet.

Apart from moves, ADR 0015's rules don't change. The same resource keeps an Ambiguous UID, and a takeover happens in the same Incoming Sync.

**Tests** (spec: "Testing Decisions"): the unit tests in `core/src/sync/caldav/incoming.rs` that build fetched resources by hand go.

- Those that repeat a seam test are deleted.
- The rest move to `core/tests/caldav_sync.rs`, against Radicale. That covers the recurrence ID cases of ADR 0012, page order in a First Sync, and Ambiguous UID cases written straight into Radicale's storage.
- The pure function that builds the Log line keeps its unit test.

So this ticket's type changes break no tests.

**Blocked by:** 109

**Status:** ready-for-agent

**Effort:** L

- [ ] `core/src/remote.rs` names no CalDAV-only concept: no Ignored Resource, UID, `Holds` or takeover in its types or docs
- [ ] The Known Server Resources crossing the Provider interface hold the address and optional ETag only
- [ ] Ignored Resources to fetch again are chosen from the listing in `core/src/sync/caldav/` and fetched in the same request as the rest; `takeover` and the second request are gone; who keeps a UID is decided only inside the Journal Transaction; ticket 107's takeover tests pass unchanged in what they check
- [ ] An Ignored Resource fetched with an unchanged ETag writes no Log line
- [ ] A kept resource moved to another href while an Ignored Resource ranking first holds its UID: the Ignored Resource takes over (seam test); a moved resource with no Ignored Resource keeps its Event Rows, as before
- [ ] No `ObjectId`, `moved` or `moved_from` remains; the CalDAV Mirror is read and written by Calendar and UID or href
- [ ] `core/src/sync/caldav/incoming.rs` has no tests; the cases only it covered are seam tests in `core/tests/caldav_sync.rs`
- [ ] Fetched Server Objects are read by the CalDAV Sync Engine, not by the CalDAV client
- [ ] Ticket 107's note on where the takeover lives, and doc comments citing it, point to the new place
