# 108: The Provider interface without CalDAV's rules

**What to build:** The Provider interface (spec: "Provider interface"; `Remote`, `core/src/remote.rs`) must fit Google, Microsoft Graph and ICS feeds without changing the Sync Engine. Since tickets 102–107 it carries CalDAV's rules for Ambiguous UIDs and Ignored Resources (ADR 0015). Move them into the CalDAV part of the Sync Engine (`core/src/sync/caldav/`), so that the Provider interface only fetches and reports what changed.

What leaked, and where it goes:

- **Known Server Resources** (`Held`) carry an `ignored` flag and the UID held, both used only for the takeover. Across the Provider interface they carry only what the server side needs: the address, the ETag, and whether FreeCal couldn't read it. To the server side, an Ignored Resource is just a known address and version that isn't fetched again until it changes. The CalDAV Sync Engine reads the UIDs and the ignored flag from its own tables.
- **Fetched Server Objects** come with `Holds`, CalDAV's reading of a resource (one UID per resource, RFC 4791 §4.1), which `Client::multiget` does (`core/src/caldav.rs`). The Provider interface gives the Server Objects as received, and the CalDAV Sync Engine reads them (`read_caldav_resource`, `core/src/import/mod.rs`), pinning floating times to the zone the Provider interface reports, as now.
- **The takeover** (`takeover`, `core/src/caldav.rs`, called from `Client::changes`) decides which Ignored Resources to fetch again when the Server Object keeping an Ambiguous UID stops keeping it (`Holds::keeps`), and fetches them in a second request. This moves to a CalDAV Sync Engine step that runs after the Provider interface reports what changed, and asks it to fetch those addresses through a plain "fetch these Server Objects" call. Like now, both requests happen before the Journal Transaction: no network request inside a Database Transaction (this keeps ticket 107's constraint, and changes only where ticket 107 placed the takeover).
- **`Remote::changes`'s contract** says nothing about UIDs, Ignored Resources or a takeover any more.

ADR 0015's rules don't change: the same resources are fetched, in the same Incoming Sync, and the same one keeps an Ambiguous UID.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] `core/src/remote.rs` names no CalDAV-only concept: no Ignored Resource, UID, `Holds` or takeover in its types or docs
- [ ] The Known Server Resources crossing the Provider interface hold the address, ETag and unreadable flag only
- [ ] The takeover is decided in `core/src/sync/caldav/` and fetched through the Provider interface before the Journal Transaction; ticket 107's takeover tests pass unchanged in what they check
- [ ] Fetched Server Objects are read by the CalDAV Sync Engine, not by the CalDAV client
- [ ] Ticket 107's note on where the takeover lives, and doc comments citing it, point to the new place
